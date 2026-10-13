use super::{EthercatDeviceIdentity, IdentifiedDevice, NewEthercatDevice};
use crate::EthercatDeviceProcessing;
use qitech_lib_ethercat_hal::DeviceError;
use qitech_lib_ethercat_hal::io::digital_input::DigitalInputDevice;
use qitech_lib_ethercat_hal::pdo::{PredefinedPdoAssignment, TxPdo, basic::BoolPdoObject};
use qitech_lib_ethercat_hal_derive::{EthercatDevice, TxPdo};
/// EL1002 2-channel digital input device
/// 24V DC, 3ms filter
#[derive(Clone, EthercatDevice)]
pub struct EL1002 {
    pub txpdo: EL1002TxPdo,
}

impl EthercatDeviceProcessing for EL1002 {}

impl std::fmt::Debug for EL1002 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL1002")
    }
}

impl NewEthercatDevice for EL1002 {
    fn new() -> Self {
        Self {
            txpdo: EL1002TxPdo::default(),
        }
    }
}

impl DigitalInputDevice for EL1002 {
    fn get_input(&self, port: usize) -> Result<bool, DeviceError> {
        let error = DeviceError::InvalidPort {
            device: "EL1002",
            port,
        };

        match port {
            0 => Ok(self.txpdo.channel1.as_ref().ok_or(error)?.value),
            1 => Ok(self.txpdo.channel2.as_ref().ok_or(error)?.value),
            _ => Err(DeviceError::InvalidPort {
                device: "EL1002",
                port,
            }),
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

#[derive(Debug, Clone)]
pub enum EL1002PredefinedPdoAssignment {
    All,
}

impl PredefinedPdoAssignment<EL1002TxPdo, ()> for EL1002PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL1002TxPdo {
        match self {
            Self::All => EL1002TxPdo {
                channel1: Some(BoolPdoObject::default()),
                channel2: Some(BoolPdoObject::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) {
        unreachable!()
    }
}

impl IdentifiedDevice for EL1002 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 65679442,
        revisions: &[1179648],
    };
}
