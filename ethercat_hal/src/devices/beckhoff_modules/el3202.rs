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

/// Sensor element enumeration of the EL32xx family
/// (CoE object 0x8xx0:19 "RTD element"; values from the Beckhoff ESI, 2026-11-10)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtdElement {
    Pt100 = 0,
    Ni100 = 1,
    /// Default for the qi-heating application (system is PT1000 throughout)
    Pt1000 = 2,
    Pt500 = 3,
    Pt200 = 4,
    Ni120 = 5,
    Ni1000 = 6,
}

/// Runtime configuration of an EL3202 (written via CoE at PreOp)
#[derive(Debug, Clone)]
pub struct EL3202Configuration {
    /// Element for channel 1 (CoE 0x8000:19)
    pub element_ch1: RtdElement,
    /// Element for channel 2 (CoE 0x8010:19)
    pub element_ch2: RtdElement,
}

impl Default for EL3202Configuration {
    fn default() -> Self {
        // qi-heating: alle Fühler sind PT1000
        Self {
            element_ch1: RtdElement::Pt1000,
            element_ch2: RtdElement::Pt1000,
        }
    }
}

impl crate::coe::Configuration for EL3202Configuration {
    fn write_config(
        &self,
        channel: crate::EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        channel.sdo_write(device_address, 0x8000, 0x19, self.element_ch1 as u16)?;
        channel.sdo_write(device_address, 0x8010, 0x19, self.element_ch2 as u16)?;
        Ok(())
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
