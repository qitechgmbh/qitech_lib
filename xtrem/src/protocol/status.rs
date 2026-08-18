use super::ProtocolError;
use std::fmt;

/// Weighing error code carried in bits 0..=4 of the device state byte (spec §8.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeighingError {
    None,
    E2promRead,
    AdcDead,
    AdcOutOfRange,
    AdcAbove30Mv,
    AdcBelowMinus30Mv,
    /// Load cell supply out of range; the device shut the supply down to protect the regulator.
    LoadCellSupply,
    /// Weight > Max + 9e.
    Overload,
    /// Weight < -19e.
    NegativeWeight,
    Unknown(u8),
}

impl From<u8> for WeighingError {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::None,
            0x01 => Self::E2promRead,
            0x02 => Self::AdcDead,
            0x03 => Self::AdcOutOfRange,
            0x04 => Self::AdcAbove30Mv,
            0x05 => Self::AdcBelowMinus30Mv,
            0x06 => Self::LoadCellSupply,
            0x07 => Self::Overload,
            0x08 => Self::NegativeWeight,
            other => Self::Unknown(other),
        }
    }
}

impl WeighingError {
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}

impl fmt::Display for WeighingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "no error"),
            Self::E2promRead => write!(f, "error reading the E2PROM settings"),
            Self::AdcDead => write!(f, "ADC (load cell) does not work"),
            Self::AdcOutOfRange => write!(f, "ADC input signal out of range"),
            Self::AdcAbove30Mv => write!(f, "ADC input signal > 30mV"),
            Self::AdcBelowMinus30Mv => write!(f, "ADC input signal < -30mV"),
            Self::LoadCellSupply => write!(f, "load cell supply out of range, output shut down"),
            Self::Overload => write!(f, "overload (weight > Max + 9e)"),
            Self::NegativeWeight => write!(f, "negative weight (weight < -19e)"),
            Self::Unknown(c) => write!(f, "unknown weighing error code {c:#04X}"),
        }
    }
}

/// State of the optional Wi-Fi module, bits 6..=7 of the device state byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    NotPresent,
    Ready,
    Connected,
    ConnectionError,
}

impl From<u8> for WifiState {
    fn from(value: u8) -> Self {
        match value & 0b11 {
            0 => Self::NotPresent,
            1 => Self::Ready,
            2 => Self::Connected,
            _ => Self::ConnectionError,
        }
    }
}

/// Register `0100h` — device state information (spec §8.6).
///
/// The device sends this unprompted as a broadcast right after boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceState {
    pub raw: u8,
    pub weighing_error: WeighingError,
    /// Vcc out < 5.8V or Vcc in > 8.5V.
    pub power_alarm: bool,
    pub wifi: WifiState,
}

impl From<u8> for DeviceState {
    fn from(raw: u8) -> Self {
        Self {
            raw,
            weighing_error: WeighingError::from(raw & 0b0001_1111),
            power_alarm: raw & 0b0010_0000 != 0,
            wifi: WifiState::from(raw >> 6),
        }
    }
}

impl DeviceState {
    /// True when neither the weighing chain nor the power supply reports a fault.
    pub const fn is_healthy(&self) -> bool {
        self.weighing_error.is_none() && !self.power_alarm
    }
}

/// Register `0107h` status field — a 12-bit word sent as 3 ASCII hex characters (spec §16.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeighingStatus {
    pub raw: u16,
}

impl WeighingStatus {
    /// Bit 0 — weight is within ±1/4e of zero.
    pub const fn zero(self) -> bool {
        self.bit(0)
    }
    /// Bit 1 — the tare device is on.
    pub const fn tare(self) -> bool {
        self.bit(1)
    }
    /// Bit 2 — the weight reading is stable.
    pub const fn stable(self) -> bool {
        self.bit(2)
    }
    /// Bit 3 — the displayed value is a net weight.
    pub const fn net(self) -> bool {
        self.bit(3)
    }
    /// Bit 4 — tare mode: `false` normal, `true` fixed.
    pub const fn fixed_tare(self) -> bool {
        self.bit(4)
    }
    /// Bit 5 — high resolution mode is on.
    pub const fn high_resolution(self) -> bool {
        self.bit(5)
    }
    /// Bit 6 — the device is performing its initial zero after start-up.
    pub const fn initial_zero(self) -> bool {
        self.bit(6)
    }
    /// Bit 7 — weight > Max + 9e.
    pub const fn overload(self) -> bool {
        self.bit(7)
    }
    /// Bit 8 — weight < -19e.
    pub const fn negative_weight(self) -> bool {
        self.bit(8)
    }
    /// Bit 9 — active range on a multi-range instrument: `false` range 1, `true` range 2.
    pub const fn range_2(self) -> bool {
        self.bit(9)
    }
    /// Bit 10 — a pre-set tare is in operation.
    pub const fn preset_tare(self) -> bool {
        self.bit(10)
    }

    const fn bit(self, index: u8) -> bool {
        self.raw & (1 << index) != 0
    }

    /// Parse the 3 ASCII hex characters of the `S` field.
    pub fn parse(bytes: &[u8]) -> Result<Self, ProtocolError> {
        const ADDRESS: u16 = 0x0107;
        if bytes.len() != 3 {
            return Err(ProtocolError::MalformedValue(
                ADDRESS,
                "status field must be 3 characters",
            ));
        }
        let mut raw = 0u16;
        for &byte in bytes {
            let nibble = match byte {
                b'0'..=b'9' => byte - b'0',
                b'A'..=b'F' => byte - b'A' + 10,
                b'a'..=b'f' => byte - b'a' + 10,
                _ => {
                    return Err(ProtocolError::MalformedValue(
                        ADDRESS,
                        "status field is not ASCII hex",
                    ));
                }
            };
            raw = (raw << 4) | u16::from(nibble);
        }
        Ok(Self { raw })
    }
}

impl From<u16> for WeighingStatus {
    fn from(raw: u16) -> Self {
        Self { raw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_spec_device_state_examples() {
        // Spec §8.6 output examples.
        let ok = DeviceState::from(0x00);
        assert!(ok.is_healthy());
        assert_eq!(ok.wifi, WifiState::NotPresent);

        assert_eq!(DeviceState::from(0x40).wifi, WifiState::Ready);
        assert_eq!(DeviceState::from(0x80).wifi, WifiState::Connected);
        assert_eq!(DeviceState::from(0xC0).wifi, WifiState::ConnectionError);

        let power = DeviceState::from(0xA0);
        assert!(power.power_alarm);
        assert!(!power.is_healthy());
        assert_eq!(power.wifi, WifiState::Connected);

        let adc = DeviceState::from(0x03);
        assert_eq!(adc.weighing_error, WeighingError::AdcOutOfRange);
        assert_eq!(adc.wifi, WifiState::NotPresent);

        let overload = DeviceState::from(0x87);
        assert_eq!(overload.weighing_error, WeighingError::Overload);
        assert!(!overload.power_alarm);
        assert_eq!(overload.wifi, WifiState::Connected);
    }

    #[test]
    fn decodes_the_captured_status_words() {
        // From the §17 UDP capture: idle at zero, stable, fixed tare mode.
        let idle = WeighingStatus::parse(b"015").unwrap();
        assert!(idle.zero());
        assert!(idle.stable());
        assert!(idle.fixed_tare());
        assert!(!idle.overload());
        assert!(!idle.net());

        // Mid-load, moving: neither zero nor stable.
        let moving = WeighingStatus::parse(b"010").unwrap();
        assert!(!moving.zero());
        assert!(!moving.stable());
        assert!(moving.fixed_tare());

        // Settled at 500 g: stable but no longer at zero.
        let settled = WeighingStatus::parse(b"014").unwrap();
        assert!(!settled.zero());
        assert!(settled.stable());
    }

    #[test]
    fn rejects_malformed_status_words() {
        assert!(WeighingStatus::parse(b"01").is_err());
        assert!(WeighingStatus::parse(b"0Z1").is_err());
    }
}
