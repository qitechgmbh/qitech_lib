use crate::{
    EtherCATThreadChannel,
    coe::{ConfigurableDevice, Configuration},
};
use super::EL4732;

const VALID_OVERSAMPLE_FACTORS: &[usize] = &[1, 2, 3, 4, 5, 8, 10, 16, 20, 25, 32, 40, 50, 100];

/// Configuration for EL4732 2-channel analog output with oversampling
#[derive(Debug, Clone)]
pub struct EL4732Configuration {
    /// Oversampling factor for both channels.
    /// Must be one of: 1, 2, 3, 4, 5, 8, 10, 16, 20, 25, 32, 40, 50, 100
    pub oversample_factor: usize,
}

impl Default for EL4732Configuration {
    fn default() -> Self {
        Self {
            oversample_factor: 1,
        }
    }
}

impl EL4732Configuration {
    pub fn new(oversample_factor: usize) -> Result<Self, anyhow::Error> {
        if !VALID_OVERSAMPLE_FACTORS.contains(&oversample_factor) {
            return Err(anyhow::anyhow!(
                "Invalid oversample factor: {}. Must be one of: {:?}",
                oversample_factor,
                VALID_OVERSAMPLE_FACTORS
            ));
        }
        Ok(Self { oversample_factor })
    }
}

impl Configuration for EL4732Configuration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        let factor = self.oversample_factor as u8;

        // Write number of PDO entries (= oversample factor) into the
        // RxPDO mapping objects for ch1 and ch2.
        // Sub-index 0 holds the entry count for each PDO assignment object.
        ecat_channel.sdo_write(device_address, 0x1600, 0x00, factor)?;
        ecat_channel.sdo_write(device_address, 0x1700, 0x00, factor)?;

        Ok(())
    }
}

impl ConfigurableDevice<EL4732Configuration> for EL4732 {
    fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        config: &EL4732Configuration,
    ) -> Result<(), anyhow::Error> {
        config.write_config(ecat_channel, device_address)?;

        // Rebuild PDOs to match the new oversample factor so the
        // in-memory layout stays in sync with what's on the device.
        self.rxpdo = crate::devices::el4732::EL4732RxPdo::new(config.oversample_factor);
        self.configuration = config.clone();

        Ok(())
    }

    fn get_config(&self) -> EL4732Configuration {
        self.configuration.clone()
    }
}
