use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::analog_input::{AnalogInputDevice, AnalogInputInput};
use crate::pdo::RxPdo;
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

#[derive(Debug, Clone)]
pub struct EL3062_0030Configuration {
    pub pdo_assignment: EL3062_0030PredefinedPdoAssignment,
    pub channel_1: EL30XXChannelConfiguration,
    pub channel_2: EL30XXChannelConfiguration,
}
#[derive(EthercatDevice)]
pub struct EL3062_0030 {
    pub configuration: EL3062_0030Configuration,
    pub txpdo: EL3062_0030TxPdo,
    pub rxpdo: EL3062_0030RxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL3062_0030 {}

impl std::fmt::Debug for EL3062_0030 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3062_0030")
    }
}

impl Default for EL3062_0030Configuration {
    fn default() -> Self {
        Self {
            pdo_assignment: EL3062_0030PredefinedPdoAssignment::Standard,
            channel_1: EL30XXChannelConfiguration::default(),
            channel_2: EL30XXChannelConfiguration::default(),
        }
    }
}

impl NewEthercatDevice for EL3062_0030 {
    fn new() -> Self {
        let configuration: EL3062_0030Configuration = EL3062_0030Configuration::default();
        Self {
            txpdo: configuration.txpdo_assignment(),
            rxpdo: configuration.rxpdo_assignment(),
            configuration,
            is_used: false,
        }
    }
}

impl AnalogInputDevice for EL3062_0030 {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error> {
        let raw_value = match port {
            0 => match &self.txpdo {
                EL3062_0030TxPdo {
                    ai_standard_channel1: Some(ai_standard_channel1),
                    ..
                } => ai_standard_channel1.value,
                EL3062_0030TxPdo {
                    ai_compact_channel1: Some(ai_compact_channel1),
                    ..
                } => ai_compact_channel1.value,
                _ => panic!("Invalid TxPdo assignment"),
            },
            1 => match &self.txpdo {
                EL3062_0030TxPdo {
                    ai_standard_channel2: Some(ai_standard_channel2),
                    ..
                } => ai_standard_channel2.value,
                EL3062_0030TxPdo {
                    ai_compact_channel2: Some(ai_compact_channel2),
                    ..
                } => ai_compact_channel2.value,
                _ => panic!("Invalid TxPdo assignment"),
            },
            _ => return Err(anyhow::anyhow!("EL3062_0030 only has TWO ports")),
        };
        let raw_value = U16SigningConverter::load_raw(raw_value);

        let presentation = match port {
            0 => &self.configuration.channel_1.presentation,
            1 => &self.configuration.channel_2.presentation,
            _ => return Err(anyhow::anyhow!("EL3062_0030 only has TWO ports")),
        };
        let value: i16 = match presentation {
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
        2
    }

    fn analog_input_range(&self) -> AnalogInputRange {
        AnalogInputRange::Potential {
            min: ElectricPotential::new::<volt>(0.0),
            max: ElectricPotential::new::<volt>(30.0),
            min_raw: 0,
            max_raw: i16::MAX,
        }
    }
}

impl ConfigurableDevice for EL3062_0030 {
    type Config = EL3062_0030Configuration;

    fn set_config(&mut self, config: EL3062_0030Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.rxpdo = config.rxpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL3062_0030Configuration {
        &self.configuration
    }
}

#[derive(Debug, Clone)]
pub enum EL3062_0030Port {
    AI1,
    AI2,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL3062_0030TxPdo {
    #[pdo_object_index(0x1A00)]
    pub ai_standard_channel1: Option<AiStandard>,
    #[pdo_object_index(0x1A01)]
    pub ai_compact_channel1: Option<AiCompact>,

    #[pdo_object_index(0x1A02)]
    pub ai_standard_channel2: Option<AiStandard>,
    #[pdo_object_index(0x1A03)]
    pub ai_compact_channel2: Option<AiCompact>,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL3062_0030RxPdo {}

impl Configuration for EL3062_0030Configuration {
    type TxPdo = EL3062_0030TxPdo;
    type RxPdo = EL3062_0030RxPdo;

    fn txpdo_assignment(&self) -> EL3062_0030TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL3062_0030RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        let mut writes = self
            .channel_1
            .get_channel_coe_writes(device_address, 0x8000);
        writes.extend(
            self.channel_2
                .get_channel_coe_writes(device_address, 0x8010),
        );
        Ok(writes)
    }
}

#[derive(Debug, Clone)]
pub enum EL3062_0030PredefinedPdoAssignment {
    Standard,
    Compact,
}

impl PredefinedPdoAssignment<EL3062_0030TxPdo, EL3062_0030RxPdo>
    for EL3062_0030PredefinedPdoAssignment
{
    fn txpdo_assignment(&self) -> EL3062_0030TxPdo {
        match self {
            Self::Standard => EL3062_0030TxPdo {
                ai_standard_channel1: Some(AiStandard::default()),
                ai_compact_channel1: None,
                ai_standard_channel2: Some(AiStandard::default()),
                ai_compact_channel2: None,
            },
            Self::Compact => EL3062_0030TxPdo {
                ai_standard_channel1: None,
                ai_compact_channel1: Some(AiCompact::default()),
                ai_standard_channel2: None,
                ai_compact_channel2: Some(AiCompact::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) -> EL3062_0030RxPdo {
        match self {
            Self::Standard => EL3062_0030RxPdo {},
            Self::Compact => EL3062_0030RxPdo {},
        }
    }
}

pub const EL3062_0030_VENDOR_ID: u32 = 0x2;
pub const EL3062_0030_PRODUCT_ID: u32 = 0x0bf63052;
pub const EL3062_0030_REVISION_A: u32 = 0x0017001e;
pub const EL3062_0030_IDENTITY_A: SubDeviceIdentityTuple = (
    EL3062_0030_VENDOR_ID,
    EL3062_0030_PRODUCT_ID,
    EL3062_0030_REVISION_A,
);
