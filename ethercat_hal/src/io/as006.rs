use units::electric_current::milliampere;
use super::analog_input::AnalogInputInput;
use super::analog_input::physical::{AnalogInputRange, AnalogInputValue};

/// Helper to safely extract current in mA from raw analog input.
/// Returns `None` if there is a wiring error or if the input isn't a current measurement.
pub fn get_current_ma(input_data: &AnalogInputInput, range: &AnalogInputRange) -> Option<f64> {
    if input_data.wiring_error {
        return None;
    }

    match input_data.get_physical(range) {
        AnalogInputValue::Current(c) => Some(c.get::<milliampere>()),
        _ => None,
    }
}

/// AS006 flow sensor converter (4–20 mA → l/min).
///
/// Formula: Q [l/min] = 0.938 × (I [mA] − 4)
/// Range: 0 l/min at 4 mA, 15 l/min at 20 mA
pub fn calculate_as006_flow_lpm(input_data: &AnalogInputInput, range: &AnalogInputRange) -> Option<f64> {
    let current_ma = get_current_ma(input_data, range)?;
    Some(0.938 * (current_ma - 4.0))
}

/// AS006 temperature sensor converter (4–20 mA → °C).
///
/// Formula: T [°C] = 9.375 × (I [mA] − 4) − 25
/// Range: −25 °C at 4 mA, 125 °C at 20 mA
pub fn calculate_as006_temperature_celsius(input_data: &AnalogInputInput, range: &AnalogInputRange) -> Option<f64> {
    let current_ma = get_current_ma(input_data, range)?;
    Some(9.375 * (current_ma - 4.0) - 25.0)
}