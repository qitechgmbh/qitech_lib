use super::{
    EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice, NewEthercatDevice,
};
use qitech_lib_ethercat_hal::io::digital_output::DigitalOutputDevice;
use qitech_lib_ethercat_hal::pdo::{RxPdo, basic::BoolPdoObject};
use qitech_lib_ethercat_hal_derive::{EthercatDevice, RxPdo};

/// EL2002 2-channel digital output device
///
/// 24V DC, 0.5A per channel
#[derive(EthercatDevice)]
pub struct EL2002 {
    pub rxpdo: EL2002RxPdo,
}

impl EthercatDeviceProcessing for EL2002 {}

impl std::fmt::Debug for EL2002 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2002")
    }
}

impl NewEthercatDevice for EL2002 {
    fn new() -> Self {
        Self {
            rxpdo: EL2002RxPdo::default(),
        }
    }
}

impl DigitalOutputDevice for EL2002 {
    fn set_output(&mut self, port: usize, value: bool) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self.rxpdo.channel1.as_mut().expect(expect_text).value = value,
            1 => self.rxpdo.channel2.as_mut().expect(expect_text).value = value,
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        2
    }
}

#[derive(Debug, Clone)]
pub enum EL2002Port {
    DO1,
    DO2,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL2002RxPdo {
    #[pdo_object_index(0x1600)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1601)]
    pub channel2: Option<BoolPdoObject>,
}

impl Default for EL2002RxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
        }
    }
}

impl IdentifiedDevice for EL2002 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 0x07d23052,
        revisions: &[0x00110000, 0x00120000],
    };
}
