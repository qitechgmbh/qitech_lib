/*
    Requires an EL2004 and an EL1008 or similiar
    where all 4 channels connect to the first 4 of an el1008, however with small changes an el1002,el1004 also works
*/

use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::{
            el1008::{EL1008, EL1008_PRODUCT_ID},
            el2004::{EL2004, EL2004_PRODUCT_ID},
        },
    },
    init_ethercat,
    io::{digital_input::DigitalInputDevice, digital_output::DigitalOutputDevice},
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

    // This variable "knows" how to read the Rx PDOs for the EL2004
    let mut el2004: EL2004 = EL2004::new();
    let mut el1008: EL1008 = EL1008::new();

    loop {
        // You can ignore this boolean but if you start reading while this is false you will
        // get stale data (might actually be fine depending on your use case)
        while eth_handle.check_inputs_ready() == false {}
        let inputs = eth_handle.get_inputs().unwrap();
        for subdevice in &subdevices {
            if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL1008_PRODUCT_ID {
                let input_slice = &inputs[subdevice.start_tx..subdevice.end_tx];
                el1008
                    .input(BitSlice::<u8, Lsb0>::from_slice(input_slice))
                    .expect("Failed to write Tx PDO");
            }
        }

        // Here, we just alternate which LED is on depending on the input terminal
        for i in 0..el2004.get_port_count() {
            el2004.set_output(
                i,
                !el1008.get_input(i).expect("Failed to read input channel"),
            );
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
