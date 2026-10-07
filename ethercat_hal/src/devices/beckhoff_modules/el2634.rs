use super::{EthercatDeviceProcessing, NewEthercatDevice};
use crate::coe::{ConfigurableDevice, Configuration};
use crate::io::digital_output::DigitalOutputDevice;
use crate::pdo::{RxPdo, basic::BoolPdoObject};
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};

/// EL2634 4-channel relay device
///
/// 250V AC / 30V DC / 4A per channel
#[derive(EthercatDevice)]
pub struct EL2634 {
    pub rxpdo: EL2634RxPdo,
    is_used: bool,
    config: EL2634Configuration,
}

impl EthercatDeviceProcessing for EL2634 {}

impl std::fmt::Debug for EL2634 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2634")
    }
}

impl NewEthercatDevice for EL2634 {
    fn new() -> Self {
        let config = EL2634Configuration::default();
        Self {
            rxpdo: config.rxpdo_assignment(),
            is_used: false,
            config,
        }
    }
}

impl ConfigurableDevice for EL2634 {
    type Config = EL2634Configuration;

    fn set_config(&mut self, config: EL2634Configuration) {
        self.rxpdo = config.rxpdo_assignment();
        self.config = config;
    }

    fn get_config(&self) -> &EL2634Configuration {
        &self.config
    }
}

impl DigitalOutputDevice for EL2634 {
    fn set_output(&mut self, port: usize, value: bool) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self.rxpdo.channel1.as_mut().expect(expect_text).value = value.into(),
            1 => self.rxpdo.channel2.as_mut().expect(expect_text).value = value.into(),
            2 => self.rxpdo.channel3.as_mut().expect(expect_text).value = value.into(),
            3 => self.rxpdo.channel4.as_mut().expect(expect_text).value = value.into(),
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

#[derive(Debug, Clone)]
pub enum EL2634Port {
    R1,
    R2,
    R3,
    R4,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL2634RxPdo {
    #[pdo_object_index(0x1600)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1601)]
    pub channel2: Option<BoolPdoObject>,
    #[pdo_object_index(0x1602)]
    pub channel3: Option<BoolPdoObject>,
    #[pdo_object_index(0x1603)]
    pub channel4: Option<BoolPdoObject>,
}

impl Default for EL2634RxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
            channel3: Some(BoolPdoObject::default()),
            channel4: Some(BoolPdoObject::default()),
        }
    }
}

/// The EL2634 has no inputs
#[derive(Debug, Clone, TxPdo)]
pub struct EL2634TxPdo {}

/// The EL2634 has a fixed PDO assignment and no further CoE parameters
#[derive(Default, Clone, PartialEq, Debug)]
pub struct EL2634Configuration {}

impl Configuration for EL2634Configuration {
    /// The EL2634 has no mailbox, its PDO assignment is fixed and can't be written over CoE
    fn get_sm_coe_writes(
        &self,
        _device_address: u16,
    ) -> Result<Vec<common::SdoRequest>, anyhow::Error> {
        Ok(vec![])
    }

    type TxPdo = EL2634TxPdo;
    type RxPdo = EL2634RxPdo;

    fn txpdo_assignment(&self) -> EL2634TxPdo {
        EL2634TxPdo {}
    }

    fn rxpdo_assignment(&self) -> EL2634RxPdo {
        EL2634RxPdo::default()
    }
}
