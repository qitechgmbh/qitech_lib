use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::digital_output::DigitalOutputDevice;
use crate::pdo::{basic::BoolPdoObject, RxPdo};
use ethercat_hal_derive::{EthercatDevice, RxPdo};

/// EL2808 8-channel digital output device
///
/// 24V DC, 0.5A per channel. Same PDO layout as EL2809 with 8 instead of 16
/// channels (RxPDO objects 0x1600..0x1607). Identity verified against the
/// official Beckhoff ESI file (EL2xxx.xml, product 0x0AF83052) on 2026-11-08.
#[derive(EthercatDevice)]
pub struct EL2808 {
    pub rxpdo: EL2808RxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL2808 {}

impl std::fmt::Debug for EL2808 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2808")
    }
}

impl NewEthercatDevice for EL2808 {
    fn new() -> Self {
        Self {
            rxpdo: EL2808RxPdo::default(),
            is_used: false,
        }
    }
}

impl DigitalOutputDevice for EL2808 {
    fn set_output(&mut self, port: usize, value: bool) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self.rxpdo.channel1.as_mut().expect(expect_text).value = value,
            1 => self.rxpdo.channel2.as_mut().expect(expect_text).value = value,
            2 => self.rxpdo.channel3.as_mut().expect(expect_text).value = value,
            3 => self.rxpdo.channel4.as_mut().expect(expect_text).value = value,
            4 => self.rxpdo.channel5.as_mut().expect(expect_text).value = value,
            5 => self.rxpdo.channel6.as_mut().expect(expect_text).value = value,
            6 => self.rxpdo.channel7.as_mut().expect(expect_text).value = value,
            7 => self.rxpdo.channel8.as_mut().expect(expect_text).value = value,
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        8
    }
}

impl EL2808 {
    /// Currently set output state of a channel (the value to be sent in the next cycle).
    pub fn get_output_state(&self, port: usize) -> bool {
        let ch = match port {
            0 => &self.rxpdo.channel1,
            1 => &self.rxpdo.channel2,
            2 => &self.rxpdo.channel3,
            3 => &self.rxpdo.channel4,
            4 => &self.rxpdo.channel5,
            5 => &self.rxpdo.channel6,
            6 => &self.rxpdo.channel7,
            7 => &self.rxpdo.channel8,
            _ => return false,
        };
        ch.as_ref().map(|c| c.value).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub enum EL2808Port {
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
pub struct EL2808RxPdo {
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

impl Default for EL2808RxPdo {
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

pub const EL2808_VENDOR_ID: u32 = 0x2;
pub const EL2808_PRODUCT_ID: u32 = 0x0AF83052;
pub const EL2808_REVISION_A: u32 = 0x00100000;
pub const EL2808_REVISION_B: u32 = 0x00110000;
pub const EL2808_REVISION_C: u32 = 0x00120000;

pub const EL2808_IDENTITY_A: SubDeviceIdentityTuple =
    (EL2808_VENDOR_ID, EL2808_PRODUCT_ID, EL2808_REVISION_A);
pub const EL2808_IDENTITY_B: SubDeviceIdentityTuple =
    (EL2808_VENDOR_ID, EL2808_PRODUCT_ID, EL2808_REVISION_B);
pub const EL2808_IDENTITY_C: SubDeviceIdentityTuple =
    (EL2808_VENDOR_ID, EL2808_PRODUCT_ID, EL2808_REVISION_C);
