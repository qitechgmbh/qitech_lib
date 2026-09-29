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
    /// Encoder wiring and resolution. An unused encoder has a resolution of
    /// zero, so "disabled" and "wired but how many counts" are one setting here
    /// rather than a type plus a number that can disagree.
    pub encoder: EncoderConfig,
}

impl Default for El7062FeedbackConfiguration {
    /// Defaults according to the datasheet
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
    /// Datasheet default: `0` = disabled.
    fn default() -> Self {
        Self::Disabled
    }
}

/// Encoder wiring and resolution for one channel (`0x8008` / `0x8108`).
///
/// A disabled encoder has no resolution, so holding the type and the increment
/// count as two independent fields would allow `Disabled` with a stale `4096`
/// left in place, or a wired encoder reporting zero counts per revolution.
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

    /// Resolution to write to `0x8008:13` / `0x8108:13`.
    ///
    /// `None` when disabled. The parameter is documented as the encoder's
    /// resolution after 4-fold evaluation, so with no encoder it has no value
    /// and the subindex is left untouched. Writing zero is rejected by the
    /// terminal (SDO abort `0x06090032`, "value of parameter written too low"),
    /// and writing the datasheet default instead would assert a resolution
    /// nobody measured.
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
    /// Datasheet default: encoder type 0 (disabled), which is what
    /// `0x8008:12` is `0` for. `0x8008:13` keeps the terminal's own value
    /// instead, because it is only meaningful once an encoder is wired.
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
    /// Which of the terminal's diagnostic extras, if any, appear in the spare
    /// statusword bits. See [`StatuswordProcessDataMonitor`].
    pub statusword_monitor: StatuswordProcessDataMonitor,

    /// # 8010:31 / 8110:31
    /// Velocity limitation in 1/min (default: `100000`).
    pub velocity_limitation: u32,

    /// # 8010:50 + 8010:51 / 8110:50 + 8110:51
    /// Following error monitoring. Disabled by default, and a disabled monitor
    /// has no window, so the two cannot be set inconsistently.
    pub following_error: FollowingErrorMonitor,

    /// # 8010:64 / 8110:64
    /// How the terminal determines the motor's commutation angle.
    pub commutation: Commutation,

    /// # 8010:73 / 8110:73
    /// Acceleration limitation in 0.1 rad/s² (default: `62832`).
    pub acceleration_limitation: u32,
}

impl Default for El7062AmplifierConfiguration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            statusword_monitor: StatuswordProcessDataMonitor::None,
            velocity_limitation: 0x000186A0, // 100000 1/min
            following_error: FollowingErrorMonitor::Disabled,
            commutation: Commutation::StepperWithInternalCounter,
            acceleration_limitation: 0x0000F570, // 62832 (0.1 rad/s²)
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
    /// Datasheet default: `16` = stepper with internal counter.
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
/// The datasheet disables monitoring by writing `0xFFFFFFFF` to the window and
/// leaves the timeout meaningless, which as two plain `u32`/`u16` fields is a
/// magic number that reads like a very large window.
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
    /// Datasheet default: monitoring disabled.
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
    /// Rated current of the motor in mA (default: `3000`).
    pub rated_current: u32,

    /// # 8011:33 / 8111:33
    /// Motor full steps per revolution (default: `200`).
    pub motor_full_steps_per_revolution: u32,

    /// # 8011:34 / 8111:34
    /// Configured motor current in mA (default: `3000`).
    pub configured_motor_current: u32,
}

impl Default for El7062MotorConfiguration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            rated_current: 0x00000BB8,                   // 3000 mA
            motor_full_steps_per_revolution: 0x000000C8, // 200
            configured_motor_current: 0x00000BB8,        // 3000 mA
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
    /// Commutation types 17 and 18 read the commutation angle from the encoder,
    /// so asking for them with no encoder connected leaves the drive unable to
    /// commutate. The terminal would accept both writes and then refuse to
    /// enable, which is why this is caught before writing anything.
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
    /// Defaults according to the datasheet
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

    /// Commutation types 17 and 18 read the angle from the encoder, so accepting
    /// them with 0x8008:12 = 0 leaves a drive that accepts the config and then
    /// never enables.
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

    /// The default is the internal-counter type, which works with no encoder.
    #[test]
    fn the_default_channel_configuration_is_valid() {
        assert!(El7062ChannelConfiguration::default().validate().is_ok());
    }

    /// A wired encoder plus an encoder-based commutation type is exactly the
    /// combination the check above is protecting.
    #[test]
    fn encoder_dependent_commutation_with_an_encoder_is_accepted() {
        let mut config = El7062ChannelConfiguration::default();
        config.feedback.encoder =
            EncoderConfig::wired(EncoderType::Rs422Differential, 2000).unwrap();
        config.amplifier.commutation = Commutation::StepperFocWithEncoder;
        assert!(config.validate().is_ok());
    }

    /// A disabled encoder has no resolution to write. It must report `None`
    /// rather than 0, because the terminal rejects a zero here with SDO abort
    /// 0x06090032 ("value of parameter written too low") -- observed on real
    /// hardware at 0x8108:13, i.e. the Channel 2 encoder.
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

    /// Zero counts per revolution has no valid meaning, and Disabled is not a
    /// wiring, so neither is a constructible "wired" encoder.
    #[test]
    fn an_impossible_wired_encoder_is_rejected() {
        assert!(EncoderConfig::wired(EncoderType::Rs422Differential, 0).is_err());
        assert!(EncoderConfig::wired(EncoderType::Disabled, 2000).is_err());
    }

    /// 0xFFFFFFFF is the terminal's own disabled value, so treating it as a
    /// threshold would silently disable monitoring instead of enabling it.
    #[test]
    fn a_following_error_window_of_zero_or_max_is_rejected() {
        assert!(FollowingErrorMonitor::enabled(0, 100).is_err());
        assert!(FollowingErrorMonitor::enabled(u32::MAX, 100).is_err());
        assert!(FollowingErrorMonitor::enabled(1_048_576, 100).is_ok());
    }

    #[test]
    fn a_disabled_following_error_monitor_writes_the_datasheet_values() {
        let monitor = FollowingErrorMonitor::default();
        assert!(!monitor.is_enabled());
        assert_eq!(monitor.window(), u32::MAX);
        assert_eq!(monitor.timeout(), 0);
    }

    /// The two 0x8010 booleans are one bit on the wire, so the monitor is a
    /// choice and neither value can be set at once.
    #[test]
    fn the_statusword_monitor_maps_to_exclusive_booleans() {
        use StatuswordProcessDataMonitor::*;

        assert!(!None.enable_txpdo_toggle() && !None.enable_input_cycle_counter());
        assert!(TxPdoToggle.enable_txpdo_toggle() && !TxPdoToggle.enable_input_cycle_counter());
        assert!(!InputCycleCounter.enable_txpdo_toggle());
        assert!(InputCycleCounter.enable_input_cycle_counter());

        // Datasheet default: both features off.
        assert_eq!(
            El7062AmplifierConfiguration::default().statusword_monitor,
            None
        );
    }

    /// The raw values 0x8010:64 accepts, and nothing else.
    #[test]
    fn commutation_raw_values_match_the_datasheet() {
        assert_eq!(Commutation::StepperWithInternalCounter.as_raw(), 16);
        assert_eq!(Commutation::StepperWithEncoder.as_raw(), 17);
        assert_eq!(Commutation::StepperFocWithEncoder.as_raw(), 18);
        assert!(!Commutation::default().requires_encoder());
        assert!(Commutation::StepperWithEncoder.requires_encoder());
        assert!(Commutation::StepperFocWithEncoder.requires_encoder());
    }
}
