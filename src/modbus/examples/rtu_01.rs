use std::time::Duration;

use modbus::rtu::{ RtuClient, RtuClientConfig };
use tokio_serial::{ DataBits, FlowControl, Parity, StopBits };

#[tokio::main]
async fn main()
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

    let mut client = RtuClient::new(config).expect("yikes");
    let mut dev    = client.device(1);

    // write 0
    dev.write_single_holding_register(0x2, 0).await.expect("S1");

    // read 0
    let res = dev.read_holding_registers(0x2, 1).await.expect("S2");
    assert!(res[0] == 0);

    // write 500
    dev.write_single_holding_register(0x2, 500).await.expect("S3");

    // read 500
    let res = dev.read_holding_registers(0x2, 1).await.expect("S4");
    assert!(res[0] == 500);
}