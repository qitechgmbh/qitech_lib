use physical::{AnalogInputRange, AnalogInputValue};
use units::{ElectricCurrent, electric_current::ampere};
pub mod physical;

pub trait AnalogCurrentInputDevice {
    /// Get the minimum current this device can output on a single port
    fn get_minimum_current(&self) -> ElectricCurrent;
    /// Get the maximum current this device can output on a single port
    fn get_maximum_current(&self) -> ElectricCurrent;

    /// Get a specific input relative to the minimum and maximum value.
    /// If the retured `Option` is `None´, a wireing error occured or
    /// actual current present at the physical input is out of range.
    ///
    /// The given `value` will be in the interval `[-1, 1]` for devices that support negative
    /// input values and in the interval `[0, 1]` for devices with only positive input values.
    /// The value should be interpolated between the minimum and maximum values of the device.
    fn get_current_relative(&self, port: usize) -> Option<f64>;

    /// Get the current of a specific input.
    /// If the retured `Option` is `None´, a wireing error occured or
    /// actual current present at the physical input is out of range.
    ///
    /// The current will be inside the interval `[get_minimum_current(), get_maximum_current()]`.
    fn get_current(&self, port: usize) -> Option<ElectricCurrent> {
        let value = from_relative(
            self.get_current_relative(port)?,
            self.get_minimum_current().get::<ampere>(),
            self.get_maximum_current().get::<ampere>(),
        );
        Some(ElectricCurrent::new::<ampere>(value))
    }

    fn get_port_count(&self) -> usize;
}

fn from_relative(value: f64, min: f64, max: f64) -> f64 {
    let supports_negative = min < 0.0;
    let mut relative = value;

    if supports_negative {
        // in [-1, 1] before
        relative = (relative + 1.0) * 0.5;
        // in [0, 1] after
    }

    // in [0, 1] before
    relative = min + relative * (max - min);
    // in [min, max] before

    relative
}

#[derive(Debug, Clone)]
pub struct AnalogInputInput {
    /// from -1.0 to 1.0
    /// Can be converted to voltage or mA knowning the type and range of the device
    pub normalized: f32,
    pub wiring_error: bool,
}

impl AnalogInputInput {
    /// Convert to physical value
    pub fn get_physical(&self, range: &AnalogInputRange) -> AnalogInputValue {
        range.normalized_to_physical(self.normalized)
    }
}

pub trait AnalogInputDevice {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error>;
    fn analog_input_range(&self) -> AnalogInputRange;
    fn get_port_count(&self) -> usize;
}

impl<T: AnalogCurrentInputDevice> AnalogInputDevice for T {
    fn get_input(&self, port: usize) -> Result<AnalogInputInput, anyhow::Error> {
        if let Some(value) = self.get_current_relative(port) {
            Ok(AnalogInputInput {
                normalized: value as f32,
                wiring_error: false,
            })
        } else {
            Ok(AnalogInputInput {
                normalized: 0.0,
                wiring_error: true,
            })
        }
    }

    fn get_port_count(&self) -> usize {
        self.get_port_count()
    }

    fn analog_input_range(&self) -> AnalogInputRange {
        let supports_negative = self.get_minimum_current().get::<ampere>() < 0.0;
        let min_raw = if supports_negative { i16::MIN } else { 0x0000 };

        AnalogInputRange::Current {
            min: self.get_minimum_current(),
            max: self.get_maximum_current(),
            min_raw,
            max_raw: i16::MAX,
        }
    }
}
