/*
    EL7062 stepper output stage, channel 1, in Cyclic Synchronous Position mode,
    driven as a clock: one 6 degree step per second, one revolution per minute.
    Each step prints the following error, i.e. the commanded position against the
    drive's reported actual position.

    WHAT "CLOSED LOOP" MEANS HERE. Commutation type 16 (0x8010:64) is "Stepper with
    internal counter": the drive counts its own steps and closes its position loop
    around that count. So the error printed below is the DRIVE's internal following
    error, not encoder feedback. It is a real closed loop, but it cannot detect a
    physically skipped step - the counter would read on target while the shaft did
    not. The external encoder covers that, via commutation types 17/18 or the
    step-loss diagnostic 0x4416. Type 16 does not use the encoder, which is why the
    example works with it disabled.

    WHY YOU SEE A FAULT ON EVERY STARTUP. Ctrl+C kills this process while the
    drive is still in Op, so it loses process data, logs DC-Link undervoltage
    (0x4411) and PD-Watchdog (0x8105), and latches a fault. The next run prints
    `FAULT: statusword 0x0008, latched by an earlier run` before clearing it on
    enable. That line is expected after an interrupted run, not a new fault, and
    each interrupt adds one more 0x4411/0x8105 pair to the drive's 0x10F3
    history. A graceful Op -> SafeOp transition is not possible here: the master's
    state-change channel acts on NoInterface, PreOp and Op only, and drops every
    other target, so there is no supported way for an example to leave Op.

    Gotchas this example exists to demonstrate, each of which cost real time:

      * UNITS. Position is 2^singleturn_bits increments per revolution (0x8000:12,
        default 20 -> 1,048,576/rev). Velocity is a different, coarser unit
        (0x9010:20, ~268435/rev); not used here, but see el7062_maximal.rs.
      * SYNC0. The EL7062's DC-Synchron OpMode declares a fixed 62500 ns SYNC0
        cycle (ESI CycleTimeSync0 Factor="0"), and SafeOp rejects anything else
        with AL status 0x0035. Hence sync0_period is pinned, not derived.
      * THREADING. This thread is deliberately not real-time, unlike the other
        examples here. The master's I/O thread owns the cycle timing and runs on
        core 3; an app thread busy-waiting at hard RT priority on that same core
        starved it, and the master stopped cycling after about a minute. So it
        sleeps out all but the last 100 us of each cycle.
      * SEEDING. The target is seeded from the terminal's own position on the
        first cycle, before the drive is enabled. A placeholder would be a jump
        of however many revolutions the axis is from zero, and the drive follows
        it the moment it enables.

    Config defaults, and the motor/encoder this assumes, are printed at startup.
    For CSV, jog profiles and scale readback, see examples/el7062_maximal.rs.
*/

use bitvec::slice::BitSlice;
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7062::{
            EL7062, EL7062_PRODUCT_ID, EL7062Port, motion::increments_per_revolution,
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

/// The motor this example assumes: 200 full steps/rev, 1.8 A/phase. Both the
/// rated and the configured current are set to this, overriding the library
/// default of 3000 mA, which is 1.67x the rating and makes the motor run hot.
const MOTOR_RATED_MA: u32 = 1800;

/// 0x8008:12 encoder type. 1 = RS422 differential, which the WEDL5541-A14 is.
/// The library default of 0 disables the encoder entirely.
const ENCODER_TYPE: u16 = 1;

/// 0x8008:13 is the resolution AFTER 4-fold evaluation, per the Beckhoff
/// parameter documentation, so 500 CPR from the encoder data sheet is 2000
/// here. The library default of 4096 belongs to a different encoder.
const ENCODER_INCR_PER_REV: u32 = 2_000;

/// 0x8010:31 velocity limitation, in rev/min. The library default of 100000 is
/// effectively no limit at all.
const VELOCITY_LIMIT_REV_PER_MIN: u32 = 300;

/// 0x8010:73 acceleration limitation, in 0.1 rad/s2, so 2000 = 200 rad/s2. The
/// library default of 62832 (6283.2 rad/s2) is 31x higher and makes each step of
/// the clock a full-power lurch.
const ACCEL_LIMIT_0_1_RAD_PER_S2: u32 = 2000;

fn main() {
    let interface = env::args().nth(1).expect(USAGE);
    let cycle_time_us: u64 = match env::args().nth(2) {
        Some(s) => s.parse().expect("cycle_time_us must be a u64"),
        None => 1000,
    };
    // get_current_cycle() counts master cycles, so one second is this many.
    let step_cycles = 1_000_000 / cycle_time_us;

    // write_config sends these values to the terminal, and the library defaults
    // are wrong for the hardware this example assumes. Say so before the bus is
    // even touched, so it is the first thing the user sees rather than a line
    // they scroll past once the motor is already turning.
    let mut el7062 = EL7062::new();
    let channel1 = &mut el7062.configuration.channel_1;
    channel1.motor.rated_current = MOTOR_RATED_MA;
    channel1.motor.configured_motor_current = MOTOR_RATED_MA;
    channel1.feedback.encoder_type = ENCODER_TYPE;
    channel1.feedback.encoder_increments_per_revolution = ENCODER_INCR_PER_REV;

    // The library defaults leave acceleration effectively unlimited
    // (62832 = 6283.2 rad/s2) and velocity at 100000 rev/min. The drive then
    // executes each 6 deg clock step as violently as it physically can, which is
    // loud, runs hot, and defeats the point of a slow smooth turn. Capping both
    // makes the motor turn quietly and keeps a stuck shaft from being driven at
    // full power.
    let amp = &mut channel1.amplifier;
    amp.velocity_limitation = VELOCITY_LIMIT_REV_PER_MIN;
    amp.acceleration_limitation = ACCEL_LIMIT_0_1_RAD_PER_S2;
    // One full revolution of following error before the drive gives up, in the
    // same increments everything downstream uses.
    amp.following_error_window = increments_per_revolution(channel1.feedback.singleturn_bits);
    amp.following_error_timeout = 100;

    // Reported before the bus is even touched, so it is the first thing the user
    // sees rather than a line they scroll past once the motor is already turning.
    let ch1 = &el7062.configuration.channel_1;
    println!(
        "assuming a {MOTOR_RATED_MA} mA (1.8 A/phase), {} full steps/rev motor and a 500 CPR \
         RS422 encoder. Writing rated_current={} mA, configured_motor_current={} mA, \
         motor_full_steps_per_revolution={}, encoder_type={} (1 = RS422), \
         encoder_increments_per_revolution={} (500 CPR after 4-fold evaluation), \
         commutation_type={}, velocity_limit={} rev/min, acceleration_limit={} rad/s^2, \
         following_error_window={} increments. The encoder is only monitored, not used for \
         commutation, at commutation type {}. The library defaults (6283.2 rad/s^2, 100000 \
         rev/min) are deliberately overridden: they make each clock step a full-power lurch \
         that is loud and runs the motor hot. Check every value against your own hardware.",
        ch1.motor.motor_full_steps_per_revolution,
        ch1.motor.rated_current,
        ch1.motor.configured_motor_current,
        ch1.motor.motor_full_steps_per_revolution,
        ch1.feedback.encoder_type,
        ch1.feedback.encoder_increments_per_revolution,
        ch1.amplifier.commutation_type,
        ch1.amplifier.velocity_limitation,
        ch1.amplifier.acceleration_limitation as f64 / 10.0,
        ch1.amplifier.following_error_window,
        ch1.amplifier.commutation_type,
    );

    let dc_config = DcConfiguration {
        start_delay: Duration::from_millis(100),
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

    let incr_per_rev =
        increments_per_revolution(el7062.configuration.channel_1.feedback.singleturn_bits) as f64;
    // Negative, so the hand runs clockwise as seen from the drive face. The
    // drive's positive position direction is fixed by the encoder wiring and
    // happens to be anticlockwise here. Negating the step is deliberate: setting
    // 0x8008:01 to invert the feedback would also flip the sign of velocity in
    // CSV mode and of the position jog. Flip this back if the motor is mounted
    // the other way round.
    let step_incr = -((incr_per_rev / STEPS_PER_REV) as i32);
    println!(
        "EL7062 @0x{addr:04X} clock: {STEPS_PER_REV:.0} steps/rev, {step_incr} incr/step, \
         {incr_per_rev:.0} incr/rev"
    );

    // Seed the setpoint from the terminal's own position, so the very first
    // target is a no-op and the drive never sees a jump on enable.
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
                     0x10F3 history, which el7062_maximal dumps at startup."
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
        // latched fault on the way.
        let statusword = el7062.get_statusword(CH).unwrap();
        el7062.apply_controlword(CH, &statusword).unwrap();

        if statusword.fault && !faulted {
            println!(
                "FAULT: statusword 0x{:04X}, latched by an earlier run",
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
            target = el7062.get_position(CH).unwrap();
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
                let actual = el7062.get_position(CH).unwrap();
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
        el7062.set_target_position(CH, target).unwrap();

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

    // No clean shutdown is attempted, because the library cannot do one. The
    // master's state-change channel (controller.rs) acts on NoInterface, PreOp
    // and Op only; every other target, including state 4 / SafeOp, falls through
    // to `_ => continue` and is dropped without a response. So Op cannot be left
    // gracefully, and killing the process here means the drive loses process
    // data: it logs DC-Link undervoltage (0x4411) plus PD-Watchdog (0x8105) and
    // latches a fault. The next run therefore starts with
    // `FAULT: statusword 0x0008, latched by an earlier run`, which the enable
    // sequence below clears. That line is expected after Ctrl+C, not a new fault.
}
