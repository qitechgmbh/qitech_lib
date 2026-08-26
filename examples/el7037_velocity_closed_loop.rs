//! Closed-loop positioning on an EL7037 with the position loop closed **in Rust**.
//!
//! Unlike `el7037_closed_loop.rs`, the terminal runs in `Velocity direct`: no
//! travel-command generator, no internal position loop, just a commanded speed.
//! This program reads the encoder every cycle and feeds it to
//! `ethercat_hal::helpers::velocity_position_loop::VelocityPositionLoop`, a
//! device-agnostic controller that computes the speed and decides on its own
//! when a move is finished. That removes this axis's actual failure mode,
//! the terminal crawling at `Velocity min.` towards a target it cannot reach,
//! and puts the acceleration limit under our control, which matters because
//! step loss, not position gain, is what breaks moves here.
//!
//! The trade: the loop now runs at this program's cycle time (250 us, matching
//! the terminal's own internal cycle) and is only as good as the scheduling
//! jitter of a non-realtime thread at that rate. Fine for point-to-point moves
//! that end in a target window; not a substitute for a real machine axis.
//!
//! # Control law
//!
//! Every cycle, for error `e = target - position` and distance-to-band-edge
//! `x = |e| - tolerance`:
//!
//! ```text
//! demand = sign(e) * min(max_speed, sqrt(2 * braking_acceleration * x), approach_gain * x)
//! ```
//!
//! slewed by at most `acceleration * dt`, and zero once `|e| <= tolerance`. No
//! integral term: an integrator against a stepper that is losing steps winds up
//! and produces exactly the runaway this axis already had. See
//! `VelocityPositionLoop::step` and `loop_config` below for why each tunable
//! has the value it does.
//!
//! # `motor_stall` is a pulse, not a latch
//!
//! Measured with `examples/el7037_stall_diagnostic`: it clears itself within
//! 50-350 ms with no `Reset`/`Enable` cycle needed, so a live boolean sampled
//! occasionally is easy to miss. The loop counts rising edges and total
//! stalled time per move instead, reported in `LoopEvent::Arrived`.
//!
//! # Direction self-check
//!
//! On startup this commands a small positive speed and checks the encoder
//! counts up. A software position loop against a backwards encoder is positive
//! feedback and will run away on the first move - this is the exact fault this
//! rig has hit before (`0x8000:0E` wrong), so the check refuses to run rather
//! than let you discover it at speed.
//!
//! # Units
//!
//! Positions and speeds are in **encoder counts** and counts/s throughout,
//! converted to full steps/s only at the `set_speed` boundary. With a 1000 ppr
//! encoder (4-fold = 4000 counts/rev) and a 200 full-step motor, one full step
//! is 20 counts.
//!
//! # Commands
//!
//! ```text
//! 2000      move to absolute encoder count 2000
//! a -2000   move to absolute count -2000
//! +800 -800 move relative
//! 1.5r      the same three forms in revolutions
//! v <n>     maximum speed, counts/s
//! hold on|off  re-correct if the axis drifts past `re_engage` (default on)
//! h         home here - set the counter to 0
//! s         stop now (ramps down at the acceleration limit)
//! e         toggle the driver stage
//! ?         help
//! q         quit
//! ```
//!
//! Usage: `cargo run --example el7037_velocity_closed_loop -- <interface>`

use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7037::{
            EL7037, EL7037_PRODUCT_ID, coe::EL7037Configuration, pdo::EL7037PredefinedPdoAssignment,
        },
    },
    helpers::{
        el70xx_velocity_converter::EL70x1VelocityConverter,
        velocity_position_loop::{LoopEvent, LoopState, VelocityPositionLoop, VelocityPositionLoopConfig},
    },
    init_ethercat,
    io::stepper_velocity_el70x1::StepperVelocityEL70x1Device,
    shared_config::el70x7::{
        EL70x1OperationMode, EL70x1SpeedRange, EL70x7InfoData, EL7037FeedbackType,
    },
};
use std::{
    env,
    io::Write,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

// ── Tunables ────────────────────────────────────────────────────────────────

const CYCLE_TIME: Duration = Duration::from_nanos(250_000);
const STATUS_EVERY: u32 = 50;

/// Motor: igus MOT-AN-S-060-005-042-L-C-AAAO (drylin E, NEMA 17).
const MOTOR_FULL_STEPS: u16 = 200;
const ENCODER_INCREMENTS: u16 = 4000;
const COUNTS_PER_REV: f64 = ENCODER_INCREMENTS as f64;
const COUNTS_PER_FULL_STEP: f64 = COUNTS_PER_REV / MOTOR_FULL_STEPS as f64;

const MAX_CURRENT_MA: u16 = 1100;
const REDUCED_CURRENT_MA: u16 = 550;
const NOMINAL_VOLTAGE_MV: u16 = 24000;
const COIL_RESISTANCE: u16 = 175;
const COIL_INDUCTANCE: u16 = 330;

const SPEED_RANGE: EL70x1SpeedRange = EL70x1SpeedRange::Steps2000;
/// The speed range caps what `set_speed` can ask for: 100 % = 2000 full steps/s.
const MAX_FULL_STEPS_PER_S: f64 = 2000.0;

/// Below this commanded speed, round the velocity deterministically instead of
/// probabilistically - right for average fidelity while moving, but dither
/// near standstill.
const DETERMINISTIC_BELOW: f64 = 400.0;

/// Direction self-check: speed and duration of the probe move.
const PROBE_SPEED: f64 = 1000.0;
const PROBE_TIME: Duration = Duration::from_millis(100);
/// The probe must move at least this far, or the axis is not turning at all.
const PROBE_MIN_TRAVEL: f64 = 30.0;

/// The tuned [`VelocityPositionLoop`] config for this rig.
fn loop_config() -> VelocityPositionLoopConfig {
    VelocityPositionLoopConfig {
        // Cruise speed, counts/s. 16000 = 800 full steps/s = 4 rev/s.
        max_speed: 16_000.0,
        // Lower this if `motor_stall` pulses often.
        acceleration: 40_000.0,
        // Half the real slew limit: gives the stop-distance envelope margin
        // for real-motor lag during a stall event (measured: removes 90-150
        // counts of overshoot on long moves, at ~3% longer move time).
        braking_acceleration: 20_000.0,
        approach_gain: 25.0,
        min_speed: 100.0,
        // The encoder's own resolution.
        tolerance: 1,
        dwell: Duration::from_millis(120),
        // Far wider than `tolerance` - if they were close, settling noise
        // would retrigger a correction and the axis would buzz.
        re_engage: 60,
        move_timeout: Duration::from_secs(15),
        runaway_factor: 3.0,
        runaway_slack: 400.0,
    }
}

// ── Configuration ───────────────────────────────────────────────────────────

fn axis_config() -> EL7037Configuration {
    let mut config = EL7037Configuration::default();

    // Velocity direct: the terminal applies the commanded speed and nothing
    // else. No travel generator, no internal position loop, nothing to fight.
    config.stm_features.operation_mode = EL70x1OperationMode::DirectVelocity;
    config.stm_features.speed_range = SPEED_RANGE;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    config.stm_features.invert_motor_polarity = false;
    config.stm_features.select_info_data_1 = EL70x7InfoData::MotorLoad;
    config.stm_features.select_info_data_2 = EL70x7InfoData::MotorDcCurrent;

    // Measured on this rig: with this false the encoder counts opposite to the
    // commanded direction. The direction self-check catches it either way.
    config.encoder.reversion_of_rotation = true;

    config.stm_motor.max_current = MAX_CURRENT_MA;
    config.stm_motor.reduced_current = REDUCED_CURRENT_MA;
    config.stm_motor.nominal_voltage = NOMINAL_VOLTAGE_MV;
    config.stm_motor.motor_coil_resistance = COIL_RESISTANCE;
    config.stm_motor.motor_coil_inductance = COIL_INDUCTANCE;
    config.stm_motor.motor_full_steps = MOTOR_FULL_STEPS;
    config.stm_motor.encoder_increments = ENCODER_INCREMENTS;
    config.stm_motor.motor_emf = 0;

    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = 10;

    // 0x8014's position half is inert in Velocity direct - the loop that
    // matters is in this file - but the velocity half still shapes how the
    // commanded speed is followed, so it's written rather than left as-is.
    config.stm_controller_3.feed_forward_pos = 100_000;
    config.stm_controller_3.kp_factor_pos = 2;
    config.stm_controller_3.kp_factor_velo = 50;
    config.stm_controller_3.tn_velo = 50_000;

    // 0x8020 / 0x8021 (travel-command generator) are unused in this mode and
    // left at their defaults on purpose.

    config.pdo_assignment = EL7037PredefinedPdoAssignment::VelocityControlCompact;
    config
}

// ── Motion ──────────────────────────────────────────────────────────────────

/// One cycle's worth of readings from the terminal.
struct Sample {
    position: i64,
    stalled: bool,
    ready_to_enable: bool,
    ready: bool,
    error: bool,
    warning: bool,
}

impl Sample {
    fn read(el7037: &EL7037) -> Self {
        let input = el7037.get_input(0).expect("velocity input unavailable");
        Self {
            position: el7037.get_position(0) as i64,
            stalled: el7037
                .txpdo
                .stm_status
                .as_ref()
                .is_some_and(|status| status.motor_stall),
            ready_to_enable: input.ready_to_enable,
            ready: input.ready,
            error: input.error,
            warning: input.warning,
        }
    }
}

/// Starts (or retargets) a move and reports it.
fn start_move(axis: &mut VelocityPositionLoop, target: i64, position: i64, now: Instant) {
    axis.start_move(target, position, now);
    println!(
        "move {:+} counts to {} ({:+.3} rev) at up to {:.0} counts/s",
        target - position,
        target,
        counts_to_revs(target - position),
        axis.max_speed()
    );
}

/// Reports what a `LoopEvent` returned from `VelocityPositionLoop::step` means
/// for this axis, in counts and revolutions.
fn report_event(event: LoopEvent, axis: &VelocityPositionLoop, position: i64) {
    match event {
        LoopEvent::Arrived {
            residual,
            elapsed,
            stall_pulses,
            stall_ms,
        } => println!(
            "\narrived at {position} ({:.3} rev), residual {residual:+} counts, {} ms{}",
            counts_to_revs(position),
            elapsed.as_millis(),
            stall_summary(stall_pulses, stall_ms)
        ),
        LoopEvent::Runaway { travelled, limit } => {
            println!("\nRUNAWAY: travelled {travelled:.0} counts, limit {limit:.0}")
        }
        LoopEvent::TimedOut { residual } => println!(
            "\ntimed out {residual:+} counts from {}; giving up",
            axis.target()
        ),
        LoopEvent::Drifted { residual } => println!("\ndrifted {residual:+} counts; correcting"),
    }
}

/// `" (3 stall pulse(s), 210 ms total)"`, or empty if there were none.
fn stall_summary(pulses: u32, ms: f64) -> String {
    if pulses > 0 {
        format!(" ({pulses} stall pulse(s), {ms:.0} ms total)")
    } else {
        String::new()
    }
}

fn status_line(axis: &VelocityPositionLoop, sample: &Sample, speed: f64) -> String {
    let error = axis.target() - sample.position;
    format!(
        "pos {:>+9} ({:>+8.3} rev)  err {:>+7}  v {:>+8.0} c/s  {:?}{}{}{}",
        sample.position,
        counts_to_revs(sample.position),
        if axis.state() == LoopState::Idle { 0 } else { error },
        speed,
        axis.state(),
        if sample.error { "  ERROR" } else { "" },
        if sample.warning { "  WARN" } else { "" },
        if sample.stalled { "  STALL" } else { "" },
    )
}

/// counts/s to the full steps/s that the velocity IO wants.
fn counts_to_full_steps_per_s(counts_per_s: f64) -> f64 {
    (counts_per_s / COUNTS_PER_FULL_STEP).clamp(-MAX_FULL_STEPS_PER_S, MAX_FULL_STEPS_PER_S)
}

fn counts_to_revs(counts: i64) -> f64 {
    counts as f64 / COUNTS_PER_REV
}

/// Commands a speed, choosing the rounding mode. Goes through `set_output`
/// rather than the trait's `set_speed` because that hard-codes probabilistic
/// rounding - right while moving, but dither near standstill.
fn command_speed(el7037: &mut EL7037, counts_per_s: f64) {
    let steps_per_second = counts_to_full_steps_per_s(counts_per_s);
    let converter = EL70x1VelocityConverter::new(&el7037.get_speed_range(0));
    let probabilistic = counts_per_s.abs() >= DETERMINISTIC_BELOW;
    let velocity = converter.steps_to_velocity(steps_per_second, probabilistic);

    if let Ok(mut output) = el7037.get_output(0) {
        output.velocity = velocity;
        let _ = el7037.set_output(0, output);
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Command {
    Absolute(i64),
    Relative(i64),
    MaxSpeed(f64),
    Hold(bool),
    Home,
    Stop,
    ToggleEnable,
    Help,
    Quit,
}

const HELP: &str = "\
  <n>       move to absolute encoder count <n>
  a <n>     move to absolute count <n> (accepts a negative sign)
  +<n> -<n> move <n> counts relative to the current position
  <n>r      the same three forms in revolutions, e.g. 1.5r  +0.25r  a -2r
  v <n>     maximum speed in counts/s
  hold on|off  re-correct if the axis drifts more than 60 counts (default on)
  h         home here - set the encoder counter to 0
  s         stop now (ramps down at the acceleration limit)
  e         toggle the driver stage
  ?         this help
  q         quit";

fn parse_command(line: &str) -> Result<Option<Command>, String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }
    match line {
        "q" | "quit" => return Ok(Some(Command::Quit)),
        "hold on" => return Ok(Some(Command::Hold(true))),
        "hold off" => return Ok(Some(Command::Hold(false))),
        "h" | "home" => return Ok(Some(Command::Home)),
        "s" | "stop" => return Ok(Some(Command::Stop)),
        "e" => return Ok(Some(Command::ToggleEnable)),
        "?" | "help" => return Ok(Some(Command::Help)),
        _ => {}
    }

    if let Some(rest) = line.strip_prefix("v ") {
        let speed: f64 = rest
            .trim()
            .parse()
            .map_err(|_| format!("not a number: {rest:?}"))?;
        if speed <= 0.0 {
            return Err("speed must be positive".into());
        }
        return Ok(Some(Command::MaxSpeed(speed)));
    }

    let (body, forced_absolute) = match line.strip_prefix("a ") {
        Some(rest) => (rest.trim(), true),
        None => (line, false),
    };
    let relative = !forced_absolute && (body.starts_with('+') || body.starts_with('-'));

    let counts = match body.strip_suffix('r') {
        Some(revs) => {
            let revs: f64 = revs
                .trim()
                .parse()
                .map_err(|_| format!("not a number: {revs:?}"))?;
            (revs * COUNTS_PER_REV).round() as i64
        }
        None => body
            .parse::<i64>()
            .map_err(|_| format!("unknown command: {line:?} (try ?)"))?,
    };

    Ok(Some(if relative {
        Command::Relative(counts)
    } else {
        Command::Absolute(counts)
    }))
}

/// Applies a parsed command. Returns `true` if the program should quit.
fn apply_command(
    command: Command,
    axis: &mut VelocityPositionLoop,
    el7037: &mut EL7037,
    position: i64,
    now: Instant,
) -> bool {
    match command {
        Command::Absolute(to) => start_move(axis, to, position, now),
        Command::Relative(delta) => start_move(axis, position + delta, position, now),
        Command::MaxSpeed(speed) => {
            axis.set_max_speed(speed);
            println!(
                "max speed {speed:.0} counts/s = {:.0} full steps/s",
                counts_to_full_steps_per_s(speed)
            );
        }
        Command::Hold(on) => {
            axis.set_hold_enabled(on);
            println!("position hold {}", if on { "on" } else { "off" });
        }
        Command::Stop => {
            axis.stop();
            println!("stopping");
        }
        Command::Home => {
            if !axis.home() {
                println!("refusing to home during a move");
            } else {
                el7037.set_position(0, 0);
                println!("homed");
            }
        }
        Command::ToggleEnable => {
            let enabled = el7037.is_enabled(0);
            axis.stop();
            el7037.set_enabled(0, !enabled);
            println!("{}", if enabled { "disabled" } else { "enabling" });
        }
        Command::Help => println!("{HELP}"),
        Command::Quit => return true,
    }
    false
}

fn spawn_input_thread() -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("stdin".into())
        .spawn(move || {
            for line in std::io::stdin().lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            return;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = sender.send(String::from("q"));
        })
        .expect("could not spawn the input thread");
    receiver
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth = init_ethercat(&interface, None);

    eth.channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    while eth.app_handle.get_state() != EtherCATState::PreOp {
        std::thread::sleep(Duration::from_millis(10));
    }

    let mut el7037 = EL7037::new();
    let mut address = None;
    for _ in 0..50 {
        let subdevices = eth
            .app_handle
            .try_get_subdevices_vec_sync()
            .expect("Failed to read subdevices!");
        for subdevice in &subdevices {
            if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
                address = Some(subdevice.device_address);
            }
        }
        if address.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let address = address.unwrap_or_else(|| panic!("no EL7037 found on {interface}"));
    el7037
        .write_config(eth.channel.clone(), address, &axis_config())
        .expect("EL7037 CoE config failed");
    println!("EL7037 at {address} in Velocity direct; position loop runs in this program");

    eth.channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    while eth.app_handle.get_state() != EtherCATState::Op {
        std::thread::sleep(Duration::from_millis(10));
    }

    let subdevice = eth
        .app_handle
        .try_get_subdevices_vec_sync()
        .expect("Failed to read subdevices!")
        .into_iter()
        .find(|s| s.device_address == address)
        .expect("EL7037 disappeared during the Op transition");
    let (start_tx, end_tx) = (subdevice.start_tx, subdevice.end_tx);
    let (start_rx, end_rx) = (subdevice.start_rx, subdevice.end_rx);

    let mut cycle = |el7037: &mut EL7037| {
        if let Some(input) = eth.app_handle.get_inputs() {
            let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(&input[start_tx..end_tx]));
            let _ = el7037.input_post_process();
            eth.app_handle.finish_read();
        }
        let _ = el7037.output_pre_process();
        if let Some(output) = eth.app_handle.write_outputs() {
            let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(
                &mut output[start_rx..end_rx],
            ));
            eth.app_handle.send_outputs();
        }
        std::thread::sleep(CYCLE_TIME);
    };

    // Wait for the drive to come up, then enable.
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        cycle(&mut el7037);
        let sample = Sample::read(&el7037);
        if sample.ready_to_enable {
            el7037.set_enabled(0, true);
        }
        if sample.ready && el7037.is_enabled(0) {
            break;
        }
        if Instant::now() > deadline {
            panic!("drive never became ready (ready_to_enable never set, or an error is latched)");
        }
    }
    println!("driver stage ready");

    if let Err(message) = direction_self_check(&mut el7037, &mut cycle) {
        // Refuse rather than let a positive-feedback loop discover this at speed.
        command_speed(&mut el7037, 0.0);
        el7037.set_enabled(0, false);
        for _ in 0..50 {
            cycle(&mut el7037);
        }
        eprintln!("\ndirection self-check failed: {message}");
        eprintln!(
            "the encoder and the motor disagree on which way is positive. Flip exactly one of\n\
             `reversion_of_rotation` (0x8000:0E) or `invert_motor_polarity` (0x8012:09)."
        );
        std::process::exit(1);
    }

    let commands = spawn_input_thread();
    let mut axis = VelocityPositionLoop::new(loop_config(), el7037.get_position(0) as i64, Instant::now());
    let mut last = Instant::now();
    let mut cycle_count = 0u32;

    println!("{HELP}");

    loop {
        cycle(&mut el7037);

        let now = Instant::now();
        // The floor has to sit well below CYCLE_TIME (250us): a floor above the
        // real per-cycle duration silently inflates every dt-based accumulation
        // on every single cycle. 5us only guards a literal zero/negative delta;
        // the 50ms ceiling guards a real stall in the polling loop.
        let dt = now
            .duration_since(last)
            .as_secs_f64()
            .clamp(0.000_005, 0.05);
        last = now;

        let sample = Sample::read(&el7037);

        match commands.try_recv() {
            Ok(line) => match parse_command(&line) {
                Ok(Some(command)) => {
                    if apply_command(command, &mut axis, &mut el7037, sample.position, now) {
                        break;
                    }
                }
                Ok(None) => {}
                Err(message) => println!("{message}"),
            },
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break,
        }

        let (speed, event) = axis.step(sample.position, sample.stalled, now, dt);
        if let Some(event) = event {
            report_event(event, &axis, sample.position);
        }
        command_speed(&mut el7037, speed);

        if cycle_count.is_multiple_of(STATUS_EVERY) {
            print!("\r{:<108}", status_line(&axis, &sample, speed));
            let _ = std::io::stdout().flush();
        }
        cycle_count = cycle_count.wrapping_add(1);
    }

    // Ramp to a stop rather than dropping the drive at speed.
    while !axis.stopped() {
        cycle(&mut el7037);
        let speed = axis.ramp_down(CYCLE_TIME.as_secs_f64());
        command_speed(&mut el7037, speed);
    }
    el7037.set_enabled(0, false);
    for _ in 0..50 {
        cycle(&mut el7037);
    }
    let _ = eth.channel.request_state_change(EtherCATState::PreOp);
    println!("\nstopped");
}

/// Commands a small positive speed and confirms the encoder counts up. See the
/// module docs for why this matters. Costs 400 ms.
fn direction_self_check(
    el7037: &mut EL7037,
    cycle: &mut impl FnMut(&mut EL7037),
) -> Result<(), String> {
    let before = el7037.get_position(0) as i64;
    let started = Instant::now();
    while started.elapsed() < PROBE_TIME {
        command_speed(el7037, PROBE_SPEED);
        cycle(el7037);
    }
    command_speed(el7037, 0.0);
    // Let it coast to a stop before reading, so the sign is not measured mid-ramp.
    for _ in 0..100 {
        cycle(el7037);
    }
    let travelled = el7037.get_position(0) as i64 - before;

    if (travelled as f64).abs() < PROBE_MIN_TRAVEL {
        return Err(format!(
            "commanded +{PROBE_SPEED:.0} counts/s but the encoder moved only {travelled:+} counts \
             - the shaft is not turning, or the encoder is not being read"
        ));
    }
    if travelled < 0 {
        return Err(format!(
            "commanded +{PROBE_SPEED:.0} counts/s but the encoder moved {travelled:+} counts"
        ));
    }
    println!("direction self-check ok (+{travelled} counts on a positive command)");
    Ok(())
}
