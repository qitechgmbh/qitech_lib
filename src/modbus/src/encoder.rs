#[derive(Debug)]
pub struct Encoder<'a>
{
    buf: &'a mut [u8],
    len: usize,
}

#[derive(Debug, Clone)]
pub enum EncoderError
{
    OutOfMemory,
}

impl<'a> Encoder<'a> 
{
    pub fn new(buf: &'a mut [u8]) -> Self
    {
        Self { buf, len: 0 }
    }

    #[allow(dead_code)]
    pub fn write_u8(&mut self, value: u8) -> Result<(), EncoderError>
    {
        if self.len + 1 > self.buf.len()
        {
            return Err(EncoderError::OutOfMemory);
        }

        self.buf[self.len] = value;
        self.len += 1;

        Ok(())
    }

    pub fn write_u16(&mut self, value: u16) -> Result<(), EncoderError>
    {
        if self.len + 2 > self.buf.len()
        {
            return Err(EncoderError::OutOfMemory);
        }

        let bytes = value.to_be_bytes();
        self.buf[self.len + 0] = bytes[0];
        self.buf[self.len + 1] = bytes[1];
        self.len += 2;

        Ok(())
    }

    pub fn data(&'a self) -> &'a [u8]
    {
        &self.buf[..self.len]
    }
}