use crate::EtherCATThreadChannel;
pub trait Configuration {
    fn write_config(
        &self,
        channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error>;
}

/// Wraps functionality of [`Configuration`] and adds getter/setter for the config
pub trait ConfigurableDevice<C>
where
    C: Configuration + Clone,
{
    fn write_config(
        &mut self,
        channel: EtherCATThreadChannel,
        device_address: u16,
        config: &C,
    ) -> Result<(), anyhow::Error>;
    fn get_config(&self) -> C;
}

pub const TX_PDO_ASSIGNMENT_REG: u16 = 0x1C13;
pub const RX_PDO_ASSIGNMENT_REG: u16 = 0x1C12;
