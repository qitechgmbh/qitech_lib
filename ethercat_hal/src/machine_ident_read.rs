use crate::{MAX_SUBDEVICES, PDI_LEN, get_async_runtime};
use ethercrab::{MainDevice, SubDeviceGroup};
use tracing::{debug, info, info_span};

#[derive(Debug, Copy, Clone)]
pub struct MachineDeviceInfo {
    pub role: u16,
    pub machine_id: u16,
    pub machine_vendor: u16,
    pub machine_serial: u16,
    pub device_address: u16,
}

pub struct MachineDeviceAddresses {
    pub vendor_word: u16,
    pub serial_word: u16,
    pub machine_word: u16,
    pub role_word: u16,
}

impl Default for MachineDeviceAddresses {
    fn default() -> Self {
        Self {
            vendor_word: 0x0028,
            machine_word: 0x0029,
            serial_word: 0x002a,
            role_word: 0x002b,
        }
    }
}

pub fn read_device_identifications(
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
) -> Result<Vec<MachineDeviceInfo>, anyhow::Error> {
    let _span = info_span!("read_device_identifications").entered();
    let addresses = MachineDeviceAddresses::default();
    let rt = get_async_runtime();
    let res: Result<Vec<MachineDeviceInfo>, ethercrab::error::Error> = rt.block_on(async {
        let mut devices: Vec<MachineDeviceInfo> = Vec::new();

        for subdevice in group.iter(maindevice) {
            let addr = subdevice.configured_address();
            debug!("Reading EEPROM identifications for subdevice 0x{:04X}", addr);

            let vendor = subdevice
                .eeprom_read::<u16>(maindevice, addresses.vendor_word)
                .await?;

            let serial = subdevice
                .eeprom_read::<u16>(maindevice, addresses.serial_word)
                .await?;

            let machine = subdevice
                .eeprom_read::<u16>(maindevice, addresses.machine_word)
                .await?;

            let role = subdevice
                .eeprom_read::<u16>(maindevice, addresses.role_word)
                .await?;

            info!(
                "Subdevice 0x{:04X}: vendor=0x{:04X}, machine=0x{:04X}, serial=0x{:04X}, role=0x{:04X}",
                addr, vendor, machine, serial, role
            );

            devices.push(MachineDeviceInfo {
                role,
                machine_vendor: vendor,
                machine_serial: serial,
                machine_id: machine,
                device_address: addr,
            });
        }
        debug!("Read identifications for {} subdevices", devices.len());
        Ok(devices)
    });

    Ok(res?)
}

pub fn write_device_identifications(
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    identifications: &[MachineDeviceInfo],
) -> Result<(), anyhow::Error> {
    let _span = info_span!("write_device_identifications").entered();
    info!(
        "Writing {} device identification(s) to EEPROM",
        identifications.len()
    );
    let addresses = MachineDeviceAddresses::default();
    let rt = get_async_runtime();
    let res: Result<(), ethercrab::error::Error> = rt.block_on(async {
        for subdevice in group.iter(maindevice) {
            let addr = subdevice.configured_address();
            let info = match identifications
                .iter()
                .find(|i| i.device_address == addr)
            {
                Some(info) => info,
                None => {
                    debug!(
                        "No identification found for subdevice 0x{:04X}, skipping",
                        addr
                    );
                    continue;
                }
            };

            debug!(
                "Writing EEPROM for subdevice 0x{:04X}: vendor=0x{:04X}, machine=0x{:04X}, serial=0x{:04X}, role=0x{:04X}",
                addr, info.machine_vendor, info.machine_id, info.machine_serial, info.role
            );

            subdevice
                .eeprom_write_dangerously(maindevice, addresses.vendor_word, info.machine_vendor)
                .await?;
            subdevice
                .eeprom_write_dangerously(maindevice, addresses.serial_word, info.machine_serial)
                .await?;
            subdevice
                .eeprom_write_dangerously(maindevice, addresses.machine_word, info.machine_id)
                .await?;
            subdevice
                .eeprom_write_dangerously(maindevice, addresses.role_word, info.role)
                .await?;
        }

        info!("EEPROM identifications written successfully");
        Ok(())
    });

    Ok(res?)
}
