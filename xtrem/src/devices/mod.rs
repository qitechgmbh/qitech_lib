//! Device drivers built on top of the bus.

mod xtrem_scale;

pub use xtrem_scale::{DEFAULT_REQUEST_TIMEOUT, Reading, ScaleMode, XtremScale};

use crate::protocol::{DataAddress, ExecuteResult, ProtocolError, WriteResult};
use std::fmt;

/// The polling contract a synchronous control loop drives.
///
/// This mirrors `modbus::ModbusDevice` on purpose: a machine's `act()` tick calls
/// [`XtremDevice::send_next_request`] and then [`XtremDevice::handle_response`], and neither
/// may block. Requests are not pipelined — a device with one in flight simply skips its turn.
pub trait XtremDevice {
    /// Emit at most one request. A no-op when one is already in flight, or when the device is
    /// in a mode that only listens.
    fn send_next_request(&mut self) -> Result<(), anyhow::Error>;

    /// Drain whatever has arrived since the last tick and fold it into the device's state.
    fn handle_response(&mut self) -> Result<(), anyhow::Error>;

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Errors a device records while talking to a module.
///
/// These are surfaced through the device's `last_error` rather than returned from the polling
/// methods: a single bad reading should not tear down the machine that owns the scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XtremError {
    /// A frame decoded, but its payload was not what the register promises.
    Protocol(ProtocolError),
    /// The module refused a write.
    Write(DataAddress, WriteResult),
    /// The module refused an execute.
    Execute(DataAddress, ExecuteResult),
    /// No answer arrived in time.
    Timeout(DataAddress),
    /// The socket rejected the send.
    Send(String),
}

impl fmt::Display for XtremError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(e) => write!(f, "protocol error: {e}"),
            Self::Write(addr, r) => {
                write!(f, "write to register {:04X}h refused: {r}", addr.as_u16())
            }
            Self::Execute(addr, r) => {
                write!(f, "execute of register {:04X}h refused: {r}", addr.as_u16())
            }
            Self::Timeout(addr) => {
                write!(f, "no response for register {:04X}h", addr.as_u16())
            }
            Self::Send(e) => write!(f, "could not send request: {e}"),
        }
    }
}

impl std::error::Error for XtremError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Protocol(e) => Some(e),
            Self::Write(..) | Self::Execute(..) | Self::Timeout(_) | Self::Send(_) => None,
        }
    }
}

impl From<ProtocolError> for XtremError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}
