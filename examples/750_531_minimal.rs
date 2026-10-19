use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EtherCATState, convert_module_to_meta_ethercat_device, init_ethercat, io::digital_output::DigitalOutputDevice,
};
use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice,
    wago_modules::{        
        wago_750_354::Wago750_354,
        wago_750_531::{WAGO_750_531_PRODUCT_ID, Wago750_531},
    },
};
use std::{env, time::Duration};
const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    // Initialize the Bus State: Init
    let mut eth_control = init_ethercat(&interface, None);

    // Transition to PreOp
    eth_control
        .change_state(EtherCATState::PreOp, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into PreOp");

    // Find devices in Preop
    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    // Find the Wago 750-354 coupler subdevice
    let coupler_subdev = subdevices
        .iter()
        .find(|s| Wago750_354::IDENTITY.matches(s))
        .expect("No Wago 750-354 coupler found on the bus");

    // Devices connected to 750-354 are not actual ethercat terminals and are commonly called "modules"
    let modules = eth_control
        .get_channel()
        .get_modules(coupler_subdev.device_address, coupler_subdev.vendor, None)
        .unwrap();

    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP");

    // Find devices Again, Now they will have their tx and rx offsets
    let mut subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    // Find the Wago 750-354 couplers offsets
    let (coupler_start_tx, coupler_start_rx) = {
        let coupler_subdev = subdevices
            .iter()
            .find(|s| Wago750_354::IDENTITY.matches(s))
            .expect("No Wago 750-354 coupler found on the bus");
        (coupler_subdev.start_tx, coupler_subdev.start_rx)
    };

    // Since modules are part of the couplers PDI we need to convert the "modules" into a
    // meta ethercat device first, so they can be handled like any other terminal
    for module in &modules {
        let device =
            convert_module_to_meta_ethercat_device(module, coupler_start_tx, coupler_start_rx);
        subdevices.push(device);
    }

    let wago750_531_sdev = subdevices
        .iter()
        .find(|s| WAGO_750_531_PRODUCT_ID == s.product_id)
        .expect("No WAGO_750_531 found on the bus");

    // Cycle through the 4 digital outputs
    let port_count = 4;
    let mut wago750_531 = Wago750_531::new();
    for iter in 0.. {
        for i in 0..port_count {
            wago750_531.set_output(i, iter % port_count == i);
        }
        if let Some(outputs) = eth_control.write_outputs() {
            let subdevice_outputs =
                &mut outputs[wago750_531_sdev.start_rx..wago750_531_sdev.end_rx];
            wago750_531
                .output(BitSlice::<u8, Lsb0>::from_slice_mut(subdevice_outputs))
                .expect("Failed to write Rx PDO");
        }
        eth_control.send_outputs();
        std::thread::sleep(Duration::from_millis(500));
    }
}
