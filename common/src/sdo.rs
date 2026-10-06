use std::any::TypeId;

/// This Trait is to convert a Configuration from struct representation to
/// a list of sdo writes needed to setup device specific coe config
pub trait Configuration {
    fn get_config_coe_writes(&self) -> Result<Vec<SdoRequest>,anyhow::Error>;
}
/// This Trait Converts an SMConfig to a list of sdo writes
pub trait SmConfiguration {
    fn get_sm_assignments(
        &self
    ) -> Result<Vec<SdoIndex>, anyhow::Error>;
    fn get_sm_coe_writes(&self,device_address : u16) -> Result<Vec<SdoRequest>,anyhow::Error>;
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
    pub sdo_index : SdoIndex,
    pub data: [u8; 4],
    pub type_flag: SdoType,
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Clone,Debug,Copy)]
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
