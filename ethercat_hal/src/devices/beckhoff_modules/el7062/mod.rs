use coe::EL7062Configuration;
use ethercat_hal_derive::EthercatDevice;

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::pdo::{PredefinedPdoAssignment, RxPdo, TxPdo};
use anyhow::anyhow;

pub mod coe;
pub mod pdo;

#[derive(EthercatDevice, Clone, Debug)]
pub struct EL7062 {
    pub txpdo: pdo::EL7062TxPdo,
    pub rxpdo: pdo::EL7062RxPdo,
    is_used: bool,
    pub configuration: EL7062Configuration,
}

impl NewEthercatDevice for EL7062 {
    fn new() -> Self {
        let configuration = EL7062Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
            configuration,
        }
    }
}

impl EthercatDeviceProcessing for EL7062 {}

/// Port selector for the two channels of the EL7062.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EL7062Port {
    Ch1,
    Ch2,
}

impl EL7062 {
    /// Get the position feedback of one channel.
    pub fn get_position(&self, port: EL7062Port) -> Result<i32, anyhow::Error> {
        let position = match port {
            EL7062Port::Ch1 => &self.txpdo.ch1_position,
            EL7062Port::Ch2 => &self.txpdo.ch2_position,
        };
        position
            .as_ref()
            .map(|p| p.position)
            .ok_or_else(|| anyhow!("Position PDO of channel {:?} is None", port))
    }

    /// Get the status word of one channel.
    pub fn get_statusword(&self, port: EL7062Port) -> Result<pdo::DrvStatusWord, anyhow::Error> {
        let statusword = match port {
            EL7062Port::Ch1 => &self.txpdo.ch1_statusword,
            EL7062Port::Ch2 => &self.txpdo.ch2_statusword,
        };
        statusword
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("Statusword PDO of channel {:?} is None", port))
    }

    /// Get the following error of one channel.
    pub fn get_following_error(&self, port: EL7062Port) -> Result<i32, anyhow::Error> {
        let following_error = match port {
            EL7062Port::Ch1 => &self.txpdo.ch1_following_error,
            EL7062Port::Ch2 => &self.txpdo.ch2_following_error,
        };
        following_error
            .as_ref()
            .map(|f| f.following_error)
            .ok_or_else(|| anyhow!("Following error PDO of channel {:?} is None", port))
    }

    /// Set the control word of one channel.
    pub fn set_controlword(
        &mut self,
        port: EL7062Port,
        controlword: pdo::DrvControlWord,
    ) -> Result<(), anyhow::Error> {
        let field = match port {
            EL7062Port::Ch1 => &mut self.rxpdo.ch1_controlword,
            EL7062Port::Ch2 => &mut self.rxpdo.ch2_controlword,
        };
        match field {
            Some(value) => {
                *value = controlword;
                Ok(())
            }
            None => Err(anyhow!("Controlword PDO of channel {:?} is None", port)),
        }
    }

    /// Set the target position of one channel.
    pub fn set_target_position(
        &mut self,
        port: EL7062Port,
        position: i32,
    ) -> Result<(), anyhow::Error> {
        let field = match port {
            EL7062Port::Ch1 => &mut self.rxpdo.ch1_position,
            EL7062Port::Ch2 => &mut self.rxpdo.ch2_position,
        };
        match field {
            Some(value) => {
                value.target_position = position;
                Ok(())
            }
            None => Err(anyhow!("Target position PDO of channel {:?} is None", port)),
        }
    }

    /// Set the target velocity of one channel (CSV mode).
    pub fn set_target_velocity(
        &mut self,
        port: EL7062Port,
        velocity: i32,
    ) -> Result<(), anyhow::Error> {
        let field = match port {
            EL7062Port::Ch1 => &mut self.rxpdo.ch1_target_velocity,
            EL7062Port::Ch2 => &mut self.rxpdo.ch2_target_velocity,
        };
        match field {
            Some(value) => {
                value.target_velocity = velocity;
                Ok(())
            }
            None => Err(anyhow!("Target velocity PDO of channel {:?} is None", port)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EL7062Ports {
    pub motor_1: EL7062Port,
    pub motor_2: EL7062Port,
}

pub const EL7062_VENDOR_ID: u32 = 0x2;
pub const EL7062_PRODUCT_ID: u32 = 0x1b963052;
pub const EL7062_REVISION_A: u32 = 0x00100000;
pub const EL7062_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7062_VENDOR_ID, EL7062_PRODUCT_ID, EL7062_REVISION_A);
