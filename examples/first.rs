use ethercat_hal::{DcConfiguration, EtherCATState, MasterConfiguration, init_ethercat};
use std::{env, time::Duration};

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut dc_config = DcConfiguration::default();
    dc_config.start_delay = Duration::from_millis(100);
    dc_config.sync0_period = Duration::from_micros(25);
    dc_config.sync0_shift = Duration::from_micros(50);
    dc_config.target_dc_tick = 100;

    let config = MasterConfiguration { target_cycle_time_us: 50, tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxIoUring, dc_config };
    let ethercat_control = init_ethercat(&interface, Some(config));

    let ethercat_interface = ethercat_control.channel;
    // Holds metadata about devices and cycle time
    let ethercat_controller = ethercat_control.controller;

    let _res = ethercat_interface.request_state_change(EtherCATState::PreOp);
    std::thread::sleep(Duration::from_millis(5000));
    println!(
        "found {:?} ethercat terminals: ",
        ethercat_controller.subdevice_count
    );

    for i in 0..ethercat_controller.subdevice_count {
        println!("{:?}", ethercat_controller.subdevices[i].get_name());
    }

    println!("Moving to OP");
    let _res = ethercat_interface.request_state_change(EtherCATState::Op);
    std::thread::sleep(Duration::from_millis(5000));

    let mut cycles : usize = 10000;
    let mut max : u64 = u64::MIN;
    let mut min : u64 = u64::MAX;
    let mut spike_count = 0;


    loop {
        std::thread::sleep(Duration::from_micros(200));
        if cycles == 0 {
            break;
        }
        cycles -= 1;
        let cycle_time = ethercat_controller.cycle_time_us;
        if cycle_time > max {
            max = cycle_time;
        }
        if cycle_time < min {
            min = cycle_time;
        }
        if cycle_time > 1000 {
            spike_count += 1;
        }
    }
    println!("cycle time spikes above 1000us {} min {} max {}", spike_count,min,max);
}
