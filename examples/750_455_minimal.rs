use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EtherCATState, MetaSubdevice, init_ethercat, io::analog_input::AnalogCurrentInputDevice,
};
use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice, wago_modules::{ec_box::{convert_module_to_meta_ethercat_device, get_modules, init_slot_modules}, wago_750_354::Wago750_354, wago_750_455::Wago750_455},
};
use qitech_lib_units::electric_current::milliampere;
use std::{env, process::exit, time::Duration};

const STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    // Transition to PreOp
    eth_control
        .change_state(EtherCATState::PreOp, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into PreOp");
    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    
    // Find the Wago 750-354 coupler subdevice
    let coupler_subdev = subdevices
        .iter()
        .find(|s| Wago750_354::IDENTITY.matches(s))
        .expect("No Wago 750-354 coupler found on the bus");
    let modules = get_modules(eth_control.get_channel(),coupler_subdev.device_address,coupler_subdev.vendor).unwrap();            
    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP");
    
    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    // Find the Wago 750-354 coupler subdevice
    let coupler_subdev = subdevices
        .iter()
        .find(|s| Wago750_354::IDENTITY.matches(s))
        .expect("No Wago 750-354 coupler found on the bus");        
    
    let mut subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    for module in &modules {
        let device = convert_module_to_meta_ethercat_device(module,coupler_subdev.start_tx,coupler_subdev.start_rx);
        subdevices.push(device);
    }    
    for sdev in &subdevices {
        println!(" - {:?}", sdev);
    }
    exit(1);
    
    /*// Main loop: ramp AO 1 up and AO 2 down over 16 steps, read back via the 455
    loop {
        // Write outputs to the EtherCAT bus
        if let Some(outputs) = eth_control.write_outputs() {
            let subdevice_outputs = &mut outputs[coupler_subdev.start_rx..coupler_subdev.end_rx];
            coupler
                .output(BitSlice::<u8, Lsb0>::from_slice_mut(subdevice_outputs))
                .expect("Failed to write Rx PDO");
        }
        eth_control.send_outputs();

        // Let the DAC and ADC settle
        std::thread::sleep(Duration::from_millis(250));

        // Read inputs from the EtherCAT bus
        if let Some(inputs) = eth_control.get_inputs() {
            let subdevice_inputs = &inputs[coupler_subdev.start_tx..coupler_subdev.end_tx];
            coupler
                .input(BitSlice::<u8, Lsb0>::from_slice(subdevice_inputs))
                .expect("Failed to read Tx PDO");
        }

        // Read back the measured current from the 455
        /*let slot_dev = coupler.slot_devices[ai_slot]
            .as_ref()
            .expect("No device in AI slot");
        let ai = slot_dev
            .as_any()
            .downcast_ref::<Wago750_455>()
            .expect("AI slot is not a Wago 750-455");
*/
  /*      if let Some(current) = ai.get_current(0) {
            println!("AI {:.2} mA", current.get::<milliampere>());
        } else {
            println!("Wirering Error");
        }*/
    }*/
}
