//! GRAM XTREM / XTREM-S weighing module communication protocol, v3.007.
//!
//! The XTREM is an ADPD weighing module (OIML R76:2006 / EN45501:2015): it drives a load cell
//! and publishes the weight over a communication interface. This crate speaks its protocol over
//! **UDP**, which is how QiTech deploys them — the modules get DHCP addresses on the machine
//! subnet and are reached by broadcast, with the device ID inside the frame selecting the target.
//! The RS232/RS485 variants of the same protocol are not implemented.
//!
//! # Layers
//!
//! - [`protocol`] — pure frame codec and register decoding, no I/O.
//! - [`transport`] — one shared UDP socket ([`XtremBus`]) that demultiplexes incoming frames
//!   by their `ID_O` field and routes them to the device that is waiting for them.
//! - [`discovery`] — a broadcast sweep that finds every module on the subnet.
//! - [`devices`] — device drivers. [`XtremScale`] is the weighing front-end.
//!
//! # Not implemented
//!
//! The software protection protocol (spec §6) is deliberately absent. It only engages when the
//! sealing switch is in the LOCK position, it uses a separate 20-byte binary framing on UART0,
//! and it requires a 128-bit signature that can only be set at the factory. Instead, discovery
//! reads register `0009h` and reports the sealing state so a sealed module is diagnosable
//! rather than silently mute.

pub mod devices;
pub mod discovery;
pub mod protocol;
pub mod transport;

pub use devices::{Reading, ScaleMode, XtremDevice, XtremError, XtremScale};
pub use discovery::{XtremProbe, discover};
pub use protocol::{DataAddress, Frame, Function, ProtocolError, WeighingRegister, Weight};
pub use transport::{XtremBus, XtremBusConfig, XtremBusHandle};
