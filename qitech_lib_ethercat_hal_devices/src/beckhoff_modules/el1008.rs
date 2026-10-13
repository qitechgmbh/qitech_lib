use super::{
    EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice, NewEthercatDevice,
};
use qitech_lib_ethercat_hal::DeviceError;
use qitech_lib_ethercat_hal::io::digital_input::DigitalInputDevice;
use qitech_lib_ethercat_hal::pdo::basic::BoolPdoObject;
use qitech_lib_ethercat_hal::pdo::{PredefinedPdoAssignment, TxPdo};
use qitech_lib_ethercat_hal_derive::{EthercatDevice, TxPdo};

/// EL1008 8-channel digital input device
///
/// 24V DC, 3ms filter
#[derive(Clone, EthercatDevice)]
pub struct EL1008 {
    pub txpdo: EL1008TxPdo,
}

impl EthercatDeviceProcessing for EL1008 {}

impl std::fmt::Debug for EL1008 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL1008")
    }
}

impl NewEthercatDevice for EL1008 {
    fn new() -> Self {
        let s = Self {
            txpdo: EL1008TxPdo::default(),
        };
        s
    }
}

impl DigitalInputDevice for EL1008 {
    fn get_input(&self, port: usize) -> Result<bool, DeviceError> {
        let val = match port {
            0 => {
                self.txpdo
                    .channel1
                    .as_ref()
                    .expect("EL1008 Channel 1 not found")
                    .value
            }
            1 => {
                self.txpdo
                    .channel2
                    .as_ref()
                    .expect("EL1008 Channel 2 not found")
                    .value
            }
            2 => {
                self.txpdo
                    .channel3
                    .as_ref()
                    .expect("EL1008 Channel 3 not found")
                    .value
            }
            3 => {
                self.txpdo
                    .channel4
                    .as_ref()
                    .expect("EL1008 Channel 4 not found")
                    .value
            }
            4 => {
                self.txpdo
                    .channel5
                    .as_ref()
                    .expect("EL1008 Channel 5 not found")
                    .value
            }
            5 => {
                self.txpdo
                    .channel6
                    .as_ref()
                    .expect("EL1008 Channel 6 not found")
                    .value
            }
            6 => {
                self.txpdo
                    .channel7
                    .as_ref()
                    .expect("EL1008 Channel 7 not found")
                    .value
            }
            7 => {
                self.txpdo
                    .channel8
                    .as_ref()
                    .expect("EL1008 Channel 8 not found")
                    .value
            }
            _ => {
                return Err(DeviceError::InvalidPort {
                    device: "EL1008",
                    port,
                });
            }
        };
        Ok(val)
    }

    fn get_port_count(&self) -> usize {
        8
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL1008Port {
    DI1,
    DI2,
    DI3,
    DI4,
    DI5,
    DI6,
    DI7,
    DI8,
}

impl EL1008Port {
    pub const fn to_bit_index(&self) -> usize {
        match self {
            Self::DI1 => 0,
            Self::DI2 => 1,
            Self::DI3 => 2,
            Self::DI4 => 3,
            Self::DI5 => 4,
            Self::DI6 => 5,
            Self::DI7 => 6,
            Self::DI8 => 7,
        }
    }
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL1008TxPdo {
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
}

impl Default for EL1008TxPdo {
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

#[derive(Debug, Clone)]
pub enum EL1008PredefinedPdoAssignment {
    All,
}

impl PredefinedPdoAssignment<EL1008TxPdo, ()> for EL1008PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL1008TxPdo {
        match self {
            Self::All => EL1008TxPdo {
                channel1: Some(BoolPdoObject::default()),
                channel2: Some(BoolPdoObject::default()),
                channel3: Some(BoolPdoObject::default()),
                channel4: Some(BoolPdoObject::default()),
                channel5: Some(BoolPdoObject::default()),
                channel6: Some(BoolPdoObject::default()),
                channel7: Some(BoolPdoObject::default()),
                channel8: Some(BoolPdoObject::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) {
        unreachable!()
    }
}

impl IdentifiedDevice for EL1008 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 0x03f03052,
        revisions: &[0x00120000, 0x00110000],
    };
}
