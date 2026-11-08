use crate::pdo::TxPdo;
use crate::{
    io::temperature_input::{TemperatureInputDevice, TemperatureInputInput},
    pdo::el32xx::RtdInput,
};
use ethercat_hal_derive::{EthercatDevice, TxPdo};

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};

/// EL3202 2-channel temperature input device
///
/// PT100 / PT1000 / Ni100 (RTD) (2 wire). Same per-channel PDO layout as
/// EL3204 (RtdInput, TxPDO objects 0x1A00/0x1A01). Identity verified against
/// the official Beckhoff ESI file (EL32xx.xml, product 0x0C823052) on 2026-11-08.
#[derive(EthercatDevice)]
pub struct EL3202 {
    pub txpdo: EL3202TxPdo,
    is_used: bool,
}
impl EthercatDeviceProcessing for EL3202 {}

impl std::fmt::Debug for EL3202 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3202")
    }
}

impl NewEthercatDevice for EL3202 {
    fn new() -> Self {
        Self {
            txpdo: EL3202TxPdo::default(),
            is_used: false,
        }
    }
}

impl TemperatureInputDevice for EL3202 {
    fn get_input(&self, port: usize) -> Result<TemperatureInputInput, anyhow::Error> {
        let expect_text = "All channels should be Some(_)";
        let channel = match port {
            0 => self.txpdo.channel1.as_ref().expect(expect_text),
            1 => self.txpdo.channel2.as_ref().expect(expect_text),
            _ => return Err(anyhow::anyhow!("port {} does not exist on EL3202 !", port)),
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
        2
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL3202Port {
    T1,
    T2,
}

impl EL3202Port {
    pub const fn to_byte_offset(&self) -> usize {
        match self {
            Self::T1 => 0,
            Self::T2 => 4,
        }
    }
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL3202TxPdo {
    #[pdo_object_index(0x1A00)]
    channel1: Option<RtdInput>,
    #[pdo_object_index(0x1A01)]
    channel2: Option<RtdInput>,
}

impl Default for EL3202TxPdo {
    fn default() -> Self {
        Self {
            channel1: Some(RtdInput::default()),
            channel2: Some(RtdInput::default()),
        }
    }
}

pub const EL3202_VENDOR_ID: u32 = 0x2;
pub const EL3202_PRODUCT_ID: u32 = 0x0c823052;
pub const EL3202_REVISION_A: u32 = 0x00100000;
pub const EL3202_REVISION_B: u32 = 0x00110000;
pub const EL3202_REVISION_C: u32 = 0x00120000;
pub const EL3202_REVISION_D: u32 = 0x00130000;
pub const EL3202_REVISION_E: u32 = 0x00140000;
pub const EL3202_REVISION_F: u32 = 0x00150000;
pub const EL3202_REVISION_G: u32 = 0x00160000;

pub const EL3202_IDENTITY_A: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_A);
pub const EL3202_IDENTITY_B: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_B);
pub const EL3202_IDENTITY_C: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_C);
pub const EL3202_IDENTITY_D: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_D);
pub const EL3202_IDENTITY_E: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_E);
pub const EL3202_IDENTITY_F: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_F);
pub const EL3202_IDENTITY_G: SubDeviceIdentityTuple =
    (EL3202_VENDOR_ID, EL3202_PRODUCT_ID, EL3202_REVISION_G);
