use qitech_lib_ethercat_hal::{
    device::{ConfigurableDevice, Configuration},
    pdo::PredefinedPdoAssignment,
    sdo::SdoRequest,
    shared_config::el70x7::{
        EncConfiguration, PosConfiguration, PosFeatures, StmControllerConfiguration,
        StmControllerSettings3Configuration, StmFeatures, StmMotorConfiguration,
    },
};

use super::{
    EL7037,
    pdo::{EL7037PredefinedPdoAssignment, EL7037RxPdo, EL7037TxPdo},
};

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
                select_info_data_1:
                    qitech_lib_ethercat_hal::shared_config::el70x7::EL70x7InfoData::MotorLoad,
                select_info_data_2:
                    qitech_lib_ethercat_hal::shared_config::el70x7::EL70x7InfoData::MotorDcCurrent,
                ..Default::default()
            },
            pos_configuration: PosConfiguration::default(),
            pos_features: PosFeatures::default(),
            pdo_assignment: EL7037PredefinedPdoAssignment::default(),
        }
    }
}

impl Configuration for EL7037Configuration {
    type TxPdo = EL7037TxPdo;
    type RxPdo = EL7037RxPdo;

    fn txpdo_assignment(&self) -> EL7037TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL7037RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(
        &self,
        device_address: u16,
    ) -> Result<Vec<SdoRequest>, qitech_lib_ethercat_hal::Error> {
        let mut writes = Vec::new();
        writes.extend(self.encoder.get_coe_writes(device_address));
        writes.extend(self.stm_motor.get_coe_writes(device_address));
        writes.extend(self.stm_controller_1.get_coe_writes(device_address, 0x8011));
        writes.extend(self.stm_controller_3.get_coe_writes(device_address));
        writes.extend(self.stm_features.get_coe_writes(device_address));
        writes.extend(self.pos_configuration.get_coe_writes(device_address));
        writes.extend(self.pos_features.get_coe_writes(device_address));
        Ok(writes)
    }
}

impl ConfigurableDevice for EL7037 {
    type Config = EL7037Configuration;

    fn set_config(&mut self, config: EL7037Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.rxpdo = config.rxpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL7037Configuration {
        &self.configuration
    }
}
