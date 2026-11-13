use qitech_lib_ethercat_hal::{
    EtherCATState, MetaSubdevice, StdEcatControl, convert_module_to_meta_ethercat_device,
    init_ethercat,
};
use qitech_lib_ethercat_hal_devices::{IdentifiedDevice, wago_modules::wago_750_354::Wago750_354};
use std::{env, time::Duration};

// EEPROM words where QiTech stores the machine identification of a subdevice
const MACHINE_VENDOR_WORD: u16 = 0x0028;
const MACHINE_ID_WORD: u16 = 0x0029;
const MACHINE_SERIAL_WORD: u16 = 0x002a;
const MACHINE_ROLE_WORD: u16 = 0x002b;

/*
Add Modules IF they exist
*/
fn add_modules(subdevices: &mut Vec<MetaSubdevice>, ecat_control: &StdEcatControl) {
    let coupler_subdev = subdevices
        .iter()
        .find(|s| Wago750_354::identity().matches(s));

    match coupler_subdev {
        Some(coupler) => {
            // Devices connected to 750-354 are not actual ethercat terminals and are commonly called "modules"
            let modules = ecat_control
                .get_channel()
                .get_modules(coupler.device_address, coupler.vendor, None)
                .unwrap();
            for module in &modules {
                let device = convert_module_to_meta_ethercat_device(module, 0, 0);
                subdevices.push(device);
            }
        }
        None => (),
    }
}

/// This example reads all information from the devices that is available in PreOP
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let machine_ident = env::args().nth(2);
    let eth_control = init_ethercat(&interface, None);
    eth_control
        .change_state(EtherCATState::PreOp, Duration::from_secs(5))
        .expect("Failed to go into PreOp");

    let mut subdevices = eth_control
        .try_get_subdevices_vec_sync()
        .expect("Failed to read subdevices");
    add_modules(&mut subdevices, &eth_control);

    for (index, subdevice) in subdevices.iter().enumerate() {
        let read_word = |word| eth_control.eeprom_read(subdevice.device_address, word);
        let ident = (|| {
            Ok::<_, qitech_lib_ethercat_hal::ChannelError>((
                read_word(MACHINE_VENDOR_WORD)?,
                read_word(MACHINE_ID_WORD)?,
                read_word(MACHINE_SERIAL_WORD)?,
                read_word(MACHINE_ROLE_WORD)?,
            ))
        })();
        let name = subdevice.get_name().expect("Failed to read name!");
        let (trunk, branch) = if index == subdevices.len() - 1 {
            (" ", "└")
        } else {
            ("│", "├")
        };

        println!(
            "{}── {} at 0x{:04x}: product_id: 0x{:08x}, revision: 0x{:08x}, vendor: 0x{:08x}, start_tx: {}, end_tx: {}, start_rx: {}, end_rx: {} (bits)",
            branch,
            name,
            subdevice.device_address,
            subdevice.product_id,
            subdevice.revision,
            subdevice.vendor,
            subdevice.start_tx,
            subdevice.end_tx,
            subdevice.start_rx,
            subdevice.end_rx
        );
        if machine_ident.is_some() {
            match ident {
                Ok((machine_vendor, machine_id, machine_serial, role)) => println!(
                    "{}   └──Machine Identification: machine_vendor: {}, machine_id: {}, machine_serial: {}, role: {}",
                    trunk, machine_vendor, machine_id, machine_serial, role,
                ),
                Err(e) => println!("{}   └──Machine Identification: unavailable ({})", trunk, e),
            }
        }
    }
}
