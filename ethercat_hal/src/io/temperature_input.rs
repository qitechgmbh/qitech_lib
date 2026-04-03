use crate::pdo::basic::Limit;
use std::{fmt};

/// Temperature Input (TI) device
///
/// Reads temperature values from the device.
pub struct TemperatureInput {
    /// Read the state of the temperature input
    get_input: Box<dyn Fn() -> TemperatureInputInput + Send + Sync>,
}

impl fmt::Debug for TemperatureInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DigitalInput")
    }
}

pub enum TemperatureInputError {
    OverVoltage,
    UnderVoltage,
}

#[derive(Debug, Clone)]
pub struct TemperatureInputInput {    
    pub temperature: f32,    
    pub undervoltage: bool,    
    pub overvoltage: bool,    
    pub limit1: Limit,
    pub limit2: Limit,    
    pub error: bool,
    /// if the TxPdo state is valid
    pub txpdo_state: bool,
    /// if the TxPdo is toggled
    pub txpdo_toggle: bool,
}

pub trait TemperatureInputDevice {
    fn get_input(&self, port: usize) -> Result<TemperatureInputInput,anyhow::Error>;
    fn get_port_count(&self) -> usize;
}
