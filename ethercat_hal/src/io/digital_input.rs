pub trait DigitalInputDevice {
    fn get_input(&self, port: usize) -> Result<bool, anyhow::Error>;
    fn get_port_count(&self) -> usize;
}
