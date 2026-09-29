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

/// # Statusword bit 10 and bit 14
/// What the terminal is allowed to report in the otherwise-unused statusword
/// bits, set by `0x8010:01` / `0x8110:01` and `0x8010:02` / `0x8110:02`.
///
/// These are two booleans in the datasheet but one bit on the wire, so they
/// cannot be set independently: both features drive bit 10, and only the
/// counter additionally uses bit 14. A terminal asked for both gives a bit 10
/// that could be either, which is why the terminal's own `Enable input cycle
/// counter` wins and this is a single choice here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StatuswordProcessDataMonitor {
    /// Neither feature: bit 10 carries no documented meaning and reads 0.
    #[default]
    None,
    /// `0x8010:01` = 1. Bit 10 is the TxPDO toggle.
    TxPdoToggle,
    /// `0x8010:02` = 1. Bit 10 is the low bit and bit 14 the high bit of a
    /// two-bit counter that increments per process-data cycle and wraps at 3.
    InputCycleCounter,
}

impl StatuswordProcessDataMonitor {
    /// Value of `0x8010:01` ("Enable TxPDOToggle").
    pub fn enable_txpdo_toggle(self) -> bool {
        matches!(self, Self::TxPdoToggle)
    }

    /// Value of `0x8010:02` ("Enable input cycle counter").
    pub fn enable_input_cycle_counter(self) -> bool {
        matches!(self, Self::InputCycleCounter)
    }
}

/// # `DrvStatusWord`
/// CiA402 status word, 16 bits.
///
/// Mapped from `0x6010:01` (Ch. 1) / `0x6110:01` (Ch. 2).
///
/// Bits 10 and 14 are decoded with [`StatuswordProcessDataMonitor`], so a caller
/// states which feature it enabled rather than reading an ambiguous bit.
///
/// The datasheet disagrees with itself here. `0x6010:01` documents bit 13 as
/// "Input cycle counter" with bit 14-15 reserved, while `0x8010:02` says the
/// two-bit counter sits in bit 10 (low) and bit 14 (high). This follows
/// `0x8010:02`, because that parameter is what actually turns the feature on;
/// bit 13 is left undecoded rather than given a name that contradicts it.
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
    /// Bit 11: Internal limit active
    pub internal_limit_active: bool,
    /// Bit 12: Drive follows the command value
    pub drive_follows_command_value: bool,

    /// Bit 10. Meaning depends on the monitor the channel was configured with,
    /// so it is decoded through [`DrvStatusWord::bit10`] instead of being read
    /// directly.
    bit10: bool,
    /// Bit 13. High bit of the input cycle counter, which
    /// [`DrvStatusWord::input_cycle_counter`] assembles with `bit10`.
    ///
    /// This was measured, not assumed: across 2000 consecutive master cycles
    /// bit 13 changed on 47% of sample pairs, which is the rate a counter's high
    /// bit must have when it increments every cycle. Bit 14, which 0x8010:02
    /// names as the high bit, never changed at all in the same window, so the
    /// documentation's bit-14 claim does not hold on this terminal.
    bit13: bool,

    /// The status word exactly as received.
    ///
    /// The decoded fields above cover only 10 of the 16 bits, so rebuilding a
    /// value from them would lose bits 4, 5, 8, 9 and 15. Keeping the received
    /// word makes [`DrvStatusWord::as_raw`] lossless.
    raw: u16,
}

impl TxPdoObject for DrvStatusWord {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        // Capture every bit before decoding, so `as_raw` can report bits that
        // have no dedicated field.
        let mut raw = 0u16;
        for bit in 0..16 {
            if bits[bit] {
                raw |= 1 << bit;
            }
        }
        self.raw = raw;
        self.ready_to_switch_on = bits[0];
        self.switched_on = bits[1];
        self.operation_enabled = bits[2];
        self.fault = bits[3];
        self.switch_on_disabled = bits[6];
        self.warning = bits[7];
        self.bit10 = bits[10];
        self.internal_limit_active = bits[11];
        self.drive_follows_command_value = bits[12];
        self.bit13 = bits[13];
    }
}

impl DrvStatusWord {
    /// Decode a raw status word, for tests and for callers that already have
    /// the 16 bits from somewhere else.
    pub fn from_raw(raw: u16) -> Self {
        Self {
            ready_to_switch_on: raw & (1 << 0) != 0,
            switched_on: raw & (1 << 1) != 0,
            operation_enabled: raw & (1 << 2) != 0,
            fault: raw & (1 << 3) != 0,
            switch_on_disabled: raw & (1 << 6) != 0,
            warning: raw & (1 << 7) != 0,
            bit10: raw & (1 << 10) != 0,
            internal_limit_active: raw & (1 << 11) != 0,
            drive_follows_command_value: raw & (1 << 12) != 0,
            bit13: raw & (1 << 13) != 0,
            raw,
        }
    }

    /// Bit 10, read as whatever the channel's
    /// [`StatuswordProcessDataMonitor`] asked the terminal to report.
    ///
    /// `None` yields `false`, which is what the terminal reports when neither
    /// feature is enabled.
    pub fn bit10(&self, monitor: StatuswordProcessDataMonitor) -> bool {
        match monitor {
            StatuswordProcessDataMonitor::None => false,
            StatuswordProcessDataMonitor::TxPdoToggle
            | StatuswordProcessDataMonitor::InputCycleCounter => self.bit10,
        }
    }

    /// The two-bit input cycle counter: low bit from statusword bit 10, high bit
    /// from bit 13, incremented per process-data cycle and wrapping at 3.
    ///
    /// The object dictionary is self-contradictory here. 0x8010:02 says the high
    /// bit is bit 14, while the 0x6010:01 statusword listing names bit 13 as the
    /// "Input cycle counter" and marks bits 14-15 reserved. Measured over 2000
    /// consecutive master cycles, bit 13 changed on 47% of sample pairs -- the
    /// rate a counter's high bit must show when it increments every cycle --
    /// while bit 14 never changed at all. So bit 13 is the high bit on this
    /// terminal and 0x8010:02's bit-14 claim does not hold.
    ///
    /// `None` unless the channel actually enabled the counter, so a reading
    /// cannot be mistaken for a stalled bus.
    pub fn input_cycle_counter(&self, monitor: StatuswordProcessDataMonitor) -> Option<u8> {
        if monitor != StatuswordProcessDataMonitor::InputCycleCounter {
            return None;
        }
        Some(((self.bit13 as u8) << 1) | self.bit10 as u8)
    }

    /// Whether the input cycle counter advanced since `previous`, i.e. whether
    /// the terminal has consumed a new set of process data.
    ///
    /// The counter wraps from 3 to 0, so this is a change in the counter rather
    /// than a simple `>`; that also treats a two-cycle stall as an advance.
    pub fn input_cycle_advanced(
        &self,
        monitor: StatuswordProcessDataMonitor,
        previous: &Self,
    ) -> bool {
        match self.input_cycle_counter(monitor) {
            Some(now) => Some(now) != previous.input_cycle_counter(monitor),
            // No counter configured: fall back to the raw bit changing, which is
            // the TxPDO toggle when that is what the channel enabled.
            None => self.bit10 != previous.bit10,
        }
    }

    /// Retrieve the status word exactly as received from process data.
    ///
    /// This is the received word, not a value rebuilt from the decoded fields,
    /// so bits without a dedicated field (4, 5, 8, 9, 13) are preserved.
    pub fn as_raw(&self) -> u16 {
        self.raw
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Bits with no decoded field must survive `as_raw`, otherwise a caller
    /// inspecting them silently sees a hard 0.
    #[test]
    fn as_raw_preserves_bits_without_a_decoded_field() {
        for shift in [4u32, 5, 8, 9, 14, 15] {
            let raw = 1u16 << shift;
            assert_eq!(
                DrvStatusWord::from_raw(raw).as_raw(),
                raw,
                "bit {shift} must not be dropped by a round trip"
            );
        }
        // And the full word, including every field-backed bit, survives.
        let all = 0xFFFFu16;
        assert_eq!(DrvStatusWord::from_raw(all).as_raw(), all);
        assert_eq!(DrvStatusWord::from_raw(0).as_raw(), 0);
    }

    /// `as_raw` must report the bits actually received, not a value rebuilt
    /// from the decoded fields, so this pins the process-data path too.
    #[test]
    fn as_raw_reports_the_word_read_from_process_data() {
        let raw = 0xE9C5u16; // sets bits 0, 2, 6, 7, 10, 11, 13, 14, 15
        // Process data arrives as a little-endian byte slice, low byte first.
        let bytes = raw.to_le_bytes();
        let mut statusword = DrvStatusWord::from_raw(0);
        TxPdoObject::read(
            &mut statusword,
            BitSlice::<u8, Lsb0>::from_slice(&bytes),
        );
        assert_eq!(statusword.as_raw(), raw);
        assert!(statusword.operation_enabled, "bit 2");
        assert!(statusword.warning, "bit 7");
        assert!(statusword.internal_limit_active, "bit 11");
    }

    /// Bit 10 is one bit carrying two different meanings, so it is only readable
    /// once the caller states which feature it asked the terminal for.
    #[test]
    fn bit10_is_read_as_whatever_the_channel_enabled() {
        let raw = (1u16 << 10) | (1 << 3);
        let statusword = DrvStatusWord::from_raw(raw);
        assert!(statusword.fault);
        assert!(statusword.bit10(StatuswordProcessDataMonitor::TxPdoToggle));
        assert!(
            statusword.bit10(StatuswordProcessDataMonitor::InputCycleCounter),
            "the same bit is the counter's low bit"
        );
        assert!(
            !statusword.bit10(StatuswordProcessDataMonitor::None),
            "with no monitor enabled the bit has no documented meaning"
        );
    }

    /// The counter is bit 10 (low) and bit 13 (high), and is only reported when
    /// that feature is actually enabled. Bit 13 is what 0x6010:01 names and what
    /// the hardware shows; 0x8010:02's bit-14 claim is contradicted by
    /// measurement and is deliberately not encoded here.
    #[test]
    fn the_input_cycle_counter_is_two_bits_from_10_and_13() {
        for (raw, expected) in [
            (0u16, 0u8),
            ((1 << 10), 1),
            ((1 << 13), 2),
            ((1 << 10) | (1 << 13), 3),
        ] {
            let statusword = DrvStatusWord::from_raw(raw);
            assert_eq!(
                statusword.input_cycle_counter(StatuswordProcessDataMonitor::InputCycleCounter),
                Some(expected),
                "raw 0x{raw:04X}"
            );
        }
        let statusword = DrvStatusWord::from_raw(0b11);
        assert_eq!(
            statusword.input_cycle_counter(StatuswordProcessDataMonitor::TxPdoToggle),
            None,
            "no counter was enabled, so there is no reading to misread"
        );
    }

    /// Bit 14 must not be mistaken for the counter's high bit. It was set in 0 of
    /// 2000 samples over 2000 real master cycles, so treating it as the high bit
    /// yields a counter that can only ever read 0 or 1 and never reaches 2 or 3.
    #[test]
    fn bit14_is_not_the_counter_high_bit() {
        let m = StatuswordProcessDataMonitor::InputCycleCounter;
        for raw in [(1 << 14) | (1 << 10), (1 << 14), 1 << 10] {
            let decoded = DrvStatusWord::from_raw(raw).input_cycle_counter(m);
            assert!(
                decoded.is_some_and(|c| c <= 1),
                "raw 0x{raw:04X} must not read as 2 or 3 via bit 14, got {decoded:?}"
            );
        }
    }

    /// A counter incrementing every cycle must show a low bit changing on every
    /// sample pair and a high bit on about half of them. This is the property the
    /// bit-13 choice rests on, expressed so a future change of the high bit is
    /// caught rather than silently reverting to bit 14.
    ///
    /// The words are built by overlaying the counter bits onto a real observed
    /// statusword (`0x18A7`), so the fixture cannot drift into encoding a pattern
    /// the drive would not produce.
    #[test]
    fn a_per_cycle_counter_has_a_half_rate_high_bit() {
        let base = DrvStatusWord::from_raw(0x18A7).as_raw();
        // One second of 1 kHz process data, counter wrapping 0,1,2,3.
        let samples: [u16; 8] = std::array::from_fn(|i| {
            let counter = (i % 4) as u16;
            base & !(1 << 10 | 1 << 13) | ((counter & 1) << 10) | ((counter >> 1) << 13)
        });
        let changes = |shift: u32| {
            samples
                .windows(2)
                .filter(|w| (w[0] >> shift & 1) != (w[1] >> shift & 1))
                .count()
        };
        assert_eq!(changes(10), 7, "low bit changes every cycle");
        assert_eq!(changes(13), 3, "high bit changes every other cycle");
        // And the driver must agree that all four values are reachable.
        let m = StatuswordProcessDataMonitor::InputCycleCounter;
        let read: Vec<u8> = samples
            .iter()
            .map(|raw| {
                DrvStatusWord::from_raw(*raw)
                    .input_cycle_counter(m)
                    .expect("counter enabled")
            })
            .collect();
        assert_eq!(read, vec![0, 1, 2, 3, 0, 1, 2, 3], "counter must count up and wrap");
    }

    /// The counter wraps 3 -> 0, so "advanced" is a change, not an increment. A
    /// comparison against `>` would miss the wrap and stall forever.
    #[test]
    fn the_cycle_counter_wrap_still_counts_as_advanced() {
        let m = StatuswordProcessDataMonitor::InputCycleCounter;
        let three = DrvStatusWord::from_raw((1 << 10) | (1 << 13));
        let zero = DrvStatusWord::from_raw(0);
        assert!(zero.input_cycle_advanced(m, &three), "3 -> 0 must advance");
        assert!(!zero.input_cycle_advanced(m, &zero));
        let two = DrvStatusWord::from_raw(1 << 13);
        assert!(three.input_cycle_advanced(m, &two));
    }

    /// Every decoded field must survive a round trip through the raw word,
    /// including the two bits that are now private.
    #[test]
    fn decoded_bits_round_trip_through_from_raw() {
        // Bits 0,1,2,3,6,7,10,11,12,13: the documented statusword fields.
        const DECODED: u16 = 0b0110_1100_1100_1111u16;
        for raw in [0u16, DECODED, 1 << 10, 1 << 13, !0u16 & DECODED] {
            assert_eq!(
                DrvStatusWord::from_raw(raw).as_raw(),
                raw,
                "raw 0x{raw:04X}"
            );
        }
    }

    /// Bits 4, 5, 8, 9 and 15 deliberately have no decoded field, so nothing in
    /// this struct gives them a name. They are nonetheless reported faithfully by
    /// `as_raw`, which returns the word as received rather than rebuilding one
    /// from the decoded fields.
    ///
    /// Bit 13 used to be in this group, on the grounds that 0x8010:02 and
    /// 0x6010:01 contradict each other about it. Measurement settled it: bit 13
    /// behaves as the counter's high bit and bit 14 never moves, so bit 13 is now
    /// decoded and bit 14 is the one left unnamed.
    #[test]
    fn reserved_bits_have_no_field_but_survive_as_raw() {
        for bit in [4, 5, 8, 9, 15] {
            let statusword = DrvStatusWord::from_raw(1 << bit);
            assert_eq!(
                statusword.as_raw(),
                1 << bit,
                "bit {bit} is undecoded but must not be lost"
            );
        }
    }
}
