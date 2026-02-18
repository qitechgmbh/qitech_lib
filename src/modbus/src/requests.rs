use crate::{ Client, DataMalformedError, Device, Encoder, Error, Result };

use crate::protocol::{ FunctionCode, Header };

// transport protocol indepdendent API functions
impl<'a> Device<'a> 
{
    pub fn new(client: &'a mut Client, id: u8) -> Self
    {
        Self { client, id }
    }

    pub async fn read_holding_registers<'b>(
        &'b mut self,
        starting_address: u16, 
        count: u16
    ) -> Result<&'b [u16]>
    {
        let header = Header { 
            slave_id:      self.id, 
            function_code: FunctionCode::ReadHoldingRegisters.into() 
        };

        let mut buf     = [0u8; 4];
        let mut encoder = Encoder::new(&mut buf);

        // TODO: handle error
        encoder.write_u16(starting_address).unwrap();
        encoder.write_u16(count).unwrap();

        // only contains registers
        let data_in = self.client.send_recv(header, encoder.data()).await?;

        if data_in.len() % 2 != 0
        {
            todo!("Expected size to be an odd number {:?}", data_in.len())
            //return Err(TransactionError::DataMalformed());
        }

        let registers: &mut [u16] = bytemuck::cast_slice_mut(data_in);

        // Modbus is big-endian, so we need to convert each u16
        for reg in registers.iter_mut() 
        {
            *reg = u16::from_be(*reg);
        }

        Ok(registers)
    }

    pub async fn write_single_holding_register(
        &mut self, 
        address: u16, 
        value:   u16
    ) -> Result<()>
    {
        use Error::*;
        use DataMalformedError::*;

        let header = Header {
            slave_id:      self.id, 
            function_code: FunctionCode::WriteSingleRegister.into() 
        };

        let mut buf     = [0u8; 4];
        let mut encoder = Encoder::new(&mut buf);

        // TODO: handle error
        encoder.write_u16(address).unwrap();
        encoder.write_u16(value).unwrap();

        // only contains registers
        let data_in = self.client.send_recv(header, encoder.data()).await?;

        if data_in != encoder.data()
        {
            return Err(DataMalformed(DataMismatch))
        }

        Ok(())
    }
}