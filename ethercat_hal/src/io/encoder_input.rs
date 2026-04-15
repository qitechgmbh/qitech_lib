use crate::devices::EthercatDevice;

#[derive(Debug, Clone)]
pub struct EncoderInputCounter {
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct EncoderInputFrequency {
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct EncoderInputPeriod {
    pub value: u32,
}

pub trait EncoderInputDevice : EthercatDevice
{
    fn get_counter_value(&self, port: usize) -> Result<EncoderInputCounter, anyhow::Error>;
    fn get_frequency(&self, port: usize) -> Result<Option<EncoderInputFrequency>, anyhow::Error>;
    fn get_period(&self, port: usize) -> Result<Option<EncoderInputPeriod>, anyhow::Error>;
    fn set_counter(&mut self, port: usize, value: u32) -> Result<(), anyhow::Error>;
}
