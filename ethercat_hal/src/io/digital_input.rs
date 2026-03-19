pub trait DigitalInputDevice<PORTS> : Clone
where
    PORTS: Clone,
{
    fn get_input(&self, port: PORTS) -> Result<bool, anyhow::Error>;
}
