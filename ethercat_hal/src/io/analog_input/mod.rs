use physical::{AnalogInputRange, AnalogInputValue};
pub mod physical;

#[derive(Debug, Clone)]
pub struct AnalogInputInput {
    /// from -1.0 to 1.0
    /// Can be converted to voltage or mA knowning the type and range of the device
    pub normalized: f32,
    pub wiring_error: bool,
}

impl AnalogInputInput {
    /// Convert to physical value
    pub fn get_physical(&self, range: &AnalogInputRange) -> AnalogInputValue {
        range.normalized_to_physical(self.normalized)
    }
}

pub trait AnalogInputDevice {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error>;
    fn analog_input_range(&self) -> AnalogInputRange;
    fn get_port_count(&self) -> usize;
}
