use ethercat_hal::{
    DcConfiguration, EtherCATState, EtherCATThreadChannel, MasterConfiguration,
    RtOptimizationConfig, StdEcatHandle, init_ethercat, set_current_thread_rt_priority,
};
use std::fs::File;
use std::io::Write;
use std::{env, time::Duration};

struct Setup {
    pub total_cycles: usize,
    pub cycle_time_us: u64,
    pub ec_interface: Option<StdEcatHandle>,
    pub ec_config_interface: Option<EtherCATThreadChannel>,
}

fn setup() -> Setup {
    let interface = env::args().nth(1).expect("No Interface given");
    let mut setup: Setup = Setup {
        cycle_time_us: env::args()
            .nth(2)
            .expect("No Target Cycle time given")
            .parse()
            .expect("Target Cycle time must be a valid number"),

        total_cycles: env::args()
            .nth(3)
            .expect("No total_cycles given")
            .parse()
            .expect("total_cycles must be a valid number"),
        ec_interface: None,
        ec_config_interface: None,
    };
    let mut dc_config = DcConfiguration::default();
    dc_config.start_delay = Duration::from_millis(100);
    dc_config.sync0_period = Duration::from_micros(setup.cycle_time_us);
    dc_config.sync0_shift = Duration::from_micros(setup.cycle_time_us / 2);
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
        target_cycle_time_us: setup.cycle_time_us as usize,
        tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxIoUring,
        dc_config,
        realtime_optimizations: Some(rt),
        wkc_mismatch_threshold: 5,
        op_ramp_grace_cycles: 10000,
    };
    let ethercat_control = init_ethercat(&interface, Some(config));
    setup.ec_interface = Some(ethercat_control.app_handle);
    setup.ec_config_interface = Some(ethercat_control.channel);
    return setup;
}

// Make sure that every index is written to, so that the pages are HOT, meaning they are actually allocated in RAM
fn warm_up_memory(vec: &mut Vec<u64>) {
    for val in vec.iter_mut() {
        *val = 0;
    }
}

//  Run The Client loop with max prio, on isolated core 2
fn apply_rt() {
    let id = core_affinity::CoreId { id: 2 };
    set_current_thread_rt_priority(99);
    core_affinity::set_for_current(id);
}

fn move_to_op(ec_config_interface: &EtherCATThreadChannel, ec_app_interface: &StdEcatHandle) {
    let _res = ec_config_interface.request_state_change(EtherCATState::PreOp);
    std::thread::sleep(Duration::from_millis(5000));

    println!(
        "found {:?} ethercat terminals: ",
        ec_app_interface.get_subdevice_count()
    );

    let subdevices = ec_app_interface.try_get_subdevices_vec_sync().unwrap();
    for i in 0..ec_app_interface.get_subdevice_count() {
        println!("{:?}", subdevices[i as usize].get_name());
        let addr = subdevices[i as usize].device_address;
        if subdevices[i as usize].get_name().unwrap() != "EL4008" {
            ec_config_interface.enable_dc_sync0(addr).unwrap();
        }
    }
    let _res = ec_config_interface.request_state_change(EtherCATState::Op);
    std::thread::sleep(Duration::from_millis(5000));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup();
    let total_cycles = setup.total_cycles;
    let ec_config_interface = setup.ec_config_interface.unwrap();
    let mut ec_app_interface = setup.ec_interface.unwrap();

    // Rust is playing smart here
    // and doesnt actually touch any pages here (on linux)
    let mut cycle_times = vec![0u64; total_cycles];
    let mut jitters = vec![0u64; total_cycles];
    let mut last_cycle = 0;
    let mut cycles_recorded = 0;
    let mut missed_frames: usize = 0;

    warm_up_memory(&mut cycle_times);
    warm_up_memory(&mut jitters);
    move_to_op(&ec_config_interface, &ec_app_interface);
    //let op_subdevices = ec_app_interface.try_get_subdevices_vec_sync().unwrap();
    apply_rt();

    // Even though no frame was actually missed. For more accurate logic the counter of missed_frames
    // should be moved to the controller logic
    while cycles_recorded < total_cycles {
        while ec_app_interface.get_inputs().is_none() {}
        let _inputs = ec_app_interface.get_inputs().unwrap();
        // Do something with the inputs here
        // ...
        ec_app_interface.finish_read();

        while ec_app_interface.write_outputs().is_none() {}
        let outputs = ec_app_interface.write_outputs().unwrap();
        outputs[0] = 0;
        ec_app_interface.send_outputs();

        // Spin until io thread has advanced past our last seen cycle
        let current_controller_cycle = ec_app_interface.get_current_cycle();
        if last_cycle != 0 {
            if current_controller_cycle - last_cycle > 1 {
                missed_frames += (current_controller_cycle - last_cycle - 1) as usize;
            }
        }

        if current_controller_cycle > last_cycle {
            last_cycle = current_controller_cycle;
            let cycle_time = ec_app_interface.get_cycle_time_us();
            cycle_times[cycles_recorded] = cycle_time;
            let jitter = (cycle_time as i64 - setup.cycle_time_us as i64).abs() as u64;
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
    println!("Target Cycle Time:    {} µs", setup.cycle_time_us);
    println!("Total Cycles Run:     {}", total_cycles);
    println!("---------------------------------------------------");
    println!("Cycle Time Metrics:");
    println!("  Min:                {} µs", min_time);
    println!("  Avg:                {:.2} µs", avg_time);
    println!("  Max:                {} µs", max_time);
    println!("  Std Dev:            {:.2} µs", std_dev);
    println!("  99th Percentile:    {} µs", p99_time);
    println!("  Missing Frames:     {}", missed_frames);
    println!("---------------------------------------------------");
    println!("Jitter Metrics (Deviation from Target):");
    println!("  99th Pct Jitter:    {} µs", p99_jitter);
    println!("  Max Jitter:         {} µs", max_jitter);
    println!("===================================================");

    Ok(())
}
