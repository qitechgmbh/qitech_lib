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
            invert_feedback_direction: false,
            encoder: EncoderConfig::default(),
        }
    }
}

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

    /// # 8010:31 / 8110:31
    /// Velocity limitation in 1/min (default `3000`). Only effective in CSP
    /// and CSV.
    pub velocity_limitation: u32,

    /// # 8010:50 + 8010:51 / 8110:50 + 8110:51
    /// Following error monitoring (default disabled).
    pub following_error: FollowingErrorMonitor,

    /// # 8010:64 / 8110:64
    /// How the terminal determines the motor's commutation angle.
    pub commutation: Commutation,

    /// # 8010:72 / 8110:72
    /// Current reduction at standstill, in thousandths of nominal current:
    /// `1000` is full current, `0` no holding torque (default `500`).
    ///
    /// Applies while the target velocity is within `0x8010:33`, and only for
    /// commutation types 16 and 17.
    pub stand_still_torque_limitation: u16,

    /// # 8010:73 / 8110:73
    /// Acceleration limitation in 0.1 rad/s² (default `6283`).
    pub acceleration_limitation: u32,
}

impl Default for El7062AmplifierConfiguration {
    /// Power-on defaults, except the three motion limits, which are lower.
    fn default() -> Self {
        Self {
            statusword_monitor: StatuswordProcessDataMonitor::None,
            velocity_limitation: 3000, // 1/min
            following_error: FollowingErrorMonitor::Disabled,
            commutation: Commutation::StepperWithInternalCounter,
            stand_still_torque_limitation: 500, // thousandths of nominal
            acceleration_limitation: 6283,      // 0.1 rad/s²
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
        ecat_channel.sdo_write(device_address, base_index, 0x31, self.velocity_limitation)?;
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
        ecat_channel.sdo_write(device_address, base_index, 0x64, self.commutation.as_raw())?;
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
    /// Rated current of the motor in mA (default `500`).
    pub rated_current: u32,

    /// # 8011:33 / 8111:33
    /// Motor full steps per revolution (default `200`).
    pub motor_full_steps_per_revolution: u32,

    /// # 8011:34 / 8111:34
    /// Configured motor current in mA (default `500`).
    pub configured_motor_current: u32,
}

impl Default for El7062MotorConfiguration {
    /// Power-on defaults, except the currents, which follow the motor.
    ///
    /// The drive takes the smaller of `0x8011:12` and `0x8011:34`, so set both
    /// to the motor's rated current. The manual also uses `0x8011:34` to share
    /// load across the two channels, which full current on both gives up.
    fn default() -> Self {
        Self {
            rated_current: 500,                          // mA
            motor_full_steps_per_revolution: 0x000000C8, // 200
            configured_motor_current: 500,               // mA
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

/// Configuration for one channel of the EL7062.
#[derive(Debug, Clone)]
pub struct El7062ChannelConfiguration {
    /// Feedback settings (`0x8000` / `0x8100` + `0x8008` / `0x8108`)
    pub feedback: El7062FeedbackConfiguration,

    /// DRV amplifier settings (`0x8010` / `0x8110`)
    pub amplifier: El7062AmplifierConfiguration,

    /// Motor settings (`0x8011` / `0x8111`)
    pub motor: El7062MotorConfiguration,
}

impl Default for El7062ChannelConfiguration {
    fn default() -> Self {
        Self {
            feedback: El7062FeedbackConfiguration::default(),
            amplifier: El7062AmplifierConfiguration::default(),
            motor: El7062MotorConfiguration::default(),
        }
    }
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

    pub pdo_assignment: EL7062PredefinedPdoAssignment,
}

impl Default for EL7062Configuration {
    /// Each channel at its own defaults.
    fn default() -> Self {
        Self {
            channel_1: El7062ChannelConfiguration::default(),
            channel_2: El7062ChannelConfiguration::default(),
            pdo_assignment: EL7062PredefinedPdoAssignment::default(),
        }
    }
}

impl Configuration for EL7062Configuration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
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
