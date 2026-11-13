use bitvec::{order::Lsb0, view::BitView};
use qitech_lib_ethercat_hal::{
    EtherCATState, init_ethercat, io::digital_input::DigitalInputDevice,
};
use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice, beckhoff_modules::el1002::EL1002,
};
use std::{env, time::Duration};
const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);
/// This example showcases a very bare bones application to read the LEDs on an EL1002

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
        println!(" - {}", sdev.get_name().expect("No utf8 name!"));
    }

    // This variable "knows" how to format the Rx PDOs for the EL2004
    let mut el1002 = EL1002::new();
    loop {
        // The controller reports every failed cycle in OP. A tolerated failure only means
        // that cycle exchanged no data, a fatal one means the controller has stopped.
        while let Some(err) = eth_control.try_recv_op_error() {
            eprintln!(
                "cycle {} failed ({} in a row): {:?}",
                err.cycle, err.consecutive_failures, err.failure
            );
            if err.fatal {
                eprintln!("EtherCAT controller stopped");
                return;
            }
        }

        // We ONLY have inputs so no need to call write_outputs
        if let Some(inputs) = eth_control.get_inputs() {
            for subdevice in &subdevices {
                // Loop over the subdevices until the EL2004 is found
                if EL1002::identity().matches(subdevice) {
                    // Get the part of the Rx PDO that is used by the EL2004
                    let subdevice_inputs =
                        &inputs.view_bits::<Lsb0>()[subdevice.start_tx..subdevice.end_tx];
                    // Create the actual Rx PDO and put it into the output
                    el1002
                        .input(subdevice_inputs)
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
        eth_control.send_outputs();
        std::thread::sleep(Duration::from_millis(50));
    }
}
