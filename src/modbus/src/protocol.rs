use num_enum::{TryFromPrimitive, IntoPrimitive};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
pub enum FunctionCode 
{
    ReadCoils                      = 0x01,
    ReadDiscreteInputs             = 0x02,
    ReadHoldingRegisters           = 0x03,
    ReadInputRegisters             = 0x04,
    WriteSingleCoil                = 0x05,
    WriteSingleRegister            = 0x06,
    ReadExceptionStatus            = 0x07,
    Diagnostic                     = 0x08,
    GetCommEventCounter            = 0x0B,
    GetCommEventLog                = 0x0C,
    WriteMultipleCoils             = 0x0F,
    WriteMultipleRegisters         = 0x10,
    ReportServerID                 = 0x11,
    ReadFileRecord                 = 0x14,
    WriteFileRecord                = 0x15,
    MaskWriteRegister              = 0x16,
    ReadWriteMultipleRegisters     = 0x17,
    ReadFifoQueue                  = 0x18,

    /// Encapsulated Interface Transport (MEI)
    /// Container function code used to tunnel non-Modbus interfaces inside a Modbus PDU.
    /// The actual operation is selected by the MEI Type field in the payload
    /// (e.g. 0x0E = Read Device Identification).
    EncapsulatedInterfaceTransport = 0x2B,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsFunctionCode 
{
    ReturnQueryData                      = 0x00,
    RestartCommunicationsOption          = 0x01,
    ReturnDiagnosticRegister             = 0x02,
    ChangeASCIIInputDelimiter            = 0x03,
    ForceListenOnlyMode                  = 0x04,
    // Reserved: 0x05 - 0x0A
    ClearCountersAndDiagnosticRegister   = 0x0a,
    ReturnBusMessageCount                = 0x0b,
    ReturnBusCommunicationErrorCount     = 0x0c,
    ReturnBusExceptionErrorCount         = 0x0d,
    ReturnServerMessageCount             = 0x0e,
    ReturnServerNoResponseCount          = 0x0f,
    ReturnServerNAKCount                 = 0x10,
    ReturnServerBusyCount                = 0x11,
    ReturnBusCharacterOverrunCount       = 0x12,
    // Reserved: 0x013
    ClearOverrunCounterAndFlag           = 0x14,
    // Reserved: 0x015 - 0xffff
}

#[derive(Debug)]
pub enum ExceptionCode
{
    IllegalFunction,
    IllegalDataAddress,
    IllegalDataValue,
    SlaveDeviceFailure,
    Acknowledge,
    SlaveDeviceBusy,
    NegativeAcknowledge,
    MemoryParityError,
    GatewayPathUnavailable,
    GatewayTargetDeviceFailedToRespond,
    Unknown(u8),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncapsulatedMEIType
{
    /// CANopen General Reference (reserved / not defined in this spec)
    CANopenGeneralReference = 0x0D,

    /// Read Device Identification
    ReadDeviceIdentification = 0x0E,

    Unknown = 0xFF,
}

impl From<u8> for ExceptionCode 
{
    fn from(code: u8) -> Self 
    {
        match code 
        {
            0x01 => ExceptionCode::IllegalFunction,
            0x02 => ExceptionCode::IllegalDataAddress,
            0x03 => ExceptionCode::IllegalDataValue,
            0x04 => ExceptionCode::SlaveDeviceFailure,
            0x05 => ExceptionCode::Acknowledge,
            0x06 => ExceptionCode::SlaveDeviceBusy,
            0x07 => ExceptionCode::NegativeAcknowledge,
            0x08 => ExceptionCode::MemoryParityError,
            0x0A => ExceptionCode::GatewayPathUnavailable,
            0x0B => ExceptionCode::GatewayTargetDeviceFailedToRespond,
            other => ExceptionCode::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Header
{
    pub slave_id:      u8,
    pub function_code: u8,
}