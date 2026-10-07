use super::{NewEthercatDevice, SubDeviceIdentityTuple};
use crate::coe::{ConfigurableDevice, Configuration};
use crate::devices::EthercatDeviceProcessing;
use crate::io::digital_input::DigitalInputDevice;
use common::pdo::{TxPdo, basic::BoolPdoObject};
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};
/// EL1002 2-channel digital input device
/// 24V DC, 3ms filter
#[derive(Clone, EthercatDevice)]
pub struct EL1002 {
    pub txpdo: EL1002TxPdo,
    is_used: bool,
    config: EL1002Configuration,
}

impl EthercatDeviceProcessing for EL1002 {}

impl std::fmt::Debug for EL1002 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL1002")
    }
}

impl NewEthercatDevice for EL1002 {
    fn new() -> Self {
        let config = EL1002Configuration::default();
        Self {
            txpdo: config.txpdo_assignment(),
            is_used: false,
            config,
        }
    }
}

impl ConfigurableDevice for EL1002 {
    type Config = EL1002Configuration;

    fn set_config(&mut self, config: EL1002Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.config = config;
    }

    fn get_config(&self) -> &EL1002Configuration {
        &self.config
    }
}

impl DigitalInputDevice for EL1002 {
    fn get_input(&self, port: usize) -> Result<bool, anyhow::Error> {
        let error = anyhow::anyhow!(
            "[{}::Device::digital_input_state] Port index {} is not available",
            module_path!(),
            port
        );

        match port {
            0 => Ok(self.txpdo.channel1.as_ref().ok_or(error)?.value),
            1 => Ok(self.txpdo.channel2.as_ref().ok_or(error)?.value),
            _ => Err(anyhow::anyhow!(
                "EL1002 has 2 ports (0-1), requested index {}",
                port
            )),
        }
    }

    fn get_port_count(&self) -> usize {
        2
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL1002Port {
    DI1,
    DI2,
}

impl EL1002Port {
    pub const fn to_bit_index(&self) -> usize {
        match self {
            Self::DI1 => 0,
            Self::DI2 => 1,
        }
    }
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL1002TxPdo {
    #[pdo_object_index(0x1A00)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A01)]
    pub channel2: Option<BoolPdoObject>,
}

impl Default for EL1002TxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
        }
    }
}

/// The EL1002 has no outputs
#[derive(Debug, Clone, RxPdo)]
pub struct EL1002RxPdo {}

/// The EL1002 has a fixed PDO assignment and no further CoE parameters
#[derive(Default, Clone, PartialEq, Debug)]
pub struct EL1002Configuration {}

impl Configuration for EL1002Configuration {
    /// The EL1002 has no mailbox, its PDO assignment is fixed and can't be written over CoE
    fn get_sm_coe_writes(
        &self,
        _device_address: u16,
    ) -> Result<Vec<common::SdoRequest>, anyhow::Error> {
        Ok(vec![])
    }

    type TxPdo = EL1002TxPdo;
    type RxPdo = EL1002RxPdo;

    fn txpdo_assignment(&self) -> EL1002TxPdo {
        EL1002TxPdo::default()
    }

    fn rxpdo_assignment(&self) -> EL1002RxPdo {
        EL1002RxPdo {}
    }
}

pub const EL1002_VENDOR_ID: u32 = 0x2;
pub const EL1002_PRODUCT_ID: u32 = 65679442;
pub const EL1002_REVISION_A: u32 = 1179648;
pub const EL1002_IDENTITY_A: SubDeviceIdentityTuple =
    (EL1002_VENDOR_ID, EL1002_PRODUCT_ID, EL1002_REVISION_A);
