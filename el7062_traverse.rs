/*
    EL7062 channel 1 as a linear traverse, controlled from the terminal.

    The drive runs Cyclic Synchronous Position with commutation type 17
    ("stepper with encoder"): it closes its position loop around the encoder,
    so a skipped step is corrected by the drive itself and shows up in the
    following error printed here. In Cyclic Synchronous Position the master
    computes every position setpoint, so the ramp, homing, speed limits and
    endstop handling all live in the traverse helper
    (ethercat_hal::helpers::traverse); this file only moves process data
    between it and the terminal, including the raw endstop levels that the
    helper applies its configured polarity to.

    Type `help` once the drive is in Op; a typical first run is `status`,
    `home`, `goto 100`, then `calibrate <measured travel>`.

    Killing the process while the drive is in Op latches a fault on the
    drive; the next run's `reset` clears it. The EL7062 wiki page has the
    details.
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
            pdo::DiInputs,
        },
    },
    helpers::traverse::{
        EndstopPolarity, Endstops, HomingConfig, HomingPhase, Traverse, TraverseConfig,
        TraverseInput, TraverseState,
    },
    init_ethercat,
};
use std::{
    env,
    io::BufRead,
    sync::mpsc,
    time::{Duration, Instant},
};

const USAGE: &str = "el7062_traverse <interface> <channel>\n \
    example: sudo ./target/release/examples/el7062_traverse enp4s0";

/// 1-based index among all EL7062 terminals found on the bus.
const TARGET_EL7062_INDEX: usize = 1;
const CH: EL7062Port = EL7062Port::Ch1;

const CYCLE_TIME_US: u64 = 1000;

// --- Mechanics: check every value against your own rig -------------------

/// Full steps per motor revolution (0x8011:33).
const MOTOR_FULL_STEPS_PER_REV: u32 = 200;

/// Travel per motor revolution, e.g. the lead-screw pitch.
const MM_PER_REV: f64 = 1.0;

/// Starting ratio in full steps per mm. `calibrate` prints a corrected value
/// to paste back here.
const STEPS_PER_MM: f64 = MOTOR_FULL_STEPS_PER_REV as f64 / MM_PER_REV;

/// Usable travel. Ignored once homing measures it (`measure_length`).
const TRAVERSE_LENGTH_MM: f64 = 300.0;

/// Set if positive motor rotation moves towards the home end.
const INVERT_DIRECTION: bool = true;

const MAX_SPEED_MM_S: f64 = 1.0;
const ACCELERATION_MM_S2: f64 = 1.0;

// --- Endstops ------------------------------------------------------------

/// `Single` searches towards one switch only — the safe default, because
/// homing with `Both { measure_length: true }` and no far switch fitted
/// would drive into the mechanical end looking for it.
// const ENDSTOPS: Endstops = Endstops::Single(HomeSide::Min);
const ENDSTOPS: Endstops = Endstops::Both {
    measure_length: true,
};

const HOME_INPUT: DiInput = DiInput::Input1;
const FAR_INPUT: DiInput = DiInput::Input2;
/// Normally open switches are `ActiveHigh`. Normally closed (`ActiveLow`) is
/// safer, because a broken wire then reads as hit.
const HOME_POLARITY: EndstopPolarity = EndstopPolarity::ActiveHigh;
const FAR_POLARITY: EndstopPolarity = EndstopPolarity::ActiveHigh;

const HOMING_SEARCH_MM_S: f64 = 5.0;
const HOMING_RELEASE_MM_S: f64 = 5.0;
const HOMING_CLEARANCE_MM: f64 = 5.0;

// --- Drive ---------------------------------------------------------------

/// 1.8 A/phase motor, run well below rated current because it gets hot.
const MOTOR_RATED_MA: u32 = 1800;
const MOTOR_CONFIGURED_MA: u32 = 500;
/// WEDL5541-A14: RS422, 500 CPR, so 2000 increments after 4-fold evaluation.
const ENCODER_TYPE: EncoderType = EncoderType::Rs422Differential;
const ENCODER_INCR_PER_REV: u32 = 2_000;
/// 0x8010:31, rev/min. The traverse speed must stay below this.
const VELOCITY_LIMIT_REV_PER_MIN: u32 = 300;
/// 0x8010:73, 0.1 rad/s².
const ACCEL_LIMIT_0_1_RAD_PER_S2: u32 = 2000;
/// 0x8010:72, thousandths of nominal current.
const STAND_STILL_TORQUE_PERMILLE: u16 = 200;

const SPIN_HEADROOM_US: u64 = 100;
const STALL_TIMEOUT: Duration = Duration::from_millis(500);
const STATUS_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
enum DiInput {
    Input1,
    Input2,
}

impl DiInput {
    fn level(self, di: &DiInputs) -> bool {
        match self {
            Self::Input1 => di.input_1,
            Self::Input2 => di.input_2,
        }
    }
}

/// Converts between the user-facing steps/mm and the drive's increments.
struct Scale {
    incr_per_step: f64,
}

impl Scale {
    fn units_per_mm(&self, steps_per_mm: f64) -> f64 {
        steps_per_mm * self.incr_per_step
    }
    fn steps_per_mm(&self, units_per_mm: f64) -> f64 {
        units_per_mm / self.incr_per_step
    }
}

fn main() {
    let interface = env::args().nth(1).expect(USAGE);

    let ch = CH;
    let cycle_time_us: u64 = CYCLE_TIME_US;

    let mut el7062 = EL7062::new();
    let channel = match ch {
        EL7062Port::Ch1 => &mut el7062.configuration.channel_1,
        EL7062Port::Ch2 => &mut el7062.configuration.channel_2,
    };
    channel.motor.rated_current = MOTOR_RATED_MA;

    channel.motor.configured_motor_current = MOTOR_CONFIGURED_MA;
    channel.motor.motor_full_steps_per_revolution = MOTOR_FULL_STEPS_PER_REV;
    channel.feedback.encoder =
        EncoderConfig::wired(ENCODER_TYPE, ENCODER_INCR_PER_REV).expect("non-zero resolution");
    // This rig's encoder counts the opposite way round to its motor; at type
    // 17 that closes the loop with positive feedback unless inverted.
    channel.feedback.invert_feedback_direction = false;

    let amp = &mut channel.amplifier;
    amp.commutation = Commutation::StepperWithEncoder;
    amp.stand_still_torque_limitation = STAND_STILL_TORQUE_PERMILLE;
    amp.velocity_limitation = VELOCITY_LIMIT_REV_PER_MIN;
    amp.acceleration_limitation = ACCEL_LIMIT_0_1_RAD_PER_S2;
    // One revolution of following error for 100 ms before the drive faults.
    amp.following_error = FollowingErrorMonitor::enabled(
        channel.feedback.position_scale.increments_per_revolution(),
        100,
    )
    .expect("usable following error window");

    let position_scale = match ch {
        EL7062Port::Ch1 => el7062.configuration.channel_1.feedback.position_scale,
        EL7062Port::Ch2 => el7062.configuration.channel_2.feedback.position_scale,
    };
    let scale = Scale {
        incr_per_step: position_scale.increments_per_step(MOTOR_FULL_STEPS_PER_REV),
    };
    let drive_limit_mm_s = VELOCITY_LIMIT_REV_PER_MIN as f64 / 60.0 * MM_PER_REV;
    assert!(
        MAX_SPEED_MM_S < drive_limit_mm_s,
        "MAX_SPEED_MM_S exceeds the drive's own velocity limit of {drive_limit_mm_s:.1} mm/s"
    );

    let mut traverse = Traverse::new(TraverseConfig {
        units_per_mm: scale.units_per_mm(STEPS_PER_MM),
        length_mm: TRAVERSE_LENGTH_MM,
        invert_direction: INVERT_DIRECTION,
        max_speed_mm_s: MAX_SPEED_MM_S,
        acceleration_mm_s2: ACCELERATION_MM_S2,
        position_tolerance_mm: 0.05,
        in_position_timeout: Duration::from_secs(1),
        homing: HomingConfig {
            endstops: ENDSTOPS,
            home_polarity: HOME_POLARITY,
            far_polarity: FAR_POLARITY,
            search_speed_mm_s: HOMING_SEARCH_MM_S,
            release_speed_mm_s: HOMING_RELEASE_MM_S,
            clearance_mm: HOMING_CLEARANCE_MM,
            // A little beyond the full length, so a switch at the far end of
            // the travel is still found.
            max_search_mm: TRAVERSE_LENGTH_MM * 1.2,
        },
    })
    .expect("valid traverse config");

    println!(
        "{MOTOR_FULL_STEPS_PER_REV} steps/rev, {MM_PER_REV} mm/rev, {STEPS_PER_MM} steps/mm, \
         {:.2} incr/step, {:.1} incr/mm, length {TRAVERSE_LENGTH_MM} mm, endstops {ENDSTOPS:?}, \
         drive velocity limit {drive_limit_mm_s:.1} mm/s",
        scale.incr_per_step,
        traverse.units_per_mm(),
    );

    let dc_config = DcConfiguration {
        start_delay: Duration::from_millis(100),
        // The EL7062's DC-Synchron OpMode only accepts a 62.5 µs SYNC0.
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

    let el7062_addrs: Vec<_> = eth_handle
        .try_get_subdevices_vec_sync()
        .unwrap()
        .iter()
        .filter(|s| s.product_id == EL7062_PRODUCT_ID)
        .map(|s| s.device_address)
        .collect();
    assert!(!el7062_addrs.is_empty(), "No EL7062 found on the bus");
    let addr = *el7062_addrs
        .get(TARGET_EL7062_INDEX - 1)
        .unwrap_or_else(|| {
            panic!(
                "Requested EL7062 index {} but only {} found ({el7062_addrs:?})",
                TARGET_EL7062_INDEX,
                el7062_addrs.len(),
            )
        });
    el7062
        .write_config(eth_control.channel.clone(), addr, &el7062.get_config())
        .expect("Failed to write EL7062 config");
    eth_control
        .channel
        .enable_dc_sync01(addr, Duration::from_micros(cycle_time_us))
        .expect("Failed to enable DC sync01");

    println!("all in op");

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to request Op");
    while !eth_handle.check_all_op() {
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("all in op");

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    let target_subdevice = subdevices
        .iter()
        .find(|s| s.device_address == addr)
        .expect("Selected EL7062 not found in process image");

    // Reading stdin blocks, so it gets its own thread and the cycle loop only
    // polls the channel.
    let (commands, command_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if commands.send(line).is_err() {
                break;
            }
        }
    });
    println!("EL7062 @0x{addr:04X} in Op. Type `help` for commands.");

    // The process-data position is a wrapping 32-bit counter, so it is
    // unwrapped into an i64 for the helper and the setpoint is wrapped back
    // the same way.
    let mut last_raw_position: Option<i32> = None;
    let mut actual_position = 0i64;
    let mut last_cycle = eth_handle.get_current_cycle();
    let mut last_state = traverse.state();
    let mut last_status = Instant::now();
    let mut last_fault = false;

    loop {
        // The master's I/O thread owns the EtherCAT timing, so this thread
        // sleeps instead of spinning at RT priority.
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
                     so the drive has dropped out of Op."
                );
            }
        }

        if let Some(inputs) = eth_handle.get_inputs() {
            let input = &inputs[target_subdevice.start_tx..target_subdevice.end_tx];
            el7062.input(BitSlice::from_slice(input)).unwrap();
            el7062.input_post_process().unwrap();
        }

        let cycle = eth_handle.get_current_cycle();
        let dt = Duration::from_micros(cycle.wrapping_sub(last_cycle) * cycle_time_us);
        last_cycle = cycle;

        let mut status_requested = false;
        while let Ok(line) = command_rx.try_recv() {
            status_requested |=
                handle_command(line.trim(), &mut traverse, &scale, drive_limit_mm_s);
        }

        let mut axis = el7062.axis(ch);
        let statusword = axis.statusword().unwrap();
        axis.apply_controlword(&statusword).unwrap();

        if statusword.fault && !last_fault {
            // One SDO read pair on the fault edge: the DC-link level answers
            // the most common "why", the amplifier temperature separates it
            // from an overtemperature event.
            let dc_link = EL7062::read_dc_link_voltage(&eth_control.channel, addr)
                .map(|mv| format!("{mv} mV"))
                .unwrap_or_else(|e| format!("unreadable: {e}"));
            let temperature = EL7062::read_amplifier_temperature(&eth_control.channel, addr)
                .map(|t| format!("{:.1} °C", t as f64 / 10.0))
                .unwrap_or_else(|e| format!("unreadable: {e}"));
            println!(
                "FAULT: statusword 0x{:04X}, DC-link {dc_link}, amplifier {temperature}",
                statusword.as_raw()
            );
        }
        last_fault = statusword.fault;

        let raw_position = axis.position().unwrap();
        actual_position = match last_raw_position {
            Some(last) => actual_position + raw_position.wrapping_sub(last) as i64,
            None => raw_position as i64,
        };
        last_raw_position = Some(raw_position);

        let di = axis.digital_inputs().unwrap();
        let output = traverse.update(
            &TraverseInput {
                actual_position,
                drive_ready: statusword.operation_enabled && !statusword.fault,
                endstop_home: HOME_INPUT.level(&di),
                endstop_far: FAR_INPUT.level(&di),
            },
            dt,
        );
        axis.set_target_position(output.target_position as i32)
            .unwrap();

        let state = traverse.state();
        let in_motion = matches!(
            state,
            TraverseState::Homing(_) | TraverseState::Moving | TraverseState::Stopping
        );
        if state != last_state {
            println!("-> {state}");
            if last_state == TraverseState::Homing(HomingPhase::Park)
                && state == TraverseState::Idle
            {
                println!("homed, length {:.3} mm", traverse.length_mm());
            }
            last_state = state;
        }
        if status_requested || (in_motion && last_status.elapsed() >= STATUS_INTERVAL) {
            let following_error = axis.following_error().unwrap_or(0);
            print_status(&traverse, &di, following_error);
            last_status = Instant::now();
        }

        if let Some(outputs) = eth_handle.write_outputs() {
            el7062.output_pre_process().unwrap();
            let out = &mut outputs[target_subdevice.start_rx..target_subdevice.end_rx];
            el7062.output(BitSlice::from_slice_mut(out)).unwrap();
        }
        eth_handle.send_outputs();
    }
}

const HELP: &str = "\
commands:
  home                 find the home switch and park (measures the length if configured)
  goto <mm> | g <mm>   move to a position on the traverse
  pause / resume       brake to a standstill and continue the same move
  stop                 brake and drop the move
  speed <mm/s>         top speed for moves
  length <mm>          usable travel
  ratio <steps/mm>     full steps per mm
  calibrate <mm>       correct the ratio from the measured length of the last move
  reset                clear a fault (homing is needed again)
  status               print position, progress, following error and endstops";

/// Applies one command line. Returns true for `status`, which needs the
/// drive's following error and is printed by the cycle loop.
fn handle_command(
    line: &str,
    traverse: &mut Traverse,
    scale: &Scale,
    drive_limit_mm_s: f64,
) -> bool {
    let mut words = line.split_whitespace();
    let Some(command) = words.next() else {
        return false;
    };
    let number = words.next().map(str::parse::<f64>);
    let result = match (command, number) {
        ("help" | "?", _) => {
            println!("{HELP}");
            return false;
        }
        ("status" | "s", _) => return true,
        ("home", _) => traverse.home(),
        ("goto" | "g", Some(Ok(mm))) => traverse.go_to(mm),
        ("pause" | "p", _) => traverse.pause(),
        ("resume" | "r", _) => traverse.resume(),
        ("stop", _) => traverse.stop(),
        ("reset", _) => {
            traverse.reset_fault();
            Ok(())
        }
        ("speed", Some(Ok(mm_s))) if mm_s >= drive_limit_mm_s => {
            println!("{mm_s} mm/s is above the drive's limit of {drive_limit_mm_s:.1} mm/s");
            return false;
        }
        ("speed", Some(Ok(mm_s))) => traverse.set_speed(mm_s),
        ("length", Some(Ok(mm))) => traverse.set_length(mm),
        ("ratio", Some(Ok(steps_per_mm))) => {
            traverse.set_units_per_mm(scale.units_per_mm(steps_per_mm))
        }
        ("calibrate", Some(Ok(measured_mm))) => match traverse.last_move_distance_mm() {
            None => {
                println!("no completed move to calibrate against yet");
                return false;
            }
            Some(commanded_mm) => traverse.calibrate(commanded_mm, measured_mm).map(|upm| {
                println!(
                    "{commanded_mm:.3} mm commanded, {measured_mm:.3} mm measured: \
                     ratio is now {:.4} steps/mm (STEPS_PER_MM)",
                    scale.steps_per_mm(upm)
                );
            }),
        },
        (_, Some(Err(_))) | ("goto" | "g" | "speed" | "length" | "ratio" | "calibrate", None) => {
            println!("`{command}` needs a number, see `help`");
            return false;
        }
        _ => {
            println!("unknown command `{command}`, see `help`");
            return false;
        }
    };
    match result {
        Ok(()) => println!("ok"),
        Err(e) => println!("rejected: {e}"),
    }
    false
}

fn print_status(traverse: &Traverse, di: &DiInputs, following_error: i32) {
    let mm = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.3} mm"));
    let progress = traverse
        .progress_percent()
        .map_or("-".to_string(), |p| format!("{p:.1} %"));
    let (home_hit, far_hit) = traverse.endstops_hit();
    println!(
        "{} | pos {} | target {} | progress {progress} | {:.1} mm/s | following error {:.4} mm | \
         home in={} hit={home_hit} | far in={} hit={far_hit}",
        traverse.state(),
        mm(traverse.position_mm()),
        mm(traverse.target_mm()),
        traverse.velocity_mm_s(),
        following_error as f64 / traverse.units_per_mm(),
        HOME_INPUT.level(di) as u8,
        FAR_INPUT.level(di) as u8,
    );
}
