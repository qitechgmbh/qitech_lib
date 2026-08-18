use super::{DataAddress, DeviceState, ProtocolError, WeighingStatus};
use std::fmt;
use units::Mass;
use units::mass::{gram, kilogram, ounce, pound};

/// Width of the numeric part of a weight field (spec §16.1): 8 right-aligned characters.
const WEIGHT_DIGITS: usize = 8;
/// Width of the unit suffix that follows it.
const WEIGHT_UNIT_CHARS: usize = 2;
/// Full width of a weight field. The spec's byte tables print `D_L = "08"` for registers
/// 0101h–0103h but the prose says `0Ah`; `0Ah` is correct — 8 digits plus 2 unit characters.
pub const WEIGHT_FIELD_LEN: usize = WEIGHT_DIGITS + WEIGHT_UNIT_CHARS;
/// Full width of the 0107h payload: `W`+10, `T`+10, `S`+3.
pub const WEIGHING_REGISTER_LEN: usize = 26;

/// The unit suffix a weight field carries. Never assume it — a device can report grams while
/// its scale definition says kilograms, which is exactly what happens after a factory reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightUnit {
    Gram,
    Kilogram,
    Pound,
    Ounce,
}

impl WeightUnit {
    fn parse(bytes: &[u8], address: u16) -> Result<Self, ProtocolError> {
        match bytes {
            b"g " | b" g" => Ok(Self::Gram),
            b"kg" => Ok(Self::Kilogram),
            b"lb" => Ok(Self::Pound),
            b"oz" => Ok(Self::Ounce),
            _ => Err(ProtocolError::MalformedValue(
                address,
                "unrecognised weighing unit suffix",
            )),
        }
    }

    fn to_mass(self, value: f64) -> Mass {
        match self {
            Self::Gram => Mass::new::<gram>(value),
            Self::Kilogram => Mass::new::<kilogram>(value),
            Self::Pound => Mass::new::<pound>(value),
            Self::Ounce => Mass::new::<ounce>(value),
        }
    }
}

impl fmt::Display for WeightUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gram => write!(f, "g"),
            Self::Kilogram => write!(f, "kg"),
            Self::Pound => write!(f, "lb"),
            Self::Ounce => write!(f, "oz"),
        }
    }
}

/// A weight reading together with the unit the device reported it in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weight {
    pub mass: Mass,
    /// The unit as sent, kept so a UI can echo the device's own formatting.
    pub unit: WeightUnit,
}

impl Weight {
    /// Parse a 10-character weight field: 8 right-aligned digits then a 2-character unit.
    pub fn parse(bytes: &[u8], address: u16) -> Result<Self, ProtocolError> {
        if bytes.len() != WEIGHT_FIELD_LEN {
            return Err(ProtocolError::MalformedValue(
                address,
                "weight field must be 10 characters",
            ));
        }
        let unit = WeightUnit::parse(&bytes[WEIGHT_DIGITS..], address)?;
        let digits = std::str::from_utf8(&bytes[..WEIGHT_DIGITS])
            .map_err(|_| ProtocolError::MalformedValue(address, "weight field is not UTF-8"))?
            .trim();
        let value: f64 = digits
            .parse()
            .map_err(|_| ProtocolError::MalformedValue(address, "weight field is not a number"))?;
        Ok(Self {
            mass: unit.to_mass(value),
            unit,
        })
    }
}

/// Register `0107h` — gross weight, tare and status in a single 26-character payload.
///
/// This is what stream mode sends, and the cheapest way to poll a scale: one round trip
/// instead of the three that 0101h + 0102h + 0104h would cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeighingRegister {
    pub gross: Weight,
    pub tare: Weight,
    pub status: WeighingStatus,
}

impl WeighingRegister {
    pub fn parse(bytes: &[u8]) -> Result<Self, ProtocolError> {
        const ADDRESS: u16 = 0x0107;
        if bytes.len() != WEIGHING_REGISTER_LEN {
            return Err(ProtocolError::MalformedValue(
                ADDRESS,
                "weighing register must be 26 characters",
            ));
        }
        if bytes[0] != b'W' || bytes[11] != b'T' || bytes[22] != b'S' {
            return Err(ProtocolError::MalformedValue(
                ADDRESS,
                "weighing register field markers are not W/T/S",
            ));
        }
        Ok(Self {
            gross: Weight::parse(&bytes[1..11], ADDRESS)?,
            tare: Weight::parse(&bytes[12..22], ADDRESS)?,
            status: WeighingStatus::parse(&bytes[23..26])?,
        })
    }

    /// Net weight, derived as gross − tare.
    ///
    /// Register `0103h` reports the same figure; this saves a round trip when the caller
    /// already has a 0107h reading.
    pub fn net(&self) -> Mass {
        self.gross.mass - self.tare.mass
    }
}

/// A parsed read response, keyed by the register it came from.
#[derive(Debug, Clone, PartialEq)]
pub enum RegisterValue {
    SerialNumber(u32),
    DeviceId(u8),
    HardwareVersion(String),
    SoftwareVersion(String),
    /// `true` when the sealing switch is in the LOCK position.
    Sealed(bool),
    StreamOutputRate(u32),
    DeviceState(DeviceState),
    /// Registers 0101h, 0102h and 0103h.
    Weight(Weight),
    /// Registers 0104h, 0105h and 0106h.
    Flag(bool),
    Weighing(WeighingRegister),
    /// Registers 0110h and 0111h.
    AdcCounts(i64),
    /// Any register this crate does not model, handed back verbatim.
    Raw(Vec<u8>),
}

impl RegisterValue {
    /// Interpret the DATA field of an `r` response according to its register address.
    pub fn parse(address: DataAddress, data: &[u8]) -> Result<Self, ProtocolError> {
        let raw_address = address.as_u16();
        match address {
            DataAddress::SerialNumber => Ok(Self::SerialNumber(parse_decimal(data, raw_address)?)),
            DataAddress::DeviceId => Ok(Self::DeviceId(parse_hex_byte(data, raw_address)?)),
            DataAddress::HardwareVersion => {
                Ok(Self::HardwareVersion(parse_text(data, raw_address)?))
            }
            DataAddress::SoftwareVersion => {
                Ok(Self::SoftwareVersion(parse_text(data, raw_address)?))
            }
            DataAddress::SealingSwitchState => Ok(Self::Sealed(parse_flag(data, raw_address)?)),
            DataAddress::StreamOutputRate => {
                Ok(Self::StreamOutputRate(parse_decimal(data, raw_address)?))
            }
            DataAddress::DeviceState => Ok(Self::DeviceState(DeviceState::from(parse_hex_byte(
                data,
                raw_address,
            )?))),
            DataAddress::GrossWeight | DataAddress::TareValue | DataAddress::NetWeight => {
                Ok(Self::Weight(Weight::parse(data, raw_address)?))
            }
            DataAddress::StabilityIndicator
            | DataAddress::ZeroIndicator
            | DataAddress::ZeroTrackingIndicator => Ok(Self::Flag(parse_flag(data, raw_address)?)),
            DataAddress::WeighingRegister => Ok(Self::Weighing(WeighingRegister::parse(data)?)),
            DataAddress::AdcCounts | DataAddress::AdcCountsFiltered => {
                let text = parse_text(data, raw_address)?;
                let counts = text.trim().parse::<i64>().map_err(|_| {
                    ProtocolError::MalformedValue(raw_address, "ADC counts are not an integer")
                })?;
                Ok(Self::AdcCounts(counts))
            }
            _ => Ok(Self::Raw(data.to_vec())),
        }
    }
}

fn parse_text(data: &[u8], address: u16) -> Result<String, ProtocolError> {
    std::str::from_utf8(data)
        .map(str::to_owned)
        .map_err(|_| ProtocolError::MalformedValue(address, "payload is not UTF-8"))
}

fn parse_decimal(data: &[u8], address: u16) -> Result<u32, ProtocolError> {
    parse_text(data, address)?
        .trim()
        .parse()
        .map_err(|_| ProtocolError::MalformedValue(address, "payload is not a decimal number"))
}

/// A single byte encoded as 2 ASCII hex characters, e.g. the device ID or the state byte.
fn parse_hex_byte(data: &[u8], address: u16) -> Result<u8, ProtocolError> {
    if data.len() != 2 {
        return Err(ProtocolError::MalformedValue(
            address,
            "payload must be 2 hex characters",
        ));
    }
    u8::from_str_radix(
        std::str::from_utf8(data)
            .map_err(|_| ProtocolError::MalformedValue(address, "payload is not UTF-8"))?,
        16,
    )
    .map_err(|_| ProtocolError::MalformedValue(address, "payload is not ASCII hex"))
}

fn parse_flag(data: &[u8], address: u16) -> Result<bool, ProtocolError> {
    match data {
        b"0" => Ok(false),
        b"1" => Ok(true),
        _ => Err(ProtocolError::MalformedValue(
            address,
            "payload must be a single '0' or '1'",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use units::mass::kilogram;

    fn kg(weight: Weight) -> f64 {
        weight.mass.get::<kilogram>()
    }

    #[test]
    fn parses_the_spec_weight_field_example() {
        // Spec §16.1: "  2053.15kg".
        let weight = Weight::parse(b" 2053.15kg", 0x0101).unwrap();
        assert_eq!(weight.unit, WeightUnit::Kilogram);
        assert!((kg(weight) - 2053.15).abs() < 1e-9);
    }

    #[test]
    fn parses_negative_and_gram_fields() {
        let weight = Weight::parse(b"  -12.50g ", 0x0103).unwrap();
        assert_eq!(weight.unit, WeightUnit::Gram);
        assert!((kg(weight) + 0.0125).abs() < 1e-12);
    }

    #[test]
    fn parses_every_unit_suffix() {
        for (bytes, unit) in [
            (b"     1.0g " as &[u8], WeightUnit::Gram),
            (b"     1.0kg", WeightUnit::Kilogram),
            (b"     1.0lb", WeightUnit::Pound),
            (b"     1.0oz", WeightUnit::Ounce),
        ] {
            assert_eq!(Weight::parse(bytes, 0x0101).unwrap().unit, unit);
        }
    }

    #[test]
    fn rejects_malformed_weight_fields() {
        assert!(Weight::parse(b"1.0kg", 0x0101).is_err());
        assert!(Weight::parse(b"     1.0zz", 0x0101).is_err());
        assert!(Weight::parse(b"     a.0kg", 0x0101).is_err());
    }

    #[test]
    fn parses_the_captured_weighing_register() {
        // From the §17 UDP capture: idle scale, then 11.5 g, then 500.0 g.
        let idle = WeighingRegister::parse(b"W     0.0g T     0.0g S015").unwrap();
        assert!((kg(idle.gross)).abs() < 1e-12);
        assert!(idle.status.zero() && idle.status.stable());
        assert!((idle.net().get::<kilogram>()).abs() < 1e-12);

        let loading = WeighingRegister::parse(b"W    11.5g T     0.0g S010").unwrap();
        assert!((kg(loading.gross) - 0.0115).abs() < 1e-12);
        assert!(!loading.status.stable());

        let settled = WeighingRegister::parse(b"W   500.0g T     0.0g S014").unwrap();
        assert!((kg(settled.gross) - 0.5).abs() < 1e-12);
        assert!(settled.status.stable() && !settled.status.zero());
    }

    #[test]
    fn nets_out_the_tare() {
        let reading = WeighingRegister::parse(b"W   500.0g T   140.0g S01E").unwrap();
        assert!((reading.net().get::<kilogram>() - 0.36).abs() < 1e-12);
    }

    #[test]
    fn rejects_malformed_weighing_registers() {
        assert!(WeighingRegister::parse(b"too short").is_err());
        // Field markers swapped.
        assert!(WeighingRegister::parse(b"T     0.0g W     0.0g S015").is_err());
    }

    #[test]
    fn parses_scalar_registers() {
        assert_eq!(
            RegisterValue::parse(DataAddress::SerialNumber, b"1123456724").unwrap(),
            RegisterValue::SerialNumber(1_123_456_724)
        );
        assert_eq!(
            RegisterValue::parse(DataAddress::DeviceId, b"A3").unwrap(),
            RegisterValue::DeviceId(163)
        );
        assert_eq!(
            RegisterValue::parse(DataAddress::SealingSwitchState, b"1").unwrap(),
            RegisterValue::Sealed(true)
        );
        assert_eq!(
            RegisterValue::parse(DataAddress::StabilityIndicator, b"0").unwrap(),
            RegisterValue::Flag(false)
        );
        assert_eq!(
            RegisterValue::parse(DataAddress::AdcCounts, b"-1048576").unwrap(),
            RegisterValue::AdcCounts(-1_048_576)
        );
        assert_eq!(
            RegisterValue::parse(DataAddress::StreamOutputRate, b"500").unwrap(),
            RegisterValue::StreamOutputRate(500)
        );

        let state = RegisterValue::parse(DataAddress::DeviceState, b"87").unwrap();
        assert!(matches!(state, RegisterValue::DeviceState(s) if !s.is_healthy()));

        // Unmodelled registers come back verbatim.
        assert_eq!(
            RegisterValue::parse(DataAddress::Other(0x0202), b"37").unwrap(),
            RegisterValue::Raw(b"37".to_vec())
        );
    }

    #[test]
    fn rejects_malformed_scalar_registers() {
        assert!(RegisterValue::parse(DataAddress::SerialNumber, b"12x4").is_err());
        assert!(RegisterValue::parse(DataAddress::DeviceId, b"A").is_err());
        assert!(RegisterValue::parse(DataAddress::SealingSwitchState, b"2").is_err());
    }
}
