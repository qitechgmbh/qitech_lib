/// EL3062 2-channel analog input (0..10 V).
///
/// Derived from the EL3062_0030 driver (0-20 mA variant). Identities verified
/// against the official Beckhoff ESI (EL30xx.xml) on 2026-11-13: standard EL3062
/// = product 0x0BF63052, revisions 0x100000..0x150000 (the -0030 variant shares
/// the product code with a different revision suffix).
///
/// qi-heating use case: oil-tank level sensor (0-10 V).
use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::EtherCATThreadChannel;
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
use ethercat_hal_derive::EthercatDevice;
use units::{electric_potential::volt, f64::ElectricPotential};

#[derive(Debug, Clone)]
pub struct EL3062Configuration {
    pub pdo_assignment: EL3062PredefinedPdoAssignment,
    pub channel_1: EL30XXChannelConfiguration,
    pub channel_2: EL30XXChannelConfiguration,
}
#[derive(EthercatDevice)]
pub struct EL3062 {
    pub configuration: EL3062Configuration,
    pub txpdo: EL3062TxPdo,
    pub rxpdo: EL3062RxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL3062 {}

impl std::fmt::Debug for EL3062 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3062")
    }
}

impl Default for EL3062Configuration {
    fn default() -> Self {
        Self {
            pdo_assignment: EL3062PredefinedPdoAssignment::Standard,
            channel_1: EL30XXChannelConfiguration::default(),
            channel_2: EL30XXChannelConfiguration::default(),
        }
    }
}

impl NewEthercatDevice for EL3062 {
    fn new() -> Self {
        let configuration: EL3062Configuration = EL3062Configuration::default();
        Self {
            configuration: configuration.clone(),
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
        }
    }
}

impl AnalogInputDevice for EL3062 {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error> {
        let raw_value = match port {
            0 => match &self.txpdo {
                EL3062TxPdo {
                    ai_standard_channel1: Some(ai_standard_channel1),
                    ..
                } => ai_standard_channel1.value,
                EL3062TxPdo {
                    ai_compact_channel1: Some(ai_compact_channel1),
                    ..
                } => ai_compact_channel1.value,
                _ => panic!("Invalid TxPdo assignment"),
            },
            1 => match &self.txpdo {
                EL3062TxPdo {
                    ai_standard_channel2: Some(ai_standard_channel2),
                    ..
                } => ai_standard_channel2.value,
                EL3062TxPdo {
                    ai_compact_channel2: Some(ai_compact_channel2),
                    ..
                } => ai_compact_channel2.value,
                _ => panic!("Invalid TxPdo assignment"),
            },
            _ => return Err(anyhow::anyhow!("EL3062 only has TWO ports")),
        };
        let raw_value = U16SigningConverter::load_raw(raw_value);

        let presentation = match port {
            0 => &self.configuration.channel_1.presentation,
            1 => &self.configuration.channel_2.presentation,
            _ => return Err(anyhow::anyhow!("EL3062 only has TWO ports")),
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
            max: ElectricPotential::new::<volt>(10.0),
            min_raw: 0,
            max_raw: i16::MAX,
        }
    }
}

impl ConfigurableDevice<EL3062Configuration> for EL3062 {
    fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        config: &EL3062Configuration,
    ) -> Result<(), anyhow::Error> {
        config.write_config(ecat_channel, device_address)?;
        self.configuration = config.clone();
        self.txpdo = config.pdo_assignment.txpdo_assignment();
        Ok(())
    }

    fn get_config(&self) -> EL3062Configuration {
        self.configuration.clone()
    }
}

#[derive(Debug, Clone)]
pub enum EL3062Port {
    AI1,
    AI2,
}

#[derive(Debug, Clone)]
pub struct EL3062TxPdo {
    pub ai_standard_channel1: Option<AiStandard>,
    pub ai_compact_channel1: Option<AiCompact>,

    pub ai_standard_channel2: Option<AiStandard>,
    pub ai_compact_channel2: Option<AiCompact>,
}

impl crate::coe::Configuration for EL3062TxPdo {
    /// Implemented by the ethercat_hal_derive::TxPdo derive macro
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        // Clear sync manager manager PDO assignment
        ecat_channel.sdo_write(device_address, 0x1C13, 0, 0u8)?;

        let mut len = 0;

        if self.ai_standard_channel1.is_some() {
            len += 1;
            ecat_channel.sdo_write(device_address, 0x1C13, len, 0x1A00u16)?;
        }
        if self.ai_compact_channel1.is_some() {
            len += 1;
            ecat_channel.sdo_write(device_address, 0x1C13, len, 0x1A01u16)?;
        }
        if self.ai_standard_channel2.is_some() {
            len += 1;
            ecat_channel.sdo_write(device_address, 0x1C13, len, 0x1A02u16)?;
        }
        if self.ai_compact_channel2.is_some() {
            len += 1;
            ecat_channel.sdo_write(device_address, 0x1C13, len, 0x1A03u16)?;
        }

        // Set the number of assigned PDOs
        ecat_channel.sdo_write(device_address, 0x1C13, 0, len)?;

        Ok(())
    }
}
impl crate::pdo::TxPdo for EL3062TxPdo {
    ///Implemented by the ethercat_hal_derive::TxPdo derive macro
    fn get_objects(&self) -> Box<[Option<&dyn crate::pdo::TxPdoObject>]> {
        Box::new([
            self.ai_standard_channel1
                .as_ref()
                .map(|o| o as &dyn crate::pdo::TxPdoObject),
            self.ai_compact_channel1
                .as_ref()
                .map(|o| o as &dyn crate::pdo::TxPdoObject),
            self.ai_standard_channel2
                .as_ref()
                .map(|o| o as &dyn crate::pdo::TxPdoObject),
            self.ai_compact_channel2
                .as_ref()
                .map(|o| o as &dyn crate::pdo::TxPdoObject),
        ])
    }
    ///Implemented by the ethercat_hal_derive::TxPdo derive macro
    fn get_objects_mut(&mut self) -> Box<[Option<&mut dyn crate::pdo::TxPdoObject>]> {
        Box::new([
            self.ai_standard_channel1
                .as_mut()
                .map(|o| o as &mut dyn crate::pdo::TxPdoObject),
            self.ai_compact_channel1
                .as_mut()
                .map(|o| o as &mut dyn crate::pdo::TxPdoObject),
            self.ai_standard_channel2
                .as_mut()
                .map(|o| o as &mut dyn crate::pdo::TxPdoObject),
            self.ai_compact_channel2
                .as_mut()
                .map(|o| o as &mut dyn crate::pdo::TxPdoObject),
        ])
    }
}

#[derive(Debug, Clone)]
pub struct EL3062RxPdo {}
impl crate::coe::Configuration for EL3062RxPdo {
    ///Implemented by the ethercat_hal_derive::RxPdo derive macro
    fn write_config(
        &self,
        _ecat_channel: EtherCATThreadChannel,
        _addr: u16,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
impl crate::pdo::RxPdo for EL3062RxPdo {
    ///Implemented by the ethercat_hal_derive::RxPdo derive macro
    fn get_objects(&self) -> Box<[Option<&dyn crate::pdo::RxPdoObject>]> {
        Box::new([])
    }
}
impl Configuration for EL3062Configuration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        self.channel_1
            .write_channel_config(ecat_channel.clone(), device_address, 0x8000)?;
        self.channel_2
            .write_channel_config(ecat_channel.clone(), device_address, 0x8010)?;
        self.pdo_assignment
            .txpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
        self.pdo_assignment
            .rxpdo_assignment()
            .write_config(ecat_channel, device_address)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum EL3062PredefinedPdoAssignment {
    Standard,
    Compact,
}

impl PredefinedPdoAssignment<EL3062TxPdo, EL3062RxPdo>
    for EL3062PredefinedPdoAssignment
{
    fn txpdo_assignment(&self) -> EL3062TxPdo {
        match self {
            Self::Standard => EL3062TxPdo {
                ai_standard_channel1: Some(AiStandard::default()),
                ai_compact_channel1: None,
                ai_standard_channel2: Some(AiStandard::default()),
                ai_compact_channel2: None,
            },
            Self::Compact => EL3062TxPdo {
                ai_standard_channel1: None,
                ai_compact_channel1: Some(AiCompact::default()),
                ai_standard_channel2: None,
                ai_compact_channel2: Some(AiCompact::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) -> EL3062RxPdo {
        match self {
            Self::Standard => EL3062RxPdo {},
            Self::Compact => EL3062RxPdo {},
        }
    }
}

pub const EL3062_VENDOR_ID: u32 = 0x2;
pub const EL3062_PRODUCT_ID: u32 = 0x0bf63052;
pub const EL3062_REVISION_A: u32 = 0x00100000;
pub const EL3062_REVISION_B: u32 = 0x00110000;
pub const EL3062_REVISION_C: u32 = 0x00120000;
pub const EL3062_REVISION_D: u32 = 0x00130000;
pub const EL3062_REVISION_E: u32 = 0x00140000;
pub const EL3062_REVISION_F: u32 = 0x00150000;

pub const EL3062_IDENTITY_A: SubDeviceIdentityTuple =
    (EL3062_VENDOR_ID, EL3062_PRODUCT_ID, EL3062_REVISION_A);
pub const EL3062_IDENTITY_B: SubDeviceIdentityTuple =
    (EL3062_VENDOR_ID, EL3062_PRODUCT_ID, EL3062_REVISION_B);
pub const EL3062_IDENTITY_C: SubDeviceIdentityTuple =
    (EL3062_VENDOR_ID, EL3062_PRODUCT_ID, EL3062_REVISION_C);
pub const EL3062_IDENTITY_D: SubDeviceIdentityTuple =
    (EL3062_VENDOR_ID, EL3062_PRODUCT_ID, EL3062_REVISION_D);
pub const EL3062_IDENTITY_E: SubDeviceIdentityTuple =
    (EL3062_VENDOR_ID, EL3062_PRODUCT_ID, EL3062_REVISION_E);
pub const EL3062_IDENTITY_F: SubDeviceIdentityTuple =
    (EL3062_VENDOR_ID, EL3062_PRODUCT_ID, EL3062_REVISION_F);
