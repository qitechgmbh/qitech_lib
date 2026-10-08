use crate::{DeviceError, PdoError};
pub mod coe;
pub mod pdo;

use crate::{
    helpers::counter_wrapper_u16_i128::CounterWrapperU16U128,
    io::stepper_velocity_el70x1::{
        StepperVelocityEL70x1Device, StepperVelocityEL70x1Input, StepperVelocityEL70x1Output,
    },
    pdo::{PredefinedPdoAssignment, RxPdo, TxPdo},
    shared_config::el70x7::EL70x1OperationMode,
};
use coe::EL7037Configuration;
use pdo::{EL7037RxPdo, EL7037TxPdo};
use qitech_lib_ethercat_hal_derive::EthercatDevice;

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};

#[derive(Debug, Clone, EthercatDevice)]
pub struct EL7037 {
    pub txpdo: EL7037TxPdo,
    pub rxpdo: EL7037RxPdo,
    pub configuration: EL7037Configuration,
    pub counter_wrapper: CounterWrapperU16U128,
}

impl EthercatDeviceProcessing for EL7037 {
    fn input_post_process(&mut self) -> Result<(), DeviceError> {
        let enc_status_compact = match &self.txpdo.enc_status_compact {
            Some(value) => value,
            None => return Err(PdoError::MissingObject("enc_status_compact").into()),
        };

        // update the counter wrapper
        self.counter_wrapper.update(
            enc_status_compact.counter_value,
            enc_status_compact.counter_underflow,
            enc_status_compact.counter_overflow,
        );
        Ok(())
    }

    fn output_pre_process(&mut self) -> Result<(), DeviceError> {
        let enc_status_compact = match &self.txpdo.enc_status_compact {
            Some(value) => value,
            None => return Err(PdoError::MissingObject("enc_status_compact").into()),
        };

        let enc_control_compact = match &mut self.rxpdo.enc_control_compact {
            Some(value) => value,
            None => return Err(PdoError::MissingObject("enc_control_compact").into()),
        };

        let stm_status = match &self.txpdo.stm_status {
            Some(value) => value,
            None => return Err(PdoError::MissingObject("stm_status").into()),
        };

        let stm_control = match &mut self.rxpdo.stm_control {
            Some(value) => value,
            None => return Err(PdoError::MissingObject("stm_control").into()),
        };

        // reset errors
        if stm_status.error {
            stm_control.reset = true;
        }

        // clear counter overflow/underflow flags by setting the counter to the current value
        if enc_status_compact.counter_overflow || enc_status_compact.counter_underflow {
            enc_control_compact.set_counter = true;
            enc_control_compact.set_counter_value = enc_status_compact.counter_value;
        }

        // set counter
        match self.counter_wrapper.pop_override() {
            Some(new_counter) => {
                enc_control_compact.set_counter = true;
                enc_control_compact.set_counter_value = new_counter;
            }
            None => {
                enc_control_compact.set_counter = false;
                enc_control_compact.set_counter_value = 0;
            }
        }

        Ok(())
    }
}

impl NewEthercatDevice for EL7037 {
    fn new() -> Self {
        let configuration: EL7037Configuration = EL7037Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            configuration,
            counter_wrapper: CounterWrapperU16U128::new(),
        }
    }
}

impl StepperVelocityEL70x1Device for EL7037 {
    fn set_output(
        &mut self,
        port: usize,
        value: StepperVelocityEL70x1Output,
    ) -> Result<(), DeviceError> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            panic!(
                "Operation mode is not velocity, but {:?}",
                self.configuration.stm_features.operation_mode
            );
        }

        match port {
            0 => {
                // set the counter override if provided
                if let Some(new_counter) = value.set_counter {
                    self.counter_wrapper.push_override(new_counter);
                }

                match &mut self.rxpdo.stm_control {
                    Some(stm_control) => {
                        stm_control.enable = value.enable;
                        stm_control.reduce_torque = value.reduce_torque;
                        stm_control.reset = value.reset;
                    }
                    None => {
                        return Err(PdoError::MissingObject("stm_control").into());
                    }
                }
                match &mut self.rxpdo.stm_velocity {
                    Some(stm_velocity) => {
                        stm_velocity.velocity = value.velocity;
                    }
                    None => {
                        return Err(PdoError::MissingObject("stm_velocity").into());
                    }
                }
                Ok(())
            }
            _ => {
                return Err(DeviceError::InvalidPort {
                    device: "EL7037",
                    port,
                });
            }
        }
    }

    fn get_input(&self, port: usize) -> Result<StepperVelocityEL70x1Input, DeviceError> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            return Err(DeviceError::InvalidState {
                device: "EL7037",
                reason: "operation mode is not velocity",
            });
        }

        match port {
            0 => {
                let stm_status = match &self.txpdo.stm_status {
                    Some(value) => value,
                    None => return Err(PdoError::MissingObject("stm_status").into()),
                };

                Ok(StepperVelocityEL70x1Input {
                    counter_value: self.counter_wrapper.current(),
                    ready_to_enable: stm_status.ready_to_enable,
                    ready: stm_status.ready,
                    warning: stm_status.warning,
                    error: stm_status.error,
                    moving_positive: stm_status.moving_positive,
                    moving_negative: stm_status.moving_negative,
                    torque_reduced: stm_status.torque_reduced,
                })
            }
            _ => {
                return Err(DeviceError::InvalidPort {
                    device: "EL7037",
                    port,
                });
            }
        }
    }

    fn get_speed_range(&self, _port: usize) -> crate::shared_config::el70x7::EL70x1SpeedRange {
        self.configuration.stm_features.speed_range
    }

    fn get_output(&self, port: usize) -> Result<StepperVelocityEL70x1Output, DeviceError> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            return Err(DeviceError::InvalidState {
                device: "EL7037",
                reason: "operation mode is not velocity",
            });
        }

        match port {
            0 => {
                let stm_control = match &self.rxpdo.stm_control {
                    Some(value) => value,
                    None => return Err(PdoError::MissingObject("stm_control").into()),
                };

                let stm_velocity = match &self.rxpdo.stm_velocity {
                    Some(value) => value,
                    None => return Err(PdoError::MissingObject("stm_velocity").into()),
                };

                Ok(StepperVelocityEL70x1Output {
                    velocity: stm_velocity.velocity,
                    enable: stm_control.enable,
                    reduce_torque: stm_control.reduce_torque,
                    reset: stm_control.reset,
                    set_counter: self.counter_wrapper.get_override(),
                })
            }
            _ => {
                return Err(DeviceError::InvalidPort {
                    device: "EL7037",
                    port,
                });
            }
        }
    }

    fn get_port_count(&self) -> usize {
        1
    }

    fn get_digital_input(&self, port: usize) -> Result<bool, DeviceError> {
        let error1 = DeviceError::InvalidPort {
            device: "EL7037",
            port,
        };

        Ok(match port {
            0 => {
                self.txpdo
                    .stm_status
                    .as_ref()
                    .ok_or(error1)?
                    .digital_input_1
            }
            1 => {
                self.txpdo
                    .stm_status
                    .as_ref()
                    .ok_or(error1)?
                    .digital_input_2
            }
            _ => {
                return Err(DeviceError::InvalidPort {
                    device: "EL7037",
                    port,
                });
            }
        })
    }

    fn get_digital_in_port_count(&self) -> usize {
        2
    }

    fn get_analog_input(
        &self,
        _port: usize,
    ) -> Result<crate::io::analog_input::AnalogInputInput, DeviceError> {
        Err(DeviceError::Unsupported {
            device: "EL7037",
            feature: "analog inputs",
        })
    }

    fn get_analog_port_count(&self) -> usize {
        0
    }

    fn analog_input_range(&self) -> Option<crate::io::analog_input::physical::AnalogInputRange> {
        None
    }

    fn is_enabled(&self, port: usize) -> bool {
        match self.get_output(port) {
            Ok(output) => output.enable,
            Err(_) => false,
        }
    }

    fn get_position(&self, port: usize) -> i128 {
        let input = self.get_input(port).unwrap();
        input.counter_value
    }

    fn set_position(&mut self, port: usize, position: i128) {
        let mut output = self.get_output(port).unwrap();
        output.set_counter = Some(position);
        self.set_output(port, output).unwrap();
    }

    fn set_enabled(&mut self, port: usize, enabled: bool) {
        let output = self.get_output(port);
        let mut output = match output {
            Ok(output) => output,
            Err(_) => return,
        };
        output.enable = enabled;
        let _ = self.set_output(port, output);
    }

    fn set_speed(&mut self, port: usize, steps_per_second: f64) -> Result<(), DeviceError> {
        // Get current state to preserve other output values
        let mut output = self.get_output(port).unwrap();

        // Get speed range from device to convert steps to velocity
        let speed_range = self.get_speed_range(port);
        let converter =
            crate::helpers::el70xx_velocity_converter::EL70x1VelocityConverter::new(&speed_range);
        let velocity = converter.steps_to_velocity(steps_per_second, true);

        output.velocity = velocity;

        // Write to device
        self.set_output(port, output)
    }

    fn get_speed(&self, port: usize) -> i32 {
        let output = self.get_output(port).unwrap();
        let speed_range = self.get_speed_range(port);
        let converter =
            crate::helpers::el70xx_velocity_converter::EL70x1VelocityConverter::new(&speed_range);
        converter.velocity_to_steps(output.velocity, true) as i32
    }
}

pub const EL7037_VENDOR_ID: u32 = 0x2;
pub const EL7037_PRODUCT_ID: u32 = 0x1b7d3052;
pub const EL7037_REVISION_A: u32 = 0x00170000;
pub const EL7037_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7037_VENDOR_ID, EL7037_PRODUCT_ID, EL7037_REVISION_A);
