use super::{
    EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice, NewEthercatDevice,
};
use qitech_lib_ethercat_hal::EtherCATThreadChannel;
use qitech_lib_ethercat_hal::coe::Configuration;
use qitech_lib_ethercat_hal::io::analog_output::{AnalogOutputDevice, AnalogOutputOutput};
use qitech_lib_ethercat_hal::pdo::PredefinedPdoAssignment;
use qitech_lib_ethercat_hal::pdo::RxPdo;
use qitech_lib_ethercat_hal::pdo::TxPdo;
use qitech_lib_ethercat_hal::pdo::el40xx::AnalogOutput;
use qitech_lib_ethercat_hal::shared_config::el40xx::EL40XXChannelConfiguration;
use qitech_lib_ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};
/// EL4002 2-channel analog output device
///
/// 0-10V DC, 12-bit resolution

#[derive(EthercatDevice)]
pub struct EL4002 {
    pub configuration: EL4002Configuration,
    pub rxpdo: EL4002RxPdo,
    pub txpdo: EL4002TxPdo,
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
        }
    }
}
fn normalize_voltage_to_int(value: f32) -> i16 {
    // Clamp the value between 0.0 and 10.0
    let clamped = value.clamp(0.0, 10.0);

    // Normalize to 0.0-1.0 range
    let normalized = clamped / 10.0;
    (normalized * 32767.0) as i16
}

impl AnalogOutputDevice for EL4002 {
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput) {
        let value = normalize_voltage_to_int(value.0);
        match port {
            0 => {
                if let Some(channel) = self.rxpdo.ao_channel1.as_mut() {
                    channel.value = value
                }
            }
            1 => {
                if let Some(channel) = self.rxpdo.ao_channel2.as_mut() {
                    channel.value = value
                }
            }
            _ => return (),
        }
    }

    fn get_port_count(&self) -> usize {
        2
    }
}

impl EL4002 {
    pub fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), qitech_lib_ethercat_hal::Error> {
        /*
        self.configuration
            .channel1
            .write_channel_config(subdevice, 0x8000)
            .await?;
        self.configuration
            .channel1
            .write_channel_config(subdevice, 0x8010)
            .await?;
        */
        self.configuration
            .pdo_assignment
            .txpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
        self.configuration
            .pdo_assignment
            .rxpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
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
        EL4002TxPdo {}
    }
}
impl IdentifiedDevice for EL4002 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 0xfa23052,
        revisions: &[0x140000],
    };
}
