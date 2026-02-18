use crate::{
    DataMalformedError, Error, protocol::{ExceptionCode, FunctionCode, Header}, rtu::{ reader }
};

pub fn create_frame<'a>(header: &Header, data: &[u8], buf: &'a mut [u8; 256]) -> &'a [u8]
{
    buf[0] = header.slave_id;
    buf[1] = header.function_code;
    buf[2..2 + data.len()].copy_from_slice(data);

    let crc = compute_crc(&buf[..data.len() + 2]);
    buf[2 + data.len() + 0] = crc[0];
    buf[2 + data.len() + 1] = crc[1];

    return &buf[..2 + data.len() + 2];
}

pub async fn get_header<'a>(reader: &mut reader::Reader<'a>) -> Result<Header, Error> 
{
    let data = reader.read_slice(2).await?;

    Ok(Header { 
        slave_id:      data[0], 
        function_code: data[1],
    })
}

pub async fn validate_crc<'a>(reader: &mut reader::Reader<'a>) -> Result<(), Error> 
{
    use Error::DataMalformed;
    use DataMalformedError::CrcMismatch;

    // first compute
    let crc_computed = compute_crc(reader.slice());
    let crc_received = reader.read_array::<2>().await?.clone();

    println!("crc: {:?} | {:?}", crc_computed, crc_received, );

    match crc_received == crc_computed 
    {
        true  => Ok(()),
        false => Err(DataMalformed(CrcMismatch)),
    }
}

pub fn validate_header(
    rq_header: &Header, 
    rsp_header: &Header
) -> Result<(), Error> 
{
    use Error::DataMalformed;
    use DataMalformedError::*;

    // ensure slave ids match between request and response
    if rsp_header.slave_id != rq_header.slave_id
    {
        return Err(DataMalformed(UnexpectedSlaveId(rsp_header.slave_id)));
    }

    // ensure function codes match between request and response
    if (rsp_header.function_code & 0x7F) != rq_header.function_code
    {
        return Err(DataMalformed(UnexpectedFunctionCode(rsp_header.function_code)));
    }

    Ok(())
}

pub async fn check_exception<'a>(
    reader: &mut reader::Reader<'a>,
    function_code: u8,
) -> Result<(), Error> 
{
    use Error::Exception;

    match (function_code & 0x80) != 0 
    {
        true => 
        {
            let data = reader.read_slice(1).await?;
            let code = ExceptionCode::from(data[0]);
            Err(Exception(code))
        },
        false => Ok(()),
    }
}

pub async fn create_data<'a, 'b>(
    reader: &mut reader::Reader<'a>, 
    data_size: usize,
    buf: &'b mut [u8]
) -> Result<&'b mut [u8], Error>
{
    let data: &mut [u8] = reader.read_slice(data_size).await?;
    buf[..data.len()].copy_from_slice(&data);
    Ok(&mut buf[..data_size])
}

pub async fn get_data_size<'a>(
    reader: &mut reader::Reader<'a>,
    function_code: u8
) -> Result<usize, Error>
{
    use FunctionCode::*;

    let function_code = FunctionCode::try_from(function_code)
        .map_err(|_| Error::IllegalState)?;

    match function_code
    {
        ReadCoils                      => todo!(),
        ReadDiscreteInputs             => todo!(),
        ReadHoldingRegisters           => read_holding_registers(reader).await,
        ReadInputRegisters             => todo!(),
        WriteSingleCoil                => todo!(),
        WriteSingleRegister            => Ok(4),
        ReadExceptionStatus            => todo!(),
        Diagnostic                     => todo!(),
        GetCommEventCounter            => todo!(),
        GetCommEventLog                => todo!(),
        WriteMultipleCoils             => todo!(),
        WriteMultipleRegisters         => todo!(),
        ReportServerID                 => todo!(),
        ReadFileRecord                 => todo!(),
        WriteFileRecord                => todo!(),
        MaskWriteRegister              => todo!(),
        ReadWriteMultipleRegisters     => todo!(),
        ReadFifoQueue                  => todo!(),
        EncapsulatedInterfaceTransport => todo!(),
    }
}

pub async fn read_holding_registers<'a>(
    reader: & mut reader::Reader<'a>
) -> Result<usize, Error>
{
    Ok(reader.read_byte().await? as usize)
}

pub fn compute_crc(data: &[u8]) -> [u8; 2] 
{
    let mut crc: u16 = 0xFFFF;

    for &byte in data 
    {
        crc ^= byte as u16;
        
        for _ in 0..8 
        {
            if crc & 0x0001 != 0 
            {
                crc = (crc >> 1) ^ 0xA001;
            } 
            
            else 
            {
                crc >>= 1;
            }
        }
    }

    // Return as low byte first, high byte second
    [(crc & 0xFF) as u8, (crc >> 8) as u8]
}