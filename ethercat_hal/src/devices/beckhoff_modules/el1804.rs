use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::digital_input::DigitalInputDevice;
use crate::pdo::basic::BoolPdoObject;
use crate::pdo::TxPdo;
use ethercat_hal_derive::{EthercatDevice, TxPdo};

/// EL1804 4-channel digital input device
///
/// 24V DC, 3 ms filter — sibling of EL1814 (10 µs). PDO layout per official
/// Beckhoff ESI (EL1xxx.xml, verified 2026-11-10): 4 × BoolPdoObject,
/// TxPDO objects 0x1A00..0x1A03. Identity: product 0x070C3052.
#[derive(Clone, EthercatDevice)]
pub struct EL1804 {
    pub txpdo: EL1804TxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL1804 {}

impl std::fmt::Debug for EL1804 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL1804")
    }
}

impl NewEthercatDevice for EL1804 {
    fn new() -> Self {
        Self {
            txpdo: EL1804TxPdo::default(),
            is_used: false,
        }
    }
}

impl DigitalInputDevice for EL1804 {
    fn get_input(&self, port: usize) -> Result<bool, anyhow::Error> {
        let channel = match port {
            0 => self.txpdo.channel1.as_ref().expect("channel1 not found"),
            1 => self.txpdo.channel2.as_ref().expect("channel2 not found"),
            2 => self.txpdo.channel3.as_ref().expect("channel3 not found"),
            3 => self.txpdo.channel4.as_ref().expect("channel4 not found"),
            _ => return Err(anyhow::anyhow!("EL1804 has 4 ports! (0-3)")),
        };
        Ok(channel.value)
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL1804Port {
    DI1,
    DI2,
    DI3,
    DI4,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL1804TxPdo {
    #[pdo_object_index(0x1A00)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A01)]
    pub channel2: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A02)]
    pub channel3: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A03)]
    pub channel4: Option<BoolPdoObject>,
}

impl Default for EL1804TxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
            channel3: Some(BoolPdoObject::default()),
            channel4: Some(BoolPdoObject::default()),
        }
    }
}

pub const EL1804_VENDOR_ID: u32 = 0x2;
pub const EL1804_PRODUCT_ID: u32 = 0x070C3052;
pub const EL1804_REVISION_A: u32 = 0x00100000;
pub const EL1804_REVISION_B: u32 = 0x00110000;
pub const EL1804_REVISION_C: u32 = 0x00120000;

pub const EL1804_IDENTITY_A: SubDeviceIdentityTuple =
    (EL1804_VENDOR_ID, EL1804_PRODUCT_ID, EL1804_REVISION_A);
pub const EL1804_IDENTITY_B: SubDeviceIdentityTuple =
    (EL1804_VENDOR_ID, EL1804_PRODUCT_ID, EL1804_REVISION_B);
pub const EL1804_IDENTITY_C: SubDeviceIdentityTuple =
    (EL1804_VENDOR_ID, EL1804_PRODUCT_ID, EL1804_REVISION_C);
