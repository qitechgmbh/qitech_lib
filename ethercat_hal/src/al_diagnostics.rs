//! Diagnostics for EtherCAT state transitions.
//!
//! A transition that times out returns a bare `Error::Timeout`, and consumes the group on the
//! way out, so there is nothing left to ask why. Everything here reads AL status with raw
//! `FPRD` commands instead, which need only a [`MainDevice`].

use ethercrab::{
    AlStatusCode, Command, EtherCrabWireRead, MainDevice, RegisterAddress, SubDeviceState,
};
use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{
    DiagnosticRequest, DiagnosticResponse, EtherCATController, EtherCATState, MAX_SUBDEVICES,
    Mailbox, TripleBufProducer,
};

const MAX_REPORTS: usize = 32;

/// With `RetryBehaviour::Count(0)` and a 30 ms PDU timeout, a dead bus would otherwise stall
/// the state machine for seconds.
const SNAPSHOT_DEADLINE: Duration = Duration::from_millis(500);

/// First configured station address ethercrab hands out; the rest are contiguous from here.
const BASE_SUBDEVICE_ADDRESS: u16 = 0x1000;

/// `AlStatus` (0x0130): low nibble is the state, bit 4 is the error flag.
const AL_STATUS_STATE_MASK: u16 = 0x000F;
const AL_STATUS_ERROR_FLAG: u16 = 0x0010;

/// One subdevice's AL state, from `AlStatus` (0x0130) and `AlStatusCode` (0x0134).
#[derive(Clone, Debug)]
pub struct SubDeviceAlStatus {
    pub device_address: u16,
    pub name: String,
    pub state: Option<SubDeviceState>,
    pub error_flag: bool,
    pub al_status_code: Option<AlStatusCode>,
    /// Set when the device did not answer, e.g. a working counter mismatch.
    pub read_error: Option<String>,
}

impl SubDeviceAlStatus {
    pub fn is_faulty(&self) -> bool {
        self.read_error.is_some()
            || self.error_flag
            || !matches!(self.al_status_code, None | Some(AlStatusCode::NoError))
    }
}

impl fmt::Display for SubDeviceAlStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @{:#06x}: ", self.name, self.device_address)?;

        if let Some(err) = &self.read_error {
            return write!(f, "unreachable ({err})");
        }

        match self.state {
            Some(state) => write!(f, "{state}")?,
            None => f.write_str("unknown state")?,
        }
        if self.error_flag {
            f.write_str(" +ERR")?;
        }
        // AlStatusCode's Display already renders "0x001e: Invalid Input Configuration".
        if let Some(code) = self.al_status_code {
            write!(f, ", {code}")?;
        }
        Ok(())
    }
}

/// Which master-level step was being attempted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtherCATTransition {
    InitGroup,
    PreOpToPreOpPdi,
    ConfigureDcSync,
    PreOpPdiToSafeOp,
    SafeOpToOpRequest,
    /// Waiting for every subdevice to confirm OP.
    OpRamp,
    /// A `tx_rx` / `tx_rx_dc` failure in the named state.
    TxRx(EtherCATState),
}

impl fmt::Display for EtherCATTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitGroup => f.write_str("Init -> PreOp (init_single_group)"),
            Self::PreOpToPreOpPdi => f.write_str("PreOp -> PreOp+PDI (into_pre_op_pdi)"),
            Self::ConfigureDcSync => f.write_str("configure_dc_sync"),
            Self::PreOpPdiToSafeOp => f.write_str("PreOp+PDI -> SafeOp (into_safe_op)"),
            Self::SafeOpToOpRequest => f.write_str("SafeOp -> OP (request_into_op)"),
            Self::OpRamp => f.write_str("OP ramp"),
            Self::TxRx(state) => write!(f, "TX/RX in {state:?}"),
        }
    }
}

/// The outcome of one transition, together with the AL snapshot taken right after it.
#[derive(Clone, Debug)]
pub struct TransitionReport {
    pub transition: EtherCATTransition,
    pub succeeded: bool,
    pub error: Option<String>,
    pub duration: Duration,
    pub statuses: Vec<SubDeviceAlStatus>,
}

impl TransitionReport {
    pub fn faulty_devices(&self) -> impl Iterator<Item = &SubDeviceAlStatus> {
        self.statuses.iter().filter(|status| status.is_faulty())
    }
}

impl fmt::Display for TransitionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} after {:.1?}",
            self.transition,
            if self.succeeded {
                "succeeded"
            } else {
                "failed"
            },
            self.duration
        )?;

        if let Some(error) = &self.error {
            write!(f, ": {error}")?;
        }

        let faulty = self.faulty_devices().collect::<Vec<_>>();
        if faulty.is_empty() {
            // Points at the master or the wire rather than a terminal.
            if !self.succeeded {
                f.write_str(" — no subdevice reported an AL error")?;
            }
            return Ok(());
        }

        f.write_str(" — ")?;
        for (i, status) in faulty.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{status}")?;
        }
        Ok(())
    }
}

/// Bounded, shared history of transition reports. Cloning shares the buffer.
///
/// Being an `Arc`, it stays readable after the state-machine thread has ended — the point of
/// the whole exercise, since the error that thread returns is otherwise lost with it.
///
/// `std::sync::Mutex` rather than the tokio one used elsewhere: it must be lockable from the
/// OP arm, which runs under `futures::executor::block_on`.
#[derive(Clone, Default)]
pub struct TransitionLog(Arc<Mutex<VecDeque<TransitionReport>>>);

impl TransitionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, report: TransitionReport) {
        // Recover from poisoning: panicking here would mask the failure being reported.
        let mut reports = match self.0.lock() {
            Ok(reports) => reports,
            Err(poisoned) => poisoned.into_inner(),
        };
        if reports.len() == MAX_REPORTS {
            reports.pop_front();
        }
        reports.push_back(report);
    }

    /// Oldest first.
    pub fn reports(&self) -> Vec<TransitionReport> {
        match self.0.lock() {
            Ok(reports) => reports.iter().cloned().collect(),
            Err(poisoned) => poisoned.into_inner().iter().cloned().collect(),
        }
    }

    pub fn last_failure(&self) -> Option<TransitionReport> {
        let reports = match self.0.lock() {
            Ok(reports) => reports,
            Err(poisoned) => poisoned.into_inner(),
        };
        reports.iter().rev().find(|r| !r.succeeded).cloned()
    }
}

impl fmt::Debug for TransitionLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TransitionLog")
            .field(&self.reports())
            .finish()
    }
}

fn decode_al_status(raw: u16) -> (Option<SubDeviceState>, bool) {
    let state = SubDeviceState::unpack_from_slice(&[(raw & AL_STATUS_STATE_MASK) as u8]).ok();
    (state, raw & AL_STATUS_ERROR_FLAG != 0)
}

async fn read_one_al_status(
    maindevice: &MainDevice<'_>,
    device_address: u16,
    name: String,
) -> SubDeviceAlStatus {
    // Default wkc of 1, so a silent device surfaces as a WorkingCounter error rather than
    // being decoded from an all-zero frame as a valid state.
    let al_status = Command::fprd(device_address, RegisterAddress::AlStatus.into())
        .receive::<u16>(maindevice)
        .await;

    let raw = match al_status {
        Ok(raw) => raw,
        Err(e) => {
            return SubDeviceAlStatus {
                device_address,
                name,
                state: None,
                error_flag: false,
                al_status_code: None,
                read_error: Some(format!("{e:?}")),
            };
        }
    };

    let (state, error_flag) = decode_al_status(raw);

    let al_status_code = Command::fprd(device_address, RegisterAddress::AlStatusCode.into())
        .ignore_wkc()
        .receive::<AlStatusCode>(maindevice)
        .await
        .ok();

    SubDeviceAlStatus {
        device_address,
        name,
        state,
        error_flag,
        al_status_code,
        read_error: None,
    }
}

/// Read `(AlStatus, AlStatusCode)` from every known subdevice.
///
/// Works in any master state, and after a group-consuming transition has dropped the group.
/// Devices not reached before [`SNAPSHOT_DEADLINE`] are reported unreachable, not omitted.
pub async fn read_al_statuses(
    maindevice: &MainDevice<'_>,
    devices: &[(u16, String)],
) -> Vec<SubDeviceAlStatus> {
    let started = Instant::now();
    let mut statuses = Vec::with_capacity(devices.len());

    for (device_address, name) in devices {
        if started.elapsed() >= SNAPSHOT_DEADLINE {
            statuses.push(SubDeviceAlStatus {
                device_address: *device_address,
                name: name.clone(),
                state: None,
                error_flag: false,
                al_status_code: None,
                read_error: Some("snapshot deadline exceeded before this device was read".into()),
            });
            continue;
        }
        statuses.push(read_one_al_status(maindevice, *device_address, name.clone()).await);
    }

    statuses
}

/// Addresses reconstructed from the subdevice count, for use before enumeration has filled in
/// [`crate::MetaSubdevice`]. An empty result is itself the diagnosis: nothing answered.
pub fn fallback_addresses(maindevice: &MainDevice<'_>) -> Vec<(u16, String)> {
    (0..maindevice.num_subdevices() as u16)
        .map(|i| {
            let address = BASE_SUBDEVICE_ADDRESS.wrapping_add(i);
            (address, format!("subdevice #{i}"))
        })
        .collect()
}

impl EtherCATController<Arc<Mailbox>, TripleBufProducer> {
    /// The subdevices to probe, as `(configured_address, name)`. Falls back to reconstructed
    /// addresses during `init_single_group`, the one transition that runs before enumeration.
    async fn diagnostic_devices(&self, maindevice: &MainDevice<'_>) -> Vec<(u16, String)> {
        let count = self
            .subdevice_count
            .load(std::sync::atomic::Ordering::Relaxed) as usize;
        if count == 0 {
            return fallback_addresses(maindevice);
        }

        let subdevices = self.subdevices.lock().await;
        subdevices[..count.min(MAX_SUBDEVICES)]
            .iter()
            .map(|meta| {
                let name = meta
                    .get_name()
                    .unwrap_or_else(|_| format!("{:#06x}", meta.device_address));
                (meta.device_address, name)
            })
            .collect()
    }

    /// File a [`TransitionReport`] with an AL snapshot taken right after `result` was produced.
    /// On failure the returned error carries the rendered snapshot.
    ///
    /// Takes an already-evaluated `Result` so it covers the group-consuming transitions too:
    /// by now the group may be dropped, and the snapshot needs only `maindevice`.
    pub(crate) async fn record<T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        maindevice: &MainDevice<'_>,
        started: Instant,
        result: Result<T, E>,
    ) -> Result<T, anyhow::Error> {
        let error = result.as_ref().err().map(|e| format!("{e:?}"));
        let succeeded = error.is_none();
        let devices = self.diagnostic_devices(maindevice).await;

        let report = TransitionReport {
            transition,
            succeeded,
            error,
            duration: started.elapsed(),
            statuses: read_al_statuses(maindevice, &devices).await,
        };

        if succeeded {
            tracing::debug!("{report}");
        } else {
            tracing::error!("{report}");
        }

        let message = report.to_string();
        self.transition_log.push(report);

        result.map_err(|_| anyhow::anyhow!(message))
    }

    /// Run a one-shot state transition, recording it either way. Recording successes gives the
    /// next failure its "before" state for free, and shows how far startup got.
    pub(crate) async fn transition<T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        maindevice: &MainDevice<'_>,
        op: impl Future<Output = Result<T, E>>,
    ) -> Result<T, anyhow::Error> {
        let started = Instant::now();
        let result = op.await;
        self.record(transition, maindevice, started, result).await
    }

    /// Run a per-cycle operation, recording it only if it fails. Snapshotting every success
    /// would put two extra PDUs per subdevice on the wire every cycle.
    pub(crate) async fn guard<T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        maindevice: &MainDevice<'_>,
        op: impl Future<Output = Result<T, E>>,
    ) -> Result<T, anyhow::Error> {
        let started = Instant::now();
        match op.await {
            Ok(value) => Ok(value),
            Err(e) => {
                self.record(transition, maindevice, started, Err::<T, E>(e))
                    .await
            }
        }
    }

    /// Answer at most one pending diagnostic read. Called from every state arm; the one-per-
    /// iteration cap keeps a busy client from starving the OP loop.
    pub(crate) async fn service_diagnostic_request(&self, maindevice: &MainDevice<'_>) {
        let Ok(request) = self.diagnostic_channel.try_recv() else {
            return;
        };

        match request {
            DiagnosticRequest::RegisterRead {
                device_address,
                register,
                response_channel,
            } => {
                let result = Command::fprd(device_address, register)
                    .receive::<u16>(maindevice)
                    .await
                    .map_err(|e| anyhow::anyhow!("register read failed: {e:?}"));
                let _ = response_channel.send(DiagnosticResponse::RegisterReadResponse(result));
            }
            DiagnosticRequest::AlStatusSnapshot { response_channel } => {
                let devices = self.diagnostic_devices(maindevice).await;
                let statuses = read_al_statuses(maindevice, &devices).await;
                let _ =
                    response_channel.send(DiagnosticResponse::AlStatusSnapshotResponse(statuses));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(name: &str, code: Option<AlStatusCode>, error_flag: bool) -> SubDeviceAlStatus {
        SubDeviceAlStatus {
            device_address: 0x1000,
            name: name.into(),
            state: Some(SubDeviceState::SafeOp),
            error_flag,
            al_status_code: code,
            read_error: None,
        }
    }

    fn report(statuses: Vec<SubDeviceAlStatus>) -> TransitionReport {
        TransitionReport {
            transition: EtherCATTransition::SafeOpToOpRequest,
            succeeded: false,
            error: Some("Timeout".into()),
            duration: Duration::from_secs(20),
            statuses,
        }
    }

    #[test]
    fn decodes_state_and_error_flag() {
        assert_eq!(
            decode_al_status(0x12),
            (Some(SubDeviceState::PreOp), true),
            "0x12 is PreOp with the error flag set"
        );
        assert_eq!(decode_al_status(0x08), (Some(SubDeviceState::Op), false));
        assert_eq!(
            decode_al_status(0x04),
            (Some(SubDeviceState::SafeOp), false)
        );
        assert_eq!(decode_al_status(0x00), (Some(SubDeviceState::None), false));
    }

    #[test]
    fn faulty_devices_skips_healthy_ones() {
        let report = report(vec![
            status("EL2004", Some(AlStatusCode::NoError), false),
            status(
                "EL7031",
                Some(AlStatusCode::InvalidInputConfiguration),
                true,
            ),
        ]);

        let faulty = report.faulty_devices().collect::<Vec<_>>();
        assert_eq!(faulty.len(), 1);
        assert_eq!(faulty[0].name, "EL7031");
    }

    #[test]
    fn unreachable_device_counts_as_faulty() {
        let mut unreachable = status("EL1008", None, false);
        unreachable.read_error = Some("WorkingCounter".into());
        assert!(unreachable.is_faulty());
    }

    #[test]
    fn renders_the_al_status_code_text() {
        let rendered = report(vec![
            status("EL2004", Some(AlStatusCode::NoError), false),
            status(
                "EL7031",
                Some(AlStatusCode::InvalidInputConfiguration),
                true,
            ),
        ])
        .to_string();

        assert!(
            rendered.contains("0x001e: Invalid Input Configuration"),
            "expected the ETG1000.6 code and reason, got: {rendered}"
        );
        assert!(rendered.contains("EL7031 @0x1000"));
        assert!(rendered.contains("+ERR"));
        assert!(
            !rendered.contains("EL2004"),
            "healthy devices should not be listed, got: {rendered}"
        );
    }

    #[test]
    fn failure_without_faulty_devices_says_so() {
        let rendered =
            report(vec![status("EL2004", Some(AlStatusCode::NoError), false)]).to_string();
        assert!(rendered.contains("no subdevice reported an AL error"));
    }

    #[test]
    fn log_caps_and_evicts_oldest_first() {
        let log = TransitionLog::new();
        for i in 0..MAX_REPORTS + 5 {
            let mut r = report(vec![]);
            r.duration = Duration::from_secs(i as u64);
            log.push(r);
        }

        let reports = log.reports();
        assert_eq!(reports.len(), MAX_REPORTS);
        assert_eq!(
            reports[0].duration,
            Duration::from_secs(5),
            "the five oldest reports should have been evicted"
        );
    }

    #[test]
    fn last_failure_finds_the_most_recent_one() {
        let log = TransitionLog::new();

        let mut failed = report(vec![]);
        failed.transition = EtherCATTransition::PreOpPdiToSafeOp;
        log.push(failed);

        let mut succeeded = report(vec![]);
        succeeded.succeeded = true;
        succeeded.error = None;
        log.push(succeeded);

        let last = log.last_failure().expect("a failure was recorded");
        assert_eq!(last.transition, EtherCATTransition::PreOpPdiToSafeOp);
    }

    #[test]
    fn last_failure_is_none_when_everything_worked() {
        let log = TransitionLog::new();
        let mut ok = report(vec![]);
        ok.succeeded = true;
        ok.error = None;
        log.push(ok);

        assert!(log.last_failure().is_none());
    }
}
