use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::pdo::{el25xx::PwmDuty, RxPdo};
use ethercat_hal_derive::{EthercatDevice, RxPdo};

/// EL2502 2-channel PWM output terminal (24 V)
///
/// Uses the simplest RxPDO assignment per channel (ESI: 0x1600/0x1601), which
/// only carries the duty cycle ("PWM output", UINT16). The PWM period is fixed
/// via CoE configuration on the terminal (object 0x8000/0x8010, defaults:
/// period 4000 = 20 Hz..20 kHz presentation) — changing the period at runtime
/// is therefore not supported by this driver (use PDO variants 0x1602..0x1605
/// if ever needed).
///
/// Identity and PDO/coE structure derived from the official Beckhoff ESI file
/// (EL25xx.xml) on 2026-11-08.
///
/// Note: duty scaling follows the terminal's "Presentation" setting
/// (CoE 0x8000); Beckhoff default is 1/100 % per digit (10_000 = 100 %).
#[derive(EthercatDevice)]
pub struct EL2502 {
    pub rxpdo: EL2502RxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL2502 {}

impl std::fmt::Debug for EL2502 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL2502")
    }
}

impl NewEthercatDevice for EL2502 {
    fn new() -> Self {
        Self {
            rxpdo: EL2502RxPdo::default(),
            is_used: false,
        }
    }
}

impl EL2502 {
    /// Sets the raw duty value for a channel (see terminal "Presentation" setting).
    pub fn set_duty_raw(&mut self, port: usize, value: u16) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self.rxpdo.channel1.as_mut().expect(expect_text).value = value,
            1 => self.rxpdo.channel2.as_mut().expect(expect_text).value = value,
            _ => (),
        }
    }

    /// Sets the duty cycle in percent [0.0, 100.0] assuming the Beckhoff default
    /// presentation (1/100 % per digit).
    pub fn set_duty_percent(&mut self, port: usize, percent: f64) {
        let expect_text = "All channels should be Some(_)";
        match port {
            0 => self
                .rxpdo
                .channel1
                .as_mut()
                .expect(expect_text)
                .set_percent(percent),
            1 => self
                .rxpdo
                .channel2
                .as_mut()
                .expect(expect_text)
                .set_percent(percent),
            _ => (),
        }
    }

    /// Current raw duty value of a channel.
    pub fn duty_raw(&self, port: usize) -> Option<u16> {
        match port {
            0 => self.rxpdo.channel1.as_ref().map(|c| c.value),
            1 => self.rxpdo.channel2.as_ref().map(|c| c.value),
            _ => None,
        }
    }

    pub fn get_port_count(&self) -> usize {
        2
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL2502Port {
    PWM1,
    PWM2,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL2502RxPdo {
    #[pdo_object_index(0x1600)]
    pub channel1: Option<PwmDuty>,
    #[pdo_object_index(0x1601)]
    pub channel2: Option<PwmDuty>,
}

impl Default for EL2502RxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(PwmDuty::default()),
            channel2: Some(PwmDuty::default()),
        }
    }
}

pub const EL2502_VENDOR_ID: u32 = 0x2;
pub const EL2502_PRODUCT_ID: u32 = 0x09C63052;
pub const EL2502_REVISION_A: u32 = 0x00100000;
pub const EL2502_REVISION_B: u32 = 0x00110000;
pub const EL2502_REVISION_C: u32 = 0x00120000;
pub const EL2502_REVISION_D: u32 = 0x00130000;
pub const EL2502_REVISION_E: u32 = 0x00140000;
pub const EL2502_REVISION_F: u32 = 0x00150000;
pub const EL2502_REVISION_G: u32 = 0x00160000;
pub const EL2502_REVISION_H: u32 = 0x00170000;
pub const EL2502_REVISION_I: u32 = 0x00180000;
pub const EL2502_REVISION_J: u32 = 0x00190000;
pub const EL2502_REVISION_K: u32 = 0x001A0000;

pub const EL2502_IDENTITY_A: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_A);
pub const EL2502_IDENTITY_B: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_B);
pub const EL2502_IDENTITY_C: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_C);
pub const EL2502_IDENTITY_D: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_D);
pub const EL2502_IDENTITY_E: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_E);
pub const EL2502_IDENTITY_F: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_F);
pub const EL2502_IDENTITY_G: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_G);
pub const EL2502_IDENTITY_H: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_H);
pub const EL2502_IDENTITY_I: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_I);
pub const EL2502_IDENTITY_J: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_J);
pub const EL2502_IDENTITY_K: SubDeviceIdentityTuple =
    (EL2502_VENDOR_ID, EL2502_PRODUCT_ID, EL2502_REVISION_K);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_duty_percent_channel1() {
        let mut dev = EL2502::new();
        dev.set_duty_percent(0, 42.5);
        assert_eq!(dev.duty_raw(0), Some(4250));
        assert_eq!(dev.duty_raw(1), Some(0));
    }

    #[test]
    fn test_set_duty_percent_channel2() {
        let mut dev = EL2502::new();
        dev.set_duty_percent(1, 90.0);
        assert_eq!(dev.duty_raw(1), Some(9000));
    }

    #[test]
    fn test_invalid_port_is_ignored() {
        let mut dev = EL2502::new();
        dev.set_duty_percent(3, 50.0);
        assert_eq!(dev.duty_raw(3), None);
    }
}
