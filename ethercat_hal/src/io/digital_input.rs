use std::fmt::Debug;
pub trait DigitalInputDevice : Debug
{
    fn get_input(&self, port : usize) -> Result<bool, anyhow::Error>;
    fn get_port_count() -> usize;
}
