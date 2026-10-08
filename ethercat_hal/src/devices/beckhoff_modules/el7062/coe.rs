//! CoE configuration for the EL7062.
//!
//! Indices, permitted values and power-on defaults are from the EL7062 user
//! manual (Beckhoff Infosys, ch. 8.1 "CoE parameters"). Diagnostic text keyed
//! by code lives in the ESI file instead, as the manual notes in ch. 7.5.1.
//! Fields whose value here departs from the power-on default say so in the
//! `Default` impl rather than on the field.

use super::EL7062;
use super::motion::{DEFAULT_POSITION_SCALE, PositionScale};
use super::pdo::{EL7062PredefinedPdoAssignment, StatuswordProcessDataMonitor};
use crate::{
    EtherCATThreadChannel,
    coe::{ConfigurableDevice, Configuration},
    pdo::PredefinedPdoAssignment,
};

/// Feedback settings for one channel of the EL7062.
///
/// Corresponds to `0x8000` / `0x8100` ("FB Settings") and `0x8008` / `0x8108`
/// ("FB Settings ENC").
#[derive(Debug, Clone)]
pub struct El7062FeedbackConfiguration {
    /// # 8000:11 / 8100:11
    /// Device type (default: `5` = stepper).
    pub device_type: u32,

    /// # 8000:12 + 8000:13 / 8100:12 + 8100:13
    /// Process-data position scale, i.e. the single-turn and multi-turn bits.
    /// See [`PositionScale`] for why the two are held together.
    pub position_scale: PositionScale,

    /// # 8000:14 / 8100:14
    /// Bandwidth of the speed observer in Hz (default: `200`).
    pub observer_bandwidth_hz: u16,

    /// # 8000:15 / 8100:15
    /// Observer feed-forward in % (default: `100`).
    pub observer_feed_forward: u8,

    /// # 8000:17 / 8100:17
    /// Position offset, subtracted from the raw encoder position.
    /// Default: `0`, i.e. subtract nothing.
    ///
    /// The terminal accepts this write only with the axis stopped, which the
    /// configuration pass in `PreOp` satisfies.
    pub position_offset: u32,

    /// # 8000:19 + 8000:1A / 8100:19 + 8100:1A
    /// Gear ratio between motor shaft and driving (load) shaft. See
    /// [`GearRatio`].
    pub gear_ratio: GearRatio,

    /// # 8000:1B + 8000:1C / 8100:1B + 8100:1C
    /// Where the process-data position wraps. See [`PositionRange`].
    pub position_range: PositionRange,

    /// # 8008:01 / 8108:01
    /// Invert feedback direction (default: `false`).
    pub invert_feedback_direction: bool,

    /// # 8008:12 + 8008:13 / 8108:12 + 8108:13
    /// Encoder wiring and resolution. See [`EncoderConfig`].
    pub encoder: EncoderConfig,
}

impl Default for El7062FeedbackConfiguration {
    /// Power-on defaults.
    fn default() -> Self {
        Self {
            device_type: 0x00000005, // 5 (stepper)
            position_scale: DEFAULT_POSITION_SCALE,
            observer_bandwidth_hz: 0x00C8, // 200 Hz
            observer_feed_forward: 0x64,   // 100 %
            position_offset: 0x00000000,
            gear_ratio: GearRatio::default(),
            position_range: PositionRange::default(),
            invert_feedback_direction: false,
            encoder: EncoderConfig::default(),
        }
    }
}

/// Gear ratio scaling every position and speed from the motor side to the
/// load side of a gear unit (`0x8000:19` + `0x8000:1A` / `0x8100:1A`).
///
/// Non-default settings here rescale every process-data position and speed,
/// silently invalidating the "increments per revolution" assumption a caller
/// makes. Held as one value rather than two free `u32`s so a ratio can never
/// be set to the load-side expectation and forgotten: dividing by zero has no
/// meaning for "X motor revolutions per Y shaft revolutions".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GearRatio {
    /// `0x8000:19` / `0x8100:19`. `5` for a gear where 5 motor revolutions
    /// produce 2 shaft revolutions.
    pub motor_shaft_revolutions: u32,
    /// `0x8000:1A` / `0x8100:1A`.
    pub driving_shaft_revolutions: u32,
}

impl GearRatio {
    /// A ratio whose motor or shaft revolution count is 0 has no geometric
    /// meaning (and would render every rescaled value undefined).
    pub fn new(
        motor_shaft_revolutions: u32,
        driving_shaft_revolutions: u32,
    ) -> Result<Self, GearRatioError> {
        if motor_shaft_revolutions == 0 || driving_shaft_revolutions == 0 {
            return Err(GearRatioError {
                motor_shaft_revolutions,
                driving_shaft_revolutions,
            });
        }
        Ok(Self {
            motor_shaft_revolutions,
            driving_shaft_revolutions,
        })
    }

    /// `0x8000:19` / `0x8100:19`.
    pub fn motor_shaft_revolutions(self) -> u32 {
        self.motor_shaft_revolutions
    }

    /// `0x8000:1A` / `0x8100:1A`.
    pub fn driving_shaft_revolutions(self) -> u32 {
        self.driving_shaft_revolutions
    }

    /// Whether the terminal rescales positions/speeds at all.
    pub fn is_one_to_one(self) -> bool {
        self.motor_shaft_revolutions == self.driving_shaft_revolutions
    }
}

impl Default for GearRatio {
    /// Power-on default: 1:1, i.e. no rescaling — the assumption the
    /// `motion` module makes about the process-data position scale.
    fn default() -> Self {
        // Both non-zero, so `new` cannot fail.
        Self::new(1, 1).unwrap()
    }
}

/// A gear ratio the terminal cannot use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GearRatioError {
    pub motor_shaft_revolutions: u32,
    pub driving_shaft_revolutions: u32,
}

impl std::fmt::Display for GearRatioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a gear ratio needs non-zero revolution counts on both sides, got motor shaft {} / \
             driving shaft {} (0x8000:19 / 0x8000:1A)",
            self.motor_shaft_revolutions, self.driving_shaft_revolutions
        )
    }
}

impl std::error::Error for GearRatioError {}

/// Where the process-data position wraps (`0x8000:1B` + `0x8000:1C` /
/// `0x8100:1B` + `0x8100:1C`).
///
/// One value rather than two free `u32`s: an unconfigured pair with `min` at
/// or above `max` violates the terminal's own rule that min must always be
/// lower than max, so it is caught here instead of left for the terminal to
/// reject or, worse, honour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionRange {
    /// `0x8000:1B` / `0x8100:1B`: lowest position; below it the terminal
    /// underflows to the maximum.
    pub min: u32,
    /// `0x8000:1C` / `0x8100:1C`: highest position; above it the terminal
    /// overflows to the minimum.
    pub max: u32,
}

impl PositionRange {
    /// # Errors
    /// `min` must be lower than `max`.
    pub fn new(min: u32, max: u32) -> Result<Self, PositionRangeError> {
        if min >= max {
            return Err(PositionRangeError { min, max });
        }
        Ok(Self { min, max })
    }

    /// `0x8000:1B` / `0x8100:1B`.
    pub const fn min(self) -> u32 {
        self.min
    }

    /// `0x8000:1C` / `0x8100:1C`.
    pub const fn max(self) -> u32 {
        self.max
    }

    /// Whether the wrapping window covers the full 32-bit position counter.
    pub fn covers_full_range(self) -> bool {
        self.min == 0 && self.max == u32::MAX
    }
}

impl Default for PositionRange {
    /// Power-on default: the full UDINT range, where the 32-bit process-image
    /// position is one continuous wrapping counter.
    fn default() -> Self {
        // 0 <= u32::MAX, so `new` cannot fail.
        Self::new(0, u32::MAX).unwrap()
    }
}

/// A position range that contradicts the terminal's own rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionRangeError {
    pub min: u32,
    pub max: u32,
}

impl std::fmt::Display for PositionRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the position range needs min < max, got {} >= {} (0x8000:1B / 0x8000:1C)",
            self.min, self.max
        )
    }
}

impl std::error::Error for PositionRangeError {}

/// Electrical interface of the incremental encoder input (`0x8008:12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderType {
    /// No encoder connected; the terminal counts its own step pulses.
    Disabled,
    /// RS422 differential.
    Rs422Differential,
    /// TTL single ended.
    TtlSingleEnded,
    /// TTL single ended with the input filters disabled.
    TtlSingleEndedNoFilters,
    /// Open collector.
    OpenCollector,
}

impl EncoderType {
    /// Raw `0x8008:12` / `0x8108:12` value.
    pub fn as_raw(self) -> u16 {
        match self {
            Self::Disabled => 0,
            Self::Rs422Differential => 1,
            Self::TtlSingleEnded => 2,
            Self::TtlSingleEndedNoFilters => 6,
            Self::OpenCollector => 7,
        }
    }
}

impl Default for EncoderType {
    /// Power-on default: `0` = disabled.
    fn default() -> Self {
        Self::Disabled
    }
}

/// Encoder wiring and resolution for one channel (`0x8008` / `0x8108`).
///
/// One setting rather than two independent fields: otherwise `Disabled` can
/// carry a stale 4096 in `0x8008:13`, or a wired encoder can report zero counts
/// per revolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderConfig {
    /// `0x8008:12` = 0. Resolution is written as 0.
    Disabled,
    /// Encoder wired as `wiring`, resolving to `increments_per_revolution`
    /// counts per revolution *after 4-fold evaluation* (`0x8008:13`).
    Wired {
        /// Physical interface of the encoder input.
        wiring: EncoderType,
        /// Resolution after 4-fold evaluation. Must be non-zero.
        increments_per_revolution: u32,
    },
}

impl EncoderConfig {
    /// Wire an encoder of the given interface and resolution.
    ///
    /// # Errors
    /// `increments_per_revolution` must be non-zero: a wired encoder that
    /// reports zero counts per revolution has no valid position.
    pub fn wired(
        wiring: EncoderType,
        increments_per_revolution: u32,
    ) -> Result<Self, EncoderConfigError> {
        if increments_per_revolution == 0 {
            return Err(EncoderConfigError {
                wiring,
                increments_per_revolution,
            });
        }
        if wiring == EncoderType::Disabled {
            return Err(EncoderConfigError {
                wiring,
                increments_per_revolution,
            });
        }
        Ok(Self::Wired {
            wiring,
            increments_per_revolution,
        })
    }

    /// `0x8008:12` / `0x8108:12`.
    pub fn encoder_type(self) -> EncoderType {
        match self {
            Self::Disabled => EncoderType::Disabled,
            Self::Wired { wiring, .. } => wiring,
        }
    }

    /// Resolution for `0x8008:13` / `0x8108:13`; `None` when disabled.
    ///
    /// `None` rather than 0: the terminal rejects 0 here with SDO abort
    /// `0x06090032` (measured on Ch. 2), and the power-on default 4096 would
    /// assert a resolution nobody measured.
    pub fn increments_per_revolution(self) -> Option<u32> {
        match self {
            Self::Disabled => None,
            Self::Wired {
                increments_per_revolution,
                ..
            } => Some(increments_per_revolution),
        }
    }

    /// Whether an encoder is actually connected.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Wired { .. })
    }
}

impl Default for EncoderConfig {
    /// Power-on default: encoder type 0 (disabled), and `0x8008:13` left at
    /// whatever the terminal holds, since it only means something once an
    /// encoder is wired.
    fn default() -> Self {
        Self::Disabled
    }
}

/// An encoder configuration the terminal will reject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderConfigError {
    pub wiring: EncoderType,
    pub increments_per_revolution: u32,
}

impl std::fmt::Display for EncoderConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a wired EL7062 encoder needs a non-zero resolution, got {} counts per revolution \
             with wiring {:?} (0x8008:13 = 0 is rejected by the terminal with 0x06090032)",
            self.increments_per_revolution, self.wiring
        )
    }
}

impl std::error::Error for EncoderConfigError {}

impl El7062FeedbackConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(device_address, base_index, 0x11, self.device_type)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x12,
            self.position_scale.singleturn_bits(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x13,
            self.position_scale.multiturn_bits(),
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x14, self.observer_bandwidth_hz)?;
        ecat_channel.sdo_write(device_address, base_index, 0x15, self.observer_feed_forward)?;
        ecat_channel.sdo_write(device_address, base_index, 0x17, self.position_offset)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x19,
            self.gear_ratio.motor_shaft_revolutions(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x1A,
            self.gear_ratio.driving_shaft_revolutions(),
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x1B, self.position_range.min())?;
        ecat_channel.sdo_write(device_address, base_index, 0x1C, self.position_range.max())?;
        ecat_channel.sdo_write(
            device_address,
            base_index + 0x0008, // 8008 / 8108
            0x01,
            self.invert_feedback_direction,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index + 0x0008,
            0x12,
            self.encoder.encoder_type().as_raw(),
        )?;
        // 0x8008:13 has a non-zero minimum in the terminal, so a disabled encoder
        // must not write one. See `EncoderConfig::increments_per_revolution`.
        if let Some(increments) = self.encoder.increments_per_revolution() {
            ecat_channel.sdo_write(device_address, base_index + 0x0008, 0x13, increments)?;
        }
        Ok(())
    }
}

/// DRV amplifier settings for one channel of the EL7062.
///
/// Corresponds to `0x8010` / `0x8110`.
#[derive(Debug, Clone)]
pub struct El7062AmplifierConfiguration {
    /// # 8010:01 + 8010:02 / 8110:01 + 8110:02
    /// Which diagnostic extra occupies statusword bit 10, if any. See
    /// [`StatuswordProcessDataMonitor`].
    pub statusword_monitor: StatuswordProcessDataMonitor,

    /// # 8010:12 / 8110:12
    /// Integral component of the current controller, in 0.1 ms (default `50`).
    pub current_loop_integral_time: u16,

    /// # 8010:13 / 8110:13
    /// Proportional component of the current controller, in 0.1 V/A
    /// (default `50`).
    pub current_loop_proportional_gain: u16,

    /// # 8010:14 / 8110:14
    /// Integral component of the velocity controller, in 0.1 ms (default `30`).
    pub velocity_loop_integral_time: u32,

    /// # 8010:15 / 8110:15
    /// Proportional component of the velocity controller, in mA/(rad/s)
    /// (default `150`).
    pub velocity_loop_proportional_gain: u32,

    /// # 8010:17 / 8110:17
    /// Proportional component of the position controller, in 1/s (default `10`).
    pub position_loop_proportional_gain: u32,

    /// # 8010:31 / 8110:31
    /// Velocity limitation in 1/min (power-on default `100000`; this driver
    /// writes a much lower value on purpose, see `Default`). Only effective
    /// in CSP and CSV.
    pub velocity_limitation: u32,

    /// # 8010:33 / 8110:33
    /// Tolerance window for standstill monitoring, in 1/min (default `1`).
    ///
    /// Decides when the current reduction of
    /// `stand_still_torque_limitation` applies: the target velocity counts as
    /// standstill as long as it stays inside this window.
    pub stand_still_window: u16,

    /// # 8010:39 / 8110:39
    /// Which value the terminal reports in TxPDO "Info data 1". See
    /// [`InfoDataSource`].
    pub info_data_1: InfoDataSource,

    /// # 8010:3A / 8110:3A
    /// Which value the terminal reports in TxPDO "Info data 2". See
    /// [`InfoDataSource`].
    pub info_data_2: InfoDataSource,

    /// # 8010:58 / 8110:58
    /// Which value the terminal reports in TxPDO "Info data 3". See
    /// [`InfoDataSource`].
    pub info_data_3: InfoDataSource,

    /// # 8010:49 / 8110:49
    /// Halt ramp deceleration in 0.1 rad/s² (power-on default `62832`; this
    /// driver writes a much lower value on purpose, see `Default`).
    ///
    /// Used by CiA 402 "halt" — a commanded stop that keeps the drive
    /// enabled, not a quick stop.
    pub halt_ramp_deceleration: u32,

    /// # 8010:50 + 8010:51 / 8110:50 + 8110:51
    /// Following error monitoring (default disabled).
    pub following_error: FollowingErrorMonitor,

    /// # 8010:52 / 8110:52
    /// What the drive does to the axis when a drive error trips. See
    /// [`FaultReaction`].
    pub fault_reaction: FaultReaction,

    /// # 8010:57 / 8110:57
    /// Position loop velocity feed-forward gain, as a scaling factor
    /// (default `100` for 100 %).
    ///
    /// Worth writing even at its default: a value left over from TwinCAT
    /// commissioning makes CSP respond to part of the setpoint twice.
    pub velocity_feed_forward_gain: u8,

    /// # 8010:62 / 8110:62
    /// Deadband window of the position controller, in process-data
    /// increments (default `0`, i.e. no deadband).
    pub position_loop_deadband: u32,

    /// # 8010:64 / 8110:64
    /// How the terminal determines the motor's commutation angle.
    pub commutation: Commutation,

    /// # 8010:65 / 8110:65
    /// Invert direction of rotation (default `false`): negates all setpoints
    /// and actual values, so a stale `true` silently swaps every direction
    /// and sign a caller computes.
    ///
    /// Not for adapting the encoder to the motor; use
    /// `invert_feedback_direction` (`0x8008:01`) for that.
    pub invert_direction: bool,

    /// # 8010:72 / 8110:72
    /// Current reduction at standstill, in thousandths of nominal current
    /// (power-on default `0x7FFF`; this driver writes a much lower value on
    /// purpose, see `Default`).
    ///
    /// Applies while the target velocity is within `stand_still_window`, and
    /// only for commutation types 16 and 17.
    pub stand_still_torque_limitation: u16,

    /// # 8010:73 / 8110:73
    /// Acceleration limitation in 0.1 rad/s² (power-on default `62832`; this
    /// driver writes a much lower value on purpose, see `Default`).
    pub acceleration_limitation: u32,
}

impl Default for El7062AmplifierConfiguration {
    /// Power-on defaults, except the three motion limits and the standstill
    /// torque, which are deliberately lower than the power-on values.
    fn default() -> Self {
        Self {
            statusword_monitor: StatuswordProcessDataMonitor::None,
            current_loop_integral_time: 0x0032,      // 50 * 0.1 ms
            current_loop_proportional_gain: 0x0032,  // 50 * 0.1 V/A
            velocity_loop_integral_time: 0x0000001E, // 30 * 0.1 ms
            velocity_loop_proportional_gain: 0x00000096, // 150 mA/(rad/s)
            position_loop_proportional_gain: 0x0000000A, // 10 1/s
            velocity_limitation: 3000,               // 1/min, power-on default 100000
            stand_still_window: 1,                   // 1/min
            info_data_1: InfoDataSource::default(),
            info_data_2: InfoDataSource::default(),
            info_data_3: InfoDataSource::default(),
            halt_ramp_deceleration: 6283, // 0.1 rad/s², power-on default 62832
            following_error: FollowingErrorMonitor::Disabled,
            fault_reaction: FaultReaction::default(),
            velocity_feed_forward_gain: 0x64, // 100
            position_loop_deadband: 0,
            commutation: Commutation::StepperWithInternalCounter,
            invert_direction: false,
            stand_still_torque_limitation: 500, // thousandths of nominal, power-on default 0x7FFF
            acceleration_limitation: 6283,      // 0.1 rad/s², power-on default 62832
        }
    }
}

/// How the terminal determines the motor's commutation angle (`0x8010:64`).
///
/// Which of these is correct depends on whether an encoder is connected, so
/// this is checked against the channel's [`EncoderConfig`] rather than left to
/// the ordering of two SDO writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commutation {
    /// 16: stepper with internal counter. No encoder needed.
    StepperWithInternalCounter,
    /// 17: stepper with encoder. Requires a connected encoder.
    StepperWithEncoder,
    /// 18: stepper FOC with encoder. Requires a connected encoder.
    StepperFocWithEncoder,
}

impl Commutation {
    /// Raw `0x8010:64` / `0x8110:64` value.
    pub fn as_raw(self) -> u8 {
        match self {
            Self::StepperWithInternalCounter => 16,
            Self::StepperWithEncoder => 17,
            Self::StepperFocWithEncoder => 18,
        }
    }

    /// Whether this mode reads commutation from a connected encoder.
    pub fn requires_encoder(self) -> bool {
        matches!(self, Self::StepperWithEncoder | Self::StepperFocWithEncoder)
    }
}

impl Default for Commutation {
    /// Power-on default: `16` = stepper with internal counter.
    fn default() -> Self {
        Self::StepperWithInternalCounter
    }
}

impl std::fmt::Display for Commutation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_raw())
    }
}

/// What the drive does to the axis when a drive error trips
/// (`0x8010:52` / `0x8110:52`).
///
/// Worth writing even in the default: a terminal commissioned by TwinCAT to
/// coast (`0`) on fault drops a hanging load where a ramp would lower it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FaultReaction {
    /// `0`: disable drive function, motor is free to rotate (coast).
    Coast,
    /// `1`: slow down on the slow-down ramp. Power-on default.
    #[default]
    Ramp,
    /// `0xFFFE` (65534): armature short-circuit brake.
    ShortCircuitBrake,
}

impl FaultReaction {
    /// Raw `0x8010:52` / `0x8110:52` value.
    pub fn as_raw(self) -> u16 {
        match self {
            Self::Coast => 0,
            Self::Ramp => 1,
            Self::ShortCircuitBrake => 0xFFFE,
        }
    }

    /// Whether the fault reaction brings the axis to a standstill instead of
    /// leaving it free to rotate.
    pub const fn brakes_axis(self) -> bool {
        !matches!(self, Self::Coast)
    }
}

/// Which value the terminal reports in one of the three TxPDO "Info data"
/// slots (`0x8010:39`, `0x8010:3A`, `0x8010:58`).
///
/// The TxPDO object is the same 16 bits each way; this selection decides what
/// those bits mean. Each source's decode lives with the corresponding PDO
/// object in [`super::pdo::DrvInfoData`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InfoDataSource {
    /// `2`: DC-link voltage, in mV. Power-on default for Info data 1.
    #[default]
    DcLinkVoltage,
    /// `4`: PCB temperature, in 0.1 °C. Power-on default for Info data 2.
    PcbTemperature,
    /// `10`: the channel's digital inputs, in the `0x6020` bit layout.
    DigitalInputs,
}

impl InfoDataSource {
    /// Raw value for `0x8010:39`, `0x8010:3A` or `0x8010:58`.
    pub fn as_raw(self) -> u8 {
        match self {
            Self::DcLinkVoltage => 2,
            Self::PcbTemperature => 4,
            Self::DigitalInputs => 10,
        }
    }
}

/// Following error monitoring for one channel (`0x8010:50` + `0x8010:51`).
///
/// `0x8010:50` = `0xFFFFFFFF` disables monitoring; as a plain `u32` that reads
/// like a very large window rather than "off".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowingErrorMonitor {
    /// Monitoring off. Written as window `0xFFFFFFFF`.
    Disabled,
    /// Fault if the following error exceeds `window` increments for longer than
    /// `timeout` ms.
    Enabled {
        /// Error threshold in process-data increments, i.e. in the units of the
        /// channel's [`PositionScale`].
        window: u32,
        /// How long the error may stay above `window` before it faults, in ms.
        timeout: u16,
    },
}

impl FollowingErrorMonitor {
    /// Enable monitoring with a threshold and timeout.
    ///
    /// # Errors
    /// A window of `0` would fault on any error at all, and `u32::MAX` is the
    /// terminal's own "disabled" value, so both are rejected rather than
    /// silently meaning the opposite of what they look like.
    pub fn enabled(window: u32, timeout: u16) -> Result<Self, FollowingErrorMonitorError> {
        if window == 0 || window == u32::MAX {
            return Err(FollowingErrorMonitorError { window, timeout });
        }
        Ok(Self::Enabled { window, timeout })
    }

    /// `0x8010:50` / `0x8110:50`.
    pub fn window(self) -> u32 {
        match self {
            Self::Disabled => u32::MAX,
            Self::Enabled { window, .. } => window,
        }
    }

    /// `0x8010:51` / `0x8110:51`.
    pub fn timeout(self) -> u16 {
        match self {
            Self::Disabled => 0,
            Self::Enabled { timeout, .. } => timeout,
        }
    }

    /// Whether monitoring is active.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }
}

impl Default for FollowingErrorMonitor {
    /// Power-on default: monitoring disabled.
    fn default() -> Self {
        Self::Disabled
    }
}

/// A following error window the terminal will not honour as given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowingErrorMonitorError {
    pub window: u32,
    pub timeout: u16,
}

impl std::fmt::Display for FollowingErrorMonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "following error window {} is not usable: 0 faults on any error and 0xFFFFFFFF is \
             the terminal's own disabled value (use FollowingErrorMonitor::Disabled)",
            self.window
        )
    }
}

impl std::error::Error for FollowingErrorMonitorError {}

impl El7062AmplifierConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x01,
            self.statusword_monitor.enable_txpdo_toggle(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x02,
            self.statusword_monitor.enable_input_cycle_counter(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x12,
            self.current_loop_integral_time,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x13,
            self.current_loop_proportional_gain,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x14,
            self.velocity_loop_integral_time,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x15,
            self.velocity_loop_proportional_gain,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x17,
            self.position_loop_proportional_gain,
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x31, self.velocity_limitation)?;
        ecat_channel.sdo_write(device_address, base_index, 0x33, self.stand_still_window)?;
        ecat_channel.sdo_write(device_address, base_index, 0x39, self.info_data_1.as_raw())?;
        ecat_channel.sdo_write(device_address, base_index, 0x3A, self.info_data_2.as_raw())?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x49,
            self.halt_ramp_deceleration,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x50,
            self.following_error.window(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x51,
            self.following_error.timeout(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x52,
            self.fault_reaction.as_raw(),
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x58, self.info_data_3.as_raw())?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x57,
            self.velocity_feed_forward_gain,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x62,
            self.position_loop_deadband,
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x64, self.commutation.as_raw())?;
        ecat_channel.sdo_write(device_address, base_index, 0x65, self.invert_direction)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x72,
            self.stand_still_torque_limitation,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x73,
            self.acceleration_limitation,
        )?;
        Ok(())
    }
}

/// Motor settings for one channel of the EL7062.
///
/// Corresponds to `0x8011` / `0x8111`.
#[derive(Debug, Clone)]
pub struct El7062MotorConfiguration {
    /// # 8011:12 / 8111:12
    /// Rated current of the motor in mA (power-on default `3000`; this driver
    /// writes a lower value on purpose, see `Default`).
    pub rated_current: u32,

    /// # 8011:33 / 8111:33
    /// Motor full steps per revolution (default `200`).
    pub motor_full_steps_per_revolution: u32,

    /// # 8011:34 / 8111:34
    /// Configured motor current in mA (power-on default `3000`; this driver
    /// writes a lower value on purpose, see `Default`).
    pub configured_motor_current: u32,
}

impl Default for El7062MotorConfiguration {
    /// Power-on defaults, except the currents, which are lower.
    ///
    /// The drive takes the smaller of `0x8011:12` and `0x8011:34`, so set both
    /// to the motor's rated current. The manual also uses `0x8011:34` to share
    /// load across the two channels, which full current on both gives up.
    fn default() -> Self {
        Self {
            rated_current: 500,                          // mA, power-on default 3000
            motor_full_steps_per_revolution: 0x000000C8, // 200
            configured_motor_current: 500,               // mA, power-on default 3000
        }
    }
}

impl El7062MotorConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(device_address, base_index, 0x12, self.rated_current)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x33,
            self.motor_full_steps_per_revolution,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x34,
            self.configured_motor_current,
        )?;
        Ok(())
    }
}

/// Brake control for one channel of the EL7062 (`0x8012` / `0x8112`).
///
/// Only meaningful with a motor whose holding brake is wired to the channel's
/// brake output. Everything here is stored in the terminal, so a stale
/// "manual override" from TwinCAT commissioning would force the brake to stay
/// applied or released regardless of what the drive controller wants —
/// written back to the terminal defaults unless configured otherwise.
#[derive(Debug, Clone)]
pub struct El7062BrakeConfiguration {
    /// # 8012:01 / 8112:01
    /// Force the brake state manually via CoE instead of the drive
    /// controller (default `false`).
    ///
    /// Keep `false`: writing `true` makes `manual_brake_state` the only thing
    /// that moves the brake, and this driver never touches it.
    pub enable_manual_override: bool,

    /// # 8012:02 / 8112:02
    /// Brake state while `enable_manual_override` is set: `false` = release,
    /// `true` = apply (default `false`).
    pub manual_brake_state: bool,

    /// # 8012:05 / 8112:05
    /// Output polarity for releasing the brake (default `0`, i.e. `true` on
    /// the output releases it). `1` releases the brake at a de-energized
    /// output, for brakes that hold when unpowered.
    pub brake_option: u8,

    /// # 8012:09 / 8112:09
    /// External hardware release override (`0x8012:09`): `0` disabled,
    /// `2`/`3` via digital input 1, `4`/`5` via digital input 2 — the second
    /// value of each pair restricts the override to INIT/PREOP/SAFEOP (default
    /// `0`, disabled).
    ///
    /// Written as the raw value: it is one commissioning choice, not a
    /// setting this driver drives on its own.
    pub external_override: u8,

    /// # 8012:11 / 8112:11
    /// Brake release delay after current is applied, in ms (default `0`).
    pub release_delay: u16,

    /// # 8012:12 / 8112:12
    /// Brake application delay after current is switched off, in ms
    /// (default `0`).
    pub application_delay: u16,

    /// # 8012:13 / 8112:13
    /// Time the amplifier waits for the speed to reach the standstill limit
    /// before applying the holding brake regardless, in ms (default `0`).
    ///
    /// For a vertical axis this should be short, so the axis does not fall
    /// far.
    pub emergency_application_timeout: u16,

    /// # 8012:14 / 8112:14
    /// Moment of inertia of the brake, in g cm² (default `0`).
    pub brake_moment_of_inertia: u16,
}

impl Default for El7062BrakeConfiguration {
    /// Power-on defaults: brake controlled automatically by the drive
    /// controller, no delays.
    fn default() -> Self {
        Self {
            enable_manual_override: false,
            manual_brake_state: false,
            brake_option: 0,
            external_override: 0,
            release_delay: 0,
            application_delay: 0,
            emergency_application_timeout: 0,
            brake_moment_of_inertia: 0,
        }
    }
}

impl El7062BrakeConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x01,
            self.enable_manual_override,
        )?;
        // 0x8012:02 is a BIT1 object; BIT-typed entries are written as u8
        // here, as with the other Beckhoff modules in this crate.
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x02,
            u8::from(self.manual_brake_state),
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x05, self.brake_option)?;
        ecat_channel.sdo_write(device_address, base_index, 0x09, self.external_override)?;
        ecat_channel.sdo_write(device_address, base_index, 0x11, self.release_delay)?;
        ecat_channel.sdo_write(device_address, base_index, 0x12, self.application_delay)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x13,
            self.emergency_application_timeout,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x14,
            self.brake_moment_of_inertia,
        )?;
        Ok(())
    }
}

/// Signal filter settings for one channel of the EL7062 (`0x8013` / `0x8113`).
///
/// Two filter stages of the same shape. Only the filter types are written
/// here: the four REAL32 gain/damping entries per stage cannot be written with
/// the channel's typed SDO interface. That is still complete defaulting in
/// practice, because a stage whose type is "no filter" does not use its stored
/// frequencies and dampings — and "no filter" is the power-on default written
/// for both stages, which also neutralizes whatever filter commissioning the
/// terminal may hold.
#[derive(Debug, Clone)]
pub struct El7062FilterConfiguration {
    /// # 8013:10-14 / 8113:10-14
    /// Filter stage 1 (default: no filter).
    pub stage_1: El7062FilterStage,

    /// # 8013:15-19 / 8113:15-19
    /// Filter stage 2 (default: no filter).
    pub stage_2: El7062FilterStage,
}

impl Default for El7062FilterConfiguration {
    /// Power-on defaults: both stages are "no filter".
    fn default() -> Self {
        Self {
            stage_1: El7062FilterStage::default(),
            stage_2: El7062FilterStage::default(),
        }
    }
}

/// One filter stage's writable settings (`0x8013:14` / `0x8113:14`).
///
/// The REAL32 low-/high-pass frequency and damping entries around it are not
/// written by this driver, see [`El7062FilterConfiguration`].
#[derive(Debug, Clone, Default)]
pub struct El7062FilterStage {
    /// # 8013:14 / 8113:14
    /// Filter type (default [`FilterType::NoFilter`]).
    pub filter_type: FilterType,
}

/// Filter characteristic of a [`El7062FilterStage`] (`0x8013:14` / `0x8113:14`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FilterType {
    /// `0`: no filter. Power-on default.
    #[default]
    NoFilter,
    /// `1`: 1st-order low pass.
    LowPass1Order,
    /// `2`: 1st-order phase correction.
    PhaseCorrection1Order,
    /// `3`: 2nd-order low pass.
    LowPass2Order,
    /// `4`: 2nd-order phase correction.
    PhaseCorrection2Order,
    /// `5`: notch filter.
    Notch,
}

impl FilterType {
    /// Raw `0x8013:14` / `0x8113:14` value (INT16).
    pub fn as_raw(self) -> i16 {
        match self {
            Self::NoFilter => 0,
            Self::LowPass1Order => 1,
            Self::PhaseCorrection1Order => 2,
            Self::LowPass2Order => 3,
            Self::PhaseCorrection2Order => 4,
            Self::Notch => 5,
        }
    }
}

impl El7062FilterConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x14,
            self.stage_1.filter_type.as_raw(),
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x19,
            self.stage_2.filter_type.as_raw(),
        )?;
        Ok(())
    }
}

/// Configuration for one channel of the EL7062.
#[derive(Debug, Clone, Default)]
pub struct El7062ChannelConfiguration {
    /// Feedback settings (`0x8000` / `0x8100` + `0x8008` / `0x8108`)
    pub feedback: El7062FeedbackConfiguration,

    /// DRV amplifier settings (`0x8010` / `0x8110`)
    pub amplifier: El7062AmplifierConfiguration,

    /// Motor settings (`0x8011` / `0x8111`)
    pub motor: El7062MotorConfiguration,

    /// Brake settings (`0x8012` / `0x8112`)
    pub brake: El7062BrakeConfiguration,

    /// Filter settings (`0x8013` / `0x8113`)
    pub filter: El7062FilterConfiguration,
}

impl El7062ChannelConfiguration {
    /// Check the settings that span more than one CoE object.
    ///
    /// # Errors
    /// Commutation types 17 and 18 take the commutation angle from the encoder,
    /// so asking for them with `0x8008:12` = 0 configures a drive that cannot
    /// commutate. The manual does not say whether the terminal rejects that on
    /// write or only later, so it is caught here instead.
    pub fn validate(&self) -> Result<(), ChannelConfigError> {
        if self.amplifier.commutation.requires_encoder() && !self.feedback.encoder.is_enabled() {
            return Err(ChannelConfigError::CommutationNeedsEncoder {
                commutation: self.amplifier.commutation,
            });
        }
        Ok(())
    }

    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        self.validate()?;
        self.feedback
            .write_config(ecat_channel.clone(), device_address, base_index)?;
        self.amplifier
            .write_config(ecat_channel.clone(), device_address, base_index + 0x10)?;
        self.motor
            .write_config(ecat_channel.clone(), device_address, base_index + 0x11)?;
        self.brake
            .write_config(ecat_channel.clone(), device_address, base_index + 0x12)?;
        self.filter
            .write_config(ecat_channel.clone(), device_address, base_index + 0x13)?;
        Ok(())
    }
}

/// A channel configuration whose parts contradict each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelConfigError {
    /// An encoder-dependent commutation type was selected without an encoder.
    CommutationNeedsEncoder {
        /// The commutation type that cannot work.
        commutation: Commutation,
    },
}

impl std::fmt::Display for ChannelConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommutationNeedsEncoder { commutation } => write!(
                f,
                "commutation type {} ({:?}) derives the commutation angle from an encoder, but \
                 0x8008:12 is 0 (disabled); wire the encoder or use \
                 Commutation::StepperWithInternalCounter",
                commutation.as_raw(),
                commutation
            ),
        }
    }
}

impl std::error::Error for ChannelConfigError {}

/// Configuration for the EL7062 2-channel stepper motor terminal.
#[derive(Debug, Clone)]
pub struct EL7062Configuration {
    /// Configuration of channel 1
    pub channel_1: El7062ChannelConfiguration,

    /// Configuration of channel 2
    pub channel_2: El7062ChannelConfiguration,

    /// Terminal-wide amplifier settings (`0xF800`). Unlike the `0x8000`-range
    /// objects this index is not doubled per channel: there is one power
    /// stage serving both, so a stale stored configuration affects both
    /// channels and is written exactly once.
    pub amplifier_settings: El7062GlobalAmplifierConfiguration,

    pub pdo_assignment: EL7062PredefinedPdoAssignment,
}

impl Default for EL7062Configuration {
    /// Each channel and the terminal itself at their own defaults.
    fn default() -> Self {
        Self {
            channel_1: El7062ChannelConfiguration::default(),
            channel_2: El7062ChannelConfiguration::default(),
            amplifier_settings: El7062GlobalAmplifierConfiguration::default(),
            pdo_assignment: EL7062PredefinedPdoAssignment::default(),
        }
    }
}

/// Terminal-wide power-stage settings (`0xF800`).
#[derive(Debug, Clone)]
pub struct El7062GlobalAmplifierConfiguration {
    /// # F800:10
    /// Nominal DC-link voltage, in mV (default `48000`).
    pub nominal_dc_link_voltage: u32,

    /// # F800:11
    /// Minimum DC-link voltage, in mV (default `6800`); below it a drive
    /// error triggers, or an inactive axis refuses to switch on.
    pub min_dc_link_voltage: u32,

    /// # F800:12
    /// Maximum DC-link voltage, in mV (default `60000`); above it a drive
    /// error triggers, or an inactive axis refuses to switch on.
    pub max_dc_link_voltage: u32,

    /// # F800:15
    /// Amplifier temperature warning threshold, in 0.1 °C (default `800`).
    pub temperature_warn_level: u16,

    /// # F800:16
    /// Amplifier temperature error threshold, in 0.1 °C (default `1000`).
    pub temperature_error_level: u16,

    /// # F800:18
    /// Whether an external fan raises the permissible current. See
    /// [`FanConfiguration`].
    ///
    /// Worth writing even at its default: this object raises the allowed
    /// current, so a stale `1` on a fan-less rig silently overrates the
    /// terminal.
    pub fan: FanConfiguration,
}

impl Default for El7062GlobalAmplifierConfiguration {
    /// Power-on defaults.
    fn default() -> Self {
        Self {
            nominal_dc_link_voltage: 0x0000BB80, // 48000 mV
            min_dc_link_voltage: 0x00001A90,     // 6800 mV
            max_dc_link_voltage: 0x0000EA60,     // 60000 mV
            temperature_warn_level: 0x0320,      // 800 * 0.1 °C
            temperature_error_level: 0x03E8,     // 1000 * 0.1 °C
            fan: FanConfiguration::default(),
        }
    }
}

/// External fan wiring at the terminal (`0xF800:18`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FanConfiguration {
    /// `0`: no fan installed; the nominal (non-fan) current applies.
    /// Power-on default.
    #[default]
    NotInstalled,
    /// `1`: fan installed; the higher with-fan current applies.
    Installed,
}

impl FanConfiguration {
    /// Raw `0xF800:18` value.
    pub fn as_raw(self) -> u8 {
        match self {
            Self::NotInstalled => 0,
            Self::Installed => 1,
        }
    }
}

impl El7062GlobalAmplifierConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(device_address, 0xF800, 0x10, self.nominal_dc_link_voltage)?;
        ecat_channel.sdo_write(device_address, 0xF800, 0x11, self.min_dc_link_voltage)?;
        ecat_channel.sdo_write(device_address, 0xF800, 0x12, self.max_dc_link_voltage)?;
        ecat_channel.sdo_write(device_address, 0xF800, 0x15, self.temperature_warn_level)?;
        ecat_channel.sdo_write(device_address, 0xF800, 0x16, self.temperature_error_level)?;
        ecat_channel.sdo_write(device_address, 0xF800, 0x18, self.fan.as_raw())?;
        Ok(())
    }
}

impl Configuration for EL7062Configuration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        // Terminal-wide power-stage objects (0xF800)
        self.amplifier_settings
            .write_config(ecat_channel.clone(), device_address)?;
        // Channel 1 CoE objects
        self.channel_1
            .write_config(ecat_channel.clone(), device_address, 0x8000)?;
        // Modes of operation Ch. 1
        ecat_channel.sdo_write(
            device_address,
            0x7010,
            0x03,
            self.pdo_assignment.mode_of_operation(),
        )?;

        // Channel 2 CoE objects
        self.channel_2
            .write_config(ecat_channel.clone(), device_address, 0x8100)?;
        // Modes of operation Ch. 2
        ecat_channel.sdo_write(
            device_address,
            0x7110,
            0x03,
            self.pdo_assignment.mode_of_operation(),
        )?;

        // PDO assignment
        self.pdo_assignment
            .txpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
        self.pdo_assignment
            .rxpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
        Ok(())
    }
}

impl ConfigurableDevice<EL7062Configuration> for EL7062 {
    fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        config: &EL7062Configuration,
    ) -> Result<(), anyhow::Error> {
        config.write_config(ecat_channel, device_address)?;
        self.configuration = config.clone();
        self.txpdo = config.pdo_assignment.txpdo_assignment();
        self.rxpdo = config.pdo_assignment.rxpdo_assignment();
        // New process image, new bookkeeping: a fault-reset pulse mid-write
        // and an old "setpoint written" mark have nothing to do with the
        // channels as reconfigured.
        self.ch1_state = super::AxisState::default();
        self.ch2_state = super::AxisState::default();
        Ok(())
    }

    fn get_config(&self) -> EL7062Configuration {
        self.configuration.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Types 17 and 18 need the encoder, which the default channel has
    /// disabled.
    #[test]
    fn encoder_dependent_commutation_without_an_encoder_is_rejected() {
        for commutation in [
            Commutation::StepperWithEncoder,
            Commutation::StepperFocWithEncoder,
        ] {
            let config = El7062ChannelConfiguration::default();
            assert!(
                config.validate().is_ok(),
                "internal counter needs no encoder"
            );
            let mut config = config;
            config.amplifier.commutation = commutation;
            assert_eq!(
                config.validate(),
                Err(ChannelConfigError::CommutationNeedsEncoder { commutation })
            );
        }
    }

    #[test]
    fn the_default_channel_configuration_is_valid() {
        assert!(El7062ChannelConfiguration::default().validate().is_ok());
    }

    #[test]
    fn encoder_dependent_commutation_with_an_encoder_is_accepted() {
        let mut config = El7062ChannelConfiguration::default();
        config.feedback.encoder =
            EncoderConfig::wired(EncoderType::Rs422Differential, 2000).unwrap();
        config.amplifier.commutation = Commutation::StepperFocWithEncoder;
        assert!(config.validate().is_ok());
    }

    /// `None` rather than 0: a zero resolution is rejected with SDO abort
    /// `0x06090032` (measured on Ch. 2 at 0x8108:13).
    #[test]
    fn a_disabled_encoder_writes_no_resolution() {
        assert_eq!(
            EncoderConfig::default().encoder_type(),
            EncoderType::Disabled
        );
        assert_eq!(EncoderConfig::default().increments_per_revolution(), None);
        assert!(!EncoderConfig::default().is_enabled());
    }

    #[test]
    fn a_wired_encoder_keeps_its_type_and_resolution() {
        let enc = EncoderConfig::wired(EncoderType::Rs422Differential, 2000).unwrap();
        assert_eq!(enc.encoder_type().as_raw(), 1);
        assert_eq!(enc.increments_per_revolution(), Some(2000));
        assert!(enc.is_enabled());
    }

    /// Zero counts has no meaning, and `Disabled` is not a wiring.
    #[test]
    fn an_impossible_wired_encoder_is_rejected() {
        assert!(EncoderConfig::wired(EncoderType::Rs422Differential, 0).is_err());
        assert!(EncoderConfig::wired(EncoderType::Disabled, 2000).is_err());
    }

    /// `0xFFFFFFFF` is the terminal's own disabled value, so it must not pass
    /// as a very large window.
    #[test]
    fn a_following_error_window_of_zero_or_max_is_rejected() {
        assert!(FollowingErrorMonitor::enabled(0, 100).is_err());
        assert!(FollowingErrorMonitor::enabled(u32::MAX, 100).is_err());
        assert!(FollowingErrorMonitor::enabled(1_048_576, 100).is_ok());
    }

    #[test]
    fn a_disabled_following_error_monitor_writes_the_power_on_values() {
        let monitor = FollowingErrorMonitor::default();
        assert!(!monitor.is_enabled());
        assert_eq!(monitor.window(), u32::MAX);
        assert_eq!(monitor.timeout(), 0);
    }

    #[test]
    fn the_statusword_monitor_maps_to_exclusive_booleans() {
        use StatuswordProcessDataMonitor::*;

        assert!(!None.enable_txpdo_toggle() && !None.enable_input_cycle_counter());
        assert!(TxPdoToggle.enable_txpdo_toggle() && !TxPdoToggle.enable_input_cycle_counter());
        assert!(!InputCycleCounter.enable_txpdo_toggle());
        assert!(InputCycleCounter.enable_input_cycle_counter());

        // Power-on default: both features off.
        assert_eq!(
            El7062AmplifierConfiguration::default().statusword_monitor,
            None
        );
    }

    /// A zero revolution count has no meaning: "X motor revolutions per 0
    /// shaft revolutions" is not a ratio.
    #[test]
    fn a_gear_ratio_with_a_zero_side_is_rejected() {
        assert!(GearRatio::new(0, 1).is_err());
        assert!(GearRatio::new(1, 0).is_err());
        assert!(GearRatio::new(5, 2).is_ok());
        assert!(GearRatio::new(5, 2).unwrap().motor_shaft_revolutions() == 5);
        assert!(!GearRatio::new(5, 2).unwrap().is_one_to_one());
    }

    /// The default ratio must be exactly 1:1, or every "increments per
    /// revolution" a caller computes for the process-data image is wrong.
    #[test]
    fn the_default_gear_ratio_is_one_to_one() {
        assert_eq!(GearRatio::default().motor_shaft_revolutions(), 1);
        assert_eq!(GearRatio::default().driving_shaft_revolutions(), 1);
        assert!(GearRatio::default().is_one_to_one());
    }

    /// "min must always be lower than max" is the terminal's own rule, so min
    /// >= max is not constructible rather than written and hoped for.
    #[test]
    fn a_position_range_with_min_at_or_above_max_is_rejected() {
        assert!(PositionRange::new(0, 0).is_err());
        assert!(PositionRange::new(10, 10).is_err());
        assert!(PositionRange::new(10, 9).is_err());
        assert!(PositionRange::new(0, u32::MAX).is_ok());
    }

    #[test]
    fn the_default_position_range_covers_the_full_counter() {
        assert!(PositionRange::default().covers_full_range());
        assert_eq!(PositionRange::default().min(), 0);
        assert_eq!(PositionRange::default().max(), u32::MAX);
    }

    /// 0x8010:52 accepts 0, 1 and 0xFFFE and nothing else.
    #[test]
    fn fault_reaction_raw_values_match_the_manual() {
        assert_eq!(FaultReaction::Coast.as_raw(), 0);
        assert_eq!(FaultReaction::Ramp.as_raw(), 1);
        assert_eq!(FaultReaction::ShortCircuitBrake.as_raw(), 0xFFFE);
        assert_eq!(FaultReaction::default(), FaultReaction::Ramp);
        assert!(!FaultReaction::Coast.brakes_axis());
        assert!(FaultReaction::Ramp.brakes_axis());
        assert!(FaultReaction::ShortCircuitBrake.brakes_axis());
    }

    /// 0x8010:39/3A/58 accept only 2, 4 and 10.
    #[test]
    fn info_data_source_raw_values_match_the_manual() {
        assert_eq!(InfoDataSource::DcLinkVoltage.as_raw(), 2);
        assert_eq!(InfoDataSource::PcbTemperature.as_raw(), 4);
        assert_eq!(InfoDataSource::DigitalInputs.as_raw(), 10);
        assert_eq!(InfoDataSource::default(), InfoDataSource::DcLinkVoltage);
    }

    /// 0x8013:14 accepts 0..=5 and nothing else.
    #[test]
    fn filter_type_raw_values_match_the_manual() {
        assert_eq!(FilterType::NoFilter.as_raw(), 0);
        assert_eq!(FilterType::LowPass1Order.as_raw(), 1);
        assert_eq!(FilterType::PhaseCorrection1Order.as_raw(), 2);
        assert_eq!(FilterType::LowPass2Order.as_raw(), 3);
        assert_eq!(FilterType::PhaseCorrection2Order.as_raw(), 4);
        assert_eq!(FilterType::Notch.as_raw(), 5);
        assert_eq!(FilterType::default(), FilterType::NoFilter);
    }

    /// The brake defaults restore automatic control, not manual override.
    #[test]
    fn the_default_brake_configuration_is_not_manual() {
        let brake = El7062BrakeConfiguration::default();
        assert!(!brake.enable_manual_override);
        assert!(!brake.manual_brake_state);
        assert_eq!(brake.brake_option, 0);
        assert_eq!(brake.external_override, 0);
        assert_eq!(brake.release_delay, 0);
        assert_eq!(brake.application_delay, 0);
        assert_eq!(brake.emergency_application_timeout, 0);
        assert_eq!(brake.brake_moment_of_inertia, 0);
    }

    /// Both filter stages default to "no filter", which is what neutralizes
    /// any stored filter commissioning without needing the REAL32 entries.
    #[test]
    fn the_default_filter_configuration_disables_both_stages() {
        let filter = El7062FilterConfiguration::default();
        assert_eq!(filter.stage_1.filter_type, FilterType::NoFilter);
        assert_eq!(filter.stage_2.filter_type, FilterType::NoFilter);
    }

    /// 0xF800 defaults straight from the manual's table.
    #[test]
    fn the_default_global_amplifier_configuration_matches_the_manual() {
        let settings = El7062GlobalAmplifierConfiguration::default();
        assert_eq!(settings.nominal_dc_link_voltage, 48000);
        assert_eq!(settings.min_dc_link_voltage, 6800);
        assert_eq!(settings.max_dc_link_voltage, 60000);
        assert_eq!(settings.temperature_warn_level, 800);
        assert_eq!(settings.temperature_error_level, 1000);
        assert_eq!(settings.fan, FanConfiguration::NotInstalled);
        assert_eq!(FanConfiguration::Installed.as_raw(), 1);
    }

    /// The objects the review singled out as never written are all present in
    /// the channel's defaults, so a write_config pass touches a terminal
    /// commissioned by other tooling without silently leaving stored values.
    #[test]
    fn the_default_channel_configuration_covers_the_never_written_objects() {
        let channel = El7062ChannelConfiguration::default();
        assert_eq!(channel.feedback.position_offset, 0);
        assert!(channel.feedback.gear_ratio.is_one_to_one());
        assert!(channel.feedback.position_range.covers_full_range());
        assert_eq!(channel.amplifier.stand_still_window, 1);
        assert_eq!(channel.amplifier.halt_ramp_deceleration, 6283);
        assert_eq!(channel.amplifier.fault_reaction, FaultReaction::Ramp);
        assert_eq!(channel.amplifier.velocity_feed_forward_gain, 100);
        assert_eq!(channel.amplifier.position_loop_deadband, 0);
        assert!(!channel.amplifier.invert_direction);
    }

    /// 0x8010:64 accepts 16, 17, 18 and nothing else.
    #[test]
    fn commutation_raw_values_match_the_manual() {
        assert_eq!(Commutation::StepperWithInternalCounter.as_raw(), 16);
        assert_eq!(Commutation::StepperWithEncoder.as_raw(), 17);
        assert_eq!(Commutation::StepperFocWithEncoder.as_raw(), 18);
        assert!(!Commutation::default().requires_encoder());
        assert!(Commutation::StepperWithEncoder.requires_encoder());
        assert!(Commutation::StepperFocWithEncoder.requires_encoder());
    }
}
