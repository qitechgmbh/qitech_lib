use crate::{
    EtherCATThreadChannel,
    coe::{ConfigurableDevice, Configuration},
    pdo::PredefinedPdoAssignment,
    shared_config::el70x1::{
        EncConfiguration, PosConfiguration, PosFeatures, StmControllerConfiguration, StmFeatures,
        StmMotorConfiguration,
    },
};

use super::{EL7031, pdo::EL7031PredefinedPdoAssignment};

/// Configuration for EL7031 Stepper Motor Terminal
#[derive(Debug, Clone)]
pub struct EL7031Configuration {
    /// Encoder configuration
    pub encoder: EncConfiguration,

    /// STM motor configuration
    pub stm_motor: StmMotorConfiguration,

    /// STM controller configuration
    pub stm_controller_1: StmControllerConfiguration,

    /// STM controller configuration
    pub stm_controller_2: StmControllerConfiguration,

    /// STM features
    pub stm_features: StmFeatures,

    /// POS configuration
    pub pos_configuration: PosConfiguration,

    /// POS features
    pub pos_features: PosFeatures,

    pub pdo_assignment: EL7031PredefinedPdoAssignment,
}

impl Default for EL7031Configuration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            encoder: EncConfiguration::default(),
            stm_motor: StmMotorConfiguration::default(),
            stm_controller_1: StmControllerConfiguration::default(),
            stm_controller_2: StmControllerConfiguration::default(),
            stm_features: StmFeatures::default(),
            pos_configuration: PosConfiguration::default(),
            pos_features: PosFeatures::default(),
            pdo_assignment: EL7031PredefinedPdoAssignment::default(),
        }
    }
}

impl Configuration for EL7031Configuration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        // All calls are now synchronous and use the (channel, address) pattern
        self.encoder
            .write_config(ecat_channel.clone(), device_address)?;
        self.stm_motor
            .write_config(ecat_channel.clone(), device_address)?;
        // Pass the base_index as the third argument for controllers
        self.stm_controller_1
            .write_config(ecat_channel.clone(), device_address, 0x8011)?;
        self.stm_controller_2
            .write_config(ecat_channel.clone(), device_address, 0x8013)?;
        self.stm_features
            .write_config(ecat_channel.clone(), device_address)?;
        self.pos_configuration
            .write_config(ecat_channel.clone(), device_address)?;
        self.pos_features
            .write_config(ecat_channel.clone(), device_address)?;

        // PDO assignments
        self.pdo_assignment
            .txpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;

        self.pdo_assignment
            .rxpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;

        Ok(())
    }
}

impl ConfigurableDevice<EL7031Configuration> for EL7031 {
    fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        config: &EL7031Configuration,
    ) -> Result<(), anyhow::Error> {
        config.write_config(ecat_channel, device_address)?;
        self.configuration = config.clone();
        self.txpdo = config.pdo_assignment.txpdo_assignment();
        self.rxpdo = config.pdo_assignment.rxpdo_assignment();
        Ok(())
    }

    fn get_config(&self) -> EL7031Configuration {
        self.configuration.clone()
    }
}
