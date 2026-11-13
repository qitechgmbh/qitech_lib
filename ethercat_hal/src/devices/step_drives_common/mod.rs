//! Code shared between the Beckhoff step-drive terminals (e.g. the EL7062,
//! EL7031).
//!
//! Anything that is not specific to one terminal lives here so more than one
//! driver can use it; terminal-specific bits (power-on defaults, CoE objects)
//! stay in the terminal's own module.

pub mod motion;
