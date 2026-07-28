use bitvec::slice::BitSlice;
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig,
    devices::{
        EthercatDevice, EthercatDeviceProcessing,
        beckhoff_modules::el4732::{EL4732, EL4732_PRODUCT_ID, EL4732Port},
    },
    init_ethercat,
    io::analog_output::{AnalogOutputDevice, AnalogOutputOutput},
    set_current_thread_rt_priority,
};
use std::{env, f64::consts::PI, time::Duration};

fn apply_rt() {
    let id = core_affinity::CoreId { id: 2 };
    set_current_thread_rt_priority(99);
    core_affinity::set_for_current(id);
}

const USAGE: &str = "el4732_minimal interface_name cycle_time_us oversampling_factor sine_freq amplitude\n example: ./target/release/examples/el4732_minimal enp4s0 1000 25 50.0 0.5";

fn main() {
    let fail = format!("{}:\n{}", "No interface name given", USAGE);
    let interface = env::args().nth(1).expect(&fail);

    let fail = format!("{}:\n{}", "No Cycle time (microseconds) given", USAGE);
    let cycle_time_us: u64 = env::args()
        .nth(2)
        .expect(&fail)
        .parse()
        .expect("cycle_time_us must be a valid u64");

    let fail = format!("{}:\n{}", "No Oversampling factor given", USAGE);
    let oversampling_factor: usize = env::args()
        .nth(3)
        .expect(&fail)
        .parse()
        .expect("oversampling_factor must be a valid usize");

    let fail = format!("{}:\n{}", "No Sinewave frequency given", USAGE);
    let sinewave_freq: f64 = env::args()
        .nth(4)
        .expect(&fail)
        .parse()
        .expect("sinewave_freq must be a valid f64");

    let fail = format!("{}:\n{}", "No Sinewave amplitude given", USAGE);
    let sinewave_amplitude: f64 = env::args()
        .nth(5)
        .expect(&fail)
        .parse()
        .expect("sinewave_amplitude must be a valid f64");

    let sync0_period_us: u64 = cycle_time_us / oversampling_factor as u64;
    let sync1_period_us: u64 = sync0_period_us * (oversampling_factor as u64 - 1);

    let mut el4732 = EL4732::new_with_oversample(oversampling_factor);
    let cycle_secs = cycle_time_us as f64 * 1e-6;
    let phase_step_per_slot = 2.0 * PI * sinewave_freq * cycle_secs / oversampling_factor as f64;
    let phase_step_per_cycle = 2.0 * PI * sinewave_freq * cycle_secs;
    let mut phase: f64 = 0.0;
    let mut samples_ch1 = vec![0.0f32; oversampling_factor];
    let samples_ch2 = vec![0.0f32; oversampling_factor];

    let mut dc_config = DcConfiguration::default();
    // Give some headroom for dc setup to finish
    dc_config.start_delay = Duration::from_millis(100);
    dc_config.sync0_period = Duration::from_micros(sync0_period_us);
    dc_config.sync0_shift = Duration::from_micros(sync0_period_us / 2);
    dc_config.target_dc_tick = 500;

    /*
        It seems like ethercat_loop_thread_core and ethercat_io_thread_core on the same core works perfectly with io_uring(linux default)
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
        wkc_mismatch_threshold: 5,
        op_ramp_grace_cycles: 10000,
    };

    let eth_control = init_ethercat(&interface, Some(config));
    let mut eth_handle = eth_control.app_handle;

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        if matches!(eth_handle.get_state(), EtherCATState::PreOp) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
        if subdevice.product_id == EL4732_PRODUCT_ID {
            if oversampling_factor > 1 {
                eth_control
                    .channel
                    .configure_oversampling(
                        subdevice.device_address,
                        el4732.configuration.oversampling_config.clone(),
                    )
                    .expect("Failed to configure oversampling");
            }

            let s1 = Duration::from_micros(sync1_period_us);
            eth_control
                .channel
                .enable_dc_sync01(subdevice.device_address, s1)
                .expect("Failed to enable DC sync01");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    std::thread::sleep(Duration::from_millis(5000));
    loop {
        if eth_handle.check_all_op() {
            break;
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    println!(
        "EL4732 running: {}Hz sine, oversample factor {}, cycle {}us, sync0 {}us, sync1 {}us",
        sinewave_freq, oversampling_factor, cycle_time_us, sync0_period_us, sync1_period_us
    );

    let mut last_cycle = eth_handle.get_current_cycle();
    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    apply_rt();
    loop {
        for (i, slot) in samples_ch1.iter_mut().enumerate() {
            let p = phase + phase_step_per_slot * i as f64;
            *slot = (p.sin() * sinewave_amplitude).clamp(-1.0, 1.0) as f32;
        }

        {
            let output = loop {
                if let Some(out) = eth_handle.write_outputs() {
                    break out;
                }
            };

            if oversampling_factor > 1 {
                el4732.set_output_samples(EL4732Port::AO1 as usize, &samples_ch1);
                el4732.set_output_samples(EL4732Port::AO2 as usize, &samples_ch2);
            } else {
                el4732.set_output(0, AnalogOutputOutput(samples_ch1[0]));
                el4732.set_output(1, AnalogOutputOutput(0.0));
            }

            for subdevice in &subdevices {
                if subdevice.product_id == EL4732_PRODUCT_ID {
                    el4732
                        .output_pre_process()
                        .expect("Failed to prepare output");
                    let out = &mut output[subdevice.start_rx..subdevice.end_rx];
                    el4732
                        .output(BitSlice::from_slice_mut(out))
                        .expect("Failed to write output");
                }
            }
        }
        let current_cycle = eth_handle.get_current_cycle();
        let cycles_elapsed = current_cycle.wrapping_sub(last_cycle);
        last_cycle = current_cycle;

        phase = (phase + phase_step_per_cycle * cycles_elapsed as f64) % (2.0 * PI);
        eth_handle.send_outputs();
    }
}
