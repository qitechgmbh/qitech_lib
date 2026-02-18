use std::time::Duration;

use tokio_serial::SerialStream;
use tokio::io::{ AsyncReadExt };

use crate::{ Error, Result };

pub struct Reader<'a>
{
    serial:   &'a mut SerialStream,
    timeout:  Duration,
    t3_5:     Duration,

    buf:     [u8; 256],
    buf_len: usize,
}

impl<'a> Reader<'a>
{
    pub fn new(serial:  &'a mut SerialStream, timeout: Duration, char_time: Duration) -> Self
    {
        Self { 
            serial, 
            timeout, 
            t3_5:    char_time * 35 / 10, 
            buf:     [0u8; 256], 
            buf_len: 0 
        }
    }

    pub fn slice(&self) -> &[u8]
    {
        &self.buf[..self.buf_len]
    }

    pub async fn read_byte(&mut self) -> Result<u8>
    {
        let data = self.read_slice(1).await?;
        Ok(data[0])
    }

    pub async fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> 
    {
        let slice = self.read_slice(N).await?;
        let arr: [u8; N] = slice.try_into().unwrap(); // safe: slice has exactly N bytes
        Ok(arr)
    }

    pub async fn read_slice(&mut self, count: usize) -> Result<&mut [u8]>
    {
        let start = self.buf_len;
        let end   = self.buf_len + count;

        if end > self.buf.len()
        {
            return Err(Error::IllegalState);
        }

        loop 
        {
            let f = self.serial.read(&mut self.buf[self.buf_len..end]);
            let n = tokio::time::timeout(self.timeout, f).await.map_err(|_| Error::Timeout)??;

            self.buf_len += n;

            if self.buf_len == end
            {
                return Ok(&mut self.buf[start..end]);
            }

            tokio::time::sleep(self.t3_5).await;
        }
    }
}