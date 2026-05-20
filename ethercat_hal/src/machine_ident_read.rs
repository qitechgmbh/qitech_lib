use crate::{MAX_SUBDEVICES, PDI_LEN, get_async_runtime};
use ethercrab::{MainDevice, SubDeviceGroup};

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
                device_address: subdevice.configured_address(),
            });
        }
        // Return the successfully populated vector wrapped in Ok
        Ok(devices)
    });

    Ok(res?)
}

pub fn write_device_identifications(
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    identifications: &[MachineDeviceInfo],
) -> Result<(), anyhow::Error> {
    tracing::info!("writing to device identifications");
    let addresses = MachineDeviceAddresses::default();
    let rt = get_async_runtime();
    let res: Result<(), ethercrab::error::Error> = rt.block_on(async {
        for (subdevice, info) in group.iter(maindevice).zip(identifications.iter()) {
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

        Ok(())
    });

    Ok(res?)
}
