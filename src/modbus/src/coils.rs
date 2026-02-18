#[derive(Debug)]
pub struct Coils<'a> 
{
    bytes: &'a mut [u8],
    count: u16,
}

impl<'a> Coils<'a> 
{
    /// Initialize Coils from a raw byte slice
    pub fn init(bytes: &'a mut [u8], count: u16) -> Self 
    {
        Self { bytes, count }
    }

    /// Create Coils from a bool slice
    pub fn from_bools(items: &[bool], buf: &'a mut [u8]) -> Result<Self, &'static str> 
    {
        if buf.len() * 8 < items.len() 
        {
            return Err("OutOfMemory");
        }

        buf.fill(0);

        for (i, &b) in items.iter().enumerate() 
        {
            if b 
            {
                buf[i >> 3] |= 1 << (i & 7);
            }
        }

        Ok(Self {
            bytes: buf,
            count: items.len() as u16,
        })
    }

    /// Read a single coil
    pub fn read(&self, index: u16) -> Result<bool, &'static str> 
    {
        if index >= self.count 
        {
            return Err("OutOfBounds");
        }

        let byte_index = (index / 8) as usize;
        let bit_index = index % 8;

        Ok((self.bytes[byte_index] & (1 << bit_index)) != 0)
    }

    /// Set a single coil
    pub fn set(&mut self, index: u16, value: bool) -> Result<(), &'static str> 
    {
        if index >= self.count 
        {
            return Err("OutOfBounds");
        }

        let byte_index = (index / 8) as usize;
        let bit_index = index % 8;
        let mask = 1 << bit_index;

        if value 
        {
            self.bytes[byte_index] |= mask;
        } 
        
        else 
        {
            self.bytes[byte_index] &= !mask;
        }

        Ok(())
    }

    /// Return an iterator over the coils
    pub fn iter(&self) -> CoilsIterator<'_> 
    {
        CoilsIterator {
            coils: self,
            pos: 0,
        }
    }
}

/// Iterator over the coils
pub struct CoilsIterator<'a> 
{
    coils: &'a Coils<'a>,
    pos: u16,
}

impl<'a> Iterator for CoilsIterator<'a> 
{
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> 
    {
        if self.pos >= self.coils.count 
        {
            return None;
        }
        let value = self.coils.read(self.pos).ok()?;
        self.pos += 1;
        Some(value)
    }
}