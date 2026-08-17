use ethercat_hal::{EtherCATState, init_ethercat};
use std::{env, time::Duration};

/// This example reads all information from the devices that is available in PreOP
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);
    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    loop {
        let val = eth_control.controller.get_state();
        match val {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let idents = eth_control
        .channel
        .read_device_identifications()
        .expect("Failed to read indents!");

    for (index, subdevice) in eth_control.controller.get_subdevices().iter().enumerate() {
        let ident = idents
            .iter()
            .find(|info| info.device_address == subdevice.device_address);
        let name = subdevice.get_name().expect("Failed to read name!");

        let (trunk, branch) = if index == eth_control.controller.get_subdevice_count() - 1 {
            (" ", "└")
        } else {
            ("│", "├")
        };

        println!(
            "{}── {} at 0x{:04x}: product_id: 0x{:08x}, revision: 0x{:08x}, vendor: 0x{:08x}, start_tx: {}, end_tx: {}, start_rx: {}, end_rx: {}",
            branch,
            name,
            subdevice.device_address,
            subdevice.product_id,
            subdevice.revision,
            subdevice.vendor,
            subdevice.start_tx,
            subdevice.end_tx,
            subdevice.start_rx,
            subdevice.start_rx
        );

        if let Some(ident) = ident {
            println!(
                "{}   └──Machine Identification: machine_vendor: {}, machine_id: {}, machine_serial: {}, role: {}",
                trunk, ident.machine_vendor, ident.machine_id, ident.role, ident.machine_serial
            );
        }
    }
}
