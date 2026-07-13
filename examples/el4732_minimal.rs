use bitvec::slice::BitSlice;
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig,
    devices::{
        EthercatDevice, EthercatDeviceProcessing,
        el4732::{EL4732, EL4732_PRODUCT_ID, EL4732Port, EL4732RxPdo},
    },
    init_ethercat,
    io::analog_output::{AnalogOutputDevice, AnalogOutputOutput},
    pdo::oversampling::OVERSAMPLE_FACTOR,
    set_current_thread_rt_priority,
};
use std::{env, f64::consts::PI, time::Duration};

const CYCLE_TIME_US: u64 = 1000;
const OVERSAMPLE: usize = OVERSAMPLE_FACTOR;
const SYNC0_PERIOD_US: u64 = CYCLE_TIME_US / OVERSAMPLE as u64;
const SYNC1_PERIOD_US: u64 = SYNC0_PERIOD_US * (OVERSAMPLE as u64-1);
const SINE_FREQ_HZ: f64 = 50.0;
const AMPLITUDE: f64 = 0.4;

fn apply_rt() {
    let id = core_affinity::CoreId { id: 2 };
    set_current_thread_rt_priority(99);
    core_affinity::set_for_current(id);
}

fn main() {
    let interface = env::args().nth(1).expect("No interface name given");
    let mut el4732 = EL4732::new_with_oversample(OVERSAMPLE);

    let cycle_secs = CYCLE_TIME_US as f64 * 1e-6;
    let phase_step_per_slot = 2.0 * PI * SINE_FREQ_HZ * cycle_secs / OVERSAMPLE as f64;
    let phase_step_per_cycle = 2.0 * PI * SINE_FREQ_HZ * cycle_secs;
    let mut phase: f64 = 0.0;

    let mut samples_ch1 = [0.0f32; OVERSAMPLE_FACTOR];
    let samples_ch2 = [0.0f32; OVERSAMPLE_FACTOR];

    let mut dc_config = DcConfiguration::default();
    dc_config.start_delay = Duration::from_millis(100);
    dc_config.sync0_period = Duration::from_micros(SYNC0_PERIOD_US);
    dc_config.sync0_shift = Duration::from_micros(SYNC0_PERIOD_US/2);
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
        target_cycle_time_us: CYCLE_TIME_US as usize,
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
            if OVERSAMPLE > 1 {
                eth_control
                    .channel
                    .configure_oversampling(subdevice.device_address)
                    .expect("Failed to configure oversampling");

                el4732.rxpdo = EL4732RxPdo::new(OVERSAMPLE);
            }

            let s1 = Duration::from_micros(SYNC1_PERIOD_US);
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

    std::thread::sleep(Duration::from_millis(2000));

    loop {
        if eth_handle.check_all_op() {
            break;
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    println!(
        "EL4732 running: {}Hz sine, oversample factor {}, cycle {}us, sync0 {}us, sync1 {}us",
        SINE_FREQ_HZ, OVERSAMPLE, CYCLE_TIME_US, SYNC0_PERIOD_US, SYNC1_PERIOD_US
    );

    let mut last_cycle = eth_handle.get_current_cycle();
    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    apply_rt();


    loop {

        {
            let output = loop {
                if let Some(out) = eth_handle.write_outputs() {
                    break out;
                }
            };


       for (i, slot) in samples_ch1.iter_mut().enumerate() {
            let p = phase + phase_step_per_slot * i as f64;
            *slot = (p.sin() * AMPLITUDE).clamp(-1.0, 1.0) as f32;
        }


            if OVERSAMPLE > 1 {
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
	if cycles_elapsed >  1  {
		println!("OMG WTF");
	}
        last_cycle = current_cycle;

        phase = (phase + phase_step_per_cycle * cycles_elapsed as f64) % (2.0 * PI);
//        println!("{} {:?}",phase,samples_ch1);
        eth_handle.send_outputs();
    }
}
