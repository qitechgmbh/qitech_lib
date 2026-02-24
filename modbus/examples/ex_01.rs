use std::{cell::RefCell, rc::Rc, thread::sleep, time::Duration};

use modbus::{
    clients::example_client::ExampleClient, 
    devices::{qitech_laser::LaserDevice, us_3202510::VfdDevice}, 
    managers::{ExampleDeviceManager, example_manager::ExampleScheduler}
};


pub fn main()
{
    let (tx, rx) = ExampleClient::create_channels();

    let mgr = ExampleDeviceManager::new(tx);

    let vfd_rc: Rc<RefCell<VfdDevice<ExampleScheduler>>> = ExampleDeviceManager::register_device(mgr.clone(), 10);

    let vfd = vfd_rc.borrow_mut();

    vfd.refresh_telemetry(); // 25.0hz

    // send
    mgr.borrow_mut().update();

    sleep(Duration::from_secs(2));

    // recv
    mgr.borrow_mut().update();

    println!("telemetry: {:?}", vfd.telemetry());
}