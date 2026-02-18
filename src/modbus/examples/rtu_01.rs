use std::time::Duration;

use modbus::rtu::{ RtuClient, RtuClientConfig };
use tokio_serial::{ DataBits, FlowControl, Parity, StopBits };

use modbus::Result;

#[tokio::main]
async fn main() -> Result<()>
{
    let config = RtuClientConfig { 
        path:         "/dev/ttyUSB0", 
        baud_rate:    9600, 
        data_bits:    DataBits::Eight, 
        parity:       Parity::None, 
        flow_control: FlowControl::None, 
        stop_bits:    StopBits::One,

        timeout: Duration::from_millis(500),
    };

    let mut client = RtuClient::new(config).expect("Failed to create client");
    let mut dev    = client.device(1);

    // write 0
    dev.write_single_holding_register(0x2, 0).await?;

    // read 0
    let res = dev.read_holding_registers(0x2, 1).await?;
    assert!(res[0] == 0);

    // write 500
    dev.write_single_holding_register(0x2, 500).await?;

    // read 500
    let res = dev.read_holding_registers(0x2, 1).await?;

    assert!(res[0] == 500);

    Ok(())
}