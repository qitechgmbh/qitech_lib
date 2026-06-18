use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig, init_ethercat,
};
use std::{env, time::Duration};

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let cycle_time_us: u64 = 200;
    let mut dc_config = DcConfiguration::default();
    dc_config.start_delay = Duration::from_millis(100);
    dc_config.sync0_period = Duration::from_micros(cycle_time_us / 2);
    dc_config.sync0_shift = Duration::from_micros(cycle_time_us);
    dc_config.target_dc_tick = 100;

    let rt = RtOptimizationConfig {
        ethercat_loop_thread_core: 2,
        ethercat_loop_thread_priority: 99,
        ethercat_io_thread_core: 3,
        ethercat_io_thread_priority: 99,
        pin_irq_core: Some(3),
    };

    let config = MasterConfiguration {
        target_cycle_time_us: cycle_time_us as usize,
        tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxIoUring,
        dc_config,
        realtime_optimizations: Some(rt),
    };

    let ethercat_control = init_ethercat(&interface, Some(config));
    let ethercat_interface = ethercat_control.channel;
    let _res = ethercat_interface.request_state_change(EtherCATState::PreOp);
    std::thread::sleep(Duration::from_millis(5000));

    println!(
        "found {:?} ethercat terminals: ",
        ethercat_control.app_handle.get_subdevice_count()
    );
    let subdevices = ethercat_control
        .app_handle
        .try_get_subdevices_vec()
        .unwrap();
    for i in 0..ethercat_control.app_handle.get_subdevice_count() {
        println!("{:?}", subdevices[i as usize].get_name());
    }

    let _res = ethercat_interface.request_state_change(EtherCATState::Op);
    std::thread::sleep(Duration::from_millis(5000));

    let total_cycles: usize = 10000;
    let mut cycle_times = Vec::with_capacity(total_cycles);
    let mut jitters = Vec::with_capacity(total_cycles);
    let mut spike_count = 0;
    // missed Frames in this case DOES NOT MEAN LOST, it means your consumer was too slow to see the valid state for that cycle
    // if your application loop runs at 800us but ecat at 200us your app will almost always not see about 3 frames each iteration
    // In general the library assumes latest state wins for ease of use
    // if you ALWAYS want to see EVERY state then that is not yet supported
    // But with a sligthly rewritten ethercat_hal/controller.rs or maybe just a different Producer/Consumer it should be possible
    // Currently By Default a Triple Buffer Producer/Consumer is used
    let mut missed_frames: Vec<u64> = vec![];
    let mut last_cycle = ethercat_control.app_handle.get_current_cycle();
    let mut cycles_recorded = 0;

    while cycles_recorded < total_cycles {
        // Spin until io thread has advanced past our last seen cycle
        while last_cycle == ethercat_control.app_handle.get_current_cycle() {
            std::thread::yield_now();
        }
        let current_controller_cycle = ethercat_control.app_handle.get_current_cycle();
        if current_controller_cycle > last_cycle {
            if current_controller_cycle - last_cycle > 1 {
                for i in (last_cycle + 1)..current_controller_cycle {
                    missed_frames.push(i as u64);
                }
            }

            last_cycle = current_controller_cycle;
            let cycle_time = ethercat_control.app_handle.get_cycle_time_us();
            cycle_times.push(cycle_time);

            let jitter = (cycle_time as i64 - cycle_time_us as i64).abs() as u64;
            jitters.push(jitter);

            if cycle_time > 1000 {
                spike_count += 1;
            }

            cycles_recorded += 1;
        }
    }
    // --- STATISTICS CALCULATION ---
    cycle_times.sort_unstable();
    jitters.sort_unstable();
    let p99_index = (total_cycles * 99) / 100;
    let p99_time = cycle_times[p99_index];
    let p99_jitter = jitters[p99_index];
    let max_jitter = *jitters.iter().max().unwrap_or(&0);

    let min_time = cycle_times.iter().min().unwrap();
    let max_time = cycle_times.iter().max().unwrap();
    let sum_time: u64 = cycle_times.iter().sum::<u64>();
    let avg_time = sum_time as f64 / total_cycles as f64;

    // Calculate standard deviation
    let variance = cycle_times
        .iter()
        .map(|&val| {
            let diff = val as f64 - avg_time;
            diff * diff
        })
        .sum::<f64>()
        / total_cycles as f64;
    let std_dev = variance.sqrt();

    println!("\n================ BENCHMARK RESULTS ================");
    println!("Target Cycle Time:    {} µs", cycle_time_us);
    println!("Total Cycles Run:     {}", total_cycles);
    println!("Spikes (>1000µs):     {}", spike_count);
    println!("Cycles missed {}", missed_frames.len());
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
}
