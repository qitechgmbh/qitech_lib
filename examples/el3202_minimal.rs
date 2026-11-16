use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::Configuration,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el3202::{EL3202, EL3202Configuration, EL3202_PRODUCT_ID},
        beckhoff_modules::el3204::{EL3204Configuration, EL3204_PRODUCT_ID},
        beckhoff_modules::el3062::{EL3062Configuration, EL3062_PRODUCT_ID},
    },
    init_ethercat,
    io::temperature_input::TemperatureInputDevice,
};
use std::{env, time::Duration};

/// Minimal example: scan the EtherCAT bus and read temperatures from an EL3202
/// (2-channel RTD, PT100/PT1000).
///
/// Usage: sudo -E ./el3202_minimal <interface>   (e.g. eth0)
fn main() {
    // Enable logging: shows ethercrab-internal errors (e.g. PDU index problems)
    // that would otherwise stay invisible. Control via RUST_LOG (e.g. =debug).
    env_logger::init();

    let interface = env::args().nth(1).expect("Usage: el3202_minimal <interface>");
    let eth_control = init_ethercat(&interface, None);
    let mut eth_handle = eth_control.app_handle;

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        if let EtherCATState::PreOp = eth_handle.get_state() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("found {:?} ethercat terminals:", eth_handle.get_subdevice_count());

    // NOTE: In PreOp the subdevices have no PDO offsets yet (start_tx==end_tx)
    // — this list only serves the scan and the config. PDO access requires the
    // list fetched after the Op transition!
    let subdevices_preop = eth_handle.try_get_subdevices_vec_sync().unwrap();
    for sdev in &subdevices_preop {
        println!(
            " - {} (vendor 0x{:x}, product 0x{:x}, revision 0x{:x})",
            sdev.get_name().expect("No utf8 name!"),
            sdev.vendor,
            sdev.product_id,
            sdev.revision
        );
    }

    // Switch EL3202 to PT1000 (factory default: PT100 — the plant is PT1000)
    // IMPORTANT: the HAL only serves SDO/CoE in PreOp → config BEFORE the Op transition!
    for meta in &subdevices_preop {
        if meta.product_id == EL3202_PRODUCT_ID {
            EL3202Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("PT1000 configuration failed");
            println!("EL3202 @0x{:x}: sensor type = PT1000", meta.device_address);
        }
        // EL3204: same PT1000 switch, 4 channels (0x8000..0x8030)
        if meta.product_id == EL3204_PRODUCT_ID {
            EL3204Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("EL3204 PT1000 configuration failed");
            println!(
                "EL3204 @0x{:x}: sensor type = PT1000 (4 channels)",
                meta.device_address
            );
        }
        // EL3062 (oil tank): write default config ≈ factory state.
        // Background 2026-11-15: the (second-hand) terminal reported AL error
        // 0x001E "Invalid Input Configuration" at PreOp→SafeOp even though the
        // SM/PDO/FMMU config was ESI-conform and we had never written to it —
        // suspicion: CoE settings persisted by the previous owner (Beckhoff
        // backup objects). Re-writing defaults cures it.
        if meta.product_id == EL3062_PRODUCT_ID {
            EL3062Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("EL3062 configuration failed");
            println!(
                "EL3062 @0x{:x}: default config (0..10 V, standard PDOs)",
                meta.device_address
            );
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to go into OP");
    // Progress + timeout instead of a silent spin: with init strays
    // (ethercrab#293) this shows whether we are even stuck in the PreOp→Op
    // transition (2026-11-15).
    let mut waited_ms = 0u32;
    loop {
        let state = eth_handle.get_state();
        if state == EtherCATState::Op {
            break;
        }
        if waited_ms % 1000 == 0 {
            println!("waiting for Op ... currently {:?} ({} ms)", state, waited_ms);
        }
        // 30 s: longer than the HAL's own state_transition timeout (20 s) so the
        // HAL can emit its AL status error diagnostics before we give up.
        if waited_ms > 30_000 {
            panic!(
                "Op transition not reached after 30 s (last state: {:?})",
                state
            );
        }
        std::thread::sleep(Duration::from_millis(10));
        waited_ms += 10;
    }
    println!("Op reached — reading temperatures …");

    // After Op: PDO offsets are assigned → now fetch the authoritative list
    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();

    // Track each EL3202 individually (address → own device object)
    let mut modules: Vec<(u16, EL3202)> = subdevices
        .iter()
        .filter(|s| s.vendor == BECKHOFF_VENDOR_ID && s.product_id == EL3202_PRODUCT_ID)
        .map(|s| (s.device_address, EL3202::new()))
        .collect();

    loop {
        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL3202_PRODUCT_ID
                {
                    if let Some((_, el)) = modules
                        .iter_mut()
                        .find(|(addr, _)| *addr == subdevice.device_address)
                    {
                        let subdevice_inputs = &inputs[subdevice.start_tx..subdevice.end_tx];
                        el.input(BitSlice::<u8, Lsb0>::from_slice(subdevice_inputs))
                            .expect("Failed to read Tx PDO");
                    }
                }
            }
        }

        for (addr, el) in &modules {
            print!("@0x{:x}: ", addr);
            for i in 0..el.get_port_count() {
                match el.get_input(i) {
                    Ok(input) => {
                        if input.error || input.overvoltage {
                            // EL32xx: 850.0 °C in the process data = open channel (no sensor)
                            print!("T{}=<open> ", i + 1);
                        } else if input.undervoltage {
                            print!("T{}=<short> ", i + 1);
                        } else {
                            // RtdInput::read() already yields °C (value/10 internally)
                            print!("T{}={:.1}°C ", i + 1, input.temperature);
                        }
                    }
                    Err(e) => print!("T{}={:?} ", i + 1, e),
                }
            }
            println!();
        }
        println!("---");

        eth_handle.send_outputs();
        std::thread::sleep(Duration::from_millis(500));
    }
}
