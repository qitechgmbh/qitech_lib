#[cfg(feature = "ethercrab")]
pub mod ethercrab_controller;
#[cfg(feature = "ethercrab")]
pub mod ethercrab_funcs;

#[cfg(feature = "ethercrab")]
pub use ethercrab_controller as controller;