/// Simple trapezoidal setpoint ramp in encoder increments.
///
/// Generates a smooth 3-segment profile (accelerate / cruise / decelerate) so a
/// drive is never asked to jump straight to the target position.
#[derive(Debug, Clone, Copy)]
pub struct SetpointRamp {
    position: f64,
    velocity: f64,
}

/// 5 rev/s = 300 rev/min at 500 CPR.
pub const RAMP_MAX_VELOCITY_INCR_PER_S: f64 = 2500.0;
/// 10 rev/s² at 500 CPR (~63 rad/s²).
pub const RAMP_ACCEL_INCR_PER_S2: f64 = 5000.0;

impl SetpointRamp {
    pub fn new(initial_position: i32) -> Self {
        Self {
            position: initial_position as f64,
            velocity: 0.0,
        }
    }

    /// Current ramp position in encoder increments.
    pub fn position(&self) -> f64 {
        self.position
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
        let v_brake = (2.0 * RAMP_ACCEL_INCR_PER_S2 * dist).sqrt();
        let v_cmd = dir * RAMP_MAX_VELOCITY_INCR_PER_S.min(v_brake);
        let dv = (v_cmd - self.velocity)
            .clamp(-RAMP_ACCEL_INCR_PER_S2 * dt, RAMP_ACCEL_INCR_PER_S2 * dt);
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