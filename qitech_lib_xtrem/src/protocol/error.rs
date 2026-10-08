/// Everything that can go wrong while encoding or decoding an XTREM frame.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolError {
    /// No STX (0x02) byte in the datagram.
    #[error("no STX (0x02) in datagram")]
    MissingStx,
    /// No ETX (0x03) byte after the STX.
    #[error("no ETX (0x03) after STX")]
    MissingEtx,
    /// Frame is shorter than the 13 mandatory ASCII characters between STX and ETX.
    /// (index: got)
    #[error("frame body is {0} bytes, needs at least 13")]
    TooShort(usize),
    /// A field that must be ASCII hex contained something else.
    /// (field name, offending bytes rendered lossily)
    #[error("field {0} is not ASCII hex: {1:?}")]
    NotHex(&'static str, String),
    /// The function character was not one of R/r/W/w/E/e.
    #[error("unknown function character {:?}", *.0 as char)]
    UnknownFunction(u8),
    /// The `D_L` field disagrees with the number of bytes actually present.
    /// (declared, actual)
    #[error("D_L declares {0} data bytes, frame carries {1}")]
    DataLengthMismatch(u8, usize),
    /// LRC checking was enabled and the checksum did not match.
    /// (expected, got)
    #[error("LRC mismatch: computed {0:02X}, frame carries {1:02X}")]
    LrcMismatch(u8, u8),
    /// The data field contained a byte outside the legal 0x20..=0xFF range.
    #[error("data byte {0:#04X} is outside the legal 0x20..=0xFF range")]
    IllegalDataByte(u8),
    /// The payload did not have the shape the register's documentation promises.
    /// (register address, reason)
    #[error("malformed payload for register {0:04X}h: {1}")]
    MalformedValue(u16, &'static str),
    /// A write request was rejected by the device.
    #[error("device rejected write: {0}")]
    WriteRejected(super::WriteResult),
    /// An execute request was rejected by the device.
    #[error("device rejected execute: {0}")]
    ExecuteRejected(super::ExecuteResult),
}
