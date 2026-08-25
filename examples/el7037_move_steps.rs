use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, NewEthercatDevice,
        el7037::{
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
    fmt::Write as _,
    io::Write as _,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

/// Moves the motor an exact number of steps and stops, driven by a state
/// machine you control from the terminal while a panel refreshes in place.
///
/// Uses the terminal's positioning interface (`PositionInterfaceWithInfoData`):
/// a travel command is loaded into POS Control (target, velocity, ramps) and
/// started with a rising edge on `execute`; the terminal reports `busy` and
/// `in target` back in POS Status.
///
/// The loop is closed twice over:
/// - inside the terminal, because `feedback_type` is `Encoder`, so the drive
///   positions against the real encoder rather than counted microsteps
/// - inside this example, because after every move the encoder counter is
///   compared against the target that was latched when the command was given,
///   and a residual error larger than `TOLERANCE` is corrected with another
///   travel command (up to `MAX_CORRECTIONS` times)
///
/// Note that POS Status `actual position` (0x6020:11) is the *travel command
/// generator's* position, not a measurement — the encoder counter (0x6000:11)
/// is the feedback signal, and that is what the correction loop uses.
///
/// Usage: cargo run --example el7037_move_steps -- <network-interface>
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.get_state() {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    println!("preop");

    // CoE config must happen in PreOp (before Op transition)
    let mut el7037 = EL7037::new();
    let mut config = EL7037Configuration::default();

    config.stm_features.operation_mode = EL70x1OperationMode::Automatic;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    config.stm_features.speed_range = SPEED_RANGE;
    // Report the positioning unit's own internal state (0x9020:03) and its
    // current velocity in the cyclic data, so the terminal says what it thinks
    // it is doing instead of us inferring it from the status bits.
    // Swappable so a run can watch the drive's own load/current instead, which
    // is what tells apart "the loop is wrong" from "the motor is out of torque".
    config.stm_features.select_info_data_1 = match env::var("EL7037_INFO1").as_deref() {
        Ok("load") => EL70x7InfoData::MotorLoad,
        Ok("current") => EL70x7InfoData::MotorDcCurrent,
        _ => EL70x7InfoData::DriveState,
    };
    config.stm_features.select_info_data_2 = match env::var("EL7037_INFO2").as_deref() {
        Ok("load") => EL70x7InfoData::MotorLoad,
        Ok("current") => EL70x7InfoData::MotorDcCurrent,
        _ => EL70x7InfoData::CurrentVelocity,
    };
    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionInterfaceWithInfoData;

    // Encoder direction (0x8000:0E) and motor direction (0x8012:09) are two
    // independent sign bits, and only their *agreement* matters to the loop.
    // All four combinations were measured on this axis: the two where they
    // disagree run away in the wrong direction, and the two where they agree
    // behave identically apart from mirroring which way is positive. This pair
    // is the one that matches the machine's own idea of forwards.
    config.encoder.reversion_of_rotation = env_flag("EL7037_REVERSION", true);
    config.stm_features.invert_motor_polarity = env_flag("EL7037_POLARITY", false);

    // Motor/encoder parameters from the igus MOT-AN-S-060-005-042-L-C-AAAO
    // datasheet. The terminal models the coil to regulate current, so leaving
    // resistance and inductance at their defaults (1.00 ohm and 0 mH) gives it
    // the wrong plant to control.
    //
    // 1.1 A is the motor's *thermal* continuous rating; its 0.5 Nm holding
    // torque is specified at the 1.8 A rated current. 1500 mA is the EL7037's
    // ceiling and well inside the motor's 1.8 A, and it is needed: at 800 mA
    // the harder direction cannot break away at all (the terminal ends the
    // travel command after 24 ms with a positioning warning and zero counts)
    // while the easy direction is unaffected.
    config.stm_motor.max_current = env_num("EL7037_CURRENT", 1500);
    // Hold at the continuous rating, but never above the run current
    config.stm_motor.reduced_current = 1100.min(config.stm_motor.max_current);
    config.stm_motor.nominal_voltage = 24_000; // EL7037 is a 24 V terminal
    config.stm_motor.motor_coil_resistance = 175; // 1.75 ohm, unit 0.01 ohm
    config.stm_motor.motor_coil_inductance = 330; // 3.30 mH, unit 0.01 mH
    config.stm_motor.motor_full_steps = 200; // 1.8 deg per full step
    // The terminal default for Motor EMF is 0; the HAL invents 200 mV/(rad/s).
    // That is a velocity-proportional term feeding a velocity loop that has an
    // integrator (Tn 0x8014:04, 500 ms), so a wrong value leaves the loop with
    // a standing output after a move.
    config.stm_motor.motor_emf = 0;
    config.stm_motor.encoder_increments = ENCODER_INCREMENTS as u16;

    // Current controller gains. The HAL's defaults (Kp 400, Ki 4) do not match
    // the terminal's own defaults (Kp 150, Ki 10) - a proportional term nearly
    // 3x too high with a weakened integrator, which makes the current loop
    // rough and cost the motor its synchronism on every move.
    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = 10;

    // Position loop (0x8014). The HAL never used to write this object at all,
    // so the terminal simply kept whatever an earlier commissioning session had
    // left in it - here Kp (pos.) = 5 against a documented default of 500.
    //
    // 5 is NOT a typo to be "fixed": measured on this axis, the position loop
    // converges at Kp 5 and 10, and is unstable from 15 upwards (a 800 step
    // move runs 3-4.7x too far and has to be aborted). Whoever set 5 was
    // detuning a loop that will not tolerate the documented gain, so this axis
    // has far more inertia or lag than the terminal expects. Until that is
    // understood, write the value that actually works rather than leaving the
    // object unwritten and the drive's behaviour dependent on its history.
    config.stm_controller_3.kp_factor_pos = env_num("EL7037_KPPOS", 5);
    // Tn was swept independently and changes nothing measurable here, so this
    // is simply the terminal's documented default rather than a tuned value.
    config.stm_controller_3.tn_velo = env_num("EL7037_TN", 50_000);

    // Positioning unit. The ramps are also sent with every travel command
    // below; keeping both consistent avoids surprises about which one applies.
    config.pos_configuration.acceleration_pos = RAMP_MS;
    config.pos_configuration.acceleration_neg = RAMP_MS;
    config.pos_configuration.deceleration_pos = RAMP_MS;
    config.pos_configuration.deceleration_neg = RAMP_MS;
    config.pos_configuration.target_window = TARGET_WINDOW;
    // Once the deceleration ramp has ended and the target is not yet reached,
    // the terminal crawls the rest of the way at "Velocity min.", which the
    // manual says must be slow enough for the motor "to stop abruptly and
    // without a step loss". At 100 that crawl is 20 full steps/s (400 encoder
    // counts/s), which looks like an obvious suspect for the overrun - but it
    // is not: dropping it to 20 and to 5 changed nothing measurable. Left at
    // the documented default.
    config.pos_configuration.velocity_min = env_num("EL7037_VMIN", 100) as i16;
    config.pos_features.start_type = StartType::Relative;

    // `get_subdevices()` is not always populated the instant PreOp is reached.
    // Configuring nothing here is silent, and only shows up much later as a
    // missing PDO object, so wait for the terminal to actually appear.
    let mut configured = 0;
    for attempt in 0..50 {
        for subdevice in eth_control.controller.get_subdevices() {
            if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
                el7037
                    .write_config(
                        eth_control.channel.clone(),
                        subdevice.device_address,
                        &config,
                    )
                    .expect("EL7037 CoE config failed");
                configured += 1;
            }
        }
        if configured > 0 {
            break;
        }
        if attempt == 49 {
            panic!("no EL7037 found on {interface} while in PreOp");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("configured {configured} EL7037");

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    // Static header, the panel below it is the part that refreshes in place
    println!("\nEL7037 move-steps · interface {interface} · positioning interface");
    println!(
        "  signs: reversion_of_rotation={} invert_motor_polarity={} max_current={}",
        config.encoder.reversion_of_rotation,
        config.stm_features.invert_motor_polarity,
        config.stm_motor.max_current
    );
    println!("  <n> / -<n>  move exactly n steps      r  repeat the last move");
    println!("  s  stop (emergency ramp)              z  zero the encoder (disable first)");
    println!("  e  toggle enable (holding torque)     c  clear a drive error");
    println!("  k  calibrate (set manual)");
    println!("  v <n>  travel velocity, {VELOCITY_MIN}-{VELOCITY_MAX}          q  quit\n");

    let commands = spawn_input_thread();
    let mut axis = Axis::new();
    // Optional second argument raises the runaway guard, so the axis can be
    // watched creeping for long enough to see whether the shaft really turns
    if let Some(limit) = env::args().nth(2).and_then(|a| a.parse::<i32>().ok()) {
        axis.runaway_limit = limit.max(10);
        println!("runaway guard raised to {} steps", axis.runaway_limit);
    }
    let mut screen = Screen::new();
    let mut speed = SpeedTracker::new();
    let start = Instant::now();
    let mut cycle = 0u32;
    let mut have_data = false;

    loop {
        // --- Read inputs (TxPDO: encoder, motor status, positioning status) ---
        if let Some(input) = eth_control.app_handle.get_inputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let tx_bytes = &input[subdevice.start_tx..subdevice.end_tx];
                    let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(tx_bytes));
                    have_data = true;
                    // No `input_post_process()`: it maintains the counter wrapper
                    // for the compact encoder PDO, which this assignment does not
                    // contain. `SpeedTracker` does the wrap handling instead.
                }
            }

            eth_control.app_handle.finish_read();
        }

        // Until the first TxPDO has landed the PDO structs still hold their
        // defaults. Running the state machine on those would latch a target of
        // zero and report a position the axis is not actually at.
        if !have_data {
            std::thread::sleep(CYCLE_TIME);
            continue;
        }

        let now = Instant::now();
        let sample = Sample::read(&el7037);
        speed.update(sample.encoder, now);

        match commands.try_recv() {
            // The terminal echoed the typed line, so the panel has to be
            // painted fresh below it instead of redrawn in place
            Ok(line) => {
                axis.command(&line, &sample, now);
                screen.reset();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => axis.quit = true,
        }

        let outputs = axis.step(&sample, now);

        if cycle.is_multiple_of(REFRESH_EVERY) || screen.is_reset() {
            screen.draw(&render(
                &axis,
                &sample,
                &speed,
                start.elapsed().as_secs_f64(),
            ));
        }

        // --- Write outputs (RxPDO: enable + travel command) ---
        // `output_pre_process()` is skipped on purpose: it would auto-acknowledge
        // drive errors via `stm_control.reset`, and the state machine here wants
        // to see an error and let you clear it deliberately with `c`.
        let stm_control = el7037.rxpdo.stm_control.as_mut().expect("No STM Control");
        stm_control.enable = outputs.enable;
        stm_control.reset = outputs.reset;
        stm_control.reduce_torque = false;

        let enc_control = el7037
            .rxpdo
            .enc_control
            .as_mut()
            .expect("No Encoder Control");
        enc_control.set_counter = outputs.set_counter;
        enc_control.set_counter_value = 0;

        let pos_control = el7037.rxpdo.pos_control.as_mut().expect("No POS Control");
        pos_control.execute = outputs.execute;
        pos_control.emergency_stop = outputs.emergency_stop;
        pos_control.target_position = outputs.distance as u32;
        pos_control.target_velocity = outputs.velocity;
        pos_control.start_type = outputs.start_type;
        pos_control.acceleration = RAMP_MS;
        pos_control.deceleration = RAMP_MS;

        if let Some(output) = eth_control.app_handle.write_outputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let rx_bytes = &mut output[subdevice.start_rx..subdevice.end_rx];
                    let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(rx_bytes));
                }
            }
            eth_control.app_handle.send_outputs();
        }

        // Quitting only after the disabling frame above has been published
        if axis.quit {
            std::thread::sleep(Duration::from_millis(50));
            println!("\n\ndrive disabled, bye");
            return;
        }

        cycle = cycle.wrapping_add(1);
        std::thread::sleep(CYCLE_TIME);
    }
}

/// Read a numeric tuning value from the environment, falling back to `default`
fn env_num(name: &str, default: u16) -> u16 {
    env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(default)
}

/// Read a boolean tuning flag from the environment, falling back to `default`
/// when it is unset or not recognised. Accepts `1/0`, `true/false`, `yes/no`.
fn env_flag(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}

// ── Tuning ──────────────────────────────────────────────────────────────────

/// Encoder increments per revolution (0x8010:07)
const ENCODER_INCREMENTS: u32 = 4000;

/// Full steps per second at 100% velocity (0x8012:05)
const SPEED_RANGE: EL70x1SpeedRange = EL70x1SpeedRange::Steps2000;
const SPEED_RANGE_STEPS: f64 = 2000.0;

/// Acceleration and deceleration ramp time in ms, sent with every travel
/// command and written to 0x8020:03..06
const RAMP_MS: u16 = 500;

/// The positioning unit's velocity scale runs 0..10000 (0x8020:01/02), where
/// 10000 is 100% of the speed range above.
///
/// 200 is the operating point every measurement on this axis was taken at
/// (200/10000 of 2000 full steps/s = 40 full steps/s = 800 encoder counts/s).
/// It is the tested default rather than a demonstrated optimum - going much
/// slower was measurably worse in an earlier session, and faster is untested.
const VELOCITY_MIN: i16 = 100;
const VELOCITY_MAX: i16 = 10000;
const DEFAULT_VELOCITY: i16 = 200;

/// In-target window reported by the terminal (0x8020:0B), in encoder steps.
/// This has to be wider than the axis actually hunts by, otherwise the terminal
/// never reaches its TARGET state, stays in PRE_TARGET and keeps "pulling the
/// motor further into the target" forever - which looks exactly like the motor
/// creeping away on its own.
const TARGET_WINDOW: u16 = 100;

/// How long to keep waiting for `in target` after the travel generator has
/// dropped `busy`. Beckhoff's documented sequence only drops `execute` once
/// `busy` is low AND `in target` is high; this bounds that wait so a terminal
/// that never reports `in target` cannot wedge the state machine.
const IN_TARGET_GRACE: Duration = Duration::from_millis(1500);

/// How close the encoder has to end up for the move to count as done
const TOLERANCE: i32 = 5;

/// Correction travel commands issued after the first move before giving up.
///
/// The hard direction reliably consumes all of them (a -800 move converges as
/// +512, +67, +13 and lands within tolerance on the third), so 3 leaves no
/// margin at all. 4 is a judgement call rather than a measured optimum - every
/// correction is still bounded by `TOLERANCE` and the overtravel guard, so the
/// only cost of the extra pass is a little time.
const MAX_CORRECTIONS: u32 = 4;

/// Motion tolerated while idle before the drive is cut out as a runaway
const RUNAWAY_LIMIT: i32 = 50;

/// A move is abandoned once the axis has travelled this many times the
/// commanded distance. With inverted feedback the terminal never sees itself
/// approach the target and drives on, so time alone is a poor bound.
const OVERTRAVEL_FACTOR: i64 = 3;

/// Small moves need absolute headroom too: 3x of 20 steps is not a fault
const OVERTRAVEL_FLOOR: i64 = 400;

/// Time the axis has to stand still before the residual error is judged
const SETTLE_TIME: Duration = Duration::from_millis(300);

/// The terminal has to accept a travel command within this time
const EXECUTE_TIMEOUT: Duration = Duration::from_millis(500);

/// A single travel command may not take longer than this
const MOVE_TIMEOUT: Duration = Duration::from_secs(60);

const CYCLE_TIME: Duration = Duration::from_millis(10);

/// Redraw every 20th cycle, ~5 frames per second
const REFRESH_EVERY: u32 = 20;

// ── State machine ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Pulse `stm_control.reset` to clear whatever the drive came up with
    Resetting,
    /// Wait for the error bits to go away, then enable
    Enabling,
    /// Enabled and holding position, waiting for a command
    Idle,
    /// Load the travel command with `execute` low, so the next cycle is an edge
    Arming,
    /// `execute` high, waiting for the terminal to report `busy`
    Executing,
    /// Travel command running, `execute` stays high until `busy` clears
    Moving,
    /// Motion finished, let the mechanics come to rest before judging the error
    Settling,
    /// Emergency ramp after `s`
    Stopping,
    /// Encoder counter is being set to zero
    Zeroing,
    /// Judging the residual error, once the axis is back in sync and still
    Evaluating,
    /// Pulsing `reset` after a move to clear the terminal's latched stall
    /// warning, which otherwise keeps creeping the motor
    ClearingLatch,
    /// Issuing a manual calibration travel command
    Calibrating,
    /// Drive reported an error, waits for `c`
    Faulted,
}

impl State {
    fn label(self) -> &'static str {
        match self {
            Self::Resetting => "resetting",
            Self::Enabling => "enabling",
            Self::Idle => "idle",
            Self::Arming => "arming",
            Self::Executing => "executing",
            Self::Moving => "moving",
            Self::Settling => "settling",
            Self::Stopping => "stopping",
            Self::Zeroing => "zeroing",
            Self::Evaluating => "evaluating",
            Self::ClearingLatch => "clearing latch",
            Self::Calibrating => "calibrating",
            Self::Faulted => "FAULTED",
        }
    }
}

/// What the state machine writes to the RxPDO this cycle
struct Outputs {
    enable: bool,
    reset: bool,
    execute: bool,
    emergency_stop: bool,
    /// Relative travel distance in steps, cast to u32 on the way out
    distance: i32,
    /// Start type for the travel command in POS Control (0x7020:22)
    start_type: u16,
    velocity: i16,
    set_counter: bool,
}

struct Axis {
    state: State,
    since: Instant,
    /// Holding torque on/off, toggled with `e`
    enabled: bool,
    velocity: i16,
    /// Absolute encoder position the current move is aiming for
    target: u32,
    /// What the user asked for, kept for `r`
    requested: i64,
    last_requested: Option<i64>,
    /// Distance handed to the travel command that is currently running
    distance: i32,
    /// Encoder position when that command was started, used to tell a refused
    /// command apart from one that finished before `busy` was ever sampled
    execute_encoder: u32,
    /// Encoder position when the whole move was requested, used to report a
    /// move that never produced any motion at all
    move_start_encoder: u32,
    /// Encoder position when the axis last went idle, used by the runaway guard
    idle_anchor: u32,
    /// Motion tolerated while idle before the drive is cut out
    runaway_limit: i32,
    /// When the travel generator dropped `busy`, so the wait for `in target`
    /// after it can be bounded
    busy_cleared: Option<Instant>,
    corrections: u32,
    message: String,
    quit: bool,
}

impl Axis {
    fn new() -> Self {
        Self {
            state: State::Resetting,
            since: Instant::now(),
            enabled: true,
            velocity: DEFAULT_VELOCITY,
            target: 0,
            requested: 0,
            last_requested: None,
            distance: 0,
            execute_encoder: 0,
            move_start_encoder: 0,
            idle_anchor: 0,
            runaway_limit: RUNAWAY_LIMIT,
            busy_cleared: None,
            corrections: 0,
            message: String::from("starting up"),
            quit: false,
        }
    }

    fn go(&mut self, state: State, now: Instant) {
        self.state = state;
        self.since = now;
    }

    /// Going idle also re-anchors the runaway guard, so the drift it watches is
    /// measured from wherever the axis actually came to rest rather than from
    /// the target of a move that may have been given up on.
    fn go_idle(&mut self, sample: &Sample, now: Instant) {
        self.idle_anchor = sample.encoder;
        self.go(State::Idle, now);
    }

    /// Steps still to go, measured against the encoder — this is the feedback
    /// the correction loop closes on. Wrapping keeps it correct across the
    /// u32 counter rolling over.
    fn remaining(&self, sample: &Sample) -> i32 {
        self.target.wrapping_sub(sample.encoder) as i32
    }

    fn command(&mut self, line: &str, sample: &Sample, now: Instant) {
        let Some(command) = Command::parse(line) else {
            return;
        };

        match command {
            Command::Quit => {
                self.quit = true;
            }
            Command::Stop => match self.state {
                State::Executing | State::Moving => {
                    self.message = String::from("stopping");
                    self.go(State::Stopping, now);
                }
                _ => self.message = String::from("nothing to stop"),
            },
            Command::ClearError => {
                self.message = String::from("clearing error");
                self.go(State::Resetting, now);
            }
            Command::ToggleEnable => {
                if matches!(self.state, State::Executing | State::Moving) {
                    self.message = String::from("cannot toggle enable while moving");
                } else {
                    self.enabled = !self.enabled;
                    self.message = format!(
                        "drive {}",
                        if self.enabled { "enabled" } else { "disabled" }
                    );
                }
            }
            Command::Velocity(velocity) => {
                self.velocity = velocity.clamp(VELOCITY_MIN, VELOCITY_MAX);
                self.message = format!(
                    "velocity {} (~{:.0} steps/s)",
                    self.velocity,
                    velocity_to_steps(self.velocity)
                );
            }
            Command::Calibrate => {
                if self.state != State::Idle {
                    self.message = String::from("calibration needs an idle axis");
                } else {
                    self.message = String::from("calibrating");
                    self.go(State::Calibrating, now);
                }
            }
            Command::Zero => {
                if self.state != State::Idle {
                    self.message = String::from("zeroing needs an idle axis");
                } else if self.enabled {
                    // Rewriting the counter under an enabled position controller
                    // moves the axis: the drive suddenly sees a large following
                    // error and drives it out. Make the user disable first.
                    self.message = String::from("disable with 'e' before zeroing");
                } else {
                    self.message = String::from("zeroing encoder");
                    self.go(State::Zeroing, now);
                }
            }
            Command::Repeat => match self.last_requested {
                Some(steps) => self.start_move(steps, sample, now),
                None => self.message = String::from("no previous move to repeat"),
            },
            Command::Move(steps) => self.start_move(steps, sample, now),
            Command::Unknown(input) => {
                self.message = format!("unknown command {input:?}");
            }
        }
    }

    fn start_move(&mut self, steps: i64, sample: &Sample, now: Instant) {
        if self.state != State::Idle {
            self.message = format!("busy ({}), command ignored", self.state.label());
            return;
        }
        if !self.enabled {
            self.message = String::from("drive is disabled, enable with 'e'");
            return;
        }
        let steps = steps.clamp(-MAX_STEPS, MAX_STEPS) as i32;
        if steps == 0 {
            self.message = String::from("zero steps, nothing to do");
            return;
        }

        // The target is latched once, in encoder counts. Every correction below
        // aims at this same absolute position, so repeated corrections cannot
        // accumulate their own error.
        self.target = sample.encoder.wrapping_add(steps as u32);
        self.move_start_encoder = sample.encoder;
        self.requested = steps as i64;
        self.last_requested = Some(steps as i64);
        self.corrections = 0;
        self.message = format!("moving {steps:+} steps");
        self.go(State::Arming, now);
    }

    fn step(&mut self, sample: &Sample, now: Instant) -> Outputs {
        if self.quit {
            return Outputs {
                enable: false,
                reset: false,
                execute: false,
                emergency_stop: false,
                distance: 0,
                start_type: u16::from(StartType::Relative),
                velocity: self.velocity,
                set_counter: false,
            };
        }

        // A drive error preempts everything except the reset that clears it
        if (sample.drive_error || sample.pos_error)
            && !matches!(self.state, State::Resetting | State::Faulted)
        {
            self.message = String::from("drive reported an error, press 'c' to clear");
            self.go(State::Faulted, now);
        }

        let mut out = Outputs {
            enable: self.enabled,
            reset: false,
            execute: false,
            emergency_stop: false,
            // Park the travel command as IDLE whenever nothing is running. The
            // terminal activates travel commands on a *change of start type*
            // ("Activation by new start types"), not only on the execute edge,
            // so leaving a live RELATIVE command with a stale distance sitting
            // in the process data is not inert.
            distance: 0,
            start_type: u16::from(StartType::Idle),
            velocity: self.velocity,
            set_counter: false,
        };

        match self.state {
            State::Resetting => {
                // Reset is a level the drive samples, hold it for a few cycles
                out.enable = false;
                out.reset = true;
                if self.since.elapsed() > Duration::from_millis(100) {
                    self.go(State::Enabling, now);
                }
            }
            State::Enabling => {
                out.reset = false;
                out.enable = self.enabled;
                if sample.drive_error || sample.pos_error {
                    if self.since.elapsed() > Duration::from_secs(2) {
                        self.message = String::from("error will not clear, check the drive");
                        self.go(State::Faulted, now);
                    }
                } else {
                    self.target = sample.encoder;
                    self.message = String::from("ready");
                    self.go_idle(sample, now);
                }
            }
            State::Idle => {
                // Nothing is commanding motion here, so the axis must stand
                // still. If it does not, the drive's position loop is running
                // away from its own feedback - almost always because the
                // encoder counts against the motor direction. Drop the enable
                // rather than let it accelerate.
                let drift = sample.encoder.wrapping_sub(self.idle_anchor) as i32;
                if drift.abs() > self.runaway_limit {
                    self.enabled = false;
                    self.message = format!(
                        "RUNAWAY: {drift:+} steps with nothing commanded, drive disabled. \
                         Check encoder direction (reversion_of_rotation / invert_motor_polarity)"
                    );
                    self.go(State::Faulted, now);
                }
            }
            State::Arming => {
                // `execute` must be low for at least one cycle so that the next
                // cycle is a rising edge for the terminal
                self.distance = self.remaining(sample);
                self.execute_encoder = sample.encoder;
                self.busy_cleared = None;
                out.distance = self.distance;
                out.start_type = u16::from(StartType::Relative);
                out.execute = false;
                self.go(State::Executing, now);
            }
            State::Executing => {
                out.distance = self.distance;
                out.start_type = u16::from(StartType::Relative);
                out.execute = true;
                if sample.busy {
                    self.go(State::Moving, now);
                } else if self.since.elapsed() > EXECUTE_TIMEOUT {
                    // A correction can be short enough that the terminal is done
                    // with it before `busy` is ever sampled, so a missing `busy`
                    // does not mean the command was refused. Let the settle step
                    // judge the position instead: it either finishes the move or
                    // spends a correction pass on it, and a genuinely refused
                    // command shows up as a drive error further up.
                    if sample.encoder == self.execute_encoder
                        && self.remaining(sample).abs() > TOLERANCE
                    {
                        self.message = String::from("travel command produced no motion");
                    }
                    self.go(State::Settling, now);
                }
            }
            State::Moving => {
                // Careful: a falling edge on `execute` aborts the travel command.
                // Beckhoff's sequence holds it high until `busy` is low *and*
                // `in target` is high - dropping it while the terminal is still
                // in PRE_TARGET aborts the pull-in half way.
                out.distance = self.distance;
                out.start_type = u16::from(StartType::Relative);
                out.execute = true;

                if sample.busy {
                    self.busy_cleared = None;
                } else if self.busy_cleared.is_none() {
                    self.busy_cleared = Some(now);
                }

                match (sample.busy, sample.in_target, self.busy_cleared) {
                    // documented completion: generator done and inside the window
                    (false, true, _) => self.go(State::Settling, now),
                    // `busy` went away but the window was never reported; give the
                    // terminal its in-target timeout before taking over
                    (false, false, Some(cleared)) if cleared.elapsed() > IN_TARGET_GRACE => {
                        self.message =
                            String::from("no 'in target' from the terminal, widen target_window?");
                        self.go(State::Settling, now);
                    }
                    _ => {
                        // If the encoder and the motor disagree about which way
                        // is positive, the terminal never sees itself approach
                        // the target and drives until the timeout - 60 s of
                        // free running. Bound the move by distance travelled,
                        // not just by time.
                        // Budget the whole command, corrections included, against
                        // what the user actually asked for.
                        let travelled =
                            sample.encoder.wrapping_sub(self.move_start_encoder) as i32 as i64;
                        let budget = self
                            .requested
                            .saturating_abs()
                            .saturating_mul(OVERTRAVEL_FACTOR)
                            .max(OVERTRAVEL_FLOOR);
                        if travelled.abs() > budget {
                            self.enabled = false;
                            self.message = format!(
                                "OVERTRAVEL: {travelled:+} steps into a {:+} step move (budget {budget}), \
                                 drive disabled. Encoder and motor directions probably disagree.",
                                self.requested
                            );
                            self.go(State::Faulted, now);
                        } else if self.since.elapsed() > MOVE_TIMEOUT {
                            self.message = String::from("move timed out, stopping");
                            self.go(State::Stopping, now);
                        }
                    }
                }
            }
            State::Settling => {
                out.execute = false;
                // Never judge the error while the axis is still moving, e.g.
                // when the drive is ramping down after the falling edge above
                if sample.busy {
                    self.since = now;
                    return out;
                }
                if self.since.elapsed() < SETTLE_TIME {
                    return out;
                }
                // Re-sync first: judging the position while the axis is still
                // creeping makes every correction chase a moving target
                self.go(State::ClearingLatch, now);
                return out;
            }
            State::Evaluating => {
                let remaining = self.remaining(sample);
                if remaining.abs() <= TOLERANCE {
                    self.message = format!(
                        "{:+} steps done, off by {:+} after {} correction{}",
                        self.requested,
                        remaining,
                        self.corrections,
                        if self.corrections == 1 { "" } else { "s" }
                    );
                    self.go_idle(sample, now);
                } else if self.corrections < MAX_CORRECTIONS {
                    self.corrections += 1;
                    self.message =
                        format!("correcting {remaining:+} steps (pass {})", self.corrections);
                    self.go(State::Arming, now);
                } else if sample.encoder == self.move_start_encoder {
                    // Not a single step in any of the attempts: the travel
                    // commands are not being acted on at all
                    self.message = String::from(
                        "no motion at all, is the drive enabled and the travel command valid?",
                    );
                    self.go_idle(sample, now);
                } else {
                    self.message = format!(
                        "gave up after {MAX_CORRECTIONS} corrections, off by {remaining:+} steps"
                    );
                    self.go_idle(sample, now);
                }
            }
            State::Stopping => {
                out.execute = false;
                out.emergency_stop = true;
                if !sample.busy {
                    // Whatever position the axis stopped at becomes the new
                    // target, otherwise the correction loop would drive it on
                    self.target = sample.encoder;
                    self.message = String::from("stopped");
                    self.go_idle(sample, now);
                }
            }
            State::Zeroing => {
                out.enable = false;
                out.set_counter = true;
                if sample.set_counter_done {
                    self.target = 0;
                    self.message = String::from("encoder zeroed");
                    self.go_idle(sample, now);
                } else if self.since.elapsed() > Duration::from_secs(1) {
                    self.message = String::from("zeroing timed out");
                    self.go_idle(sample, now);
                }
            }
            State::ClearingLatch => {
                // Every travel command leaves the motor out of synchronism with
                // the energised field: the terminal latches a stall warning and
                // keeps stepping the motor afterwards. Clearing the latch with
                // `enable` held high does NOT stop that - only dropping and
                // re-applying `enable` does, which is the usual way of pulling a
                // stepper that has lost its step back into sync.
                out.enable = false;
                out.reset = true;
                if self.since.elapsed() > Duration::from_millis(150) {
                    // Re-energised; the rotor may have snapped to the nearest
                    // detent, so let the settle/correct pass judge the position
                    self.go(State::Evaluating, now);
                }
            }
            State::Calibrating => {
                // Set-manual calibration: the terminal marks the axis calibrated
                // at the calibration position (0x8020:08) without needing a cam
                out.start_type = u16::from(StartType::CalibrationSetManualAuto);
                out.distance = 0;
                let elapsed = self.since.elapsed();
                // hold execute low briefly so the terminal sees a rising edge
                out.execute = elapsed > Duration::from_millis(50);
                if sample.calibrated {
                    self.message = String::from("axis calibrated");
                    self.go_idle(sample, now);
                } else if elapsed > Duration::from_secs(3) {
                    self.message = String::from("calibration timed out, flag never came on");
                    self.go_idle(sample, now);
                }
            }
            State::Faulted => {
                out.enable = false;
                out.execute = false;
            }
        }

        out
    }
}

/// Biggest move accepted from the command line, so a typo cannot send the axis
/// on a very long journey
const MAX_STEPS: i64 = 1_000_000;

fn velocity_to_steps(velocity: i16) -> f64 {
    velocity as f64 / VELOCITY_MAX as f64 * SPEED_RANGE_STEPS
}

// ── Commands ────────────────────────────────────────────────────────────────

enum Command {
    Move(i64),
    Calibrate,
    Repeat,
    Stop,
    Zero,
    ToggleEnable,
    ClearError,
    Velocity(i16),
    Quit,
    Unknown(String),
}

impl Command {
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        if let Some(rest) = line.strip_prefix('v') {
            return Some(match rest.trim().parse::<i16>() {
                Ok(velocity) => Self::Velocity(velocity),
                Err(_) => Self::Unknown(line.to_string()),
            });
        }

        Some(match line {
            "q" | "quit" | "exit" => Self::Quit,
            "s" | "stop" => Self::Stop,
            "r" | "repeat" => Self::Repeat,
            "z" | "zero" => Self::Zero,
            "k" | "calibrate" => Self::Calibrate,
            "e" | "enable" => Self::ToggleEnable,
            "c" | "clear" => Self::ClearError,
            _ => match line.parse::<i64>() {
                Ok(steps) => Self::Move(steps),
                Err(_) => Self::Unknown(line.to_string()),
            },
        })
    }
}

/// Reads commands off stdin without blocking the cyclic loop. Line buffered,
/// so no raw terminal mode and no extra dependency is needed.
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
            // Ctrl-D closes stdin, treat it like an explicit quit
            let _ = sender.send(String::from("q"));
        })
        .expect("could not spawn the input thread");
    receiver
}

// ── Process data ────────────────────────────────────────────────────────────

/// Everything the panel and the state machine read, copied out of the PDOs so
/// the borrow on the device ends before the RxPDO is written.
struct Sample {
    encoder: u32,
    set_counter_done: bool,
    drive_ready: bool,
    drive_error: bool,
    drive_warning: bool,
    motor_stall: bool,
    moving_positive: bool,
    moving_negative: bool,
    busy: bool,
    in_target: bool,
    pos_warning: bool,
    pos_error: bool,
    calibrated: bool,
    ready_to_execute: bool,
    /// Position of the travel command generator, not a measurement
    generator_position: u32,
    generator_velocity: i16,
    drive_time: u32,
    motor_load: u16,
    dc_current: u16,
}

impl Sample {
    fn read(el7037: &EL7037) -> Self {
        let enc_status = el7037.txpdo.enc_status.as_ref().expect("No Encoder Status");
        let stm_status = el7037.txpdo.stm_status.as_ref().expect("No STM Status");
        let pos_status = el7037.txpdo.pos_status.as_ref().expect("No POS Status");
        let info = el7037.txpdo.stm_synchron_info_data.as_ref();

        Self {
            encoder: enc_status.counter_value,
            set_counter_done: enc_status.set_counter_done,
            drive_ready: stm_status.ready,
            drive_error: stm_status.error,
            drive_warning: stm_status.warning,
            motor_stall: stm_status.motor_stall,
            moving_positive: stm_status.moving_positive,
            moving_negative: stm_status.moving_negative,
            busy: pos_status.busy,
            in_target: pos_status.in_target,
            pos_warning: pos_status.warning,
            pos_error: pos_status.error,
            calibrated: pos_status.calibrated,
            ready_to_execute: pos_status.ready_to_execute,
            generator_position: pos_status.actual_position,
            generator_velocity: pos_status.actual_velocity,
            drive_time: pos_status.actual_drive_time,
            motor_load: info.map_or(0, |i| i.info_data_1),
            dc_current: info.map_or(0, |i| i.info_data_2),
        }
    }
}

/// Measured speed from encoder deltas, wrap-around safe as long as less than
/// 2^31 increments happen between two samples.
struct SpeedTracker {
    previous: Option<(u32, Instant)>,
    steps_per_second: f64,
}

impl SpeedTracker {
    fn new() -> Self {
        Self {
            previous: None,
            steps_per_second: 0.0,
        }
    }

    fn update(&mut self, encoder: u32, now: Instant) {
        if let Some((previous, at)) = self.previous {
            let elapsed = now.duration_since(at).as_secs_f64();
            if elapsed < 0.1 {
                return;
            }
            let delta = encoder.wrapping_sub(previous) as i32;
            self.steps_per_second = delta as f64 / elapsed;
        }
        self.previous = Some((encoder, now));
    }
}

// ── Panel ───────────────────────────────────────────────────────────────────

const RULE: &str = "──────────────────────────────────────────────────────────";

fn render(axis: &Axis, sample: &Sample, speed: &SpeedTracker, seconds: f64) -> String {
    let mut f = String::new();
    let remaining = axis.remaining(sample);

    let _ = writeln!(f, "{RULE}");
    let _ = writeln!(
        f,
        "  state            {:<20} {seconds:>8.1} s",
        axis.state.label()
    );
    let _ = writeln!(
        f,
        "  move             {:<20} {:>10}",
        match axis.last_requested {
            Some(steps) => format!("{steps:+} steps"),
            None => String::from("none yet"),
        },
        format!("pass {}/{}", axis.corrections, MAX_CORRECTIONS)
    );
    let _ = writeln!(f, "{RULE}");
    let _ = writeln!(f, "  encoder          0x6000:11  {:>14}", sample.encoder);
    let _ = writeln!(f, "  target                      {:>14}", axis.target);
    let _ = writeln!(f, "  remaining                   {remaining:>+14} steps");
    let _ = writeln!(
        f,
        "  generator        0x6020:11  {:>14}",
        sample.generator_position
    );
    let _ = writeln!(f, "{RULE}");
    let _ = writeln!(
        f,
        "  speed measured              {:>+14.1} steps/s",
        speed.steps_per_second
    );
    let _ = writeln!(
        f,
        "  speed commanded  0x6020:21  {:>+14} (~{:.0} steps/s)",
        sample.generator_velocity,
        velocity_to_steps(sample.generator_velocity)
    );
    let _ = writeln!(
        f,
        "  travel velocity             {:>14} (~{:.0} steps/s)",
        axis.velocity,
        velocity_to_steps(axis.velocity)
    );
    let _ = writeln!(
        f,
        "  drive time       0x6020:22  {:>14} ms",
        sample.drive_time
    );
    let _ = writeln!(
        f,
        "  drive state / velo          {:>#14x} / {}",
        sample.motor_load, sample.dc_current as i16
    );
    let _ = writeln!(f, "{RULE}");

    // Two columns of flags. The cells are always two visible characters wide,
    // so the padding stays correct even with the color escapes mixed in.
    let left = [
        ("busy", sample.busy, false),
        ("in target", sample.in_target, false),
        ("ready to execute", sample.ready_to_execute, false),
        ("calibrated", sample.calibrated, false),
        ("enabled", axis.enabled, false),
    ];
    let right = [
        ("drive ready", sample.drive_ready, false),
        ("drive error", sample.drive_error, true),
        ("drive warning", sample.drive_warning, true),
        ("pos error", sample.pos_error, true),
        ("pos warning", sample.pos_warning, true),
        ("motor stall", sample.motor_stall, true),
        ("moving +", sample.moving_positive, false),
        ("moving -", sample.moving_negative, false),
    ];

    for i in 0..left.len().max(right.len()) {
        match left.get(i) {
            Some(&(label, on, alert)) => {
                let _ = write!(f, "  {label:<18}{}", flag(on, alert));
            }
            // 18 label columns plus the 2 of the flag cell
            None => {
                let _ = write!(f, "  {:<20}", "");
            }
        }
        if let Some(&(label, on, alert)) = right.get(i) {
            let _ = write!(f, "    {label:<18}{}", flag(on, alert));
        }
        let _ = writeln!(f);
    }

    let _ = writeln!(f, "{RULE}");
    let _ = write!(f, "  {}", axis.message);

    f
}

/// Two visible characters: `ON` when set, a dimmed `--` when not. Flags that
/// signal trouble turn red so they stand out in the still panel.
fn flag(on: bool, alert: bool) -> String {
    match (on, alert) {
        (true, true) => "\x1b[1;31mON\x1b[0m".to_string(),
        (true, false) => "\x1b[1mON\x1b[0m".to_string(),
        (false, _) => "\x1b[2m--\x1b[0m".to_string(),
    }
}

const PROMPT: &str = "> ";

/// Draws the panel in place above a prompt line. Every refresh saves the cursor
/// (which sits after whatever the user has typed so far), rewrites the block
/// above it and jumps back, so typing is never clobbered by a redraw.
struct Screen {
    lines: usize,
}

impl Screen {
    fn new() -> Self {
        Self { lines: 0 }
    }

    /// Forget the current block, so the next draw paints a fresh one below.
    /// Needed after the user presses enter, because the terminal echoed a
    /// newline and everything shifted up by a line.
    fn reset(&mut self) {
        self.lines = 0;
    }

    fn is_reset(&self) -> bool {
        self.lines == 0
    }

    fn draw(&mut self, frame: &str) {
        let lines: Vec<&str> = frame.lines().collect();
        let mut out = String::new();

        if self.lines == 0 {
            for line in &lines {
                let _ = writeln!(out, "{line}\x1b[K");
            }
            out.push_str(PROMPT);
        } else {
            // Save the cursor, walk up to the first line of the block, overwrite
            // every line, then drop back onto the prompt line
            let _ = write!(out, "\x1b[s\x1b[{}A\r", self.lines);
            for (i, line) in lines.iter().enumerate() {
                let _ = write!(out, "{line}\x1b[K");
                // No newline after the last line: that would land on the prompt
                // line and could scroll the whole block
                if i + 1 < lines.len() {
                    out.push_str("\n\r");
                }
            }
            out.push_str("\x1b[u");
        }

        print!("{out}");
        let _ = std::io::stdout().flush();
        self.lines = lines.len();
    }
}
