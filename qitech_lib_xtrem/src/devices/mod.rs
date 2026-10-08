//! Device drivers built on top of the bus.

mod xtrem_scale;

pub use xtrem_scale::{DEFAULT_REQUEST_TIMEOUT, Reading, ScaleMode, XtremScale};

use crate::protocol::{DataAddress, ExecuteResult, ProtocolError, WriteResult};

/// The polling contract a synchronous control loop drives.
///
/// This mirrors `modbus::ModbusDevice` on purpose: a machine's `act()` tick calls
/// [`XtremDevice::send_next_request`] and then [`XtremDevice::handle_response`], and neither
/// may block. Requests are not pipelined — a device with one in flight simply skips its turn.
pub trait XtremDevice {
    /// Emit at most one request. A no-op when one is already in flight, or when the device is
    /// in a mode that only listens.
    fn send_next_request(&mut self) -> Result<(), DeviceError>;

    /// Drain whatever has arrived since the last tick and fold it into the device's state.
    fn handle_response(&mut self) -> Result<(), DeviceError>;

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Errors that stop a device's polling loop.
///
/// Unlike [`XtremError`], these are not per-request hiccups: the device cannot make progress
/// until the caller intervenes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeviceError {
    /// Every handle to the bus was dropped, so no more frames can arrive.
    #[error("xtrem bus closed while device {device_id:02X}h was attached")]
    BusClosed { device_id: u8 },
}

/// Errors a device records while talking to a module.
///
/// These are surfaced through the device's `last_error` rather than returned from the polling
/// methods: a single bad reading should not tear down the machine that owns the scale.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XtremError {
    /// A frame decoded, but its payload was not what the register promises.
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    /// The module refused a write.
    #[error("write to register {addr:04X}h refused: {1}", addr = .0.as_u16())]
    Write(DataAddress, WriteResult),
    /// The module refused an execute.
    #[error("execute of register {addr:04X}h refused: {1}", addr = .0.as_u16())]
    Execute(DataAddress, ExecuteResult),
    /// No answer arrived in time.
    #[error("no response for register {addr:04X}h", addr = .0.as_u16())]
    Timeout(DataAddress),
    /// The socket rejected the send.
    #[error("could not send request: {0}")]
    Send(String),
}
