pub mod diagnosis_history;

use crate::EtherCATThreadChannel;
use log::debug;

/// Dump the distributed-clock (DC) registers of a subdevice while the master is
/// ramping through the OP transition. Best-effort, purely diagnostic.
pub fn dump_dc_registers(channel: &EtherCATThreadChannel, addr: u16) {
    let pairs: [(u16, &str); 11] = [
        (0x0980, "AssignActivate(classic)"),
        (0x0981, "DcSyncActive(module)"),
        (0x0984, "StartTime-classic-lo"),
        (0x0985, "StartTime-classic-hi"),
        (0x0988, "Sync0Cycle-classic-lo"),
        (0x0989, "Sync0Cycle-classic-hi"),
        (0x098C, "Sync1Cycle-classic-lo"),
        (0x098D, "Sync1Cycle-classic-hi"),
        (0x0990, "StartTime-module-lo"),
        (0x09A0, "Sync0Cycle-module-lo"),
        (0x09A4, "Sync1Cycle-module-lo"),
    ];
    for (reg, label) in pairs {
        match channel.register_read(addr, reg) {
            Ok(v) => debug!("  {label} @0x{reg:04X} = 0x{v:04x}"),
            Err(e) => debug!("  {label} @0x{reg:04X} read failed: {e}"),
        }
    }
}
