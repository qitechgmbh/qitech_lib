use crate::devices::{EthercatDevice, beckhoff_modules::el7062::CiA402ChannelState};
use anyhow::Error;

/// Snapshot of one EL7062 channel's current status (derived from the TxPDO status word).
#[derive(Debug, Clone)]
pub struct ServoPositionEL7062Input {
    /// Unwrapped feedback position in increments. Wraps modulo `2^32` at the drive.
    pub position: i128,

    /// Whether the channel is requesting operation and the drive reports operation enabled.
    pub is_enabled: bool,

    /// Whether the channel is in a CiA 402 fault state.
    pub is_in_fault: bool,

    /// Whether the drive reports following the commanded position (status word bit 12).
    pub drive_follows_command: bool,

    /// Raw CiA 402 status word from the channel.
    pub status_word: u16,

    /// Current CiA 402 channel state.
    pub state: CiA402ChannelState,

    /// Mode displayed by the drive (CSP = 8), if the PDO is mapped.
    pub mode_display: Option<u8>,
}

/// Snapshot of the values last written to one EL7062 channel.
#[derive(Debug, Clone)]
pub struct ServoPositionEL7062Output {
    /// CiA 402 control word currently being written to the channel.
    pub control_word: u16,

    /// Mode of operation requested (CSP = 8).
    pub mode: u8,

    /// Target position in increments, wrapped modulo `2^32` at the drive.
    pub target_position: i128,
}

pub trait ServoPositionEL7062Device: EthercatDevice {
    fn get_input(&self, port: usize) -> Result<ServoPositionEL7062Input, Error>;
    fn get_output(&self, port: usize) -> Result<ServoPositionEL7062Output, Error>;

    fn set_target_position(&mut self, port: usize, position: i128) -> Result<(), Error>;
    fn get_target_position(&self, port: usize) -> Result<i128, Error>;

    fn set_enabled(&mut self, port: usize, enabled: bool) -> Result<(), Error>;
    fn is_enabled(&self, port: usize) -> bool {
        self.get_input(port).map(|i| i.is_enabled).unwrap_or(false)
    }

    fn is_in_fault(&self, port: usize) -> bool {
        self.get_input(port).map(|i| i.is_in_fault).unwrap_or(false)
    }

    fn drive_follows_command(&self, port: usize) -> bool {
        self.get_input(port)
            .map(|i| i.drive_follows_command)
            .unwrap_or(false)
    }

    fn get_position(&self, port: usize) -> i128 {
        self.get_input(port).map(|i| i.position).unwrap_or(0)
    }

    /// Issue a CiA 402 fault reset for a channel.
    fn reset_fault(&mut self, port: usize) -> Result<(), Error>;

    fn get_port_count(&self) -> usize;
}
