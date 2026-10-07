use super::EthercatDeviceProcessing;
use super::{NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::analog_input::physical::AnalogInputRange;
use crate::io::analog_input::{AnalogInputDevice, AnalogInputInput};
use crate::pdo::RxPdo;
use crate::pdo::TxPdo;
use crate::{
    coe::{ConfigurableDevice, Configuration},
    helpers::signing_converter_u16::U16SigningConverter,
    pdo::{
        PredefinedPdoAssignment,
        analog_input::{AiCompact, AiStandard},
    },
    shared_config::el30xx::{EL30XXChannelConfiguration, EL30XXPresentation},
};
use common::SdoRequest;
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};
use units::electric_current::milliampere;
use units::f64::ElectricCurrent;

#[derive(Debug, Clone)]
pub struct EL3021Configuration {
    pub pdo_assignment: EL3021PredefinedPdoAssignment,
    // Input1+ and Input1-
    pub channel1: EL30XXChannelConfiguration,
}

#[derive(EthercatDevice)]
pub struct EL3021 {
    pub configuration: EL3021Configuration,
    pub txpdo: EL3021TxPdo,
    pub rxpdo: EL3021RxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL3021 {}

impl std::fmt::Debug for EL3021 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3021")
    }
}

impl Default for EL3021PredefinedPdoAssignment {
    fn default() -> Self {
        Self::Standard
    }
}

impl Default for EL3021Configuration {
    fn default() -> Self {
        Self {
            pdo_assignment: EL3021PredefinedPdoAssignment::Standard,
            channel1: EL30XXChannelConfiguration::default(),
        }
    }
}

impl NewEthercatDevice for EL3021 {
    fn new() -> Self {
        let configuration = EL3021Configuration::default();
        Self {
            txpdo: configuration.txpdo_assignment(),
            rxpdo: configuration.rxpdo_assignment(),
            configuration,
            is_used: false,
        }
    }
}

impl AnalogInputDevice for EL3021 {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error> {
        let raw_value = match port {
            0 => match &self.txpdo {
                EL3021TxPdo {
                    ai_standard_channel1: Some(ai_standard),
                    ..
                } => ai_standard.value,
                EL3021TxPdo {
                    ai_compact_channel1: Some(ai_compact),
                    ..
                } => ai_compact.value,
                _ => panic!("EL3001 only has one port"),
            },
            _ => return Err(anyhow::anyhow!("EL3001 Only has ONE port")),
        };
        let channel_config = &self.configuration.channel1;
        let raw_value = U16SigningConverter::load_raw(raw_value);
        let value: i16 = match channel_config.presentation {
            EL30XXPresentation::Unsigned => raw_value.as_unsigned() as i16,
            EL30XXPresentation::Signed => raw_value.as_signed(),
            EL30XXPresentation::SignedMagnitude => raw_value.as_signed_magnitude(),
        };
        let normalized = f32::from(value) / f32::from(i16::MAX);
        Ok(AnalogInputInput {
            normalized,
            wiring_error: false,
        })
    }

    fn get_port_count(&self) -> usize {
        1
    }

    fn analog_input_range(&self) -> AnalogInputRange {
        AnalogInputRange::Current {
            min: ElectricCurrent::new::<milliampere>(4.0),
            max: ElectricCurrent::new::<milliampere>(20.0),
            min_raw: 0,
            max_raw: i16::MAX,
        }
    }
}

impl ConfigurableDevice for EL3021 {
    type Config = EL3021Configuration;

    fn set_config(&mut self, config: EL3021Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.rxpdo = config.rxpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL3021Configuration {
        &self.configuration
    }
}

#[derive(Debug, Clone)]
pub enum EL3021Port {
    AI1,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL3021TxPdo {
    #[pdo_object_index(0x1A00)]
    pub ai_standard_channel1: Option<AiStandard>,
    #[pdo_object_index(0x1A01)]
    pub ai_compact_channel1: Option<AiCompact>,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL3021RxPdo {}

impl Configuration for EL3021Configuration {
    type TxPdo = EL3021TxPdo;
    type RxPdo = EL3021RxPdo;

    fn txpdo_assignment(&self) -> EL3021TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL3021RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        // Channel 1
        Ok(self.channel1.get_channel_coe_writes(device_address, 0x8000))
    }
}

#[derive(Debug, Clone)]
pub enum EL3021PredefinedPdoAssignment {
    Standard,
    Compact,
}

impl PredefinedPdoAssignment<EL3021TxPdo, EL3021RxPdo> for EL3021PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL3021TxPdo {
        match self {
            Self::Standard => EL3021TxPdo {
                ai_standard_channel1: Some(AiStandard::default()),
                ai_compact_channel1: None,
            },
            Self::Compact => EL3021TxPdo {
                ai_standard_channel1: None,
                ai_compact_channel1: Some(AiCompact::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) -> EL3021RxPdo {
        match self {
            Self::Standard => EL3021RxPdo {},
            Self::Compact => EL3021RxPdo {},
        }
    }
}

pub const EL3021_VENDOR_ID: u32 = 2;
pub const EL3021_PRODUCT_ID: u32 = 197996626;
pub const EL3021_REVISION_A: u32 = 1310720;
pub const EL3021_IDENTITY_A: SubDeviceIdentityTuple =
    (EL3021_VENDOR_ID, EL3021_PRODUCT_ID, EL3021_REVISION_A);
