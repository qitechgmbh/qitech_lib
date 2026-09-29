use crate::EtherCATThreadChannel;
use log::{debug, error, info};
use std::time::Duration;

/// TextID alias listing for the EL7062's DiagMessages facility so the ESI
/// TextID table does not get duplicated into every consumer.
///
/// Every entry is the English `MessageText` (`LcId="1033"`) for that TextId in
/// the device's own `EL7062.xml` ESI, and the table covers all 49 TextIds the
/// ESI defines. Keep it in sync when the ESI is updated.
///
/// Note that a TextID (16-bit, bytes 6..7 of a `0x10F3` record) is a different
/// namespace from the 32-bit Diag Code (bytes 0..3) tabulated in
/// `resources/EL7062 EtherCAT Diagnostics + CoE Params.md`. Several values
/// appear in both namespaces with the same numeric value, but the drive reports
/// Diag Codes that this table cannot resolve, and vice versa.
pub fn diag_name(text_id: u16) -> Option<&'static str> {
    match text_id {
        0x4101 => Some("Amplifier-Overtemperature"),
        0x4102 => Some("PDO-configuration is incompatible to the selected mode of operation"),
        0x4103 => Some("Undervoltage Us"),
        0x4104 => Some("Overvoltage Us"),
        0x410B => Some("Error detected, but disabled by suppression mask"),
        0x4400 => Some("Calibration data corrupted or missing"),
        0x4411 => Some("DC-Link undervoltage"),
        0x4412 => Some("DC-Link overvoltage"),
        0x441E => Some("Invalid configuration of touchprobe inputs"),
        0x4424 => Some("Modes of operation invalid"),
        0x8103 => Some("Undervoltage Us"),
        0x8104 => Some("Amplifier-Overtemperature"),
        0x8105 => Some("PD-Watchdog"),
        0x8144 => Some("Hardware fault"),
        0x817F => Some("Error"),
        0x81B0 => Some("Content of PDO 0x%X is invalid: Item 0x%X:%X cannot be mapped"),
        0x81B1 => Some(
            "Content of PDO 0x%X is invalid: Item 0x%X:%X has an unsupported length (%d bit)",
        ),
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
        0x8420 => Some("Teach-In Process (%d) failed"),
        0x8421 => Some("Teach-In Process Timeout (DC-Link, ...)"),
        0x8422 => Some("Drive configuration missing"),
        0x8423 => Some("Invalid process data format (singleturn+multiturn bits != 32)"),
        0x8441 => Some("Maximum following error distance exceeded"),
        0x8442 => Some("Encoder-Resolution insufficient"),
        0x8443 => Some("Combination of Mode of Operation and Commutation Type is invalid"),
        0x8449 => Some("Target position not in modulo range"),
        0x8450 => Some("Invalid start type 0x%x"),
        0x8451 => Some("Invalid limit switch level"),
        0x8452 => Some("Drive error during positioning"),
        0x8453 => Some("Latch unit will be used by multiple modules"),
        0x8454 => Some("Drive not in control"),
        0x8455 => Some("Invalid value for \"Target acceleration\""),
        0x8456 => Some("Invalid value for \"Target deceleration\""),
        0x8457 => Some("Invalid value for Target velocity"),
        0x8458 => Some("Invalid value for Target position"),
        0x8459 => Some("Emergency stop active"),
        0x845A => Some("Target position exceeds Modulofactor"),
        0x845B => Some("Drive must be disabled"),
        0x845D => Some("Modulo factor invalid"),
        0x845E => Some("Invalid target position window"),
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
        for attempt in 1..=3 {
            match channel.sdo_read::<u8>(addr, 0x10F3, sub) {
                Ok(v) => return Some(v),
                Err(e) => {
                    debug!(
                        "  0x10F3:0x{:02X} read failed (attempt {}/3): {}",
                        sub, attempt, e
                    );
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        None
    };
    let new_available = read_u8(0x04);
    let count = read_u8(0x00);
    let newest = read_u8(0x02);
    if count.is_none() || newest.is_none() {
        error!(
            "DiagMessages (0x10F3): header read failed, so no history is available. The \
             terminal only services mailbox SDOs in Init/PreOp/SafeOp, so this dump must not be \
             attempted while the bus is still in Op."
        );
        return;
    }
    let count = count.unwrap();
    let newest = newest.unwrap();
    info!(
        "DiagMessages (0x10F3): slots={} newest_index={} new_available={}",
        count,
        newest,
        new_available.unwrap_or(0)
    );
    if count == 0 {
        return;
    }
    // Messages live in subs 0x06.., and message N sits at subindex 0x05 + N.
    // Read the newest ones, ending at 0x10F3:02 -- not the oldest. The newest
    // window is the one that matters: a fresh encoder or commutation fault lands
    // there, and scanning upwards from 0x06 silently drops it once the ring has
    // more history than the read window. 15 reads max.
    let newest_sub = 0x05u8.saturating_add(newest);
    let last_sub = newest_sub.min(0x37);
    let start_sub = last_sub.saturating_sub(14).max(0x06);
    if start_sub > last_sub {
        info!("  no DiagMessage slots in range (newest_index={newest})");
        return;
    }
    let mut shown = 0;
    for sub in start_sub..=last_sub {
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