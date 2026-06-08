use super::analog_input::{AnalogInputInput, physical::AnalogInputRange};
use crate::{devices::EthercatDevice, helpers::el70xx_velocity_converter::EL70x1VelocityConverter};
use anyhow::Error;

#[derive(Debug, Clone)]
pub struct StepperVelocityEL70x1Input {
    /// Combination of `counter_underflow`, `counter_overflow`, and `counter_value` from [`crate::pdo::el70x1::EncControlCompact`]
    pub counter_value: i128,

    /// `ready_to_enable` from [`crate::pdo::el70x1::StmStatus`]
    pub ready_to_enable: bool,

    /// `ready` from [`crate::pdo::el70x1::StmStatus`]
    pub ready: bool,

    /// `warning` from [`crate::pdo::el70x1::StmStatus`]
    pub warning: bool,

    /// `error` from [`crate::pdo::el70x1::StmStatus`]
    pub error: bool,

    /// `moving_positive` from [`crate::pdo::el70x1::StmStatus`]
    pub moving_positive: bool,

    /// `moving_negative` from [`crate::pdo::el70x1::StmStatus`]
    pub moving_negative: bool,

    /// `torque_reduced` from [`crate::pdo::el70x1::StmStatus`]
    pub torque_reduced: bool,
}

#[derive(Debug, Clone)]
pub struct StepperVelocityEL70x1Output {
    /// `velocity` from [`crate::pdo::el70x1::StmVelocity`]
    pub velocity: i16,

    /// `enable` from [`crate::pdo::el70x1::StmControl`]
    pub enable: bool,

    /// `reduce_torque` from [`crate::pdo::el70x1::StmControl`]
    pub reduce_torque: bool,

    /// `reset` from [`crate::pdo::el70x1::StmControl`]
    pub reset: bool,

    /// `set_counter` and `set_counter_value` from [`crate::pdo::el70x1::EncControl`]
    pub set_counter: Option<i128>,
}

pub trait StepperVelocityEL70x1Device: EthercatDevice {
    fn set_output(&mut self, port: usize, value: StepperVelocityEL70x1Output) -> Result<(), Error>;
    fn get_input(&self, port: usize) -> Result<StepperVelocityEL70x1Input, Error>;
    fn get_output(&self, port: usize) -> Result<StepperVelocityEL70x1Output, Error>;
    fn get_speed_range(&self, port: usize) -> crate::shared_config::el70x1::EL70x1SpeedRange;
    /// Set the speed in steps per second
    fn set_speed(&mut self, port: usize, steps_per_second: f64) -> Result<(), Error> {
        // Get current state to preserve other output values
        let mut output = self.get_output(port).unwrap();

        // Get speed range from device to convert steps to velocity
        let speed_range = self.get_speed_range(port);
        let converter = EL70x1VelocityConverter::new(&speed_range);
        let velocity = converter.steps_to_velocity(steps_per_second, true);

        output.velocity = velocity;

        // Write to device
        self.set_output(port, output)
    }

    /// Get the speed in steps per second
    fn get_speed(&self, port: usize) -> i32 {
        let output = self.get_output(port).unwrap();
        let speed_range = self.get_speed_range(port);
        let converter = EL70x1VelocityConverter::new(&speed_range);
        converter.velocity_to_steps(output.velocity, true) as i32
    }

    fn get_port_count(&self) -> usize;
    fn is_enabled(&self, port: usize) -> bool;
    
    fn set_enabled(&mut self, port: usize, enabled: bool) {
        let mut output = self.get_output(port).unwrap();
        output.enable = enabled;
        let _ = self.set_output(port, output);
    }
    
    fn get_position(&self, port: usize) -> i128;
    fn set_position(&mut self, port: usize, position: i128);

    fn get_digital_input(&self, port: usize) -> Result<bool, anyhow::Error>;
    fn get_digital_in_port_count(&self) -> usize;

    fn get_analog_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error>;
    fn get_analog_port_count(&self) -> usize;
    fn analog_input_range(&self) -> Option<AnalogInputRange>;
}
