use std::time::Duration;

use common::get_async_runtime;
use modbus::{ModbusDevice, devices::qitech_laser::LaserDevice};
use tokio::signal;
use tokio::time::{Interval, interval};

pub fn main() {
    let serial_device = LaserDevice::new("/dev/ttyUSB0".to_owned(), 1, None);
    let mut serial_device = match serial_device {
        Ok(device) => device,
        Err(_) => return,
    };

    let mut loop_count = 0;

    while loop_count < 1000 {
        serial_device.send_next_request().expect("wtf");
        std::thread::sleep_ms(3);
        serial_device.handle_response().expect("wtf2");
        {
            println!("laser_measurement: {:?}", serial_device.measurement);
        }
        loop_count += 1;
    }
    drop(serial_device);
}
