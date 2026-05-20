use tokio_serial::{SerialPortInfo, available_ports};

pub fn get_available_ports() -> Result<Vec<SerialPortInfo>,anyhow::Error>{
	Ok(available_ports()?)
}