use coe::EL7062Configuration;
use ethercat_hal_derive::EthercatDevice;
use log::warn;

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::pdo::{PredefinedPdoAssignment, RxPdo, TxPdo};
use anyhow::anyhow;

pub mod coe;
pub mod diagnostics;
pub mod motion;
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

/// One channel of the EL7062, borrowed from the device.
///
/// A typed view rather than a `(device, port)` pair, so a getter and a setter
/// cannot be handed different channels and the compiler tracks the borrow.
pub struct Axis<'a> {
    channel: EL7062Port,
    /// The channel's own configuration, used to interpret process data.
    config: &'a coe::El7062ChannelConfiguration,
    position: &'a Option<pdo::FbPosition>,
    statusword: &'a Option<pdo::DrvStatusWord>,
    following_error: &'a Option<pdo::DrvFollowingError>,
    /// The selected channel's RxPDO slots, borrowed out of the whole struct so
    /// the getters above stay `&self`.
    rx: RxPdoSlot<'a>,
}

enum RxPdoSlot<'a> {
    Ch1 {
        controlword: &'a mut Option<pdo::DrvControlWord>,
        position: &'a mut Option<pdo::DrvTargetPosition>,
        target_velocity: &'a mut Option<pdo::DrvTargetVelocity>,
    },
    Ch2 {
        controlword: &'a mut Option<pdo::DrvControlWord>,
        position: &'a mut Option<pdo::DrvTargetPosition>,
        target_velocity: &'a mut Option<pdo::DrvTargetVelocity>,
    },
}

impl EL7062 {
    /// Borrow one channel for reading and writing process data.
    pub fn axis(&mut self, port: EL7062Port) -> Axis<'_> {
        let config = match port {
            EL7062Port::Ch1 => &self.configuration.channel_1,
            EL7062Port::Ch2 => &self.configuration.channel_2,
        };
        let (position, statusword, following_error) = match port {
            EL7062Port::Ch1 => (
                &self.txpdo.ch1_position,
                &self.txpdo.ch1_statusword,
                &self.txpdo.ch1_following_error,
            ),
            EL7062Port::Ch2 => (
                &self.txpdo.ch2_position,
                &self.txpdo.ch2_statusword,
                &self.txpdo.ch2_following_error,
            ),
        };
        let rx = match port {
            EL7062Port::Ch1 => RxPdoSlot::Ch1 {
                controlword: &mut self.rxpdo.ch1_controlword,
                position: &mut self.rxpdo.ch1_position,
                target_velocity: &mut self.rxpdo.ch1_target_velocity,
            },
            EL7062Port::Ch2 => RxPdoSlot::Ch2 {
                controlword: &mut self.rxpdo.ch2_controlword,
                position: &mut self.rxpdo.ch2_position,
                target_velocity: &mut self.rxpdo.ch2_target_velocity,
            },
        };
        Axis {
            channel: port,
            config,
            position,
            statusword,
            following_error,
            rx,
        }
    }
}

impl Axis<'_> {
    /// Which channel this is.
    pub fn port(&self) -> EL7062Port {
        self.channel
    }

    /// The channel's configuration.
    pub fn config(&self) -> &coe::El7062ChannelConfiguration {
        self.config
    }

    /// The channel's statusword monitor, for decoding bit 10.
    pub fn statusword_monitor(&self) -> pdo::StatuswordProcessDataMonitor {
        self.config.amplifier.statusword_monitor
    }

    /// Get the position feedback, in process-data increments.
    pub fn position(&self) -> Result<i32, anyhow::Error> {
        self.position
            .as_ref()
            .map(|p| p.position)
            .ok_or_else(|| anyhow!("Position PDO of channel {:?} is None", self.channel))
    }

    /// Get the status word.
    pub fn statusword(&self) -> Result<pdo::DrvStatusWord, anyhow::Error> {
        self.statusword
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("Statusword PDO of channel {:?} is None", self.channel))
    }

    /// Get the following error, in process-data increments.
    ///
    /// Only mapped in CSP; other modes leave it `None`.
    pub fn following_error(&self) -> Result<i32, anyhow::Error> {
        self.following_error
            .as_ref()
            .map(|f| f.following_error)
            .ok_or_else(|| anyhow!("Following error PDO of channel {:?} is None", self.channel))
    }

    /// Set the control word.
    pub fn set_controlword(
        &mut self,
        controlword: pdo::DrvControlWord,
    ) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 { controlword, .. } => controlword,
            RxPdoSlot::Ch2 { controlword, .. } => controlword,
        };
        match field {
            Some(value) => {
                *value = controlword;
                Ok(())
            }
            None => Err(anyhow!(
                "Controlword PDO of channel {:?} is None",
                self.channel
            )),
        }
    }

    /// Set the target position, in process-data increments.
    ///
    /// Only mapped in CSP; other modes leave it `None`.
    pub fn set_target_position(&mut self, position: i32) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 { position, .. } => position,
            RxPdoSlot::Ch2 { position, .. } => position,
        };
        match field {
            Some(value) => {
                value.target_position = position;
                Ok(())
            }
            None => Err(anyhow!(
                "Target position PDO of channel {:?} is None (CSP only)",
                self.channel
            )),
        }
    }

    /// Set the target velocity (CSV mode), in increments per second.
    pub fn set_target_velocity(&mut self, velocity: i32) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 {
                target_velocity, ..
            } => target_velocity,
            RxPdoSlot::Ch2 {
                target_velocity, ..
            } => target_velocity,
        };
        match field {
            Some(value) => {
                value.target_velocity = velocity;
                Ok(())
            }
            None => Err(anyhow!(
                "Target velocity PDO of channel {:?} is None (CSV only)",
                self.channel
            )),
        }
    }

    /// Drive the CiA402 state machine via the control word.
    ///
    /// Steps the drive towards `Operation enabled` (fault reset is left to the
    /// caller, which may want to latch faults instead of auto-clearing them).
    /// Bits on the EL7062: 0=switch on, 1=enable voltage, 3=enable operation.
    pub fn apply_controlword(
        &mut self,
        statusword: &pdo::DrvStatusWord,
    ) -> Result<(), anyhow::Error> {
        if statusword.fault {
            warn!(
                "Fault detected! Applying fault reset to EL7062 ({:?})",
                self.channel
            );
            self.set_controlword(pdo::DrvControlWord {
                fault_reset: true,
                ..Default::default()
            })?;
        } else if !statusword.ready_to_switch_on {
            self.set_controlword(pdo::DrvControlWord {
                enable_voltage: true,
                quick_stop: true,
                ..Default::default()
            })?;
        } else if !statusword.switched_on {
            self.set_controlword(pdo::DrvControlWord {
                switch_on: true,
                enable_voltage: true,
                quick_stop: true,
                ..Default::default()
            })?;
        } else {
            self.set_controlword(pdo::DrvControlWord {
                switch_on: true,
                enable_voltage: true,
                quick_stop: true,
                enable_operation: true,
                ..Default::default()
            })?;
        }
        Ok(())
    }
}

pub const EL7062_VENDOR_ID: u32 = 0x2;
pub const EL7062_PRODUCT_ID: u32 = 0x1b963052;
pub const EL7062_REVISION_A: u32 = 0x00100000;
pub const EL7062_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7062_VENDOR_ID, EL7062_PRODUCT_ID, EL7062_REVISION_A);
