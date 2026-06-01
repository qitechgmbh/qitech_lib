use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::digital_input::DigitalInputDevice;
use crate::pdo::basic::BoolPdoObject;
use crate::pdo::{PredefinedPdoAssignment, TxPdo};
use ethercat_hal_derive::{EthercatDevice, TxPdo};

/// EL1124 4-channel digital input device
///
/// 5V DC, 0.05µs filter
#[derive(Clone, EthercatDevice)]
pub struct EL1124 {
    pub txpdo: EL1124TxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL1124 {}

impl std::fmt::Debug for EL1124 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL1124")
    }
}

impl NewEthercatDevice for EL1124 {
    fn new() -> Self {
        let s = Self {
            txpdo: EL1124TxPdo::default(),
            is_used: false,
        };
        s
    }
}

impl DigitalInputDevice for EL1124 {
    fn get_input(&self, port: usize) -> Result<bool, anyhow::Error> {
        let val = match port {
            0 => {
                self.txpdo
                    .channel1
                    .as_ref()
                    .expect("EL1124 Channel 1 not found")
                    .value
            }
            1 => {
                self.txpdo
                    .channel2
                    .as_ref()
                    .expect("EL1124 Channel 2 not found")
                    .value
            }
            2 => {
                self.txpdo
                    .channel3
                    .as_ref()
                    .expect("EL1124 Channel 3 not found")
                    .value
            }
            3 => {
                self.txpdo
                    .channel4
                    .as_ref()
                    .expect("EL1124 Channel 4 not found")
                    .value
            }
            _ => return Err(anyhow::anyhow!("EL1124 has 4 ports! (0-3)")),
        };
        Ok(val)
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum EL1124Port {
    DI1,
    DI2,
    DI3,
    DI4,
}

#[allow(dead_code)]
impl EL1124Port {
    pub const fn to_bit_index(&self) -> usize {
        match self {
            Self::DI1 => 0,
            Self::DI2 => 1,
            Self::DI3 => 2,
            Self::DI4 => 3,
        }
    }
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL1124TxPdo {
    #[pdo_object_index(0x1A00)]
    pub channel1: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A01)]
    pub channel2: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A02)]
    pub channel3: Option<BoolPdoObject>,
    #[pdo_object_index(0x1A03)]
    pub channel4: Option<BoolPdoObject>,
}

impl Default for EL1124TxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(BoolPdoObject::default()),
            channel2: Some(BoolPdoObject::default()),
            channel3: Some(BoolPdoObject::default()),
            channel4: Some(BoolPdoObject::default()),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum EL1124PredefinedPdoAssignment {
    All,
}

impl PredefinedPdoAssignment<EL1124TxPdo, ()> for EL1124PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL1124TxPdo {
        match self {
            Self::All => EL1124TxPdo {
                channel1: Some(BoolPdoObject::default()),
                channel2: Some(BoolPdoObject::default()),
                channel3: Some(BoolPdoObject::default()),
                channel4: Some(BoolPdoObject::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) {
        unreachable!()
    }
}

pub const EL1124_VENDOR_ID: u32 = 0x2;
pub const EL1124_PRODUCT_ID: u32 = 0x04643052;
pub const EL1124_REVISION_A: u32 = 0x00120000;

pub const EL1124_IDENTITY_A: SubDeviceIdentityTuple =
    (EL1124_VENDOR_ID, EL1124_PRODUCT_ID, EL1124_REVISION_A);