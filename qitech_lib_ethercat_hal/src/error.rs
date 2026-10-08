//! Error types of this crate.
//!
//! Every subsystem that can fail has its own enum, so callers can match on what went wrong
//! instead of only logging it. [`Error`] unites them for functions that touch several
//! subsystems at once.

use std::time::Duration;

/// Encoding or decoding of process data (PDOs).
#[derive(Debug, Clone, thiserror::Error)]
pub enum PdoError {
    /// The process-data slice handed to a device has the wrong size.
    #[error("{device}: got {actual} bits of process data, expected {expected}")]
    LengthMismatch {
        device: &'static str,
        expected: usize,
        actual: usize,
    },
    /// A PDO object does not fit into the process-data slice.
    #[error("PDO range {start}..{end} is out of bounds for a buffer of {len} bits")]
    OutOfBounds {
        start: usize,
        end: usize,
        len: usize,
    },
    /// A PDO object the operation needs is not part of the current PDO assignment.
    #[error("PDO object `{0}` is not assigned")]
    MissingObject(&'static str),
    /// The configured PDO assignment is not one the device supports.
    #[error("invalid PDO assignment")]
    InvalidAssignment,
    /// A message is larger than the PDO buffer that carries it.
    #[error("message of {len} bytes does not fit into the {max} byte PDO buffer")]
    MessageTooLong { len: usize, max: usize },
}

/// Using a device driver: ports, modes and driver lookup.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DeviceError {
    /// The device has no such port, or the port can not be used this way.
    #[error("{device} has no port {port}")]
    InvalidPort { device: &'static str, port: usize },
    /// The device does not offer the requested functionality.
    #[error("{device} does not support {feature}")]
    Unsupported {
        device: &'static str,
        feature: &'static str,
    },
    /// The operation needs a different mode or setup than the device is currently in.
    #[error("{device}: {reason}")]
    InvalidState {
        device: &'static str,
        reason: &'static str,
    },
    /// A value handed to the device is out of range.
    #[error("{device}: invalid value {value} for {what}")]
    InvalidValue {
        device: &'static str,
        what: &'static str,
        value: f64,
    },
    /// The device reports a fault on one of its channels.
    #[error("{device} channel {channel}: {fault}")]
    ChannelFault {
        device: &'static str,
        channel: usize,
        fault: &'static str,
    },
    /// No driver is implemented for this subdevice identity.
    #[error("no driver for vendor 0x{vendor:x}, product 0x{product:x}, revision 0x{revision:x}")]
    NoDriver {
        vendor: u32,
        product: u32,
        revision: u32,
    },
    /// The driver is not of the type it was downcast to.
    #[error("device is not of type {0}")]
    Downcast(&'static str),
    /// A raw value read from the device does not map to any known variant.
    #[error("unknown value {value} for {what}")]
    UnknownValue { what: &'static str, value: u64 },
    #[error(transparent)]
    Pdo(#[from] PdoError),
}

/// Requests to the EtherCAT master thread and the subdevice operations it executes
/// (SDO access, DC configuration, EEPROM, register reads).
#[derive(Debug, Clone, thiserror::Error)]
pub enum ChannelError {
    /// The master thread is not running (anymore), so the request could not be delivered
    /// or answered.
    #[error("EtherCAT master thread is not running")]
    Disconnected,
    /// The master thread did not answer in time.
    #[error("EtherCAT master thread did not answer within {0:?}")]
    Timeout(Duration),
    /// The master thread answered with a response that does not belong to the request.
    #[error("unexpected response from the EtherCAT master thread")]
    UnexpectedResponse,
    /// No subdevice with this configured address is on the bus.
    #[error("no subdevice with address 0x{0:04x}")]
    UnknownSubdevice(u16),
    /// A value did not arrive in the expected type.
    #[error("SDO value of type {0} can not be converted to {1}")]
    SdoTypeMismatch(&'static str, &'static str),
    /// The subdevice answered, but with a value that makes no sense.
    #[error("subdevice returned an invalid value for {0}")]
    InvalidResponse(&'static str),
    /// A sequence of SDO writes did not reach the expected state in time.
    #[error("timeout waiting for {0}")]
    StatusTimeout(String),
    /// An SDO access failed on the bus.
    #[error("SDO access to 0x{index:04x}:{sub_index:02x} failed: {source}")]
    Sdo {
        index: u16,
        sub_index: u8,
        #[source]
        source: ethercrab::error::Error,
    },
    /// The operation failed on the bus.
    #[error(transparent)]
    Ethercrab(#[from] ethercrab::error::Error),
}

/// Finding and probing network interfaces.
#[derive(Debug, thiserror::Error)]
pub enum InterfaceError {
    /// The interface name can not be passed to the OS (it contains a NUL byte).
    #[error("invalid interface name {0:?}")]
    InvalidName(String),
    /// The OS does not know this interface.
    #[error("interface {0} not found")]
    NotFound(String),
    /// No EtherCAT subdevice answered on this interface.
    #[error("no EtherCAT response on {0}")]
    NoResponse(String),
    /// All BPF devices are in use (macOS).
    #[error("no free BPF device")]
    NoFreeBpfDevice,
    /// Not all bytes of the probe frame could be written.
    #[error("probe frame write sent {sent} of {len} bytes")]
    ShortWrite { sent: isize, len: usize },
    /// Raw interface probing is not implemented for this OS.
    #[error("interface probing is not supported on this platform")]
    UnsupportedPlatform,
    /// A system call failed.
    #[error("{call} failed: {source}")]
    Os {
        call: &'static str,
        #[source]
        source: std::io::Error,
    },
}

impl InterfaceError {
    /// Wrap the last OS error (`errno`) for the failed `call`.
    pub(crate) fn last_os_error(call: &'static str) -> Self {
        Self::Os {
            call,
            source: std::io::Error::last_os_error(),
        }
    }
}

/// Failures of the EtherCAT state machine and its transitions.
#[derive(Debug, thiserror::Error)]
pub enum EthercatErr {
    #[error("Ethercat-error: {0}")]
    Custom(String),
    #[error("Failed to transition to PREOP state")]
    PreopTransitionFailed,
    #[error("Failed to transition to SAFEOP state")]
    SafeopTransitionFailed,
    #[error("Failed to transition to OP state")]
    OpTransitionFailed,
    #[error("Working counter error (WKC)")]
    WorkingCounterErr,
    /// Locking the process memory for realtime operation failed.
    #[error("memory locking failed with result {0}")]
    MemoryLock(i32),
}

/// Any error of this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Pdo(#[from] PdoError),
    #[error(transparent)]
    Device(#[from] DeviceError),
    #[error(transparent)]
    Channel(#[from] ChannelError),
    #[error(transparent)]
    Interface(#[from] InterfaceError),
    #[error(transparent)]
    State(#[from] EthercatErr),
}

impl From<ethercrab::error::Error> for Error {
    fn from(e: ethercrab::error::Error) -> Self {
        Self::Channel(e.into())
    }
}
