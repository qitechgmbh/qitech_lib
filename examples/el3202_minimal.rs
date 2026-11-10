use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::Configuration,
    devices::{
        EthercatDevice, NewEthercatDevice,
        beckhoff_modules::el3202::{EL3202, EL3202Configuration, EL3202_PRODUCT_ID},
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
    // Logging einschalten: zeigt ethercrab-interne Fehler (z. B. PDU-Index-Probleme)
    // an, die sonst unsichtbar bleiben. Ansteuerung via RUST_LOG (z. B. =debug).
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

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    for sdev in &subdevices {
        println!(
            " - {} (vendor 0x{:x}, product 0x{:x}, revision 0x{:x})",
            sdev.get_name().expect("No utf8 name!"),
            sdev.vendor,
            sdev.product_id,
            sdev.revision
        );
    }

    // EL3202 auf PT1000 stellen (Auslieferungszustand: PT100 — Anlage ist PT1000)
    // WICHTIG: SDO/CoE wird vom HAL nur im PreOp bedient → Konfig VOR dem Op-Übergang!
    for meta in &subdevices {
        if meta.product_id == EL3202_PRODUCT_ID {
            EL3202Configuration::default()
                .write_config(eth_control.channel.clone(), meta.device_address)
                .expect("PT1000-Konfiguration fehlgeschlagen");
            println!("EL3202 @0x{:x}: Sensortyp = PT1000", meta.device_address);
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to go into OP");
    loop {
        if let EtherCATState::Op = eth_handle.get_state() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    // Alle EL3202 einzeln verfolgen (Adresse → eigenes Device-Objekt)
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
                            // EL32xx: 850,0 °C im Prozessdatum = offener Kanal (kein Fühler)
                            print!("T{}=<offen> ", i + 1);
                        } else if input.undervoltage {
                            print!("T{}=<kurz> ", i + 1);
                        } else {
                            // RtdInput::read() liefert bereits °C (Wert/10 intern)
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
