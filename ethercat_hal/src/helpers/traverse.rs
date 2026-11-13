//! Linear traverse on top of any stepper drive: homing on one or two endstops,
//! positioning in millimetres, pause/resume/stop and progress reporting.
//!
//! The helper is pure logic. The caller reads the drive once per cycle, hands
//! the actual position, the drive state and the *raw* endstop levels to
//! [`Traverse::update`], and writes back what it returns:
//!
//! - position-mode drives (CSP) use [`TraverseOutput::target_position`],
//! - velocity-mode drives use [`TraverseOutput::target_velocity`] as the
//!   feed-forward and close their own loop around the position.
//!
//! All positions crossing that boundary are in *drive units* (EL7062:
//! process-data increments, EL70x1: steps); [`TraverseConfig::units_per_mm`] is
//! the only place millimetres meet them.
//!
//! # Coordinates
//!
//! Traverse position 0 is the home end, `length_mm` the far end. Homing puts 0
//! `clearance_mm` off the home switch's release edge, so a move to 0 or to
//! `length_mm` never touches a switch.
//!
//! # Setpoint continuity
//!
//! In CSP the drive closes its loops around whatever setpoint it is sent, so a
//! step in the setpoint turns straight into following error. Every motion here
//! therefore runs through one trapezoidal profile, including pause and stop.
//! Only a fault freezes the setpoint at the actual position, which is a hard
//! stop on purpose.

use std::fmt;
use std::time::Duration;

/// Below this many drive units of remaining distance the profile counts as
/// arrived and snaps to its target.
const ARRIVED_EPSILON_UNITS: f64 = 1e-3;

/// What the raw input level of an endstop means.
///
/// Held here rather than left to the caller, so every driver interprets the
/// switch the same way and an inverted switch cannot silently turn a hard
/// limit into a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndstopPolarity {
    /// Normally open: the input is high while the switch is hit.
    ActiveHigh,
    /// Normally closed: the input is low while the switch is hit. Preferable
    /// for limit switches, because a broken wire then reads as hit and stops
    /// the axis instead of disabling the limit.
    ActiveLow,
}

impl EndstopPolarity {
    fn is_hit(self, raw: bool) -> bool {
        match self {
            Self::ActiveHigh => raw,
            Self::ActiveLow => !raw,
        }
    }
}

/// Which end of the traverse a single endstop sits at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeSide {
    /// The switch is at 0; its edge plus clearance becomes 0.
    Min,
    /// The switch is at the far end; its edge minus clearance becomes
    /// `length_mm`.
    Max,
}

/// The endstops fitted to the traverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endstops {
    /// One switch, wired to the home input. The other end is limited by
    /// `length_mm` only.
    Single(HomeSide),
    /// Home switch at 0, far switch at the other end, both hard limits.
    /// With `measure_length`, homing also finds the far switch and uses that
    /// distance, less the clearance, as `length_mm`.
    Both { measure_length: bool },
}

/// One of the two endstop inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endstop {
    Home,
    Far,
}

impl fmt::Display for Endstop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Home => write!(f, "home"),
            Self::Far => write!(f, "far"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomingConfig {
    pub endstops: Endstops,
    pub home_polarity: EndstopPolarity,
    /// Ignored with [`Endstops::Single`].
    pub far_polarity: EndstopPolarity,
    /// Approach speed towards a switch.
    pub search_speed_mm_s: f64,
    /// Back-off speed; the edge where the switch opens again is the
    /// reference, so this sets the homing repeatability.
    pub release_speed_mm_s: f64,
    /// Distance between a switch's release edge and the nearest end of the
    /// traverse.
    pub clearance_mm: f64,
    /// Give up if a switch has not been found (or has not opened again)
    /// within this distance.
    pub max_search_mm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraverseConfig {
    /// Drive units per millimetre of travel.
    pub units_per_mm: f64,
    /// Usable travel, 0..=length_mm.
    pub length_mm: f64,
    /// Set when increasing drive units move towards the home end.
    pub invert_direction: bool,
    pub max_speed_mm_s: f64,
    pub acceleration_mm_s2: f64,
    /// A move has arrived once the profile is done and the *actual* position
    /// is within this distance of the target.
    pub position_tolerance_mm: f64,
    /// How long the actual position may stay outside the tolerance after the
    /// profile is done before the move faults.
    pub in_position_timeout: Duration,
    pub homing: HomingConfig,
}

/// One cycle's view of the drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraverseInput {
    /// Actual position in drive units. Must be continuous: unwrap a wrapping
    /// counter before passing it in.
    pub actual_position: i64,
    /// The drive follows setpoints, e.g. CiA402 operation enabled and no
    /// fault.
    pub drive_ready: bool,
    /// Raw level of the home endstop input.
    pub endstop_home: bool,
    /// Raw level of the far endstop input. Ignored with [`Endstops::Single`].
    pub endstop_far: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraverseOutput {
    /// Setpoint for position-mode drives, in drive units.
    pub target_position: i64,
    /// Setpoint velocity in drive units per second, as feed-forward for
    /// velocity-mode drives.
    pub target_velocity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomingPhase {
    /// Approaching the home switch at search speed.
    SearchHome,
    /// Backing off the home switch at release speed until it opens.
    ReleaseHome,
    /// Approaching the far switch to measure the length.
    SearchFar,
    /// Backing off the far switch until it opens.
    ReleaseFar,
    /// Reference set, moving to the nearest end of the traverse.
    Park,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraverseFault {
    /// A switch was hit while the profile moved towards it outside homing.
    EndstopHit(Endstop),
    /// Homing did not reach the switch within `max_search_mm`.
    EndstopNotFound(Endstop),
    /// The switch did not open again within `max_search_mm` of backing off,
    /// e.g. a broken wire on a normally closed switch.
    EndstopStuck(Endstop),
    /// The far switch was found at a distance that leaves no usable travel.
    MeasuredLengthImplausible { measured_mm: f64 },
    /// The profile finished but the actual position stayed out of tolerance
    /// for longer than `in_position_timeout`.
    NotInPosition { error_mm: f64 },
}

impl fmt::Display for TraverseFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndstopHit(e) => write!(f, "{e} endstop hit while moving towards it"),
            Self::EndstopNotFound(e) => write!(f, "{e} endstop not found within search distance"),
            Self::EndstopStuck(e) => write!(f, "{e} endstop did not release (wiring/polarity?)"),
            Self::MeasuredLengthImplausible { measured_mm } => {
                write!(
                    f,
                    "measured length {measured_mm:.3} mm leaves no usable travel"
                )
            }
            Self::NotInPosition { error_mm } => {
                write!(f, "did not reach target, still {error_mm:.3} mm off")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraverseState {
    /// The drive is not ready; the setpoint tracks the actual position.
    Disabled,
    /// Drive ready but no reference: only homing is possible.
    Unhomed,
    Homing(HomingPhase),
    /// Homed and at rest.
    Idle,
    Moving,
    /// Braking to, or standing at, a standstill with the move's target kept
    /// for [`Traverse::resume`].
    Paused,
    /// Braking after [`Traverse::stop`]; the move's target is dropped.
    Stopping,
    /// Stopped hard. Needs [`Traverse::reset_fault`] and a new homing.
    Fault(TraverseFault),
}

impl fmt::Display for TraverseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "disabled"),
            Self::Unhomed => write!(f, "unhomed"),
            Self::Homing(phase) => write!(f, "homing ({phase:?})"),
            Self::Idle => write!(f, "idle"),
            Self::Moving => write!(f, "moving"),
            Self::Paused => write!(f, "paused"),
            Self::Stopping => write!(f, "stopping"),
            Self::Fault(fault) => write!(f, "FAULT: {fault}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraverseError {
    InvalidConfig(&'static str),
    InvalidValue(&'static str),
    NotHomed,
    OutOfRange {
        requested: f64,
        min: f64,
        max: f64,
    },
    Faulted,
    /// Homing or stopping is in progress.
    Busy,
    DriveNotReady,
    NotMoving,
    NotPaused,
}

impl fmt::Display for TraverseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(why) => write!(f, "invalid traverse config: {why}"),
            Self::InvalidValue(why) => write!(f, "invalid value: {why}"),
            Self::NotHomed => write!(f, "not homed, run homing first"),
            Self::OutOfRange {
                requested,
                min,
                max,
            } => write!(f, "{requested:.3} mm is outside {min:.3}..={max:.3} mm"),
            Self::Faulted => write!(f, "traverse is faulted, reset it first"),
            Self::Busy => write!(f, "busy homing or stopping"),
            Self::DriveNotReady => write!(f, "drive is not ready"),
            Self::NotMoving => write!(f, "no move in progress"),
            Self::NotPaused => write!(f, "not paused"),
        }
    }
}

impl std::error::Error for TraverseError {}

/// Trapezoidal setpoint profile in drive units.
#[derive(Debug, Clone, Copy, Default)]
struct Profile {
    position: f64,
    velocity: f64,
}

impl Profile {
    /// Accelerate, cruise and brake towards `target`, landing on it exactly.
    /// A profile that is already too fast (after a retarget) runs past the
    /// target and comes back rather than stopping instantly.
    fn advance(&mut self, target: f64, max_velocity: f64, acceleration: f64, dt: f64) {
        let remaining = target - self.position;
        let distance = remaining.abs();
        let dv_max = acceleration * dt;
        if distance < ARRIVED_EPSILON_UNITS
            || (self.velocity.abs() <= dv_max && distance <= dv_max * dt)
        {
            self.position = target;
            self.velocity = 0.0;
            return;
        }
        let v_brake = discrete_braking_velocity(distance, acceleration, dt);
        let v_cmd = remaining.signum() * max_velocity.min(v_brake).min(distance / dt);
        self.velocity += (v_cmd - self.velocity).clamp(-dv_max, dv_max);
        self.position += self.velocity * dt;
    }

    fn is_at(&self, target: f64) -> bool {
        self.velocity == 0.0 && (self.position - target).abs() < ARRIVED_EPSILON_UNITS
    }

    /// Where the profile comes to rest if it brakes now: the sum of the
    /// per-cycle steps `v - k·a·dt` it still takes.
    fn stopping_point(&self, acceleration: f64, dt: f64) -> f64 {
        let speed = self.velocity.abs();
        let dv = acceleration * dt;
        let n = (speed / dv).floor();
        self.position + self.velocity.signum() * dt * (n * speed - dv * n * (n + 1.0) / 2.0)
    }
}

/// Highest velocity to take this cycle that can still stop within `distance`.
///
/// The continuous braking curve `sqrt(2·a·d)` is wrong here: the profile
/// integrates once per cycle, so it would brake about half a cycle late and
/// overshoot. Taking `v = (n + f)·a·dt` now and braking by `a·dt` per cycle
/// afterwards covers `a·dt²·(n(n+1)/2 + f(n+1))`, which is solved for the
/// largest `n` and `f` that fit.
fn discrete_braking_velocity(distance: f64, acceleration: f64, dt: f64) -> f64 {
    let steps = distance / (acceleration * dt * dt);
    let n = ((-1.0 + (1.0 + 8.0 * steps).sqrt()) / 2.0).floor();
    let f = ((steps - n * (n + 1.0) / 2.0) / (n + 1.0)).clamp(0.0, 1.0);
    (n + f) * acceleration * dt
}

/// See the [module documentation](self).
#[derive(Debug, Clone)]
pub struct Traverse {
    config: TraverseConfig,
    state: TraverseState,
    profile: Profile,
    /// Where the profile is heading, in drive units.
    profile_target: f64,
    /// Speed limit of the current motion, in mm/s.
    profile_speed_mm_s: f64,
    /// Drive units at traverse position 0. `None` until homed.
    reference: Option<i64>,
    /// Target and start of the current (or paused) move, in drive units.
    move_target: f64,
    move_start: f64,
    last_move_distance_mm: Option<f64>,
    /// Time spent out of tolerance after the profile finished.
    settle_time: Duration,
    actual: i64,
    home_hit: bool,
    far_hit: bool,
    seeded: bool,
    /// Last cycle time in seconds, for braking from a command.
    cycle_s: f64,
}

impl Traverse {
    pub fn new(config: TraverseConfig) -> Result<Self, TraverseError> {
        validate(&config)?;
        Ok(Self {
            config,
            state: TraverseState::Disabled,
            profile: Profile::default(),
            profile_target: 0.0,
            profile_speed_mm_s: config.max_speed_mm_s,
            reference: None,
            move_target: 0.0,
            move_start: 0.0,
            last_move_distance_mm: None,
            settle_time: Duration::ZERO,
            actual: 0,
            home_hit: false,
            far_hit: false,
            seeded: false,
            cycle_s: 1e-3,
        })
    }

    /// Advance one cycle. `dt` is the time since the previous call.
    pub fn update(&mut self, input: &TraverseInput, dt: Duration) -> TraverseOutput {
        self.actual = input.actual_position;
        let homing = self.config.homing;
        self.home_hit = homing.home_polarity.is_hit(input.endstop_home);
        self.far_hit = matches!(homing.endstops, Endstops::Both { .. })
            && homing.far_polarity.is_hit(input.endstop_far);

        if !self.seeded {
            self.hold_at_actual();
            self.seeded = true;
        }

        if !input.drive_ready {
            if self.state != TraverseState::Disabled {
                // An aborted homing leaves no usable reference.
                if matches!(self.state, TraverseState::Homing(_)) {
                    self.reference = None;
                }
                if !matches!(self.state, TraverseState::Fault(_)) {
                    self.state = TraverseState::Disabled;
                }
            }
            // The drive holds wherever it is; tracking it means re-enabling
            // never jumps the setpoint.
            self.hold_at_actual();
            return self.output();
        }
        if self.state == TraverseState::Disabled {
            self.hold_at_actual();
            self.state = self.rest_state();
        }

        let dt_s = dt.as_secs_f64();
        if dt_s > 0.0 {
            self.cycle_s = dt_s;
            self.step(dt, dt_s);
        }
        self.output()
    }

    fn step(&mut self, dt: Duration, dt_s: f64) {
        // A homing phase drives into its own switch on purpose; every other
        // switch stays a hard limit throughout.
        let homing_on = match self.state {
            TraverseState::Homing(HomingPhase::SearchHome | HomingPhase::ReleaseHome) => {
                Some(Endstop::Home)
            }
            TraverseState::Homing(HomingPhase::SearchFar | HomingPhase::ReleaseFar) => {
                Some(Endstop::Far)
            }
            _ => None,
        };
        if let Some(endstop) = self.endstop_in_the_way().filter(|&e| Some(e) != homing_on) {
            self.fault(TraverseFault::EndstopHit(endstop));
        } else if let TraverseState::Homing(phase) = self.state {
            self.step_homing(phase);
        }
        if matches!(self.state, TraverseState::Fault(_)) {
            return;
        }

        let max_velocity = self.profile_speed_mm_s * self.config.units_per_mm;
        let acceleration = self.config.acceleration_mm_s2 * self.config.units_per_mm;
        self.profile
            .advance(self.profile_target, max_velocity, acceleration, dt_s);

        if !self.profile.is_at(self.profile_target) {
            return;
        }
        match self.state {
            TraverseState::Moving => {
                if self.check_in_position(dt) {
                    self.last_move_distance_mm =
                        Some((self.move_target - self.move_start).abs() / self.config.units_per_mm);
                    self.state = TraverseState::Idle;
                }
            }
            TraverseState::Homing(HomingPhase::Park) => {
                if self.check_in_position(dt) {
                    self.state = TraverseState::Idle;
                }
            }
            TraverseState::Homing(HomingPhase::SearchHome) => {
                self.fault(TraverseFault::EndstopNotFound(Endstop::Home))
            }
            TraverseState::Homing(HomingPhase::ReleaseHome) => {
                self.fault(TraverseFault::EndstopStuck(Endstop::Home))
            }
            TraverseState::Homing(HomingPhase::SearchFar) => {
                self.fault(TraverseFault::EndstopNotFound(Endstop::Far))
            }
            TraverseState::Homing(HomingPhase::ReleaseFar) => {
                self.fault(TraverseFault::EndstopStuck(Endstop::Far))
            }
            TraverseState::Stopping => self.state = self.rest_state(),
            _ => {}
        }
    }

    fn step_homing(&mut self, phase: HomingPhase) {
        let homing = self.config.homing;
        let upm = self.config.units_per_mm;
        let to_home = self.home_direction();
        let search = homing.max_search_mm * upm;
        match phase {
            HomingPhase::SearchHome => {
                if self.home_hit {
                    // Reversing straight away lets the profile brake through
                    // the switch and back, without a velocity step.
                    self.start_homing_phase(
                        HomingPhase::ReleaseHome,
                        -to_home * search,
                        homing.release_speed_mm_s,
                    );
                }
            }
            HomingPhase::ReleaseHome => {
                if !self.home_hit && self.profile.velocity * -to_home > 0.0 {
                    let edge = self.actual;
                    let s = self.sign();
                    let edge_mm = match homing.endstops {
                        Endstops::Single(HomeSide::Max) => {
                            self.config.length_mm + homing.clearance_mm
                        }
                        _ => -homing.clearance_mm,
                    };
                    self.reference = Some(edge - (s * edge_mm * upm).round() as i64);
                    if homing.endstops
                        == (Endstops::Both {
                            measure_length: true,
                        })
                    {
                        self.start_homing_phase(
                            HomingPhase::SearchFar,
                            -to_home * search,
                            homing.search_speed_mm_s,
                        );
                    } else {
                        self.park();
                    }
                }
            }
            HomingPhase::SearchFar => {
                if self.far_hit {
                    self.start_homing_phase(
                        HomingPhase::ReleaseFar,
                        to_home * search,
                        homing.release_speed_mm_s,
                    );
                }
            }
            HomingPhase::ReleaseFar => {
                if !self.far_hit && self.profile.velocity * to_home > 0.0 {
                    let measured_mm = self.mm_of(self.actual as f64) - homing.clearance_mm;
                    if measured_mm <= 0.0 {
                        self.fault(TraverseFault::MeasuredLengthImplausible { measured_mm });
                        return;
                    }
                    self.config.length_mm = measured_mm;
                    self.park();
                }
            }
            HomingPhase::Park => {}
        }
    }

    /// Start a homing phase that moves `distance` units from the current
    /// profile position.
    fn start_homing_phase(&mut self, phase: HomingPhase, distance: f64, speed_mm_s: f64) {
        self.state = TraverseState::Homing(phase);
        self.profile_target = self.profile.position + distance;
        self.profile_speed_mm_s = speed_mm_s;
    }

    /// Move to the end of the traverse nearest the home switch.
    fn park(&mut self) {
        let park_mm = match self.config.homing.endstops {
            Endstops::Single(HomeSide::Max) => self.config.length_mm,
            _ => 0.0,
        };
        self.state = TraverseState::Homing(HomingPhase::Park);
        self.profile_target = self.units_of(park_mm);
        self.profile_speed_mm_s = self.config.max_speed_mm_s;
        self.settle_time = Duration::ZERO;
    }

    /// A switch that is hit while the profile moves towards it.
    fn endstop_in_the_way(&self) -> Option<Endstop> {
        let v = self.profile.velocity;
        let to_home = self.home_direction();
        if self.home_hit && v * to_home > 0.0 {
            Some(Endstop::Home)
        } else if self.far_hit && v * -to_home > 0.0 {
            Some(Endstop::Far)
        } else {
            None
        }
    }

    /// True once the actual position is inside the tolerance; faults after
    /// the timeout otherwise.
    fn check_in_position(&mut self, dt: Duration) -> bool {
        let error_units = (self.actual as f64 - self.profile_target).abs();
        if error_units <= self.config.position_tolerance_mm * self.config.units_per_mm {
            self.settle_time = Duration::ZERO;
            return true;
        }
        self.settle_time += dt;
        if self.settle_time > self.config.in_position_timeout {
            self.fault(TraverseFault::NotInPosition {
                error_mm: error_units / self.config.units_per_mm,
            });
        }
        false
    }

    fn fault(&mut self, fault: TraverseFault) {
        self.state = TraverseState::Fault(fault);
        self.reference = None;
        self.hold_at_actual();
    }

    fn hold_at_actual(&mut self) {
        self.profile = Profile {
            position: self.actual as f64,
            velocity: 0.0,
        };
        self.profile_target = self.profile.position;
        self.settle_time = Duration::ZERO;
    }

    /// Brake along the profile and stay there.
    fn brake(&mut self) {
        let acceleration = self.config.acceleration_mm_s2 * self.config.units_per_mm;
        self.profile_target = self.profile.stopping_point(acceleration, self.cycle_s);
    }

    fn rest_state(&self) -> TraverseState {
        if self.reference.is_some() {
            TraverseState::Idle
        } else {
            TraverseState::Unhomed
        }
    }

    fn output(&self) -> TraverseOutput {
        TraverseOutput {
            target_position: self.profile.position.round() as i64,
            target_velocity: self.profile.velocity,
        }
    }

    /// +1 if increasing drive units move away from home end, -1 otherwise.
    fn sign(&self) -> f64 {
        if self.config.invert_direction {
            -1.0
        } else {
            1.0
        }
    }

    /// Direction of the home switch in drive units.
    fn home_direction(&self) -> f64 {
        match self.config.homing.endstops {
            Endstops::Single(HomeSide::Max) => self.sign(),
            _ => -self.sign(),
        }
    }

    fn units_of(&self, mm: f64) -> f64 {
        self.reference.unwrap_or(0) as f64 + self.sign() * mm * self.config.units_per_mm
    }

    fn mm_of(&self, units: f64) -> f64 {
        self.sign() * (units - self.reference.unwrap_or(0) as f64) / self.config.units_per_mm
    }

    fn check_commandable(&self) -> Result<(), TraverseError> {
        match self.state {
            TraverseState::Disabled => Err(TraverseError::DriveNotReady),
            TraverseState::Fault(_) => Err(TraverseError::Faulted),
            TraverseState::Homing(_) | TraverseState::Stopping => Err(TraverseError::Busy),
            _ => Ok(()),
        }
    }

    // --- Commands ---------------------------------------------------------

    /// Start homing. Clears any existing reference.
    pub fn home(&mut self) -> Result<(), TraverseError> {
        self.check_commandable()?;
        if matches!(self.state, TraverseState::Moving | TraverseState::Paused) {
            return Err(TraverseError::Busy);
        }
        self.reference = None;
        if self.home_hit {
            // Already on the switch: back off it straight away.
            let distance = -self.home_direction() * self.config.homing.max_search_mm;
            self.start_homing_phase(
                HomingPhase::ReleaseHome,
                distance * self.config.units_per_mm,
                self.config.homing.release_speed_mm_s,
            );
        } else {
            let distance = self.home_direction() * self.config.homing.max_search_mm;
            self.start_homing_phase(
                HomingPhase::SearchHome,
                distance * self.config.units_per_mm,
                self.config.homing.search_speed_mm_s,
            );
        }
        Ok(())
    }

    /// Move to `mm`. While moving or paused this retargets the move.
    pub fn go_to(&mut self, mm: f64) -> Result<(), TraverseError> {
        self.check_commandable()?;
        if self.reference.is_none() {
            return Err(TraverseError::NotHomed);
        }
        if !mm.is_finite() || !(0.0..=self.config.length_mm).contains(&mm) {
            return Err(TraverseError::OutOfRange {
                requested: mm,
                min: 0.0,
                max: self.config.length_mm,
            });
        }
        self.move_start = self.actual as f64;
        self.move_target = self.units_of(mm);
        self.profile_target = self.move_target;
        self.profile_speed_mm_s = self.config.max_speed_mm_s;
        self.settle_time = Duration::ZERO;
        self.state = TraverseState::Moving;
        Ok(())
    }

    /// Brake to a standstill, keeping the target for [`Self::resume`].
    pub fn pause(&mut self) -> Result<(), TraverseError> {
        if self.state != TraverseState::Moving {
            return Err(TraverseError::NotMoving);
        }
        self.brake();
        self.state = TraverseState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), TraverseError> {
        if self.state != TraverseState::Paused {
            return Err(TraverseError::NotPaused);
        }
        self.profile_target = self.move_target;
        self.profile_speed_mm_s = self.config.max_speed_mm_s;
        self.settle_time = Duration::ZERO;
        self.state = TraverseState::Moving;
        Ok(())
    }

    /// Brake to a standstill and drop the target. Stopping a homing run
    /// before the reference is set leaves the traverse unhomed.
    pub fn stop(&mut self) -> Result<(), TraverseError> {
        match self.state {
            TraverseState::Moving | TraverseState::Paused | TraverseState::Homing(_) => {
                self.brake();
                self.state = TraverseState::Stopping;
                Ok(())
            }
            _ => Err(TraverseError::NotMoving),
        }
    }

    /// Leave a fault. The reference is gone, so homing is needed again.
    pub fn reset_fault(&mut self) {
        if matches!(self.state, TraverseState::Fault(_)) {
            self.hold_at_actual();
            self.state = TraverseState::Unhomed;
        }
    }

    /// Top speed of moves and parking, in mm/s. Applies to a move in
    /// progress.
    pub fn set_speed(&mut self, mm_s: f64) -> Result<(), TraverseError> {
        if !(mm_s.is_finite() && mm_s > 0.0) {
            return Err(TraverseError::InvalidValue("speed must be positive"));
        }
        self.config.max_speed_mm_s = mm_s;
        if matches!(
            self.state,
            TraverseState::Moving | TraverseState::Homing(HomingPhase::Park)
        ) {
            self.profile_speed_mm_s = mm_s;
        }
        Ok(())
    }

    /// Change the usable travel. Rejected if it would leave the current
    /// position or target outside the traverse.
    pub fn set_length(&mut self, mm: f64) -> Result<(), TraverseError> {
        if !(mm.is_finite() && mm > 0.0) {
            return Err(TraverseError::InvalidValue("length must be positive"));
        }
        let mut furthest = self.position_mm().unwrap_or(0.0);
        if matches!(self.state, TraverseState::Moving | TraverseState::Paused) {
            furthest = furthest.max(self.mm_of(self.move_target));
        }
        if mm < furthest {
            return Err(TraverseError::OutOfRange {
                requested: mm,
                min: furthest,
                max: f64::INFINITY,
            });
        }
        self.config.length_mm = mm;
        Ok(())
    }

    /// Change the ratio. The reference is held in drive units, so it stays
    /// valid; millimetre positions shift to the new scale.
    pub fn set_units_per_mm(&mut self, units_per_mm: f64) -> Result<(), TraverseError> {
        if !(units_per_mm.is_finite() && units_per_mm > 0.0) {
            return Err(TraverseError::InvalidValue("units per mm must be positive"));
        }
        if self.check_commandable().is_err()
            || matches!(self.state, TraverseState::Moving | TraverseState::Paused)
        {
            return Err(TraverseError::Busy);
        }
        self.config.units_per_mm = units_per_mm;
        Ok(())
    }

    /// Correct the ratio from a measurement: a move commanded as
    /// `commanded_mm` that really travelled `measured_mm`. Returns the new
    /// units per mm.
    pub fn calibrate(&mut self, commanded_mm: f64, measured_mm: f64) -> Result<f64, TraverseError> {
        if !(commanded_mm.is_finite()
            && commanded_mm > 0.0
            && measured_mm.is_finite()
            && measured_mm > 0.0)
        {
            return Err(TraverseError::InvalidValue("distances must be positive"));
        }
        let corrected = self.config.units_per_mm * commanded_mm / measured_mm;
        self.set_units_per_mm(corrected)?;
        Ok(corrected)
    }

    // --- Queries ----------------------------------------------------------

    pub fn state(&self) -> TraverseState {
        self.state
    }

    pub fn config(&self) -> &TraverseConfig {
        &self.config
    }

    pub fn is_homed(&self) -> bool {
        self.reference.is_some()
    }

    /// Actual position in mm, once homed.
    pub fn position_mm(&self) -> Option<f64> {
        self.reference.map(|_| self.mm_of(self.actual as f64))
    }

    /// Setpoint position in mm, once homed.
    pub fn setpoint_mm(&self) -> Option<f64> {
        self.reference.map(|_| self.mm_of(self.profile.position))
    }

    /// Target of the current or paused move.
    pub fn target_mm(&self) -> Option<f64> {
        match self.state {
            TraverseState::Moving | TraverseState::Paused => Some(self.mm_of(self.move_target)),
            _ => None,
        }
    }

    /// Setpoint velocity in mm/s, signed along the traverse.
    pub fn velocity_mm_s(&self) -> f64 {
        self.sign() * self.profile.velocity / self.config.units_per_mm
    }

    pub fn length_mm(&self) -> f64 {
        self.config.length_mm
    }

    pub fn units_per_mm(&self) -> f64 {
        self.config.units_per_mm
    }

    /// Commanded distance of the last move that arrived, for
    /// [`Self::calibrate`].
    pub fn last_move_distance_mm(&self) -> Option<f64> {
        self.last_move_distance_mm
    }

    /// How far the current or paused move has come, 0..=100, measured on the
    /// actual position.
    pub fn progress_percent(&self) -> Option<f64> {
        if !matches!(self.state, TraverseState::Moving | TraverseState::Paused) {
            return None;
        }
        let total = self.move_target - self.move_start;
        if total.abs() < ARRIVED_EPSILON_UNITS {
            return Some(100.0);
        }
        Some(((self.actual as f64 - self.move_start) / total * 100.0).clamp(0.0, 100.0))
    }

    /// Interpreted (polarity applied) endstop state: (home, far).
    pub fn endstops_hit(&self) -> (bool, bool) {
        (self.home_hit, self.far_hit)
    }
}

fn validate(config: &TraverseConfig) -> Result<(), TraverseError> {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    let h = &config.homing;
    let checks = [
        (
            positive(config.units_per_mm),
            "units_per_mm must be positive",
        ),
        (positive(config.length_mm), "length_mm must be positive"),
        (
            positive(config.max_speed_mm_s),
            "max_speed_mm_s must be positive",
        ),
        (
            positive(config.acceleration_mm_s2),
            "acceleration_mm_s2 must be positive",
        ),
        (
            positive(config.position_tolerance_mm),
            "position_tolerance_mm must be positive",
        ),
        (
            positive(h.search_speed_mm_s),
            "search_speed_mm_s must be positive",
        ),
        (
            positive(h.release_speed_mm_s),
            "release_speed_mm_s must be positive",
        ),
        (
            h.clearance_mm.is_finite() && h.clearance_mm >= 0.0,
            "clearance_mm must not be negative",
        ),
        (positive(h.max_search_mm), "max_search_mm must be positive"),
    ];
    match checks.iter().find(|(ok, _)| !ok) {
        Some((_, why)) => Err(TraverseError::InvalidConfig(why)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: Duration = Duration::from_millis(1);
    const UPM: f64 = 100.0;

    fn config(endstops: Endstops) -> TraverseConfig {
        TraverseConfig {
            units_per_mm: UPM,
            length_mm: 150.0,
            invert_direction: false,
            max_speed_mm_s: 50.0,
            acceleration_mm_s2: 500.0,
            position_tolerance_mm: 0.05,
            in_position_timeout: Duration::from_millis(500),
            homing: HomingConfig {
                endstops,
                home_polarity: EndstopPolarity::ActiveHigh,
                far_polarity: EndstopPolarity::ActiveHigh,
                search_speed_mm_s: 20.0,
                release_speed_mm_s: 2.0,
                clearance_mm: 1.0,
                max_search_mm: 400.0,
            },
        }
    }

    /// An ideal drive: the actual position follows the setpoint one cycle
    /// late. Switches are hit at and beyond their unit positions.
    struct Rig {
        traverse: Traverse,
        actual: i64,
        ready: bool,
        /// Hit while `actual <= home_at` (or `>=` with `home_above`).
        home_at: i64,
        home_above: bool,
        far_at: Option<i64>,
        /// Inverts the raw level, for normally closed switches.
        home_nc: bool,
        far_nc: bool,
        /// Raw far level forced, for a broken wire.
        far_forced: Option<bool>,
        /// Constant offset between setpoint and actual, for a stuck axis.
        lag: i64,
    }

    impl Rig {
        fn new(config: TraverseConfig, actual: i64) -> Self {
            Self {
                traverse: Traverse::new(config).unwrap(),
                actual,
                ready: true,
                home_at: 0,
                home_above: false,
                far_at: None,
                home_nc: false,
                far_nc: false,
                far_forced: None,
                lag: 0,
            }
        }

        fn cycle(&mut self) -> TraverseOutput {
            let home_hit = if self.home_above {
                self.actual >= self.home_at
            } else {
                self.actual <= self.home_at
            };
            let far_hit = self.far_at.is_some_and(|at| self.actual >= at);
            let input = TraverseInput {
                actual_position: self.actual,
                drive_ready: self.ready,
                endstop_home: home_hit != self.home_nc,
                endstop_far: self.far_forced.unwrap_or(far_hit != self.far_nc),
            };
            let out = self.traverse.update(&input, DT);
            if self.ready {
                self.actual = out.target_position - self.lag;
            }
            out
        }

        /// Cycle until `done` or the time budget runs out.
        fn run_until(&mut self, seconds: f64, done: impl Fn(&Traverse) -> bool) {
            for _ in 0..(seconds * 1000.0) as usize {
                self.cycle();
                if done(&self.traverse) {
                    return;
                }
            }
            panic!(
                "condition not reached in {seconds} s, state {}",
                self.traverse.state()
            );
        }

        fn home(&mut self) {
            self.cycle();
            self.traverse.home().unwrap();
            self.run_until(60.0, |t| {
                t.state() != TraverseState::Homing(HomingPhase::SearchHome)
                    && !matches!(t.state(), TraverseState::Homing(_))
            });
        }
    }

    #[test]
    fn single_min_endstop_homes_to_its_release_edge_plus_clearance() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        // The switch opens at 1 unit; 0 mm is one clearance beyond that.
        assert_eq!(rig.traverse.reference, Some(1 + 100));
        assert_eq!(rig.actual, 101);
        assert!(rig.traverse.position_mm().unwrap().abs() < 1e-9);
    }

    #[test]
    fn single_max_endstop_becomes_length_less_clearance() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Max)), 3000);
        rig.home_at = 20_000;
        rig.home_above = true;
        rig.home();
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        // Released at 19999, which is 151 mm; parks at the 150 mm end.
        assert_eq!(rig.traverse.reference, Some(19_999 - 15_100));
        assert_eq!(rig.actual, 19_999 - 100);
        assert!((rig.traverse.position_mm().unwrap() - 150.0).abs() < 1e-9);
    }

    #[test]
    fn both_endstops_measure_the_length() {
        let mut rig = Rig::new(
            config(Endstops::Both {
                measure_length: true,
            }),
            3000,
        );
        rig.far_at = Some(25_000);
        rig.home();
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        // Far switch releases at 24999: (24999 - 101) / 100 - 1 mm clearance.
        let expected = (24_999.0 - 101.0) / UPM - 1.0;
        assert!((rig.traverse.length_mm() - expected).abs() < 1e-9);
        assert!(rig.traverse.position_mm().unwrap().abs() < 1e-9);
    }

    #[test]
    fn homing_from_on_the_switch_backs_off_first() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), -500);
        rig.home();
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        assert_eq!(rig.traverse.reference, Some(101));
    }

    #[test]
    fn a_move_travels_exactly_the_commanded_units() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        let start = rig.actual;
        rig.traverse.go_to(100.0).unwrap();
        rig.run_until(10.0, |t| t.state() == TraverseState::Idle);
        assert_eq!(rig.actual - start, 100 * UPM as i64);
        assert_eq!(rig.traverse.last_move_distance_mm(), Some(100.0));
    }

    /// EL7062 scale: 2^20 increments/rev, 200 steps/rev, 40 steps/mm. At
    /// ~2·10^5 units/mm rounding would show up first.
    #[test]
    fn a_move_is_exact_at_el7062_scale() {
        let mut c = config(Endstops::Single(HomeSide::Min));
        c.units_per_mm = 1_048_576.0 / 200.0 * 40.0;
        let mut rig = Rig::new(c, 3_000_000);
        rig.home();
        let start = rig.actual;
        rig.traverse.go_to(100.0).unwrap();
        rig.run_until(10.0, |t| t.state() == TraverseState::Idle);
        assert_eq!(rig.actual - start, 20_971_520);
    }

    #[test]
    fn inverted_direction_moves_the_other_way() {
        let mut c = config(Endstops::Single(HomeSide::Min));
        c.invert_direction = true;
        let mut rig = Rig::new(c, -3000);
        rig.home_at = 0;
        rig.home_above = true;
        rig.home();
        assert_eq!(rig.traverse.reference, Some(-1 - 100));
        rig.traverse.go_to(10.0).unwrap();
        rig.run_until(10.0, |t| t.state() == TraverseState::Idle);
        assert_eq!(rig.actual, -101 - 1000);
    }

    #[test]
    fn pause_brakes_without_overshoot_and_resume_finishes() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        rig.traverse.go_to(150.0).unwrap();
        for _ in 0..500 {
            rig.cycle();
        }
        assert!(rig.traverse.velocity_mm_s() > 40.0, "should be cruising");
        rig.traverse.pause().unwrap();
        let mut last = rig.actual;
        let mut last_v = rig.traverse.velocity_mm_s();
        for _ in 0..1000 {
            rig.cycle();
            let v = rig.traverse.velocity_mm_s();
            assert!(rig.actual >= last, "paused traverse moved backwards");
            assert!(v <= last_v + 1e-9, "paused traverse sped up");
            last = rig.actual;
            last_v = v;
        }
        assert_eq!(rig.traverse.state(), TraverseState::Paused);
        assert_eq!(rig.traverse.velocity_mm_s(), 0.0);
        let held = rig.traverse.progress_percent().unwrap();
        assert!(held > 0.0 && held < 100.0);
        rig.cycle();
        assert_eq!(rig.traverse.progress_percent(), Some(held));

        rig.traverse.resume().unwrap();
        rig.run_until(10.0, |t| t.state() == TraverseState::Idle);
        assert!((rig.traverse.position_mm().unwrap() - 150.0).abs() < 1e-9);
    }

    #[test]
    fn stop_brakes_and_drops_the_target() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        rig.traverse.go_to(150.0).unwrap();
        for _ in 0..500 {
            rig.cycle();
        }
        rig.traverse.stop().unwrap();
        rig.run_until(2.0, |t| t.state() == TraverseState::Idle);
        let at = rig.traverse.position_mm().unwrap();
        assert!(at > 0.0 && at < 150.0);
        assert_eq!(rig.traverse.resume(), Err(TraverseError::NotPaused));
    }

    #[test]
    fn progress_rises_monotonically_to_the_end() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        rig.traverse.go_to(80.0).unwrap();
        let mut last = 0.0;
        while rig.traverse.state() == TraverseState::Moving {
            rig.cycle();
            if let Some(p) = rig.traverse.progress_percent() {
                assert!(p >= last, "progress fell from {last} to {p}");
                last = p;
            }
        }
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        assert!(last > 99.0, "last progress {last}");
    }

    #[test]
    fn a_far_endstop_hit_mid_move_faults_and_needs_rehoming() {
        let mut rig = Rig::new(
            config(Endstops::Both {
                measure_length: false,
            }),
            3000,
        );
        rig.far_at = Some(50_000);
        rig.home();
        rig.far_at = Some(rig.actual + 5_000);
        rig.traverse.go_to(100.0).unwrap();
        rig.run_until(10.0, |t| matches!(t.state(), TraverseState::Fault(_)));
        assert_eq!(
            rig.traverse.state(),
            TraverseState::Fault(TraverseFault::EndstopHit(Endstop::Far))
        );
        assert_eq!(rig.traverse.go_to(10.0), Err(TraverseError::Faulted));
        rig.traverse.reset_fault();
        assert_eq!(rig.traverse.state(), TraverseState::Unhomed);
        assert_eq!(rig.traverse.go_to(10.0), Err(TraverseError::NotHomed));
    }

    #[test]
    fn targets_outside_the_traverse_are_rejected() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        assert!(matches!(
            rig.traverse.go_to(-0.1),
            Err(TraverseError::OutOfRange { .. })
        ));
        assert!(matches!(
            rig.traverse.go_to(150.1),
            Err(TraverseError::OutOfRange { .. })
        ));
        assert!(rig.traverse.go_to(150.0).is_ok());
    }

    #[test]
    fn normally_closed_switches_home_the_same() {
        let mut c = config(Endstops::Single(HomeSide::Min));
        c.homing.home_polarity = EndstopPolarity::ActiveLow;
        let mut rig = Rig::new(c, 3000);
        rig.home_nc = true;
        rig.home();
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        assert_eq!(rig.traverse.reference, Some(101));
    }

    #[test]
    fn a_broken_normally_closed_far_switch_faults_moving_towards_it() {
        let mut c = config(Endstops::Both {
            measure_length: false,
        });
        c.homing.far_polarity = EndstopPolarity::ActiveLow;
        let mut rig = Rig::new(c, 3000);
        rig.far_forced = Some(false); // wire cut: reads low, which is "hit"
        rig.cycle();
        rig.traverse.home().unwrap();
        rig.run_until(60.0, |t| matches!(t.state(), TraverseState::Fault(_)));
        assert_eq!(
            rig.traverse.state(),
            TraverseState::Fault(TraverseFault::EndstopHit(Endstop::Far))
        );
        // Caught while parking, within one clearance of the home switch.
        assert!(rig.actual < 200, "travelled to {}", rig.actual);
    }

    /// Backing off the home switch moves towards the far one, which must
    /// still stop the axis if the home switch never opens.
    #[test]
    fn the_far_switch_is_a_hard_limit_while_backing_off_home() {
        let mut rig = Rig::new(
            config(Endstops::Both {
                measure_length: false,
            }),
            3000,
        );
        rig.home_at = i64::MAX; // stuck closed
        rig.far_at = Some(8_000);
        rig.cycle();
        rig.traverse.home().unwrap();
        rig.run_until(60.0, |t| matches!(t.state(), TraverseState::Fault(_)));
        assert_eq!(
            rig.traverse.state(),
            TraverseState::Fault(TraverseFault::EndstopHit(Endstop::Far))
        );
        assert!(
            rig.actual < 8_100,
            "ran {} units past the far switch",
            rig.actual - 8_000
        );
    }

    #[test]
    fn a_missing_switch_faults_after_the_search_distance() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home_at = i64::MIN;
        rig.cycle();
        rig.traverse.home().unwrap();
        rig.run_until(60.0, |t| matches!(t.state(), TraverseState::Fault(_)));
        assert_eq!(
            rig.traverse.state(),
            TraverseState::Fault(TraverseFault::EndstopNotFound(Endstop::Home))
        );
    }

    #[test]
    fn a_stuck_axis_faults_not_in_position() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        rig.lag = 50; // 0.5 mm short, tolerance is 0.05 mm
        rig.traverse.go_to(20.0).unwrap();
        rig.run_until(10.0, |t| matches!(t.state(), TraverseState::Fault(_)));
        match rig.traverse.state() {
            TraverseState::Fault(TraverseFault::NotInPosition { error_mm }) => {
                assert!((error_mm - 0.5).abs() < 1e-9)
            }
            other => panic!("unexpected state {other}"),
        }
    }

    #[test]
    fn calibration_scales_the_ratio() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        let corrected = rig.traverse.calibrate(100.0, 98.0).unwrap();
        assert!((corrected - UPM * 100.0 / 98.0).abs() < 1e-9);
        assert_eq!(rig.traverse.units_per_mm(), corrected);
        // The reference is in units, so 0 mm has not moved.
        assert!(rig.traverse.position_mm().unwrap().abs() < 1e-9);
    }

    #[test]
    fn a_drive_dropout_never_jumps_the_setpoint() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 3000);
        rig.home();
        rig.traverse.go_to(150.0).unwrap();
        for _ in 0..300 {
            rig.cycle();
        }
        rig.ready = false;
        // The axis coasts somewhere else while the drive is off.
        rig.actual += 1234;
        let out = rig.cycle();
        assert_eq!(rig.traverse.state(), TraverseState::Disabled);
        assert_eq!(out.target_position, rig.actual);
        rig.ready = true;
        let held = rig.actual;
        let out = rig.cycle();
        assert_eq!(rig.traverse.state(), TraverseState::Idle);
        assert_eq!(out.target_position, held);
        assert!(rig.traverse.is_homed());
    }

    #[test]
    fn the_first_cycle_seeds_from_the_actual_position() {
        let mut rig = Rig::new(config(Endstops::Single(HomeSide::Min)), 987_654);
        let out = rig.cycle();
        assert_eq!(out.target_position, 987_654);
        assert_eq!(rig.traverse.state(), TraverseState::Unhomed);
    }
}
