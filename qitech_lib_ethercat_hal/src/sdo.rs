use crate::ChannelError;
pub const RX_PDO_ASSIGNMENT_REG: u16 = 0x1C12;
pub const TX_PDO_ASSIGNMENT_REG: u16 = 0x1C13;

#[derive(Debug, Clone, Copy)]
pub enum SdoType {
    BOOL,
    U8,
    U16,
    U32,
    I16,
    I32,
}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Clone, Copy)]
pub struct SdoIndex {
    pub index: u32,
    pub sub_index: u16,
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

impl SdoRequest {
    /// Typed sdo write of `value` to `index:sub_index`
    pub fn new<T: EthercatSdoBytes>(
        device_address: u16,
        index: u16,
        sub_index: u8,
        value: T,
    ) -> Self {
        Self {
            device_address,
            sdo_index: SdoIndex {
                index: index as u32,
                sub_index: sub_index as u16,
            },
            data: value.to_bytes(),
            type_flag: T::SDO_TYPE,
        }
    }
}

/// Values that can be sent as an SDO: their wire type and their little endian bytes.
pub trait EthercatSdoBytes {
    const SDO_TYPE: SdoType;

    fn size(&self) -> usize;
    fn to_bytes(&self) -> [u8; 4];
    fn from_bytes(bytes: [u8; 4]) -> Self
    where
        Self: Sized;
}

macro_rules! impl_ethercat_sdo_bytes {
    ($t:ty, $sdo_type:ident) => {
        impl EthercatSdoBytes for $t {
            const SDO_TYPE: SdoType = SdoType::$sdo_type;

            fn size(&self) -> usize {
                size_of::<$t>()
            }

            fn to_bytes(&self) -> [u8; 4] {
                let mut bytes = [0; 4];
                bytes[..size_of::<$t>()].copy_from_slice(&self.to_le_bytes());
                bytes
            }

            fn from_bytes(bytes: [u8; 4]) -> Self {
                let mut value = [0; size_of::<$t>()];
                value.copy_from_slice(&bytes[..size_of::<$t>()]);
                <$t>::from_le_bytes(value)
            }
        }
    };
}

impl_ethercat_sdo_bytes!(u8, U8);
impl_ethercat_sdo_bytes!(u16, U16);
impl_ethercat_sdo_bytes!(i16, I16);
impl_ethercat_sdo_bytes!(u32, U32);
impl_ethercat_sdo_bytes!(i32, I32);

impl EthercatSdoBytes for bool {
    const SDO_TYPE: SdoType = SdoType::BOOL;

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

/// Turns the typed answer of an sdo read back into the requested type.
pub trait EthercatResponseTypedResult: Sized {
    fn from_bool(_v: bool) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "bool",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_u8(_v: u8) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "u8",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_u16(_v: u16) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "u16",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_i16(_v: i16) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "i16",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_u32(_v: u32) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "u32",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_i32(_v: i32) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "i32",
            std::any::type_name::<Self>(),
        ))
    }
}

macro_rules! impl_ethercat_typed_result {
    ($t:ty, $func:ident) => {
        impl EthercatResponseTypedResult for $t {
            fn $func(v: $t) -> Result<Self, ChannelError> {
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

#[cfg(test)]
mod tests {
    use crate::device::SmConfiguration;

    use super::*;

    struct TwoObjects;

    impl SmConfiguration for TwoObjects {
        const ASSIGNMENT_REG: u16 = TX_PDO_ASSIGNMENT_REG;

        fn get_sm_assignments(&self) -> Vec<u16> {
            vec![0x1A00, 0x1A01]
        }
    }

    #[test]
    fn sm_coe_writes_clear_assign_then_set_length() {
        let writes = TwoObjects.get_sm_coe_writes(0x1001).unwrap();
        let summary: Vec<_> = writes
            .iter()
            .map(|w| (w.sdo_index.index, w.sdo_index.sub_index, w.data))
            .collect();
        assert_eq!(
            summary,
            vec![
                (0x1C13, 0, [0, 0, 0, 0]),
                (0x1C13, 1, [0x00, 0x1A, 0, 0]),
                (0x1C13, 2, [0x01, 0x1A, 0, 0]),
                (0x1C13, 0, [2, 0, 0, 0]),
            ]
        );
        assert!(matches!(writes[0].type_flag, SdoType::U8));
        assert!(matches!(writes[1].type_flag, SdoType::U16));
    }

    #[test]
    fn sdo_bytes_round_trip() {
        assert_eq!(i16::from_bytes((-2i16).to_bytes()), -2);
        assert_eq!((-2i16).to_bytes(), [0xFE, 0xFF, 0, 0]);
        assert_eq!(u32::from_bytes(0x12345678u32.to_bytes()), 0x12345678);
    }
}
