use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::coe::{ConfigurableDevice, Configuration};
use crate::io::encoder_input::{
    EncoderInputCounter, EncoderInputDevice, EncoderInputFrequency, EncoderInputPeriod,
};
use crate::pdo::PredefinedPdoAssignment;
use crate::pdo::el5152::{
    El5152EncoderControl, El5152EncoderFrequency, El5152EncoderPeriod, El5152EncoderStatus,
};

use common::SdoRequest;
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};

/// EL5152 2-channel incremental encoder interface
///
/// 24V HTL, 100 kHz, dual channel

#[derive(EthercatDevice)]
pub struct EL5152 {
    pub configuration: EL5152Configuration,
    pub rxpdo: EL5152RxPdo,
    pub txpdo: EL5152TxPdo,
    is_used: bool,
}

impl EthercatDeviceProcessing for EL5152 {}

#[derive(Debug, Clone)]
pub struct EL5152Configuration {
    pub pdo_assignment: EL5152PredefinedPdoAssignment,
    pub channel1: EL5152ChannelConfiguration,
    pub channel2: EL5152ChannelConfiguration,
}

#[derive(Debug, Clone)]
pub enum EL5152PredefinedPdoAssignment {
    Period,
    Frequency,
}

impl std::fmt::Debug for EL5152 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL5152")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL5152Port {
    ENC1,
    ENC2,
}

impl Default for EL5152Configuration {
    fn default() -> Self {
        Self {
            pdo_assignment: EL5152PredefinedPdoAssignment::Period,
            channel1: EL5152ChannelConfiguration::default(),
            channel2: EL5152ChannelConfiguration::default(),
        }
    }
}

impl NewEthercatDevice for EL5152 {
    fn new() -> Self {
        let configuration: EL5152Configuration = EL5152Configuration::default();
        Self {
            rxpdo: configuration.rxpdo_assignment(),
            txpdo: configuration.txpdo_assignment(),
            configuration,
            is_used: false,
        }
    }
}

impl ConfigurableDevice for EL5152 {
    type Config = EL5152Configuration;

    fn set_config(&mut self, config: EL5152Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.rxpdo = config.rxpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL5152Configuration {
        &self.configuration
    }
}

impl EncoderInputDevice for EL5152 {
    fn get_counter_value(&self, port: usize) -> Result<EncoderInputCounter, anyhow::Error> {
        let value = match port {
            0 => self
                .txpdo
                .status_channel1
                .as_ref()
                .map_or(0, |status| status.counter_value),
            1 => self
                .txpdo
                .status_channel2
                .as_ref()
                .map_or(0, |status| status.counter_value),
            _ => return Err(anyhow::anyhow!("EL5152 only has two Encoder ports!!!")),
        };
        Ok(EncoderInputCounter { value })
    }

    fn get_frequency(&self, port: usize) -> Result<Option<EncoderInputFrequency>, anyhow::Error> {
        let frequency = match port {
            0 => self
                .txpdo
                .frequency_channel1
                .as_ref()
                .map(|f| EncoderInputFrequency {
                    value: f.frequency_value,
                }),
            1 => self
                .txpdo
                .frequency_channel2
                .as_ref()
                .map(|f| EncoderInputFrequency {
                    value: f.frequency_value,
                }),
            _ => return Err(anyhow::anyhow!("EL5152 only has two Encoder ports!!!")),
        };
        Ok(frequency)
    }

    fn get_period(&self, port: usize) -> Result<Option<EncoderInputPeriod>, anyhow::Error> {
        let period = match port {
            0 => self
                .txpdo
                .period_channel1
                .as_ref()
                .map(|p| EncoderInputPeriod {
                    value: p.period_value,
                }),
            1 => self
                .txpdo
                .period_channel2
                .as_ref()
                .map(|p| EncoderInputPeriod {
                    value: p.period_value,
                }),
            _ => return Err(anyhow::anyhow!("EL5152 only has two Encoder ports!!!")),
        };
        Ok(period)
    }

    fn set_counter(&mut self, port: usize, value: u32) -> Result<(), anyhow::Error> {
        match port {
            0 => {
                if let Some(control) = self.rxpdo.control_channel1.as_mut() {
                    control.set_counter_value = value;
                    control.set_counter = true;
                }
            }
            1 => {
                if let Some(control) = self.rxpdo.control_channel2.as_mut() {
                    control.set_counter_value = value;
                    control.set_counter = true;
                }
            }
            _ => return Err(anyhow::anyhow!("EL5152 only has two Encoder ports!!!")),
        }
        Ok(())
    }
}

impl Configuration for EL5152Configuration {
    type TxPdo = EL5152TxPdo;
    type RxPdo = EL5152RxPdo;

    fn txpdo_assignment(&self) -> EL5152TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL5152RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(&self, device_address: u16) -> Result<Vec<SdoRequest>, anyhow::Error> {
        let mut writes = Vec::new();
        // Configure channel 1
        writes.extend(self.channel1.get_channel_coe_writes(device_address, 0x8000));
        // Configure channel 2
        writes.extend(self.channel2.get_channel_coe_writes(device_address, 0x8010));
        Ok(writes)
    }
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL5152RxPdo {
    #[pdo_object_index(0x1600)]
    pub control_channel1: Option<El5152EncoderControl>,
    #[pdo_object_index(0x1602)]
    pub control_channel2: Option<El5152EncoderControl>,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL5152TxPdo {
    #[pdo_object_index(0x1A00)]
    pub status_channel1: Option<El5152EncoderStatus>,
    #[pdo_object_index(0x1A02)]
    pub period_channel1: Option<El5152EncoderPeriod>,
    #[pdo_object_index(0x1A03)]
    pub frequency_channel1: Option<El5152EncoderFrequency>,

    #[pdo_object_index(0x1A04)]
    pub status_channel2: Option<El5152EncoderStatus>,
    #[pdo_object_index(0x1A06)]
    pub period_channel2: Option<El5152EncoderPeriod>,
    #[pdo_object_index(0x1A07)]
    pub frequency_channel2: Option<El5152EncoderFrequency>,
}

impl PredefinedPdoAssignment<EL5152TxPdo, EL5152RxPdo> for EL5152PredefinedPdoAssignment {
    fn rxpdo_assignment(&self) -> EL5152RxPdo {
        EL5152RxPdo {
            control_channel1: Some(El5152EncoderControl::default()),
            control_channel2: Some(El5152EncoderControl::default()),
        }
    }

    fn txpdo_assignment(&self) -> EL5152TxPdo {
        match self {
            Self::Period => EL5152TxPdo {
                status_channel1: Some(El5152EncoderStatus::default()),
                period_channel1: Some(El5152EncoderPeriod::default()),
                frequency_channel1: None,

                status_channel2: Some(El5152EncoderStatus::default()),
                period_channel2: Some(El5152EncoderPeriod::default()),
                frequency_channel2: None,
            },
            Self::Frequency => EL5152TxPdo {
                status_channel1: Some(El5152EncoderStatus::default()),
                period_channel1: None,
                frequency_channel1: Some(El5152EncoderFrequency::default()),

                status_channel2: Some(El5152EncoderStatus::default()),
                period_channel2: None,
                frequency_channel2: Some(El5152EncoderFrequency::default()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct EL5152ChannelConfiguration {
    // 80n0:3
    pub enable_counter: u8,
    // 80n0:8
    pub disable_filter: u8,
    // 80n0:A
    pub enable_micro_increment: u8,
    // 80n0:E
    pub reversion_rotation: u8,
    // 80n0:F 0-µs, 1-ms
    pub frequency_based_window: u8,
    // 80n0:11
    pub frequency_window: u16,
    // 80n0:13
    pub frequency_scaling: u16,
    // 80n0:14
    pub period_scaling: u16,
    // 80n0:15 100: "0.01 Hz"
    pub frequency_resolution: u16,
    // 80n0:16 100: "100 ns"
    pub period_resolution: u16,
    // 80n0:17
    pub frequency_wait_time: u16,
}

impl EL5152ChannelConfiguration {
    pub fn get_channel_coe_writes(&self, device_address: u16, base_index: u16) -> Vec<SdoRequest> {
        vec![
            SdoRequest::new(device_address, base_index, 0x03, self.enable_counter),
            SdoRequest::new(device_address, base_index, 0x08, self.disable_filter),
            SdoRequest::new(
                device_address,
                base_index,
                0x0A,
                self.enable_micro_increment,
            ),
            SdoRequest::new(device_address, base_index, 0x0E, self.reversion_rotation),
            SdoRequest::new(
                device_address,
                base_index,
                0x0F,
                self.frequency_based_window,
            ),
            SdoRequest::new(device_address, base_index, 0x11, self.frequency_window),
            SdoRequest::new(device_address, base_index, 0x13, self.frequency_scaling),
            SdoRequest::new(device_address, base_index, 0x14, self.period_scaling),
            SdoRequest::new(device_address, base_index, 0x15, self.frequency_resolution),
            SdoRequest::new(device_address, base_index, 0x16, self.period_resolution),
            SdoRequest::new(device_address, base_index, 0x17, self.frequency_wait_time),
        ]
    }
}

impl Default for EL5152ChannelConfiguration {
    fn default() -> Self {
        Self {
            enable_counter: 0x01u8,
            disable_filter: 0x00u8,
            enable_micro_increment: 0x00u8,
            reversion_rotation: 0x00u8,
            frequency_based_window: 0x00u8,
            frequency_window: 0x2710u16,
            frequency_scaling: 0x0064u16,
            period_scaling: 0x0064u16,
            frequency_resolution: 0x0064u16,
            period_resolution: 0x0064u16,
            frequency_wait_time: 0x0640u16,
        }
    }
}

pub const EL5152_VENDOR_ID: u32 = 0x2;
pub const EL5152_PRODUCT_ID: u32 = 0x14203052;
pub const EL5152_REVISION_A: u32 = 0x140000;
pub const EL5152_IDENTITY_A: SubDeviceIdentityTuple =
    (EL5152_VENDOR_ID, EL5152_PRODUCT_ID, EL5152_REVISION_A);
