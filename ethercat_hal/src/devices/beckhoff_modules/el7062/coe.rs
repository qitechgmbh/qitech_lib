use super::EL7062;
use super::pdo::EL7062PredefinedPdoAssignment;
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

    /// # 8000:12 / 8100:12
    /// Number of single-turn bits of the position display (default: `20`).
    pub singleturn_bits: u8,

    /// # 8000:13 / 8100:13
    /// Number of multi-turn bits of the position display (default: `12`).
    /// The sum of single-turn and multi-turn bits must be `32`.
    pub multiturn_bits: u8,

    /// # 8000:14 / 8100:14
    /// Bandwidth of the speed observer in Hz (default: `200`).
    pub observer_bandwidth_hz: u16,

    /// # 8000:15 / 8100:15
    /// Observer feed-forward in % (default: `100`).
    pub observer_feed_forward: u8,

    /// # 8008:01 / 8108:01
    /// Invert feedback direction (default: `false`).
    pub invert_feedback_direction: bool,

    /// # 8008:12 / 8108:12
    /// Encoder type (default: `0` = disabled).
    pub encoder_type: u16,

    /// # 8008:13 / 8108:13
    /// Encoder increments per revolution (default: `4096`).
    pub encoder_increments_per_revolution: u32,
}

impl Default for El7062FeedbackConfiguration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            device_type: 0x00000005,       // 5 (stepper)
            singleturn_bits: 0x14,         // 20
            multiturn_bits: 0x0C,          // 12
            observer_bandwidth_hz: 0x00C8, // 200 Hz
            observer_feed_forward: 0x64,   // 100 %
            invert_feedback_direction: false,
            encoder_type: 0x0000,                          // 0 (disabled)
            encoder_increments_per_revolution: 0x00001000, // 4096
        }
    }
}

impl El7062FeedbackConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(device_address, base_index, 0x11, self.device_type)?;
        ecat_channel.sdo_write(device_address, base_index, 0x12, self.singleturn_bits)?;
        ecat_channel.sdo_write(device_address, base_index, 0x13, self.multiturn_bits)?;
        ecat_channel.sdo_write(device_address, base_index, 0x14, self.observer_bandwidth_hz)?;
        ecat_channel.sdo_write(device_address, base_index, 0x15, self.observer_feed_forward)?;
        ecat_channel.sdo_write(
            device_address,
            base_index + 0x0008, // 8008 / 8108
            0x01,
            self.invert_feedback_direction,
        )?;
        ecat_channel.sdo_write(device_address, base_index + 0x0008, 0x12, self.encoder_type)?;
        ecat_channel.sdo_write(
            device_address,
            base_index + 0x0008,
            0x13,
            self.encoder_increments_per_revolution,
        )?;
        Ok(())
    }
}

/// DRV amplifier settings for one channel of the EL7062.
///
/// Corresponds to `0x8010` / `0x8110`.
#[derive(Debug, Clone)]
pub struct El7062AmplifierConfiguration {
    /// # 8010:01 / 8110:01
    /// Show the TxPDO toggle in the status word (default: `false`).
    pub enable_txpdo_toggle: bool,

    /// # 8010:02 / 8110:02
    /// Enable the input cycle counter (default: `false`).
    pub enable_input_cycle_counter: bool,

    /// # 8010:31 / 8110:31
    /// Velocity limitation in 1/min (default: `100000`).
    pub velocity_limitation: u32,

    /// # 8010:50 / 8110:50
    /// Following error window (default: `0xFFFFFFFF` = monitoring disabled).
    pub following_error_window: u32,

    /// # 8010:51 / 8110:51
    /// Following error timeout in ms (default: `0`).
    pub following_error_timeout: u16,

    /// # 8010:64 / 8110:64
    /// Commutation type (default: `16` = stepper with internal counter).
    pub commutation_type: u8,

    /// # 8010:73 / 8110:73
    /// Acceleration limitation in 0.1 rad/s² (default: `62832`).
    pub acceleration_limitation: u32,
}

impl Default for El7062AmplifierConfiguration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            enable_txpdo_toggle: false,
            enable_input_cycle_counter: false,
            velocity_limitation: 0x000186A0,     // 100000 1/min
            following_error_window: u32::MAX,    // monitoring disabled
            following_error_timeout: 0x0000,     // 0 ms
            commutation_type: 0x10,              // 16 (stepper with internal counter)
            acceleration_limitation: 0x0000F570, // 62832 (0.1 rad/s²)
        }
    }
}

impl El7062AmplifierConfiguration {
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        ecat_channel.sdo_write(device_address, base_index, 0x01, self.enable_txpdo_toggle)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x02,
            self.enable_input_cycle_counter,
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x31, self.velocity_limitation)?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x50,
            self.following_error_window,
        )?;
        ecat_channel.sdo_write(
            device_address,
            base_index,
            0x51,
            self.following_error_timeout,
        )?;
        ecat_channel.sdo_write(device_address, base_index, 0x64, self.commutation_type)?;
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
    pub fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        base_index: u16,
    ) -> Result<(), anyhow::Error> {
        self.feedback
            .write_config(ecat_channel.clone(), device_address, base_index)?;
        self.amplifier
            .write_config(ecat_channel.clone(), device_address, base_index + 0x10)?;
        self.motor
            .write_config(ecat_channel.clone(), device_address, base_index + 0x11)?;
        Ok(())
    }
}

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
