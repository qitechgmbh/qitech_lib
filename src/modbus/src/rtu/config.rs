use std::time::Duration;

use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

pub struct ClientConfig<'a>
{
    pub path:         &'a str,
    pub baud_rate:    u32,
    pub data_bits:    DataBits,
    pub parity:       Parity,
    pub flow_control: FlowControl,
    pub stop_bits:    StopBits,
    pub timeout:      Duration,
}

impl<'a> ClientConfig<'a> 
{
    pub fn char_time(&self) -> Duration 
    {
        let data_bits = match self.data_bits 
        {
            DataBits::Five => 5,
            DataBits::Six => 6,
            DataBits::Seven => 7,
            DataBits::Eight => 8,
        };

        let parity_bit = match self.parity 
        {
            Parity::None => 0,
            _ => 1,
        };

        let stop_bits = match self.stop_bits 
        {
            StopBits::One => 1,
            StopBits::Two => 2,
        };

        let bits_per_char = 1 + data_bits + parity_bit + stop_bits;

        // nanoseconds = (bits / baud) * 1_000_000_000
        let nanos = (bits_per_char as u64 * 1_000_000_000) / self.baud_rate as u64;

        Duration::from_nanos(nanos)
    }
}