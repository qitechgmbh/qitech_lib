use std::{cell::RefCell, rc::Rc, thread::sleep, time::Duration};

use modbus::{
    clients::example_client::ExampleClient,
    devices::qitech_laser::LaserDevice,
    managers::{ExampleDeviceManager, example_manager::ExampleScheduler},
};
use tokio_modbus::{Slave, client::rtu};
use tokio_serial::SerialStream;

#[tokio::main]
pub async fn main() {
    let (tx, rx) = ExampleClient::create_channels();

    let mgr = ExampleDeviceManager::new(tx);

    tokio::spawn(async move {
        let tty_path = "/dev/ttyUSB0";
        let slave = Slave(0x17);
        let builder = tokio_serial::new(tty_path, 38400);
        let port = SerialStream::open(&builder).unwrap();
        let ctx = rtu::attach_slave(port, slave);

        ExampleClient::run(ctx, rx).await;
    });
    let laser_device: Rc<RefCell<LaserDevice<ExampleScheduler>>> =
        ExampleDeviceManager::register_device(mgr.clone(), 1);

    loop {
        {
            let mut laser = laser_device.borrow_mut();
            laser.refresh_measurement();
        }

        // send
        mgr.borrow_mut().update();
        sleep(Duration::from_secs(1));

        // recv
        mgr.borrow_mut().update();

        {
            let laser = laser_device.borrow_mut();
            println!("laser_measurement: {:?}", laser.measurement());
        }
    }
}
