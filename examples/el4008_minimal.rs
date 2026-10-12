use bitvec::slice::BitSlice;
use qitech_lib_ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el4008::{EL4008, EL4008_PRODUCT_ID},
    },
    init_ethercat,
    io::analog_output::AnalogVoltageOutputDevice,
};
use std::{env, time::Duration};

const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimal example for the EL4008 digital output (0V to 10V).
///
/// It create a stepfunction with 16 individual values.
/// Best viewed with an oscilloscope.
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .change_state(EtherCATState::PreOp, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into PreOp");

    println!("preop");

    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP");

    println!("op");

    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();

    // This variable "knows" how to format the Rx PDOs for the EL4008
    let mut el4008: EL4008 = EL4008::new();
    for iter in 0.. {
        // Tick our application.
        // Here, we just set output.
        for port in 0..el4008.get_port_count() {
            let jumps = 16;
            let value = (iter % jumps) as f64 / (jumps as f64);
            el4008.set_voltage_relative(port, value);
        }

        // We ONLY have outputs so no need to call get_inputs
        if let Some(outputs) = eth_control.write_outputs() {
            for subdevice in &subdevices {
                // Loop over the subdevices until the EL4008 is found
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL4008_PRODUCT_ID
                {
                    // Get the part of the Rx PDO that is used by the EL4008
                    let subdevice_outputs = &mut outputs[subdevice.start_rx..subdevice.end_rx];

                    // Create the actual Rx PDO and put it into the output
                    el4008
                        .output(BitSlice::from_slice_mut(subdevice_outputs))
                        .expect("Failed to write Rx PDO");
                }
            }
        }

        // Send the output through the EtherCAT terminals
        eth_control.send_outputs();
        std::thread::sleep(Duration::from_millis(50));
    }
}
