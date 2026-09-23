pub mod coe;
pub mod pdo;

use anyhow::anyhow;
use coe::EL7062Configuration;
use ethercat_hal_derive::EthercatDevice;
use pdo::CSP_MODE;

pub use coe::{DmcDriveStatus, read_dmc_drive_status, read_dmc_error_id};

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::{
    io::servo_position_el7062::{
        ServoPositionEL7062Device, ServoPositionEL7062Input, ServoPositionEL7062Output,
    },
    pdo::{PredefinedPdoAssignment, RxPdo, TxPdo},
};

pub const EL7062_VENDOR_ID: u32 = 0x0000_0002;
pub const EL7062_PRODUCT_ID: u32 = 0x1b96_3052;
pub const EL7062_REVISION_A: u32 = 0x0010_0000;
pub const EL7062_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7062_VENDOR_ID, EL7062_PRODUCT_ID, EL7062_REVISION_A);

/// CiA 402 control word commands
const CONTROL_WORD_DISABLE_VOLTAGE: u16 = 0x0000;
const CONTROL_WORD_SHUTDOWN: u16 = 0x0006;
const CONTROL_WORD_SWITCH_ON: u16 = 0x0007;
const CONTROL_WORD_ENABLE_OPERATION: u16 = 0x000F;
const CONTROL_WORD_FAULT_RESET: u16 = 0x0080;

/// CiA 402 status word bits
const STATUS_READY_TO_SWITCH_ON: u16 = 1 << 0;
const STATUS_SWITCHED_ON: u16 = 1 << 1;
const STATUS_OPERATION_ENABLED: u16 = 1 << 2;
const STATUS_FAULT: u16 = 1 << 3;
const STATUS_SWITCH_ON_DISABLED: u16 = 1 << 6;
const STATUS_DRIVE_FOLLOWS_COMMAND: u16 = 1 << 12;

/// The CiA 402 drive states relevant for the EL7062
///
/// Note: the EL7062 control word has no "quick stop" bit, so states involving
/// quick stop are not modelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CiA402ChannelState {
    /// No valid status read back yet
    #[default]
    Start,
    SwitchOnDisabled,
    ReadyToSwitchOn,
    SwitchedOn,
    OperationEnabled,
    Fault,
}

impl CiA402ChannelState {
    pub fn from_status_word(status_word: u16) -> Self {
        if status_word & STATUS_FAULT != 0 {
            return Self::Fault;
        }
        if status_word & STATUS_OPERATION_ENABLED != 0 {
            return Self::OperationEnabled;
        }
        if status_word & STATUS_SWITCHED_ON != 0 {
            return Self::SwitchedOn;
        }
        if status_word & STATUS_READY_TO_SWITCH_ON != 0 {
            return Self::ReadyToSwitchOn;
        }
        let _ = STATUS_SWITCH_ON_DISABLED;
        Self::SwitchOnDisabled
    }

    /// Whether the drive is currently enabled (operation enabled plus switched on plus ready)
    pub const fn is_operation_enabled(self) -> bool {
        matches!(self, Self::OperationEnabled)
    }

    pub const fn is_fault(self) -> bool {
        matches!(self, Self::Fault)
    }
}

/// Unwraps the raw `u32` feedback position of a channel into a continuous [`i128`]
///
/// The EL7062 wraps its position at `2^32` without exposing overflow flags, so
/// wrap-around is detected by assuming that a jump of more than half the range
/// between two samples is a wrap.
#[derive(Debug, Clone, Copy)]
pub struct PositionWrapperU32I128 {
    position: i128,
    last_raw: u32,
    initialized: bool,
}

impl Default for PositionWrapperU32I128 {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionWrapperU32I128 {
    pub const fn new() -> Self {
        Self {
            position: 0,
            last_raw: 0,
            initialized: false,
        }
    }

    pub fn update(&mut self, raw: u32) {
        if !self.initialized {
            self.position = raw as i128;
            self.last_raw = raw;
            self.initialized = true;
            return;
        }

        let delta = (raw as i128) - (self.last_raw as i128);
        let absolute_max_step = 1i128 << 31;
        let adjusted = if delta > absolute_max_step {
            delta - (1i128 << 32)
        } else if delta < -absolute_max_step {
            delta + (1i128 << 32)
        } else {
            delta
        };

        self.position += adjusted;
        self.last_raw = raw;
    }

    pub const fn current(&self) -> i128 {
        self.position
    }
}

#[derive(EthercatDevice, Clone, Debug)]
pub struct EL7062 {
    pub txpdo: pdo::EL7062TxPdo,
    pub rxpdo: pdo::EL7062RxPdo,
    is_used: bool,
    pub configuration: EL7062Configuration,
    enable_requests: [bool; 2],
    fault_reset_pending: [bool; 2],
    target_positions: [i128; 2],
    position_wrappers: [PositionWrapperU32I128; 2],
}

impl EthercatDeviceProcessing for EL7062 {
    fn output_pre_process(&mut self) -> Result<(), anyhow::Error> {
        self.process_channel(0)?;
        self.process_channel(1)?;
        Ok(())
    }
}

impl NewEthercatDevice for EL7062 {
    fn new() -> Self {
        let configuration = EL7062Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
            configuration,
            enable_requests: [false; 2],
            fault_reset_pending: [false; 2],
            target_positions: [0; 2],
            position_wrappers: [PositionWrapperU32I128::new(); 2],
        }
    }
}

impl EL7062 {
    pub fn get_port_count(&self) -> usize {
        2
    }

    /// Request a channel to be enabled or disabled.
    ///
    /// The CiA 402 state machine is driven in [`EthercatDeviceProcessing::output_pre_process`]
    /// and advances the control word towards operation enabled / switch on disabled.
    pub fn set_enabled(&mut self, port: usize, enabled: bool) -> Result<(), anyhow::Error> {
        self.check_port(port)?;
        self.enable_requests[port] = enabled;
        if !enabled {
            self.fault_reset_pending[port] = false;
        }
        Ok(())
    }

    /// Whether the channel is requesting operation and the drive reports operation enabled.
    pub fn is_enabled(&self, port: usize) -> Result<bool, anyhow::Error> {
        self.check_port(port)?;
        Ok(self.enable_requests[port]
            && CiA402ChannelState::from_status_word(self.channel_status_word(port)?)
                .is_operation_enabled())
    }

    /// Set the CSP target position of a channel. The value wraps modulo `2^32`.
    pub fn set_target_position(
        &mut self,
        port: usize,
        position: i128,
    ) -> Result<(), anyhow::Error> {
        self.check_port(port)?;
        self.target_positions[port] = position;
        Ok(())
    }

    pub fn get_target_position(&self, port: usize) -> Result<i128, anyhow::Error> {
        self.check_port(port)?;
        Ok(self.target_positions[port])
    }

    /// The unwrapped feedback position of a channel.
    pub fn get_actual_position(&self, port: usize) -> Result<i128, anyhow::Error> {
        self.check_port(port)?;
        Ok(self.position_wrappers[port].current())
    }

    pub fn is_in_fault(&self, port: usize) -> Result<bool, anyhow::Error> {
        self.check_port(port)?;
        Ok(CiA402ChannelState::from_status_word(self.channel_status_word(port)?).is_fault())
    }

    /// Issue a CiA 402 fault reset for a channel (edge triggered on control word bit 7).
    pub fn reset_fault(&mut self, port: usize) -> Result<(), anyhow::Error> {
        self.check_port(port)?;
        self.fault_reset_pending[port] = true;
        Ok(())
    }

    pub fn status_word(&self, port: usize) -> Result<u16, anyhow::Error> {
        self.check_port(port)?;
        self.channel_status_word(port)
    }

    pub fn state(&self, port: usize) -> Result<CiA402ChannelState, anyhow::Error> {
        self.check_port(port)?;
        Ok(CiA402ChannelState::from_status_word(
            self.channel_status_word(port)?,
        ))
    }

    /// Whether the drive reports targeting the commanded position (status word bit 12).
    pub fn drive_follows_command(&self, port: usize) -> Result<bool, anyhow::Error> {
        self.check_port(port)?;
        Ok(self.channel_status_word(port)? & STATUS_DRIVE_FOLLOWS_COMMAND != 0)
    }

    /// Modes of operation display (0x6010:03 Ch.1 / 0x6110:03 Ch.2), if the PDO is mapped.
    pub fn mode_of_operation_display(&self, port: usize) -> Result<Option<u8>, anyhow::Error> {
        self.check_port(port)?;
        self.channel_mode_display(port)
    }

    fn check_port(&self, port: usize) -> Result<(), anyhow::Error> {
        if port < 2 {
            Ok(())
        } else {
            Err(anyhow!("Invalid port, expected 0 or 1, got {port}"))
        }
    }

    fn channel_status_word(&self, port: usize) -> Result<u16, anyhow::Error> {
        match port {
            0 => self
                .txpdo
                .status_word_ch1
                .as_ref()
                .map(|o| o.status_word)
                .ok_or_else(|| anyhow!("status_word_ch1 is None")),
            1 => self
                .txpdo
                .status_word_ch2
                .as_ref()
                .map(|o| o.status_word)
                .ok_or_else(|| anyhow!("status_word_ch2 is None")),
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn channel_fb_position(&self, port: usize) -> Result<u32, anyhow::Error> {
        match port {
            0 => self
                .txpdo
                .fb_position_ch1
                .as_ref()
                .map(|o| o.position)
                .ok_or_else(|| anyhow!("fb_position_ch1 is None")),
            1 => self
                .txpdo
                .fb_position_ch2
                .as_ref()
                .map(|o| o.position)
                .ok_or_else(|| anyhow!("fb_position_ch2 is None")),
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn channel_mode_display(&self, port: usize) -> Result<Option<u8>, anyhow::Error> {
        match port {
            0 => Ok(self
                .txpdo
                .mode_of_operation_display_ch1
                .as_ref()
                .map(|o| o.mode_display)),
            1 => Ok(self
                .txpdo
                .mode_of_operation_display_ch2
                .as_ref()
                .map(|o| o.mode_display)),
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn process_channel(&mut self, port: usize) -> Result<(), anyhow::Error> {
        let status_word = self.channel_status_word(port)?;
        let state = CiA402ChannelState::from_status_word(status_word);
        let actual_position = self.channel_fb_position(port)?;
        let target_position = self.target_positions[port];
        let enable = self.enable_requests[port];

        self.position_wrappers[port].update(actual_position);

        tracing::trace!(
            "EL7062 ch{port}: status=0x{status_word:04X} state={state:?} enable={enable} target={target_position} actual={actual_position}"
        );

        if state.is_fault() {
            // edge-triggered reset; hold the reset while pending, otherwise keep the drive off
            let control_word = if self.fault_reset_pending[port] {
                CONTROL_WORD_FAULT_RESET
            } else {
                CONTROL_WORD_DISABLE_VOLTAGE
            };
            tracing::trace!(
                "EL7062 ch{port}: fault -> control_word=0x{control_word:04X} (reset_pending={})",
                self.fault_reset_pending[port]
            );
            self.rxpdo_write_control_word(port, control_word)?;
        } else {
            // leaving the fault state clears any pending reset
            self.fault_reset_pending[port] = false;

            let control_word = match state {
                CiA402ChannelState::Start | CiA402ChannelState::SwitchOnDisabled => {
                    if enable {
                        CONTROL_WORD_SHUTDOWN
                    } else {
                        CONTROL_WORD_DISABLE_VOLTAGE
                    }
                }
                CiA402ChannelState::ReadyToSwitchOn => {
                    if enable {
                        CONTROL_WORD_SWITCH_ON
                    } else {
                        CONTROL_WORD_DISABLE_VOLTAGE
                    }
                }
                CiA402ChannelState::SwitchedOn => {
                    if enable {
                        CONTROL_WORD_ENABLE_OPERATION
                    } else {
                        CONTROL_WORD_SHUTDOWN
                    }
                }
                CiA402ChannelState::OperationEnabled => {
                    if enable {
                        CONTROL_WORD_ENABLE_OPERATION
                    } else {
                        CONTROL_WORD_SWITCH_ON
                    }
                }
                CiA402ChannelState::Fault => unreachable!(),
            };
            tracing::trace!("EL7062 ch{port}: state={state:?} -> control_word=0x{control_word:04X}");
            self.rxpdo_write_control_word(port, control_word)?;
        }

        // feed the target position and keep the mode of operation in CSP
        self.rxpdo_write_target_position(port, target_position)?;
        self.rxpdo_write_mode(port, CSP_MODE)?;
        Ok(())
    }

    fn rxpdo_write_control_word(
        &mut self,
        port: usize,
        control_word: u16,
    ) -> Result<(), anyhow::Error> {
        match port {
            0 => {
                let field = self
                    .rxpdo
                    .control_word_ch1
                    .as_mut()
                    .ok_or_else(|| anyhow!("control_word_ch1 is None"))?;
                field.control_word = control_word;
                Ok(())
            }
            1 => {
                let field = self
                    .rxpdo
                    .control_word_ch2
                    .as_mut()
                    .ok_or_else(|| anyhow!("control_word_ch2 is None"))?;
                field.control_word = control_word;
                Ok(())
            }
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn rxpdo_write_target_position(
        &mut self,
        port: usize,
        position: i128,
    ) -> Result<(), anyhow::Error> {
        match port {
            0 => {
                let field = self
                    .rxpdo
                    .target_position_ch1
                    .as_mut()
                    .ok_or_else(|| anyhow!("target_position_ch1 is None"))?;
                field.target_position = position as u32;
                Ok(())
            }
            1 => {
                let field = self
                    .rxpdo
                    .target_position_ch2
                    .as_mut()
                    .ok_or_else(|| anyhow!("target_position_ch2 is None"))?;
                field.target_position = position as u32;
                Ok(())
            }
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn rxpdo_write_mode(&mut self, port: usize, mode: u8) -> Result<(), anyhow::Error> {
        match port {
            0 => {
                if let Some(field) = self.rxpdo.modes_of_operation_ch1.as_mut() {
                    field.mode = mode;
                }
                Ok(())
            }
            1 => {
                if let Some(field) = self.rxpdo.modes_of_operation_ch2.as_mut() {
                    field.mode = mode;
                }
                Ok(())
            }
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn rxpdo_control_word(&self, port: usize) -> Result<u16, anyhow::Error> {
        match port {
            0 => self
                .rxpdo
                .control_word_ch1
                .as_ref()
                .map(|o| o.control_word)
                .ok_or_else(|| anyhow!("control_word_ch1 is None")),
            1 => self
                .rxpdo
                .control_word_ch2
                .as_ref()
                .map(|o| o.control_word)
                .ok_or_else(|| anyhow!("control_word_ch2 is None")),
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }

    fn rxpdo_mode(&self, port: usize) -> Result<Option<u8>, anyhow::Error> {
        match port {
            0 => Ok(self.rxpdo.modes_of_operation_ch1.as_ref().map(|o| o.mode)),
            1 => Ok(self.rxpdo.modes_of_operation_ch2.as_ref().map(|o| o.mode)),
            _ => Err(anyhow!("Invalid port: {port}")),
        }
    }
}

impl ServoPositionEL7062Device for EL7062 {
    fn get_input(&self, port: usize) -> Result<ServoPositionEL7062Input, anyhow::Error> {
        self.check_port(port)?;
        let status_word = self.channel_status_word(port)?;
        let state = CiA402ChannelState::from_status_word(status_word);

        Ok(ServoPositionEL7062Input {
            position: self.position_wrappers[port].current(),
            is_enabled: self.enable_requests[port] && state.is_operation_enabled(),
            is_in_fault: state.is_fault(),
            drive_follows_command: status_word & STATUS_DRIVE_FOLLOWS_COMMAND != 0,
            status_word,
            state,
            mode_display: self.channel_mode_display(port)?,
        })
    }

    fn get_output(&self, port: usize) -> Result<ServoPositionEL7062Output, anyhow::Error> {
        self.check_port(port)?;
        let control_word = self.rxpdo_control_word(port)?;
        let mode = self.rxpdo_mode(port)?.unwrap_or(CSP_MODE);

        Ok(ServoPositionEL7062Output {
            control_word,
            mode,
            target_position: self.target_positions[port],
        })
    }

    fn set_target_position(&mut self, port: usize, position: i128) -> Result<(), anyhow::Error> {
        EL7062::set_target_position(self, port, position)
    }

    fn get_target_position(&self, port: usize) -> Result<i128, anyhow::Error> {
        EL7062::get_target_position(self, port)
    }

    fn set_enabled(&mut self, port: usize, enabled: bool) -> Result<(), anyhow::Error> {
        EL7062::set_enabled(self, port, enabled)
    }

    fn reset_fault(&mut self, port: usize) -> Result<(), anyhow::Error> {
        EL7062::reset_fault(self, port)
    }

    fn get_port_count(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_word_decoding() {
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0000),
            CiA402ChannelState::SwitchOnDisabled
        );
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0040),
            CiA402ChannelState::SwitchOnDisabled
        );
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0021),
            CiA402ChannelState::ReadyToSwitchOn
        );
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0023),
            CiA402ChannelState::SwitchedOn
        );
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0027),
            CiA402ChannelState::OperationEnabled
        );
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0008),
            CiA402ChannelState::Fault
        );
        assert_eq!(
            CiA402ChannelState::from_status_word(0x0028),
            CiA402ChannelState::Fault
        );
    }

    #[test]
    fn enable_sequence_advances_states() {
        let mut device = EL7062::new();
        device.set_enabled(0, true).unwrap();

        // switch on disabled -> shutdown
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0040,
        });
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_SHUTDOWN
        );

        // ready to switch on -> switch on
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0021,
        });
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_SWITCH_ON
        );

        // switched on -> enable operation
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0023,
        });
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_ENABLE_OPERATION
        );

        // operation enabled -> hold and report is_enabled
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0027,
        });
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_ENABLE_OPERATION
        );
        assert!(device.is_enabled(0).unwrap());
        assert!(!device.is_in_fault(0).unwrap());
    }

    #[test]
    fn disable_sequence_retreats_states() {
        let mut device = EL7062::new();
        device.set_enabled(0, true).unwrap();
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0027,
        });
        device.process_channel(0).unwrap();
        assert!(device.is_enabled(0).unwrap());

        device.set_enabled(0, false).unwrap();
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_SWITCH_ON
        );

        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0023,
        });
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_SHUTDOWN
        );

        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0021,
        });
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_DISABLE_VOLTAGE
        );

        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0040,
        });
        device.process_channel(0).unwrap();
        assert!(!device.is_enabled(0).unwrap());
    }

    #[test]
    fn fault_reset_is_edge_driven() {
        let mut device = EL7062::new();
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0008,
        });

        // no pending reset -> keep the drive off
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_DISABLE_VOLTAGE
        );
        assert!(device.is_in_fault(0).unwrap());

        // pending reset -> fault reset held
        device.reset_fault(0).unwrap();
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.control_word_ch1.unwrap().control_word,
            CONTROL_WORD_FAULT_RESET
        );

        // fault cleared -> pending flag dropped
        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0040,
        });
        device.process_channel(0).unwrap();
        assert!(!device.fault_reset_pending[0]);
    }

    #[test]
    fn target_position_writes_and_wraps() {
        let mut device = EL7062::new();
        device.set_target_position(0, -1).unwrap();
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.target_position_ch1.unwrap().target_position,
            0xFFFF_FFFF
        );

        device
            .set_target_position(0, (1i128 << 32) + 12345)
            .unwrap();
        device.process_channel(0).unwrap();
        assert_eq!(
            device.rxpdo.target_position_ch1.unwrap().target_position,
            12345
        );
    }

    #[test]
    fn position_wrapper_unwraps() {
        // plain increments
        let mut wrapper = PositionWrapperU32I128::new();
        wrapper.update(0);
        wrapper.update(1000);
        assert_eq!(wrapper.current(), 1000);

        // small backwards step, no wrap
        wrapper.update(900);
        assert_eq!(wrapper.current(), 900);

        // backward wrap: raw 900 -> 0xFFFF_FFF0 crosses 0 downwards
        wrapper.update(0xFFFF_FFF0);
        assert_eq!(wrapper.current(), -16);
    }

    #[test]
    fn position_wrapper_detects_wrap() {
        // forward wrap: last raw 0xFFFF_FFF0, next small raw -> a small positive step
        let mut wrapper = PositionWrapperU32I128::new();
        wrapper.update(0xFFFF_FFF0);
        wrapper.update(5);
        assert_eq!(wrapper.current(), 0xFFFF_FFF0i128 + 21);

        // backward wrap: last raw small, next raw 0xFFFF_FFF0 -> a small negative step
        let mut wrapper = PositionWrapperU32I128::new();
        wrapper.update(5);
        wrapper.update(0xFFFF_FFF0);
        assert_eq!(wrapper.current(), -16);

        // no wrap: large but within half-range step
        let mut wrapper = PositionWrapperU32I128::new();
        wrapper.update(1_000_000_000);
        wrapper.update(1_100_000_000);
        assert_eq!(wrapper.current(), 1_100_000_000);
    }

    #[test]
    fn invalid_port_is_rejected() {
        let device = EL7062::new();
        assert!(device.is_enabled(2).is_err());
        assert!(device.get_actual_position(2).is_err());
    }

    #[test]
    fn io_snapshot_reflects_state() {
        use crate::io::servo_position_el7062::{
            ServoPositionEL7062Device as _, ServoPositionEL7062Input,
        };

        let mut device = EL7062::new();
        device.set_enabled(0, true).unwrap();
        device.set_target_position(1, 4242).unwrap();

        device.txpdo.status_word_ch1 = Some(pdo::DrvStatusWord {
            status_word: 0x0027,
        });
        device.txpdo.status_word_ch2 = Some(pdo::DrvStatusWord {
            status_word: 0x0040,
        });
        device.txpdo.mode_of_operation_display_ch1 = Some(pdo::DrvModeOfOperationDisplay {
            mode_display: CSP_MODE,
        });
        device.process_channel(0).unwrap();
        device.process_channel(1).unwrap();

        let input1: ServoPositionEL7062Input = device.get_input(0).unwrap();
        assert!(input1.is_enabled);
        assert_eq!(input1.state, CiA402ChannelState::OperationEnabled);
        assert_eq!(input1.mode_display, Some(CSP_MODE));

        let input2 = device.get_input(1).unwrap();
        assert!(!input2.is_enabled);
        assert_eq!(input2.state, CiA402ChannelState::SwitchOnDisabled);

        let output = device.get_output(1).unwrap();
        assert_eq!(output.target_position, 4242);
        assert_eq!(output.mode, CSP_MODE);

        assert_eq!(device.get_port_count(), 2);
    }
}
