pub mod devices;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio_modbus::{
    Request, Response, Slave,
    client::{rtu, tcp},
};
use tokio_serial::{SerialPortBuilder, SerialStream};
struct DataBits(tokio_serial::DataBits);
struct StopBits(tokio_serial::StopBits);

pub type ModbusRequest = Request<'static>;
pub type ModbusResponse = Response;
pub type ClientContext = tokio_modbus::client::Context;

pub struct SerialDeviceManager {
    pub port_contexts: HashMap<String, Mutex<ClientContext>>,
    // keep track of devices
    pub devices: HashMap<String, SerialDeviceMeta>,
}

pub struct ModbusSettings {
    pub baudrate: u32,
    pub bits: u8,
    pub stop_bits: u8,
    pub parity: Parity,
    pub modbus_type: ModbusType,
}

/*
    This trait is suited to applications of Modbus where you
    continually poll the device with the same request over and over
*/
pub trait ModbusDevice {
    fn new(
        path: String,
        slave_id: u8,
        settings: Option<ModbusSettings>,
    ) -> Result<Self, anyhow::Error>
    where
        Self: Sized;
    // When send_request fails device is dead and should be dropped
    // Prepare your request to be sent with whatever logic you wish
    fn send_next_request(&mut self) -> Result<(), anyhow::Error>;
    fn handle_response(&mut self) -> Result<(), anyhow::Error>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub fn create_modbus_device_context(
    dev: &SerialDeviceMeta,
) -> Result<ClientContext, anyhow::Error> {
    let device_builder = create_modbus_builder(dev)?;
    let port = SerialStream::open(&device_builder).unwrap();
    let slave = Slave(dev.slave_id);
    match dev.modbus_type {
        ModbusType::Rtu => Ok(rtu::attach_slave(port, slave)),
        ModbusType::Tcp => Ok(tcp::attach_slave(port, slave)),
    }
}

fn create_modbus_builder(dev: &SerialDeviceMeta) -> Result<SerialPortBuilder, anyhow::Error> {
    let data_bits: DataBits = dev.bits.try_into()?;
    let stop_bits: StopBits = dev.stop_bits.try_into()?;
    Ok(tokio_serial::new(dev.path.clone(), dev.baudrate)
        .flow_control(tokio_serial::FlowControl::None)
        .data_bits(data_bits.0)
        .stop_bits(stop_bits.0)
        .parity(dev.parity.clone().into()))
}

impl TryInto<DataBits> for u8 {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<DataBits, Self::Error> {
        match self {
            5 => Ok(DataBits {
                0: tokio_serial::DataBits::Five,
            }),
            6 => Ok(DataBits {
                0: tokio_serial::DataBits::Six,
            }),
            7 => Ok(DataBits {
                0: tokio_serial::DataBits::Seven,
            }),
            8 => Ok(DataBits {
                0: tokio_serial::DataBits::Eight,
            }),
            _ => Err(anyhow::anyhow!("DataBits specified not Valid".to_owned())),
        }
    }
}

impl TryInto<StopBits> for u8 {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<StopBits, Self::Error> {
        match self {
            1 => Ok(StopBits {
                0: tokio_serial::StopBits::One,
            }),
            2 => Ok(StopBits {
                0: tokio_serial::StopBits::Two,
            }),
            _ => Err(anyhow::anyhow!("StopBits specified not Valid".to_owned())),
        }
    }
}

impl Into<tokio_serial::Parity> for Parity {
    fn into(self) -> tokio_serial::Parity {
        match self {
            Parity::None => tokio_serial::Parity::None,
            Parity::Even => tokio_serial::Parity::Even,
            Parity::Odd => tokio_serial::Parity::Odd,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModbusType {
    Rtu,
    Tcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SerialDeviceMeta {
    pub path: String, // can be IP or Fs Path
    pub device_name: Option<String>,
    pub slave_id: u8,
    pub baudrate: u32,
    pub bits: u8,
    pub stop_bits: u8,
    pub parity: Parity,
    pub modbus_type: ModbusType,
}
