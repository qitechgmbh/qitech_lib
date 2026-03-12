use std::any::TypeId;
use ethercrab::{EtherCrabWireWrite, MainDevice, SubDeviceGroup};
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadChannel, EtherCATThreadResponseChannel, MAX_SUBDEVICES, PDI_LEN, SdoReadRequest, SdoRequest, SdoType, get_async_runtime};

pub trait EthercatSdoBytes{
    fn size(&self) -> usize;
    fn to_bytes(&self) -> [u8;4];
}

impl EthercatSdoBytes for u8 {
    fn size(&self) -> usize {
        1
    }

    fn to_bytes(&self) -> [u8;4] {
        [*self,0,0,0]
    }
}

impl EthercatSdoBytes for u16 {
    fn size(&self) -> usize {
        2
    }

    fn to_bytes(&self) -> [u8;4] {
        let bytes = u16::to_le_bytes(*self);
        [bytes[0],bytes[1],0,0]
    }
}

impl EthercatSdoBytes for i16 {
    fn size(&self) -> usize {
        2
    }

    fn to_bytes(&self) -> [u8;4] {
        let bytes = i16::to_le_bytes(*self);
        [bytes[0],bytes[1],0,0]
    }
}

impl EthercatSdoBytes for i32{
    fn size(&self) -> usize {
        4
    }

    fn to_bytes(&self) -> [u8;4] {
        i32::to_le_bytes(*self)
    }
}

impl EthercatSdoBytes for u32 {
    fn size(&self) -> usize {
        4
    }

    fn to_bytes(&self) -> [u8;4] {
        u32::to_le_bytes(*self)
    }
}


impl EthercatSdoBytes for bool {
    fn size(&self) -> usize {
        1
    }

    fn to_bytes(&self) -> [u8;4] {        
        [*self as u8,0,0,0]
    }
}

pub fn sdo_write_helper<T : 'static>(ecat_channel : EtherCATThreadChannel,device_address : u16, index : u16,sub_index : u8, value : T) -> Result<(),ethercrab::error::Error>
where T : EtherCrabWireWrite + EthercatSdoBytes
{        
    let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
    let bytes : [u8;4] = T::to_bytes(&value);
    let t_id = TypeId::of::<T>();

    // TypeId is not const compatible ... i love rust 
    // now get read for ten billion if statements, because match also doesnt work
    let sdo_type : SdoType = {
        if t_id == TypeId::of::<bool>() {
            SdoType::BOOL
        }
        else if t_id == TypeId::of::<u8>() {
            SdoType::U8
        }
        else if t_id == TypeId::of::<u16>() {
            SdoType::U16
        }
        else if t_id == TypeId::of::<u32>() {
            SdoType::U32
        }
        else if t_id == TypeId::of::<i16>() {
            SdoType::I16
        }
        else if t_id == TypeId::of::<i32>() {
            SdoType::I32
        }
        else {
            SdoType::U8  
        }        
    };
    println!("type: {:?}",sdo_type);
    let sdo_request : SdoRequest = SdoRequest { device_address, index, sub_index: sub_index as u16, data:bytes, type_flag:sdo_type};
    let req : ChannelRequest = ChannelRequest{ 
        channel_request: crate::ChannelRequests::SdoWriteRequest(sdo_request), 
        response_channel: EtherCATThreadResponseChannel(tx) 
    };
    let _res = ecat_channel.0.send(req);
    let _res = rx.recv();
   /* match res {
        Ok(_) => todo!(),
        Err(_) => todo!(),
    }*/

    Ok(())
}




/*
 Value type needs to have EtherCrabWireWrite + Copy at the least to be able to write with ethecrab
*/
pub fn sdo_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoRequest,
) -> Result<(), ethercrab::error::Error>
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();

            let res = match request.type_flag {
                SdoType::U8 =>  
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        request.data[0]
                    )),
                SdoType::U16 =>                     
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        u16::from_le_bytes([request.data[0],request.data[1]])
                    )),
                SdoType::U32 =>                     
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        u32::from_le_bytes(request.data)
                    )),
                SdoType::I16 =>                     
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        i16::from_le_bytes([request.data[0],request.data[1]])
                    )),
                SdoType::I32 =>                     
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        i32::from_le_bytes(request.data)
                    )),
                SdoType::BOOL =>  {
                    let b : bool= request.data[0] == 1;
                    println!("{} {} {} {:?}",request.device_address,request.index,request.sub_index,request.data);
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        b
                    ))
                }
                    
            };
            println!("res: {} {} {:?}",request.index,request.sub_index,res);
            return res;

        }
    }
    Err(ethercrab::error::Error::UnknownSubDevice)
}

pub fn sdo_read_signed(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoReadRequest,
) -> Result<i32, ethercrab::error::Error>
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res : Result<i32,ethercrab::error::Error> = match request.type_flag {
                SdoType::I16 => runtime.block_on(device.sdo_read::<i16>(request.index, request.sub_index as u8)).map(i32::from),
                SdoType::I32 => runtime.block_on(device.sdo_read::<i32>(request.index, request.sub_index as u8)),
                _ => unreachable!(),
            };
            return res;
        }
    }
    Err(ethercrab::error::Error::UnknownSubDevice)
}

pub fn sdo_read_unsigned(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoReadRequest,
) -> Result<u32, ethercrab::error::Error>
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res : Result<u32,ethercrab::error::Error> = match request.type_flag {
                SdoType::U8 => runtime.block_on(device.sdo_read::<u8>(request.index, request.sub_index as u8)).map(u32::from),
                SdoType::U16 => runtime.block_on(device.sdo_read::<u16>(request.index, request.sub_index as u8)).map(u32::from),
                SdoType::U32 => runtime.block_on(device.sdo_read::<u32>(request.index, request.sub_index as u8)),
                _ => unreachable!(),
            };
            return res;
        }
    }
    Err(ethercrab::error::Error::UnknownSubDevice)
}