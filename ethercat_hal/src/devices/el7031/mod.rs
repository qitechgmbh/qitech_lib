pub mod coe;
pub mod pdo;
use crate::{
    helpers::counter_wrapper_u16_i128::CounterWrapperU16U128,
    io::stepper_velocity_el70x1::{
        StepperVelocityEL70x1Device, StepperVelocityEL70x1Input, StepperVelocityEL70x1Output,
    },
    pdo::{PredefinedPdoAssignment, RxPdo, TxPdo},
    shared_config::el70x1::EL70x1OperationMode,
};
use anyhow::anyhow;
use coe::EL7031Configuration;
use ethercat_hal_derive::EthercatDevice;
use pdo::{EL7031RxPdo, EL7031TxPdo};

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};

#[derive(Debug, Clone, EthercatDevice)]
pub struct EL7031 {
    pub txpdo: EL7031TxPdo,
    pub rxpdo: EL7031RxPdo,
    is_used: bool,
    pub configuration: EL7031Configuration,
    pub counter_wrapper: CounterWrapperU16U128,
}

impl EthercatDeviceProcessing for EL7031 {
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

impl NewEthercatDevice for EL7031 {
    fn new() -> Self {
        let configuration: EL7031Configuration = EL7031Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
            configuration,
            counter_wrapper: CounterWrapperU16U128::new(),
        }
    }
}

impl StepperVelocityEL70x1Device for EL7031 {
    fn set_output(
        &mut self,
        port: usize,
        value: StepperVelocityEL70x1Output,
    ) -> Result<(), anyhow::Error> {
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
                        return Err(anyhow!("stm_control is None"));
                    }
                }
                match &mut self.rxpdo.stm_velocity {
                    Some(stm_velocity) => {
                        stm_velocity.velocity = value.velocity;
                    }
                    None => {
                        return Err(anyhow!("stm_velocity is None"));
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
                "Operation mode is not velocity, but {:?}",
                self.configuration.stm_features.operation_mode
            ));
        }

        match port {
            0 => {
                let stm_status = match &self.txpdo.stm_status {
                    Some(value) => value,
                    None => return Err(anyhow!("stm_status is None")),
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

    fn get_speed_range(&self, _port: usize) -> crate::shared_config::el70x1::EL70x1SpeedRange {
        self.configuration.stm_features.speed_range
    }

    fn get_output(&self, port: usize) -> Result<StepperVelocityEL70x1Output, anyhow::Error> {
        // check if operating mode is velocity
        if self.configuration.stm_features.operation_mode != EL70x1OperationMode::DirectVelocity {
            return Err(anyhow!(
                "Operation mode is not velocity, but {:?}",
                self.configuration.stm_features.operation_mode
            ));
        }

        match port {
            0 => {
                let stm_control = match &self.rxpdo.stm_control {
                    Some(value) => value,
                    None => return Err(anyhow!("stm_control is None")),
                };

                let stm_velocity = match &self.rxpdo.stm_velocity {
                    Some(value) => value,
                    None => return Err(anyhow!("stm_velocity is None")),
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

    fn get_port_count(&self) -> usize {
        1
    }

    fn get_digital_input(&self, port: usize) -> Result<bool, anyhow::Error> {
        let error1 = anyhow::anyhow!(
            "[{}::Device::digital_input_state] Port {:?} is not available",
            module_path!(),
            port
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

    fn get_analog_input(
        &self,
        _port: usize,
    ) -> Result<crate::io::analog_input::AnalogInputInput, anyhow::Error> {
        Err(anyhow!("EL7031 has no analog in ports",))
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
        let mut output = self.get_output(port).unwrap();
        output.enable = enabled;
        self.set_output(port, output);
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
pub enum EL7031StepperPort {
    STM1,
}

#[derive(Debug, Clone, Copy)]
pub enum EL7031DigitalInputPort {
    DI1,
    DI2,
}

pub const EL7031_VENDOR_ID: u32 = 0x2;
pub const EL7031_PRODUCT_ID: u32 = 0x1b773052;
pub const EL7031_REVISION_A: u32 = 0x1A0000;
pub const EL7031_REVISION_B: u32 = 0x190000;
pub const EL7031_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7031_VENDOR_ID, EL7031_PRODUCT_ID, EL7031_REVISION_A);
pub const EL7031_IDENTITY_B: SubDeviceIdentityTuple =
    (EL7031_VENDOR_ID, EL7031_PRODUCT_ID, EL7031_REVISION_B);
