use crate::protocol::ExceptionCode;

pub mod scan;
pub mod protocol;
pub mod requests;
mod client;
mod rtu;
mod encoder;

pub use client::Client;
pub use client::ClientConfigRtu;

pub(crate) use encoder::Encoder;
pub use encoder::EncoderError;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error
{
    Timeout,
    Overflow,
    InvalidCount,
    InvalidFunctionCode(u8),
    DataMalformed(DataMalformedError),
    IoError(std::io::Error),
    IllegalState,
    Exception(ExceptionCode),
}

impl From<std::io::Error> for Error 
{
    fn from(err: std::io::Error) -> Self 
    {
        Error::IoError(err)
    }
}

#[derive(Debug)]
pub enum DataMalformedError
{
    //TODO, avoid allocation
    FrameIncomplete(String),
    UnexpectedSlaveId(u8),
    UnexpectedFunctionCode(u8),
    InvalidExceptionCode(u8),
    SlaveIdMismatch((u8, u8)),
    CrcMismatch,
    DataMismatch,
}

#[derive(Debug)]
pub struct Device<'a>
{
    client: &'a mut Client,
    id:     u8, 
}