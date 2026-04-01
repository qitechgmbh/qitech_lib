use std::{env, time::Duration};
use ethercat_hal::{EtherCATState, start_ethercat_thread};

fn main()
{ 	
	let args : Vec<String> = env::args().into_iter().collect();	
	let ethercat_control = start_ethercat_thread(args.get(1).unwrap());	
	// Used for configuration
	let ethercat_interface = ethercat_control.channel;
	let ethercat_controller  = ethercat_control.controller;

    let _res = ethercat_interface.request_state_change(EtherCATState::PreOp);
    std::thread::sleep(Duration::from_millis(1000));

    for i in 0..ethercat_controller.subdevice_count {
    	println!("{:?}",ethercat_controller.subdevices[i].get_name());
    }
}