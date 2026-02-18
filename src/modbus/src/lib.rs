use crate::protocol::{ExceptionCode, Header};

pub mod rtu;
pub mod scan;
pub mod protocol;
pub mod requests;
mod encoder;

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

pub trait ModbusClient
{
    fn send_recv(
        &mut self, 
        header: Header,
        data:   &[u8]
    ) -> impl Future<Output = Result<&mut [u8]>>;
}

#[derive(Debug)]
pub struct Device<'a, Client: ModbusClient>
{
    client: &'a mut Client,
    id:     u8, 
}