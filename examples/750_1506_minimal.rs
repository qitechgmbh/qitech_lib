use bitvec::{order::Lsb0, slice::BitSlice};

use qitech_lib_ethercat_hal::{
     EtherCATState, init_ethercat, io::digital_output::DigitalOutputDevice,
};

use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice, wago_modules::{ec_box::{convert_module_to_meta_ethercat_device, get_modules}, wago_750_354::Wago750_354, wago_750_1506::{WAGO_750_1506_PRODUCT_ID, Wago750_1506}},
};

use std::{env,time::Duration};
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

    let wago750_1506_sdev = subdevices
        .iter()
        .find(|s| WAGO_750_1506_PRODUCT_ID == s.product_id)
        .expect("No WAGO_750_455 found on the bus"); 
    let mut wago750_1506 = Wago750_1506::new();
    loop {       
        if let Some(outputs) = eth_control.write_outputs() {
            let subdevice_inputs = &mut outputs[wago750_1506_sdev.start_rx..wago750_1506_sdev.end_rx];
            wago750_1506.set_output(0, true);
            wago750_1506
                .output(BitSlice::<u8, Lsb0>::from_slice_mut(subdevice_inputs))
                .expect("Failed to read Tx PDO");
        }
        eth_control.send_outputs();
    }
}