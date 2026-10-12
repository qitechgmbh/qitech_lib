use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EtherCATState,
    devices::{
        EthercatDevice, IdentifiedDevice, NewEthercatDevice, beckhoff_modules::el2004::EL2004,
    },
    init_ethercat,
    io::digital_output::DigitalOutputDevice,
};
use std::{env, time::Duration};

const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);

/// This example showcases a very bare bones application to toggle the LEDs on an EL2004
fn main() {
    // Initialize EtherCAT Master with the default configuration
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .change_state(EtherCATState::PreOp, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into PreOp");

    println!(
        "found {:?} ethercat terminals: ",
        eth_control.get_subdevice_count()
    );

    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP");

    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
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
        if let Some(outputs) = eth_control.write_outputs() {
            for subdevice in &subdevices {
                // Loop over the subdevices until the EL2004 is found
                if EL2004::IDENTITY.matches(subdevice) {
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
        eth_control.send_outputs();
        std::thread::sleep(Duration::from_millis(50));
    }
}
