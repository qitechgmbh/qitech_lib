const U16_MAX: i128 = std::u16::MAX as i128;

/// This is a wrapper for a counter that stores a u16 that frequently overflows or underflows
///
/// We convert the overflows and underflows to an i128 value for easier calculations.
///
/// When overriding the counter we don't set the valeu direclty but schedula it wiht the [`push_override`] so it can be synced with [`pop_override`] to an EtherCAT device.
#[derive(Debug, Clone)]
pub struct CounterWrapperU16U128 {
    counter: i128,
    last_counter: u16,
    last_counter_underflow: bool,
    last_counter_overflow: bool,
    set_counter: Option<i128>,
    /// Raw value a popped override is waiting for the device to adopt.
    pending_override: Option<u16>,
    /// Cycles spent waiting, so a device that never reports the value exactly
    /// cannot freeze the counter forever.
    pending_cycles: u16,
}

impl Default for CounterWrapperU16U128 {
    fn default() -> Self {
        Self::new()
    }
}

impl CounterWrapperU16U128 {
    pub const fn new() -> Self {
        Self {
            counter: 0,
            last_counter: 0,
            last_counter_underflow: false,
            last_counter_overflow: false,
            set_counter: None,
            pending_override: None,
            pending_cycles: 0,
        }
    }

    /// How many cycles to wait for a device to adopt an override before giving
    /// up and resynchronising to whatever it is reporting.
    const OVERRIDE_TIMEOUT_CYCLES: u16 = 200;

    pub const fn update(&mut self, counter: u16, counter_underflow: bool, counter_overflow: bool) {
        // While an override is in flight the raw value is about to jump to the
        // value we asked for. That jump is not motion, so track the raw counter
        // without accumulating it - otherwise the widened counter picks up the
        // whole difference between the old and new raw values as travel.
        match self.pending_override {
            Some(pending) => {
                self.last_counter = counter;
                self.last_counter_underflow = counter_underflow;
                self.last_counter_overflow = counter_overflow;
                self.pending_cycles += 1;
                if counter == pending || self.pending_cycles >= Self::OVERRIDE_TIMEOUT_CYCLES {
                    self.pending_override = None;
                    self.pending_cycles = 0;
                }
                return;
            }
            None => {}
        }

        // Only process rising edges of the underflow and overflow flags
        let counter_underflow_rising = counter_underflow && !self.last_counter_underflow;
        let counter_overflow_rising = counter_overflow && !self.last_counter_overflow;

        let change = counter_change(
            self.last_counter,
            counter,
            counter_underflow_rising,
            counter_overflow_rising,
        );
        self.counter += change as i128;
        self.last_counter = counter;
        self.last_counter_underflow = counter_underflow;
        self.last_counter_overflow = counter_overflow;
    }

    pub const fn current(&self) -> i128 {
        self.counter
    }

    /// Schedules a counter override
    ///
    /// The value is only set when `pop_set` is called.
    pub const fn push_override(&mut self, new_counter: i128) {
        self.set_counter = Some(new_counter);
    }

    /// Return the override value as an u16 and overrides the current counter.
    pub const fn pop_override(&mut self) -> Option<u16> {
        match self.set_counter {
            Some(counter) => {
                // set our counter to the new value
                self.counter = counter;

                // Convert the i128 counter to u16
                let u16_counter = set_counter_u16_to_i128(counter);

                // Clear the set counter
                self.set_counter = None;

                // The device has not adopted this yet. Until it reports the new
                // raw value back, `update` must not treat the difference as
                // travel.
                self.pending_override = Some(u16_counter);
                self.pending_cycles = 0;

                Some(u16_counter)
            }
            None => None, // No set counter available
        }
    }

    /// Returns the override value as an i128
    pub const fn get_override(&self) -> Option<i128> {
        self.set_counter
    }
}

const fn counter_change(
    last_counter: u16,
    counter: u16,
    counter_underflow: bool,
    counter_overflow: bool,
) -> i32 {
    let base_change = (counter as i32) - (last_counter as i32);

    if counter_overflow {
        // Only add the U16_MAX + 1 if there was an actual wrap-around
        // This is typically indicated by the counter being less than the last counter
        if counter < last_counter || (last_counter == counter && base_change != 0) {
            base_change + (U16_MAX + 1) as i32
        } else {
            // If counter values are the same and there's no real change,
            // don't add the overflow value
            base_change
        }
    } else if counter_underflow {
        // Only subtract the U16_MAX + 1 if there was an actual wrap-around
        if counter > last_counter || (last_counter == counter && base_change != 0) {
            base_change - (U16_MAX + 1) as i32
        } else {
            base_change
        }
    } else {
        // Normal case
        base_change
    }
}

const fn set_counter_u16_to_i128(new_counter: i128) -> u16 {
    // Use modulo arithmetic to handle both positive and negative values
    let modulo = (new_counter % (U16_MAX + 1)) as i32;

    if modulo < 0 {
        // If negative, wrap around from the end
        (modulo + (U16_MAX + 1) as i32) as u16
    } else {
        modulo as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: `pop_override` used to set `counter` but leave `last_counter`
    /// holding the pre-override raw value, so the very next `update` added
    /// `new_raw - old_raw` as if the axis had travelled it. Homing at raw 1328
    /// produced a widened position of -1328 instead of 0.
    #[test]
    fn test_override_does_not_produce_phantom_travel() {
        let mut wrapper = CounterWrapperU16U128::new();
        wrapper.update(1328, false, false);
        assert_eq!(wrapper.current(), 1328);

        // Ask for 0 and hand the value to the device.
        wrapper.push_override(0);
        assert_eq!(wrapper.pop_override(), Some(0));
        assert_eq!(wrapper.current(), 0);

        // The device has not adopted it yet and still reports the old raw value.
        wrapper.update(1328, false, false);
        assert_eq!(wrapper.current(), 0, "stale raw value must not count as travel");

        // Now it adopts the override.
        wrapper.update(0, false, false);
        assert_eq!(wrapper.current(), 0);

        // And normal counting resumes from there.
        wrapper.update(25, false, false);
        assert_eq!(wrapper.current(), 25);
    }

    /// A device that never reports the exact override value must not freeze the
    /// counter permanently.
    #[test]
    fn test_override_wait_is_bounded() {
        let mut wrapper = CounterWrapperU16U128::new();
        wrapper.push_override(500);
        assert_eq!(wrapper.pop_override(), Some(500));
        for _ in 0..CounterWrapperU16U128::OVERRIDE_TIMEOUT_CYCLES {
            wrapper.update(9999, false, false);
        }
        assert_eq!(wrapper.current(), 500);
        // Resynchronised: further motion accumulates again.
        wrapper.update(10_009, false, false);
        assert_eq!(wrapper.current(), 510);
    }

    #[test]
    fn test_set_counter_u16_to_i128() {
        // Test normal values within u16 range
        assert_eq!(set_counter_u16_to_i128(0), 0);
        assert_eq!(set_counter_u16_to_i128(1000), 1000);
        assert_eq!(set_counter_u16_to_i128(65535), 65535);

        // Test overflow values
        assert_eq!(set_counter_u16_to_i128(65536), 0);
        assert_eq!(set_counter_u16_to_i128(65537), 1);
        assert_eq!(set_counter_u16_to_i128(65536 + 1000), 1000);
        assert_eq!(set_counter_u16_to_i128(65536 * 2 + 5), 5);

        // Test negative values
        assert_eq!(set_counter_u16_to_i128(-1), 65535);
        assert_eq!(set_counter_u16_to_i128(-2), 65534);
        assert_eq!(set_counter_u16_to_i128(-1000), 64536);
        assert_eq!(set_counter_u16_to_i128(-65536), 0);
        assert_eq!(set_counter_u16_to_i128(-65537), 65535);

        // Test large values
        assert_eq!(set_counter_u16_to_i128(65536 * 1000 + 42), 42);
        assert_eq!(set_counter_u16_to_i128(-65536 * 1000 - 42), 65494);
    }

    #[test]
    fn test_set_counter_u16_to_i128_roundtrip() {
        // Test that converting to u16 and back to i128 gives consistent results
        for i in -100..100 {
            let u16_value = set_counter_u16_to_i128(i);
            let i128_value = i % 65536;
            let normalized_i128 = if i128_value < 0 {
                i128_value + 65536
            } else {
                i128_value
            };
            assert_eq!(u16_value, normalized_i128 as u16);
        }
    }

    #[test]
    fn test_normal_increment() {
        // Simple increment with no overflow/underflow
        assert_eq!(counter_change(100, 105, false, false), 5);
    }

    #[test]
    fn test_normal_decrement() {
        // Simple decrement with no overflow/underflow
        assert_eq!(counter_change(100, 95, false, false), -5);
    }

    #[test]
    fn test_overflow() {
        // Test overflow: counter went from 65535 to 2
        // The difference should be 2 - 65535 + 65536 = 3
        assert_eq!(counter_change(65535, 2, false, true), 3);

        // Test overflow: counter went from 65535 to 0
        // The difference should be 0 - 65535 + 65536 = 1
        assert_eq!(counter_change(65535, 0, false, true), 1);
    }

    #[test]
    fn test_underflow() {
        // Test underflow: counter went from 0 to 65533
        // The difference should be 65533 - 0 - 65536 = -3
        assert_eq!(counter_change(0, 65533, true, false), -3);

        // Test underflow: counter went from 0 to 65535
        // The difference should be 65535 - 0 - 65536 = -1
        assert_eq!(counter_change(0, 65535, true, false), -1);
    }

    #[test]
    fn test_large_changes() {
        // Test a large forward jump (no overflow)
        assert_eq!(counter_change(1000, 5000, false, false), 4000);

        // Test a large backward jump (no underflow)
        assert_eq!(counter_change(5000, 1000, false, false), -4000);
    }

    #[test]
    fn test_edge_cases() {
        // Test consecutive overflows
        // First overflow: 65535 -> 10
        let diff1 = counter_change(65535, 10, false, true);
        assert_eq!(diff1, 11);

        // Second overflow: counter from 10 to 20
        let diff2 = counter_change(10, 20, false, false);
        assert_eq!(diff2, 10);

        // Test max positive difference
        assert_eq!(counter_change(0, 65535, false, false), 65535);

        // Test max negative difference
        assert_eq!(counter_change(65535, 0, false, false), -65535);
    }

    #[test]
    fn test_integration() {
        // Simulate a full rotation: 0 -> 65535 -> 0
        let mut position: i128 = 0;
        #[allow(unused_assignments)]
        let mut raw_counter: u16 = 0;

        // Increment to 65530
        raw_counter = 65530;
        position += counter_change(0, raw_counter, false, false) as i128;
        assert_eq!(position, 65530);

        // Increment to 65535
        let mut prev = raw_counter;
        raw_counter = 65535;
        position += counter_change(prev, raw_counter, false, false) as i128;
        assert_eq!(position, 65535);

        // Overflow to 5
        prev = raw_counter;
        raw_counter = 5;
        position += counter_change(prev, raw_counter, false, true) as i128;
        assert_eq!(position, 65541); // 65535 + 6

        // Normal decrement to 65530 (no underflow)
        prev = raw_counter;
        raw_counter = 65530;
        position += counter_change(prev, raw_counter, false, false) as i128;
        assert_eq!(position, 131066); // 65541 + (65530 - 5) = 65541 + 65525 = 131066

        // Now test a true underflow: position 5 -> 65535
        position = 5; // Reset position to 5
        raw_counter = 5; // Set raw counter to match
        prev = raw_counter;
        raw_counter = 65535;
        position += counter_change(prev, raw_counter, true, false) as i128;
        assert_eq!(position, -1); // 5 + (65535 - 5 - 65536) = 5 - 6 = -1
    }
}
