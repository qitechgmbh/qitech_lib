use qitech_lib_ethercat_hal::{EtherCATState, init_ethercat};
use std::{env, time::Duration};

const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);

/// This example connect to the EtherCAT hardware and list the found devices
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);
    eth_control
        .change_state(EtherCATState::PreOp, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into PreOp");

    println!("Subdevices:");
    for subdevice in eth_control.try_get_subdevices_vec_sync().unwrap() {
        println!("  - {}", subdevice.get_name().expect("No subdevice name"));
    }

    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP, maybe a device needs DC Sync");

    std::thread::sleep(Duration::from_secs(1));
}
