use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EtherCATState, init_ethercat, io::analog_input::AnalogCurrentInputDevice,
};
use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice, wago_modules::{ec_box::{convert_module_to_meta_ethercat_device}, wago_750_354::Wago750_354, wago_750_455::{WAGO_750_455_PRODUCT_ID, Wago750_455}},
};
use qitech_lib_units::electric_current::milliampere;
use std::{env, time::Duration};

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
    let wago750_455_sdev = subdevices
        .iter()
        .find(|s| WAGO_750_455_PRODUCT_ID == s.product_id)
        .expect("No WAGO_750_455 found on the bus"); 
    let mut wago750_455 = Wago750_455::new();

    loop {
        std::thread::sleep(Duration::from_millis(250));
        // Read inputs from the EtherCAT bus
        if let Some(inputs) = eth_control.get_inputs() {
            let subdevice_inputs = &inputs[wago750_455_sdev.start_tx..wago750_455_sdev.end_tx];
            wago750_455
                .input(BitSlice::<u8, Lsb0>::from_slice(subdevice_inputs))
                .expect("Failed to read Tx PDO");
        }
        if let Some(current) = wago750_455.get_current(0) {
            println!("AI {:.2} mA", current.get::<milliampere>());
        } else {
            println!("Wirering Error");
        }
        eth_control.finish_read();
    }
}