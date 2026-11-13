//! Debug log of every SDO write, appended to [`SDO_WRITE_LOG_PATH`].

use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SDO_WRITE_LOG_PATH: &str = "/tmp/sdo_writes.txt";

/// Append one line per SDO write:
/// `<unix time> dev=0x1001 0x8000:0x06 = 1 (0x1) -> ok`
///
/// Failing to write the log only warns, so it never affects the SDO write itself.
pub(crate) fn log_sdo_write<V, T, E>(
    device_address: u16,
    index: u16,
    sub_index: u8,
    value: V,
    result: &Result<T, E>,
) where
    V: fmt::Display + fmt::LowerHex,
    E: fmt::Debug,
{
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let outcome = match result {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("err {e:?}"),
    };
    // One write_all per line; O_APPEND keeps lines from different threads intact.
    let line = format!(
        "{}.{:06} dev={device_address:#06x} {index:#06x}:{sub_index:#04x} = {value} ({value:#x}) -> {outcome}\n",
        now.as_secs(),
        now.subsec_micros(),
    );

    let res = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SDO_WRITE_LOG_PATH)
        .and_then(|mut file| file.write_all(line.as_bytes()));
    if let Err(e) = res {
        tracing::warn!("Failed to append to {SDO_WRITE_LOG_PATH}: {e}");
    }
}
