use crate::{
    coe::{ConfigurableDevice, Configuration},
    pdo::PredefinedPdoAssignment,
    shared_config::el70x1::{
        EncConfiguration, PosConfiguration, PosFeatures, StmControllerConfiguration, StmFeatures,
        StmMotorConfiguration,
    },
};
use common::SdoRequest;

use super::{
    EL7031,
    pdo::{EL7031PredefinedPdoAssignment, EL7031RxPdo, EL7031TxPdo},
};

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
    type TxPdo = EL7031TxPdo;
    type RxPdo = EL7031RxPdo;

    fn txpdo_assignment(&self) -> EL7031TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL7031RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        let mut writes = Vec::new();
        writes.extend(self.encoder.get_coe_writes(device_address));
        writes.extend(self.stm_motor.get_coe_writes(device_address));
        writes.extend(self.stm_controller_1.get_coe_writes(device_address, 0x8011));
        writes.extend(self.stm_controller_2.get_coe_writes(device_address, 0x8013));
        writes.extend(self.stm_features.get_coe_writes(device_address));
        writes.extend(self.pos_configuration.get_coe_writes(device_address));
        writes.extend(self.pos_features.get_coe_writes(device_address));
        Ok(writes)
    }
}

impl ConfigurableDevice for EL7031 {
    type Config = EL7031Configuration;

    fn set_config(&mut self, config: EL7031Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.rxpdo = config.rxpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL7031Configuration {
        &self.configuration
    }
}
