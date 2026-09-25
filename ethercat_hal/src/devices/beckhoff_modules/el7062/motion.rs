/// Trapezoidal setpoint ramp in the terminal's position increments.
///
/// # Position scale
///
/// The EL7062 scales every process-data position (`0x6000:11` actual,
/// `0x7010:05` target, following error, ...) to `2^singleturn_bits` increments
/// per motor revolution. `singleturn_bits` is 0x8000:12 / 0x8100:12 and defaults
/// to 20, i.e. **1,048,576 increments per revolution**.
///
/// That scale is a property of the terminal and is *independent of the encoder*.
/// 0x8008:13 "Encoder Increments per Revolution" only tells the terminal how many
/// raw encoder counts make up one revolution so it can rescale them into the
/// scale above; it does not change the scale. One full motor step of a
/// 200-steps/rev motor is therefore 2^20/200 = 5242.88 increments.
///
/// Because the scale is easy to get wrong by three orders of magnitude, limits
/// are given in **revolutions** and converted with the terminal's own
/// `singleturn_bits`.

/// Default 0x8000:12 / 0x8100:12 "Singleturn bits" of the EL7062.
pub const DEFAULT_SINGLETURN_BITS: u8 = 0x14;

/// Increments per motor revolution for a given 0x8000:12 setting.
pub const fn increments_per_revolution(singleturn_bits: u8) -> u32 {
    1u32 << singleturn_bits
}

/// Default ramp top speed in revolutions per second.
pub const DEFAULT_MAX_REV_PER_S: f64 = 5.0;
/// Default ramp acceleration in revolutions per second squared.
pub const DEFAULT_MAX_REV_PER_S2: f64 = 10.0;

/// Generates a smooth 3-segment profile (accelerate / cruise / decelerate) so a
/// drive is never asked to jump straight to the target position.
#[derive(Debug, Clone, Copy)]
pub struct SetpointRamp {
    position: f64,
    velocity: f64,
    max_velocity: f64,
    acceleration: f64,
}

impl SetpointRamp {
    /// Ramp starting at `initial_position` with the default speed limits.
    pub fn new(initial_position: i32, singleturn_bits: u8) -> Self {
        Self::with_limits(
            initial_position,
            singleturn_bits,
            DEFAULT_MAX_REV_PER_S,
            DEFAULT_MAX_REV_PER_S2,
        )
    }

    /// Ramp with explicit limits in revolutions and revolutions per second.
    pub fn with_limits(
        initial_position: i32,
        singleturn_bits: u8,
        max_rev_per_s: f64,
        max_rev_per_s2: f64,
    ) -> Self {
        let scale = increments_per_revolution(singleturn_bits) as f64;
        Self {
            position: initial_position as f64,
            velocity: 0.0,
            max_velocity: max_rev_per_s * scale,
            acceleration: max_rev_per_s2 * scale,
        }
    }

    /// Current ramp position in encoder increments.
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Current ramp velocity in encoder increments per second.
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
            increments_per_revolution(DEFAULT_SINGLETURN_BITS),
            1_048_576
        );
    }

    /// The EL7062 only emits a full step once the setpoint crosses
    /// 2^singleturn_bits/steps_per_rev increments. Anything below that is not a
    /// move at all, which is what made a "500 increment" target look like a
    /// broken motor.
    #[test]
    fn a_few_hundred_increments_is_well_under_one_full_step() {
        let full_steps_per_rev = 200;
        let per_step =
            increments_per_revolution(DEFAULT_SINGLETURN_BITS) as f64 / full_steps_per_rev as f64;
        assert!((per_step - 5242.88).abs() < 0.01, "per_step = {per_step}");
        assert!(
            500.0 < per_step / 10.0,
            "500 increments must be under a tenth of a full step"
        );
    }

    /// One revolution at the default profile should take well under a second.
    /// With limits expressed in the wrong scale this used to take hours.
    #[test]
    fn one_revolution_takes_about_a_second() {
        let incr_per_rev = increments_per_revolution(DEFAULT_SINGLETURN_BITS) as f64;
        let dt = 1e-3; // 1 ms master cycle
        let mut ramp = SetpointRamp::new(0, DEFAULT_SINGLETURN_BITS);
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
        let incr_per_rev = increments_per_revolution(DEFAULT_SINGLETURN_BITS) as f64;
        let mut ramp = SetpointRamp::new(0, DEFAULT_SINGLETURN_BITS);
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
