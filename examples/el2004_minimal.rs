use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el2004::{EL2004, EL2004_PRODUCT_ID},
    },
    init_ethercat,
    io::digital_output::DigitalOutputDevice,
};
use std::{env, time::Duration};

/// This example showcases a very bare bones application to toggle the LEDs on an EL2004
fn main() {
    // Initialize EtherCAT Master with the default configuration
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);
    let mut eth_handle = eth_control.app_handle;

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    // Wait for state change
    loop {
        let val = eth_handle.get_state();
        match val {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    println!(
        "found {:?} ethercat terminals: ",
        eth_handle.get_subdevice_count()
    );

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to go into OP");

    // Wait for state change
    loop {
        match eth_handle.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    for sdev in &subdevices {
        println!(" - {:?}", sdev);
    }

    // This variable "knows" how to format the Rx PDOs for the EL2004
    let mut el2004: EL2004 = EL2004::new();
    for iter in 0.. {
        // Tick our application.
        // Here, we just alternate which LED is on
        for i in 0..el2004.get_port_count() {
            el2004.set_output(i, iter % el2004.get_port_count() == i);
        }

        // We ONLY have outputs so no need to call get_inputs
        if let Some(outputs) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                // Loop over the subdevices until the EL2004 is found
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL2004_PRODUCT_ID
                {
                    // Get the part of the Rx PDO that is used by the EL2004
                    let subdevice_outputs = &mut outputs[subdevice.start_rx..subdevice.end_rx];

                    // Create the actual Rx PDO and put it into the output
                    el2004
                        .output(BitSlice::<u8, Lsb0>::from_slice_mut(subdevice_outputs))
                        .expect("Failed to write Rx PDO");
                }
            }
        }

        // Send the output through the EtherCAT terminals
        eth_handle.send_outputs();
        std::thread::sleep(Duration::from_millis(50));
    }
}
