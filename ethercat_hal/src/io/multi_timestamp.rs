#[derive(Debug, Default, Clone, Copy)]
/// A multi-timestamp event encodes that a digital input has changed
/// to a specific boolean value at a specific time,
/// or that a digital ouput will change to a specific boolean value at a specified time.
pub struct MultiTimestampEvent {
    /// Value at `dc_timestamp_ns`.
    pub value: bool,
    /// Timestamp in nanoseconds relative to the EtherCAT distributed clock.
    pub dc_timestamp_ns: u64,
}

/// A multi-timestamp input can detect multiple rising or falling edges,
/// called events, per cycle.
/// Events are saved until they are popped.
pub trait MultiTimestampInput {
    /// View the oldest, saved event on the given port.
    fn peek(&self, port: usize) -> Option<&MultiTimestampEvent>;
    /// Pop the oldest, saved event on the given port.
    /// It will no longer be saved after this.
    fn pop(&mut self, port: usize) -> Option<MultiTimestampEvent>;

    /// View all saved events on the given port.
    fn peek_all(&self, port: usize) -> &[MultiTimestampEvent];
    /// Pop all events on the given port.
    /// No events will be saved after this.
    fn pop_all(&mut self, port: usize) -> Vec<MultiTimestampEvent>;

    /// Total number of multi-timestamp inputs.
    fn get_port_count(&self) -> usize;
}

/// A multi-timestamp output can generate multiple rising or falling edges,
/// called events, per cycle.
pub trait MultiTimestampOutput {
    /// Schedule a single event on the given port.
    fn push(&mut self, port: usize, event: MultiTimestampEvent);
    /// Schedule all events on the given port.
    fn push_all(&mut self, port: usize, events: &[MultiTimestampEvent]);

    /// Total number of multi-timestamp outputs.
    fn get_port_count(&self) -> usize;
}
