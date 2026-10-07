use bitvec::slice::BitSlice;
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el4002::{EL4002, EL4002_PRODUCT_ID, EL4002Configuration},
    },
    init_ethercat,
    io::analog_output::AnalogVoltageOutputDevice,
};
use std::{env, time::Duration};

/// Minimal example for the EL4008 digital output (0V to 10V).
///
/// It create a stepfunction with 16 individual values.
/// Best viewed with an oscilloscope.
fn main() {
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

    println!("preop");

    // This variable "knows" how to format the Rx PDOs for the EL4002
    let mut el4008: EL4002 = EL4002::new();
    el4008.set_config(EL4002Configuration::default());

    // The configuration has to be written in PreOp, before requesting Op
    for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
        if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL4002_PRODUCT_ID {
            let results = eth_control
                .channel
                .write_configuration(
                    subdevice.device_address,
                    el4008.get_config(),
                    Duration::from_secs(10),
                )
                .expect("Failed to write configuration");
            for (sdo_index, error) in results {
                if let Some(error) = error {
                    panic!("Failed to write {:?}: {:?}", sdo_index, error);
                }
            }
        }
    }

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

    println!("op");

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();

    for iter in 0.. {
        // Tick our application.
        // Here, we just set output.
        for port in 0..el4008.get_port_count() {
            let jumps = 16;
            let value = (iter % jumps) as f64 / (jumps as f64);
            el4008.set_voltage_relative(port, value);
        }
        println!("hello");

        // We ONLY have outputs so no need to call get_inputs
        if let Some(outputs) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                // Loop over the subdevices until the EL4008 is found
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL4002_PRODUCT_ID
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
        eth_handle.send_outputs();
        std::thread::sleep(Duration::from_millis(50));
    }
}
