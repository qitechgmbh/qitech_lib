use crate::{
    ChannelError, MAX_SUBDEVICES, PDI_LEN, SdoReadRequest, SdoRequest, SdoType, get_async_runtime,
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
