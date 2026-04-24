use super::{EthercatDeviceProcessing, NewEthercatDevice};
use crate::devices::SubDeviceIdentityTuple;
use crate::io::digital_input::DigitalInputDevice;
use crate::io::digital_output::DigitalOutputDevice;
use crate::pdo::basic::BoolPdoObject;
use crate::pdo::{RxPdo, TxPdo};
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};

/// EP2339_0021 16-channel; digital input/output
#[derive(EthercatDevice)]
pub struct EP2339_0021 {
    pub rxpdo: EP2339_0021RxPdo,
    pub txpdo: EP2339_0021TxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EP2339_0021 {}

impl std::fmt::Debug for EP2339_0021 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EP2339_0021")
    }
}

impl NewEthercatDevice for EP2339_0021 {
    fn new() -> Self {
        Self {
            rxpdo: EP2339_0021RxPdo::default(),
            txpdo: EP2339_0021TxPdo::default(),
            is_used: false,
        }
    }
}

impl DigitalOutputDevice for EP2339_0021 {
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

impl DigitalInputDevice for EP2339_0021 {
    fn get_input(&self, port: usize) -> Result<bool, anyhow::Error> {
                let error = anyhow::anyhow!(
            "[{}::Device::digital_input_state] Port index {} is not available",
            module_path!(),
            port
        );

        match port {
            0 => Ok(self.txpdo.channel1.as_ref().ok_or(error)?.value),
            1 => Ok(self.txpdo.channel2.as_ref().ok_or(error)?.value),
            2 => Ok(self.txpdo.channel3.as_ref().ok_or(error)?.value),
            3 => Ok(self.txpdo.channel4.as_ref().ok_or(error)?.value),
            4 => Ok(self.txpdo.channel5.as_ref().ok_or(error)?.value),
            5 => Ok(self.txpdo.channel6.as_ref().ok_or(error)?.value),
            6 => Ok(self.txpdo.channel7.as_ref().ok_or(error)?.value),
            7 => Ok(self.txpdo.channel8.as_ref().ok_or(error)?.value),
            8 => Ok(self.txpdo.channel9.as_ref().ok_or(error)?.value),
            9 => Ok(self.txpdo.channel10.as_ref().ok_or(error)?.value),
            10 => Ok(self.txpdo.channel11.as_ref().ok_or(error)?.value),
            11 => Ok(self.txpdo.channel12.as_ref().ok_or(error)?.value),
            12 => Ok(self.txpdo.channel13.as_ref().ok_or(error)?.value),
            13 => Ok(self.txpdo.channel14.as_ref().ok_or(error)?.value),
            14 => Ok(self.txpdo.channel15.as_ref().ok_or(error)?.value),
            15 => Ok(self.txpdo.channel16.as_ref().ok_or(error)?.value),
            _ => Err(anyhow::anyhow!(
                "EL1002 has 2 ports (0-1), requested index {}",
                port
            )),
        }

    }

    fn get_port_count(&self) -> usize {
        16
    }
}

#[derive(Debug, Clone)]
pub enum EP2339_0021OutputPort {
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

#[derive(Debug, Clone)]
pub enum EP2339_0021InputPort {
    DI1,
    DI2,
    DI3,
    DI4,
    DI5,
    DI6,
    DI7,
    DI8,
    DI9,
    DI10,
    DI11,
    DI12,
    DI13,
    DI14,
    DI15,
    DI16,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EP2339_0021RxPdo {
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

impl Default for EP2339_0021RxPdo {
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

#[derive(Debug, Clone, TxPdo)]
pub struct EP2339_0021TxPdo {
    #[pdo_object_index(0x1A00)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A01)]
    pub channel2: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A02)]
    pub channel3: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A03)]
    pub channel4: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A04)]
    pub channel5: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A05)]
    pub channel6: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A06)]
    pub channel7: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A07)]
    pub channel8: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A08)]
    pub channel9: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A09)]
    pub channel10: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A0A)]
    pub channel11: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A0B)]
    pub channel12: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A0C)]
    pub channel13: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A0D)]
    pub channel14: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A0E)]
    pub channel15: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A0F)]
    pub channel16: Option<BoolPdoObject>,
}

impl Default for EP2339_0021TxPdo {
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

pub const EP2339_0021_VENDOR_ID: u32 = 0x2;
pub const EP2339_0021_PRODUCT_ID_A: u32 = 0x9234052;
pub const EP2339_0021_REVISION_A: u32 = 0x130015;
pub const EP2339_0021_IDENTITY_A: SubDeviceIdentityTuple = (
    EP2339_0021_VENDOR_ID,
    EP2339_0021_PRODUCT_ID_A,
    EP2339_0021_REVISION_A,
);
