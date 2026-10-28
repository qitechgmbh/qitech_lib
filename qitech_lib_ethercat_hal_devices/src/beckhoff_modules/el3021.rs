use super::EthercatDeviceProcessing;
use super::{EthercatDeviceIdentity, IdentifiedDevice, NewEthercatDevice};
use qitech_lib_ethercat_hal::DeviceError;
use qitech_lib_ethercat_hal::device::{ConfigurableDevice, Configuration};
use qitech_lib_ethercat_hal::io::analog_input::AnalogCurrentInputDevice;
use qitech_lib_ethercat_hal::pdo::RxPdo;
use qitech_lib_ethercat_hal::pdo::TxPdo;
use qitech_lib_ethercat_hal::sdo::SdoRequest;
use qitech_lib_ethercat_hal::{
    pdo::{
        PredefinedPdoAssignment,
        analog_input::{AiCompact, AiStandard},
    },
    shared_config::el30xx::EL30XXChannelConfiguration,
};
use qitech_lib_ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};
use qitech_lib_units::electric_current::milliampere;
use qitech_lib_units::f64::ElectricCurrent;

#[derive(Debug, Clone)]
pub struct EL3021Configuration {
    pub pdo_assignment: EL3021PredefinedPdoAssignment,
    // Input1+ and Input1-
    pub channel1: EL30XXChannelConfiguration,
}

#[derive(EthercatDevice)]
pub struct EL3021 {
    pub configuration: EL3021Configuration,
    pub txpdo: EL3021TxPdo,
    pub rxpdo: EL3021RxPdo,
}

impl EthercatDeviceProcessing for EL3021 {}

impl std::fmt::Debug for EL3021 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL3021")
    }
}

impl Default for EL3021PredefinedPdoAssignment {
    fn default() -> Self {
        Self::Standard
    }
}

impl Default for EL3021Configuration {
    fn default() -> Self {
        Self {
            pdo_assignment: EL3021PredefinedPdoAssignment::Standard,
            channel1: EL30XXChannelConfiguration::default(),
        }
    }
}

impl NewEthercatDevice for EL3021 {
    fn new() -> Self {
        let configuration = EL3021Configuration::default();
        Self {
            txpdo: configuration.txpdo_assignment(),
            rxpdo: configuration.rxpdo_assignment(),
            configuration,
        }
    }
}

impl AnalogCurrentInputDevice for EL3021 {
    fn get_minimum_current(&self) -> ElectricCurrent {
        ElectricCurrent::new::<milliampere>(4.0)
    }

    fn get_maximum_current(&self) -> ElectricCurrent {
        ElectricCurrent::new::<milliampere>(20.0)
    }

    fn get_port_count(&self) -> usize {
        1
    }

    fn get_current_relative(&self, port: usize) -> Result<f64, DeviceError> {
        if port != 0 {
            return Err(DeviceError::InvalidPort {
                device: "EL3021",
                port,
            });
        }

        let raw = match &self.txpdo {
            EL3021TxPdo {
                ai_standard_channel1: Some(ai_standard),
                ..
            } => {
                if ai_standard.undervoltage {
                    return Err(DeviceError::ChannelFault {
                        device: "EL3021",
                        channel: port,
                        fault: "Undervoltage",
                    });
                }

                if ai_standard.overvoltage {
                    return Err(DeviceError::ChannelFault {
                        device: "EL3021",
                        channel: port,
                        fault: "Overvoltage",
                    });
                }

                if ai_standard.error {
                    return Err(DeviceError::ChannelFault {
                        device: "EL3021",
                        channel: port,
                        fault: "Wiring error",
                    });
                }

                ai_standard.value
            }
            EL3021TxPdo {
                ai_compact_channel1: Some(ai_compact),
                ..
            } => ai_compact.value,
            _ => {
                return Err(DeviceError::InvalidState {
                    device: "EL3021",
                    reason: "No input channel configured",
                });
            }
        };

        let presentation = self.configuration.channel1.presentation;
        Ok(presentation.raw_to_relative(raw))
    }
}

impl ConfigurableDevice for EL3021 {
    type Config = EL3021Configuration;

    fn set_config(&mut self, config: EL3021Configuration) {
        self.txpdo = config.txpdo_assignment();
        self.rxpdo = config.rxpdo_assignment();
        self.configuration = config;
    }

    fn get_config(&self) -> &EL3021Configuration {
        &self.configuration
    }
}

#[derive(Debug, Clone)]
pub enum EL3021Port {
    AI1,
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL3021TxPdo {
    #[pdo_object_index(0x1A00)]
    pub ai_standard_channel1: Option<AiStandard>,
    #[pdo_object_index(0x1A01)]
    pub ai_compact_channel1: Option<AiCompact>,
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL3021RxPdo {}

impl Configuration for EL3021Configuration {
    type TxPdo = EL3021TxPdo;
    type RxPdo = EL3021RxPdo;

    fn txpdo_assignment(&self) -> EL3021TxPdo {
        self.pdo_assignment.txpdo_assignment()
    }

    fn rxpdo_assignment(&self) -> EL3021RxPdo {
        self.pdo_assignment.rxpdo_assignment()
    }

    fn get_config_coe_writes(
        &self,
        device_address: u16,
    ) -> Result<Vec<SdoRequest>, qitech_lib_ethercat_hal::Error> {
        // Channel 1
        Ok(self.channel1.get_channel_coe_writes(device_address, 0x8000))
    }
}

#[derive(Debug, Clone)]
pub enum EL3021PredefinedPdoAssignment {
    Standard,
    Compact,
}

impl PredefinedPdoAssignment<EL3021TxPdo, EL3021RxPdo> for EL3021PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL3021TxPdo {
        match self {
            Self::Standard => EL3021TxPdo {
                ai_standard_channel1: Some(AiStandard::default()),
                ai_compact_channel1: None,
            },
            Self::Compact => EL3021TxPdo {
                ai_standard_channel1: None,
                ai_compact_channel1: Some(AiCompact::default()),
            },
        }
    }

    fn rxpdo_assignment(&self) -> EL3021RxPdo {
        match self {
            Self::Standard => EL3021RxPdo {},
            Self::Compact => EL3021RxPdo {},
        }
    }
}

impl IdentifiedDevice for EL3021 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 2,
        product: 197996626,
        revisions: &[1310720],
    };
}
