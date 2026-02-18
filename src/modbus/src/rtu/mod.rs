use std::{time::Duration};

use tokio_serial::{ SerialPortBuilderExt, SerialStream };
use tokio::io::AsyncWriteExt;

use crate::{Result, Error};
use crate::protocol::Header;

use reader::Reader;
pub use config::ClientConfig;

mod helpers;
mod reader;
mod config;

#[derive(Debug)]
pub struct Client
{
    //TODO: implement wrapper for tokio/tokio-serial
    serial:    SerialStream,
    timeout:   Duration,
    char_time: Duration,
    buf:       [u8; 252]
}

impl Client
{
    pub fn new(config: ClientConfig) -> tokio_serial::Result<Self> 
    {
        let serial = tokio_serial::new(config.path, config.baud_rate)
            .data_bits(config.data_bits)
            .parity(config.parity)
            .flow_control(config.flow_control)
            .stop_bits(config.stop_bits)
            .timeout(Duration::from_millis(500))
            .open_native_async()?;

        Ok(Self { 
            serial, 
            timeout:   config.timeout,
            char_time: config.char_time(),
            buf:       [0u8; 252]
        })
    }

    pub(crate) async fn send(&mut self, header: &Header, data: &[u8]) -> Result<()> 
    {
        use helpers::*;

        let mut buf = [0u8; 256];

        let frame = create_frame(header, data, &mut buf);

        self.serial
            .write_all(&frame)
            .await
            .map_err(Error::IoError)
    }

    pub(crate) async fn recv(&mut self, rq_header: &Header) -> Result<&mut [u8]> 
    {
        use helpers::*;

        let mut reader = Reader::new(&mut self.serial, self.timeout, self.char_time);

        let rsp_header = get_header(&mut reader).await?;

        validate_header(rq_header, &rsp_header)?;

        let function_code = rsp_header.function_code;

        check_exception(&mut reader, function_code).await?;

        let size = get_data_size(&mut reader, function_code).await?;

        // read data and copy into self.buf
        let data = create_data(&mut reader, size, &mut self.buf).await?;

        validate_crc(&mut reader).await?;

        Ok(data)
    }
}