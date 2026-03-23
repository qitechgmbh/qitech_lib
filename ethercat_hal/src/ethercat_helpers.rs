use std::{any::TypeId, time::Duration};
use ethercrab::{EtherCrabWireRead, EtherCrabWireSized, EtherCrabWireWrite, MainDevice, SubDeviceGroup};
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadChannel, EtherCATThreadResponseChannel, MAX_SUBDEVICES, PDI_LEN, SdoReadRequest, SdoRequest, SdoType, get_async_runtime};

pub trait EthercatResponseTypedResult: Sized {
    fn from_bool(_v: bool) -> anyhow::Result<Self> { Err(anyhow::anyhow!("Conversion from bool not supported")) }
    fn from_u8(_v: u8) -> anyhow::Result<Self> { Err(anyhow::anyhow!("Conversion from u8 not supported")) }
    fn from_u16(_v: u16) -> anyhow::Result<Self> { Err(anyhow::anyhow!("Conversion from u16 not supported")) }
    fn from_i16(_v: i16) -> anyhow::Result<Self> { Err(anyhow::anyhow!("Conversion from i16 not supported")) }
    fn from_u32(_v: u32) -> anyhow::Result<Self> { Err(anyhow::anyhow!("Conversion from u32 not supported")) }
    fn from_i32(_v: i32) -> anyhow::Result<Self> { Err(anyhow::anyhow!("Conversion from i32 not supported")) }
}

macro_rules! impl_ethercat_typed_result {
    ($t:ty, $func:ident) => {
        impl EthercatResponseTypedResult for $t {
            fn $func(v: $t) -> anyhow::Result<Self> {
                Ok(v)
            }
        }
    };
}

impl_ethercat_typed_result!(bool, from_bool);
impl_ethercat_typed_result!(u8, from_u8);
impl_ethercat_typed_result!(u16, from_u16);
impl_ethercat_typed_result!(i16, from_i16);
impl_ethercat_typed_result!(u32, from_u32);
impl_ethercat_typed_result!(i32, from_i32);

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


impl EtherCATThreadChannel {
    pub fn sdo_read<T : 'static>(&self,device_address : u16, index : u16,sub_index : u8) -> Result<T,anyhow::Error>
    where T : EthercatSdoBytes + EthercatResponseTypedResult
    {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let sdo_type = type_id_to_sdo_type::<T>()?;
        let sdo_request : SdoReadRequest = SdoReadRequest { device_address, index, sub_index: sub_index as u16, type_flag: sdo_type };
        let req : ChannelRequest = ChannelRequest{ 
            channel_request: crate::ChannelRequests::SdoReadRequest(sdo_request), 
            response_channel: EtherCATThreadResponseChannel(tx) 
        };
        match self.0.send(req) {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        let res = rx.recv_timeout(Duration::from_millis(500));
        let response : ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res : Result<T,anyhow::Error> = match response {
            ChannelResponse::SdoResponseBool(r) => T::from_bool(r?),
            ChannelResponse::SdoResponseU8(r)   => T::from_u8(r?),
            ChannelResponse::SdoResponseU16(r)  =>T::from_u16(r?),
            ChannelResponse::SdoResponseU32(r)  => T::from_u32(r?),
            ChannelResponse::SdoResponseI16(r)  => T::from_i16(r?),
            ChannelResponse::SdoResponseI32(r)  => T::from_i32(r?),
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        };
        return res;
    }

pub fn sdo_write<T : 'static>(&self,device_address : u16, index : u16,sub_index : u8, value : T) -> Result<(),anyhow::Error>
where T : EtherCrabWireWrite + EthercatSdoBytes
{        
    let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
    let bytes : [u8;4] = T::to_bytes(&value);
    let sdo_type = type_id_to_sdo_type::<T>()?;
    let sdo_request : SdoRequest = SdoRequest { device_address, index, sub_index: sub_index as u16, data:bytes, type_flag:sdo_type};
    let req : ChannelRequest = ChannelRequest{ 
        channel_request: crate::ChannelRequests::SdoWriteRequest(sdo_request), 
        response_channel: EtherCATThreadResponseChannel(tx) 
    };
    let res = self.0.send(req);
    match res {
        Ok(_) => (),
        Err(e) => return Err(anyhow::anyhow!(e)),
    };
    let res = rx.recv_timeout(Duration::from_millis(500));
    let response : ChannelResponse = match res {
        Ok(res) => res,
        Err(e) => return Err(anyhow::anyhow!(e)),
    };
    match response {
        ChannelResponse::SdoWriteResponse(result) => result,
        _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
    }
}}


pub fn type_id_to_sdo_type<T : 'static>() -> Result<SdoType, anyhow::Error>{
    let t_id = TypeId::of::<T>();
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
    return Ok(sdo_type);
}

/*
 Value type needs to have EtherCrabWireWriteSized at the least to be able to write with ethecrab
*/
pub fn sdo_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoRequest,
) -> Result<(), anyhow::Error>
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
                    runtime.block_on(device.sdo_write(
                        request.index,
                        request.sub_index as u8,
                        b
                    ))
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
where T : EtherCrabWireRead + EtherCrabWireSized
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res : Result<T,ethercrab::error::Error> = 
            runtime.block_on(device.sdo_read::<T>(request.index, request.sub_index as u8));
            return Ok(res?);
        }
    }
    Err(anyhow::anyhow!("Unknown Subdevice"))
}