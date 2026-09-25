use crate::pdo::{PredefinedPdoAssignment, RxPdoObject, TxPdoObject};
use bitvec::prelude::*;
use ethercat_hal_derive::{PdoObject, RxPdo, TxPdo};

/// # `DrvControlWord`
/// CiA402 control word, 16 bits.
///
/// Mapped from `0x7010:01` (Ch. 1) / `0x7110:01` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct DrvControlWord {
    /// Bit 0: Switch on
    pub switch_on: bool,
    /// Bit 1: Enable voltage
    pub enable_voltage: bool,
    /// Bit 2: Quick stop
    pub quick_stop: bool,
    /// Bit 3: Enable operation
    pub enable_operation: bool,
    /// Bit 7: Fault reset
    pub fault_reset: bool,
}

impl RxPdoObject for DrvControlWord {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits.set(0, self.switch_on);
        bits.set(1, self.enable_voltage);
        bits.set(2, self.quick_stop);
        bits.set(3, self.enable_operation);
        bits.set(7, self.fault_reset);
    }
}

impl DrvControlWord {
    /// CiA402 402 state machine commands expressed as a plain word.
    /// This allows the drive to be operated without the helper flags.
    pub fn from_raw(raw: u16) -> Self {
        Self {
            switch_on: raw & (1 << 0) != 0,
            enable_voltage: raw & (1 << 1) != 0,
            quick_stop: raw & (1 << 2) != 0,
            enable_operation: raw & (1 << 3) != 0,
            fault_reset: raw & (1 << 7) != 0,
        }
    }
}

/// # `DrvTargetPosition`
/// CiA402 target position, 32 bits (UDINT).
///
/// Mapped from `0x7010:05` (Ch. 1) / `0x7110:05` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct DrvTargetPosition {
    /// Target position in increments.
    pub target_position: i32,
}

impl RxPdoObject for DrvTargetPosition {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits[0..32].store_le(self.target_position as u32);
    }
}

/// # `DrvTargetVelocity`
/// CiA402 target velocity, 32 bits (DINT).
///
/// Mapped from `0x7010:06` (Ch. 1) / `0x7110:06` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct DrvTargetVelocity {
    /// Target velocity in increments per second.
    pub target_velocity: i32,
}

impl RxPdoObject for DrvTargetVelocity {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits[0..32].store_le(self.target_velocity as u32);
    }
}

/// # `DrvTargetTorque`
/// CiA402 target torque, 16 bits (INT).
///
/// Mapped from `0x7010:09` (Ch. 1) / `0x7110:09` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct DrvTargetTorque {
    /// Target torque in thousandths of the nominal current.
    pub target_torque: i16,
}

impl RxPdoObject for DrvTargetTorque {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits[0..16].store_le(self.target_torque as u16);
    }
}

/// # `DrvCommutationAngle`
/// Commutation angle for CSTCA mode, 16 bits (UINT).
///
/// Mapped from `0x7010:0E` (Ch. 1) / `0x7110:0E` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct DrvCommutationAngle {
    /// Commutation angle in electrical degrees (0 - 359).
    pub commutation_angle: u16,
}

impl RxPdoObject for DrvCommutationAngle {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits[0..16].store_le(self.commutation_angle);
    }
}

/// # `FbPosition`
/// Feedback position, 32 bits (UDINT).
///
/// Mapped from `0x6000:11` (Ch. 1) / `0x6100:11` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct FbPosition {
    /// Actual position in increments.
    pub position: i32,
}

impl TxPdoObject for FbPosition {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.position = bits[0..32].load_le::<u32>() as i32;
    }
}

/// # `DrvStatusWord`
/// CiA402 status word, 16 bits.
///
/// Mapped from `0x6010:01` (Ch. 1) / `0x6110:01` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct DrvStatusWord {
    /// Bit 0: Ready to switch on
    pub ready_to_switch_on: bool,
    /// Bit 1: Switched on
    pub switched_on: bool,
    /// Bit 2: Operation enabled
    pub operation_enabled: bool,
    /// Bit 3: Fault
    pub fault: bool,
    /// Bit 6: Switch on disabled
    pub switch_on_disabled: bool,
    /// Bit 7: Warning
    pub warning: bool,
    /// Bit 10: TxPDOToggle (enabled via 0x8010:01) *or* the low bit of the input
    /// cycle counter (enabled via 0x8010:02). Both claim this bit, so enabling
    /// the two features at once makes the readings ambiguous.
    pub txpdo_toggle: bool,
    /// Bit 11: Internal limit active
    pub internal_limit_active: bool,
    /// Bit 12: Drive follows the command value
    pub drive_follows_command_value: bool,
    /// Bit 13: Input cycle counter (as documented for 0x6010:01)
    pub input_cycle_counter: bool,
    /// Bit 14: High bit of the two-bit input cycle counter that 0x8010:02 places
    /// in bit 10 (low) and bit 14 (high). Only valid when 0x8010:02 is enabled.
    pub input_cycle_counter_high: bool,
}

impl TxPdoObject for DrvStatusWord {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.ready_to_switch_on = bits[0];
        self.switched_on = bits[1];
        self.operation_enabled = bits[2];
        self.fault = bits[3];
        self.switch_on_disabled = bits[6];
        self.warning = bits[7];
        self.txpdo_toggle = bits[10];
        self.internal_limit_active = bits[11];
        self.drive_follows_command_value = bits[12];
        self.input_cycle_counter = bits[13];
        self.input_cycle_counter_high = bits[14];
    }
}

impl DrvStatusWord {
    /// Retrieve the raw status word value.
    pub fn as_raw(&self) -> u16 {
        let mut raw = 0u16;
        if self.ready_to_switch_on {
            raw |= 1 << 0;
        }
        if self.switched_on {
            raw |= 1 << 1;
        }
        if self.operation_enabled {
            raw |= 1 << 2;
        }
        if self.fault {
            raw |= 1 << 3;
        }
        if self.switch_on_disabled {
            raw |= 1 << 6;
        }
        if self.warning {
            raw |= 1 << 7;
        }
        if self.txpdo_toggle {
            raw |= 1 << 10;
        }
        if self.internal_limit_active {
            raw |= 1 << 11;
        }
        if self.drive_follows_command_value {
            raw |= 1 << 12;
        }
        if self.input_cycle_counter {
            raw |= 1 << 13;
        }
        if self.input_cycle_counter_high {
            raw |= 1 << 14;
        }
        raw
    }
}

/// # `DrvFollowingError`
/// Following error actual value, 32 bits (DINT).
///
/// Mapped from `0x6010:06` (Ch. 1) / `0x6110:06` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct DrvFollowingError {
    /// Actual following error in increments.
    pub following_error: i32,
}

impl TxPdoObject for DrvFollowingError {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.following_error = bits[0..32].load_le::<u32>() as i32;
    }
}

/// # `DrvVelocityActual`
/// Velocity actual value, 32 bits (DINT).
///
/// Mapped from `0x6010:07` (Ch. 1) / `0x6110:07` (Ch. 2).
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct DrvVelocityActual {
    /// Actual velocity in increments per second.
    pub velocity_actual: i32,
}

impl TxPdoObject for DrvVelocityActual {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.velocity_actual = bits[0..32].load_le::<u32>() as i32;
    }
}

/// # TxPDO of the EL7062.
///
/// Each field corresponds to one TxPDO mapping object. The order and the
/// `Option` state must match the SyncManager 3 assignment (`0x1C13`) for the
/// selected device mode.
#[derive(Debug, Clone, TxPdo)]
pub struct EL7062TxPdo {
    #[pdo_object_index(0x1A00)]
    pub ch1_position: Option<FbPosition>,
    #[pdo_object_index(0x1A01)]
    pub ch1_statusword: Option<DrvStatusWord>,
    #[pdo_object_index(0x1A06)]
    pub ch1_following_error: Option<DrvFollowingError>,
    #[pdo_object_index(0x1A02)]
    pub ch1_velocity_actual: Option<DrvVelocityActual>,

    #[pdo_object_index(0x1A80)]
    pub ch2_position: Option<FbPosition>,
    #[pdo_object_index(0x1A81)]
    pub ch2_statusword: Option<DrvStatusWord>,
    #[pdo_object_index(0x1A86)]
    pub ch2_following_error: Option<DrvFollowingError>,
    #[pdo_object_index(0x1A82)]
    pub ch2_velocity_actual: Option<DrvVelocityActual>,
}

/// # RxPDO of the EL7062.
///
/// Each field corresponds to one RxPDO mapping object. The order and the
/// `Option` state must match the SyncManager 2 assignment (`0x1C12`) for the
/// selected device mode.
#[derive(Debug, Clone, RxPdo)]
pub struct EL7062RxPdo {
    #[pdo_object_index(0x1600)]
    pub ch1_controlword: Option<DrvControlWord>,
    #[pdo_object_index(0x1606)]
    pub ch1_position: Option<DrvTargetPosition>,
    #[pdo_object_index(0x1601)]
    pub ch1_target_velocity: Option<DrvTargetVelocity>,
    #[pdo_object_index(0x1602)]
    pub ch1_target_torque: Option<DrvTargetTorque>,
    #[pdo_object_index(0x1603)]
    pub ch1_commutation_angle: Option<DrvCommutationAngle>,

    #[pdo_object_index(0x1680)]
    pub ch2_controlword: Option<DrvControlWord>,
    #[pdo_object_index(0x1686)]
    pub ch2_position: Option<DrvTargetPosition>,
    #[pdo_object_index(0x1681)]
    pub ch2_target_velocity: Option<DrvTargetVelocity>,
    #[pdo_object_index(0x1682)]
    pub ch2_target_torque: Option<DrvTargetTorque>,
    #[pdo_object_index(0x1683)]
    pub ch2_commutation_angle: Option<DrvCommutationAngle>,
}

/// Predefined PDO assignments for the EL7062.
///
/// The device default (and the default of this enum) is CSP (Cyclic synchronous
/// position mode).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EL7062PredefinedPdoAssignment {
    /// Cyclic synchronous position mode (CSP), mode of operation `8`.
    #[default]
    CyclicSynchronousPosition,
    /// Cyclic synchronous velocity mode (CSV), mode of operation `9`.
    CyclicSynchronousVelocity,
    /// Cyclic synchronous torque mode (CST), mode of operation `10`.
    CyclicSynchronousTorque,
    /// Cyclic synchronous torque mode with commutation angle (CSTCA), mode of operation `11`.
    CyclicSynchronousTorqueWithCommutationAngle,
}

impl EL7062PredefinedPdoAssignment {
    /// The value of `0x7010:03` / `0x7110:03` (Modes of operation) that belongs
    /// to this PDO assignment.
    pub fn mode_of_operation(&self) -> u8 {
        match self {
            Self::CyclicSynchronousPosition => 8,
            Self::CyclicSynchronousVelocity => 9,
            Self::CyclicSynchronousTorque => 10,
            Self::CyclicSynchronousTorqueWithCommutationAngle => 11,
        }
    }
}

impl PredefinedPdoAssignment<EL7062TxPdo, EL7062RxPdo> for EL7062PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL7062TxPdo {
        match self {
            Self::CyclicSynchronousPosition => EL7062TxPdo {
                ch1_position: Some(FbPosition::default()),
                ch1_statusword: Some(DrvStatusWord::default()),
                ch1_following_error: Some(DrvFollowingError::default()),
                ch1_velocity_actual: None,
                ch2_position: Some(FbPosition::default()),
                ch2_statusword: Some(DrvStatusWord::default()),
                ch2_following_error: Some(DrvFollowingError::default()),
                ch2_velocity_actual: None,
            },
            Self::CyclicSynchronousVelocity
            | Self::CyclicSynchronousTorque
            | Self::CyclicSynchronousTorqueWithCommutationAngle => EL7062TxPdo {
                ch1_position: Some(FbPosition::default()),
                ch1_statusword: Some(DrvStatusWord::default()),
                ch1_following_error: None,
                ch1_velocity_actual: None,
                ch2_position: Some(FbPosition::default()),
                ch2_statusword: Some(DrvStatusWord::default()),
                ch2_following_error: None,
                ch2_velocity_actual: None,
            },
        }
    }

    fn rxpdo_assignment(&self) -> EL7062RxPdo {
        match self {
            Self::CyclicSynchronousPosition => EL7062RxPdo {
                ch1_controlword: Some(DrvControlWord::default()),
                ch1_position: Some(DrvTargetPosition::default()),
                ch1_target_velocity: None,
                ch1_target_torque: None,
                ch1_commutation_angle: None,
                ch2_controlword: Some(DrvControlWord::default()),
                ch2_position: Some(DrvTargetPosition::default()),
                ch2_target_velocity: None,
                ch2_target_torque: None,
                ch2_commutation_angle: None,
            },
            Self::CyclicSynchronousVelocity => EL7062RxPdo {
                ch1_controlword: Some(DrvControlWord::default()),
                ch1_position: None,
                ch1_target_velocity: Some(DrvTargetVelocity::default()),
                ch1_target_torque: None,
                ch1_commutation_angle: None,
                ch2_controlword: Some(DrvControlWord::default()),
                ch2_position: None,
                ch2_target_velocity: Some(DrvTargetVelocity::default()),
                ch2_target_torque: None,
                ch2_commutation_angle: None,
            },
            Self::CyclicSynchronousTorque => EL7062RxPdo {
                ch1_controlword: Some(DrvControlWord::default()),
                ch1_position: None,
                ch1_target_velocity: None,
                ch1_target_torque: Some(DrvTargetTorque::default()),
                ch1_commutation_angle: None,
                ch2_controlword: Some(DrvControlWord::default()),
                ch2_position: None,
                ch2_target_velocity: None,
                ch2_target_torque: Some(DrvTargetTorque::default()),
                ch2_commutation_angle: None,
            },
            Self::CyclicSynchronousTorqueWithCommutationAngle => EL7062RxPdo {
                ch1_controlword: Some(DrvControlWord::default()),
                ch1_position: None,
                ch1_target_velocity: None,
                ch1_target_torque: Some(DrvTargetTorque::default()),
                ch1_commutation_angle: Some(DrvCommutationAngle::default()),
                ch2_controlword: Some(DrvControlWord::default()),
                ch2_position: None,
                ch2_target_velocity: None,
                ch2_target_torque: Some(DrvTargetTorque::default()),
                ch2_commutation_angle: Some(DrvCommutationAngle::default()),
            },
        }
    }
}
