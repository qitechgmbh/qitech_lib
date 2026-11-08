use super::RxPdoObject;
use bitvec::prelude::*;
use ethercat_hal_derive::PdoObject;

/// PDO Object for EL2502 PWM output duty cycle ("PWM output", ESI object 0x7000:17)
///
/// 16 bit unsigned value. The scaling depends on the terminal configuration
/// (CoE object 0x8000 "Presentation"). The Beckhoff default is 1/100 % per digit,
/// i.e. a raw value of 10_000 corresponds to a duty cycle of 100 %.
/// (Derived from the official Beckhoff ESI file, EL25xx, 2026-11-08)
#[derive(Debug, Clone, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct PwmDuty {
    /// Raw duty value (default presentation: 1/100 % per digit → 10_000 = 100 %)
    pub value: u16,
}

impl RxPdoObject for PwmDuty {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits[0..16].store_le(self.value);
    }
}

impl PwmDuty {
    /// Sets the duty cycle in percent [0.0, 100.0] assuming the default
    /// presentation of 1/100 % per digit.
    pub fn set_percent(&mut self, percent: f64) {
        let clamped = percent.clamp(0.0, 100.0);
        self.value = (clamped * 100.0).round() as u16;
    }

    /// Current duty cycle in percent assuming the default presentation.
    pub fn percent(&self) -> f64 {
        self.value as f64 / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pwm_duty_write_raw() {
        let duty = PwmDuty { value: 0x1234 };
        let mut buffer = [0u8; 2];
        {
            let bits = buffer.view_bits_mut::<Lsb0>();
            duty.write(&mut bits[0..16]);
        }
        assert_eq!(buffer, [0x34, 0x12]);
    }

    #[test]
    fn test_pwm_duty_percent_roundtrip() {
        let mut duty = PwmDuty::default();
        duty.set_percent(55.5);
        assert_eq!(duty.value, 5550);
        assert!((duty.percent() - 55.5).abs() < 0.01);
    }

    #[test]
    fn test_pwm_duty_percent_clamping() {
        let mut duty = PwmDuty::default();
        duty.set_percent(150.0);
        assert_eq!(duty.value, 10_000);
        duty.set_percent(-5.0);
        assert_eq!(duty.value, 0);
    }
}
