use crate::{
    EthercatDevice, EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice,
    NewEthercatDevice,
};
use qitech_lib_ethercat_hal::PdoError;

// For both the rx and tx The Wago Coupler has 4 bytes, which we dont care about and skip
pub struct Wago750_354 {
    pub dev_count: usize,
    pub module_count: usize,
    tx_size: usize,
    rx_size: usize,
}

impl EthercatDevice for Wago750_354 {
    fn into_any_boxed(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn input(
        &mut self,
        _input: &bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), PdoError> {
        Ok(())
    }

    fn input_len(&self) -> usize {
        self.tx_size
    }

    fn output(
        &self,
        _output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), PdoError> {
        Ok(())
    }

    fn output_len(&self) -> usize {
        self.rx_size
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn is_module(&self) -> bool {
        false
    }
}

impl EthercatDeviceProcessing for Wago750_354 {}

impl NewEthercatDevice for Wago750_354 {
    fn new() -> Self {
        Self {
            module_count: 0,
            dev_count: 0,
            tx_size: 0,
            rx_size: 0,
        }
    }
}

impl std::fmt::Debug for Wago750_354 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wago_750_354")
    }
}

impl IdentifiedDevice for Wago750_354 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x00000021,
        product: 0x07500354,
        revisions: &[0x2],
    };
}
