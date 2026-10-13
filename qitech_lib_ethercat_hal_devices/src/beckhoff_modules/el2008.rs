use super::{
    EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice, NewEthercatDevice,
};
use qitech_lib_ethercat_hal::io::digital_output::DigitalOutputDevice;
use qitech_lib_ethercat_hal::pdo::{RxPdo, basic::BoolPdoObject};
use qitech_lib_ethercat_hal_derive::{EthercatDevice, RxPdo};

/// EL2008 8-channel digital output device
///
/// 24V DC, 0.5A per channel
#[derive(EthercatDevice)]
pub struct EL2008 {
    pub rxpdo: EL2008RxPdo,
}

impl EthercatDeviceProcessing for EL2008 {}

impl std::fmt::Debug for EL2008 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2008")
    }
}

impl NewEthercatDevice for EL2008 {
    fn new() -> Self {
        Self {
            rxpdo: EL2008RxPdo::default(),
        }
    }
}

impl DigitalOutputDevice for EL2008 {
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
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        8
    }
}

#[derive(Debug, Clone)]
pub enum EL2008Port {
    DO1,
    DO2,
    DO3,
    DO4,
    DO5,
    DO6,
    DO7,
    DO8,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL2008RxPdo {
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
}

impl Default for EL2008RxPdo {
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
        }
    }
}

impl IdentifiedDevice for EL2008 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 0x7d83052,
        revisions: &[0x00110000, 0x120000],
    };
}
