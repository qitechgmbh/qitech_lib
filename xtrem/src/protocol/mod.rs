//! The XTREM wire format: frame codec and register value decoding.
//!
//! Everything in this module is pure — no sockets, no timers — so it can be exercised
//! entirely from unit tests against the byte sequences printed in the specification.

mod address;
mod error;
pub mod frame;
mod function;
mod status;
mod value;

pub use address::DataAddress;
pub use error::ProtocolError;
pub use frame::{BROADCAST_ID, ETX, Frame, STX, lrc};
pub use function::{ExecuteResult, Function, WriteResult};
pub use status::{DeviceState, WeighingError, WeighingStatus, WifiState};
pub use value::{
    RegisterValue, WEIGHING_REGISTER_LEN, WEIGHT_FIELD_LEN, WeighingRegister, Weight, WeightUnit,
};
