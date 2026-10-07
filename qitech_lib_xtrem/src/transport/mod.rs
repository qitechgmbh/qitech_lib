//! UDP transport: one socket, many modules.

mod bus;
pub mod udp;

pub use bus::{BusStats, Destination, Inbound, XtremBus, XtremBusConfig, XtremBusHandle};
pub use udp::{DEFAULT_DEVICE_LOCAL_PORT, DEFAULT_DEVICE_REMOTE_PORT, broadcast_addr_for};
