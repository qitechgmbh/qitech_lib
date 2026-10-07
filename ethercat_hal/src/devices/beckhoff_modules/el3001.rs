use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::analog_input::{AnalogInputDevice, AnalogInputInput};
use crate::{
    coe::{ConfigurableDevice, Configuration},
    helpers::signing_converter_u16::U16SigningConverter,
    io::analog_input::physical::AnalogInputRange,
    pdo::{
        PredefinedPdoAssignment, TxPdo,
        analog_input::{AiCompact, AiStandard},
    },
    shared_config::el30xx::{EL30XXChannelConfiguration, EL30XXPresentation},
};
use common::SdoRequest;
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};
use units::{electric_potential::volt, f64::ElectricPotential};

#[derive(EthercatDevice)]
pub struct EL3001 {
    pub txpdo: EL3001TxPdo,
    pub configuration: EL3001Configuration,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL3001 {}

impl std::fmt::Debug for EL3001 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3001")
    }
}

impl Default for EL3001PredefinedPdoAssignment {
    fn default() -> Self {
        Self::Standard
    }
}

impl NewEthercatDevice for EL3001 {
    fn new() -> Self {
        let configuration: EL3001Configuration = EL3001Configuration::default();
        Self {
            txpdo: configuration.txpdo_assignment(),
            configuration,
            is_used: false,
        }
    }
}

impl AnalogInputDevice for EL3001 {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error> {
        let raw_value = match port {
            0 => match &self.txpdo {
                EL3001TxPdo {
                    ai_standard: Some(ai_standard),
                    ..
                } => ai_standard.value,
                EL3001TxPdo {
                    ai_compact: Some(ai_compact),
                    ..
                } => ai_compact.value,
                _ => panic!("EL3001 only has one port"),
            },
            _ => return Err(anyhow::anyhow!("EL3001 Only has ONE port")),
        };
        let channel_config = &self.configuration.channel_1;
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

    fn analog_input_range(&self) -> AnalogInputRange {
        AnalogInputRange::Potential {
            min: ElectricPotential::new::<volt>(-10.0),
            max: ElectricPotential::new::<volt>(10.0),
            min_raw: i16::MIN,
            max_raw: i16::MAX,
        }
    }

    fn get_port_count(&self) -> usize {
        1
    }
}

impl ConfigurableDevice for EL3001 {
    type Config = EL3001Configuration;

    fn set_config(&mut self, config: EL3001Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL3001Configuration {
        &self.configuration
    }
}

#[derive(Debug, Clone)]
pub enum EL3001Port {
    AI1,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL3001TxPdo {
    #[pdo_object_index(0x1A00)]
    pub ai_standard: Option<AiStandard>,
    #[pdo_object_index(0x1A01)]
    pub ai_compact: Option<AiCompact>,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL3001RxPdo {}

#[derive(Debug, Clone, Default)]
pub struct EL3001Configuration {
    pub pdo_assignment: EL3001PredefinedPdoAssignment,
    pub channel_1: EL30XXChannelConfiguration,
}

impl Configuration for EL3001Configuration {
    type TxPdo = EL3001TxPdo;
    type RxPdo = EL3001RxPdo;

    fn txpdo_assignment(&self) -> EL3001TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL3001RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        Ok(self
            .channel_1
            .get_channel_coe_writes(device_address, 0x8000))
    }
}

#[derive(Debug, Clone)]
pub enum EL3001PredefinedPdoAssignment {
    Standard,
    Compact,
}

impl PredefinedPdoAssignment<EL3001TxPdo, EL3001RxPdo> for EL3001PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL3001TxPdo {
        match self {
            Self::Standard => EL3001TxPdo {
                ai_standard: Some(AiStandard::default()),
                ai_compact: None,
            },
            Self::Compact => EL3001TxPdo {
                ai_standard: None,
                ai_compact: Some(AiCompact::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) -> EL3001RxPdo {
        match self {
            Self::Standard => EL3001RxPdo {},
            Self::Compact => EL3001RxPdo {},
        }
    }
}

pub const EL3001_VENDOR_ID: u32 = 0x2;
pub const EL3001_PRODUCT_ID: u32 = 0x0bb93052;
pub const EL3001_REVISION_A: u32 = 0x00160000;
pub const EL3001_IDENTITY_A: SubDeviceIdentityTuple =
    (EL3001_VENDOR_ID, EL3001_PRODUCT_ID, EL3001_REVISION_A);
