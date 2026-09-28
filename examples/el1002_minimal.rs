use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el1002::{EL1002, EL1002_PRODUCT_ID},
    },
    init_ethercat,
    io::digital_input::DigitalInputDevice,
};
use std::{env, time::Duration};

/// This example showcases a very bare bones application to read the LEDs on an EL1002
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
        println!(" - {}", sdev.get_name().expect("No utf8 name!"));
    }

    // This variable "knows" how to format the Rx PDOs for the EL2004
    let mut el1002 = EL1002::new();
    loop {
        // We ONLY have inputs so no need to call write_outputs
        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                // Loop over the subdevices until the EL2004 is found
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL1002_PRODUCT_ID
                {
                    // Get the part of the Rx PDO that is used by the EL2004
                    let subdevice_inputs = &inputs[subdevice.start_tx..subdevice.end_tx];

                    // Create the actual Rx PDO and put it into the output
                    el1002
                        .input(BitSlice::<u8, Lsb0>::from_slice(subdevice_inputs))
                        .expect("Failed to read Tx PDO");
                }
            }
        }

        // Tick our application.
        // Here, we just alternate which LED is on
        print!("[");

        for i in 0..el1002.get_port_count() {
            let input = if el1002
                .get_input(i)
                .expect("Failed to read input from subdevice!")
            {
                "X"
            } else {
                "."
            };

            print!("{}", input)
        }
        println!("]");

        // Send the output through the EtherCAT terminals
        eth_handle.send_outputs();
        std::thread::sleep(Duration::from_millis(50));
    }
}
