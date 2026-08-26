//! Automatic position-loop gain sweep for the EL7037.
//!
//! Tuning `0x8014:02` (Kp of the position controller) by hand means editing a
//! constant, rebuilding, running a move, squinting at a scrolling panel, and
//! remembering what the last one did. This does the whole grid automatically and
//! prints one table at the end.
//!
//! For each candidate gain it:
//!
//! 1. drops to PreOp and writes the full CoE configuration with that gain,
//! 2. goes to Op, clears any latched error, enables and homes,
//! 3. runs `REPEATS` out-and-back travel commands of `DISTANCE` counts,
//! 4. records, per move: peak overshoot past the target, the residual error when
//!    the terminal reported in-target, how long the move took, whether the motor
//!    stalled, and the creep rate measured over a quiet window afterwards,
//! 5. de-energises and moves on.
//!
//! It is a grid sweep with a scoring function, not a self-tuner: a relay-feedback
//! autotuner is the wrong tool for a stepper whose inner current loop is already
//! closed by the terminal, and the manual documents sensible starting values. The
//! point here is to make the comparison cheap and repeatable, and to make the
//! *measurement* honest - every number below comes from the encoder.
//!
//! # Why this exists
//!
//! The gain on this rig was set to 5 against a documented default of 500, on the
//! evidence that anything above ~15 overshot by 3-4.7x. That evidence was
//! gathered while the encoder counting direction was inverted, i.e. while the
//! position loop had *positive* feedback - under which raising the gain is
//! supposed to run away. With `0x8000:0E` corrected the sweep needs redoing, and
//! this is the tool for it.
//!
//! # Safety
//!
//! Every move is bounded: if the encoder travels past `RUNAWAY_FACTOR` times the
//! commanded distance the move is aborted and the axis de-energised, and that
//! candidate is marked as a runaway. That guard is the whole reason it is safe to
//! sweep a gain that may be unstable.
//!
//! # Why it spawns itself
//!
//! Measured on this master: SDO writes are only serviced in PreOp, and an
//! Op -> PreOp transition requested from a running application hangs. So a
//! candidate cannot be reconfigured in place - each one needs a fresh process
//! that goes PreOp, writes the gain, goes Op, measures, and exits. The sweep
//! mode therefore re-executes this same binary once per candidate with
//! `--single` and aggregates the one-line result each child prints.
//!
//! Usage:
//! ```text
//! cargo run --example el7037_tune -- <interface> [kp,kp,...]   # sweep
//! cargo run --example el7037_tune -- <interface> <kp> --single # one candidate
//! ```

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
    time::{Duration, Instant},
};

// ── Sweep parameters ────────────────────────────────────────────────────────

/// Which motor value the sweep varies.
///
/// Worth knowing before reaching for "PID": this terminal has **no derivative
/// term anywhere**, and the position loop is proportional-plus-feedforward only.
/// The cascade is
///
/// | loop | object | terms |
/// |---|---|---|
/// | position | `0x8014:01` feed forward, `0x8014:02` Kp | P + feedforward |
/// | velocity | `0x8014:03` Kp, `0x8014:04` Tn | PI (`Tn` is the reset time - smaller means more integral action) |
/// | current | `0x8011:01` Kp, `0x8011:02` Ki | PI |
///
/// So the only integral terms available are `Tn` (velocity) and `Ki` (current).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Param {
    /// `0x8014:02` Kp of the position controller.
    Kp,
    /// `0x8014:01` feed forward of the position controller. With a P-only
    /// position loop this carries most of the tracking effort.
    FeedForward,
    /// `0x8014:03` Kp of the velocity controller.
    KpVelo,
    /// `0x8014:04` Tn of the velocity controller, in 0.01 ms. The velocity
    /// loop's integral term.
    Tn,
    /// `0x8011:02` Ki of the current controller.
    KiCurrent,
    /// `0x7020:23` / `0x7020:24` acceleration and deceleration, ms 0 to 100 %.
    /// A prime suspect for step loss.
    Accel,
    /// `0x7020:21` travel velocity, 0..10000 = 0..100 % of the speed range.
    Velocity,
    /// `0x8010:01` maximum coil current in mA, capped by the terminal at 1500.
    Current,
}

impl Param {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "kp" => Self::Kp,
            "ff" | "feedforward" => Self::FeedForward,
            "kpvelo" => Self::KpVelo,
            "tn" => Self::Tn,
            "ki" => Self::KiCurrent,
            "accel" => Self::Accel,
            "velocity" | "velo" => Self::Velocity,
            "current" => Self::Current,
            _ => return None,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Self::Kp => "Kp pos. 0x8014:02",
            Self::FeedForward => "feed forward 0x8014:01",
            Self::KpVelo => "Kp velo. 0x8014:03",
            Self::Tn => "Tn velo. 0x8014:04",
            Self::KiCurrent => "Ki curr. 0x8011:02",
            Self::Accel => "accel/decel ms 0x7020:23",
            Self::Velocity => "velocity 0x7020:21",
            Self::Current => "max current mA 0x8010:01",
        }
    }

    fn defaults(self) -> Vec<u32> {
        match self {
            // Downwards as well as up: 5 was already the best of 5..500, so the
            // optimum may well be below it.
            Self::Kp => vec![1, 2, 3, 5, 8, 12],
            Self::FeedForward => vec![0, 25_000, 50_000, 100_000, 150_000],
            Self::KpVelo => vec![10, 25, 50, 100, 200],
            Self::Tn => vec![5_000, 20_000, 50_000, 100_000, 65_535],
            Self::KiCurrent => vec![2, 5, 10, 20, 40],
            Self::Accel => vec![200, 500, 1000, 2000, 4000],
            Self::Velocity => vec![500, 1000, 2000, 4000, 8000],
            Self::Current => vec![700, 900, 1100, 1300, 1500],
        }
    }
}

/// Values held fixed while some *other* parameter is swept.
const BASE_KP: u16 = 2;
const BASE_FEED_FORWARD: u32 = 100_000;
const BASE_KP_VELO: u32 = 50;
const BASE_TN: u32 = 50_000;
const BASE_KI_CURRENT: u16 = 10;
const BASE_ACCEL: u16 = 500;
const BASE_VELOCITY: i16 = 2000;
const BASE_CURRENT: u16 = 1100;
/// Travel distance per move, in encoder counts. 800 = 0.2 rev.
const DISTANCE: i64 = 800;
/// Out-and-back pairs per candidate.
const REPEATS: usize = 2;
/// Abort a move that travels past this multiple of the commanded distance.
const RUNAWAY_FACTOR: i64 = 3;
const RUNAWAY_SLACK: i64 = 200;
/// How long to sit still after a move before measuring the creep rate.
const SETTLE: Duration = Duration::from_millis(1500);
/// Give up on a move that never reports in-target.
const MOVE_TIMEOUT: Duration = Duration::from_secs(6);

const CYCLE_TIME: Duration = Duration::from_millis(2);
const COUNTS_PER_REV: i64 = 4000;

// ── Axis configuration (see el7037_closed_loop.rs for the full commentary) ──

fn axis_config(param: Param, value: u32) -> EL7037Configuration {
    let pick = |p: Param, base: u32| if param == p { value } else { base };
    let kp_factor_pos = pick(Param::Kp, u32::from(BASE_KP)) as u16;
    let feed_forward = pick(Param::FeedForward, BASE_FEED_FORWARD);
    let kp_velo = pick(Param::KpVelo, BASE_KP_VELO);
    let tn_velo = pick(Param::Tn, BASE_TN) as u16;
    let ki_curr = pick(Param::KiCurrent, u32::from(BASE_KI_CURRENT)) as u16;
    let max_current = pick(Param::Current, u32::from(BASE_CURRENT)) as u16;
    let accel = pick(Param::Accel, u32::from(BASE_ACCEL)) as u16;
    let mut config = EL7037Configuration::default();

    config.stm_features.operation_mode = EL70x1OperationMode::PositionController;
    config.stm_features.speed_range = EL70x1SpeedRange::Steps2000;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    config.stm_features.invert_motor_polarity = false;
    config.stm_features.select_info_data_1 = EL70x7InfoData::DriveState;
    config.stm_features.select_info_data_2 = EL70x7InfoData::DriveStatusWord;

    // Measured: with this false the encoder counts opposite to the commanded
    // direction, which puts the position loop into positive feedback.
    config.encoder.reversion_of_rotation = true;

    config.stm_motor.max_current = max_current;
    config.stm_motor.reduced_current = max_current / 2;
    config.stm_motor.nominal_voltage = 24000;
    config.stm_motor.motor_coil_resistance = 175;
    config.stm_motor.motor_coil_inductance = 330;
    config.stm_motor.motor_full_steps = 200;
    config.stm_motor.encoder_increments = 4000;
    config.stm_motor.motor_emf = 0;

    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = ki_curr;

    config.stm_controller_3.feed_forward_pos = feed_forward;
    config.stm_controller_3.kp_factor_pos = kp_factor_pos;
    config.stm_controller_3.kp_factor_velo = kp_velo;
    config.stm_controller_3.tn_velo = tn_velo;

    config.pos_configuration.velocity_min = 100;
    config.pos_configuration.velocity_max = 10_000;
    config.pos_configuration.acceleration_pos = accel;
    config.pos_configuration.acceleration_neg = accel;
    config.pos_configuration.deceleration_pos = accel;
    config.pos_configuration.deceleration_neg = accel;
    config.pos_configuration.emergency_deceleration = 100;
    config.pos_configuration.target_window = 20;
    config.pos_configuration.in_target_timeout = 1000;
    config.pos_configuration.position_lag_max = 0;

    config.pos_features.start_type = StartType::CalibrationSetManualAuto;
    config.pos_features.emergency_stop_on_position_lag_error = false;

    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionInterfaceWithInfoData;
    config
}

// ── Bus plumbing ────────────────────────────────────────────────────────────

// These are macros rather than functions because naming `EtherCATControl`'s
// type parameters would pull in `triple_buffer`, which the examples crate does
// not depend on.

macro_rules! to_state {
    ($eth:expr, $state:expr) => {{
        let state = $state;
        $eth.channel
            .request_state_change(state)
            .expect("Channel was not ready");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if $eth.app_handle.get_state() == state {
                break;
            }
            if Instant::now() > deadline {
                panic!("timed out waiting for {state:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }};
}

macro_rules! find_el7037 {
    ($eth:expr) => {{
        let mut found = None;
        for _ in 0..50 {
            let subdevices = $eth
                .app_handle
                .try_get_subdevices_vec_sync()
                .expect("Failed to read subdevices!");
            for subdevice in &subdevices {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    found = Some(subdevice.device_address);
                }
            }
            if found.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        found.expect("no EL7037 found")
    }};
}

// ── Measurements ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
struct MoveResult {
    /// Furthest the encoder went past the target, in counts. 0 means none.
    overshoot: i64,
    /// Encoder minus target at the moment the terminal reported in-target.
    residual: i64,
    /// Milliseconds from the execute edge to in-target.
    duration_ms: u128,
    /// The terminal reported in-target rather than timing out.
    reached: bool,
    /// `0x6010:08` latched during the move.
    stalled: bool,
    /// Counts per second measured while parked afterwards.
    creep_per_s: f64,
    /// The runaway guard fired.
    runaway: bool,
}

#[derive(Debug, Clone)]
struct Candidate {
    param: Param,
    value: u32,
    moves: Vec<MoveResult>,
    /// The drive faulted and the candidate could not be measured.
    faulted: bool,
}

impl Candidate {
    fn new(param: Param, value: u32) -> Self {
        Self {
            param,
            value,
            moves: Vec::new(),
            faulted: false,
        }
    }

    /// One line the sweep parent can parse back out of a child's stdout.
    fn encode(&self) -> String {
        let mut out = format!("RESULT {}", u8::from(self.faulted));
        for m in &self.moves {
            out.push_str(&format!(
                " {}:{}:{}:{}:{}:{:.3}:{}",
                m.overshoot,
                m.residual,
                m.duration_ms,
                u8::from(m.reached),
                u8::from(m.stalled),
                m.creep_per_s,
                u8::from(m.runaway),
            ));
        }
        out
    }

    fn decode(param: Param, value: u32, rest: &str) -> Self {
        let mut fields = rest.split_whitespace();
        let faulted = fields.next().map(|v| v == "1").unwrap_or(true);
        let moves = fields
            .filter_map(|field| {
                let f: Vec<&str> = field.split(':').collect();
                if f.len() != 7 {
                    return None;
                }
                Some(MoveResult {
                    overshoot: f[0].parse().ok()?,
                    residual: f[1].parse().ok()?,
                    duration_ms: f[2].parse().ok()?,
                    reached: f[3] == "1",
                    stalled: f[4] == "1",
                    creep_per_s: f[5].parse().ok()?,
                    runaway: f[6] == "1",
                })
            })
            .collect();
        Self {
            param,
            value,
            moves,
            faulted,
        }
    }

    fn mean(&self, f: impl Fn(&MoveResult) -> f64) -> f64 {
        if self.moves.is_empty() {
            return f64::NAN;
        }
        self.moves.iter().map(f).sum::<f64>() / self.moves.len() as f64
    }

    fn worst_overshoot(&self) -> i64 {
        self.moves.iter().map(|m| m.overshoot).max().unwrap_or(0)
    }

    fn reached_all(&self) -> bool {
        !self.moves.is_empty() && self.moves.iter().all(|m| m.reached)
    }

    fn any_runaway(&self) -> bool {
        self.moves.iter().any(|m| m.runaway)
    }

    /// Lower is better. Overshoot and creep are the two symptoms that actually
    /// make this axis unusable, so they dominate; settling time breaks ties.
    fn score(&self) -> f64 {
        if self.faulted || self.moves.is_empty() {
            return f64::INFINITY;
        }
        let overshoot = self.worst_overshoot() as f64;
        let creep = self.mean(|m| m.creep_per_s.abs());
        let residual = self.mean(|m| m.residual.abs() as f64);
        let settle = self.mean(|m| m.duration_ms as f64);
        let missed = self.moves.iter().filter(|m| !m.reached).count() as f64;
        // A runaway is disqualifying; a missed target is bad but still ranked,
        // because on this axis nothing yet reaches every target and an
        // all-infinite table names no winner at all.
        let runaway = if self.any_runaway() { 100_000.0 } else { 0.0 };
        overshoot * 2.0 + creep * 3.0 + residual + settle / 100.0 + missed * 500.0 + runaway
    }
}

// ── Cyclic helpers ──────────────────────────────────────────────────────────

struct Io {
    encoder: i64,
    ready_to_enable: bool,
    ready: bool,
    stm_error: bool,
    motor_stall: bool,
    busy: bool,
    in_target: bool,
    pos_error: bool,
    set_counter_done: bool,
}

fn read_io(el7037: &EL7037) -> Io {
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
    let pos = el7037
        .txpdo
        .pos_status
        .as_ref()
        .expect("POS Status missing");
    Io {
        encoder: i64::from(enc.counter_value as i32),
        ready_to_enable: stm.ready_to_enable,
        ready: stm.ready,
        stm_error: stm.error,
        motor_stall: stm.motor_stall,
        busy: pos.busy,
        in_target: pos.in_target,
        pos_error: pos.error,
        set_counter_done: enc.set_counter_done,
    }
}

#[derive(Clone, Copy)]
struct Drive {
    velocity: i16,
    accel: u16,
    enable: bool,
    reset: bool,
    set_counter: bool,
    execute: bool,
    target: i64,
    start_type: u16,
}

impl Default for Drive {
    fn default() -> Self {
        Self {
            velocity: BASE_VELOCITY,
            accel: BASE_ACCEL,
            enable: false,
            reset: false,
            set_counter: false,
            execute: false,
            target: 0,
            start_type: 0,
        }
    }
}

fn write_io(el7037: &mut EL7037, drive: &Drive) {
    let stm = el7037
        .rxpdo
        .stm_control
        .as_mut()
        .expect("STM Control missing");
    stm.enable = drive.enable;
    stm.reset = drive.reset;
    stm.reduce_torque = false;

    let enc = el7037
        .rxpdo
        .enc_control
        .as_mut()
        .expect("ENC Control missing");
    enc.set_counter = drive.set_counter;
    enc.set_counter_value = 0;

    let pos = el7037
        .rxpdo
        .pos_control
        .as_mut()
        .expect("POS Control missing");
    pos.execute = drive.execute;
    pos.emergency_stop = false;
    pos.target_position = drive.target as u32;
    pos.target_velocity = drive.velocity;
    pos.start_type = drive.start_type;
    pos.acceleration = drive.accel;
    pos.deceleration = drive.accel;
}

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let param = env::args()
        .nth(2)
        .as_deref()
        .and_then(Param::parse)
        .unwrap_or(Param::Kp);
    let single = env::args().any(|a| a == "--single");

    if single {
        let value: u32 = env::args()
            .nth(3)
            .expect("no value given")
            .parse()
            .expect("value must be an integer");
        let candidate = measure_candidate(&interface, param, value);
        println!("{}", candidate.encode());
        return;
    }

    let candidates: Vec<u32> = match env::args().nth(3) {
        Some(list) => list
            .split(',')
            .map(|v| v.trim().parse().expect("candidate list must be integers"))
            .collect(),
        None => param.defaults(),
    };

    println!(
        "sweeping {} over {candidates:?}, {REPEATS} out-and-back moves of \
         {DISTANCE} counts each",
        param.label()
    );
    println!("each candidate runs in its own process - see the module docs for why\n");

    let exe = env::current_exe().expect("cannot find own executable");
    let mut results = Vec::new();
    for value in candidates {
        println!("--- {} = {value} ---", param.label());
        let output = std::process::Command::new(&exe)
            .arg(&interface)
            .arg(arg_name(param))
            .arg(value.to_string())
            .arg("--single")
            .output()
            .expect("could not re-execute for this candidate");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut candidate = Candidate::new(param, value);
        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("RESULT ") {
                candidate = Candidate::decode(param, value, rest);
            } else if !line.trim().is_empty() {
                println!("  {}", line.trim_end());
            }
        }
        if !output.status.success() && candidate.moves.is_empty() {
            println!(
                "  candidate failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            candidate.faulted = true;
        }
        results.push(candidate);
        println!();
    }

    report(&results);
}

/// The command-line spelling of a parameter, for re-executing ourselves.
fn arg_name(param: Param) -> &'static str {
    match param {
        Param::Kp => "kp",
        Param::FeedForward => "ff",
        Param::KpVelo => "kpvelo",
        Param::Tn => "tn",
        Param::KiCurrent => "ki",
        Param::Accel => "accel",
        Param::Velocity => "velocity",
        Param::Current => "current",
    }
}

/// One full PreOp -> configure -> Op -> measure -> exit pass for a single value.
fn measure_candidate(interface: &str, param: Param, value: u32) -> Candidate {
    let mut eth = init_ethercat(interface, None);
    let mut el7037 = EL7037::new();
    let mut candidate = Candidate::new(param, value);

    to_state!(eth, EtherCATState::PreOp);
    let address = find_el7037!(eth);
    el7037
        .write_config(eth.channel.clone(), address, &axis_config(param, value))
        .expect("EL7037 CoE config failed");
    to_state!(eth, EtherCATState::Op);

    let subdevice = eth
        .app_handle
        .try_get_subdevices_vec_sync()
        .expect("Failed to read subdevices!")
        .into_iter()
        .find(|s| s.device_address == address)
        .expect("EL7037 disappeared during the Op transition");
    let (start_tx, end_tx) = (subdevice.start_tx, subdevice.end_tx);
    let (start_rx, end_rx) = (subdevice.start_rx, subdevice.end_rx);

    let mut cycle = |el7037: &mut EL7037, drive: &Drive| -> Option<Io> {
        let mut io = None;
        if let Some(input) = eth.app_handle.get_inputs() {
            let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(&input[start_tx..end_tx]));
            io = Some(read_io(el7037));
            eth.app_handle.finish_read();
        }
        write_io(el7037, drive);
        if let Some(output) = eth.app_handle.write_outputs() {
            let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(
                &mut output[start_rx..end_rx],
            ));
            eth.app_handle.send_outputs();
        }
        std::thread::sleep(CYCLE_TIME);
        io
    };

    let mut drive = Drive {
        velocity: if param == Param::Velocity {
            value as i16
        } else {
            BASE_VELOCITY
        },
        accel: if param == Param::Accel {
            value as u16
        } else {
            BASE_ACCEL
        },
        ..Drive::default()
    };
    if !recover(&mut el7037, &mut cycle, &mut drive) {
        println!("drive never became ready");
        candidate.faulted = true;
        return candidate;
    }

    // Home, so every candidate starts from the same coordinate.
    drive.set_counter = true;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Some(io) = cycle(&mut el7037, &drive)
            && io.set_counter_done
        {
            break;
        }
    }
    drive.set_counter = false;
    for _ in 0..50 {
        cycle(&mut el7037, &drive);
    }

    for repeat in 0..REPEATS * 2 {
        let target = if repeat % 2 == 0 { DISTANCE } else { 0 };
        let result = run_move(&mut el7037, &mut cycle, &mut drive, target);
        println!(
            "move -> {target:>5}: {}overshoot {:>5}  residual {:>+5}  {:>5} ms  \
             creep {:>7.1} c/s{}{}",
            if result.reached { "" } else { "TIMEOUT " },
            result.overshoot,
            result.residual,
            result.duration_ms,
            result.creep_per_s,
            if result.stalled { "  STALL" } else { "" },
            if result.runaway { "  RUNAWAY" } else { "" },
        );
        let runaway = result.runaway;
        candidate.moves.push(result);
        if runaway {
            break;
        }
        // Each move starts from a clean drive: cycling enable is the only thing
        // measured to clear a latched stall and stop the creep, and without it
        // every later move inherits the previous one's mess.
        if !recover(&mut el7037, &mut cycle, &mut drive) {
            println!("drive did not recover; stopping this candidate");
            candidate.faulted = true;
            break;
        }
    }

    drive = Drive::default();
    for _ in 0..60 {
        cycle(&mut el7037, &drive);
    }
    candidate
}

/// Drops `enable`, clears any latched error, and brings the drive back to ready.
/// Returns false if it never gets there.
fn recover(
    el7037: &mut EL7037,
    cycle: &mut impl FnMut(&mut EL7037, &Drive) -> Option<Io>,
    drive: &mut Drive,
) -> bool {
    drive.execute = false;
    drive.start_type = StartType::Idle as u16;
    drive.enable = false;
    for _ in 0..60 {
        cycle(el7037, drive);
    }

    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        let Some(io) = cycle(el7037, drive) else {
            continue;
        };
        if io.stm_error && !drive.enable {
            drive.reset = true;
            continue;
        }
        drive.reset = false;
        if io.ready_to_enable {
            drive.enable = true;
        }
        if io.ready && drive.enable && !io.stm_error {
            // Let the drive-on delay (0x8010:10) settle before measuring.
            for _ in 0..60 {
                cycle(el7037, drive);
            }
            return true;
        }
    }
    false
}

/// Runs one absolute travel command and measures it.
fn run_move(
    el7037: &mut EL7037,
    cycle: &mut impl FnMut(&mut EL7037, &Drive) -> Option<Io>,
    drive: &mut Drive,
    target: i64,
) -> MoveResult {
    let mut result = MoveResult::default();

    // Where we start from, for the overshoot and runaway calculations.
    let start = loop {
        if let Some(io) = cycle(el7037, drive) {
            break io.encoder;
        }
    };
    let commanded = (target - start).abs();
    if commanded == 0 {
        result.reached = true;
        return result;
    }
    let limit = commanded * RUNAWAY_FACTOR + RUNAWAY_SLACK;
    let forwards = target > start;

    // Arm: parameters with execute low for a full cycle, then the rising edge.
    drive.target = target;
    drive.start_type = StartType::Absolute as u16;
    drive.execute = false;
    cycle(el7037, drive);
    drive.execute = true;

    let started = Instant::now();
    let mut seen_busy = false;
    loop {
        let Some(io) = cycle(el7037, drive) else {
            continue;
        };
        result.stalled |= io.motor_stall;

        // Overshoot is only meaningful past the target, in the travel direction.
        let past = if forwards {
            io.encoder - target
        } else {
            target - io.encoder
        };
        result.overshoot = result.overshoot.max(past);

        if (io.encoder - start).abs() > limit {
            result.runaway = true;
            break;
        }
        if io.busy {
            seen_busy = true;
        }
        if io.pos_error {
            // Record where it actually stopped rather than reporting zeroes.
            result.residual = io.encoder - target;
            result.duration_ms = started.elapsed().as_millis();
            break;
        }
        if seen_busy && !io.busy {
            result.reached = io.in_target;
            result.residual = io.encoder - target;
            result.duration_ms = started.elapsed().as_millis();
            break;
        }
        if started.elapsed() > MOVE_TIMEOUT {
            result.residual = io.encoder - target;
            result.duration_ms = started.elapsed().as_millis();
            break;
        }
    }

    // Park the travel command before measuring anything else.
    drive.execute = false;
    drive.start_type = StartType::Idle as u16;
    if result.runaway {
        drive.enable = false;
        for _ in 0..60 {
            cycle(el7037, drive);
        }
        return result;
    }

    // Creep: how far the shaft moves with no command outstanding.
    let quiet_start = loop {
        if let Some(io) = cycle(el7037, drive) {
            break io.encoder;
        }
    };
    let at = Instant::now();
    let mut quiet_end = quiet_start;
    while at.elapsed() < SETTLE {
        if let Some(io) = cycle(el7037, drive) {
            quiet_end = io.encoder;
            result.stalled |= io.motor_stall;
        }
    }
    result.creep_per_s = (quiet_end - quiet_start) as f64 / at.elapsed().as_secs_f64();
    result
}

fn report(results: &[Candidate]) {
    let label = results.first().map(|c| c.param.label()).unwrap_or("");
    println!("\n=== sweep of {label} ===\n");
    println!(
        "{:>8}  {:>9}  {:>9}  {:>8}  {:>10}  {:>6}  {:>8}",
        "value", "overshoot", "residual", "settle", "creep c/s", "stall", "score"
    );
    println!("{}", "-".repeat(70));

    let mut ranked: Vec<&Candidate> = results.iter().collect();
    ranked.sort_by(|a, b| a.score().total_cmp(&b.score()));

    for candidate in results {
        let note = if candidate.faulted {
            "  drive fault"
        } else if candidate.any_runaway() {
            "  RUNAWAY"
        } else if !candidate.reached_all() {
            "  timed out"
        } else {
            ""
        };
        let score = candidate.score();
        println!(
            "{:>8}  {:>9}  {:>+9.0}  {:>6.0} ms  {:>10.1}  {:>6}  {:>8}{note}",
            candidate.value,
            candidate.worst_overshoot(),
            candidate.mean(|m| m.residual as f64),
            candidate.mean(|m| m.duration_ms as f64),
            candidate.mean(|m| m.creep_per_s),
            candidate.moves.iter().filter(|m| m.stalled).count(),
            if score.is_finite() {
                format!("{score:.0}")
            } else {
                "-".into()
            },
        );
    }

    println!();
    match ranked.first() {
        Some(best) if best.score().is_finite() => {
            println!(
                "best: {} = {}  (worst overshoot {} counts = {:.3} rev, \
                 creep {:.1} counts/s)",
                best.param.label(),
                best.value,
                best.worst_overshoot(),
                best.worst_overshoot() as f64 / COUNTS_PER_REV as f64,
                best.mean(|m| m.creep_per_s),
            );
            println!("apply it in examples/el7037_closed_loop.rs");
        }
        _ => println!("no candidate completed every move cleanly"),
    }
    println!(
        "\nnote: CoE persists in the terminal - the last candidate swept is what is\n\
         left in it. Re-run el7037_closed_loop (or el7037_coe_dump) to confirm."
    );
}
