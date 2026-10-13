use super::{
    EthercatDeviceIdentity, EthercatDeviceProcessing, IdentifiedDevice, NewEthercatDevice,
};
use qitech_lib_ethercat_hal_derive::EthercatDevice;

/// EK1100 bus coupler
#[derive(Clone, EthercatDevice)]
pub struct EK1100 {}

impl EthercatDeviceProcessing for EK1100 {}

impl NewEthercatDevice for EK1100 {
    fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Debug for EK1100 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EK1100")
    }
}

impl IdentifiedDevice for EK1100 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        vendor: 0x2,
        product: 0x044c2c52,
        revisions: &[0x00120000],
    };
}
