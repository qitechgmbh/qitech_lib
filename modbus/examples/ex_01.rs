use std::{cell::RefCell, rc::Rc, thread::sleep, time::Duration};

use modbus::{
    clients::example_client::ExampleClient, 
    devices::{ us_3202510::VfdDevice }, 
    managers::{ExampleDeviceManager, example_manager::ExampleScheduler}
};
use tokio_modbus::{Slave, client::rtu};
use tokio_serial::SerialStream;


#[tokio::main]
pub async fn main()
{
    let (tx, rx) = ExampleClient::create_channels();

    tokio::spawn(async move 
    {
        let tty_path = "/dev/ttyUSB0";
        let slave = Slave(0x17);

        let builder = tokio_serial::new(tty_path, 19200);

        let port = SerialStream::open(&builder).unwrap();
        let ctx = rtu::attach_slave(port, slave);
        
        ExampleClient::run(ctx, rx).await;
    });

    let mgr = ExampleDeviceManager::new(tx);

    let vfd_rc: Rc<RefCell<VfdDevice<ExampleScheduler>>> = ExampleDeviceManager::register_device(mgr.clone(), 1);

    {
        let mut vfd = vfd_rc.borrow_mut();
        vfd.refresh_telemetry(); // 25.0hz
    }

    // send
    mgr.borrow_mut().update();

    sleep(Duration::from_secs(2));

    // recv
    mgr.borrow_mut().update();

    {
        let vfd = vfd_rc.borrow_mut();
        println!("telemetry: {:?}", vfd.telemetry());
    }
}