pub use common::{Configuration, RX_PDO_ASSIGNMENT_REG, TX_PDO_ASSIGNMENT_REG};

/// A device whose CoE configuration is described by a [`Configuration`].
///
/// The device builds its PDO structs from the config, so the writes from
/// [`Configuration::get_coe_writes`] always match what the device decodes.
pub trait ConfigurableDevice {
    type Config: Configuration;

    /// Store the config and switch the device's PDOs to the config's assignment
    fn set_config(&mut self, config: Self::Config);
    fn get_config(&self) -> &Self::Config;
}
