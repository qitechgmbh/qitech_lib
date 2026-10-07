use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::coe::{ConfigurableDevice, Configuration};
use crate::io::digital_output::DigitalOutputDevice;
use crate::pdo::{RxPdo, basic::BoolPdoObject};
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};
/// EL2004 4-channel digital output device
/// 24V DC, 0.5A per channel
#[derive(EthercatDevice)]
pub struct EL2004 {
    pub rxpdo: EL2004RxPdo,
    is_used: bool,
    config: EL2004Configuration,
}

impl EthercatDeviceProcessing for EL2004 {}

impl std::fmt::Debug for EL2004 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2004")
    }
}

impl NewEthercatDevice for EL2004 {
    fn new() -> Self {
        let config = EL2004Configuration::default();
        Self {
            rxpdo: config.rxpdo_assignment(),
            is_used: false,
            config,
        }
    }
}

impl ConfigurableDevice for EL2004 {
    type Config = EL2004Configuration;

    fn set_config(&mut self, config: EL2004Configuration) {
        self.rxpdo = config.rxpdo_assignment();
        self.config = config;
    }

    fn get_config(&self) -> &EL2004Configuration {
        &self.config
    }
}

impl DigitalOutputDevice for EL2004 {
    fn set_output(&mut self, port: usize, value: bool) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self.rxpdo.channel1.as_mut().expect(expect_text).value = value,
            1 => self.rxpdo.channel2.as_mut().expect(expect_text).value = value,
            2 => self.rxpdo.channel3.as_mut().expect(expect_text).value = value,
            3 => self.rxpdo.channel4.as_mut().expect(expect_text).value = value,
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

#[derive(Debug, Clone)]
pub enum EL2004Port {
    DO1,
    DO2,
    DO3,
    DO4,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL2004RxPdo {
    #[pdo_object_index(0x1600)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1601)]
    pub channel2: Option<BoolPdoObject>,
    #[pdo_object_index(0x1602)]
    pub channel3: Option<BoolPdoObject>,
    #[pdo_object_index(0x1603)]
    pub channel4: Option<BoolPdoObject>,
}

impl Default for EL2004RxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
            channel3: Some(BoolPdoObject::default()),
            channel4: Some(BoolPdoObject::default()),
        }
    }
}

/// The EL2004 has no inputs
#[derive(Debug, Clone, TxPdo)]
pub struct EL2004TxPdo {}

/// The EL2004 has a fixed PDO assignment and no further CoE parameters
#[derive(Default, Clone, PartialEq, Debug)]
pub struct EL2004Configuration {}

impl Configuration for EL2004Configuration {
    /// The EL2004 has no mailbox, its PDO assignment is fixed and can't be written over CoE
    fn get_sm_coe_writes(
        &self,
        _device_address: u16,
    ) -> Result<Vec<common::SdoRequest>, anyhow::Error> {
        Ok(vec![])
    }

    type TxPdo = EL2004TxPdo;
    type RxPdo = EL2004RxPdo;

    fn txpdo_assignment(&self) -> EL2004TxPdo {
        EL2004TxPdo {}
    }

    fn rxpdo_assignment(&self) -> EL2004RxPdo {
        EL2004RxPdo::default()
    }
}

pub const EL2004_VENDOR_ID: u32 = 0x2;
pub const EL2004_PRODUCT_ID: u32 = 131346514;
pub const EL2004_REVISION_A: u32 = 1179648;
pub const EL2004_IDENTITY_A: SubDeviceIdentityTuple =
    (EL2004_VENDOR_ID, EL2004_PRODUCT_ID, EL2004_REVISION_A);
