//! Finding the modules on a subnet.
//!
//! The sweep is a single broadcast read of register `0000h` (serial number): every module that
//! hears it answers, and its response carries both the serial number and — in the `ID_O` field
//! and the datagram's source address — everything needed to address it directly afterwards.
//!
//! # Device ID collisions
//!
//! Every XTREM leaves the factory with device ID `01`. On a fresh multi-module install they
//! therefore all answer to the same `ID_D`, and unicast-by-ID is ambiguous. Discovery works
//! anyway because it keys on the source IP address, and it flags the ambiguity in
//! [`XtremProbe::id_collision`]. Resolving it means writing unique values to register `0001h`,
//! one module at a time — see [`assign_device_id`].

use crate::protocol::{
    BROADCAST_ID, DataAddress, DeviceState, Function, RegisterValue, WriteResult,
};
use crate::transport::{Destination, Inbound, XtremBusHandle};
use std::collections::BTreeMap;
use std::net::SocketAddrV4;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{Instant, timeout};

/// How long to listen for answers to the broadcast probe.
pub const DEFAULT_DISCOVERY_WINDOW: Duration = Duration::from_millis(2000);

/// How long a single follow-up read may take before it is given up on.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_millis(500);

/// One module found on the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtremProbe {
    /// The `ID_O` the module answered with — its network address.
    pub device_id: u8,
    /// Register `0000h`. Factory-set and unique, so this is the stable identity to key on.
    pub serial: u32,
    /// Where the response came from. Use this for unicast traffic.
    pub addr: SocketAddrV4,
    /// Register `0009h`. `Some(true)` means the sealing switch is LOCKed, which blocks writes
    /// to legally relevant parameters and enables the software protection protocol this crate
    /// does not implement. `None` if the follow-up read timed out.
    pub sealed: Option<bool>,
    /// Register `0100h`, if the follow-up read succeeded.
    pub state: Option<DeviceState>,
    /// Another module on the network answered with the same `device_id`.
    pub id_collision: bool,
}

/// Broadcast a serial-number read and collect everything that answers within `window`.
///
/// Results are sorted by serial number so repeated sweeps produce a stable order.
pub async fn discover(
    handle: &XtremBusHandle,
    window: Duration,
) -> Result<Vec<XtremProbe>, anyhow::Error> {
    // Subscribe before probing, otherwise a fast module can answer before we are listening.
    let mut events = handle.events();
    handle
        .send(
            &handle.read_frame(BROADCAST_ID, DataAddress::SerialNumber),
            Destination::Broadcast,
        )
        .await?;

    // Keyed by source address: two modules that share a device ID are still distinguishable
    // here, which is what makes collision reporting possible.
    let mut found: BTreeMap<SocketAddrV4, (u8, u32)> = BTreeMap::new();
    let deadline = Instant::now() + window;

    while let Some(inbound) = next_before(&mut events, deadline).await {
        if let Some(RegisterValue::SerialNumber(serial)) =
            read_response(&inbound, DataAddress::SerialNumber)
        {
            found.insert(inbound.from, (inbound.frame.id_origin, serial));
        }
    }

    let mut id_counts: BTreeMap<u8, usize> = BTreeMap::new();
    for (device_id, _) in found.values() {
        *id_counts.entry(*device_id).or_default() += 1;
    }

    let mut probes = Vec::with_capacity(found.len());
    for (addr, (device_id, serial)) in found {
        // Follow-ups go unicast, so a shared device ID does not confuse them.
        let sealed = match read_once(
            handle,
            device_id,
            addr,
            DataAddress::SealingSwitchState,
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
        {
            Some(RegisterValue::Sealed(sealed)) => Some(sealed),
            _ => None,
        };
        let state = match read_once(
            handle,
            device_id,
            addr,
            DataAddress::DeviceState,
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
        {
            Some(RegisterValue::DeviceState(state)) => Some(state),
            _ => None,
        };

        probes.push(XtremProbe {
            device_id,
            serial,
            addr,
            sealed,
            state,
            id_collision: id_counts.get(&device_id).copied().unwrap_or(0) > 1,
        });
    }

    probes.sort_by_key(|probe| probe.serial);
    Ok(probes)
}

/// Read a single register from one module and wait for the answer.
///
/// Returns `None` on timeout or if the module answered with something unparseable.
pub async fn read_once(
    handle: &XtremBusHandle,
    device_id: u8,
    addr: SocketAddrV4,
    address: DataAddress,
    request_timeout: Duration,
) -> Option<RegisterValue> {
    let mut events = handle.events();
    handle
        .send(
            &handle.read_frame(device_id, address),
            Destination::Unicast(addr),
        )
        .await
        .ok()?;

    let deadline = Instant::now() + request_timeout;
    while let Some(inbound) = next_before(&mut events, deadline).await {
        if inbound.from != addr || inbound.frame.id_origin != device_id {
            continue;
        }
        if let Some(value) = read_response(&inbound, address) {
            return Some(value);
        }
    }
    None
}

/// Write a new device ID to register `0001h`.
///
/// Do this to one module at a time — while several share an ID, a broadcast write would
/// renumber all of them to the same new value.
pub async fn assign_device_id(
    handle: &XtremBusHandle,
    current_id: u8,
    addr: SocketAddrV4,
    new_id: u8,
    request_timeout: Duration,
) -> Result<(), anyhow::Error> {
    let mut events = handle.events();
    let payload = format!("{new_id:02X}").into_bytes();
    handle
        .send(
            &handle.write_frame(current_id, DataAddress::DeviceId, payload),
            Destination::Unicast(addr),
        )
        .await?;

    let deadline = Instant::now() + request_timeout;
    while let Some(inbound) = next_before(&mut events, deadline).await {
        if inbound.from != addr || inbound.frame.function != Function::WriteResponse {
            continue;
        }
        if inbound.frame.address != DataAddress::DeviceId {
            continue;
        }
        // Spec §8.2: the response still carries the *old* device ID in ID_O.
        let result = inbound
            .frame
            .data
            .first()
            .copied()
            .map_or(WriteResult::FlashError(0), WriteResult::from);
        return if result.is_ok() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "device {current_id:02X}h refused ID write: {result}"
            ))
        };
    }
    Err(anyhow::anyhow!(
        "device {current_id:02X}h did not answer the ID write within {request_timeout:?}"
    ))
}

/// Extract the value from `inbound` if it is a read response for `address`.
fn read_response(inbound: &Inbound, address: DataAddress) -> Option<RegisterValue> {
    if inbound.frame.function != Function::ReadResponse || inbound.frame.address != address {
        return None;
    }
    RegisterValue::parse(address, &inbound.frame.data).ok()
}

/// Next frame from the bus, or `None` once `deadline` passes.
async fn next_before(
    events: &mut broadcast::Receiver<Inbound>,
    deadline: Instant,
) -> Option<Inbound> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match timeout(remaining, events.recv()).await {
            Ok(Ok(inbound)) => return Some(inbound),
            // A burst of stream traffic can outrun us; skipping it is fine, we only need to
            // see one answer per register.
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => return None,
        }
    }
}
