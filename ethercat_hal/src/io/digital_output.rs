pub trait DigitalOutputDevice {
    fn set_output(&mut self, port: usize, value: bool);    
    fn get_port_count(&self) -> usize;
}
