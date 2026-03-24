use crate::{MAX_SUBDEVICES, PDI_LEN, get_async_runtime};
use ethercrab::{MainDevice, SubDeviceGroup};

#[derive(Debug)]
pub struct MachineDeviceInfo {
    pub role: u16,
    pub machine_id: u16,
    pub machine_vendor: u16,
    pub machine_serial: u16,
    pub device_address: usize,
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
    let addresses = MachineDeviceAddresses::default();
    let rt = get_async_runtime();
    let res: Result<Vec<MachineDeviceInfo>, ethercrab::error::Error> = rt.block_on(async {
        let mut devices: Vec<MachineDeviceInfo> = Vec::new();

        for subdevice in group.iter(maindevice) {
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

            devices.push(MachineDeviceInfo {
                role,
                machine_vendor: vendor,
                machine_serial: serial,
                machine_id: machine,
                device_address: subdevice.configured_address() as usize,
            });
        }
        // Return the successfully populated vector wrapped in Ok
        Ok(devices)
    });

    Ok(res?)
}
