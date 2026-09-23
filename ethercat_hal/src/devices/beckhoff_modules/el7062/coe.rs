use super::{
    EL7062,
    pdo::{CSP_MODE, EL7062PredefinedPdoAssignment},
};
use crate::{
    EtherCATThreadChannel,
    coe::{ConfigurableDevice, Configuration},
    pdo::PredefinedPdoAssignment,
};

const DRV_MODES_OF_OPERATION_CH1: (u16, u8) = (0x7010, 0x03);
const DRV_MODES_OF_OPERATION_CH2: (u16, u8) = (0x7110, 0x03);

/// DMCEA1022 drive-management controller input object (Ch.1)
const DMC_INPUTS_CH1: u16 = 0x6060;
/// DMC input object (Ch.2)
const DMC_INPUTS_CH2: u16 = 0x6160;

/// Subindices of [`DmcDriveStatus`] inside the DMC input object (DT6060).
const DMC_DRIVE_STATUS_READY_TO_ENABLE: u8 = 17;
const DMC_DRIVE_STATUS_READY: u8 = 18;
const DMC_DRIVE_STATUS_WARNING: u8 = 19;
const DMC_DRIVE_STATUS_ERROR: u8 = 20;
const DMC_DRIVE_STATUS_MOVING_POSITIVE: u8 = 21;
const DMC_DRIVE_STATUS_MOVING_NEGATIVE: u8 = 22;

/// Subindex of the DMC unit error code (`UDINT`, 0 = no error).
const DMC_ERROR_ID: u8 = 55;

/// Snapshot of the EL7062's DMC unit "DriveStatus" flags.
///
/// Unlike the CiA 402 status word (which only advances once the enable ladder
/// reaches it), the DMC unit reports its own readiness independently, so these
/// bits tell you *why* a channel will not switch on - e.g. a missing 24 V/48 V
/// motor supply leaves `ready_to_enable` = 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DmcDriveStatus {
    /// 0x6060:17 (Ch.1) / 0x6160:17 (Ch.2): power stage may be activated.
    pub ready_to_enable: bool,
    /// 0x6060:18 / 0x6160:18: DMC unit ready.
    pub ready: bool,
    /// 0x6060:19 / 0x6160:19: DMC unit warning active.
    pub warning: bool,
    /// 0x6060:20 / 0x6160:20: DMC unit error active.
    pub error: bool,
    /// 0x6060:21 / 0x6160:21: currently moving in positive direction.
    pub moving_positive: bool,
    /// 0x6060:22 / 0x6160:22: currently moving in negative direction.
    pub moving_negative: bool,
}

/// Read the DMC `DriveStatus` flags of one EL7062 channel over SDO.
///
/// The DMC input objects expose each flag as an independent BOOL subindex, so a
/// full snapshot is a handful of small SDO reads. They are serviced in every
/// master state, including `Op`, alongside the cyclic process data.
pub fn read_dmc_drive_status(
    channel: &EtherCATThreadChannel,
    device_address: u16,
    port: usize,
) -> Result<DmcDriveStatus, anyhow::Error> {
    let index = match port {
        0 => DMC_INPUTS_CH1,
        1 => DMC_INPUTS_CH2,
        _ => anyhow::bail!("Invalid port, expected 0 or 1, got {port}"),
    };

    let mut status = DmcDriveStatus::default();
    status.ready_to_enable =
        channel.sdo_read::<bool>(device_address, index, DMC_DRIVE_STATUS_READY_TO_ENABLE)?;
    status.ready = channel.sdo_read::<bool>(device_address, index, DMC_DRIVE_STATUS_READY)?;
    status.warning = channel.sdo_read::<bool>(device_address, index, DMC_DRIVE_STATUS_WARNING)?;
    status.error = channel.sdo_read::<bool>(device_address, index, DMC_DRIVE_STATUS_ERROR)?;
    status.moving_positive =
        channel.sdo_read::<bool>(device_address, index, DMC_DRIVE_STATUS_MOVING_POSITIVE)?;
    status.moving_negative =
        channel.sdo_read::<bool>(device_address, index, DMC_DRIVE_STATUS_MOVING_NEGATIVE)?;
    Ok(status)
}

/// Read the DMC unit error code of one EL7062 channel over SDO
/// (`0x6060:55` Ch.1 / `0x6160:55` Ch.2). `0` means no error recorded.
pub fn read_dmc_error_id(
    channel: &EtherCATThreadChannel,
    device_address: u16,
    port: usize,
) -> Result<u32, anyhow::Error> {
    let index = match port {
        0 => DMC_INPUTS_CH1,
        1 => DMC_INPUTS_CH2,
        _ => anyhow::bail!("Invalid port, expected 0 or 1, got {port}"),
    };
    channel.sdo_read::<u32>(device_address, index, DMC_ERROR_ID)
}

const DRV_AMPLIFIER_SETTINGS_CH1: u16 = 0x8010;
const DRV_AMPLIFIER_SETTINGS_CH2: u16 = 0x8110;
const FOLLOWING_ERROR_WINDOW: u8 = 0x50;
const FOLLOWING_ERROR_TIMEOUT: u8 = 0x51;

/// Per channel configuration for the EL7062
#[derive(Debug, Clone)]
pub struct EL7062ChannelConfiguration {
    /// # 0x8010:0x50 (Ch.1) / 0x8110:0x50 (Ch.2)
    /// Following error window in increments.
    ///
    /// `u32::MAX` (= 4294967295) disables the following error monitoring.
    ///
    /// default: `u32::MAX` (disabled)
    pub following_error_window: u32,

    /// # 0x8010:0x51 (Ch.1) / 0x8110:0x51 (Ch.2)
    /// Following error time out in ms. Only effective while [`following_error_window`]
    /// is enabled.
    ///
    /// default: `0`
    pub following_error_timeout: u16,
}

impl Default for EL7062ChannelConfiguration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            following_error_window: u32::MAX,
            following_error_timeout: 0,
        }
    }
}

impl EL7062ChannelConfiguration {
    fn write_config(
        &self,
        channel: EtherCATThreadChannel,
        device_address: u16,
        amplifier_settings: u16,
    ) -> Result<(), anyhow::Error> {
        tracing::debug!(
            "EL7062 following error window: 0x{:04X}:0x{:02X} = {}",
            amplifier_settings,
            FOLLOWING_ERROR_WINDOW,
            self.following_error_window
        );
        channel.sdo_write(
            device_address,
            amplifier_settings,
            FOLLOWING_ERROR_WINDOW,
            self.following_error_window,
        )?;
        tracing::debug!(
            "EL7062 following error timeout: 0x{:04X}:0x{:02X} = {}",
            amplifier_settings,
            FOLLOWING_ERROR_TIMEOUT,
            self.following_error_timeout
        );
        channel.sdo_write(
            device_address,
            amplifier_settings,
            FOLLOWING_ERROR_TIMEOUT,
            self.following_error_timeout,
        )?;
        Ok(())
    }
}

/// Configuration for the EL7062 2-channel stepper output stage (CiA 402 CSP mode)
#[derive(Debug, Clone)]
pub struct EL7062Configuration {
    pub channel_1: EL7062ChannelConfiguration,
    pub channel_2: EL7062ChannelConfiguration,
    pub pdo_assignment: EL7062PredefinedPdoAssignment,
}

impl Default for EL7062Configuration {
    /// Defaults according to the datasheet
    fn default() -> Self {
        Self {
            channel_1: EL7062ChannelConfiguration::default(),
            channel_2: EL7062ChannelConfiguration::default(),
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
        tracing::info!(
            "EL7062 write_config: device={} pdo_assignment={:?}",
            device_address,
            self.pdo_assignment
        );
        self.channel_1.write_config(
            ecat_channel.clone(),
            device_address,
            DRV_AMPLIFIER_SETTINGS_CH1,
        )?;
        self.channel_2.write_config(
            ecat_channel.clone(),
            device_address,
            DRV_AMPLIFIER_SETTINGS_CH2,
        )?;

        // persistent fallback; the mode is also fed live via the 0x1608/0x1688 PDOs
        for (name, mode_object) in [
            ("ch.1", DRV_MODES_OF_OPERATION_CH1),
            ("ch.2", DRV_MODES_OF_OPERATION_CH2),
        ] {
            tracing::debug!(
                "EL7062 {name} mode of operation: 0x{:04X}:0x{:02X} = {} (CSP)",
                mode_object.0,
                mode_object.1,
                CSP_MODE
            );
            ecat_channel.sdo_write(
                device_address,
                mode_object.0,
                mode_object.1,
                CSP_MODE,
            )?;
        }

        self.pdo_assignment
            .txpdo_assignment()
            .write_config(ecat_channel.clone(), device_address)?;
        self.pdo_assignment
            .rxpdo_assignment()
            .write_config(ecat_channel, device_address)?;
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
