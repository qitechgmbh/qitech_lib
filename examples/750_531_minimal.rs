use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EtherCATState,
    devices::{
        EthercatDevice, IdentifiedDevice, NewEthercatDevice,
        wago_modules::{wago_750_354::Wago750_354, wago_750_531::Wago750_531},
    },
    init_ethercat,
    io::digital_output::DigitalOutputDevice,
};
use std::{env, time::Duration};

const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);

/// This example showcases a very bare bones application to toggle the outputs
/// on a Wago 750-531 (4x digital output) sitting behind a Wago 750-354 coupler.
fn main() {
    // Initialize EtherCAT Master with the default configuration
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    // Transition to PreOp
    eth_control
        .change_state(EtherCATState::PreOp, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into PreOp");

    println!(
        "Found {:?} EtherCAT terminals",
        eth_control.get_subdevice_count()
    );

    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    for sdev in &subdevices {
        println!(" - {:?}", sdev);
    }

    // Find the Wago 750-354 coupler subdevice
    let coupler_subdev = subdevices
        .iter()
        .find(|s| Wago750_354::IDENTITY.matches(s))
        .expect("No Wago 750-354 coupler found on the bus");

    // Initialize the coupler and discover its bus modules (750-531, etc.)
    let mut coupler = Wago750_354::new();
    let modules =
        Wago750_354::initialize_modules(eth_control.get_channel(), coupler_subdev.device_address)
            .expect("Failed to initialize coupler modules");
    for module in &modules {
        println!(
            "  Found module: {} (product_id: 0x{:08x})",
            module.name, module.product_id
        );
    }
    for module in modules {
        coupler.set_module(module);
    }
    coupler.init_slot_modules(eth_control.get_channel(), coupler_subdev.device_address);

    // Verify slot 0 has a device
    assert!(
        coupler.slot_devices[0].is_some(),
        "No device in slot 0 — is the 750-531 registered in init_slot_modules?"
    );

    println!("Wago 750-531 found in slot 0, starting output loop...");

    // Transition to Op
    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP");

    // Re-fetch subdevices now that we're in Op — the PDO offsets (start_rx, end_rx, etc.)
    // are only populated during the SafeOp transition, so the earlier snapshot had zeros.
    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    let coupler_subdev = subdevices
        .iter()
        .find(|s| Wago750_354::IDENTITY.matches(s))
        .expect("No Wago 750-354 coupler found on the bus");

    // Main loop: cycle through the 4 digital outputs
    let port_count = 4;
    for iter in 0.. {
        {
            let slot_dev = coupler.slot_devices[0]
                .as_mut()
                .expect("No device in slot 0");
            let wago750_531 = slot_dev
                .as_any_mut()
                .downcast_mut::<Wago750_531>()
                .expect("Slot 0 is not a Wago 750-531");
            for i in 0..port_count {
                wago750_531.set_output(i, iter % port_count == i);
            }
        }

        // Write outputs to the EtherCAT bus
        // The coupler's output() delegates to all slot devices
        if let Some(outputs) = eth_control.write_outputs() {
            let subdevice_outputs = &mut outputs[coupler_subdev.start_rx..coupler_subdev.end_rx];
            coupler
                .output(BitSlice::<u8, Lsb0>::from_slice_mut(subdevice_outputs))
                .expect("Failed to write Rx PDO");
        }

        eth_control.send_outputs();
        std::thread::sleep(Duration::from_millis(500));
    }
}
