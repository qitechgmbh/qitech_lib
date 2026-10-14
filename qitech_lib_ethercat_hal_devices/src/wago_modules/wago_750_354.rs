use crate::{
    EthercatDevice, EthercatDeviceIdentity, EthercatDeviceProcessing,
    IdentifiedDevice, Module, NewEthercatDevice,
    wago_modules::ec_box::{get_module_count, get_modules},
};
use qitech_lib_ethercat_hal::EtherCATThreadChannel;
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
        /*for slot_device in &mut self.slot_devices {
            match slot_device {
                Some(device) => {
                    let _ = device.input(input);
                }
                None => break,
            }
        }*/

        Ok(())
    }

    fn input_len(&self) -> usize {
        self.tx_size
    }

    fn output(
        &self,
        _output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), PdoError> {
/*        for slot_device in &self.slot_devices {
            match slot_device {
                Some(device) => {
                    let _ = device.output(output);
                }
                None => break,
            }
        }*/
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

impl Wago750_354 {
    /*pub fn initialize_modules(
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<Vec<Module>, qitech_lib_ethercat_hal::Error> {
        let count = match get_module_count(ecat_channel.clone(), device_address) {
            Ok(count) => count,
            Err(e) => return Err(e),
        };
        if count == 0 {
            return Ok(vec![]);
        }
        let modules = get_modules(
            ecat_channel,
            device_address,            
            Wago750_354::IDENTITY.vendor,
        )?;
        Ok(modules)
    }*/
}

impl IdentifiedDevice for Wago750_354 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x00000021,
        product: 0x07500354,
        revisions: &[0x2],
    };
}
