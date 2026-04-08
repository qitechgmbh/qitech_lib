use ethercat_hal::{EtherCATState, start_ethercat_thread};
use std::{env, time::Duration};

fn main() {
    let args: Vec<String> = env::args().into_iter().collect();
    let ethercat_control = start_ethercat_thread(args.get(1).expect("No Interface-name given"));

    // Used for configuration
    let ethercat_interface = ethercat_control.channel;
    // Holds metadata about devices and cycle time
    let ethercat_controller = ethercat_control.controller;

    let _res = ethercat_interface.request_state_change(EtherCATState::PreOp);
    std::thread::sleep(Duration::from_millis(5000));
    println!("found {:?} ethercat terminals",ethercat_controller.subdevice_count);
    for i in 0..ethercat_controller.subdevice_count {
        println!("{:?}", ethercat_controller.subdevices[i].get_name());
    }
}
