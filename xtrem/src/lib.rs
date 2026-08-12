//! GRAM XTREM / XTREM-S weighing module communication protocol, v3.007.
//!
//! The XTREM is an ADPD weighing module (OIML R76:2006 / EN45501:2015): it drives a load cell
//! and publishes the weight over a UDP communication interface.
//!
//! # Layers
//!
//! - [`protocol`] — pure frame codec and register decoding, no I/O.
//! - [`transport`] — one shared UDP socket ([`XtremBus`]) that demultiplexes incoming frames
//!   by their `ID_O` field and routes them to the device that is waiting for them.
//! - [`discovery`] — a broadcast sweep that finds every module on the subnet.
//! - [`devices`] — device drivers. [`XtremScale`] is the weighing front-end.

pub mod devices;
pub mod discovery;
pub mod protocol;
pub mod transport;

pub use devices::{Reading, ScaleMode, XtremDevice, XtremError, XtremScale};
pub use discovery::{XtremProbe, discover};
pub use protocol::{DataAddress, Frame, Function, ProtocolError, WeighingRegister, Weight};
pub use transport::{XtremBus, XtremBusConfig, XtremBusHandle};
