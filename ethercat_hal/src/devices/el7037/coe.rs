use crate::{
    EtherCATThreadChannel,
    coe::{ConfigurableDevice, Configuration},
    pdo::PredefinedPdoAssignment,
    shared_config::el70x7::{
        EncConfiguration, PosConfiguration, PosFeatures, StmControllerConfiguration,
        StmControllerSettings3Configuration, StmFeatures, StmMotorConfiguration,
    },
};

use super::{EL7037, pdo::EL7037PredefinedPdoAssignment};

/// Configuration for EL7037 Stepper Motor Terminal
#[derive(Debug, Clone)]
pub struct EL7037Configuration {
    /// Encoder configuration
    pub encoder: EncConfiguration,

    /// STM motor configuration
    pub stm_motor: StmMotorConfiguration,

    /// STM controller configuration (0x8011, current loop)
    pub stm_controller_1: StmControllerConfiguration,

    /// STM controller configuration (0x8014, position and velocity loop)
    pub stm_controller_3: StmControllerSettings3Configuration,

    /// STM features
    pub stm_features: StmFeatures,

    /// POS configuration
    pub pos_configuration: PosConfiguration,

    /// POS features
    pub pos_features: PosFeatures,

    pub pdo_assignment: EL7037PredefinedPdoAssignment,
}

impl Default for EL7037Configuration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            encoder: EncConfiguration::default(),
            stm_motor: StmMotorConfiguration::default(),
            stm_controller_1: StmControllerConfiguration::default(),
            stm_controller_3: StmControllerSettings3Configuration::default(),
            stm_features: StmFeatures {
                select_info_data_1: crate::shared_config::el70x7::EL70x7InfoData::MotorLoad,
                select_info_data_2: crate::shared_config::el70x7::EL70x7InfoData::MotorDcCurrent,
                ..Default::default()
            },
            pos_configuration: PosConfiguration::default(),
            pos_features: PosFeatures::default(),
            pdo_assignment: EL7037PredefinedPdoAssignment::default(),
        }
    }
}

impl Configuration for EL7037Configuration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        self.encoder
            .write_config(ecat_channel.clone(), device_address)?;
        self.stm_motor
            .write_config(ecat_channel.clone(), device_address)?;
        self.stm_controller_1
            .write_config(ecat_channel.clone(), device_address, 0x8011)?;
        self.stm_controller_3
            .write_config(ecat_channel.clone(), device_address)?;
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

impl ConfigurableDevice<EL7037Configuration> for EL7037 {
    fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        config: &EL7037Configuration,
    ) -> Result<(), anyhow::Error> {
        config.write_config(ecat_channel, device_address)?;
        self.configuration = config.clone();
        self.txpdo = config.pdo_assignment.txpdo_assignment();
        self.rxpdo = config.pdo_assignment.rxpdo_assignment();
        Ok(())
    }

    fn get_config(&self) -> EL7037Configuration {
        self.configuration.clone()
    }
}
