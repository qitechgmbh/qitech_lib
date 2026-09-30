//! Trapezoidal setpoint generation in the terminal's position increments.
//!
//! # Position scale
//!
//! The EL7062 scales every process-data position (`0x6000:11` actual,
//! `0x7010:05` target, following error, ...) to `2^singleturn_bits` increments
//! per motor revolution. `singleturn_bits` is 0x8000:12 / 0x8100:12 and defaults
//! to 20, i.e. **1,048,576 increments per revolution**.
//!
//! That scale is a property of the terminal and is *independent of the encoder*.
//! 0x8008:13 "Encoder Increments per Revolution" only tells the terminal how many
//! raw encoder counts make up one revolution so it can rescale them into the
//! scale above; it does not change the scale. One full motor step of a
//! 200-steps/rev motor is therefore 2^20/200 = 5242.88 increments.
//!
//! Because the scale is easy to get wrong by three orders of magnitude, limits
//! are given in **revolutions** and converted with the terminal's own
//! [`PositionScale`].

/// The terminal's process-data position scale: 0x8000:12 / 0x8100:12
/// ("Singleturn bits") together with 0x8000:13 / 0x8100:13 ("Multiturn bits").
///
/// These are one setting rather than two. The manual requires their sum to be
/// 32, and the ESI file lists drive error `0x8423` for a violation, so holding
/// them as a pair is what makes that rule unrepresentable instead of merely
/// documented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionScale {
    singleturn_bits: u8,
    multiturn_bits: u8,
}

/// Default 0x8000:12 / 0x8100:12 and 0x8000:13 / 0x8100:13 "Singleturn bits"
/// and "Multiturn bits" of the EL7062.
pub const DEFAULT_POSITION_SCALE: PositionScale = PositionScale {
    singleturn_bits: 20,
    multiturn_bits: 12,
};

/// A singleturn/multiturn bit pair the terminal will not accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionScaleError {
    pub singleturn_bits: u8,
    pub multiturn_bits: u8,
}

impl std::fmt::Display for PositionScaleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "0x8000:12 = {} and 0x8000:13 = {} are not a valid position scale: 1..=31 singleturn \
             bits are required and the two must sum to 32",
            self.singleturn_bits, self.multiturn_bits
        )
    }
}

impl std::error::Error for PositionScaleError {}

impl Default for PositionScale {
    /// Power-on defaults: 20 singleturn bits + 12 multiturn bits.
    fn default() -> Self {
        DEFAULT_POSITION_SCALE
    }
}

impl PositionScale {
    /// `singleturn_bits` is capped at 31 because the process-data position is a
    /// 32-bit UDINT: a scale of 2^32 increments per revolution leaves no room
    /// for the revolution count, so the terminal cannot represent it. That bound
    /// is also what keeps [`Self::increments_per_revolution`] total, since
    /// `1u32 << 32` would otherwise overflow and, with overflow checks off,
    /// silently mask down to 1 increment per revolution.
    pub fn new(singleturn_bits: u8, multiturn_bits: u8) -> Result<Self, PositionScaleError> {
        if !(1..=31).contains(&singleturn_bits)
            || singleturn_bits as u16 + multiturn_bits as u16 != 32
        {
            return Err(PositionScaleError {
                singleturn_bits,
                multiturn_bits,
            });
        }
        Ok(Self {
            singleturn_bits,
            multiturn_bits,
        })
    }

    /// 0x8000:12 / 0x8100:12.
    pub const fn singleturn_bits(self) -> u8 {
        self.singleturn_bits
    }

    /// 0x8000:13 / 0x8100:13.
    pub const fn multiturn_bits(self) -> u8 {
        self.multiturn_bits
    }

    /// Increments per motor revolution.
    pub const fn increments_per_revolution(self) -> u32 {
        1u32 << self.singleturn_bits
    }

    /// Increments per full motor step at this scale.
    pub fn increments_per_step(self, full_steps_per_revolution: u32) -> f64 {
        self.increments_per_revolution() as f64 / full_steps_per_revolution as f64
    }
}

/// Default ramp top speed in revolutions per second.
pub const DEFAULT_MAX_REV_PER_S: f64 = 5.0;
/// Default ramp acceleration in revolutions per second squared.
pub const DEFAULT_MAX_REV_PER_S2: f64 = 10.0;

/// Trapezoidal profile (accelerate / cruise / decelerate) towards a target.
#[derive(Debug, Clone, Copy)]
pub struct SetpointRamp {
    position: f64,
    velocity: f64,
    max_velocity: f64,
    acceleration: f64,
}

impl SetpointRamp {
    /// Ramp starting at `initial_position` with the default speed limits.
    pub fn new(initial_position: i32, scale: PositionScale) -> Self {
        Self::with_limits(
            initial_position,
            scale,
            DEFAULT_MAX_REV_PER_S,
            DEFAULT_MAX_REV_PER_S2,
        )
    }

    /// Ramp with explicit limits in revolutions and revolutions per second.
    pub fn with_limits(
        initial_position: i32,
        scale: PositionScale,
        max_rev_per_s: f64,
        max_rev_per_s2: f64,
    ) -> Self {
        let incr_per_rev = scale.increments_per_revolution() as f64;
        Self {
            position: initial_position as f64,
            velocity: 0.0,
            max_velocity: max_rev_per_s * incr_per_rev,
            acceleration: max_rev_per_s2 * incr_per_rev,
        }
    }

    /// Current ramp position in process-data increments.
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Current ramp velocity in process-data increments per second.
    pub fn velocity(&self) -> f64 {
        self.velocity
    }

    /// Step the ramp towards `target` by one `dt`-sized time slice.
    pub fn advance(&mut self, target: f64, dt: f64) {
        let remaining = target - self.position;
        if remaining.abs() < 1e-3 {
            self.position = target;
            self.velocity = 0.0;
            return;
        }
        let dir = remaining.signum();
        let dist = remaining.abs();
        // Cap the velocity so we can still stop within the remaining distance.
        let v_brake = (2.0 * self.acceleration * dist).sqrt();
        let v_cmd = dir * self.max_velocity.min(v_brake);
        let dv = (v_cmd - self.velocity).clamp(-self.acceleration * dt, self.acceleration * dt);
        self.velocity += dv;

        let next = self.position + self.velocity * dt;
        // Do not overshoot the target.
        if (target - self.position).signum() != (target - next).signum() {
            self.position = target;
            self.velocity = 0.0;
        } else {
            self.position = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scale_is_one_million_increments_per_revolution() {
        assert_eq!(
            DEFAULT_POSITION_SCALE.increments_per_revolution(),
            1_048_576
        );
    }

    /// The terminal requires 0x8000:12 + 0x8000:13 == 32, so a lone singleturn
    /// setting is not a configuration, it is half of one.
    #[test]
    fn a_bit_pair_that_does_not_sum_to_32_is_rejected() {
        assert!(PositionScale::new(20, 11).is_err());
        assert!(PositionScale::new(20, 13).is_err());
        assert!(PositionScale::new(4, 4).is_err());
    }

    /// `1u32 << 32` masks down to 1 increment per revolution when overflow
    /// checks are off, which the `1..=31` bound in `new` prevents.
    #[test]
    fn a_scale_that_overflows_the_u32_position_is_not_constructible() {
        assert!(PositionScale::new(32, 0).is_err());
        assert!(PositionScale::new(0, 32).is_err());
        assert!(PositionScale::new(255, 0).is_err());
        assert_eq!(
            PositionScale::new(31, 1)
                .unwrap()
                .increments_per_revolution(),
            1u32 << 31
        );
    }

    #[test]
    fn the_terminal_default_pair_is_accepted() {
        assert!(PositionScale::new(20, 12).is_ok());
    }

    /// 2^20/200 = 5242.88 increments per full step, so a target of a few
    /// hundred is far below one step of a 200-step motor.
    #[test]
    fn a_few_hundred_increments_is_well_under_one_full_step() {
        let full_steps_per_rev = 200;
        let per_step = DEFAULT_POSITION_SCALE.increments_per_step(full_steps_per_rev);
        assert!((per_step - 5242.88).abs() < 0.01, "per_step = {per_step}");
        assert!(
            500.0 < per_step / 10.0,
            "500 increments must be under a tenth of a full step"
        );
    }

    /// About 1 s, not hours: a regression here means the limits were passed in
    /// increments instead of revolutions.
    #[test]
    fn one_revolution_takes_about_a_second() {
        let incr_per_rev = DEFAULT_POSITION_SCALE.increments_per_revolution() as f64;
        let dt = 1e-3; // 1 ms master cycle
        let mut ramp = SetpointRamp::new(0, DEFAULT_POSITION_SCALE);
        let target = incr_per_rev;
        let mut seconds = 0.0;
        loop {
            ramp.advance(target, dt);
            seconds += dt;
            assert!(seconds < 5.0, "1 rev did not finish within 5 s");
            if (target - ramp.position()).abs() < 1e-3 {
                break;
            }
        }
        assert!(
            (0.5..1.5).contains(&seconds),
            "1 rev took {seconds} s, expected roughly 1 s"
        );
        // Peak velocity stays inside the configured 5 rev/s.
        assert!(ramp.velocity().abs() <= DEFAULT_MAX_REV_PER_S * incr_per_rev);
    }

    #[test]
    fn ramp_never_overshoots() {
        let dt = 1e-3;
        let incr_per_rev = DEFAULT_POSITION_SCALE.increments_per_revolution() as f64;
        let mut ramp = SetpointRamp::new(0, DEFAULT_POSITION_SCALE);
        // Odd target, so the brake-distance clamp has to round a partial step.
        let target = 0.37 * incr_per_rev;
        for _ in 0..5000 {
            ramp.advance(target, dt);
            assert!(ramp.position() <= target + 1e-6, "overshot forward");
            assert!(ramp.position() >= 0.0, "overshot backwards");
        }
        assert!((ramp.position() - target).abs() < 1e-3);
    }
}
