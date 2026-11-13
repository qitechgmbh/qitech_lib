use crate::{
    ChannelError, MAX_SUBDEVICES, PDI_LEN, get_async_runtime,
    sdo::{SdoReadRequest, SdoRequest, SdoType},
    sdo_log::log_sdo_write,
};
use ethercrab::{DcSync, EtherCrabWireRead, EtherCrabWireSized, MainDevice, SubDeviceGroup};
use std::time::Duration;

/*
 Value type needs to have EtherCrabWireWriteSized at the least to be able to write with ethecrab
*/
pub fn sdo_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoRequest,
) -> Result<(), ChannelError> {
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let index = request.sdo_index.index as u16;
            let sub_index = request.sdo_index.sub_index as u8;

            // Write `value`, then log it together with the result.
            macro_rules! write_logged {
                ($value:expr, $logged:expr) => {{
                    let res = runtime.block_on(device.sdo_write(index, sub_index, $value));
                    log_sdo_write(request.device_address, index, sub_index, $logged, &res);
                    res
                }};
            }

            let res = match request.type_flag {
                SdoType::U8 => write_logged!(request.data[0], request.data[0]),
                SdoType::U16 => {
                    let v = u16::from_le_bytes([request.data[0], request.data[1]]);
                    write_logged!(v, v)
                }
                SdoType::U32 => {
                    let v = u32::from_le_bytes(request.data);
                    write_logged!(v, v)
                }
                SdoType::I16 => {
                    let v = i16::from_le_bytes([request.data[0], request.data[1]]);
                    write_logged!(v, v)
                }
                SdoType::I32 => {
                    let v = i32::from_le_bytes(request.data);
                    write_logged!(v, v)
                }
                // bool has no hex formatting, so it's logged as 0/1
                SdoType::BOOL => {
                    let b = request.data[0] == 1;
                    write_logged!(b, b as u8)
                }
            };
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(request.device_address))
}

pub fn sdo_read<T>(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoReadRequest,
) -> Result<T, ChannelError>
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
    Err(ChannelError::UnknownSubdevice(request.device_address))
}

pub fn eeprom_read(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    device_address: u16,
    word_address: u16,
) -> Result<u16, ChannelError> {
    for device in group.iter(maindevice) {
        if device.configured_address() == device_address {
            let runtime = get_async_runtime();
            let res = runtime.block_on(device.eeprom_read::<u16>(maindevice, word_address));
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(device_address))
}

pub fn eeprom_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    device_address: u16,
    word_address: u16,
    value: u16,
) -> Result<(), ChannelError> {
    for device in group.iter(maindevice) {
        if device.configured_address() == device_address {
            let runtime = get_async_runtime();
            let res =
                runtime.block_on(device.eeprom_write_dangerously(maindevice, word_address, value));
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(device_address))
}

pub fn enable_dc_sync(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
) -> Result<(), ChannelError> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_dc_sync(DcSync::Sync0);
                return Ok(());
            }
        }
        return Err(ChannelError::UnknownSubdevice(device_address as u16));
    })
}

pub fn enable_dc_sync01(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
    sync1_period: Duration,
) -> Result<(), ChannelError> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_dc_sync(DcSync::Sync01 { sync1_period });
                return Ok(());
            }
        }
        Err(ChannelError::UnknownSubdevice(device_address as u16))
    })
}

pub fn configure_oversampling(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
    oversampling_settings: &[(u16, u16)],
) -> Result<(), ChannelError> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_oversampling(oversampling_settings);
                return Ok(());
            }
        }
        Err(ChannelError::UnknownSubdevice(device_address as u16))
    })
}
