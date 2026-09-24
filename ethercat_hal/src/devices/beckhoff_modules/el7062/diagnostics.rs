use crate::EtherCATThreadChannel;
use log::{debug, info};
use std::time::Duration;

/// Escape alias listing for the EL7062's DiagMessages facility so the ESI
/// TextID table does not get duplicated into every consumer.
pub fn diag_name(text_id: u16) -> Option<&'static str> {
    match text_id {
        0x4101 => Some("Amplifier-Overtemperature"),
        0x4102 => Some("PDO-configuration is incompatible to the selected mode of operation"),
        0x4103 => Some("Undervoltage Us"),
        0x4104 => Some("Overvoltage Us"),
        0x4400 => Some("Calibration data corrupted or missing"),
        0x4411 => Some("DC-Link undervoltage"),
        0x4412 => Some("DC-Link overvoltage"),
        0x8103 => Some("Undervoltage Us"),
        0x8104 => Some("Amplifier-Overtemperature"),
        0x8105 => Some("PD-Watchdog"),
        0x8144 => Some("Hardware fault"),
        0x817F => Some("Error"),
        0x8404 => Some("Overcurrent"),
        0x8406 => Some("Undervoltage DC-Link"),
        0x8407 => Some("Overvoltage DC-Link"),
        0x840A => Some("Overall current threshold exceeded"),
        0x840B => Some("Commutation error"),
        0x840C => Some("Motor not connected"),
        0x840F => Some("Commutation Type requires an encoder, but feedback is disabled"),
        0x8415 => Some("Invalid modulo range"),
        0x8417 => Some("Maximum rotating field velocity exceeded"),
        0x841F => Some("Torque limitation too low"),
        0x8422 => Some("Drive configuration missing"),
        0x8423 => Some("Invalid process data format (singleturn+multiturn bits != 32)"),
        0x8441 => Some("Maximum following error distance exceeded"),
        0x8442 => Some("Encoder-Resolution insufficient"),
        0x8443 => Some("Combination of Mode of Operation and Commutation Type is invalid"),
        0x8452 => Some("Drive error during positioning"),
        0x8457 => Some("Invalid value for Target velocity"),
        0x8458 => Some("Invalid value for Target position"),
        0x8459 => Some("Emergency stop active"),
        _ => None,
    }
}

/// Dump the EL7062 DiagMessages history (`0x10F3`).
///
/// Record layout per ETG.1020 (see `ethercat_hal::debugging::diagnosis_history`):
/// bytes 0..3 DiagCode, 4..5 Flags, 6..7 TextID, 8..15 timestamp, then P1/P2 data.
/// Mailbox reads are retried because they can race the cyclic frames in OP.
pub fn dump_diag_messages(channel: &EtherCATThreadChannel, addr: u16) {
    let read_u8 = |sub: u8| -> Option<u8> {
        for _ in 0..3 {
            if let Ok(v) = channel.sdo_read::<u8>(addr, 0x10F3, sub) {
                return Some(v);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    };
    let new_available = read_u8(0x04).unwrap_or(0);
    let count = read_u8(0x00).unwrap_or(0);
    let newest = read_u8(0x02).unwrap_or(0);
    info!(
        "DiagMessages (0x10F3): slots={} newest_index={} new_available={}",
        count, newest, new_available
    );
    if count == 0 {
        return;
    }
    // Messages live in subs 0x06..; scan only the meaningful window around the
    // newest index (whole ring if newest is unknown). 16 reads max.
    let window = if newest >= 0x06 { newest - 0x05 } else { 8 }.min(15);
    let end = 0x06u8 + window;
    let mut shown = 0;
    for sub in 0x06u8..end {
        match channel.sdo_read_raw(addr, 0x10F3, sub) {
            Ok(bytes) => {
                if bytes.iter().all(|&b| b == 0) {
                    continue;
                }
                shown += 1;
                let diag_code = if bytes.len() >= 4 {
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                } else {
                    0
                };
                let flags = if bytes.len() >= 6 {
                    u16::from_le_bytes([bytes[4], bytes[5]])
                } else {
                    0
                };
                let text_id = if bytes.len() >= 8 {
                    u16::from_le_bytes([bytes[6], bytes[7]])
                } else {
                    0
                };
                let msg_type = match flags {
                    0x0000 => "Info",
                    0x0001 => "Warning",
                    0x0002 => "Error",
                    _ => "Unknown",
                };
                let raw = if bytes.len() >= 26 {
                    &bytes[..26]
                } else {
                    &bytes[..]
                };
                let ts = if bytes.len() >= 16 {
                    u64::from_le_bytes([
                        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
                        bytes[14], bytes[15],
                    ])
                } else {
                    0
                };
                info!(
                    "  [{}] sub=0x{:02X} {} TextID=0x{:04X} DiagCode=0x{:04X} ts=0x{:016X} msg(26B)={:02X?}",
                    shown - 1,
                    sub,
                    msg_type,
                    text_id,
                    diag_code & 0xFFFF,
                    ts,
                    raw,
                );
                if let Some(name) = diag_name(text_id) {
                    info!("       -> {name}");
                }
            }
            Err(e) => {
                debug!("  [{}] 0x10F3:0x{:02X} read failed: {}", shown, sub, e);
            }
        }
    }
    if shown == 0 {
        info!("  (message slots present but all empty)");
    }
}