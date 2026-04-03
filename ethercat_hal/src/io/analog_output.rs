#[derive(Debug, Clone)]
pub struct AnalogOutputOutput(pub f32);

impl From<f32> for AnalogOutputOutput {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<AnalogOutputOutput> for f32 {
    fn from(value: AnalogOutputOutput) -> Self {
        value.0
    }
}

pub trait AnalogOutputDevice  {
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput);
    fn get_port_count(&self) -> usize;
}
