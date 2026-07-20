pub mod coe;
pub mod pdo;

use anyhow::anyhow;
use coe::EL7031_0030Configuration;
use ethercat_hal_derive::EthercatDevice;
use pdo::{EL7031_0030RxPdo, EL7031_0030TxPdo};
use units::{electric_potential::volt, f64::ElectricPotential};

use crate::{
    helpers::{
        counter_wrapper_u16_i128::CounterWrapperU16U128, signing_converter_u16::U16SigningConverter,
    },
    io::{
        analog_input::{AnalogInputInput, physical::AnalogInputRange},
        stepper_velocity_el70x1::{
            StepperVelocityEL70x1Device, StepperVelocityEL70x1Input, StepperVelocityEL70x1Output,
        },
    },
    pdo::{PredefinedPdoAssignment, RxPdo, TxPdo},
    shared_config::el70x1::EL70x1OperationMode,
};

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};

#[derive(Debug, Clone, EthercatDevice)]
pub struct EL7031_0030 {
    pub txpdo: EL7031_0030TxPdo,
    pub rxpdo: EL7031_0030RxPdo,
    is_used: bool,
    pub configuration: EL7031_0030Configuration,
    pub counter_wrapper: CounterWrapperU16U128,
}

impl EthercatDeviceProcessing for EL7031_0030 {
    fn input_post_process(&mut self) -> Result<(), anyhow::Error> {
        let enc_status_compact = match &self.txpdo.enc_status_compact {
            Some(value) => value,
            None => return Err(anyhow!("enc_status_compact is None")),
        };

        // update the counter wrapper
        self.counter_wrapper.update(
            enc_status_compact.counter_value,
            enc_status_compact.counter_underflow,
            enc_status_compact.counter_overflow,
        );

        Ok(())
    }

    fn output_pre_process(&mut self) -> Result<(), anyhow::Error> {
        let enc_status_compact = match &self.txpdo.enc_status_compact {
            Some(value) => value,
            None => return Err(anyhow!("enc_status_compact is None")),
        };

        let enc_control_compact = match &mut self.rxpdo.enc_control_compact {
            Some(value) => value,
            None => return Err(anyhow!("enc_control_compact is None")),
        };

        let stm_status = match &self.txpdo.stm_status {
            Some(value) => value,
            None => return Err(anyhow!("stm_status is None")),
        };

        let stm_control = match &mut self.rxpdo.stm_control {
            Some(value) => value,
            None => return Err(anyhow!("stm_control is None")),
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

impl NewEthercatDevice for EL7031_0030 {
    fn new() -> Self {
        let configuration: EL7031_0030Configuration = EL7031_0030Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
            configuration,
            counter_wrapper: CounterWrapperU16U128::new(),
        }
    }
}

impl StepperVelocityEL70x1Device for EL7031_0030 {
    fn set_output(
        &mut self,
        port: usize,
        value: StepperVelocityEL70x1Output,
    ) -> Result<(), anyhow::Error> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            panic!(
                "[{}::StepperVelocityEL70x1Device::stepper_velocity_write] Operation mode is not velocity, but {:?}",
                module_path!(),
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
                        return Err(anyhow!(
                            "[{}::StepperVelocityEL70x1Device::stepper_velocity_write] stm_control is None",
                            module_path!()
                        ));
                    }
                }
                match &mut self.rxpdo.stm_velocity {
                    Some(stm_velocity) => {
                        stm_velocity.velocity = value.velocity;
                    }
                    None => {
                        return Err(anyhow!(
                            "[{}::StepperVelocityEL70x1Device::stepper_velocity_write] stm_velocity is None",
                            module_path!()
                        ));
                    }
                }
                Ok(())
            }
            _ => {
                return Err(anyhow!(
                    "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] Invalid Port",
                    module_path!()
                ));
            }
        }
    }

    fn get_input(&self, port: usize) -> Result<StepperVelocityEL70x1Input, anyhow::Error> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            return Err(anyhow!(
                "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] Operation mode is not velocity, but {:?}",
                module_path!(),
                self.configuration.stm_features.operation_mode
            ));
        }

        match port {
            0 => {
                let stm_status = match &self.txpdo.stm_status {
                    Some(value) => value,
                    None => {
                        return Err(anyhow!(
                            "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] stm_status is None",
                            module_path!()
                        ));
                    }
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
                return Err(anyhow!(
                    "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] Invalid Port",
                    module_path!()
                ));
            }
        }
    }

    fn get_output(&self, port: usize) -> Result<StepperVelocityEL70x1Output, anyhow::Error> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            return Err(anyhow!(
                "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] Operation mode is not velocity, but {:?}",
                module_path!(),
                self.configuration.stm_features.operation_mode
            ));
        }

        match port {
            0 => {
                let stm_control = match &self.rxpdo.stm_control {
                    Some(value) => value,
                    None => {
                        return Err(anyhow!(
                            "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] stm_control is None",
                            module_path!()
                        ));
                    }
                };

                let stm_velocity = match &self.rxpdo.stm_velocity {
                    Some(value) => value,
                    None => {
                        return Err(anyhow!(
                            "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] stm_velocity is None",
                            module_path!()
                        ));
                    }
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
                return Err(anyhow!(
                    "[{}::StepperVelocityEL70x1Device::stepper_velocity_state] Invalid Port",
                    module_path!()
                ));
            }
        }
    }

    fn get_speed_range(&self, _port: usize) -> crate::shared_config::el70x1::EL70x1SpeedRange {
        self.configuration.stm_features.speed_range
    }

    fn get_port_count(&self) -> usize {
        1
    }

    fn get_analog_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error> {
        let (raw_value, wiring_error) = match port {
            0 => match &self.txpdo {
                EL7031_0030TxPdo {
                    ai_standard_channel_1: Some(ai_standard_channel_1),
                    ..
                } => (ai_standard_channel_1.value, ai_standard_channel_1.error),
                EL7031_0030TxPdo {
                    ai_compact_channel_1: Some(ai_compact_channel_1),
                    ..
                } => (ai_compact_channel_1.value, false),
                _ => return Err(anyhow::anyhow!("Invalid TxPdo assignment")),
            },
            1 => match &self.txpdo {
                EL7031_0030TxPdo {
                    ai_standard_channel_2: Some(ai_standard_channel_2),
                    ..
                } => (ai_standard_channel_2.value, ai_standard_channel_2.error),
                EL7031_0030TxPdo {
                    ai_compact_channel_2: Some(ai_compact_channel_2),
                    ..
                } => (ai_compact_channel_2.value, false),
                _ => return Err(anyhow::anyhow!("Invalid TxPdo assignment")),
            },
            _ => return Err(anyhow::anyhow!("EL7031_0030 only has TWO AnalogInputs")),
        };
        let converted_raw_value = U16SigningConverter::load_raw(raw_value);
        let value: i16 = converted_raw_value.as_signed();

        let normalized = f32::from(value) / f32::from(i16::MAX);
        Ok(AnalogInputInput {
            normalized,
            wiring_error,
        })
    }

    fn get_analog_port_count(&self) -> usize {
        2
    }

    fn analog_input_range(&self) -> Option<AnalogInputRange> {
        Some(AnalogInputRange::Potential {
            min: ElectricPotential::new::<volt>(0.0),
            max: ElectricPotential::new::<volt>(10.0),
            min_raw: 0,
            max_raw: i16::MAX,
        })
    }

    fn get_digital_input(&self, port: usize) -> Result<bool, anyhow::Error> {
        let error1 = anyhow::anyhow!(
            "[{}::DigitalInputDevice::digital_input_state] StmStatus is None",
            module_path!(),
        );
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
                return Err(anyhow!(
                    "Port {:?} is not supported for digital input EL7041_0052",
                    port
                ));
            }
        })
    }

    fn get_digital_in_port_count(&self) -> usize {
        2
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

    fn set_speed(&mut self, port: usize, steps_per_second: f64) -> Result<(), anyhow::Error> {
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

#[derive(Debug, Clone, Copy)]
pub enum EL7031_0030StepperPort {
    STM1,
}

#[derive(Debug, Clone, Copy)]
pub enum EL7031_0030DigitalInputPort {
    DI1,
    DI2,
}

#[derive(Debug, Clone, Copy)]
pub enum EL7031_0030AnalogInputPort {
    AI1,
    AI2,
}

pub const EL7031_0030_VENDOR_ID: u32 = 0x2;
pub const EL7031_0030_PRODUCT_ID: u32 = 0x1b773052;
pub const EL7031_0030_REVISION_A: u32 = 0x10001E;

pub const EL7031_0030_IDENTITY_A: SubDeviceIdentityTuple = (
    EL7031_0030_VENDOR_ID,
    EL7031_0030_PRODUCT_ID,
    EL7031_0030_REVISION_A,
);
