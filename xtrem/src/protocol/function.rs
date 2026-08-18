use super::ProtocolError;
use std::fmt;

/// The `F` field: what the recipient should do, or which action a response answers.
///
/// Uppercase characters are requests, lowercase are the matching responses (spec §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Function {
    ReadRequest,
    ReadResponse,
    WriteRequest,
    WriteResponse,
    ExecuteRequest,
    ExecuteResponse,
}

impl Function {
    pub const fn as_byte(self) -> u8 {
        match self {
            Self::ReadRequest => b'R',
            Self::ReadResponse => b'r',
            Self::WriteRequest => b'W',
            Self::WriteResponse => b'w',
            Self::ExecuteRequest => b'E',
            Self::ExecuteResponse => b'e',
        }
    }

    /// True for the three uppercase (request) variants.
    pub const fn is_request(self) -> bool {
        matches!(
            self,
            Self::ReadRequest | Self::WriteRequest | Self::ExecuteRequest
        )
    }

    /// The response variant the device will answer this request with.
    pub const fn response(self) -> Self {
        match self {
            Self::ReadRequest | Self::ReadResponse => Self::ReadResponse,
            Self::WriteRequest | Self::WriteResponse => Self::WriteResponse,
            Self::ExecuteRequest | Self::ExecuteResponse => Self::ExecuteResponse,
        }
    }
}

impl TryFrom<u8> for Function {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            b'R' => Ok(Self::ReadRequest),
            b'r' => Ok(Self::ReadResponse),
            b'W' => Ok(Self::WriteRequest),
            b'w' => Ok(Self::WriteResponse),
            b'E' => Ok(Self::ExecuteRequest),
            b'e' => Ok(Self::ExecuteResponse),
            other => Err(ProtocolError::UnknownFunction(other)),
        }
    }
}

impl From<Function> for u8 {
    fn from(value: Function) -> Self {
        value.as_byte()
    }
}

/// Result code carried in the single data byte of a `w` response (spec §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteResult {
    Ok,
    /// The register is a legally relevant parameter and the sealing switch is LOCKed.
    SealProtected,
    ReadOnly,
    /// Incorrect value / out of range.
    InvalidValue,
    /// Anything else means the device failed to write its flash memory.
    FlashError(u8),
}

impl From<u8> for WriteResult {
    fn from(value: u8) -> Self {
        match value {
            b'0' => Self::Ok,
            b'1' => Self::SealProtected,
            b'2' => Self::ReadOnly,
            b'3' => Self::InvalidValue,
            other => Self::FlashError(other),
        }
    }
}

impl WriteResult {
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

impl fmt::Display for WriteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::SealProtected => write!(f, "register is protected by the sealing switch"),
            Self::ReadOnly => write!(f, "read-only register"),
            Self::InvalidValue => write!(f, "incorrect value / out of range"),
            Self::FlashError(c) => write!(f, "flash write error (code {:?})", *c as char),
        }
    }
}

/// Result code carried in the single data byte of an `e` response (spec §5.2).
///
/// Beyond `'0'` and `'1'` the meaning is per-register; `0102h` (set tare) for instance
/// uses `'4'` for "timeout waiting for stability" and `'3'` for "tare above Max1".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecuteResult {
    Ok,
    /// The function is protected by the sealing switch.
    SealProtected,
    /// Register-specific failure code.
    Other(u8),
}

impl From<u8> for ExecuteResult {
    fn from(value: u8) -> Self {
        match value {
            b'0' => Self::Ok,
            b'1' => Self::SealProtected,
            other => Self::Other(other),
        }
    }
}

impl ExecuteResult {
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

impl fmt::Display for ExecuteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::SealProtected => write!(f, "execution is protected by the sealing switch"),
            Self::Other(c) => write!(f, "device error (code {:?})", *c as char),
        }
    }
}
