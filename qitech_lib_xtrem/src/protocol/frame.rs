use super::{DataAddress, Function, ProtocolError};

/// Start of message.
pub const STX: u8 = 0x02;
/// End of message.
pub const ETX: u8 = 0x03;
/// Destination address that every device on the network answers (spec §5.1).
pub const BROADCAST_ID: u8 = 0xFF;

/// Number of ASCII characters between STX and ETX in a frame carrying no data:
/// `ID_O`(2) + `ID_D`(2) + `F`(1) + `D_ADDRESS`(4) + `D_L`(2) + `LRC`(2).
const MIN_BODY_LEN: usize = 13;

/// `D_L` is two hex characters, so DATA can never be longer than this.
pub const MAX_DATA_LEN: usize = u8::MAX as usize;

/// Longest datagram the protocol can produce: STX + body + the largest possible DATA + ETX,
/// plus the optional CR+LF terminator. Nothing legal exceeds this, so it is the size a
/// receive buffer needs to be.
pub const MAX_FRAME_LEN: usize = 1 + MIN_BODY_LEN + MAX_DATA_LEN + 1 + 2;

/// Offset of the first `D_L` character within the body.
const DATA_LENGTH_OFFSET: usize = 9;
/// Offset of the first DATA byte within the body.
const DATA_OFFSET: usize = 11;

/// A decoded XTREM message. Requests and responses share one structure (spec §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// `ID_O` — network address of the sender.
    pub id_origin: u8,
    /// `ID_D` — network address of the destination, or [`BROADCAST_ID`].
    pub id_dest: u8,
    pub function: Function,
    /// `D_ADDRESS` — the register the function acts on.
    pub address: DataAddress,
    /// `DATA` — raw ASCII, *not* hex-encoded.
    pub data: Vec<u8>,
}

impl Frame {
    /// A read request for `address`, which never carries data.
    pub fn read(id_origin: u8, id_dest: u8, address: DataAddress) -> Self {
        Self {
            id_origin,
            id_dest,
            function: Function::ReadRequest,
            address,
            data: Vec::new(),
        }
    }

    /// A write request. `data` is the raw ASCII payload, e.g. `b"0500"` for 500 ms.
    pub fn write(id_origin: u8, id_dest: u8, address: DataAddress, data: Vec<u8>) -> Self {
        Self {
            id_origin,
            id_dest,
            function: Function::WriteRequest,
            address,
            data,
        }
    }

    /// An execute request for `address`, which never carries data.
    pub fn execute(id_origin: u8, id_dest: u8, address: DataAddress) -> Self {
        Self {
            id_origin,
            id_dest,
            function: Function::ExecuteRequest,
            address,
            data: Vec::new(),
        }
    }

    /// Serialise into `buf`, which is cleared first.
    ///
    /// `crlf` appends the CR+LF terminator. The Wi-Fi / Ethernet module requires it
    /// (spec §17) and it is the factory default for the serial port too (register `0012h`).
    /// It sits outside the frame and is excluded from the LRC.
    pub fn encode(&self, buf: &mut Vec<u8>, crlf: bool) {
        buf.clear();
        buf.push(STX);

        let body_start = buf.len();
        push_hex_u8(buf, self.id_origin);
        push_hex_u8(buf, self.id_dest);
        buf.push(self.function.as_byte());
        push_hex_u16(buf, self.address.as_u16());
        // The data length is an 8-bit field, so payloads never exceed 255 bytes.
        push_hex_u8(buf, self.data.len() as u8);
        buf.extend_from_slice(&self.data);

        let checksum = lrc(&buf[body_start..]);
        push_hex_u8(buf, checksum);

        buf.push(ETX);
        if crlf {
            buf.extend_from_slice(b"\r\n");
        }
    }

    /// Convenience wrapper around [`Frame::encode`] for callers that want an owned buffer.
    pub fn to_bytes(&self, crlf: bool) -> Vec<u8> {
        let mut buf = Vec::with_capacity(MIN_BODY_LEN + self.data.len() + 4);
        self.encode(&mut buf, crlf);
        buf
    }

    /// Parse the first complete frame in `bytes`.
    ///
    /// Anything before STX or after ETX is ignored, which is what the spec asks receivers to
    /// do (§4) and which absorbs the trailing CR+LF for free. Set `verify_lrc` to `false` when
    /// talking to a device that has LRC checking switched off (register `0011h`) — the field is
    /// still parsed and length-checked, only the comparison is skipped.
    pub fn decode(bytes: &[u8], verify_lrc: bool) -> Result<Self, ProtocolError> {
        let stx = bytes
            .iter()
            .position(|&b| b == STX)
            .ok_or(ProtocolError::MissingStx)?;
        let etx = bytes[stx + 1..]
            .iter()
            .position(|&b| b == ETX)
            .ok_or(ProtocolError::MissingEtx)?
            + stx
            + 1;

        let body = &bytes[stx + 1..etx];
        if body.len() < MIN_BODY_LEN {
            return Err(ProtocolError::TooShort(body.len()));
        }

        let id_origin = parse_hex_u8(&body[0..2], "ID_O")?;
        let id_dest = parse_hex_u8(&body[2..4], "ID_D")?;
        let function = Function::try_from(body[4])?;
        let address = parse_hex_u16(&body[5..9], "D_ADDRESS")?;
        let declared_len = parse_hex_u8(&body[DATA_LENGTH_OFFSET..DATA_OFFSET], "D_L")?;

        // Everything between the data length field and the trailing LRC is payload.
        let actual_len = body.len() - MIN_BODY_LEN;
        if usize::from(declared_len) != actual_len {
            return Err(ProtocolError::DataLengthMismatch(declared_len, actual_len));
        }

        let data = &body[DATA_OFFSET..DATA_OFFSET + actual_len];
        if let Some(&bad) = data.iter().find(|&&b| b < 0x20) {
            return Err(ProtocolError::IllegalDataByte(bad));
        }

        let lrc_field = &body[body.len() - 2..];
        let received = parse_hex_u8(lrc_field, "LRC")?;
        if verify_lrc {
            let computed = lrc(&body[..body.len() - 2]);
            if computed != received {
                return Err(ProtocolError::LrcMismatch(computed, received));
            }
        }

        Ok(Self {
            id_origin,
            id_dest,
            function,
            address: DataAddress::from(address),
            data: data.to_vec(),
        })
    }
}

/// Longitudinal redundancy check: XOR of every byte from `ID_O` through the last DATA byte
/// (spec §5.6). STX, ETX, the LRC characters themselves and any CR+LF are excluded.
pub fn lrc(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, &b| acc ^ b)
}

fn push_hex_u8(buf: &mut Vec<u8>, value: u8) {
    buf.push(hex_nibble(value >> 4));
    buf.push(hex_nibble(value & 0x0F));
}

fn push_hex_u16(buf: &mut Vec<u8>, value: u16) {
    push_hex_u8(buf, (value >> 8) as u8);
    push_hex_u8(buf, (value & 0x00FF) as u8);
}

const fn hex_nibble(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'A' + (nibble - 10),
    }
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        // Devices emit uppercase, but accept lowercase on the way in.
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn parse_hex_u8(bytes: &[u8], field: &'static str) -> Result<u8, ProtocolError> {
    debug_assert_eq!(bytes.len(), 2);
    match (hex_value(bytes[0]), hex_value(bytes[1])) {
        (Some(hi), Some(lo)) => Ok((hi << 4) | lo),
        _ => Err(ProtocolError::NotHex(
            field,
            String::from_utf8_lossy(bytes).into_owned(),
        )),
    }
}

fn parse_hex_u16(bytes: &[u8], field: &'static str) -> Result<u16, ProtocolError> {
    debug_assert_eq!(bytes.len(), 4);
    let hi = parse_hex_u8(&bytes[0..2], field)?;
    let lo = parse_hex_u8(&bytes[2..4], field)?;
    Ok((u16::from(hi) << 8) | u16::from(lo))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Execute response captured in spec §17, device 01 → host 00, register 1011h.
    const EXECUTE_RESPONSE: &[u8] = &[
        0x02, 0x30, 0x31, 0x30, 0x30, 0x65, 0x31, 0x30, 0x31, 0x31, 0x30, 0x31, 0x30, 0x35, 0x34,
        0x03, 0x0D, 0x0A,
    ];

    /// Weighing register response from the same capture: `W   0.0g T   0.0g S015`.
    const WEIGHING_RESPONSE: &[u8] = b"\x020100r01071AW     0.0g T     0.0g S01561\x03\r\n";

    /// The host request from the same capture. Its LRC field is `00` where the correct
    /// value is `45` — the PC in that trace had LRC generation switched off.
    const BAD_LRC_REQUEST: &[u8] = b"\x020001E10110000\x03\r\n";

    #[test]
    fn lrc_matches_captured_frames() {
        assert_eq!(lrc(b"0100e1011010"), 0x54);
        assert_eq!(lrc(b"0100r01071AW     0.0g T     0.0g S015"), 0x61);
        assert_eq!(lrc(b"0001E101100"), 0x45);
    }

    #[test]
    fn decodes_captured_execute_response() {
        let frame = Frame::decode(EXECUTE_RESPONSE, true).unwrap();
        assert_eq!(frame.id_origin, 0x01);
        assert_eq!(frame.id_dest, 0x00);
        assert_eq!(frame.function, Function::ExecuteResponse);
        assert_eq!(frame.address, DataAddress::StartStreamWeight);
        assert_eq!(frame.data, b"0");
    }

    #[test]
    fn decodes_captured_weighing_response() {
        let frame = Frame::decode(WEIGHING_RESPONSE, true).unwrap();
        assert_eq!(frame.id_origin, 0x01);
        assert_eq!(frame.function, Function::ReadResponse);
        assert_eq!(frame.address, DataAddress::WeighingRegister);
        assert_eq!(frame.data.len(), 0x1A);
        assert_eq!(frame.data, b"W     0.0g T     0.0g S015");
    }

    #[test]
    fn rejects_bad_lrc_but_accepts_it_when_checking_is_off() {
        assert_eq!(
            Frame::decode(BAD_LRC_REQUEST, true),
            Err(ProtocolError::LrcMismatch(0x45, 0x00))
        );
        let frame = Frame::decode(BAD_LRC_REQUEST, false).unwrap();
        assert_eq!(frame.function, Function::ExecuteRequest);
        assert_eq!(frame.address, DataAddress::StartStreamWeight);
    }

    #[test]
    fn encodes_the_spec_read_request_example() {
        // Spec §5.4: device 23 (0x17) asks device 1 to read register 0101h.
        let frame = Frame::read(0x17, 0x01, DataAddress::GrossWeight);
        let bytes = frame.to_bytes(false);
        assert_eq!(&bytes[..12], b"\x021701R010100");
        assert_eq!(*bytes.last().unwrap(), ETX);
    }

    #[test]
    fn encodes_the_spec_stream_rate_write_example() {
        // Spec §9.4: device 00 writes "500" ms to register 0013h of device 01, LRC 62.
        let frame = Frame::write(0x00, 0x01, DataAddress::StreamOutputRate, b"500".to_vec());
        assert_eq!(frame.to_bytes(false), b"\x020001W00130350062\x03".to_vec());
    }

    #[test]
    fn decodes_the_spec_stream_rate_write_response() {
        // Spec §9.4: device 01 answers the write above with result '0', LRC 45.
        let frame = Frame::decode(b"\x020100w001301045\x03", true).unwrap();
        assert_eq!(frame.function, Function::WriteResponse);
        assert_eq!(frame.address, DataAddress::StreamOutputRate);
        assert_eq!(frame.data, b"0");
    }

    #[test]
    fn round_trips_every_function() {
        let cases = [
            Frame::read(0x17, 0xFF, DataAddress::SerialNumber),
            Frame::write(0x00, 0x01, DataAddress::DeviceId, b"0E".to_vec()),
            Frame::execute(0xA3, 0x01, DataAddress::FactoryReset),
        ];
        for original in cases {
            for crlf in [false, true] {
                let bytes = original.to_bytes(crlf);
                assert_eq!(Frame::decode(&bytes, true).unwrap(), original);
            }
        }
    }

    #[test]
    fn ignores_noise_around_the_frame() {
        let mut noisy = b"garbage before ".to_vec();
        noisy.extend_from_slice(WEIGHING_RESPONSE);
        noisy.extend_from_slice(b"and after");
        assert_eq!(
            Frame::decode(&noisy, true).unwrap(),
            Frame::decode(WEIGHING_RESPONSE, true).unwrap()
        );
    }

    #[test]
    fn rejects_malformed_frames() {
        assert_eq!(
            Frame::decode(b"no stx here", true),
            Err(ProtocolError::MissingStx)
        );
        assert_eq!(
            Frame::decode(b"\x020100r0107", true),
            Err(ProtocolError::MissingEtx)
        );
        assert_eq!(
            Frame::decode(b"\x020100r\x03", true),
            Err(ProtocolError::TooShort(5))
        );

        // D_L claims two data bytes but only one is present.
        assert!(matches!(
            Frame::decode(b"\x020100r010702X61\x03", true),
            Err(ProtocolError::DataLengthMismatch(2, 1))
        ));

        // Non-hex register address.
        assert!(matches!(
            Frame::decode(b"\x020100r01Z70000\x03", true),
            Err(ProtocolError::NotHex("D_ADDRESS", _))
        ));

        // Unknown function character.
        assert_eq!(
            Frame::decode(b"\x020100X01070000\x03", true),
            Err(ProtocolError::UnknownFunction(b'X'))
        );

        // Control character inside the data field.
        assert_eq!(
            Frame::decode(b"\x020100r010701\x0700\x03", true),
            Err(ProtocolError::IllegalDataByte(0x07))
        );
    }
}
