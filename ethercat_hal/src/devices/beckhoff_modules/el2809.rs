use super::{EthercatDeviceProcessing, NewEthercatDevice};
use crate::coe::{ConfigurableDevice, Configuration};
use crate::io::digital_output::DigitalOutputDevice;
use crate::pdo::{RxPdo, basic::BoolPdoObject};
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};

/// EL2809 16-channel digital output device
///
/// 24V DC, 0.5A per channel
#[derive(EthercatDevice)]
pub struct EL2809 {
    pub rxpdo: EL2809RxPdo,
    is_used: bool,
    config: EL2809Configuration,
}

impl EthercatDeviceProcessing for EL2809 {}

impl std::fmt::Debug for EL2809 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2809")
    }
}

impl NewEthercatDevice for EL2809 {
    fn new() -> Self {
        let config = EL2809Configuration::default();
        Self {
            rxpdo: config.rxpdo_assignment(),
            is_used: false,
            config,
        }
    }
}

impl ConfigurableDevice for EL2809 {
    type Config = EL2809Configuration;

    fn set_config(&mut self, config: EL2809Configuration) {
        self.rxpdo = config.rxpdo_assignment();
        self.config = config;
    }

    fn get_config(&self) -> &EL2809Configuration {
        &self.config
    }
}

impl DigitalOutputDevice for EL2809 {
    fn set_output(&mut self, port: usize, value: bool) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self.rxpdo.channel1.as_mut().expect(expect_text).value = value.into(),
            1 => self.rxpdo.channel2.as_mut().expect(expect_text).value = value.into(),
            2 => self.rxpdo.channel3.as_mut().expect(expect_text).value = value.into(),
            3 => self.rxpdo.channel4.as_mut().expect(expect_text).value = value.into(),
            4 => self.rxpdo.channel5.as_mut().expect(expect_text).value = value.into(),
            5 => self.rxpdo.channel6.as_mut().expect(expect_text).value = value.into(),
            6 => self.rxpdo.channel7.as_mut().expect(expect_text).value = value.into(),
            7 => self.rxpdo.channel8.as_mut().expect(expect_text).value = value.into(),
            8 => self.rxpdo.channel9.as_mut().expect(expect_text).value = value.into(),
            9 => self.rxpdo.channel10.as_mut().expect(expect_text).value = value.into(),
            10 => self.rxpdo.channel11.as_mut().expect(expect_text).value = value.into(),
            11 => self.rxpdo.channel12.as_mut().expect(expect_text).value = value.into(),
            12 => self.rxpdo.channel13.as_mut().expect(expect_text).value = value.into(),
            13 => self.rxpdo.channel14.as_mut().expect(expect_text).value = value.into(),
            14 => self.rxpdo.channel15.as_mut().expect(expect_text).value = value.into(),
            15 => self.rxpdo.channel16.as_mut().expect(expect_text).value = value.into(),
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        16
    }
}

#[derive(Debug, Clone)]
pub enum EL2809Port {
    DO1,
    DO2,
    DO3,
    DO4,
    DO5,
    DO6,
    DO7,
    DO8,
    DO9,
    DO10,
    DO11,
    DO12,
    DO13,
    DO14,
    DO15,
    DO16,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL2809RxPdo {
    #[pdo_object_index(0x1600)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1601)]
    pub channel2: Option<BoolPdoObject>,
    #[pdo_object_index(0x1602)]
    pub channel3: Option<BoolPdoObject>,
    #[pdo_object_index(0x1603)]
    pub channel4: Option<BoolPdoObject>,
    #[pdo_object_index(0x1604)]
    pub channel5: Option<BoolPdoObject>,
    #[pdo_object_index(0x1605)]
    pub channel6: Option<BoolPdoObject>,
    #[pdo_object_index(0x1606)]
    pub channel7: Option<BoolPdoObject>,
    #[pdo_object_index(0x1607)]
    pub channel8: Option<BoolPdoObject>,
    #[pdo_object_index(0x1608)]
    pub channel9: Option<BoolPdoObject>,
    #[pdo_object_index(0x1609)]
    pub channel10: Option<BoolPdoObject>,
    #[pdo_object_index(0x160A)]
    pub channel11: Option<BoolPdoObject>,
    #[pdo_object_index(0x160B)]
    pub channel12: Option<BoolPdoObject>,
    #[pdo_object_index(0x160C)]
    pub channel13: Option<BoolPdoObject>,
    #[pdo_object_index(0x160D)]
    pub channel14: Option<BoolPdoObject>,
    #[pdo_object_index(0x160E)]
    pub channel15: Option<BoolPdoObject>,
    #[pdo_object_index(0x160F)]
    pub channel16: Option<BoolPdoObject>,
}

impl Default for EL2809RxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
            channel3: Some(BoolPdoObject::default()),
            channel4: Some(BoolPdoObject::default()),
            channel5: Some(BoolPdoObject::default()),
            channel6: Some(BoolPdoObject::default()),
            channel7: Some(BoolPdoObject::default()),
            channel8: Some(BoolPdoObject::default()),
            channel9: Some(BoolPdoObject::default()),
            channel10: Some(BoolPdoObject::default()),
            channel11: Some(BoolPdoObject::default()),
            channel12: Some(BoolPdoObject::default()),
            channel13: Some(BoolPdoObject::default()),
            channel14: Some(BoolPdoObject::default()),
            channel15: Some(BoolPdoObject::default()),
            channel16: Some(BoolPdoObject::default()),
        }
    }
}

/// The EL2809 has no inputs
#[derive(Debug, Clone, TxPdo)]
pub struct EL2809TxPdo {}

/// The EL2809 has a fixed PDO assignment and no further CoE parameters
#[derive(Default, Clone, PartialEq, Debug)]
pub struct EL2809Configuration {}

impl Configuration for EL2809Configuration {
    /// The EL2809 has no mailbox, its PDO assignment is fixed and can't be written over CoE
    fn get_sm_coe_writes(
        &self,
        _device_address: u16,
    ) -> Result<Vec<common::SdoRequest>, anyhow::Error> {
        Ok(vec![])
    }

    type TxPdo = EL2809TxPdo;
    type RxPdo = EL2809RxPdo;

    fn txpdo_assignment(&self) -> EL2809TxPdo {
        EL2809TxPdo {}
    }

    fn rxpdo_assignment(&self) -> EL2809RxPdo {
        EL2809RxPdo::default()
    }
}
