/// Data address codes from the XTREM memory map (spec §7).
///
/// Only the registers this crate actually models are named. Any other address can still be
/// reached with [`DataAddress::Other`], which round-trips the raw `u16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataAddress {
    // -- general settings ------------------------------------------------
    /// Decimal ASCII, 0..=4294967294.
    SerialNumber,
    /// Network address, 2 ASCII hex chars.
    DeviceId,
    HardwareVersion,
    SoftwareVersion,
    /// `'0'` unlocked, `'1'` locked.
    SealingSwitchState,

    // -- serial port / stream mode settings ------------------------------
    BaudRate,
    LrcCheck,
    AddCrLf,
    /// Stream mode interval in milliseconds, decimal ASCII.
    StreamOutputRate,

    // -- weighing --------------------------------------------------------
    /// 2 ASCII hex chars, see [`crate::protocol::DeviceState`].
    DeviceState,
    /// Gross weight, 10 chars.
    GrossWeight,
    /// Tare value, 10 chars. Executing this register tares to the current weight.
    TareValue,
    /// Net weight, 10 chars.
    NetWeight,
    /// `'0'`/`'1'`.
    StabilityIndicator,
    /// `'0'`/`'1'`. Executing this register zeroes the scale.
    ZeroIndicator,
    /// `'0'`/`'1'`.
    ZeroTrackingIndicator,
    /// Gross + tare + status in one 26-char payload. The register stream mode sends.
    WeighingRegister,
    /// Instantaneous ADC counts.
    AdcCounts,
    /// ADC counts after the digital filter.
    AdcCountsFiltered,

    // -- executable functions --------------------------------------------
    StopStream,
    /// Stream `WeighingRegister` (0107h).
    StartStreamWeight,
    /// Stream `AdcCounts` (0110h).
    StartStreamAdc,
    /// Stream `AdcCountsFiltered` (0111h).
    StartStreamAdcFiltered,
    ClearTare,
    DeviceReset,
    FactoryReset,

    /// Any address not modelled above.
    Other(u16),
}

impl DataAddress {
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::SerialNumber => 0x0000,
            Self::DeviceId => 0x0001,
            Self::HardwareVersion => 0x0007,
            Self::SoftwareVersion => 0x0008,
            Self::SealingSwitchState => 0x0009,

            Self::BaudRate => 0x0010,
            Self::LrcCheck => 0x0011,
            Self::AddCrLf => 0x0012,
            Self::StreamOutputRate => 0x0013,

            Self::DeviceState => 0x0100,
            Self::GrossWeight => 0x0101,
            Self::TareValue => 0x0102,
            Self::NetWeight => 0x0103,
            Self::StabilityIndicator => 0x0104,
            Self::ZeroIndicator => 0x0105,
            Self::ZeroTrackingIndicator => 0x0106,
            Self::WeighingRegister => 0x0107,
            Self::AdcCounts => 0x0110,
            Self::AdcCountsFiltered => 0x0111,

            Self::StopStream => 0x1010,
            Self::StartStreamWeight => 0x1011,
            Self::StartStreamAdc => 0x1012,
            Self::StartStreamAdcFiltered => 0x1013,
            Self::ClearTare => 0x1103,
            Self::DeviceReset => 0x9999,
            Self::FactoryReset => 0xEEEE,

            Self::Other(raw) => raw,
        }
    }
}

impl From<u16> for DataAddress {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => Self::SerialNumber,
            0x0001 => Self::DeviceId,
            0x0007 => Self::HardwareVersion,
            0x0008 => Self::SoftwareVersion,
            0x0009 => Self::SealingSwitchState,

            0x0010 => Self::BaudRate,
            0x0011 => Self::LrcCheck,
            0x0012 => Self::AddCrLf,
            0x0013 => Self::StreamOutputRate,

            0x0100 => Self::DeviceState,
            0x0101 => Self::GrossWeight,
            0x0102 => Self::TareValue,
            0x0103 => Self::NetWeight,
            0x0104 => Self::StabilityIndicator,
            0x0105 => Self::ZeroIndicator,
            0x0106 => Self::ZeroTrackingIndicator,
            0x0107 => Self::WeighingRegister,
            0x0110 => Self::AdcCounts,
            0x0111 => Self::AdcCountsFiltered,

            0x1010 => Self::StopStream,
            0x1011 => Self::StartStreamWeight,
            0x1012 => Self::StartStreamAdc,
            0x1013 => Self::StartStreamAdcFiltered,
            0x1103 => Self::ClearTare,
            0x9999 => Self::DeviceReset,
            0xEEEE => Self::FactoryReset,

            other => Self::Other(other),
        }
    }
}

impl From<DataAddress> for u16 {
    fn from(value: DataAddress) -> Self {
        value.as_u16()
    }
}
