use crate::devices::{
    EthercatDevice,
    panasonic_modules::minas_a6::{PositionSpec, PositionTransitionSpec, State},
};
use anyhow::Error;

/// Snapshot of the motor's current status (derived from TxPDO + internal state machine).
#[derive(Debug, Clone)]
pub struct MinasA6BInput {
    /// Current motor position in revolutions. `None` if encoder resolution has not been read yet.
    pub position: Option<f64>,

    /// Whether the motor's state machine is in a `Run*` state.
    pub is_enabled: bool,

    /// Whether homing has been completed successfully.
    pub is_homed: bool,

    /// Whether the motor has finished all queued motion commands (`RunIdle`).
    pub is_motion_done: bool,

    /// Whether the motor is fully shut down (`PreOpSwitchOnDisabled`).
    pub is_shut_down: bool,

    /// Whether the state machine is in any error state.
    pub has_error: bool,

    /// Raw CiA 402 status word from the drive.
    pub status_word: u16,

    /// Raw error code from the drive (e.g. `0xFF28` = encoder multi-turn error).
    pub error_code: u16,

    /// Current internal state machine state.
    pub state: State,

    /// Mode currently displayed by the drive (PP = 1, Homing = 6).
    pub mode_display: u8,
}

/// Snapshot of the last RxPDO values being sent to the drive.
#[derive(Debug, Clone)]
pub struct MinasA6BOutput {
    /// CiA 402 control word currently being written to the drive.
    pub control_word: u16,

    /// Operation mode currently requested (PP = 1, Homing = 6).
    pub mode: u8,

    /// Target position in encoder increments.
    pub target_position: i32,

    /// Target velocity in encoder increments/s.
    pub target_velocity: u32,

    /// Target acceleration in encoder increments/s².
    pub target_accel: u32,

    /// Target deceleration in encoder increments/s².
    pub target_decel: u32,
}

pub trait MinasA6BDevice: EthercatDevice {
    fn get_input(&self) -> Result<MinasA6BInput, Error>;
    fn get_output(&self) -> Result<MinasA6BOutput, Error>;

    fn get_position(&self) -> Option<f64> {
        self.get_input().ok().and_then(|i| i.position)
    }
    fn is_enabled(&self) -> bool {
        self.get_input().map(|i| i.is_enabled).unwrap_or(false)
    }
    fn is_homed(&self) -> bool {
        self.get_input().map(|i| i.is_homed).unwrap_or(false)
    }
    fn is_motion_done(&self) -> bool {
        self.get_input().map(|i| i.is_motion_done).unwrap_or(false)
    }
    fn is_shut_down(&self) -> bool {
        self.get_input().map(|i| i.is_shut_down).unwrap_or(false)
    }
    fn has_error(&self) -> bool {
        self.get_input().map(|i| i.has_error).unwrap_or(false)
    }

    fn enable(&mut self) -> Result<(), Error>;
    fn disable(&mut self) -> Result<(), Error>;
    fn shut_down(&mut self) -> Result<(), Error> {
        self.disable()
    }

    fn quick_stop(&mut self) -> Result<(), Error>;
    fn leave_quick_stop(&mut self) -> Result<(), Error>;

    fn move_position(
        &mut self,
        position: PositionSpec,
        transition: Option<PositionTransitionSpec>,
    ) -> Result<(), Error>;

    fn move_positions(
        &mut self,
        positions: Vec<PositionSpec>,
        transition: Option<PositionTransitionSpec>,
    ) -> Result<(), Error>;

    fn abort_motion(&mut self) -> Result<(), Error>;
    fn home(&mut self) -> Result<(), Error>;
}
