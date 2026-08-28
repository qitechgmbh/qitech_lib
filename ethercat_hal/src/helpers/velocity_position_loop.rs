//! A closed position loop for actuators that only take a speed command and
//! report a live position (e.g. a stepper terminal run in "velocity direct"
//! against its encoder, closed here instead of in the terminal).
//!
//! This is pure state - no device, no I/O, no printing. Each cycle the
//! caller reads its own position/stall feedback, calls [`VelocityPositionLoop::step`],
//! and writes the returned speed. Units are whatever the caller's position and
//! speed values are in (e.g. encoder counts and counts/s); the loop never
//! interprets them.
//!
//! For the reasoning behind the default tunables and the control law itself,
//! see `examples/el7037_velocity_closed_loop.md` in this repo, and
//! `examples/el7037_velocity_closed_loop.rs` for a worked usage example.

use std::time::{Duration, Instant};

/// Tunables. See [`Default`] for the values measured against a NEMA17 stepper
/// with a 4000 count/rev encoder (`examples/el7037_velocity_closed_loop.md`) -
/// treat those as a starting point, not a universal default.
#[derive(Debug, Clone, Copy)]
pub struct VelocityPositionLoopConfig {
    /// Cruise speed cap, position units/s.
    pub max_speed: f64,
    /// Real slew limit applied to the commanded speed, position units/s^2.
    pub acceleration: f64,
    /// Used only in the stop-distance envelope (`sqrt(2 * a * x)`), not the
    /// real slew. Lower than `acceleration` so the profile starts braking
    /// earlier/gentler than the physical limit requires, leaving margin for
    /// real-world lag.
    pub braking_acceleration: f64,
    /// Proportional gain of the close-in phase, in 1/s.
    pub approach_gain: f64,
    /// Speed floor while outside `tolerance`, so the approach isn't asymptotic.
    pub min_speed: f64,
    /// Arrival band. Once the position is inside this the loop commands zero.
    pub tolerance: i64,
    /// Must stay inside `tolerance` this long before counting as arrived, so a
    /// fly-through during a fast move isn't mistaken for it.
    pub dwell: Duration,
    /// Once holding, only re-engage past this. Should be well wider than
    /// `tolerance`, or settling noise will retrigger a correction and the
    /// axis will buzz.
    pub re_engage: i64,
    /// Give up on a move that hasn't landed in this long.
    pub move_timeout: Duration,
    /// Abort a move that travels more than `abs(delta) * runaway_factor +
    /// runaway_slack` past its start.
    pub runaway_factor: f64,
    pub runaway_slack: f64,
}

impl Default for VelocityPositionLoopConfig {
    fn default() -> Self {
        Self {
            max_speed: 16_000.0,
            acceleration: 40_000.0,
            braking_acceleration: 20_000.0,
            approach_gain: 25.0,
            min_speed: 100.0,
            tolerance: 5,
            dwell: Duration::from_millis(120),
            re_engage: 60,
            move_timeout: Duration::from_secs(15),
            runaway_factor: 3.0,
            runaway_slack: 400.0,
        }
    }
}

/// What the loop is currently doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopState {
    /// No target, or the last move was abandoned. Commands zero.
    Idle,
    /// Running the profile towards the target.
    Moving,
    /// Arrived and latched. Commands zero and does not correct until the
    /// position drifts past `re_engage`.
    Holding,
}

/// What happened on a [`VelocityPositionLoop::step`] call. At most one fires
/// per cycle.
#[derive(Debug, Clone, Copy)]
pub enum LoopEvent {
    /// Settled inside `tolerance` for `dwell` and coasted to a stop.
    Arrived {
        /// `target - position` at the moment of arrival (signed).
        residual: i64,
        elapsed: Duration,
        stall_pulses: u32,
        stall_ms: f64,
    },
    /// Travelled further than the runaway guard allows. The loop has gone Idle.
    Runaway { travelled: f64, limit: f64 },
    /// `move_timeout` elapsed without arriving. The loop has gone Idle.
    TimedOut {
        /// `target - position` at the moment of the timeout (signed).
        residual: i64,
    },
    /// Drifted past `re_engage` while holding; the loop has resumed moving
    /// towards the same target.
    Drifted {
        /// `target - position` at the moment the drift was detected (signed).
        residual: i64,
    },
}

/// Counts rising edges and total time of a live "stalled" bit that pulses
/// rather than latches, so a caller sampling it occasionally doesn't miss it.
#[derive(Debug, Default, Clone, Copy)]
struct StallTracker {
    was_set: bool,
    pulses: u32,
    ms: f64,
}

impl StallTracker {
    fn record(&mut self, stalled: bool, dt: f64) {
        if stalled && !self.was_set {
            self.pulses += 1;
        }
        if stalled {
            self.ms += dt * 1000.0;
        }
        self.was_set = stalled;
    }

    fn reset(&mut self) {
        self.pulses = 0;
        self.ms = 0.0;
    }
}

/// Speed profile driven by the live position error: the time-optimal
/// square-root deceleration envelope combined with a proportional close-in,
/// slewed at the acceleration limit.
#[derive(Debug, Clone, Copy)]
struct Profile {
    speed: f64,
}

impl Profile {
    fn new() -> Self {
        Self { speed: 0.0 }
    }

    /// `error` is `target - position`; `None` ramps down to a stop. Returns
    /// the speed to command.
    fn step(&mut self, error: Option<i64>, dt: f64, config: &VelocityPositionLoopConfig) -> f64 {
        let demand = match error {
            None => 0.0,
            Some(error) if error.abs() <= config.tolerance => 0.0,
            Some(error) => {
                let excess = error.abs() as f64 - config.tolerance as f64;
                let stoppable = (2.0 * config.braking_acceleration * excess).sqrt();
                let gentle = config.approach_gain * excess;
                let magnitude = stoppable
                    .min(gentle)
                    .min(config.max_speed)
                    .max(config.min_speed);
                magnitude * (error.signum() as f64)
            }
        };

        let slew = config.acceleration * dt;
        self.speed += (demand - self.speed).clamp(-slew, slew);
        // Snap to zero rather than dithering around it.
        if demand == 0.0 && self.speed.abs() < config.min_speed / 2.0 {
            self.speed = 0.0;
        }
        self.speed
    }

    fn stopped(&self) -> bool {
        self.speed == 0.0
    }
}

/// A closed position loop over a speed-only actuator. See the module docs.
pub struct VelocityPositionLoop {
    config: VelocityPositionLoopConfig,
    state: LoopState,
    profile: Profile,
    hold_enabled: bool,
    target: i64,
    move_started: Instant,
    move_from: i64,
    move_limit: f64,
    in_window_since: Option<Instant>,
    stall: StallTracker,
}

impl VelocityPositionLoop {
    pub fn new(config: VelocityPositionLoopConfig, position: i64, now: Instant) -> Self {
        Self {
            config,
            state: LoopState::Idle,
            profile: Profile::new(),
            hold_enabled: true,
            target: position,
            move_started: now,
            move_from: position,
            move_limit: f64::INFINITY,
            in_window_since: None,
            stall: StallTracker::default(),
        }
    }

    pub fn state(&self) -> LoopState {
        self.state
    }

    pub fn target(&self) -> i64 {
        self.target
    }

    pub fn max_speed(&self) -> f64 {
        self.config.max_speed
    }

    pub fn set_max_speed(&mut self, speed: f64) {
        self.config.max_speed = speed;
    }

    pub fn hold_enabled(&self) -> bool {
        self.hold_enabled
    }

    pub fn set_hold_enabled(&mut self, on: bool) {
        self.hold_enabled = on;
    }

    /// Starts (or retargets) a move.
    pub fn start_move(&mut self, target: i64, position: i64, now: Instant) {
        self.target = target;
        self.state = LoopState::Moving;
        self.move_started = now;
        self.move_from = position;
        self.move_limit =
            (target - position).abs() as f64 * self.config.runaway_factor + self.config.runaway_slack;
        self.in_window_since = None;
        self.stall.reset();
    }

    /// Commands zero and goes idle immediately.
    pub fn stop(&mut self) {
        self.state = LoopState::Idle;
    }

    /// Resets the target to `0`, for a caller that has just zeroed its own
    /// position feedback. Refuses (returns `false`) while a move is running.
    pub fn home(&mut self) -> bool {
        if self.state == LoopState::Moving {
            return false;
        }
        self.target = 0;
        self.state = LoopState::Idle;
        true
    }

    /// Advances the loop by one cycle. Returns the speed to command and, at
    /// most, one event describing what just happened.
    pub fn step(
        &mut self,
        position: i64,
        stalled: bool,
        now: Instant,
        dt: f64,
    ) -> (f64, Option<LoopEvent>) {
        self.stall.record(stalled, dt);

        let error = self.target - position;
        let mut event = None;
        match self.state {
            LoopState::Moving => {
                let travelled = (position - self.move_from).abs() as f64;
                if travelled > self.move_limit {
                    event = Some(LoopEvent::Runaway {
                        travelled,
                        limit: self.move_limit,
                    });
                    self.state = LoopState::Idle;
                } else if error.abs() <= self.config.tolerance {
                    let since = *self.in_window_since.get_or_insert(now);
                    if now.duration_since(since) >= self.config.dwell && self.profile.stopped() {
                        event = Some(LoopEvent::Arrived {
                            residual: -error,
                            elapsed: now.duration_since(self.move_started),
                            stall_pulses: self.stall.pulses,
                            stall_ms: self.stall.ms,
                        });
                        // Latch and disengage; nothing further is commanded
                        // unless the position drifts past `re_engage`.
                        self.state = LoopState::Holding;
                    }
                } else {
                    self.in_window_since = None;
                    if now.duration_since(self.move_started) > self.config.move_timeout {
                        event = Some(LoopEvent::TimedOut { residual: -error });
                        self.state = LoopState::Idle;
                    }
                }
            }
            LoopState::Holding => {
                if self.hold_enabled && error.abs() > self.config.re_engage {
                    event = Some(LoopEvent::Drifted { residual: -error });
                    self.start_move(self.target, position, now);
                }
            }
            LoopState::Idle => {}
        }

        let speed = self.profile.step(
            (self.state == LoopState::Moving).then_some(self.target - position),
            dt,
            &self.config,
        );
        (speed, event)
    }

    /// Ramps the profile down to a stop, ignoring the target.
    pub fn ramp_down(&mut self, dt: f64) -> f64 {
        self.profile.step(None, dt, &self.config)
    }

    pub fn stopped(&self) -> bool {
        self.profile.stopped()
    }
}
