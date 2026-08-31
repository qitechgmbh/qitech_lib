use units::ElectricPotential;

pub trait AnalogVoltageOutputDevice {
    /// Get the minimum voltage this device can output on a single port
    fn get_minimum_output(&self) -> ElectricPotential;
    /// Get the maximum voltage this device can output on a single port
    fn get_maximum_output(&self) -> ElectricPotential;

    /// Set a specific output.
    ///
    /// The given value must be in the interval `[-1, 1]` for devices that support negative
    /// output values and in the interval [0, 1] for devices with only positve output values.
    /// The value is interpolated between the minimum and maximum values of the device.
    fn set_output(&mut self, port: usize, value: f64);

    fn get_port_count(&self) -> usize;
}

#[derive(Debug, Clone)]
#[deprecated(note = "Use AnalogVoltageOutputDevice instead, which supports floats directly")]
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

#[deprecated(note = "Use AnalogVoltageOutputDevice instead")]
pub trait AnalogOutputDevice {
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput);
    fn get_port_count(&self) -> usize;
}

impl<T: AnalogVoltageOutputDevice> AnalogOutputDevice for T {
    fn get_port_count(&self) -> usize {
        self.get_port_count()
    }

    fn set_output(&mut self, port: usize, value: AnalogOutputOutput) {
        self.set_output(port, f32::from(value) as f64);
    }
}
