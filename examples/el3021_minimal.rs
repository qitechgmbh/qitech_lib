use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EtherCATState, init_ethercat,
    io::analog_input::AnalogCurrentInputDevice,
};
use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice, beckhoff_modules::el3021::EL3021,
};
use qitech_lib_units::electric_current::milliampere;
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

    eth_control
        .change_state(EtherCATState::Op, STATE_CHANGE_TIMEOUT)
        .expect("Failed to go into OP");

    let subdevices = eth_control.try_get_subdevices_vec_sync().unwrap();
    let subdevice = subdevices.iter().find(|sub| EL3021::IDENTITY.matches(sub)).expect("No EL3021 found!");

    let mut el3021 = EL3021::new();

    loop {
        if let Some(inputs) = eth_control.get_inputs() {
            let subdevice_inputs = &inputs[subdevice.start_tx..subdevice.end_tx];
            el3021
                .input(BitSlice::<u8, Lsb0>::from_slice(subdevice_inputs))
                .expect("Failed to read Tx PDO");
        }
        eth_control.finish_read();

        match el3021.get_current(0) {
            Ok(current) => println!("AI {:.2} mA", current.get::<milliampere>()),
            Err(err) => eprintln!("Device Error: {err}"),
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
