//! What does `motor_stall` (0x6010:08) actually tell you?
//!
//! Every EL7037 example so far treats `motor_stall` as a live "is losing steps
//! right now" indicator and prints `STALL` for as long as it reads true. Two
//! things about that have never been checked against hardware:
//!
//! 1. **Does it clear itself**, or does it stay latched once set until you
//!    explicitly reset it? The manual (`el70x7_en.pdf`) is silent - the object
//!    description just says "A loss of step has occurred" (past tense), and
//!    section 8.2 only clarifies that the *generic* "Ack. Message" diagnosis
//!    history button has no effect on the drive's own state machine or its
//!    error list.
//! 2. **Does a bare `Reset` pulse (0x7010:02) clear it**, or does it need a
//!    full `Enable` power-cycle? Prior sessions only ever tried the latter.
//!
//! If the bit latches and only clears on an enable cycle, then printing it as
//! a continuous flag is actively misleading in both directions at once: a
//! single transient slip early in a run makes `STALL` show for every cycle
//! after it, including ones with no step loss at all (a false positive by
//! construction) - and once latched, a *second, worse* stall later in the same
//! run produces no new signal, because the bit was already 1 (a false negative
//! for anything but the very first event).
//!
//! # What this measures
//!
//! Unlike every other example here, the TxPDO is **not** one of
//! `EL7037PredefinedPdoAssignment`'s variants. `EL7037TxPdo` and `EL7037RxPdo`
//! implement `Configuration` via the same derive macro the predefined
//! assignments use (see `ethercat_hal_derive::{TxPdo, RxPdo}`), so any
//! combination of their `Option` fields can be written directly - the
//! predefined enum is a convenience, not the only interface. This program maps
//! `ENC Status` (0x1A01) + `STM Status` (0x1A03) + `STM Internal Position`
//! (0x1A08) + `STM External Position` (0x1A09) together, none of which exclude
//! each other per the SM3 PDO assignment table in the manual. That combination
//! is not offered by any predefined variant, and it is what makes this
//! diagnostic possible: `STM Internal Position` (0x6010:14, the terminal's own
//! commanded microstep count) and `STM External Position` (0x6010:15, the
//! encoder as the STM object family reports it) are both readable at once,
//! alongside the raw `ENC` counter (0x6000:11) for cross-checking scale.
//!
//! # What it does
//!
//! 1. Configures the axis exactly like `el7037_velocity_closed_loop`, but with
//!    the custom PDO combination above.
//! 2. Runs the same continuous-feedback velocity profile from that example
//!    (encoder error drives the speed demand, arrival latch with hysteresis)
//!    for a fixed, unattended sequence of `+800`/`-800` moves - the exact move
//!    size measured earlier to stall on most attempts.
//! 3. Logs every cycle: the raw `motor_stall` bit, the encoder counter, and
//!    both STM position objects.
//! 4. At a fixed point mid-sequence, pulses `Reset` for a few cycles **without
//!    touching `Enable`**, to test whether that alone clears a latched stall.
//! 5. At the end, cycles `Enable` off and on once, the method already known to
//!    work, as a control.
//! 6. Prints an analysis: every rising edge of `motor_stall`, whether it ever
//!    fell on its own, whether the reset-only pulse cleared it, whether the
//!    enable cycle did, and the internal/external position delta measured at
//!    each edge.
//!
//! Usage: `cargo run --example el7037_stall_diagnostic -- <network-interface>`

use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::Configuration,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el7037::{
            EL7037, EL7037_PRODUCT_ID,
            coe::EL7037Configuration,
            pdo::{EL7037PredefinedPdoAssignment, EL7037RxPdo, EL7037TxPdo},
        },
    },
    init_ethercat,
    pdo::el70x7::{
        EncControl, EncStatus, StmControl, StmExternalPosition, StmInternalPosition, StmStatus,
        StmSynchronInfoData, StmVelocity,
    },
    shared_config::el70x7::{
        EL70x1OperationMode, EL70x1SpeedRange, EL70x7InfoData, EL7037FeedbackType,
    },
};
use std::{env, time::Duration};

const CYCLE_TIME: Duration = Duration::from_millis(2);
const MOTOR_FULL_STEPS: u16 = 200;
const ENCODER_INCREMENTS: u16 = 4000;
const COUNTS_PER_FULL_STEP: f64 = ENCODER_INCREMENTS as f64 / MOTOR_FULL_STEPS as f64;
const MAX_FULL_STEPS_PER_S: f64 = 2000.0;

// Same control law as el7037_velocity_closed_loop.rs, copied rather than
// imported - examples do not share a lib crate, and duplicating ~15 lines of
// a stable, already-verified control law is simpler than restructuring the
// crate to expose it.
const DEFAULT_MAX_SPEED: f64 = 8000.0;
const ACCELERATION: f64 = 40_000.0;
const APPROACH_GAIN: f64 = 25.0;
const MIN_SPEED: f64 = 100.0;
const FINE_TOLERANCE: i64 = 3;
const TOLERANCE: i64 = 20;
const DWELL: Duration = Duration::from_millis(120);
const RUNAWAY_FACTOR: f64 = 3.0;
const RUNAWAY_SLACK: f64 = 400.0;
const MOVE_TIMEOUT: Duration = Duration::from_secs(6);

/// The move sequence: alternating relative steps, in counts. 800 counts = 0.2
/// rev, measured earlier to stall on most attempts at the default profile.
const MOVES: [i64; 4] = [8000, -8000, 8000, -8000];
/// Index into `MOVES` (0-based, counting completed moves) after which the
/// reset-only experiment runs, if a stall has been observed by then.
const RESET_EXPERIMENT_AFTER_MOVE: usize = 2;
/// See el7037_velocity_closed_loop.rs: gives the deceleration law a built-in
/// margin instead of gating on `motor_stall` (rejected - that bit reads true
/// for ~98% of an 8000-count move's cruise phase here, not just the overshoot
/// window, so gating on it just crippled every move).
const BRAKING_ACCELERATION: f64 = ACCELERATION / 3.0;
/// How long to log every single cycle (not sampled) after each arrival,
/// looking for a twitch too fast for sampled status-line logging to catch.
const DWELL_SCAN: Duration = Duration::from_secs(3);

fn axis_config() -> EL7037Configuration {
    let mut config = EL7037Configuration::default();
    config.stm_features.operation_mode = EL70x1OperationMode::DirectVelocity;
    config.stm_features.speed_range = EL70x1SpeedRange::Steps2000;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    config.stm_features.invert_motor_polarity = false;
    // 0x9010:01, packing the 0xA010 STM diag bits cyclically - the STM
    // equivalent of what DriveStatusWord (150) does for the POS side.
    config.stm_features.select_info_data_1 = EL70x7InfoData::StatusWord;
    config.stm_features.select_info_data_2 = EL70x7InfoData::MotorDcCurrent;
    // Measured on this rig: false counts opposite to the commanded direction.
    config.encoder.reversion_of_rotation = true;
    config.stm_motor.max_current = 1100;
    config.stm_motor.reduced_current = 550;
    config.stm_motor.nominal_voltage = 24000;
    config.stm_motor.motor_coil_resistance = 175;
    config.stm_motor.motor_coil_inductance = 330;
    config.stm_motor.motor_full_steps = MOTOR_FULL_STEPS;
    config.stm_motor.encoder_increments = ENCODER_INCREMENTS;
    config.stm_motor.motor_emf = 0;
    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = 10;
    config.stm_controller_3.feed_forward_pos = 100_000;
    config.stm_controller_3.kp_factor_pos = 2;
    config.stm_controller_3.kp_factor_velo = 50;
    config.stm_controller_3.tn_velo = 50_000;
    // The PDO assignment field is set but unused for the actual wire
    // configuration below - kept only so `EL7037Configuration` stays valid to
    // construct. See `custom_txpdo`/`custom_rxpdo`.
    config.pdo_assignment = EL7037PredefinedPdoAssignment::VelocityControl;
    config
}

/// `ENC Status` (0x1A01) + `STM Status` (0x1A03) + `STM Internal Position`
/// (0x1A08) + `STM External Position` (0x1A09). Not offered by any predefined
/// variant; see the module docs for why this combination and why it is valid.
fn custom_txpdo() -> EL7037TxPdo {
    EL7037TxPdo {
        enc_status_compact: None,
        enc_status: Some(EncStatus::default()),
        enc_timestamp_compact: None,
        stm_status: Some(StmStatus::default()),
        stm_synchron_info_data: Some(StmSynchronInfoData::default()),
        pos_status_compact: None,
        pos_status: None,
        stm_internal_position: Some(StmInternalPosition::default()),
        stm_external_position: Some(StmExternalPosition::default()),
        pos_actual_position_lag: None,
    }
}

/// `ENC Control` (0x1601) + `STM Control` (0x1602) + `STM Velocity` (0x1604) -
/// the same RxPDO as the `VelocityControl` predefined assignment.
fn custom_rxpdo() -> EL7037RxPdo {
    EL7037RxPdo {
        enc_control_compact: None,
        enc_control: Some(EncControl::default()),
        stm_control: Some(StmControl::default()),
        stm_position: None,
        stm_velocity: Some(StmVelocity::default()),
        pos_control_compact: None,
        pos_control: None,
        pos_control_2: None,
    }
}

fn counts_to_full_steps_per_s(counts_per_s: f64) -> f64 {
    (counts_per_s / COUNTS_PER_FULL_STEP).clamp(-MAX_FULL_STEPS_PER_S, MAX_FULL_STEPS_PER_S)
}

/// The velocity profile from el7037_velocity_closed_loop.rs, copied verbatim.
struct Profile {
    speed: f64,
    max_speed: f64,
}

impl Profile {
    fn new() -> Self {
        Self {
            speed: 0.0,
            max_speed: DEFAULT_MAX_SPEED,
        }
    }

    fn step(&mut self, error: Option<i64>, dt: f64) -> f64 {
        let demand = match error {
            None => 0.0,
            Some(0) => 0.0,
            Some(error) => {
                let distance = error.abs() as f64;
                let stoppable = (2.0 * BRAKING_ACCELERATION * distance).sqrt();
                let gentle = APPROACH_GAIN * distance;
                let magnitude = stoppable.min(gentle).min(self.max_speed);
                let magnitude = if error.abs() > FINE_TOLERANCE {
                    magnitude.max(MIN_SPEED)
                } else {
                    magnitude
                };
                magnitude * (error.signum() as f64)
            }
        };
        let slew = ACCELERATION * dt;
        self.speed += (demand - self.speed).clamp(-slew, slew);
        if demand == 0.0 && self.speed.abs() < MIN_SPEED / 2.0 {
            self.speed = 0.0;
        }
        self.speed
    }

    fn stopped(&self) -> bool {
        self.speed == 0.0
    }
}

/// One cycle's worth of everything this diagnostic cares about.
#[derive(Debug, Clone, Copy)]
struct Sample {
    t_ms: u128,
    encoder: i32,
    internal_position: u32,
    external_position: u32,
    stall: bool,
    warning: bool,
    error: bool,
    status_word: u16,
    enable: bool,
    reset: bool,
}

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

    // Configure everything except the PDO assignment through the normal path,
    // then overwrite the PDO assignment with the custom combination.
    let config = axis_config();
    Configuration::write_config(&config, eth.channel.clone(), address)
        .expect("EL7037 CoE config failed");
    let txpdo = custom_txpdo();
    let rxpdo = custom_rxpdo();
    Configuration::write_config(&txpdo, eth.channel.clone(), address)
        .expect("custom TxPDO assignment failed - see module docs for the object combination");
    Configuration::write_config(&rxpdo, eth.channel.clone(), address)
        .expect("custom RxPDO assignment failed");
    el7037.configuration = config;
    el7037.txpdo = txpdo;
    el7037.rxpdo = rxpdo;
    println!(
        "EL7037 at {address}, custom PDO: ENC Status + STM Status + STM Internal/External Position"
    );

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

    let mut enable = false;
    let mut reset = false;
    let mut set_counter = false;
    let mut velocity_out: i16 = 0;

    let mut cycle = |el7037: &mut EL7037,
                     enable: bool,
                     reset: bool,
                     set_counter: bool,
                     velocity: i16|
     -> Option<Sample> {
        let mut sample = None;
        if let Some(input) = eth.app_handle.get_inputs() {
            let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(&input[start_tx..end_tx]));
            let enc = el7037
                .txpdo
                .enc_status
                .as_ref()
                .expect("ENC Status missing");
            let stm = el7037
                .txpdo
                .stm_status
                .as_ref()
                .expect("STM Status missing");
            let internal = el7037
                .txpdo
                .stm_internal_position
                .as_ref()
                .expect("STM Internal Position missing");
            let external = el7037
                .txpdo
                .stm_external_position
                .as_ref()
                .expect("STM External Position missing");
            sample = Some(Sample {
                t_ms: 0,
                encoder: enc.counter_value as i32,
                internal_position: internal.internal_position,
                external_position: external.external_position,
                stall: stm.motor_stall,
                warning: stm.warning,
                error: stm.error,
                status_word: el7037
                    .txpdo
                    .stm_synchron_info_data
                    .as_ref()
                    .map(|i| i.info_data_1)
                    .unwrap_or(0),
                enable,
                reset,
            });
            eth.app_handle.finish_read();
        }

        let stm_control = el7037
            .rxpdo
            .stm_control
            .as_mut()
            .expect("STM Control missing");
        stm_control.enable = enable;
        stm_control.reset = reset;
        stm_control.reduce_torque = false;
        let stm_velocity = el7037
            .rxpdo
            .stm_velocity
            .as_mut()
            .expect("STM Velocity missing");
        stm_velocity.velocity = velocity;
        let enc_control = el7037
            .rxpdo
            .enc_control
            .as_mut()
            .expect("ENC Control missing");
        enc_control.set_counter = set_counter;
        enc_control.set_counter_value = 0;

        if let Some(output) = eth.app_handle.write_outputs() {
            let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(
                &mut output[start_rx..end_rx],
            ));
            eth.app_handle.send_outputs();
        }
        std::thread::sleep(CYCLE_TIME);
        sample
    };

    // Bring the drive up.
    let start = std::time::Instant::now();
    let deadline = start + Duration::from_secs(8);
    loop {
        if let Some(s) = cycle(&mut el7037, enable, reset, set_counter, velocity_out) {
            if s.error {
                reset = true;
            } else {
                reset = false;
                enable = true;
            }
        }
        if std::time::Instant::now() > deadline {
            panic!("drive never became ready");
        }
        // ready_to_enable/ready are on StmStatus but not in this Sample's
        // struct - reuse the raw txpdo directly for the readiness check.
        if let Some(status) = el7037.txpdo.stm_status.as_ref()
            && status.ready
            && enable
        {
            break;
        }
    }
    println!("driver stage ready\n");

    // Home so the log's encoder column starts at 0. Internal/external STM
    // position are not affected by ENC's set-counter - they are logged as
    // deltas from their own starting values, not from zero.
    set_counter = true;
    for _ in 0..50 {
        cycle(&mut el7037, enable, reset, set_counter, velocity_out);
    }
    set_counter = false;
    for _ in 0..20 {
        cycle(&mut el7037, enable, reset, set_counter, velocity_out);
    }

    let mut profile = Profile::new();
    let mut log: Vec<Sample> = Vec::with_capacity(20_000);
    let mut reset_experiment_done = false;
    let mut reset_experiment_stall_before: Option<bool> = None;
    let mut reset_experiment_stall_after: Option<bool> = None;
    let mut self_cleared = false;
    let mut last_stall = false;

    for (index, &delta) in MOVES.iter().enumerate() {
        let move_started = std::time::Instant::now();
        let move_log_start = log.len();
        let mut position = el7037
            .txpdo
            .enc_status
            .as_ref()
            .map(|e| e.counter_value as i64)
            .unwrap_or(0);
        let from = position;
        let target = from + delta;
        let limit = delta.unsigned_abs() as f64 * RUNAWAY_FACTOR + RUNAWAY_SLACK;
        println!("move {index}: {delta:+} counts, {from} -> {target}");

        let mut in_window_since = None;
        let mut last = std::time::Instant::now();
        loop {
            let Some(sample) = cycle(&mut el7037, enable, reset, set_counter, velocity_out) else {
                continue;
            };
            let now = std::time::Instant::now();
            let dt = now.duration_since(last).as_secs_f64().clamp(0.0005, 0.05);
            last = now;

            let mut sample = sample;
            sample.t_ms = now.duration_since(start).as_millis();
            position = sample.encoder as i64;

            if sample.stall && !last_stall {
                println!(
                    "  rising edge: motor_stall at t={} ms, encoder={}, internal={}, external={}",
                    sample.t_ms, sample.encoder, sample.internal_position, sample.external_position
                );
            }
            if !sample.stall && last_stall && enable && !reset {
                self_cleared = true;
                println!("  motor_stall CLEARED ON ITS OWN at t={} ms", sample.t_ms);
            }
            last_stall = sample.stall;
            log.push(sample);

            let error = target - position;
            if (position - from).abs() as f64 > limit {
                println!("  RUNAWAY, aborting move");
                break;
            }
            if error.abs() <= TOLERANCE {
                let since = in_window_since.get_or_insert(now);
                if now.duration_since(*since) >= DWELL && profile.stopped() {
                    println!(
                        "  arrived at {position}, residual {:+}, {} ms{}",
                        -error,
                        now.duration_since(move_started).as_millis(),
                        if sample.stall {
                            " (motor_stall set)"
                        } else {
                            ""
                        }
                    );
                    break;
                }
            } else {
                in_window_since = None;
                if now.duration_since(move_started) > MOVE_TIMEOUT {
                    println!("  timed out {:+} counts short", -error);
                    break;
                }
            }

            let speed = profile.step(Some(error), dt);
            velocity_out =
                ethercat_hal::helpers::el70xx_velocity_converter::EL70x1VelocityConverter::new(
                    &EL70x1SpeedRange::Steps2000,
                )
                .steps_to_velocity(counts_to_full_steps_per_s(speed), speed.abs() >= 400.0);
        }

        // Did the approach overshoot the target and correct back? Walk this
        // move's full per-cycle log (not sampled - every cycle is here) and
        // find the point of maximum travel-direction overshoot, then print a
        // window around it so the actual trajectory is visible rather than
        // just the residual at the moment "arrived" fired.
        {
            let forwards = delta > 0;
            let approach = &log[move_log_start..];
            let mut peak = (0i64, 0usize);
            for (i, s) in approach.iter().enumerate() {
                let past = if forwards {
                    s.encoder as i64 - target
                } else {
                    target - s.encoder as i64
                };
                if past > peak.0 {
                    peak = (past, i);
                }
            }
            if peak.0 > 0 {
                println!(
                    "  overshoot: {} counts past target at t={} ms (cycle {} of {})",
                    peak.0,
                    approach[peak.1].t_ms,
                    peak.1,
                    approach.len()
                );
                let window_start = peak.1.saturating_sub(8);
                let window_end = (peak.1 + 12).min(approach.len());
                for s in &approach[window_start..window_end] {
                    let marker = if s.encoder as i64 == approach[peak.1].encoder as i64 {
                        " <-- peak"
                    } else {
                        ""
                    };
                    println!(
                        "    t={:>6} ms  encoder={:>+6}  target-encoder={:>+5}  stall={}{}",
                        s.t_ms,
                        s.encoder,
                        target - s.encoder as i64,
                        s.stall,
                        marker
                    );
                }
            }
        }

        // Ramp fully to a stop between moves so each one starts from rest.
        while !profile.stopped() {
            if let Some(mut sample) = cycle(&mut el7037, enable, reset, set_counter, velocity_out) {
                sample.t_ms = std::time::Instant::now().duration_since(start).as_millis();
                log.push(sample);
            }
            let speed = profile.step(None, CYCLE_TIME.as_secs_f64());
            velocity_out =
                ethercat_hal::helpers::el70xx_velocity_converter::EL70x1VelocityConverter::new(
                    &EL70x1SpeedRange::Steps2000,
                )
                .steps_to_velocity(counts_to_full_steps_per_s(speed), false);
        }
        velocity_out = 0;

        // Full-resolution dwell: log every single cycle (not sampled) for a
        // few seconds after arrival with the axis commanded stationary. A
        // twitch faster than one status-line print interval (12.5 ms at
        // STATUS_EVERY=50 in the interactive examples) would be invisible to
        // sampled logging even though it is visible in person; this cannot
        // miss anything, since every cycle is captured.
        let dwell_start_index = log.len();
        let dwell_deadline = std::time::Instant::now() + DWELL_SCAN;
        while std::time::Instant::now() < dwell_deadline {
            if let Some(mut sample) = cycle(&mut el7037, enable, reset, set_counter, 0) {
                sample.t_ms = std::time::Instant::now().duration_since(start).as_millis();
                log.push(sample);
            }
        }
        let dwell = &log[dwell_start_index..];
        if let Some(first) = dwell.first() {
            let baseline = first.encoder;
            let max_dev = dwell
                .iter()
                .map(|s| (s.encoder - baseline).abs())
                .max()
                .unwrap_or(0);
            if max_dev > 0 {
                println!(
                    "  DWELL: {} counts max deviation from {baseline} over {} cycles \
                     ({:.1} s, full resolution)",
                    max_dev,
                    dwell.len(),
                    DWELL_SCAN.as_secs_f64()
                );
            } else {
                println!(
                    "  DWELL: rock steady at {baseline} over {} cycles ({:.1} s, full resolution)",
                    dwell.len(),
                    DWELL_SCAN.as_secs_f64()
                );
            }
        }

        if index == RESET_EXPERIMENT_AFTER_MOVE && last_stall && !reset_experiment_done {
            reset_experiment_done = true;
            reset_experiment_stall_before = Some(last_stall);
            println!("\n  --- reset-only experiment: pulsing Reset, Enable stays high ---");
            reset = true;
            for _ in 0..10 {
                cycle(&mut el7037, enable, reset, set_counter, velocity_out);
            }
            reset = false;
            for _ in 0..20 {
                if let Some(s) = cycle(&mut el7037, enable, reset, set_counter, velocity_out) {
                    reset_experiment_stall_after = Some(s.stall);
                }
            }
            println!(
                "  motor_stall before: {:?}, after Reset-only: {:?}\n",
                reset_experiment_stall_before, reset_experiment_stall_after
            );
        }
    }

    let stall_before_enable_cycle = last_stall;
    println!("\n--- control: cycling Enable, the method already known to clear it ---");
    enable = false;
    for _ in 0..60 {
        cycle(&mut el7037, enable, reset, set_counter, velocity_out);
    }
    enable = true;
    let mut stall_after_enable_cycle = false;
    for _ in 0..60 {
        if let Some(s) = cycle(&mut el7037, enable, reset, set_counter, velocity_out) {
            stall_after_enable_cycle = s.stall;
        }
    }
    println!(
        "motor_stall before: {stall_before_enable_cycle}, after Enable cycle: {stall_after_enable_cycle}"
    );

    enable = false;
    for _ in 0..30 {
        cycle(&mut el7037, enable, reset, set_counter, velocity_out);
    }
    let _ = eth.channel.request_state_change(EtherCATState::PreOp);

    // ── Analysis ─────────────────────────────────────────────────────────
    println!("\n=== analysis ===\n");
    println!("{} cycles logged over {} moves", log.len(), MOVES.len());

    let mut edges = Vec::new();
    let mut prev = false;
    for s in &log {
        if s.stall && !prev {
            edges.push(*s);
        }
        prev = s.stall;
    }
    println!("rising edges of motor_stall: {}", edges.len());
    for (i, e) in edges.iter().enumerate() {
        println!(
            "  edge {i}: t={} ms  encoder={}  internal={}  external={}  (internal-external={})",
            e.t_ms,
            e.encoder,
            e.internal_position,
            e.external_position,
            e.internal_position.wrapping_sub(e.external_position) as i32
        );
    }

    let warnings_logged = log.iter().filter(|s| s.warning).count();
    let errors_logged = log.iter().filter(|s| s.error).count();
    let cycles_enabled = log.iter().filter(|s| s.enable).count();
    let cycles_resetting = log.iter().filter(|s| s.reset).count();
    println!(
        "cycles with warning set: {warnings_logged}, with error set: {errors_logged}, \
         enabled: {cycles_enabled}/{}, resetting: {cycles_resetting}",
        log.len()
    );

    // 0xA010 packed into 0x9010:01 (bit n = subindex n+1), same convention as
    // the POS diag word. Histogram which combination of bits is actually live
    // during "warning" cycles.
    let names = [
        "saturated",
        "over_temp",
        "torque_overload",
        "under_voltage",
        "over_voltage",
        "short_circuit",
        "bit6",
        "no_control_power",
        "misc_error",
        "config_not_adopted",
        "motor_stall",
    ];
    let mut counts = std::collections::BTreeMap::new();
    for s in log.iter().filter(|s| s.warning) {
        *counts.entry(s.status_word).or_insert(0u32) += 1;
    }
    println!("\ndistinct STM status words (0x9010:01) while `warning` was set:");
    for (word, count) in counts {
        let bits: Vec<&str> = names
            .iter()
            .enumerate()
            .filter(|(i, _)| word & (1 << i) != 0)
            .map(|(_, n)| *n)
            .collect();
        println!("  {word:#06x} ({count:>5} cycles): {}", bits.join("+"));
    }
    println!(
        "\nself-cleared without Reset/Enable: {}",
        if self_cleared { "YES" } else { "no" }
    );
    println!(
        "cleared by Reset alone (Enable held high): {:?}",
        match (reset_experiment_stall_before, reset_experiment_stall_after) {
            (Some(true), Some(false)) => "YES".to_string(),
            (Some(true), Some(true)) => "no".to_string(),
            _ => "not tested (no stall observed by the experiment point)".to_string(),
        }
    );
    println!(
        "cleared by an Enable cycle: {}",
        if stall_before_enable_cycle && !stall_after_enable_cycle {
            "YES"
        } else if !stall_before_enable_cycle {
            "not tested (no stall was latched at that point)"
        } else {
            "no"
        }
    );

    // Scale check: how much does STM External Position move per ENC counter
    // tick? If it tracks 1:1 with the raw ENC counter it is the same physical
    // signal at the same scale; if it moves 3.2x faster, it is expressed in
    // the same microstep units as STM Internal Position (12800/rev vs
    // 4000/rev = 3.2x), which is what would make internal-external directly
    // meaningful as a following-error measurement.
    if log.len() > 200 {
        let a = &log[50];
        let b = &log[log.len() - 50];
        let d_enc = (b.encoder - a.encoder) as f64;
        let d_ext = b.external_position.wrapping_sub(a.external_position) as i32 as f64;
        if d_enc.abs() > 50.0 {
            println!(
                "\nSTM External Position moved {:.2}x per ENC counter tick over the run \
                 (d_external={d_ext:.0}, d_enc={d_enc:.0})",
                d_ext / d_enc
            );
        }
    }
}
