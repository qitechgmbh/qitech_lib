use std::fmt;

/// Everything that can go wrong while encoding or decoding an XTREM frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// No STX (0x02) byte in the datagram.
    MissingStx,
    /// No ETX (0x03) byte after the STX.
    MissingEtx,
    /// Frame is shorter than the 13 mandatory ASCII characters between STX and ETX.
    /// (index: got)
    TooShort(usize),
    /// A field that must be ASCII hex contained something else.
    /// (field name, offending bytes rendered lossily)
    NotHex(&'static str, String),
    /// The function character was not one of R/r/W/w/E/e.
    UnknownFunction(u8),
    /// The `D_L` field disagrees with the number of bytes actually present.
    /// (declared, actual)
    DataLengthMismatch(u8, usize),
    /// LRC checking was enabled and the checksum did not match.
    /// (expected, got)
    LrcMismatch(u8, u8),
    /// The data field contained a byte outside the legal 0x20..=0xFF range.
    IllegalDataByte(u8),
    /// The payload did not have the shape the register's documentation promises.
    /// (register address, reason)
    MalformedValue(u16, &'static str),
    /// A write request was rejected by the device.
    WriteRejected(super::WriteResult),
    /// An execute request was rejected by the device.
    ExecuteRejected(super::ExecuteResult),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStx => write!(f, "no STX (0x02) in datagram"),
            Self::MissingEtx => write!(f, "no ETX (0x03) after STX"),
            Self::TooShort(got) => {
                write!(f, "frame body is {got} bytes, needs at least 13")
            }
            Self::NotHex(field, raw) => write!(f, "field {field} is not ASCII hex: {raw:?}"),
            Self::UnknownFunction(b) => write!(f, "unknown function character {:?}", *b as char),
            Self::DataLengthMismatch(declared, actual) => {
                write!(
                    f,
                    "D_L declares {declared} data bytes, frame carries {actual}"
                )
            }
            Self::LrcMismatch(expected, got) => {
                write!(
                    f,
                    "LRC mismatch: computed {expected:02X}, frame carries {got:02X}"
                )
            }
            Self::IllegalDataByte(b) => {
                write!(
                    f,
                    "data byte {b:#04X} is outside the legal 0x20..=0xFF range"
                )
            }
            Self::MalformedValue(addr, why) => {
                write!(f, "malformed payload for register {addr:04X}h: {why}")
            }
            Self::WriteRejected(r) => write!(f, "device rejected write: {r}"),
            Self::ExecuteRejected(r) => write!(f, "device rejected execute: {r}"),
        }
    }
}

impl std::error::Error for ProtocolError {}
