use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::EtherCATThreadChannel;
use crate::io::analog_output::{AnalogVoltageOutputDevice};
use common::pdo::PredefinedPdoAssignment;
use common::pdo::RxPdo;
use common::pdo::TxPdo;
use common::pdo::el40xx::AnalogOutput;
use units::ElectricPotential;
use units::electric_potential::volt;
use crate::shared_config::el40xx::EL40XXChannelConfiguration;
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};

/// EL4002 2-channel analog output device
///
/// 0-10V DC, 12-bit resolution
#[derive(EthercatDevice)]
pub struct EL4002 {
    pub configuration: EL4002Configuration,
    pub rxpdo: EL4002RxPdo,
    pub txpdo: EL4002TxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL4002 {}

#[derive(Debug, Clone)]
pub struct EL4002Configuration {
    pub pdo_assignment: EL4002PredefinedPdoAssignment,
    pub channel1: EL40XXChannelConfiguration,
    pub channel2: EL40XXChannelConfiguration,
}

#[derive(Debug, Clone)]
pub enum EL4002PredefinedPdoAssignment {
    Standard,
}
 
impl std::fmt::Debug for EL4002 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL4002")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL4002Port {
    AO1,
    AO2,
}

impl Default for EL4002Configuration {
    fn default() -> Self {
        Self {
            pdo_assignment: EL4002PredefinedPdoAssignment::Standard,
            channel1: EL40XXChannelConfiguration::default(),
            channel2: EL40XXChannelConfiguration::default(),
        }
    }
}
impl NewEthercatDevice for EL4002 {
    fn new() -> Self {
        let configuration: EL4002Configuration = EL4002Configuration::default();
        Self {
            configuration: configuration.clone(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            is_used: false,
        }
    }
}

impl AnalogVoltageOutputDevice for EL4002 {
    fn get_port_count(&self) -> usize {
        2
    }

    fn get_minimum_voltage(&self) -> ElectricPotential {
        ElectricPotential::new::<volt>(0.0)
    }

    fn get_maximum_voltage(&self) -> ElectricPotential {
        ElectricPotential::new::<volt>(10.0)
    }

    fn set_voltage_relative(&mut self, port: usize, value: f64) {
        let option = match port {
            0 => self.rxpdo.ao_channel1.as_mut(),
            1 => self.rxpdo.ao_channel2.as_mut(),
            _ => panic!("Port {} index out of range [0, 1]", port),
        };

        option
            .expect("All channels should be Some(_)")
            .set_f64(value);
    }
}

impl EL4002 {
    pub fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        
        self.configuration
            .channel1
            .write_channel_config(ecat_channel.clone(),device_address, 0x8000)?;
        self.configuration
            .channel1
            .write_channel_config(ecat_channel.clone(),device_address, 0x8010)?;
        /*self.configuration
            .pdo_assignment
            .txpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
        self.configuration
            .pdo_assignment
            .rxpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;*/
        Ok(())
    }
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL4002RxPdo {
    #[pdo_object_index(0x1600)]
    pub ao_channel1: Option<AnalogOutput>,
    #[pdo_object_index(0x1601)]
    pub ao_channel2: Option<AnalogOutput>,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL4002TxPdo {}

impl PredefinedPdoAssignment<EL4002TxPdo, EL4002RxPdo> for EL4002PredefinedPdoAssignment {
    fn rxpdo_assignment(&self) -> EL4002RxPdo {
        match self {
            Self::Standard => EL4002RxPdo {
                ao_channel1: Some(AnalogOutput::default()),
                ao_channel2: Some(AnalogOutput::default()),
            },
        }
    }

    fn txpdo_assignment(&self) -> EL4002TxPdo {
        EL4002TxPdo {

        }
    }
}
pub const EL4002_VENDOR_ID: u32 = 0x2;
pub const EL4002_PRODUCT_ID: u32 = 0xfa23052;
pub const EL4002_REVISION_A: u32 = 0x140000;
pub const EL4002_IDENTITY_A: SubDeviceIdentityTuple =
    (EL4002_VENDOR_ID, EL4002_PRODUCT_ID, EL4002_REVISION_A);
