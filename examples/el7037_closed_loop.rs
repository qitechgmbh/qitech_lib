//! Closed-loop point-to-point positioning on a Beckhoff EL7037 stepper terminal.
//!
//! You type a target position; the *terminal* drives there. The EL7037 owns both
//! the travel command generator (accelerate / constant / decelerate ramps,
//! in-target detection) and the position control loop, and it closes that loop
//! against the external encoder. This program only issues travel commands and
//! reports status - it never corrects the position in software.
//!
//! # How it is configured
//!
//! | what | value | why |
//! |---|---|---|
//! | Operating mode `0x8012:01` | `Position controller` (3) | `Automatic` (0) would infer the same mode from the PDO assignment; writing it explicitly removes the guesswork. `Ext. Position mode` (5) is Beckhoff-AS10xx-only. |
//! | Predefined PDO assignment | `Positioning interface with info data` | `0x1601 + 0x1602 + 0x1606` / `0x1A01 + 0x1A03 + 0x1A04 + 0x1A07`. The full `POS Control` object carries target position *plus* per-move velocity, start type, acceleration and deceleration; the compact variant (`0x1605`) carries only the target position. The info data adds the terminal's own state machine to the cyclic data. |
//! | Feedback type `0x8012:08` | `Encoder` (0) | This is what makes the loop closed on the real shaft. All positions are then in **encoder counts**, not microsteps. |
//!
//! # `motor_stall` is a pulse, not a latch
//!
//! Measured with `examples/el7037_stall_diagnostic`: `motor_stall` (0x6010:08)
//! clears itself within 50-350 ms with no `Reset` and no `Enable` cycle, and
//! at the moment it fires the commanded microstep count and the encoder are
//! already within a few counts of each other. It reads as the terminal's own
//! step generator briefly disagreeing with the encoder near the resolution of
//! one encoder count, not as a large loss-of-steps event. A live boolean is
//! easy to miss between pulses, so this program counts rising edges per move
//! (`Axis::stall_pulses`) and reports the count when the move completes.
//!
//! # The travel command handshake
//!
//! Beckhoff splits this into four stages (EL70x7 manual, "Standard sequence of a
//! travel command"). Three rules are easy to get wrong:
//!
//! * **Never drop `execute` while the terminal is still pulling into the target.**
//!   A falling edge on `0x7020:01` *aborts* the command. Hold it high until
//!   `busy` is low **and** `in_target` is high.
//! * **The terminal also reacts to a change of `start_type`**, not only to the
//!   execute edge - so park `start_type` at `Idle` whenever nothing is running.
//! * `POS Status.actual_position` (`0x6020:11`) is the **generator's** position,
//!   not a measurement. The measurement is `ENC Status.counter_value`
//!   (`0x6000:11`). This program shows both, and their difference.
//!
//! # Units
//!
//! | quantity | unit |
//! |---|---|
//! | target position (`0x7020:11`) | encoder counts |
//! | velocity (`0x7020:21`, `0x8020:01/02`) | `0..10000` = `0..100 %` of the speed range `0x8012:05` |
//! | acceleration / deceleration (`0x7020:23/24`) | **ms from 0 to 100 % of the speed range** |
//! | target window (`0x8020:0B`) | encoder counts |
//!
//! With the speed range at 2000 full steps/s and a 1000 ppr encoder
//! (4-fold = 4000 counts/rev), one full step is 20 encoder counts and a
//! velocity of 2000 is 400 full steps/s = 8000 counts/s.
//!
//! # Commands (read from stdin, so runs can be scripted)
//!
//! ```text
//! 2000      move to absolute encoder count 2000
//! a -2000   move to absolute count -2000 (bare numbers cannot carry a sign)
//! +800      move 800 counts in the positive direction
//! -800      move 800 counts in the negative direction
//! 1.5r      move to absolute 1.5 revolutions;  +0.5r / -0.5r are relative
//! v 2000    travel velocity, 0..10000
//! h         home here - set the encoder counter to 0
//! s         emergency stop
//! e         toggle the driver stage
//! c         clear a latched error
//! ?         command help
//! q         quit (drops execute, then enable, then leaves OP)
//! ```
//!
//! # Before you run this
//!
//! CoE objects are **persistent in the terminal**: anything a previous session or
//! TwinCAT wrote stays there, and an object this program does not write is
//! whatever somebody left behind. Run
//! `cargo run --example el7037_coe_dump -- <interface>` first to see what the
//! terminal is actually holding.
//!
//! Usage: `cargo run --example el7037_closed_loop -- <network-interface>`

use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el7037::{
            EL7037, EL7037_PRODUCT_ID, coe::EL7037Configuration, pdo::EL7037PredefinedPdoAssignment,
        },
    },
    init_ethercat,
    shared_config::{
        el70x1::StartType,
        el70x7::{EL70x1OperationMode, EL70x1SpeedRange, EL70x7InfoData, EL7037FeedbackType},
    },
};
use std::{
    env,
    io::Write,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

// ── Tunables ────────────────────────────────────────────────────────────────

/// Cycle time of this program. The terminal's own control cycle is a fixed
/// 250 us, so some of its internal states last less than one cycle here and are
/// never observed - that is expected and the state machine below only reacts to
/// `busy` / `in_target`, never to a specific internal state.
const CYCLE_TIME: Duration = Duration::from_nanos(250_000);
/// How often the status line is repainted, in cycles.
const STATUS_EVERY: u32 = 50;

// Motor: igus MOT-AN-S-060-005-042-L-C-AAAO (drylin E, NEMA 17).
/// `0x8010:06` - 1.8 deg step angle.
const MOTOR_FULL_STEPS: u16 = 200;
/// `0x8010:07` - 1000 pulses/rev with 4-fold evaluation.
const ENCODER_INCREMENTS: u16 = 4000;
/// `0x8010:01` - the motor's *continuous* rating, not its 1.8 A peak.
const MAX_CURRENT_MA: u16 = 1100;
/// `0x8010:02` - standstill current, halves the dissipation (and the torque).
const REDUCED_CURRENT_MA: u16 = 550;
/// `0x8010:03` - the EL7037 is a 24 V terminal. The HAL default of 50 V is the
/// EL7041's. Written to the terminal in 10 mV units by the HAL.
const NOMINAL_VOLTAGE_MV: u16 = 24000;
/// `0x8010:04` - 1.75 ohm in units of 10 mOhm.
const COIL_RESISTANCE: u16 = 175;
/// `0x8010:0A` - 3.30 mH in units of 0.01 mH. The HAL default of 0 is never
/// right for a real motor.
const COIL_INDUCTANCE: u16 = 330;

/// `0x8012:01` - operating mode.
///
/// `PositionController` (3) is the safe choice: it drives the stepper open-loop
/// on its own generator and closes the position loop on the encoder, and it
/// explicitly supports third-party motors.
///
/// `ExtendedPositionController` (5) adds field-oriented control, which the
/// manual's advantage matrix credits with "step losses are avoided" and a load
/// angle held at 90 deg. Beckhoff restricts it to their own AS10xx motors and
/// requires an encoder of at least 4000 INC/360 deg, and it runs a commutation
/// determination on every enable - the shaft twitches a few degrees each way.
///
/// Tried on this rig with the igus motor: the commutation determination
/// oscillates the shaft over ~130 counts, never reaches `ready`, and latches a
/// drive error (`0x6010:04`). The third-party-motor restriction is real, not
/// advisory. Left here as a documented dead end.
const OPERATION_MODE: EL70x1OperationMode = EL70x1OperationMode::PositionController;

/// `0x8000:0E` - invert the encoder's counting direction.
///
/// The encoder and the motor must agree on which way is positive, or the
/// position loop feeds back with the wrong sign. Measured on this rig: with
/// this at `false`, a commanded +400 counts drove the generator to +400 while
/// the encoder read -407 - same magnitude, opposite sign, in both directions.
const REVERSION_OF_ROTATION: bool = true;
/// `0x8012:09` - invert the motor's direction of rotation instead. Use exactly
/// one of these two; setting both just restores the original disagreement.
const INVERT_MOTOR_POLARITY: bool = false;

/// `0x8012:05` - 100 % of velocity means this many full steps per second.
const SPEED_RANGE: EL70x1SpeedRange = EL70x1SpeedRange::Steps2000;
const SPEED_RANGE_FULL_STEPS_PER_S: f64 = 2000.0;

/// `0x8014:02` - Kp of the *position* controller, the loop that pulls the motor
/// into the target window.
///
/// The documented default is 500. On this axis that is wildly unstable. Swept
/// with `examples/el7037_tune`, overshoot and post-move creep both scale
/// monotonically with this gain:
///
/// | Kp | worst overshoot | creep | stalls |
/// |---|---|---|---|
/// | 1 | 64 counts | -0.2 c/s | 1 of 4 moves |
/// | 2 | 82 counts | -1.5 c/s | 3 of 4 |
/// | 5 | 168 counts | -7.9 c/s | 4 of 4 |
/// | 12 | 1069 counts | -113.9 c/s | 4 of 4 |
/// | 25+ | runaway, aborted by the guard | | |
///
/// So the low gain is deliberate, and 5 was still ~4x higher than it needed to
/// be. Do not "fix" this back to the documented value without re-measuring.
const KP_FACTOR_POS: u16 = 2;

/// `0x7020:21` - default travel velocity, 20 % of the speed range.
const TRAVEL_VELOCITY: i16 = 2000;
/// `0x7020:23` / `0x7020:24` - ms to ramp between 0 and 100 % of the speed range.
const ACCELERATION_MS: u16 = 500;
const DECELERATION_MS: u16 = 500;
/// `0x8020:01` - the crawl speed used once the deceleration ramp has ended. It
/// must be low enough that the motor can stop abruptly without losing a step.
const VELOCITY_MIN: i16 = 100;
/// `0x8020:0B` - in encoder counts. One full step is 20 counts here. The
/// terminal only reports `in_target` once it comes to rest inside this window,
/// so a window narrower than the axis's hunting amplitude leaves it in
/// PRE_TARGET until the timeout expires - which looks like the motor creeping
/// on its own.
const TARGET_WINDOW: u16 = 20;
/// `0x8020:0C` - if the target window is not reached within this, `busy` falls
/// with `in_target` still low. That is the only way to detect the condition.
const IN_TARGET_TIMEOUT_MS: u16 = 1000;

/// Abort and de-energise if the encoder travels past this multiple of the
/// commanded distance. This axis has a known fault where one physical direction
/// can run away; the guard is what makes running it safe.
const RUNAWAY_FACTOR: i64 = 3;
const RUNAWAY_SLACK: i64 = 200;

/// How long to wait after raising `execute` for the terminal to acknowledge
/// with `busy`. If it never does, the travel command was rejected.
const START_GRACE: Duration = Duration::from_millis(300);
/// Movement of more than this many counts per window while the axis is idle is
/// creep. The measured free-run on this rig is 45-100 counts/s, so the threshold
/// has to sit well below 500 ms worth of that.
const CREEP_COUNTS: i64 = 15;
const CREEP_WINDOW: Duration = Duration::from_millis(500);

// ── Units ───────────────────────────────────────────────────────────────────

const COUNTS_PER_REV: i64 = ENCODER_INCREMENTS as i64;

fn counts_to_revs(counts: i64) -> f64 {
    counts as f64 / COUNTS_PER_REV as f64
}

fn revs_to_counts(revs: f64) -> i64 {
    (revs * COUNTS_PER_REV as f64).round() as i64
}

/// A POS velocity of 10000 is 100 % of the speed range.
fn velocity_to_counts_per_s(velocity: i16) -> f64 {
    let full_steps_per_s = SPEED_RANGE_FULL_STEPS_PER_S * f64::from(velocity) / 10_000.0;
    full_steps_per_s * COUNTS_PER_REV as f64 / f64::from(MOTOR_FULL_STEPS)
}

// ── Configuration ───────────────────────────────────────────────────────────

/// Every CoE object this example depends on, written explicitly. Nothing is
/// left to whatever the terminal happens to be holding.
fn axis_config() -> EL7037Configuration {
    let mut config = EL7037Configuration::default();

    // 0x8012 - operating mode and feedback source.
    config.stm_features.operation_mode = OPERATION_MODE;
    config.stm_features.speed_range = SPEED_RANGE;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    config.stm_features.invert_motor_polarity = INVERT_MOTOR_POLARITY;
    // Info data 1 exposes 0x9020:03, the terminal's own positioning state
    // machine, in the cyclic data - far better than inferring it from bits.
    config.stm_features.select_info_data_1 = EL70x7InfoData::DriveState;
    // 0x9020:01 packs the whole 0xA020 POS diag object into one word (bit n =
    // subindex n+1). Mapping it cyclically means a failed move can be explained
    // without a blocking SDO read from inside the cyclic loop - which, measured,
    // times out and stalls the process image.
    config.stm_features.select_info_data_2 = EL70x7InfoData::DriveStatusWord;

    // 0x8000 - encoder.
    config.encoder.reversion_of_rotation = REVERSION_OF_ROTATION;

    // 0x8010 - motor plate data.
    config.stm_motor.max_current = MAX_CURRENT_MA;
    config.stm_motor.reduced_current = REDUCED_CURRENT_MA;
    config.stm_motor.nominal_voltage = NOMINAL_VOLTAGE_MV;
    config.stm_motor.motor_coil_resistance = COIL_RESISTANCE;
    config.stm_motor.motor_coil_inductance = COIL_INDUCTANCE;
    config.stm_motor.motor_full_steps = MOTOR_FULL_STEPS;
    config.stm_motor.encoder_increments = ENCODER_INCREMENTS;
    // The terminal's own default is 0; the HAL ships 200.
    config.stm_motor.motor_emf = 0;

    // 0x8011 - current loop. These are the terminal's documented defaults; the
    // HAL ships 400/4, which is a much stiffer P term with a weaker integrator.
    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = 10;

    // 0x8014 - position and velocity loop. This is the loop that settles a move.
    config.stm_controller_3.feed_forward_pos = 100_000;
    config.stm_controller_3.kp_factor_pos = KP_FACTOR_POS;
    config.stm_controller_3.kp_factor_velo = 50;
    config.stm_controller_3.tn_velo = 50_000;

    // 0x8020 - travel command generator. Velocity and the ramps are also sent
    // per move in the PDO; these are the fallbacks and the limits.
    config.pos_configuration.velocity_min = VELOCITY_MIN;
    config.pos_configuration.velocity_max = 10_000;
    config.pos_configuration.acceleration_pos = ACCELERATION_MS;
    config.pos_configuration.acceleration_neg = ACCELERATION_MS;
    config.pos_configuration.deceleration_pos = DECELERATION_MS;
    config.pos_configuration.deceleration_neg = DECELERATION_MS;
    config.pos_configuration.emergency_deceleration = 100;
    config.pos_configuration.target_window = TARGET_WINDOW;
    config.pos_configuration.in_target_timeout = IN_TARGET_TIMEOUT_MS;
    // Following error monitoring stays off: this axis would trip it, and an
    // emergency stop mid-move is worse than a reported overshoot.
    config.pos_configuration.position_lag_max = 0;

    // 0x8021 - `Set calibration manual auto` makes the terminal mark itself
    // calibrated on the first rising edge of `enable`, which is what an axis
    // without a reference cam needs in order to accept absolute travel commands.
    config.pos_features.start_type = StartType::CalibrationSetManualAuto;
    config.pos_features.emergency_stop_on_position_lag_error = false;

    // 0x1C12 / 0x1C13 - see the module docs for why this assignment.
    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionInterfaceWithInfoData;

    config
}

// ── Process data ────────────────────────────────────────────────────────────

/// One cycle's worth of inputs, widened and named.
#[derive(Debug, Clone, Copy)]
struct Sample {
    /// `0x6000:11` - the encoder. This is the measurement.
    encoder: i64,
    /// `0x6020:11` - where the travel command generator thinks it is.
    generator: i64,
    /// `0x6000:03`
    set_counter_done: bool,
    /// `0x6010:*`
    ready_to_enable: bool,
    ready: bool,
    stm_warning: bool,
    stm_error: bool,
    motor_stall: bool,
    /// `0x6020:*`
    busy: bool,
    in_target: bool,
    pos_warning: bool,
    pos_error: bool,
    calibrated: bool,
    accelerate: bool,
    decelerate: bool,
    /// `0x6020:21` - the generator's set velocity, 0..10000.
    set_velocity: i16,
    /// `0x6020:22`
    drive_time: u32,
    /// `0x6010:11` mapped to `0x9020:03` - the terminal's positioning state.
    drive_state: u16,
    /// `0x6010:12` mapped to `0x9020:01` - the 0xA020 POS diag bits.
    pos_diag: u16,
}

impl Sample {
    fn read(el7037: &EL7037) -> Self {
        let enc = el7037
            .txpdo
            .enc_status
            .as_ref()
            .expect("ENC Status missing - wrong PDO assignment");
        let stm = el7037
            .txpdo
            .stm_status
            .as_ref()
            .expect("STM Status missing - wrong PDO assignment");
        let pos = el7037
            .txpdo
            .pos_status
            .as_ref()
            .expect("POS Status missing - wrong PDO assignment");
        let info = el7037
            .txpdo
            .stm_synchron_info_data
            .as_ref()
            .expect("STM Synchron info data missing - wrong PDO assignment");

        Self {
            // The counter is 32 bit, so at 4000 counts/rev it takes about a
            // million revolutions to wrap. Sign-extending is enough here.
            encoder: i64::from(enc.counter_value as i32),
            generator: i64::from(pos.actual_position as i32),
            set_counter_done: enc.set_counter_done,
            ready_to_enable: stm.ready_to_enable,
            ready: stm.ready,
            stm_warning: stm.warning,
            stm_error: stm.error,
            motor_stall: stm.motor_stall,
            busy: pos.busy,
            in_target: pos.in_target,
            pos_warning: pos.warning,
            pos_error: pos.error,
            calibrated: pos.calibrated,
            accelerate: pos.accelerate,
            decelerate: pos.decelerate,
            set_velocity: pos.actual_velocity,
            drive_time: pos.actual_drive_time,
            drive_state: info.info_data_1,
            pos_diag: info.info_data_2,
        }
    }
}

/// One cycle's worth of outputs.
#[derive(Debug, Clone, Copy, Default)]
struct Outputs {
    enable: bool,
    reset: bool,
    set_counter: bool,
    execute: bool,
    emergency_stop: bool,
    target_position: u32,
    target_velocity: i16,
    start_type: u16,
}

/// `0x9020:03`. Taken from the object's permitted-value list rather than the
/// prose table in the same manual, which omits `Wait for init`.
fn drive_state_name(state: u16) -> &'static str {
    match state {
        0x0000 => "INIT",
        0x0001 => "IDLE",
        0x0010 => "START",
        0x0011 => "ACCEL",
        0x0012 => "CONST",
        0x0013 => "DECEL",
        0x0020 => "EMERGENCY_STOP",
        0x0021 => "STOP",
        0x0100 => "CALI_START",
        0x0110 => "CALI_GO_CAM",
        0x0111 => "CALI_ON_CAM",
        0x0120 => "CALI_GO_SYNC",
        0x0121 => "CALI_LEAVE_CAM",
        0x0130 => "CALI_STOP",
        0x0140 => "CALIBRATED",
        0x0141 => "NOT_CALIBRATED",
        0x1000 => "PRE_TARGET",
        0x1001 => "TARGET",
        0x1002 => "TARGET_RESTART",
        0x2000 => "END",
        0x2001 => "WAIT_FOR_INIT",
        0x4000 => "WARNING",
        0x8000 => "ERROR",
        0xFFFF => "UNDEFINED",
        _ => "?",
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Command {
    Absolute(i64),
    Relative(i64),
    Velocity(i16),
    Home,
    EmergencyStop,
    ToggleEnable,
    ClearError,
    Help,
    Quit,
}

const HELP: &str = "\
  <n>       move to absolute encoder count <n>
  a <n>     move to absolute count <n> (accepts a negative sign)
  +<n> -<n> move <n> counts relative to the current position
  <n>r      the same three forms in revolutions, e.g. 1.5r  +0.25r  a -2r
  v <n>     travel velocity, 0..10000 (10000 = 100 % of the speed range)
  h         home here - set the encoder counter to 0
  s         emergency stop
  e         toggle the driver stage
  c         clear a latched error
  ?         this help
  q         quit";

fn parse_command(line: &str) -> Result<Option<Command>, String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }
    match line {
        "q" | "quit" => return Ok(Some(Command::Quit)),
        "h" | "home" => return Ok(Some(Command::Home)),
        "s" | "stop" => return Ok(Some(Command::EmergencyStop)),
        "e" => return Ok(Some(Command::ToggleEnable)),
        "c" => return Ok(Some(Command::ClearError)),
        "?" | "help" => return Ok(Some(Command::Help)),
        _ => {}
    }

    if let Some(rest) = line.strip_prefix("v ") {
        let velocity: i32 = rest
            .trim()
            .parse()
            .map_err(|_| format!("not a number: {rest:?}"))?;
        if !(0..=10_000).contains(&velocity) {
            return Err(format!("velocity {velocity} is outside 0..10000"));
        }
        return Ok(Some(Command::Velocity(velocity as i16)));
    }

    // `a <n>` forces an absolute move, so a negative absolute target is
    // expressible without colliding with the relative `-<n>` form.
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
            revs_to_counts(revs)
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

/// Reads commands off stdin without blocking the cyclic loop. Line buffered, so
/// no raw terminal mode and no extra dependency is needed, and a run can be
/// driven from a shell pipeline.
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
            // Ctrl-D closes stdin; treat it like an explicit quit.
            let _ = sender.send(String::from("q"));
        })
        .expect("could not spawn the input thread");
    receiver
}

// ── The axis state machine ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Driver stage off, waiting for `ready_to_enable`.
    WaitReadyToEnable,
    /// `enable` raised, waiting for `ready` (drive-on delay, `0x8010:10`).
    WaitReady,
    /// Enabled and parked: `start_type` at `Idle`, `execute` low.
    Idle,
    /// Travel command parameters written with `execute` still low. Held for one
    /// full cycle so the terminal latches them before the rising edge.
    Arm,
    /// `execute` raised. Hold it - and everything else - until the terminal
    /// reports `busy` low *and* `in_target` high.
    Running,
    /// Deliberately de-energised (user, or a guard fired).
    Disabled,
    /// Latched fault; waits for `c`.
    Fault,
}

struct Axis {
    state: State,
    /// Pending travel command, target in absolute encoder counts.
    target: i64,
    velocity: i16,
    /// Set once `busy` has actually been observed for the current command. A
    /// move is only complete after that: `in_target` is still latched from the
    /// previous command at the moment `execute` goes high.
    seen_busy: bool,
    started: Instant,
    /// Encoder position when the current command started, for the runaway guard.
    start_encoder: i64,
    runaway_limit: i64,
    /// Set-counter request; held until the terminal acknowledges.
    homing: Option<Instant>,
    /// Emergency stop pulse, in cycles remaining.
    estop: u8,
    /// Error reset pulse, in cycles remaining.
    reset: u8,
    /// Startup error clears already attempted, so a genuinely stuck drive does
    /// not spin here forever.
    auto_resets: u8,
    /// Creep watchdog.
    creep_from: (i64, Instant),
    creep_reported: bool,
    /// Homing rewrites the counter, which looks exactly like a large jump to the
    /// creep watchdog. Hold it off until the new baseline has settled.
    creep_hold_until: Instant,
    /// `motor_stall` is a pulse, not a latch (measured with
    /// `examples/el7037_stall_diagnostic` - see that file's module docs), so a
    /// live boolean in the status line can miss it entirely. Count rising
    /// edges for the current move instead.
    stall_was_set: bool,
    stall_pulses: u32,
    quit: bool,
}

impl Axis {
    fn new(now: Instant) -> Self {
        Self {
            state: State::WaitReadyToEnable,
            target: 0,
            velocity: TRAVEL_VELOCITY,
            seen_busy: false,
            started: now,
            start_encoder: 0,
            runaway_limit: i64::MAX,
            homing: None,
            estop: 0,
            reset: 0,
            auto_resets: 0,
            creep_from: (0, now),
            creep_reported: false,
            creep_hold_until: now,
            stall_was_set: false,
            stall_pulses: 0,
            quit: false,
        }
    }

    fn energised(&self) -> bool {
        !matches!(
            self.state,
            State::WaitReadyToEnable | State::Disabled | State::Fault
        )
    }

    fn command(&mut self, command: Command, sample: &Sample, now: Instant) {
        match command {
            Command::Help => println!("{HELP}"),
            Command::Quit => self.quit = true,
            Command::Velocity(velocity) => {
                self.velocity = velocity;
                println!(
                    "velocity {velocity} = {:.0} counts/s",
                    velocity_to_counts_per_s(velocity)
                );
            }
            Command::ToggleEnable => match self.state {
                State::Disabled | State::Fault | State::WaitReadyToEnable => {
                    self.state = State::WaitReadyToEnable;
                    println!("enabling");
                }
                _ => {
                    self.state = State::Disabled;
                    println!("disabled");
                }
            },
            Command::ClearError => {
                // A rising edge on 0x7010:02 clears the latched drive errors.
                self.reset = 3;
                self.state = State::WaitReadyToEnable;
                println!("clearing error");
            }
            Command::EmergencyStop => {
                self.estop = 5;
                self.state = State::Idle;
                self.seen_busy = false;
                println!("emergency stop");
            }
            Command::Home => {
                if sample.busy || self.state == State::Running {
                    println!("refusing to home while a travel command is running");
                } else {
                    self.homing = Some(now);
                    println!("homing: setting the encoder counter to 0");
                }
            }
            Command::Absolute(target) => self.start_move(target, sample, now),
            Command::Relative(delta) => self.start_move(sample.encoder + delta, sample, now),
        }
    }

    fn start_move(&mut self, target: i64, sample: &Sample, now: Instant) {
        match self.state {
            State::Idle => {}
            State::Running => {
                println!("a travel command is still running; wait for it or press s");
                return;
            }
            _ => {
                println!("axis is not ready ({:?})", self.state);
                return;
            }
        }

        let delta = target - sample.encoder;
        if delta == 0 {
            println!("already at {target}");
            return;
        }

        self.target = target;
        self.seen_busy = false;
        self.started = now;
        self.start_encoder = sample.encoder;
        self.runaway_limit = delta.abs() * RUNAWAY_FACTOR + RUNAWAY_SLACK;
        self.stall_pulses = 0;
        self.state = State::Arm;
        println!(
            "move {:+} counts to {} ({:+.3} rev, {:.3} rev absolute) at {:.0} counts/s",
            delta,
            target,
            counts_to_revs(delta),
            counts_to_revs(target),
            velocity_to_counts_per_s(self.velocity),
        );
    }

    /// Advances the state machine by one cycle and returns what to write.
    fn step(&mut self, sample: &Sample, now: Instant) -> Outputs {
        if sample.motor_stall && !self.stall_was_set {
            self.stall_pulses += 1;
        }
        self.stall_was_set = sample.motor_stall;

        // Faults from the drive itself take priority over whatever is running.
        if sample.stm_error {
            match self.state {
                // A drive error latched by an earlier session survives a power
                // cycle and blocks `enable` forever. Beckhoff's StartUp stage is
                // "test the system and the ready status of the motor", so clear
                // it here rather than making every run start with a manual `c`.
                State::WaitReadyToEnable | State::WaitReady => {
                    if self.reset == 0 && self.auto_resets < 3 {
                        self.auto_resets += 1;
                        self.reset = 3;
                        println!(
                            "clearing a latched drive error (0x6010:04), attempt {}",
                            self.auto_resets
                        );
                    } else if self.auto_resets >= 3 {
                        println!();
                        println!("drive error (0x6010:04) will not clear");
                        report_stm_diag();
                        self.state = State::Fault;
                    }
                }
                State::Fault => {}
                // An error raised once the axis was live is a real fault.
                _ => {
                    println!();
                    println!("drive error (0x6010:04)");
                    report_stm_diag();
                    self.state = State::Fault;
                }
            }
        }

        match self.state {
            State::WaitReadyToEnable => {
                if sample.ready_to_enable {
                    self.state = State::WaitReady;
                }
            }
            State::WaitReady => {
                if sample.ready {
                    println!("driver stage ready");
                    self.auto_resets = 0;
                    self.state = State::Idle;
                }
            }
            State::Idle | State::Disabled | State::Fault => {}
            // Hold: `outputs()` writes the travel command parameters with
            // `execute` still low, so the terminal has latched all of them
            // before it sees the rising edge next cycle.
            State::Arm => {}
            State::Running => self.step_running(sample, now),
        }

        self.watch_creep(sample, now);
        let outputs = self.outputs(sample);

        // The arming cycle has now been written to the wire. Advance only after
        // that, so `execute` rises on the cycle *after* the parameters, never
        // together with them.
        if self.state == State::Arm {
            self.state = State::Running;
            // Measure the acknowledge grace from the execute edge itself.
            self.started = now;
        }

        outputs
    }

    fn step_running(&mut self, sample: &Sample, now: Instant) {
        if sample.busy {
            self.seen_busy = true;
        }

        let travelled = (sample.encoder - self.start_encoder).abs();
        if travelled > self.runaway_limit {
            println!();
            println!(
                "RUNAWAY: travelled {travelled} counts, limit {}. De-energising.",
                self.runaway_limit
            );
            self.state = State::Disabled;
            return;
        }

        if sample.pos_error {
            println!();
            println!("positioning error (0x6020:04)");
            report_pos_diag(sample.pos_diag);
            self.state = State::Fault;
            return;
        }

        if !self.seen_busy {
            if now.duration_since(self.started) > START_GRACE {
                // The terminal never acknowledged. Either the command was
                // rejected or the distance was below its resolution.
                println!();
                println!("travel command was not acknowledged within {START_GRACE:?}");
                report_pos_diag(sample.pos_diag);
                self.state = State::Idle;
            }
            return;
        }

        if sample.busy {
            return;
        }

        // `busy` has fallen. `execute` is still high, which is what the
        // documented sequence requires until in-target is confirmed.
        let error = sample.encoder - self.target;
        if sample.in_target {
            println!();
            println!(
                "in target: {} ({:.3} rev), residual {:+} counts, {} ms{}",
                sample.encoder,
                counts_to_revs(sample.encoder),
                error,
                sample.drive_time,
                if self.stall_pulses > 0 {
                    format!(" ({} stall pulse(s))", self.stall_pulses)
                } else {
                    String::new()
                }
            );
        } else {
            println!();
            println!(
                "in-target timeout: stopped at {} , {:+} counts short of {}",
                sample.encoder, -error, self.target
            );
            report_pos_diag(sample.pos_diag);
        }
        if sample.pos_warning && sample.in_target {
            // The timeout branch above has already dumped the diag word.
            report_pos_diag(sample.pos_diag);
        }
        self.state = State::Idle;
        self.creep_from = (sample.encoder, now);
        self.creep_reported = false;
    }

    /// The known post-move fault on this axis: the generator jumps forward to
    /// match the encoder, the terminal enters `WAIT_FOR_INIT`, and the shaft
    /// free-runs until `enable` is cycled. Reported, never silently corrected.
    fn watch_creep(&mut self, sample: &Sample, now: Instant) {
        if self.state != State::Idle || sample.busy || now < self.creep_hold_until {
            self.creep_from = (sample.encoder, now);
            return;
        }
        if now.duration_since(self.creep_from.1) < CREEP_WINDOW {
            return;
        }
        let drift = sample.encoder - self.creep_from.0;
        if drift.abs() > CREEP_COUNTS && !self.creep_reported {
            println!();
            println!(
                "CREEP: {drift:+} counts in {CREEP_WINDOW:?} with no travel command \
                 (state {}). Cycle `e` twice to re-sync the drive.",
                drive_state_name(sample.drive_state)
            );
            self.creep_reported = true;
        }
        self.creep_from = (sample.encoder, now);
    }

    fn outputs(&mut self, sample: &Sample) -> Outputs {
        let mut outputs = Outputs {
            enable: self.energised(),
            target_velocity: self.velocity,
            // Parked at Idle whenever nothing is running: the terminal starts a
            // command on a change of start type, not only on the execute edge.
            start_type: StartType::Idle as u16,
            ..Outputs::default()
        };

        if self.reset > 0 {
            self.reset -= 1;
            outputs.reset = true;
        }
        if self.estop > 0 {
            self.estop -= 1;
            outputs.emergency_stop = true;
        }

        if let Some(since) = self.homing {
            if sample.set_counter_done {
                self.homing = None;
                // The counter has just jumped to the new value; that is not creep.
                self.creep_hold_until = Instant::now() + CREEP_WINDOW * 2;
                println!("homed");
            } else if since.elapsed() > Duration::from_millis(500) {
                self.homing = None;
                println!("homing was not acknowledged by the terminal");
            } else {
                outputs.set_counter = true;
            }
        }

        match self.state {
            State::Arm => {
                // Parameters only; execute stays low for this whole cycle.
                outputs.target_position = self.target as u32;
                outputs.start_type = StartType::Absolute as u16;
            }
            State::Running => {
                outputs.target_position = self.target as u32;
                outputs.start_type = StartType::Absolute as u16;
                outputs.execute = true;
            }
            _ => {}
        }

        outputs
    }
}

// ── Diagnostics ─────────────────────────────────────────────────────────────

/// Decodes `0x9020:01` - the `0xA020` POS diag object packed into one word,
/// bit n being subindex n+1. It arrives in the cyclic data via info data 2, so
/// explaining a failed move costs no mailbox traffic.
fn report_pos_diag(pos_diag: u16) {
    let names = [
        "command rejected",
        "command aborted",
        "target overrun",
        "target timeout",
        "position lag",
        "emergency stop",
    ];
    let mut any = false;
    for (bit, name) in names.iter().enumerate() {
        if pos_diag & (1 << bit) != 0 {
            println!("  POS diag 0xA020:{:02X} {name}", bit + 1);
            any = true;
        }
    }
    if !any {
        println!("  POS diag 0x9020:01 = {pos_diag:#06x}, no known bit set");
    }
}

/// `0xA010` holds the latched driver-stage conditions, but reading it here is
/// the wrong move: an SDO is mailbox traffic, and issuing one from inside a 2 ms
/// cyclic loop measurably times out ("timed out waiting on channel") and stalls
/// the process image while it does. The POS diagnostics are in the cyclic data
/// instead (see `select_info_data_2`); there is no third info slot for the STM
/// ones, so point at the tool that can read them safely from PreOp.
fn report_stm_diag() {
    println!("  run `cargo run --example el7037_coe_dump -- <interface>` to read 0xA010");
}

fn status_line(axis: &Axis, sample: &Sample) -> String {
    let mut flags = String::new();
    for (set, name) in [
        (sample.busy, "busy"),
        (sample.in_target, "in-target"),
        (sample.calibrated, "cal"),
        (sample.accelerate, "accel"),
        (sample.decelerate, "decel"),
        (sample.pos_warning, "POS-WARN"),
        (sample.pos_error, "POS-ERR"),
        (sample.stm_warning, "STM-WARN"),
        (sample.stm_error, "STM-ERR"),
        (sample.motor_stall, "STALL"),
    ] {
        if set {
            flags.push(' ');
            flags.push_str(name);
        }
    }
    format!(
        "enc {:>+9} ({:>+8.3} rev)  gen {:>+9}  d {:>+6}  v {:>6}  {:?}/{}{}",
        sample.encoder,
        counts_to_revs(sample.encoder),
        sample.generator,
        sample.encoder - sample.generator,
        sample.set_velocity,
        axis.state,
        drive_state_name(sample.drive_state),
        flags,
    )
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        match eth_control.app_handle.get_state() {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    // CoE has to be written in PreOp, before the Op transition maps the PDOs.
    let mut el7037 = EL7037::new();
    let config = axis_config();

    // The subdevice list is not always populated the instant PreOp is reached,
    // and configuring nothing fails silently much later as a missing PDO object.
    let mut address = None;
    for _ in 0..50 {
        let subdevices = eth_control
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
        .write_config(eth_control.channel.clone(), address, &config)
        .expect("EL7037 CoE config failed");
    println!("EL7037 at address {address} configured for closed-loop positioning");

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        match eth_control.app_handle.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    // PDO offsets are only assigned during the Op transition, so the list
    // fetched in PreOp is stale.
    let subdevices = eth_control
        .app_handle
        .try_get_subdevices_vec_sync()
        .expect("Failed to read subdevices!");
    let subdevice = subdevices
        .iter()
        .find(|s| s.device_address == address)
        .copied()
        .expect("EL7037 disappeared during the Op transition");

    let commands = spawn_input_thread();
    let mut axis = Axis::new(Instant::now());
    let mut have_data = false;
    let mut cycle = 0u32;

    println!("{HELP}");

    loop {
        // --- Read (TxPDO: encoder, drive status, positioning status) ---
        if let Some(input) = eth_control.app_handle.get_inputs() {
            let tx_bytes = &input[subdevice.start_tx..subdevice.end_tx];
            let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(tx_bytes));
            have_data = true;
            // `input_post_process()` is skipped deliberately: it maintains the
            // counter wrapper for the *compact* encoder PDO, which this
            // assignment does not contain, and would only return an error here.
            eth_control.app_handle.finish_read();
        }

        // Until the first TxPDO has landed the PDO structs still hold their
        // defaults. Running the state machine on those would latch a target of
        // zero and report a position the axis is not at.
        if !have_data {
            std::thread::sleep(CYCLE_TIME);
            continue;
        }

        let now = Instant::now();
        let sample = Sample::read(&el7037);

        match commands.try_recv() {
            Ok(line) => match parse_command(&line) {
                Ok(Some(command)) => axis.command(command, &sample, now),
                Ok(None) => {}
                Err(message) => println!("{message}"),
            },
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => axis.quit = true,
        }

        if axis.quit {
            break;
        }

        let outputs = axis.step(&sample, now);

        if cycle.is_multiple_of(STATUS_EVERY) {
            print!("\r{:<118}", status_line(&axis, &sample));
            let _ = std::io::stdout().flush();
        }
        cycle = cycle.wrapping_add(1);

        // --- Write (RxPDO: enable + travel command) ---
        // `output_pre_process()` is skipped deliberately too: it latches
        // `stm_control.reset` whenever the drive reports an error, which would
        // fight the explicit error handling above (and it also needs the compact
        // encoder PDO).
        write_outputs(&mut el7037, &outputs);

        if let Some(output) = eth_control.app_handle.write_outputs() {
            let rx_bytes = &mut output[subdevice.start_rx..subdevice.end_rx];
            let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(rx_bytes));
            eth_control.app_handle.send_outputs();
        }

        std::thread::sleep(CYCLE_TIME);
    }

    // Drop the travel command first, then the driver stage, and give each one a
    // cycle on the wire before leaving Op.
    for outputs in [
        Outputs {
            enable: true,
            ..Outputs::default()
        },
        Outputs::default(),
    ] {
        write_outputs(&mut el7037, &outputs);
        if let Some(output) = eth_control.app_handle.write_outputs() {
            let rx_bytes = &mut output[subdevice.start_rx..subdevice.end_rx];
            let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(rx_bytes));
            eth_control.app_handle.send_outputs();
        }
        std::thread::sleep(CYCLE_TIME * 5);
    }

    let _ = eth_control
        .channel
        .request_state_change(EtherCATState::PreOp);
    println!("\nstopped");
}

fn write_outputs(el7037: &mut EL7037, outputs: &Outputs) {
    let stm_control = el7037
        .rxpdo
        .stm_control
        .as_mut()
        .expect("STM Control missing - wrong PDO assignment");
    stm_control.enable = outputs.enable;
    stm_control.reset = outputs.reset;
    stm_control.reduce_torque = false;

    let enc_control = el7037
        .rxpdo
        .enc_control
        .as_mut()
        .expect("ENC Control missing - wrong PDO assignment");
    enc_control.set_counter = outputs.set_counter;
    enc_control.set_counter_value = 0;
    enc_control.enable_latch_c = false;
    enc_control.enable_latch_extern_on_positive_edge = false;
    enc_control.enable_latch_extern_on_negative_edge = false;

    let pos_control = el7037
        .rxpdo
        .pos_control
        .as_mut()
        .expect("POS Control missing - wrong PDO assignment");
    pos_control.execute = outputs.execute;
    pos_control.emergency_stop = outputs.emergency_stop;
    pos_control.target_position = outputs.target_position;
    pos_control.target_velocity = outputs.target_velocity;
    pos_control.start_type = outputs.start_type;
    pos_control.acceleration = ACCELERATION_MS;
    pos_control.deceleration = DECELERATION_MS;
}
