use crate::DeviceError;
pub trait DigitalInputDevice {
    fn get_input(&self, port: usize) -> Result<bool, DeviceError>;
    fn get_port_count(&self) -> usize;
}
