use std::time::Duration;
use ethercrab::{DcSync, EtherCrabWireRead, EtherCrabWireSized, MainDevice, SubDeviceGroup};
use crate::{MachineDeviceAddresses, SdoReadRequest, SdoRequest, SdoType, controller::ethercrab_controller::PDI_LEN, get_async_runtime};
use crate::MAX_SUBDEVICES;
use crate::MachineDeviceInfo;
/*
 Value type needs to have EtherCrabWireWriteSized at the least to be able to write with ethecrab
*/
pub fn sdo_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES , PDI_LEN>,
    request: SdoRequest,
) -> Result<(), anyhow::Error> {
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res = match request.type_flag {
                SdoType::U8 => runtime.block_on(device.sdo_write(
                    request.index,
                    request.sub_index as u8,
                    request.data[0],
                )),
                SdoType::U16 => runtime.block_on(device.sdo_write(
                    request.index,
                    request.sub_index as u8,
                    u16::from_le_bytes([request.data[0], request.data[1]]),
                )),
                SdoType::U32 => runtime.block_on(device.sdo_write(
                    request.index,
                    request.sub_index as u8,
                    u32::from_le_bytes(request.data),
                )),
                SdoType::I16 => runtime.block_on(device.sdo_write(
                    request.index,
                    request.sub_index as u8,
                    i16::from_le_bytes([request.data[0], request.data[1]]),
                )),
                SdoType::I32 => runtime.block_on(device.sdo_write(
                    request.index,
                    request.sub_index as u8,
                    i32::from_le_bytes(request.data),
                )),
                SdoType::BOOL => {
                    let b: bool = request.data[0] == 1;
                    runtime.block_on(device.sdo_write(request.index, request.sub_index as u8, b))
                }
            };
            return Ok(res?);
        }
    }
    Err(anyhow::anyhow!("Unknown Subdevice"))
}

pub fn sdo_read<T>(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoReadRequest,
) -> Result<T, anyhow::Error>
where
    T: EtherCrabWireRead + EtherCrabWireSized,
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res: Result<T, ethercrab::error::Error> =
                runtime.block_on(device.sdo_read::<T>(request.index, request.sub_index as u8));
            return Ok(res?);
        }
    }
    Err(anyhow::anyhow!("Unknown Subdevice"))
}

pub fn enable_dc_sync(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
) -> Result<(), anyhow::Error> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_dc_sync(DcSync::Sync0);
                return Ok(());
            }
        }
        return Err(anyhow::anyhow!("Unknown Subdevice"));
    })
}

pub fn enable_dc_sync01(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
    sync1_period: Duration,
) -> Result<(), anyhow::Error> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_dc_sync(DcSync::Sync01 { sync1_period });
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Unknown Subdevice"))
    })
}

pub fn configure_oversampling(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
    oversampling_settings: &[(u16, u16)],
) -> Result<(), anyhow::Error> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_oversampling(oversampling_settings);
                return Ok(());
            }
        }
        Err(anyhow::anyhow!(
            "Unknown Subdevice at address 0x{:04X}",
            device_address
        ))
    })
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
        for subdevice in group.iter(maindevice) {
            let info = match identifications
                .iter()
                .find(|i| i.device_address == subdevice.configured_address())
            {
                Some(info) => info,
                None => {
                    tracing::debug!(
                        "No identification found for subdevice at address {}, skipping",
                        subdevice.configured_address()
                    );
                    continue;
                }
            };

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
