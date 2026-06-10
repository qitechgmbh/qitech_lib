use ethercat_hal::{ EtherCATState, init_ethercat, };
use std::{env, time::Duration};

/// This example connect to the EtherCAT hardware and list the found devices
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");

    let eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    std::thread::sleep(Duration::from_secs(1));

    if !matches!(eth_control.controller.state, EtherCATState::PreOp) {
        panic!("Not yet in Pre Op!");
    }

    println!("Subdevices:");
    for subdevice in eth_control.controller.get_subdevices() {
        println!("  - {}", subdevice.get_name().expect("No subdevice name"));
    }


    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    std::thread::sleep(Duration::from_secs(1));

    if !matches!(eth_control.controller.state, EtherCATState::Op) {
        panic!("Not yet in Op, maybe a device needs DC Sync");
    }

    std::thread::sleep(Duration::from_secs(1));
}
