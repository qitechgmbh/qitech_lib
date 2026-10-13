//! Drivers for EtherCAT terminals (Beckhoff, WAGO, Panasonic), built on the traits in
//! [`qitech_lib_ethercat_hal::device`].
pub mod beckhoff_modules;
pub mod io;
pub mod panasonic_modules;
pub mod wago_modules;

pub use qitech_lib_ethercat_hal::device::*;
