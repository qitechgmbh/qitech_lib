use ethercat_hal::{EtherCATState, init_ethercat};
use std::{env, time::Duration};

/// This example connect to the EtherCAT hardware and list the found devices
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);
    let eth_handle = eth_control.app_handle;
    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    std::thread::sleep(Duration::from_secs(1));

    if !matches!(eth_handle.get_state(), EtherCATState::PreOp) {
        panic!("Not yet in Pre Op!");
    }

    println!("Subdevices:");
    for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
        println!("  - {}", subdevice.get_name().expect("No subdevice name"));
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    std::thread::sleep(Duration::from_secs(1));

    if !matches!(eth_handle.get_state(), EtherCATState::Op) {
        panic!("Not yet in Op, maybe a device needs DC Sync");
    }

    std::thread::sleep(Duration::from_secs(1));
}
