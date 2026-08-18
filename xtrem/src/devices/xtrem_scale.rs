use super::{XtremDevice, XtremError};
use crate::protocol::{
    DataAddress, DeviceState, ExecuteResult, Frame, Function, RegisterValue, WeighingStatus,
    Weight, WriteResult,
};
use crate::transport::{Destination, Inbound, XtremBusHandle};
use std::collections::VecDeque;
use std::net::SocketAddrV4;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use units::Mass;

/// How long to wait for an answer before giving up on it. The spec allows up to one second
/// between the STX and ETX of an incoming message (§4), so anything slower is a lost frame.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_millis(1000);

/// A stream is considered dead after this many missed intervals, at which point the driver
/// re-sends the start sequence. A module that reboots stops streaming without saying so, and
/// nothing else would ever notice.
const STREAM_STALE_INTERVALS: u32 = 10;

/// Floor for the staleness window, so a fast stream does not re-arm on a momentary hiccup.
const STREAM_STALE_FLOOR: Duration = Duration::from_millis(1000);

fn stream_stale_window(interval_ms: u16) -> Duration {
    Duration::from_millis(u64::from(interval_ms))
        .saturating_mul(STREAM_STALE_INTERVALS)
        .max(STREAM_STALE_FLOOR)
}

/// How the driver gets its readings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    /// Read register `0107h` once per in-flight slot. Deterministic, and the only mode that
    /// tells you the link is alive, because silence is indistinguishable from a stalled stream.
    #[default]
    Poll,
    /// Ask the module to push `0107h` every `interval_ms` (register `0013h` + execute `1011h`).
    /// Cheaper on the wire, but the module keeps streaming until told to stop.
    Stream { interval_ms: u16 },
}

/// One weighing sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub gross: Weight,
    pub tare: Weight,
    /// Gross − tare.
    pub net: Mass,
    pub status: WeighingStatus,
    pub at: Instant,
}

/// A command the caller has asked for but which has not been sent yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Execute `0102h` — set the tare to the current weight.
    Tare,
    /// Execute `1103h`.
    ClearTare,
    /// Execute `0105h`.
    Zero,
    /// Read `0100h`.
    ReadDeviceState,
    /// Write `0013h` with the stream interval.
    SetStreamInterval(u16),
    /// Execute `1011h`.
    StartStream,
    /// Execute `1010h`.
    StopStream,
}

impl Command {
    fn to_frame(self, bus: &XtremBusHandle, device_id: u8) -> Frame {
        match self {
            Self::Tare => bus.execute_frame(device_id, DataAddress::TareValue),
            Self::ClearTare => bus.execute_frame(device_id, DataAddress::ClearTare),
            Self::Zero => bus.execute_frame(device_id, DataAddress::ZeroIndicator),
            Self::ReadDeviceState => bus.read_frame(device_id, DataAddress::DeviceState),
            Self::SetStreamInterval(ms) => bus.write_frame(
                device_id,
                DataAddress::StreamOutputRate,
                ms.to_string().into_bytes(),
            ),
            Self::StartStream => bus.execute_frame(device_id, DataAddress::StartStreamWeight),
            Self::StopStream => bus.execute_frame(device_id, DataAddress::StopStream),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    function: Function,
    address: DataAddress,
    sent_at: Instant,
}

/// A driver for one XTREM weighing module.
///
/// Poll it from a synchronous control loop:
///
/// ```no_run
/// # use xtrem::{XtremDevice, XtremScale};
/// # fn tick(scale: &mut XtremScale) -> Result<(), anyhow::Error> {
/// scale.send_next_request()?;
/// scale.handle_response()?;
/// if let Some(reading) = scale.reading {
///     // reading.net is a `units::Mass`
/// }
/// # Ok(())
/// # }
/// ```
pub struct XtremScale {
    bus: XtremBusHandle,
    device_id: u8,
    /// Learned from discovery. Requests go unicast when it is known, which sidesteps device-ID
    /// collisions and keeps broadcast traffic off the subnet.
    addr: Option<SocketAddrV4>,
    serial: Option<u32>,
    inbox: mpsc::Receiver<Inbound>,
    mode: ScaleMode,
    /// Stream start-up has been queued, so it is not queued again every tick.
    stream_configured: bool,
    /// When the stream start sequence was last begun, for staleness detection.
    stream_armed_at: Instant,
    /// When the last frame arrived from this module, for staleness detection.
    last_frame_at: Option<Instant>,
    pending: Option<Pending>,
    commands: VecDeque<Command>,
    request_timeout: Duration,

    /// The most recent weighing sample, from a poll response or a streamed frame.
    pub reading: Option<Reading>,
    /// Register `0100h`, refreshed whenever [`XtremScale::request_device_state`] is used.
    pub device_state: Option<DeviceState>,
    /// The last thing that went wrong. Cleared by [`XtremScale::take_error`].
    pub last_error: Option<XtremError>,
}

impl XtremScale {
    /// Attach to the module with network address `device_id`.
    ///
    /// Registering the same `device_id` on one bus twice steals the route from the earlier
    /// driver, so build at most one `XtremScale` per module.
    pub fn new(bus: &XtremBusHandle, device_id: u8, mode: ScaleMode) -> Self {
        let inbox = bus.subscribe(device_id);
        Self {
            bus: bus.clone(),
            device_id,
            addr: None,
            serial: None,
            inbox,
            mode,
            stream_configured: false,
            stream_armed_at: Instant::now(),
            last_frame_at: None,
            pending: None,
            commands: VecDeque::new(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            reading: None,
            device_state: None,
            last_error: None,
        }
    }

    /// Attach using everything discovery learned, including the unicast address.
    pub fn from_probe(
        bus: &XtremBusHandle,
        probe: &crate::discovery::XtremProbe,
        mode: ScaleMode,
    ) -> Self {
        let mut scale = Self::new(bus, probe.device_id, mode);
        scale.addr = Some(probe.addr);
        scale.serial = Some(probe.serial);
        scale.device_state = probe.state;
        scale
    }

    pub const fn device_id(&self) -> u8 {
        self.device_id
    }

    pub const fn serial(&self) -> Option<u32> {
        self.serial
    }

    pub const fn addr(&self) -> Option<SocketAddrV4> {
        self.addr
    }

    /// Point subsequent requests at a specific module instead of the broadcast address.
    pub fn set_addr(&mut self, addr: SocketAddrV4) {
        self.addr = Some(addr);
    }

    pub fn set_request_timeout(&mut self, timeout: Duration) {
        self.request_timeout = timeout;
    }

    /// Take the last error, clearing it.
    pub fn take_error(&mut self) -> Option<XtremError> {
        self.last_error.take()
    }

    /// Set the tare to the current weight (execute `0102h`).
    ///
    /// The module waits for a stable reading and answers `'4'` if that never happens.
    pub fn tare(&mut self) {
        self.commands.push_back(Command::Tare);
    }

    /// Clear the tare (execute `1103h`).
    pub fn clear_tare(&mut self) {
        self.commands.push_back(Command::ClearTare);
    }

    /// Zero the scale (execute `0105h`).
    pub fn zero(&mut self) {
        self.commands.push_back(Command::Zero);
    }

    /// Queue a read of register `0100h`; the answer lands in [`XtremScale::device_state`].
    pub fn request_device_state(&mut self) {
        self.commands.push_back(Command::ReadDeviceState);
    }

    /// Switch between polling and streaming. Leaving stream mode stops the module's stream.
    pub fn set_mode(&mut self, mode: ScaleMode) {
        if mode == self.mode {
            return;
        }
        if matches!(self.mode, ScaleMode::Stream { .. }) {
            self.commands.push_back(Command::StopStream);
        }
        self.mode = mode;
        self.stream_configured = false;
    }

    pub const fn mode(&self) -> ScaleMode {
        self.mode
    }

    fn destination(&self) -> Destination {
        self.addr
            .map_or(Destination::Broadcast, Destination::Unicast)
    }

    /// The request to send this tick, if any.
    fn next_frame(&mut self) -> Option<Frame> {
        if let Some(command) = self.commands.pop_front() {
            return Some(command.to_frame(&self.bus, self.device_id));
        }

        match self.mode {
            ScaleMode::Poll => Some(
                self.bus
                    .read_frame(self.device_id, DataAddress::WeighingRegister),
            ),
            ScaleMode::Stream { interval_ms } => {
                if self.stream_configured && !self.stream_is_stale(interval_ms) {
                    // The module is pushing frames; there is nothing to ask for.
                    return None;
                }
                self.stream_configured = true;
                self.stream_armed_at = Instant::now();
                self.commands.push_back(Command::StartStream);
                Some(Command::SetStreamInterval(interval_ms).to_frame(&self.bus, self.device_id))
            }
        }
    }

    /// True when nothing has arrived for long enough that the stream is presumed dead.
    ///
    /// Measured from the later of the last frame and the last arming attempt, so a re-arm that
    /// itself goes unanswered still gets a full window before the next retry.
    fn stream_is_stale(&self, interval_ms: u16) -> bool {
        let reference = match self.last_frame_at {
            Some(at) if at > self.stream_armed_at => at,
            _ => self.stream_armed_at,
        };
        reference.elapsed() > stream_stale_window(interval_ms)
    }

    /// Fold one frame into the driver's state.
    fn apply(&mut self, inbound: Inbound) {
        let frame = inbound.frame;
        self.last_frame_at = Some(inbound.at);

        // A response clears the in-flight slot even if its payload turns out to be junk,
        // otherwise one malformed answer would stall the driver until the timeout.
        if let Some(pending) = self.pending
            && pending.function.response() == frame.function
            && pending.address == frame.address
        {
            self.pending = None;
        }

        match frame.function {
            Function::ReadResponse => self.apply_read(frame.address, &frame.data, inbound.at),
            Function::WriteResponse => {
                let result =
                    first_byte(&frame.data).map_or(WriteResult::FlashError(0), WriteResult::from);
                if !result.is_ok() {
                    self.last_error = Some(XtremError::Write(frame.address, result));
                }
            }
            Function::ExecuteResponse => {
                let result =
                    first_byte(&frame.data).map_or(ExecuteResult::Other(0), ExecuteResult::from);
                if !result.is_ok() {
                    self.last_error = Some(XtremError::Execute(frame.address, result));
                }
            }
            // Requests from other hosts on the bus are not ours to answer.
            _ => {}
        }
    }

    fn apply_read(&mut self, address: DataAddress, data: &[u8], at: Instant) {
        let value = match RegisterValue::parse(address, data) {
            Ok(value) => value,
            Err(error) => {
                self.last_error = Some(XtremError::Protocol(error));
                return;
            }
        };

        match value {
            RegisterValue::Weighing(register) => {
                self.reading = Some(Reading {
                    gross: register.gross,
                    tare: register.tare,
                    net: register.net(),
                    status: register.status,
                    at,
                });
            }
            RegisterValue::DeviceState(state) => self.device_state = Some(state),
            RegisterValue::SerialNumber(serial) => self.serial = Some(serial),
            // Other registers are not requested by this driver.
            _ => {}
        }
    }
}

impl XtremDevice for XtremScale {
    fn send_next_request(&mut self) -> Result<(), anyhow::Error> {
        if let Some(pending) = self.pending {
            if pending.sent_at.elapsed() < self.request_timeout {
                // Already waiting; do not pipeline.
                return Ok(());
            }
            self.last_error = Some(XtremError::Timeout(pending.address));
            self.pending = None;
        }

        let Some(frame) = self.next_frame() else {
            return Ok(());
        };

        match self.bus.try_send(&frame, self.destination()) {
            Ok(()) => {
                self.pending = Some(Pending {
                    function: frame.function,
                    address: frame.address,
                    sent_at: Instant::now(),
                });
            }
            Err(error) => {
                // A full socket buffer is a transient condition, not a dead device. Record it
                // and let the next tick try again.
                self.last_error = Some(XtremError::Send(error.to_string()));
            }
        }
        Ok(())
    }

    fn handle_response(&mut self) -> Result<(), anyhow::Error> {
        loop {
            match self.inbox.try_recv() {
                Ok(inbound) => self.apply(inbound),
                Err(mpsc::error::TryRecvError::Empty) => return Ok(()),
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(anyhow::anyhow!(
                        "xtrem bus closed while device {:02X}h was attached",
                        self.device_id
                    ));
                }
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Drop for XtremScale {
    fn drop(&mut self) {
        // Leave the module quiet: a stream that outlives its driver keeps flooding the subnet.
        if matches!(self.mode, ScaleMode::Stream { .. }) {
            let frame = Command::StopStream.to_frame(&self.bus, self.device_id);
            let destination = self.destination();
            if self.bus.try_send(&frame, destination).is_err() {
                // The send buffer was full. Drop cannot wait, so hand the retry to the runtime;
                // the cloned handle keeps the bus alive until it completes.
                let bus = self.bus.clone();
                common::get_async_runtime().spawn(async move {
                    let _ = bus.send(&frame, destination).await;
                });
            }
        }
        self.bus.unsubscribe(self.device_id);
    }
}

fn first_byte(data: &[u8]) -> Option<u8> {
    data.first().copied()
}
