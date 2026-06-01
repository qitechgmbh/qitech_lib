use crate::{devices::EthercatDevice, pdo::basic::Limit};
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

pub trait TemperatureInputDevice: EthercatDevice {
    fn get_input(&self, port: usize) -> Result<TemperatureInputInput, anyhow::Error>;
    fn get_port_count(&self) -> usize;
}
