use std::time::Duration;

use common::get_async_runtime;
use modbus::{ModbusDevice, devices::us_3202510::VfdDevice};
use tokio::signal;
use tokio::time::{Interval, interval};

pub fn main() {
    let vfd_device = VfdDevice::new("/dev/ttyUSB0".to_owned(), 1, None);
    let mut vfd_device = match vfd_device {
        Ok(device) => device,
        Err(_) => return,
    };

    let mut loop_count = 0;

    while loop_count < 1000 {
        vfd_device.refresh_telemetry();
        vfd_device.send_next_request().expect("wtf");
        std::thread::sleep_ms(3);
        vfd_device.handle_response().expect("wtf2");
        {
           // println!("vfd_device telemetry: {:?}", vfd_device.telemetry());
        }
        loop_count += 1;
    }
    drop(vfd_device);
}
