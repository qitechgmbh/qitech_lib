use super::{
    EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice, NewEthercatDevice,
};
use qitech_lib_ethercat_hal_derive::EthercatDevice;

/// EL9505 power supply terminal, 5 V DC output voltage.
/// No process data — present on the bus purely to supply 5 V to field devices.
#[derive(Clone, EthercatDevice)]
pub struct EL9505 {}

impl EthercatDeviceProcessing for EL9505 {}

impl NewEthercatDevice for EL9505 {
    fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Debug for EL9505 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL9505")
    }
}

impl IdentifiedDevice for EL9505 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 0x25213052,
        revisions: &[0x00120000],
    };
}
