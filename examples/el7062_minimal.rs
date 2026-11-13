/*
    EL7062 channel 1 in Cyclic Synchronous Position mode, driven as a clock:
    one 6 degree step per second, one revolution per minute. Each step prints
    the commanded position against the drive's reported actual position.

    Commutation type 17 (0x8010:64) is "stepper with encoder": the drive closes
    its position loop around the external encoder rather than its own step
    count. The error printed here is therefore where the shaft actually is, and
    a skipped step or a hand on the shaft shows up in it. Type 16 would report
    on target either way, because its counter cannot know.

    0x8008:01 (invert feedback direction) is set, because this rig's encoder
    counts the opposite way round to its motor. At type 17 that object is in
    the control path: left as the default, the loop closes with positive
    feedback and the motor runs away. At type 16 the encoder was only
    monitored, so the object had no effect on motion.

    Ctrl+C kills this process while the drive is in Op, so it loses process data,
    logs DC-Link undervoltage (0x4411) and PD-Watchdog (0x8105), and latches a
    fault. The next run prints `FAULT: ... latched by an earlier run` — that
    line is expected after an interrupt, not a new fault — and then pulses
    controlword bit 7 until the reset takes, reading the DC-link level on the
    edge. Each interrupt adds one more 0x4411/0x8105 pair to the drive's 0x10F3
    history. A graceful Op -> SafeOp transition is not possible: the master's
    state-change channel acts on NoInterface, PreOp and Op only, and drops
    every other target.
*/

use bitvec::slice::BitSlice;
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7062::{
            EL7062, EL7062_PRODUCT_ID, EL7062Port,
            coe::{Commutation, EncoderConfig, EncoderType, FollowingErrorMonitor},
        },
    },
    init_ethercat,
};
use std::{
    env,
    time::{Duration, Instant},
};

const USAGE: &str = "el7062_minimal <interface> [cycle_time_us]\n \
     example: sudo ./target/release/examples/el7062_minimal enp4s0 1000";

const CH: EL7062Port = EL7062Port::Ch1;

/// One second hand: 60 steps per revolution, so one revolution per minute.
const STEPS_PER_REV: f64 = 60.0;

/// Left spinning at the end of each cycle to catch the new process image, so
/// sleep timing jitter does not cost a cycle.
const SPIN_HEADROOM_US: u64 = 100;

/// How long to tolerate a master that has stopped producing process images
/// before giving up, rather than spinning at 100% forever.
const STALL_TIMEOUT: Duration = Duration::from_millis(500);

/// The motor this example assumes: 200 full steps/rev, 1.8 A/phase.
const MOTOR_RATED_MA: u32 = 1800;

/// Motor runs rather hot when running at the rated current of 1800 mA.
const MOTOR_CONFIGURED_MA: u32 = 100;

/// 0x8008:12 encoder type. RS422 differential, which the WEDL5541-A14 is.
/// The library default of 0 disables the encoder entirely.
const ENCODER_TYPE: EncoderType = EncoderType::Rs422Differential;

/// 0x8008:13 is the resolution AFTER 4-fold evaluation, per the Beckhoff
/// parameter documentation, so 500 CPR from the encoder data sheet is 2000
/// here. The library default of 4096 belongs to a different encoder.
const ENCODER_INCR_PER_REV: u32 = 2_000;

/// 0x8010:31 velocity limitation, in rev/min.
const VELOCITY_LIMIT_REV_PER_MIN: u32 = 300;

/// 0x8010:73 acceleration limitation, in 0.1 rad/s2, so 2000 = 200 rad/s2.
const ACCEL_LIMIT_0_1_RAD_PER_S2: u32 = 2000;

/// 0x8010:72 standstill current, in thousandths of nominal current, so 1000 is
/// the full nominal current and 0 is no holding torque. Setting it to a low
/// number so an error can be caused manually to test error correction.
const STAND_STILL_TORQUE_PERMILLE: u16 = 200;

fn main() {
    let interface = env::args().nth(1).expect(USAGE);
    let cycle_time_us: u64 = 1000;
    // get_current_cycle() counts master cycles, so one second is this many.
    let step_cycles = 1_000_000 / cycle_time_us;

    let mut el7062 = EL7062::new();
    let channel1 = &mut el7062.configuration.channel_1;
    channel1.motor.rated_current = MOTOR_RATED_MA;
    channel1.motor.configured_motor_current = MOTOR_CONFIGURED_MA;
    channel1.feedback.encoder =
        EncoderConfig::wired(ENCODER_TYPE, ENCODER_INCR_PER_REV).expect("non-zero resolution");
    channel1.feedback.invert_feedback_direction = true;

    let amp = &mut channel1.amplifier;
    amp.commutation = Commutation::StepperWithEncoder;
    amp.stand_still_torque_limitation = STAND_STILL_TORQUE_PERMILLE;
    amp.velocity_limitation = VELOCITY_LIMIT_REV_PER_MIN;
    amp.acceleration_limitation = ACCEL_LIMIT_0_1_RAD_PER_S2;
    // One full revolution of following error before the drive gives up, in the
    // same increments everything downstream uses.
    amp.following_error = FollowingErrorMonitor::enabled(
        channel1.feedback.position_scale.increments_per_revolution(),
        100,
    )
    .expect("usable following error window");

    let ch1 = &el7062.configuration.channel_1;
    println!(
        "assuming a {MOTOR_RATED_MA} mA (1.8 A/phase), {} full steps/rev motor and a 500 CPR \
         RS422 encoder. Writing rated_current={} mA, configured_motor_current={} mA, \
         motor_full_steps_per_revolution={}, encoder_type={} (1 = RS422), \
         encoder_increments_per_revolution={} (500 CPR after 4x evaluation = 2000), \
         commutation_type={}, invert_feedback_direction={}, velocity_limit={} rev/min, \
         acceleration_limit={} rad/s^2, standstill_current={}/1000 of nominal, \
         following_error_window={} increments. At commutation \
         type {} the encoder closes the position loop, so the error printed each step is where \
         the shaft really is. Check every value against your own hardware.",
        ch1.motor.motor_full_steps_per_revolution,
        ch1.motor.rated_current,
        ch1.motor.configured_motor_current,
        ch1.motor.motor_full_steps_per_revolution,
        ch1.feedback.encoder.encoder_type().as_raw(),
        ch1.feedback
            .encoder
            .increments_per_revolution()
            .unwrap_or(0),
        ch1.amplifier.commutation.as_raw(),
        ch1.feedback.invert_feedback_direction,
        ch1.amplifier.velocity_limitation,
        ch1.amplifier.acceleration_limitation as f64 / 10.0,
        ch1.amplifier.stand_still_torque_limitation,
        ch1.amplifier.following_error.window(),
        ch1.amplifier.commutation.as_raw(),
    );

    let dc_config = DcConfiguration {
        start_delay: Duration::from_millis(100),
        // Pinned, not derived: the EL7062's DC-Synchron OpMode declares a fixed
        // 62500 ns SYNC0 cycle (ESI CycleTimeSync0 Factor="0") and SafeOp rejects
        // anything else with AL status 0x0035.
        sync0_period: Duration::from_nanos(62_500),
        sync0_shift: Duration::ZERO,
        target_dc_tick: 500,
    };
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
    std::thread::sleep(Duration::from_millis(1000));

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Failed to request PreOp");
    while !matches!(eth_handle.get_state(), EtherCATState::PreOp) {
        std::thread::sleep(Duration::from_millis(100));
    }

    let addr = eth_handle
        .try_get_subdevices_vec_sync()
        .unwrap()
        .iter()
        .find(|s| s.product_id == EL7062_PRODUCT_ID)
        .map(|s| s.device_address)
        .expect("No EL7062 found on the bus");
    el7062
        .write_config(eth_control.channel.clone(), addr, &el7062.get_config())
        .expect("Failed to write EL7062 config");
    eth_control
        .channel
        .enable_dc_sync01(addr, Duration::from_micros(cycle_time_us))
        .expect("Failed to enable DC sync01");

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to request Op");
    while !eth_handle.check_all_op() {
        std::thread::sleep(Duration::from_millis(10));
    }

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();

    let incr_per_rev = el7062
        .configuration
        .channel_1
        .feedback
        .position_scale
        .increments_per_revolution() as f64;
    // Negated so the hand runs clockwise as seen from the drive face. Inverting
    // 0x8008:01 (the feedback direction) instead would also flip the sign of
    // velocity in CSV mode and of the position jog. Flip this back if the motor
    // is mounted the other way round.
    let step_incr = -((incr_per_rev / STEPS_PER_REV) as i32);
    println!(
        "EL7062 @0x{addr:04X} clock: {STEPS_PER_REV:.0} steps/rev, {step_incr} incr/step, \
         {incr_per_rev:.0} incr/rev"
    );

    let mut target = 0i32;
    let mut seeded = false;
    let mut last_step_cycle = eth_handle.get_current_cycle();
    let mut steps = 0u32;
    let mut faulted = false;
    let mut was_enabled = false;

    loop {
        // The master's I/O thread owns the timing; this thread only consumes the
        // process image, so it does not need to be real-time. Busy-waiting here
        // at hard RT priority on the master's own core starves that thread and
        // pegs a core, so sleep instead. check_inputs_ready() is a level flag,
        // so overshooting a cycle boundary just costs a little latency.
        let sleep_us = eth_handle
            .get_cycle_time_us()
            .saturating_sub(SPIN_HEADROOM_US);
        if sleep_us > 0 {
            std::thread::sleep(Duration::from_micros(sleep_us));
        }

        let stall_start = Instant::now();
        while !eth_handle.check_inputs_ready() {
            std::hint::spin_loop();
            if stall_start.elapsed() > STALL_TIMEOUT {
                panic!(
                    "no new process image for {STALL_TIMEOUT:?}: the master stopped cycling, \
                     so the drive has dropped out of Op. Check the terminal's display and the \
                     0x10F3 diagnosis history; the DC-link and follower readings this example \
                     prints on a fault edge tell the undervoltage and loss-of-step stories."
                );
            }
        }

        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    let input = &inputs[subdevice.start_tx..subdevice.end_tx];
                    el7062.input(BitSlice::from_slice(input)).unwrap();
                    el7062.input_post_process().unwrap();
                }
            }
        }

        // Walks the CiA402 state machine up to operation enabled, and resets a
        // latched fault on the way. The axis is borrowed once so the getter and
        // the setter cannot end up on different channels.
        let statusword = el7062.axis(CH).statusword().unwrap();
        el7062.axis(CH).apply_controlword(&statusword).unwrap();

        if statusword.fault && !faulted {
            // One sync SDO read on the fault edge: the DC-link level answers
            // the "why" for the most common fault here (0x4411 undervoltage
            // after an interrupted run). Other readers live on the type —
            // supply voltage, amplifier temperature, output stage state.
            let dc_link = EL7062::read_dc_link_voltage(&eth_control.channel, addr)
                .map(|mv| format!("{mv} mV"))
                .unwrap_or_else(|e| format!("unreadable: {e}"));
            println!(
                "FAULT: statusword 0x{:04X}, DC-link {dc_link}, latched by an earlier run",
                statusword.as_raw()
            );
            faulted = true;
        } else if !statusword.fault {
            faulted = false;
        }

        // Seed once, from the position the terminal actually reports, on the very
        // first cycle. This must not wait for operation_enabled: until then the
        // drive is still holding the last commanded target, and a placeholder
        // would be a jump of however many revolutions the axis happens to be
        // away from zero.
        if !seeded {
            target = el7062.axis(CH).position().unwrap();
            seeded = true;
        }

        // Start the clock only once the drive is really enabled, and give it a
        // full step period from that moment. A drive that is slow to enable
        // (after a fault reset, say) would otherwise swallow the first steps.
        let cycle = eth_handle.get_current_cycle();
        if statusword.operation_enabled && !was_enabled {
            last_step_cycle = cycle;
        }
        was_enabled = statusword.operation_enabled;

        if statusword.operation_enabled && cycle.wrapping_sub(last_step_cycle) >= step_cycles {
            last_step_cycle = cycle;
            if steps > 0 {
                // The motor has had a full step period to catch up, so this is
                // the closed-loop error of the previous step.
                let actual = el7062.axis(CH).position().unwrap();
                let err = actual - target;
                println!(
                    "step {steps:>3}: target {:.4} rev, actual {:.4} rev, error {err} incr \
                     ({:.5} rev){}",
                    target as f64 / incr_per_rev,
                    actual as f64 / incr_per_rev,
                    err as f64 / incr_per_rev,
                    if statusword.fault { "  FAULT" } else { "" }
                );
            }
            steps += 1;
            target += step_incr;
        }
        el7062.axis(CH).set_target_position(target).unwrap();

        if let Some(outputs) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    el7062.output_pre_process().unwrap();
                    let out = &mut outputs[subdevice.start_rx..subdevice.end_rx];
                    el7062.output(BitSlice::from_slice_mut(out)).unwrap();
                }
            }
        }
        eth_handle.send_outputs();
    }
}
