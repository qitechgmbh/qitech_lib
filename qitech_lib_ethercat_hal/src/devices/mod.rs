pub mod beckhoff_modules;
pub mod panasonic_modules;
pub mod wago_modules;
use crate::{DeviceError, MetaSubdevice, PdoError};
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use std::{any::Any, fmt::Debug};

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub slot: u16,
    pub belongs_to_addr: u16,
    pub has_tx: bool,
    pub has_rx: bool,
    pub vendor_id: u32,
    pub product_id: u32,
    pub tx_offset: usize,
    pub rx_offset: usize,
}

/// A trait for all devices
///
/// provides interface to read and write the PDO data
pub trait EthercatDevice
where
    Self: NewEthercatDevice + EthercatDeviceProcessing + Any + Send + Sync + Debug,
{
    /// Input data from the last cycle
    /// `ts` is the timestamp when the input data was sent by the device
    fn input(&mut self, _input: &BitSlice<u8, Lsb0>) -> Result<(), PdoError>;

    /// The accepted length of the input data
    fn input_len(&self) -> usize;

    /// automatically validate input length, then calls input
    fn input_checked(&mut self, input: &BitSlice<u8, Lsb0>) -> Result<(), PdoError> {
        self.input(input)
    }

    /// Output data for the next cycle
    /// `ts` is the timestamp when the output data is predicted to be received by the device
    fn output(&self, _output: &mut BitSlice<u8, Lsb0>) -> Result<(), PdoError>;

    /// The accepted length of the output data
    fn output_len(&self) -> usize;

    fn output_checked(&self, output: &mut BitSlice<u8, Lsb0>) -> Result<(), PdoError> {
        self.output(output)?;
        Ok(())
    }

    fn into_any_boxed(self: Box<Self>) -> Box<dyn Any>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn is_module(&self) -> bool;
    fn get_module(&self) -> Option<Module>;
    fn set_module(&mut self, module: Module);
}

pub trait DynamicEthercatDevice: EthercatDevice + EthercatDynamicPDO {}
pub trait EthercatDynamicPDO {
    fn get_tx_offset(&self) -> usize;
    fn get_rx_offset(&self) -> usize;
    fn set_tx_offset(&mut self, offset: usize);
    fn set_rx_offset(&mut self, offset: usize);
}

/// A trait for devices that want to process input and output data
pub trait EthercatDeviceProcessing {
    /// Devices can override this function if they want to post process the input data
    /// This might be the case if the pdo is not what is needed in the io layer
    fn input_post_process(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Devices can override this function if they want to pre process the output data
    /// This might be the case if the pdo is not what is needed in the io layer
    fn output_pre_process(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// A constructor trait for devices
pub trait NewEthercatDevice {
    /// Create a new device
    fn new() -> Self
    where
        Self: Sized;
}

/// The vendor and product ID of a device and the revisions its driver supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthercatDeviceIdentity {
    pub vendor: u32,
    pub product: u32,
    pub revisions: &'static [u32],
}

impl EthercatDeviceIdentity {
    pub fn matches(&self, subdevice: &MetaSubdevice) -> bool {
        subdevice.vendor == self.vendor
            && subdevice.product_id == self.product
            && self.revisions.contains(&subdevice.revision)
    }
}

/// A driver for one kind of subdevice. Find a subdevice for it with
/// `EL2004::IDENTITY.matches(&subdevice)`.
pub trait IdentifiedDevice {
    const IDENTITY: EthercatDeviceIdentity;
}

// vendor id at 0, and prod id at 1
pub type SubDeviceProductTuple = (u32, u32);
