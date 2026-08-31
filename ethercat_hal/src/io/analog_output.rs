use units::{ElectricPotential, electric_potential::volt, ratio::ratio};

pub trait AnalogVoltageOutputDevice {
    /// Get the minimum voltage this device can output on a single port
    fn get_minimum_output(&self) -> ElectricPotential;
    /// Get the maximum voltage this device can output on a single port
    fn get_maximum_output(&self) -> ElectricPotential;

    /// Set a specific output relative to the minimum and maximum value.
    ///
    /// The given `value` must be in the interval `[-1, 1]` for devices that support negative
    /// output values and in the interval `[0, 1]` for devices with only positive output values.
    /// The value is interpolated between the minimum and maximum values of the device.
    fn set_output_relative(&mut self, port: usize, value: f64);

    /// Set a specific output to the given `voltage`.
    ///
    /// The voltage must be inside the interval `[get_minimum_output(), get_maximum_output()]`.
    fn set_output(&mut self, port: usize, voltage: ElectricPotential) {
        let supports_negative = self.get_minimum_output().get::<volt>() < 0.0;

        // in [0, 1]
        let value = (voltage - self.get_minimum_output()) / (self.get_maximum_output() - self.get_minimum_output());
        let mut value = value.get::<ratio>();

        if supports_negative {
            // in [-1, 1]
            value = 2.0 * value - 1.0;
        }

        self.set_output_relative(port, value);
    }

    fn get_port_count(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct AnalogOutputOutput(pub f32);

impl From<f32> for AnalogOutputOutput {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<AnalogOutputOutput> for f32 {
    fn from(value: AnalogOutputOutput) -> Self {
        value.0
    }
}

pub trait AnalogOutputDevice {
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput);
    fn get_port_count(&self) -> usize;
}

impl<T: AnalogVoltageOutputDevice> AnalogOutputDevice for T {
    fn get_port_count(&self) -> usize {
        self.get_port_count()
    }

    fn set_output(&mut self, port: usize, value: AnalogOutputOutput) {
        self.set_output_relative(port, f32::from(value) as f64);
    }
}
