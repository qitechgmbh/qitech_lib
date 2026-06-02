use crate::pdo::TxPdo;
use crate::{
    io::temperature_input::{TemperatureInputDevice, TemperatureInputInput},
    pdo::el32xx::RtdInput,
};
use ethercat_hal_derive::{EthercatDevice, TxPdo};

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};

/// EL3204 4-channel temperature input device
///
/// PT100 / Ni100 (RTD) / (2 wire)
#[derive(EthercatDevice)]
pub struct EL3204 {
    pub txpdo: EL3204TxPdo,
    is_used: bool,
}
impl EthercatDeviceProcessing for EL3204 {}

impl std::fmt::Debug for EL3204 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3204")
    }
}

impl NewEthercatDevice for EL3204 {
    fn new() -> Self {
        Self {
            txpdo: EL3204TxPdo::default(),
            is_used: false,
        }
    }
}

impl TemperatureInputDevice for EL3204 {
    fn get_input(&self, port: usize) -> Result<TemperatureInputInput, anyhow::Error> {
        let expect_text = "All channels should be Some(_)";
        let channel = match port {
            0 => self.txpdo.channel1.as_ref().expect(expect_text),
            1 => self.txpdo.channel2.as_ref().expect(expect_text),
            2 => self.txpdo.channel3.as_ref().expect(expect_text),
            3 => self.txpdo.channel4.as_ref().expect(expect_text),
            _ => return Err(anyhow::anyhow!("port {} does not exist on EL3204 !", port)),
        };

        Ok(TemperatureInputInput {
            temperature: channel.temperature,
            undervoltage: channel.undervoltage,
            overvoltage: channel.overvoltage,
            limit1: channel.limit1,
            limit2: channel.limit2,
            error: channel.error,
            txpdo_state: channel.txpdo_state,
            txpdo_toggle: channel.txpdo_toggle,
        })
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL3204Port {
    T1,
    T2,
    T3,
    T4,
}

impl EL3204Port {
    pub const fn to_byte_offset(&self) -> usize {
        match self {
            Self::T1 => 0,
            Self::T2 => 4,
            Self::T3 => 8,
            Self::T4 => 12,
        }
    }
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL3204TxPdo {
    #[pdo_object_index(0x1A00)]
    channel1: Option<RtdInput>,
    #[pdo_object_index(0x1A01)]
    channel2: Option<RtdInput>,
    #[pdo_object_index(0x1A02)]
    channel3: Option<RtdInput>,
    #[pdo_object_index(0x1A03)]
    channel4: Option<RtdInput>,
}

impl Default for EL3204TxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(RtdInput::default()),
            channel2: Some(RtdInput::default()),
            channel3: Some(RtdInput::default()),
            channel4: Some(RtdInput::default()),
        }
    }
}

pub const EL3204_VENDOR_ID: u32 = 0x2;
pub const EL3204_PRODUCT_ID: u32 = 0xc843052;
pub const EL3204_REVISION_A: u32 = 0x160000;
pub const EL3204_REVISION_B: u32 = 0x150000;

pub const EL3204_IDENTITY_A: SubDeviceIdentityTuple =
    (EL3204_VENDOR_ID, EL3204_PRODUCT_ID, EL3204_REVISION_A);

pub const EL3204_IDENTITY_B: SubDeviceIdentityTuple =
    (EL3204_VENDOR_ID, EL3204_PRODUCT_ID, EL3204_REVISION_B);

#[cfg(feature = "mock")]
impl crate::devices::MockEtherCatSdos for EL3204 {
    fn get_sdo_map() -> std::collections::HashMap<crate::SdoIndex, crate::devices::TypeErasedValue>
    {
        let mut map = std::collections::HashMap::new();
        // Helper to insert values into the map
        let mut insert_sdo = |idx: u32, sub: u16, val: u64, size: usize, tid: std::any::TypeId| {
            let bytes = val.to_le_bytes()[..size].to_vec();
            map.insert(
                crate::SdoIndex {
                    index: idx,
                    sub_index: sub,
                },
                crate::devices::TypeErasedValue {
                    type_id: tid,
                    value: bytes,
                },
            );
        };

        // Data for Channels 1 through 4 (0x1a00 to 0x1a03)
        let channels = [
            (
                0x1a00,
                [
                    0x60000101, 0x60000201, 0x60000302, 0x60000502, 0x60000701, 0x00000007,
                    0x60000f01, 0x60001001, 0x60001110,
                ],
            ),
            (
                0x1a01,
                [
                    0x60100101, 0x60100201, 0x60100302, 0x60100502, 0x60100701, 0x00000007,
                    0x60100f01, 0x60101001, 0x60101110,
                ],
            ),
            (
                0x1a02,
                [
                    0x60200101, 0x60200201, 0x60200302, 0x60200502, 0x60200701, 0x00000007,
                    0x60200f01, 0x60201001, 0x60201110,
                ],
            ),
            (
                0x1a03,
                [
                    0x60300101, 0x60300201, 0x60300302, 0x60300502, 0x60300701, 0x00000007,
                    0x60300f01, 0x60301001, 0x60301110,
                ],
            ),
        ];

        for (base_index, sub_values) in channels {
            // SubIndex 0: Number of entries (UNSIGNED8)
            insert_sdo(base_index, 0x00, 0x09, 1, std::any::TypeId::of::<u8>());
            // SubIndices 1 through 9: PDO Mappings (UNSIGNED32)
            for (i, &val) in sub_values.iter().enumerate() {
                insert_sdo(
                    base_index,
                    (i + 1) as u16,
                    val as u64,
                    4,
                    std::any::TypeId::of::<u32>(),
                );
            }
        }
        map
    }
}
