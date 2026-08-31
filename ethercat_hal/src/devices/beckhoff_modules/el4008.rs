use crate::{io::analog_output::AnalogVoltageOutputDevice, pdo::RxPdo};
use ethercat_hal_derive::{EthercatDevice, RxPdo};
use units::{ElectricPotential, electric_potential::volt};

use crate::{
    devices::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple},
    pdo::el40xx::AnalogOutput,
};

/// EL4008 8-channel analog output device
///
/// 12-bit resolution, 0-10V
///
/// load > 5kOhm
#[derive(EthercatDevice)]
pub struct EL4008 {
    pub rxpdo: EL4008RxPdo,
    pub is_used: bool,
}

impl std::fmt::Debug for EL4008 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL4008")
    }
}

impl EthercatDeviceProcessing for EL4008 {}

impl NewEthercatDevice for EL4008 {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            rxpdo: EL4008RxPdo::default(),
            is_used: false,
        }
    }
}

impl AnalogVoltageOutputDevice for EL4008 {
    fn get_port_count(&self) -> usize {
        8
    }

    fn get_minimum_output(&self) -> ElectricPotential {
        ElectricPotential::new::<volt>(0.0)
    }

    fn get_maximum_output(&self) -> ElectricPotential {
        ElectricPotential::new::<volt>(10.0)
    }

    fn set_output_relative(&mut self, port: usize, value: f64) {
        let option = match port {
            0 => self.rxpdo.channel1.as_mut(),
            1 => self.rxpdo.channel2.as_mut(),
            2 => self.rxpdo.channel3.as_mut(),
            3 => self.rxpdo.channel4.as_mut(),
            4 => self.rxpdo.channel5.as_mut(),
            5 => self.rxpdo.channel6.as_mut(),
            6 => self.rxpdo.channel7.as_mut(),
            7 => self.rxpdo.channel8.as_mut(),
            _ => panic!("Port {} index out of range [0, 7]", port),
        };

        option
            .expect("All channels should be Some(_)")
            .set_f64(value);
    }
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL4008RxPdo {
    #[pdo_object_index(0x1600)]
    pub channel1: Option<AnalogOutput>,
    #[pdo_object_index(0x1601)]
    pub channel2: Option<AnalogOutput>,
    #[pdo_object_index(0x1602)]
    pub channel3: Option<AnalogOutput>,
    #[pdo_object_index(0x1603)]
    pub channel4: Option<AnalogOutput>,
    #[pdo_object_index(0x1604)]
    pub channel5: Option<AnalogOutput>,
    #[pdo_object_index(0x1605)]
    pub channel6: Option<AnalogOutput>,
    #[pdo_object_index(0x1606)]
    pub channel7: Option<AnalogOutput>,
    #[pdo_object_index(0x1607)]
    pub channel8: Option<AnalogOutput>,
}

impl Default for EL4008RxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(AnalogOutput::default()),
            channel2: Some(AnalogOutput::default()),
            channel3: Some(AnalogOutput::default()),
            channel4: Some(AnalogOutput::default()),
            channel5: Some(AnalogOutput::default()),
            channel6: Some(AnalogOutput::default()),
            channel7: Some(AnalogOutput::default()),
            channel8: Some(AnalogOutput::default()),
        }
    }
}

pub const EL4008_VENDOR_ID: u32 = 0x2;
pub const EL4008_PRODUCT_ID: u32 = 0x0fa83052;
pub const EL4008_REVISION_A: u32 = 0x00140000;
pub const EL4008_IDENTITY_A: SubDeviceIdentityTuple =
    (EL4008_VENDOR_ID, EL4008_PRODUCT_ID, EL4008_REVISION_A);
