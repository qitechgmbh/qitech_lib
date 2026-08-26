//! Minimal linear traverse on an EL7037 stepper terminal.
//!
//! A single axis that can be referenced against a home switch, taught a start
//! and an end point, and then driven to any position between them - in
//! millimetres, not encoder counts - while reporting how far along the move it
//! is, and while it can be paused and continued at any moment.
//!
//! ## What it demonstrates
//!
//! * software start/end points, used both as the interpolation endpoints and as
//!   the travel limits,
//! * a digital input used as a home switch, and a move aborted when it closes,
//! * a three-phase homing routine that defines the zero point,
//! * teaching the current position as the start or the end point,
//! * fractional positioning: `0` -> start, `1` -> end, `0.5` -> the midpoint,
//! * absolute positioning in real length units (the workspace `units` crate),
//! * a status report: unreferenced / homing / idle / moving(%) / paused(%) /
//!   disabled / error,
//! * pause and continue, with a progress percentage that survives the pause,
//! * cutting the current so the carriage can be pushed by hand, without losing
//!   the position: the encoder input is a separate block from the motor driver,
//!   so a disabled axis keeps counting and stays referenced. That is the easy
//!   way to teach the start and end points - push the carriage there and say
//!   `start` or `end`.
//!
//! ## How it works
//!
//! The terminal runs in `DirectVelocity` mode: it only ever applies the speed we
//! give it, and reports the encoder position back. The position loop is closed
//! here in Rust by [`VelocityPositionLoop`]. That helper is deliberately dumb -
//! it knows about a target, a position and a speed, and nothing else. Everything
//! this example adds (referencing, limits, units, pause/continue) sits on top of
//! it in [`Traverse`], which is likewise pure state: no device handle, no I/O.
//! `main` owns the EtherCAT cycle, feeds `Traverse` its feedback, and writes back
//! the speed it asks for.
//!
//! For the control law, the tunables and why they are what they are, see
//! `examples/el7037_velocity_closed_loop.rs`.
//!
//! ## Mechanical convention
//!
//! The home switch sits at the **negative** end of travel and defines position
//! `0`. Positive positions move away from it. If your machine is the other way
//! round, set `config.encoder.reversion_of_rotation` in [`axis_config`].
//!
//! ## Wiring
//!
//! * Motor + encoder on the EL7037 as usual.
//! * Home switch on **digital input 1**. It is configured as a plain input, so
//!   the terminal itself ignores it and this program decides what to do.
//!
//! ## Usage
//!
//! ```text
//! cargo run --example el7037_linear_traverse -- <interface>
//! ```
//!
//! Then type `?` for the command list.

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
        velocity_position_loop::{LoopEvent, VelocityPositionLoop, VelocityPositionLoopConfig},
    },
    init_ethercat,
    io::stepper_velocity_el70x1::StepperVelocityEL70x1Device,
    shared_config::el70x7::{
        EL70x1InputFunction, EL70x1OperationMode, EL70x1SpeedRange, EL7037FeedbackType,
    },
};
use std::{
    env, fmt,
    io::Write,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};
use units::{
    Length, Velocity,
    length::{centimeter, meter, millimeter},
    velocity::millimeter_per_second,
};

// ── Machine constants ───────────────────────────────────────────────────────
// Everything below the EtherCAT layer counts in encoder counts. These four
// constants are the only place where counts, revolutions and millimetres meet.

/// Encoder resolution. igus MOT-AN-S-060-005-042-L-C-AAAO: 1000 pulses/rev read
/// 4-fold.
const COUNTS_PER_REV: f64 = 4000.0;

/// **Set this for your mechanics.** How far the carriage travels per motor
/// revolution: the lead-screw pitch, or the belt pitch times the pulley tooth
/// count. Everything the program prints or accepts in millimetres depends on it.
const MM_PER_REV: f64 = 8.0;

const COUNTS_PER_MM: f64 = COUNTS_PER_REV / MM_PER_REV;

/// 4000 counts/rev over a 200 full-step motor.
const COUNTS_PER_FULL_STEP: f64 = COUNTS_PER_REV / 200.0;

/// Ceiling of the configured speed range (`EL70x1SpeedRange::Steps2000`).
const MAX_FULL_STEPS_PER_S: f64 = 2000.0;

// ── Tuning ──────────────────────────────────────────────────────────────────

/// EtherCAT cycle. Every number below that is "per second" is integrated at
/// this rate.
const CYCLE_TIME: Duration = Duration::from_nanos(250_000);

/// Redraw the status line every N cycles (~12 ms at 250 us).
const STATUS_EVERY: u32 = 50;

/// Cruise speed for ordinary moves, in mm/s. Changeable at runtime with `v`.
const DEFAULT_CRUISE_MM_S: f64 = 25.0;

/// End point assumed right after homing, until you teach a real one.
const DEFAULT_TRAVEL_MM: f64 = 100.0;

/// First, fast run at the home switch.
const HOMING_FAST_MM_S: f64 = 10.0;

/// Back-off and final approach. Slow, because this is the speed that decides how
/// repeatable the zero point is.
const HOMING_SLOW_MM_S: f64 = 1.0;

/// How far to pull clear of the switch before the final approach.
const HOMING_BACKOFF_MM: f64 = 2.0;

/// Slew limit for the homing moves, in mm/s^2. Homing drives the motor directly
/// rather than through the position loop, so it needs its own ramp - a stepper
/// given a step change in speed simply loses steps.
const HOMING_ACCEL_MM_S2: f64 = 80.0;

/// Give up on homing if the switch has not been found in this long.
const HOMING_TIMEOUT: Duration = Duration::from_secs(60);

/// After zeroing the encoder, wait this long before accepting a move. Writing
/// the counter is asynchronous - the terminal has to echo the new value back
/// before `get_position` reports it.
const HOME_SETTLE: Duration = Duration::from_millis(300);

/// The terminal clears its own error bit (see `EL7037::output_pre_process`), so
/// a momentary error is normal. Only fault if it stays set this long.
const DRIVE_ERROR_GRACE: Duration = Duration::from_millis(250);

// ── Device configuration ────────────────────────────────────────────────────

/// CoE configuration written while the bus is in PreOp.
///
/// The motor values are for the igus NEMA 17 on the reference rig - replace them
/// with your own motor's datasheet figures. The controller gains and the
/// `reversion_of_rotation` flag were measured on that rig; see
/// `examples/el7037_velocity_closed_loop.rs`.
fn axis_config() -> EL7037Configuration {
    let mut config = EL7037Configuration::default();

    // Speed-only mode: the terminal applies what we command and nothing else.
    // The position loop lives in this program.
    config.stm_features.operation_mode = EL70x1OperationMode::DirectVelocity;
    config.stm_features.speed_range = EL70x1SpeedRange::Steps2000;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    config.encoder.reversion_of_rotation = true;

    // The home switch. The default for a digital input is `PlcCam`, which makes
    // the terminal's *own* travel generator react to it - not what we want here,
    // where this program owns every decision. `NormalInput` just reports the
    // level in the process image.
    config.stm_features.function_for_input_1 = EL70x1InputFunction::NormalInput;
    // Set this to `true` for a normally-closed switch, so that a broken wire
    // reads as "triggered" instead of "clear".
    config.stm_features.invert_digital_input_1 = false;

    config.stm_motor.max_current = 1100;
    config.stm_motor.reduced_current = 550;
    config.stm_motor.nominal_voltage = 24000;
    config.stm_motor.motor_coil_resistance = 175;
    config.stm_motor.motor_coil_inductance = 330;
    config.stm_motor.motor_full_steps = 200;
    config.stm_motor.encoder_increments = 4000;

    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = 10;
    config.stm_controller_3.feed_forward_pos = 100_000;
    config.stm_controller_3.kp_factor_pos = 2;
    config.stm_controller_3.kp_factor_velo = 50;
    config.stm_controller_3.tn_velo = 50_000;

    config.pdo_assignment = EL7037PredefinedPdoAssignment::VelocityControlCompact;
    config
}

// ── Units ───────────────────────────────────────────────────────────────────
// The `units` crate is used at the edges - what the user types, what the program
// prints, what the public methods of `Traverse` take. The control loop itself
// works in plain encoder counts, because that is what the encoder produces and
// converting on every cycle would only add rounding.

fn counts_from_length(length: Length) -> i64 {
    (length.get::<millimeter>() * COUNTS_PER_MM).round() as i64
}

fn length_from_counts(counts: i64) -> Length {
    Length::new::<millimeter>(counts as f64 / COUNTS_PER_MM)
}

fn counts_per_s_from_velocity(velocity: Velocity) -> f64 {
    velocity.get::<millimeter_per_second>() * COUNTS_PER_MM
}

/// Millimetres of a position, for printing.
fn mm(counts: i64) -> f64 {
    length_from_counts(counts).get::<millimeter>()
}

// ── State machine ───────────────────────────────────────────────────────────

/// Where the axis is in its life cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraverseState {
    /// Never homed, so the position is meaningless. Only `home` is accepted.
    Unreferenced,
    /// The homing routine is running.
    Homing(HomingPhase),
    /// Referenced and standing still.
    Idle,
    /// Running a move towards `move_target`.
    Moving,
    /// The move is suspended. The motor has ramped to a stop, but the target and
    /// the progress are remembered and `continue` picks it up again.
    Paused,
    /// Something went wrong. Clear it with `reset`, or re-home.
    Fault(&'static str),
}

/// The homing routine, one state per phase.
///
/// Two approaches rather than one: the first is fast so that homing does not
/// take forever from the far end of the axis, the second is slow so that the
/// point at which the switch trips - which is what becomes zero - does not
/// depend on how fast the carriage was moving when it got there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomingPhase {
    /// Driving at the switch quickly.
    Approach,
    /// Switch found; backing off until it releases again. `from` is where the
    /// back-off started, so we can guarantee a minimum clearance.
    BackOff { from: i64 },
    /// Creeping back at the switch to find its trip point precisely.
    ReApproach,
    /// Standing still while the encoder counter is zeroed.
    Settle { until: Instant },
}

/// What [`Traverse::status`] reports. This is the "motor status" of the issue:
/// idle, error, or moving with a percentage.
#[derive(Debug, Clone, Copy, PartialEq)]
enum MotorStatus {
    Unreferenced,
    Homing,
    Idle,
    Moving {
        percent: f64,
    },
    Paused {
        percent: f64,
    },
    Error(&'static str),
    /// The driver stage is off and the carriage can be pushed by hand.
    /// `referenced` says whether the position it reports still means anything.
    Disabled {
        referenced: bool,
    },
}

impl fmt::Display for MotorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MotorStatus::Unreferenced => write!(f, "unreferenced"),
            MotorStatus::Homing => write!(f, "homing"),
            MotorStatus::Idle => write!(f, "idle"),
            MotorStatus::Moving { percent } => write!(f, "moving {percent:5.1}%"),
            MotorStatus::Paused { percent } => write!(f, "paused {percent:5.1}%"),
            MotorStatus::Error(reason) => write!(f, "error: {reason}"),
            MotorStatus::Disabled { referenced: true } => write!(f, "disabled"),
            MotorStatus::Disabled { referenced: false } => write!(f, "disabled, unref"),
        }
    }
}

/// Something the caller has to know about, returned at most once per cycle.
#[derive(Debug, Clone, Copy)]
enum Event {
    /// The homing routine has found the switch. **The caller must zero the
    /// encoder counter now** - `Traverse` has no device handle of its own.
    AtHome,
    /// Homing finished; the axis is referenced and ready.
    Referenced,
    /// A move finished. `residual` is `position - target` in counts.
    Arrived { residual: i64 },
    /// The axis stopped for a reason that needs the operator's attention.
    Faulted(&'static str),
}

/// One cycle's worth of feedback.
#[derive(Debug, Clone, Copy)]
struct Feedback {
    /// Encoder position in counts.
    position: i64,
    /// The terminal's stall detection.
    stalled: bool,
    /// Digital input 1: the home switch, `true` while it is pressed.
    home_switch: bool,
    /// The terminal's error bit.
    drive_error: bool,
}

// ── The traverse ────────────────────────────────────────────────────────────

/// A referenced linear axis on top of [`VelocityPositionLoop`].
///
/// Pure state: it never talks to the terminal. Feed it [`Feedback`] once per
/// cycle via [`Traverse::step`], write the speed it returns, and act on the
/// [`Event`] it hands back.
struct Traverse {
    /// The closed position loop doing the actual regulating.
    axis: VelocityPositionLoop,
    state: TraverseState,

    /// Whether the driver stage should be energised. Orthogonal to `state`: the
    /// encoder input of the EL7037 is a separate block from the motor driver, so
    /// cutting the current does not stop the position from being tracked. That is
    /// what makes teaching a point by pushing the carriage there possible.
    enabled: bool,

    /// Software travel limits, and the two endpoints that `f <x>` interpolates
    /// between. Counts from home.
    start: i64,
    end: i64,

    /// Where the current move began and where it is going. Kept here rather than
    /// read out of `axis`, because `VelocityPositionLoop::start_move` resets its
    /// own idea of the starting point on every call - including on a resume,
    /// which would snap the progress percentage back to 0 %.
    move_from: i64,
    move_target: i64,

    /// Last speed handed to the caller, in counts/s. The homing phases ramp from
    /// it, and the travel-limit check needs to know which way we are going.
    speed: f64,

    homing_deadline: Instant,
    /// Set while the terminal's error bit is up, so a momentary error that the
    /// terminal clears by itself does not fault the axis.
    error_since: Option<Instant>,
}

impl Traverse {
    fn new(now: Instant) -> Self {
        // The defaults were measured on this motor; see the helper's docs. Only
        // the cruise speed is ours, and `v` changes it at runtime.
        let config = VelocityPositionLoopConfig {
            max_speed: DEFAULT_CRUISE_MM_S * COUNTS_PER_MM,
            ..Default::default()
        };
        Self {
            axis: VelocityPositionLoop::new(config, 0, now),
            state: TraverseState::Unreferenced,
            enabled: true,
            start: 0,
            end: (DEFAULT_TRAVEL_MM * COUNTS_PER_MM) as i64,
            move_from: 0,
            move_target: 0,
            speed: 0.0,
            homing_deadline: now,
            error_since: None,
        }
    }

    // ── Reporting ───────────────────────────────────────────────────────────

    /// The motor status, including how far along the current move we are.
    fn status(&self, position: i64) -> MotorStatus {
        if !self.enabled {
            return MotorStatus::Disabled {
                referenced: self.referenced(),
            };
        }
        match self.state {
            TraverseState::Unreferenced => MotorStatus::Unreferenced,
            TraverseState::Homing(_) => MotorStatus::Homing,
            TraverseState::Idle => MotorStatus::Idle,
            TraverseState::Moving => MotorStatus::Moving {
                percent: self.progress(position) * 100.0,
            },
            TraverseState::Paused => MotorStatus::Paused {
                percent: self.progress(position) * 100.0,
            },
            TraverseState::Fault(reason) => MotorStatus::Error(reason),
        }
    }

    /// Fraction of the current move that is done, `0.0 ..= 1.0`.
    ///
    /// Measured against the distance actually commanded, so it is unaffected by
    /// a pause: pausing leaves `move_from` alone, and continuing carries on from
    /// the same percentage rather than restarting at zero.
    fn progress(&self, position: i64) -> f64 {
        let span = (self.move_target - self.move_from).abs();
        if span == 0 {
            return 1.0;
        }
        ((position - self.move_from).abs() as f64 / span as f64).clamp(0.0, 1.0)
    }

    /// Whether the encoder zero still means something. Note this stays true
    /// while the motor is disabled - that is the whole point of disabling it.
    fn referenced(&self) -> bool {
        matches!(
            self.state,
            TraverseState::Idle | TraverseState::Moving | TraverseState::Paused
        )
    }

    /// Whether `main` should energise the driver stage this cycle.
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn target(&self) -> i64 {
        self.move_target
    }

    fn limits(&self) -> (i64, i64) {
        (self.start.min(self.end), self.start.max(self.end))
    }

    // ── Commands ────────────────────────────────────────────────────────────

    /// Cuts the current to the motor so the carriage can be pushed by hand.
    ///
    /// The encoder keeps counting, so the axis stays referenced: push the
    /// carriage where you want a point and `start`/`end` will record it. Any
    /// running move is abandoned first, so that the loop is not left holding a
    /// target it would snap back to the moment the current returns.
    fn disable(&mut self) {
        self.stop();
        self.enabled = false;
        self.speed = 0.0;
    }

    /// Re-energises the driver stage, wherever the carriage now is. The loop is
    /// retargeted to the current position, so nothing jumps back to where the
    /// move was going before the motor was freed.
    fn enable(&mut self, position: i64) {
        self.enabled = true;
        self.axis.stop();
        self.move_from = position;
        self.move_target = position;
        self.error_since = None;
    }

    /// Starts the homing routine. Refused while a move is running - stop first.
    fn home(&mut self, now: Instant) -> Result<(), &'static str> {
        if !self.enabled {
            return Err("the motor is disabled - `enable` first");
        }
        if matches!(self.state, TraverseState::Moving) {
            return Err("refusing to home during a move - stop it first");
        }
        self.axis.stop();
        self.state = TraverseState::Homing(HomingPhase::Approach);
        self.homing_deadline = now + HOMING_TIMEOUT;
        Ok(())
    }

    /// Sets the start point to an absolute position measured from home.
    fn set_start(&mut self, at: Length) {
        self.start = counts_from_length(at);
    }

    fn set_end(&mut self, at: Length) {
        self.end = counts_from_length(at);
    }

    /// Teaches the point the carriage is standing on right now - jog it there by
    /// hand or with `go`, then call this.
    fn teach_start(&mut self, position: i64) -> Result<(), &'static str> {
        if !self.referenced() {
            return Err("not referenced - home first");
        }
        self.start = position;
        Ok(())
    }

    fn teach_end(&mut self, position: i64) -> Result<(), &'static str> {
        if !self.referenced() {
            return Err("not referenced - home first");
        }
        self.end = position;
        Ok(())
    }

    /// Moves to an absolute position measured from home, clamped to the travel
    /// limits. Returns the position it will actually go to.
    fn move_to(&mut self, to: Length, position: i64, now: Instant) -> Result<i64, &'static str> {
        if !self.enabled {
            return Err("the motor is disabled - `enable` first");
        }
        if !self.referenced() {
            return Err("not referenced - home first");
        }
        let (low, high) = self.limits();
        let target = counts_from_length(to).clamp(low, high);

        self.move_from = position;
        self.move_target = target;
        self.axis.start_move(target, position, now);
        self.state = TraverseState::Moving;
        Ok(target)
    }

    /// Moves to a point interpolated between the start and the end point:
    /// `0.0` is the start, `1.0` the end, `0.5` the midpoint. Values outside
    /// `0..=1` are clamped, so this can never leave the taught span.
    fn move_to_fraction(
        &mut self,
        fraction: f64,
        position: i64,
        now: Instant,
    ) -> Result<i64, &'static str> {
        let fraction = fraction.clamp(0.0, 1.0);
        let span = (self.end - self.start) as f64;
        let target = length_from_counts(self.start + (fraction * span).round() as i64);
        self.move_to(target, position, now)
    }

    /// Ramps to a stop but keeps the target, so `resume` can pick the move back
    /// up. The progress percentage freezes rather than resetting.
    fn pause(&mut self) -> Result<(), &'static str> {
        if self.state != TraverseState::Moving {
            return Err("no move to pause");
        }
        // `stop` only takes the loop out of the moving state; the speed profile
        // still ramps down at the acceleration limit on the following cycles.
        self.axis.stop();
        self.state = TraverseState::Paused;
        Ok(())
    }

    /// Continues a paused move towards the same target. `move_from` is left
    /// alone so the percentage carries on where it stopped, while the loop's own
    /// runaway guard is re-armed from where the carriage is now.
    fn resume(&mut self, position: i64, now: Instant) -> Result<(), &'static str> {
        if !self.enabled {
            return Err("the motor is disabled - `enable` first");
        }
        if self.state != TraverseState::Paused {
            return Err("nothing is paused");
        }
        self.axis.start_move(self.move_target, position, now);
        self.state = TraverseState::Moving;
        Ok(())
    }

    /// Abandons the move. Unlike a move that ends by arriving, this leaves the
    /// loop disengaged: it will not correct the position afterwards.
    fn stop(&mut self) {
        match self.state {
            TraverseState::Moving | TraverseState::Paused => self.state = TraverseState::Idle,
            // Homing was interrupted, so zero was never found: the position is
            // meaningless again.
            TraverseState::Homing(_) => self.state = TraverseState::Unreferenced,
            _ => {}
        }
        self.axis.stop();
    }

    /// Clears a fault. The axis stays referenced, so no re-homing is needed
    /// unless the fault was a stall or a runaway, where the encoder may no
    /// longer agree with the mechanics.
    fn reset(&mut self) {
        if let TraverseState::Fault(_) = self.state {
            self.state = TraverseState::Idle;
        }
        self.error_since = None;
    }

    fn set_cruise(&mut self, speed: Velocity) {
        self.axis.set_max_speed(counts_per_s_from_velocity(speed));
    }

    /// Ramps to a stop regardless of the target, for shutting down.
    fn ramp_down(&mut self, dt: f64) -> f64 {
        let loop_speed = self.axis.ramp_down(dt);
        // Whichever of the two is actually steering has to come down: the loop's
        // own profile during a move, our hand-driven speed during homing.
        self.speed = if matches!(self.state, TraverseState::Homing(_)) {
            slew(self.speed, 0.0, dt)
        } else {
            loop_speed
        };
        self.speed
    }

    /// Whether the commanded speed has actually reached zero.
    fn stopped(&self) -> bool {
        self.speed == 0.0 && self.axis.stopped()
    }

    fn fault(&mut self, reason: &'static str) -> Option<Event> {
        self.axis.stop();
        self.state = TraverseState::Fault(reason);
        Some(Event::Faulted(reason))
    }

    // ── The cycle ───────────────────────────────────────────────────────────

    /// Advances the axis by one cycle. Returns the speed to command, in counts/s,
    /// and at most one event.
    fn step(&mut self, feedback: Feedback, now: Instant, dt: f64) -> (f64, Option<Event>) {
        let Feedback {
            position,
            stalled,
            home_switch,
            drive_error,
        } = feedback;

        // Free-wheeling. Command nothing, judge nothing - a de-energised drive
        // being pushed around would otherwise look like a runaway. The encoder
        // is still counting, so the position stays valid throughout.
        if !self.enabled {
            self.axis.ramp_down(dt);
            self.speed = 0.0;
            return (0.0, None);
        }

        // The terminal resets its own error bit, so only a persistent error is
        // worth faulting on.
        if drive_error {
            let since = *self.error_since.get_or_insert(now);
            if now.duration_since(since) > DRIVE_ERROR_GRACE
                && !matches!(self.state, TraverseState::Fault(_))
            {
                let event = self.fault("the terminal is reporting an error");
                let (speed, _) = self.axis.step(position, stalled, now, dt);
                self.speed = speed;
                return (speed, event);
            }
        } else {
            self.error_since = None;
        }

        // Outside the homing routine the home switch is a hard travel limit: the
        // moment it closes while we are driving towards it, the move is over.
        if home_switch
            && self.speed < 0.0
            && !matches!(
                self.state,
                TraverseState::Homing(_) | TraverseState::Fault(_)
            )
        {
            let event = self.fault("home switch hit during a move");
            let (speed, _) = self.axis.step(position, stalled, now, dt);
            self.speed = speed;
            return (speed, event);
        }

        match self.state {
            TraverseState::Homing(phase) => self.step_homing(phase, position, home_switch, now, dt),
            _ => {
                let (speed, event) = self.axis.step(position, stalled, now, dt);
                self.speed = speed;
                (speed, self.translate(event))
            }
        }
    }

    /// Turns a [`LoopEvent`] into our own vocabulary and updates the state.
    fn translate(&mut self, event: Option<LoopEvent>) -> Option<Event> {
        match event? {
            LoopEvent::Arrived { residual, .. } => {
                if self.state == TraverseState::Moving {
                    // The loop is now holding this position and will correct it
                    // if the carriage is pushed off.
                    self.state = TraverseState::Idle;
                }
                Some(Event::Arrived { residual })
            }
            LoopEvent::Runaway { .. } => self.fault("runaway - travelled far past the target"),
            LoopEvent::TimedOut { .. } => self.fault("timed out before reaching the target"),
            // The loop noticed the carriage was pushed off its held position and
            // is correcting it. Nothing for the operator to do.
            LoopEvent::Drifted { .. } => None,
        }
    }

    /// One cycle of the homing routine. Homing drives the motor directly instead
    /// of through the position loop, because until the switch is found there is
    /// no target to drive to.
    fn step_homing(
        &mut self,
        phase: HomingPhase,
        position: i64,
        home_switch: bool,
        now: Instant,
        dt: f64,
    ) -> (f64, Option<Event>) {
        // Keep the position loop's speed profile drained to zero while we steer
        // by hand, so that handing control back after homing does not start from
        // a stale speed.
        self.axis.ramp_down(dt);

        if now > self.homing_deadline {
            let event = self.fault("homing timed out - is the switch connected?");
            self.speed = 0.0;
            return (0.0, event);
        }

        let fast = HOMING_FAST_MM_S * COUNTS_PER_MM;
        let slow = HOMING_SLOW_MM_S * COUNTS_PER_MM;
        let backoff = (HOMING_BACKOFF_MM * COUNTS_PER_MM) as i64;

        let mut event = None;
        let demand = match phase {
            // Run at the switch. If it is already pressed this falls straight
            // through to the back-off, which is exactly right.
            HomingPhase::Approach => {
                if home_switch {
                    self.state = TraverseState::Homing(HomingPhase::BackOff { from: position });
                    0.0
                } else {
                    -fast
                }
            }
            // Pull clear: both the switch has to release and a minimum clearance
            // has to be travelled, so the final approach always starts from
            // outside the switch's hysteresis.
            HomingPhase::BackOff { from } => {
                if !home_switch && position - from >= backoff {
                    self.state = TraverseState::Homing(HomingPhase::ReApproach);
                    0.0
                } else {
                    slow
                }
            }
            // Creep back on. Where it trips is the zero point - the caller has to
            // write it into the encoder counter for us.
            HomingPhase::ReApproach => {
                if home_switch {
                    self.state = TraverseState::Homing(HomingPhase::Settle {
                        until: now + HOME_SETTLE,
                    });
                    event = Some(Event::AtHome);
                    0.0
                } else {
                    -slow
                }
            }
            // Stand still until the new counter value has made it through the
            // process image, then hand control back to the position loop.
            HomingPhase::Settle { until } => {
                if now >= until && self.speed == 0.0 {
                    self.start = 0;
                    self.end = (DEFAULT_TRAVEL_MM * COUNTS_PER_MM) as i64;
                    self.move_from = 0;
                    self.move_target = 0;
                    self.axis.home();
                    self.state = TraverseState::Idle;
                    event = Some(Event::Referenced);
                }
                0.0
            }
        };

        self.speed = slew(self.speed, demand, dt);
        (self.speed, event)
    }
}

/// Ramps `current` towards `demand` at the homing acceleration limit. A stepper
/// handed a step change in speed just loses steps, so even the homing moves get
/// a ramp.
fn slew(current: f64, demand: f64, dt: f64) -> f64 {
    let step = HOMING_ACCEL_MM_S2 * COUNTS_PER_MM * dt;
    let next = current + (demand - current).clamp(-step, step);
    // Snap to zero instead of dithering around it.
    if demand == 0.0 && next.abs() < step {
        return 0.0;
    }
    next
}

// ── Commands ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Command {
    Home,
    TeachStart,
    TeachEnd,
    SetStart(Length),
    SetEnd(Length),
    Go(Length),
    Fraction(f64),
    Cruise(Velocity),
    Pause,
    Resume,
    Stop,
    Reset,
    Enable,
    Disable,
    Status,
    Help,
    Quit,
}

const HELP: &str = "\
  home           reference the axis against the home switch and call it 0
  start          teach the current position as the start point
  end            teach the current position as the end point
  start <len>    set the start point to an absolute position
  end <len>      set the end point to an absolute position
  go <len>       move to an absolute position, clamped to start..end
  f <0..1>       move between the points: 0 = start, 1 = end, 0.5 = middle
  v <mm/s>       cruise speed
  p              pause the running move (ramps to a stop, keeps the target)
  c              continue the paused move
  s              abandon the move
  disable        cut the current so you can push the carriage by hand - the
                 encoder keeps counting, so `start`/`end` still work
  enable         energise the motor again, wherever the carriage now is
  reset          clear a fault
  status         print the status once
  ?              this help
  q              quit

  <len> is millimetres unless you say otherwise: 42  42mm  4.2cm  0.042m";

/// Parses a length. Bare numbers are millimetres.
fn parse_length(text: &str) -> Result<Length, String> {
    let text = text.trim();
    // Longest suffix first, or "mm" would be read as "m".
    let (number, unit): (&str, fn(f64) -> Length) = if let Some(rest) = text.strip_suffix("mm") {
        (rest, Length::new::<millimeter>)
    } else if let Some(rest) = text.strip_suffix("cm") {
        (rest, Length::new::<centimeter>)
    } else if let Some(rest) = text.strip_suffix('m') {
        (rest, Length::new::<meter>)
    } else {
        (text, Length::new::<millimeter>)
    };
    let value: f64 = number
        .trim()
        .parse()
        .map_err(|_| format!("not a length: {text:?}"))?;
    Ok(unit(value))
}

fn parse_command(line: &str) -> Result<Option<Command>, String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }
    match line {
        "home" | "h" => return Ok(Some(Command::Home)),
        "start" => return Ok(Some(Command::TeachStart)),
        "end" => return Ok(Some(Command::TeachEnd)),
        "p" | "pause" => return Ok(Some(Command::Pause)),
        "c" | "continue" | "resume" => return Ok(Some(Command::Resume)),
        "s" | "stop" => return Ok(Some(Command::Stop)),
        "d" | "disable" | "free" => return Ok(Some(Command::Disable)),
        "e" | "enable" => return Ok(Some(Command::Enable)),
        "reset" => return Ok(Some(Command::Reset)),
        "status" => return Ok(Some(Command::Status)),
        "?" | "help" => return Ok(Some(Command::Help)),
        "q" | "quit" => return Ok(Some(Command::Quit)),
        _ => {}
    }

    let (verb, argument) = line
        .split_once(char::is_whitespace)
        .ok_or_else(|| format!("unknown command: {line:?} (try ?)"))?;

    Ok(Some(match verb {
        "start" => Command::SetStart(parse_length(argument)?),
        "end" => Command::SetEnd(parse_length(argument)?),
        "go" => Command::Go(parse_length(argument)?),
        "f" => {
            let fraction: f64 = argument
                .trim()
                .parse()
                .map_err(|_| format!("not a number: {argument:?}"))?;
            Command::Fraction(fraction)
        }
        "v" => {
            let speed: f64 = argument
                .trim()
                .parse()
                .map_err(|_| format!("not a number: {argument:?}"))?;
            if speed <= 0.0 {
                return Err("speed must be positive".into());
            }
            Command::Cruise(Velocity::new::<millimeter_per_second>(speed))
        }
        _ => return Err(format!("unknown command: {line:?} (try ?)")),
    }))
}

/// Applies a parsed command. Returns `true` if the program should quit.
fn apply_command(
    command: Command,
    traverse: &mut Traverse,
    feedback: Feedback,
    now: Instant,
) -> bool {
    let position = feedback.position;

    /// Prints either what happened or why it could not.
    fn report(result: Result<(), &'static str>, ok: &str) {
        match result {
            Ok(()) => println!("{ok}"),
            Err(reason) => println!("{reason}"),
        }
    }

    match command {
        Command::Home => report(traverse.home(now), "homing"),
        Command::TeachStart => report(
            traverse.teach_start(position),
            &format!("start point taught at {:.2} mm", mm(position)),
        ),
        Command::TeachEnd => report(
            traverse.teach_end(position),
            &format!("end point taught at {:.2} mm", mm(position)),
        ),
        Command::SetStart(at) => {
            traverse.set_start(at);
            println!("start point at {:.2} mm", at.get::<millimeter>());
        }
        Command::SetEnd(at) => {
            traverse.set_end(at);
            println!("end point at {:.2} mm", at.get::<millimeter>());
        }
        Command::Go(to) => match traverse.move_to(to, position, now) {
            Ok(target) => {
                let asked = to.get::<millimeter>();
                let going = mm(target);
                if (going - asked).abs() > 0.005 {
                    println!(
                        "{asked:.2} mm is outside start..end - going to {going:.2} mm instead"
                    );
                } else {
                    println!("moving to {going:.2} mm");
                }
            }
            Err(reason) => println!("{reason}"),
        },
        Command::Fraction(fraction) => match traverse.move_to_fraction(fraction, position, now) {
            Ok(target) => println!(
                "moving to {fraction:.3} of start..end = {:.2} mm",
                mm(target)
            ),
            Err(reason) => println!("{reason}"),
        },
        Command::Cruise(speed) => {
            traverse.set_cruise(speed);
            println!(
                "cruise speed {:.1} mm/s",
                speed.get::<millimeter_per_second>()
            );
        }
        Command::Pause => report(traverse.pause(), "paused"),
        Command::Resume => report(traverse.resume(position, now), "continuing"),
        Command::Stop => {
            traverse.stop();
            println!("stopping");
        }
        Command::Disable => {
            traverse.disable();
            println!("motor disabled - push the carriage by hand, the encoder keeps counting");
        }
        Command::Enable => {
            traverse.enable(position);
            println!("motor enabled at {:.2} mm", mm(position));
        }
        Command::Reset => {
            traverse.reset();
            println!("fault cleared");
        }
        Command::Status => println!("{}", status_line(traverse, feedback)),
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
            // stdin closed (a piped script ran out, or Ctrl-D): shut down tidily
            // rather than leaving the axis energised.
            let _ = sender.send(String::from("q"));
        })
        .expect("could not spawn the input thread");
    receiver
}

// ── Reporting ───────────────────────────────────────────────────────────────

fn status_line(traverse: &Traverse, feedback: Feedback) -> String {
    format!(
        "{:<16} pos {:>8.2} mm | target {:>8.2} mm | v {:>6.1} mm/s | home {} | start {:.2} end {:.2} mm",
        traverse.status(feedback.position).to_string(),
        mm(feedback.position),
        mm(traverse.target()),
        traverse.speed / COUNTS_PER_MM,
        if feedback.home_switch { "ON " } else { "off" },
        mm(traverse.start),
        mm(traverse.end),
    )
}

fn report_event(event: Event, traverse: &Traverse, position: i64) {
    // The status line is redrawn in place, so clear it before printing over it.
    print!("\r{:<118}\r", "");
    match event {
        Event::AtHome => println!("home switch found - zeroing the encoder"),
        Event::Referenced => println!(
            "referenced. start {:.2} mm, end {:.2} mm - teach your own with `start`/`end`",
            mm(traverse.start),
            mm(traverse.end)
        ),
        Event::Arrived { residual } => println!(
            "arrived at {:.2} mm ({residual:+} counts = {:+.3} mm off)",
            mm(position),
            residual as f64 / COUNTS_PER_MM
        ),
        Event::Faulted(reason) => println!("stopped: {reason}"),
    }
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let interface = env::args()
        .nth(1)
        .expect("usage: el7037_linear_traverse <interface>");

    // ── Bring the bus up and configure the terminal in PreOp ────────────────
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
        address = eth
            .app_handle
            .try_get_subdevices_vec_sync()
            .expect("Failed to read subdevices!")
            .iter()
            .find(|s| s.vendor == BECKHOFF_VENDOR_ID && s.product_id == EL7037_PRODUCT_ID)
            .map(|s| s.device_address);
        if address.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let address = address.unwrap_or_else(|| panic!("no EL7037 found on {interface}"));
    el7037
        .write_config(eth.channel.clone(), address, &axis_config())
        .expect("EL7037 CoE config failed");

    eth.channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    while eth.app_handle.get_state() != EtherCATState::Op {
        std::thread::sleep(Duration::from_millis(10));
    }

    // Where this terminal's data sits in the process image.
    let subdevice = eth
        .app_handle
        .try_get_subdevices_vec_sync()
        .expect("Failed to read subdevices!")
        .into_iter()
        .find(|s| s.device_address == address)
        .expect("EL7037 disappeared during the Op transition");
    let (start_tx, end_tx) = (subdevice.start_tx, subdevice.end_tx);
    let (start_rx, end_rx) = (subdevice.start_rx, subdevice.end_rx);

    // ── One EtherCAT cycle: read the inputs, write the outputs ──────────────
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

    // ── Wait for the driver stage, then enable it ───────────────────────────
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        cycle(&mut el7037);
        let input = el7037.get_input(0).expect("velocity input unavailable");
        if input.ready_to_enable {
            el7037.set_enabled(0, true);
        }
        if input.ready && el7037.is_enabled(0) {
            break;
        }
        assert!(Instant::now() <= deadline, "drive never became ready");
    }

    println!(
        "EL7037 ready. {MM_PER_REV} mm/rev = {COUNTS_PER_MM:.0} counts/mm, \
         top speed {:.0} mm/s.",
        MAX_FULL_STEPS_PER_S * COUNTS_PER_FULL_STEP / COUNTS_PER_MM
    );
    println!("The axis is unreferenced - type `home` to start. `?` for help.");

    // ── The control loop ────────────────────────────────────────────────────
    let commands = spawn_input_thread();
    let mut traverse = Traverse::new(Instant::now());
    let mut last = Instant::now();
    let mut cycle_count: u32 = 0;

    loop {
        cycle(&mut el7037);

        let now = Instant::now();
        // Clamped so a scheduling hiccup cannot turn into a huge acceleration
        // step, and a zero-length cycle cannot divide by zero.
        let dt = now
            .duration_since(last)
            .as_secs_f64()
            .clamp(0.000_005, 0.05);
        last = now;

        let input = el7037.get_input(0).expect("velocity input unavailable");
        let feedback = Feedback {
            position: el7037.get_position(0) as i64,
            // Not carried by `StepperVelocityEL70x1Input`, so read straight off
            // the status PDO.
            stalled: el7037
                .txpdo
                .stm_status
                .as_ref()
                .is_some_and(|status| status.motor_stall),
            home_switch: el7037.get_digital_input(0).unwrap_or(false),
            drive_error: input.error,
        };

        match commands.try_recv() {
            Ok(line) => match parse_command(&line) {
                Ok(Some(command)) => {
                    if apply_command(command, &mut traverse, feedback, now) {
                        break;
                    }
                }
                Ok(None) => {}
                Err(message) => println!("{message}"),
            },
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break,
        }

        let (speed, event) = traverse.step(feedback, now, dt);
        command_speed(&mut el7037, speed);
        // Written every cycle rather than only on the `enable`/`disable`
        // commands, so the process image can never disagree with the state
        // machine about whether the motor should be holding.
        el7037.set_enabled(0, traverse.enabled());

        if let Some(event) = event {
            report_event(event, &traverse, feedback.position);
            // Homing asks us to define the zero point; only `main` can, because
            // `Traverse` has no device handle.
            if matches!(event, Event::AtHome) {
                el7037.set_position(0, 0);
            }
        }

        if cycle_count.is_multiple_of(STATUS_EVERY) {
            print!("\r{:<118}", status_line(&traverse, feedback));
            let _ = std::io::stdout().flush();
        }
        cycle_count = cycle_count.wrapping_add(1);
    }

    // ── Shut down: ramp to a stop rather than dropping the drive at speed ───
    println!();
    traverse.stop();
    while !traverse.stopped() {
        cycle(&mut el7037);
        let speed = traverse.ramp_down(CYCLE_TIME.as_secs_f64());
        command_speed(&mut el7037, speed);
    }
    command_speed(&mut el7037, 0.0);
    el7037.set_enabled(0, false);
    for _ in 0..50 {
        cycle(&mut el7037);
    }
    let _ = eth.channel.request_state_change(EtherCATState::PreOp);
    println!("done");
}

/// Writes a speed, in encoder counts per second, to the terminal. The terminal
/// wants a fraction of its configured speed range, so this converts counts/s to
/// full steps/s and then to that fraction.
fn command_speed(el7037: &mut EL7037, counts_per_s: f64) {
    let steps =
        (counts_per_s / COUNTS_PER_FULL_STEP).clamp(-MAX_FULL_STEPS_PER_S, MAX_FULL_STEPS_PER_S);
    let converter = EL70x1VelocityConverter::new(&el7037.get_speed_range(0));
    if let Ok(mut output) = el7037.get_output(0) {
        output.velocity = converter.steps_to_velocity(steps, true);
        let _ = el7037.set_output(0, output);
    }
}
