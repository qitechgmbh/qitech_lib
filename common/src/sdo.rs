use std::any::TypeId;

use crate::pdo::{RxPdo, TxPdo};

pub const RX_PDO_ASSIGNMENT_REG: u16 = 0x1C12;
pub const TX_PDO_ASSIGNMENT_REG: u16 = 0x1C13;

/// Full CoE configuration of a device: device specific parameters plus the PDO assignment.
///
/// The config only selects which PDO structs are used, the SM assignment is derived from them
/// (see [`SmConfiguration`]), so the written assignment always matches the decoded process image.
pub trait Configuration {
    type TxPdo: TxPdo + SmConfiguration;
    type RxPdo: RxPdo + SmConfiguration;

    fn txpdo_assignment(&self) -> Self::TxPdo;
    fn rxpdo_assignment(&self) -> Self::RxPdo;

    /// Device specific CoE parameters, without the PDO assignment
    fn get_config_coe_writes(
        &self,
        _device_address: u16,
    ) -> Result<Vec<SdoRequest>, anyhow::Error> {
        Ok(vec![])
    }

    /// PDO assignment writes for 0x1C13 and 0x1C12
    fn get_sm_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        let mut writes = self.txpdo_assignment().get_sm_coe_writes(device_address)?;
        writes.extend(self.rxpdo_assignment().get_sm_coe_writes(device_address)?);
        Ok(writes)
    }

    /// All writes needed to configure the device, ready for a bulk sdo write
    fn get_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        let mut writes = self.get_config_coe_writes(device_address)?;
        writes.extend(self.get_sm_coe_writes(device_address)?);
        Ok(writes)
    }
}

/// Converts the PDO assignment of a PDO struct to a list of sdo writes.
///
/// Derived by the `RxPdo`/`TxPdo` macros from the `#[pdo_object_index]` attributes.
pub trait SmConfiguration {
    /// [`RX_PDO_ASSIGNMENT_REG`] or [`TX_PDO_ASSIGNMENT_REG`]
    const ASSIGNMENT_REG: u16;

    fn get_sm_assignments(&self) -> Vec<u16>;

    fn get_sm_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        let pdo_assignment_reg = Self::ASSIGNMENT_REG;
        let assignments = self.get_sm_assignments();
        let mut sub_index = 0;
        let mut writes = vec![];

        if assignments.is_empty() {
            return Ok(writes);
        }

        // Set len of Mappings to 0 (reset)
        let reset_req = SdoRequest {
            device_address,
            sdo_index: SdoIndex {
                index: pdo_assignment_reg as u32,
                sub_index,
            },
            data: [0, 0, 0, 0],
            type_flag: type_id_to_sdo_type::<u8>()?,
        };
        sub_index += 1;

        writes.push(reset_req);
        // go through all assignments and write them
        for i in 0..assignments.len() {
            let req = SdoRequest {
                device_address,
                sdo_index: SdoIndex {
                    index: pdo_assignment_reg as u32,
                    sub_index,
                },
                data: assignments[i].to_bytes(),
                type_flag: type_id_to_sdo_type::<u16>()?,
            };
            writes.push(req);
            sub_index += 1;
        }

        let len_req = SdoRequest {
            device_address,
            sdo_index: SdoIndex {
                index: pdo_assignment_reg as u32,
                sub_index: 0,
            },
            data: ((sub_index - 1) as u8).to_bytes(),
            type_flag: type_id_to_sdo_type::<u8>()?,
        };
        writes.push(len_req);

        Ok(writes)
    }
}

#[derive(Debug)]
pub enum SdoType {
    BOOL,
    U8,
    U16,
    U32,
    I16,
    I32,
}

pub fn type_id_to_sdo_type<T: 'static>() -> Result<SdoType, anyhow::Error> {
    let t_id = TypeId::of::<T>();
    let sdo_type: SdoType = {
        if t_id == TypeId::of::<bool>() {
            SdoType::BOOL
        } else if t_id == TypeId::of::<u8>() {
            SdoType::U8
        } else if t_id == TypeId::of::<u16>() {
            SdoType::U16
        } else if t_id == TypeId::of::<u32>() {
            SdoType::U32
        } else if t_id == TypeId::of::<i16>() {
            SdoType::I16
        } else if t_id == TypeId::of::<i32>() {
            SdoType::I32
        } else {
            SdoType::U8
        }
    };
    return Ok(sdo_type);
}

#[derive(Debug)]
pub struct SdoReadRequest {
    pub device_address: u16,
    pub index: u16,
    pub sub_index: u16,
    pub type_flag: SdoType,
}

#[derive(Debug)]
pub struct SdoRequest {
    pub device_address: u16,
    pub sdo_index: SdoIndex,
    pub data: [u8; 4],
    pub type_flag: SdoType,
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Clone, Debug, Copy)]
pub struct SdoIndex {
    pub index: u32,
    pub sub_index: u16,
}

pub trait EthercatResponseTypedResult: Sized {
    fn from_bool(_v: bool) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from bool not supported"))
    }
    fn from_u8(_v: u8) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from u8 not supported"))
    }
    fn from_u16(_v: u16) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from u16 not supported"))
    }
    fn from_i16(_v: i16) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from i16 not supported"))
    }
    fn from_u32(_v: u32) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from u32 not supported"))
    }
    fn from_i32(_v: i32) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from i32 not supported"))
    }
}

macro_rules! impl_ethercat_typed_result {
    ($t:ty, $func:ident) => {
        impl EthercatResponseTypedResult for $t {
            fn $func(v: $t) -> anyhow::Result<Self> {
                Ok(v)
            }
        }
    };
}

impl_ethercat_typed_result!(bool, from_bool);
impl_ethercat_typed_result!(u8, from_u8);
impl_ethercat_typed_result!(u16, from_u16);
impl_ethercat_typed_result!(i16, from_i16);
impl_ethercat_typed_result!(u32, from_u32);
impl_ethercat_typed_result!(i32, from_i32);
pub trait EthercatSdoBytes {
    fn size(&self) -> usize;
    fn to_bytes(&self) -> [u8; 4];
    fn from_bytes(bytes: [u8; 4]) -> Self
    where
        Self: Sized;
}

impl EthercatSdoBytes for u8 {
    fn size(&self) -> usize {
        1
    }

    fn to_bytes(&self) -> [u8; 4] {
        [*self, 0, 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        bytes[0]
    }
}

impl EthercatSdoBytes for u16 {
    fn size(&self) -> usize {
        2
    }

    fn to_bytes(&self) -> [u8; 4] {
        let bytes = u16::to_le_bytes(*self);
        [bytes[0], bytes[1], 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl EthercatSdoBytes for i16 {
    fn size(&self) -> usize {
        2
    }

    fn to_bytes(&self) -> [u8; 4] {
        let bytes = i16::to_le_bytes(*self);
        [bytes[0], bytes[1], 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        i16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl EthercatSdoBytes for i32 {
    fn size(&self) -> usize {
        4
    }

    fn to_bytes(&self) -> [u8; 4] {
        i32::to_le_bytes(*self)
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        i32::from_le_bytes(bytes)
    }
}

impl EthercatSdoBytes for u32 {
    fn size(&self) -> usize {
        4
    }

    fn to_bytes(&self) -> [u8; 4] {
        u32::to_le_bytes(*self)
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        u32::from_le_bytes(bytes)
    }
}

impl EthercatSdoBytes for bool {
    fn size(&self) -> usize {
        1
    }

    fn to_bytes(&self) -> [u8; 4] {
        [*self as u8, 0, 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        bytes[0] != 0
    }
}
