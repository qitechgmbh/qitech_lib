use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig, init_ethercat,
};
use std::{env, time::Duration};
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let cycle_time_us: u64 = env::args()
    .nth(2)
    .expect("No Target Cycle time given")
    .parse()
    .expect("Target Cycle time must be a valid number");

    let total_cycles: usize = env::args()
    .nth(3)
    .expect("No total_cycles given")
    .parse()
    .expect("total_cycles must be a valid number");

    let mut dc_config = DcConfiguration::default();
    dc_config.start_delay = Duration::from_millis(100);
    dc_config.sync0_period = Duration::from_micros(cycle_time_us);
    dc_config.sync0_shift = Duration::from_micros(cycle_time_us / 2);
    dc_config.target_dc_tick = 500;

    /*
        It seems like ethercat_loop_thread_core and ethercat_io_thread_core on the same core works
        with SCHED_FIFO, however ethercat_loop_thread_priority needs to have a much lower priority, like 50 for example.
        This means that the io code will never get preempted, while the io only actually runs when triggered through the tx_rx_dc code
    */
    let rt = RtOptimizationConfig {
        ethercat_loop_thread_core: 3,
        ethercat_loop_thread_priority: 50,
        ethercat_io_thread_core: 3,
        ethercat_io_thread_priority: 99,
        pin_irq_core: Some(3),
        lock_memory: true,
    };

    let config = MasterConfiguration {
        target_cycle_time_us: cycle_time_us as usize,
        tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxIoUring,
        dc_config,
        realtime_optimizations: Some(rt),
    };

    let ethercat_control = init_ethercat(&interface, Some(config));
    let ethercat_interface = ethercat_control.channel;
    
    // Rust is playing smart here
    // and doesnt actually touch any pages here (on linux)
    let mut cycle_times = vec![0u64;total_cycles];
    let mut jitters = vec![0u64;total_cycles];
    let mut last_cycle = 0;
    let mut cycles_recorded = 0;
    
    // Make sure that every index is written to, so that the pages are HOT
    for val in cycle_times.iter_mut() {
        *val = 0;
    }

    for val in jitters.iter_mut() {
        *val = 0;
    }


    let _res = ethercat_interface.request_state_change(EtherCATState::PreOp);
    std::thread::sleep(Duration::from_millis(5000));

    println!(
        "found {:?} ethercat terminals: ",
        ethercat_control.app_handle.get_subdevice_count()
    );

    let subdevices = ethercat_control
        .app_handle
        .try_get_subdevices_vec_sync()
        .unwrap();

    for i in 0..ethercat_control.app_handle.get_subdevice_count() {
        println!("{:?}", subdevices[i as usize].get_name());
        let addr = subdevices[i as usize].device_address;
        if subdevices[i as usize].get_name().unwrap() != "EL4008" {
            ethercat_interface.enable_dc_sync0(addr).unwrap();
        }
    }

    let _res = ethercat_interface.request_state_change(EtherCATState::Op);
    std::thread::sleep(Duration::from_millis(5000));

    while cycles_recorded < total_cycles {
        // Spin until io thread has advanced past our last seen cycle
        while last_cycle == ethercat_control.app_handle.get_current_cycle() {}
        let current_controller_cycle = ethercat_control.app_handle.get_current_cycle();
        if current_controller_cycle > last_cycle {
            last_cycle = current_controller_cycle;
            let cycle_time = ethercat_control.app_handle.get_cycle_time_us();
            cycle_times[cycles_recorded] = cycle_time;
            let jitter = (cycle_time as i64 - cycle_time_us as i64).abs() as u64;
            jitters[cycles_recorded] = jitter;
            cycles_recorded += 1;
        }
    }

    std::thread::sleep(Duration::from_millis(1000));
    
    // --- STATISTICS CALCULATION ---
    let mut sorted_cycle_times = cycle_times.clone();
    sorted_cycle_times.sort_unstable();    
    
    let mut sorted_jitters = jitters.clone();
    sorted_jitters.sort_unstable();

    let p99_index = (total_cycles * 99) / 100;
    let p99_time = sorted_cycle_times[p99_index];
    let p99_jitter = sorted_jitters[p99_index];
    let max_jitter = *sorted_jitters.iter().max().unwrap_or(&0);

    let min_time = sorted_cycle_times.iter().min().unwrap();
    let max_time = sorted_cycle_times.iter().max().unwrap();
    let sum_time: u64 = sorted_cycle_times.iter().sum::<u64>();
    let avg_time = sum_time as f64 / total_cycles as f64;

    // Calculate standard deviation
    let variance = sorted_cycle_times
        .iter()
        .map(|&val| {
            let diff = val as f64 - avg_time;
            diff * diff
        })
        .sum::<f64>()
        / total_cycles as f64;
    let std_dev = variance.sqrt();

    let json_string = format!("{:?}", cycle_times);
    let mut file = File::create("/tmp/output.json")?;
    file.write_all(json_string.as_bytes())?;

    println!("\n================ BENCHMARK RESULTS ================");
    println!("Target Cycle Time:    {} µs", cycle_time_us);
    println!("Total Cycles Run:     {}", total_cycles);        
    println!("---------------------------------------------------");
    println!("Cycle Time Metrics:");
    println!("  Min:                {} µs", min_time);
    println!("  Avg:                {:.2} µs", avg_time);
    println!("  Max:                {} µs", max_time);
    println!("  Std Dev:            {:.2} µs", std_dev);
    println!("  99th Percentile:    {} µs", p99_time);
    println!("---------------------------------------------------");
    println!("Jitter Metrics (Deviation from Target):");
    println!("  99th Pct Jitter:    {} µs", p99_jitter);
    println!("  Max Jitter:         {} µs", max_jitter);
    println!("===================================================");

    Ok(())
}
