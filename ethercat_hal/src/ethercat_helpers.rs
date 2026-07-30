#[cfg(not(feature = "mock"))]
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadResponseChannel};
use crate::{
    EtherCATState, EtherCATThreadChannel, SdoReadRequest, SdoRequest,
    SdoType
};
use ethercrab::{
    EtherCrabWireWrite,
};
use std::any::TypeId;
use std::time::Duration;
use crate::MachineDeviceInfo;
pub trait EthercatResponseTypedResult: Sized {
    fn from_bool(_v: bool) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from bool not supported"))
    }
    fn from_u8(_v: u8) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from u8 not supported"))
    }
    fn from_u16(_v: u16) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from u16 not supported"))
    }
    fn from_i16(_v: i16) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from i16 not supported"))
    }
    fn from_u32(_v: u32) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from u32 not supported"))
    }
    fn from_i32(_v: i32) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Conversion from i32 not supported"))
    }
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
pub trait EthercatSdoBytes {
    fn size(&self) -> usize;
    fn to_bytes(&self) -> [u8; 4];
    fn from_bytes(bytes: [u8; 4]) -> Self
    where
        Self: Sized;
}

impl EthercatSdoBytes for u8 {
    fn size(&self) -> usize {
        1
    }

    fn to_bytes(&self) -> [u8; 4] {
        [*self, 0, 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        bytes[0]
    }
}

impl EthercatSdoBytes for u16 {
    fn size(&self) -> usize {
        2
    }

    fn to_bytes(&self) -> [u8; 4] {
        let bytes = u16::to_le_bytes(*self);
        [bytes[0], bytes[1], 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl EthercatSdoBytes for i16 {
    fn size(&self) -> usize {
        2
    }

    fn to_bytes(&self) -> [u8; 4] {
        let bytes = i16::to_le_bytes(*self);
        [bytes[0], bytes[1], 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        i16::from_le_bytes([bytes[0], bytes[1]])
    }
}

impl EthercatSdoBytes for i32 {
    fn size(&self) -> usize {
        4
    }

    fn to_bytes(&self) -> [u8; 4] {
        i32::to_le_bytes(*self)
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        i32::from_le_bytes(bytes)
    }
}

impl EthercatSdoBytes for u32 {
    fn size(&self) -> usize {
        4
    }

    fn to_bytes(&self) -> [u8; 4] {
        u32::to_le_bytes(*self)
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        u32::from_le_bytes(bytes)
    }
}

impl EthercatSdoBytes for bool {
    fn size(&self) -> usize {
        1
    }

    fn to_bytes(&self) -> [u8; 4] {
        [*self as u8, 0, 0, 0]
    }

    fn from_bytes(bytes: [u8; 4]) -> Self {
        bytes[0] != 0
    }
}

#[cfg(feature = "mock")]
impl EtherCATThreadChannel {
    pub fn sdo_read<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
    ) -> Result<T, anyhow::Error>
    where
        T: EthercatSdoBytes,
    {
        use crate::SdoIndex;
        let index = SdoIndex {
            index: index as u32,
            sub_index: sub_index as u16,
        };
        let res = self.sdo_map.get(&index);

        let result = match res {
            Some(r) => r,
            None => {
                return Err(anyhow::anyhow!(
                    "Sdo Index {}:{} for device {} not found",
                    index.index,
                    index.sub_index,
                    device_address
                ));
            }
        };

        if TypeId::of::<T>() == result.type_id {
            let mut bytes: [u8; 4] = [0u8; 4];
            for i in 0..result.value.len() {
                if i > 3 {
                    break;
                }
                bytes[i] = result.value.get(i).unwrap().clone();
            }
            Ok(T::from_bytes(bytes))
        } else {
            use std::any::type_name;
            Err(anyhow::anyhow!(
                "sdo_read: Unknown TypeId {} or Invalid Size {}!!",
                type_name::<T>(),
                result.value.len()
            ))
        }
    }

    pub fn read_device_identifications(&self) -> Result<Vec<MachineDeviceInfo>, anyhow::Error> {
        Ok(self.machine_device_infos.clone())
    }

    pub fn sdo_write<T: 'static>(
        &self,
        _device_address: u16,
        _index: u16,
        _sub_index: u8,
        _value: T,
    ) -> Result<(), anyhow::Error>
    where
        T: EtherCrabWireWrite + EthercatSdoBytes,
    {
        Ok(())
    }

    pub fn request_state_change(&self, _state: EtherCATState) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn enable_dc_sync0(&self, _device_address: u16) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn enable_dc_sync01(
        &self,
        _device_address: u16,
        _sync1_period: Duration,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn configure_oversampling(
        &self,
        _device_address: u16,
        _factor: u16,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[cfg(not(feature = "mock"))]
impl EtherCATThreadChannel {
    pub fn sdo_read<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
    ) -> Result<T, anyhow::Error>
    where
        T: EthercatSdoBytes + EthercatResponseTypedResult,
    {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let sdo_type = type_id_to_sdo_type::<T>()?;
        let sdo_request: SdoReadRequest = SdoReadRequest {
            device_address,
            index,
            sub_index: sub_index as u16,
            type_flag: sdo_type,
        };
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::SdoReadRequest(sdo_request),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        match self.0.send(req) {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        let res = rx.recv_timeout(Duration::from_millis(500));
        let response: ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res: Result<T, anyhow::Error> = match response {
            ChannelResponse::SdoResponseBool(r) => T::from_bool(r?),
            ChannelResponse::SdoResponseU8(r) => T::from_u8(r?),
            ChannelResponse::SdoResponseU16(r) => T::from_u16(r?),
            ChannelResponse::SdoResponseU32(r) => T::from_u32(r?),
            ChannelResponse::SdoResponseI16(r) => T::from_i16(r?),
            ChannelResponse::SdoResponseI32(r) => T::from_i32(r?),
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        };
        return res;
    }

    pub fn read_device_identifications(&self) -> Result<Vec<MachineDeviceInfo>, anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::ReadMachineIdent(),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        let res = self.0.send(req);
        match res {
            Ok(response) => response,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res = rx.recv_timeout(Duration::from_millis(5000));
        let response: ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        match response {
            ChannelResponse::MachineDeviceInfoResponse(machine_device_infos) => {
                machine_device_infos
            }
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    pub fn write_machine_device_info_eeprom(
        &self,
        info: Vec<MachineDeviceInfo>,
    ) -> Result<(), anyhow::Error> {
        use crate::ChannelRequests;

        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: ChannelRequests::WriteMachineIdent(info),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        let res = self.0.send(req);
        match res {
            Ok(response) => response,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        let res = rx.recv_timeout(Duration::from_millis(5000));
        let response: ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        match response {
            ChannelResponse::WriteMachineInfoResponse(result) => result,
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    pub fn sdo_write<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
        value: T,
    ) -> Result<(), anyhow::Error>
    where
        T: EtherCrabWireWrite + EthercatSdoBytes,
    {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let bytes: [u8; 4] = T::to_bytes(&value);
        let sdo_type = type_id_to_sdo_type::<T>()?;

        let sdo_request: SdoRequest = SdoRequest {
            device_address,
            index,
            sub_index: sub_index as u16,
            data: bytes,
            type_flag: sdo_type,
        };

        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::SdoWriteRequest(sdo_request),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        let res = self.0.send(req);
        match res {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res = rx.recv_timeout(Duration::from_millis(500));
        let response: ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        match response {
            ChannelResponse::SdoWriteResponse(result) => result,
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    pub fn request_state_change(&self, state: EtherCATState) -> Result<(), anyhow::Error> {
        let (tx, _rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::ChangeState(state),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        let _res = self.0.send(req);
        Ok(())
    }

    pub fn enable_dc_sync0(&self, device_address: u16) -> Result<(), anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::EnableDCSync0(device_address.into()),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        let res = self.0.send(req);
        match res {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res = rx.recv_timeout(Duration::from_millis(500));
        let response: ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        match response {
            ChannelResponse::EnableDCSync0Response(result) => result,
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    pub fn enable_dc_sync01(
        &self,
        device_address: u16,
        sync1_period: Duration,
    ) -> Result<(), anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::EnableDCSync01(
                device_address.into(),
                sync1_period,
            ),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        match self.0.send(req) {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res = rx.recv_timeout(Duration::from_millis(500));
        let response = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        match response {
            ChannelResponse::EnableDCSync01Response(result) => result,
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    pub fn configure_oversampling(
        &self,
        device_address: u16,
        oversampling_settings: Vec<(u16, u16)>,
    ) -> Result<(), anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req = ChannelRequest {
            channel_request: crate::ChannelRequests::ConfigureOversampling(
                device_address.into(),
                oversampling_settings,
            ),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        match self.0.send(req) {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        let res = rx.recv_timeout(Duration::from_millis(500));
        let response = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        match response {
            ChannelResponse::ConfigureOversamplingResponse(result) => result,
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    pub fn set_mut_beckhoff_eeprom_lock_active(
        &self,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        const BECKHOFF_EEPROM_LOCK_CODEWORD: u32 = 0x12345678;
        const BECKHOFF_CODEWORD_INDEX: u16 = 0xF008;

        let code_word = match self.sdo_read::<u32>(device_address, BECKHOFF_CODEWORD_INDEX, 0) {
            Ok(code_word) => code_word,
            // This happens when the subdevice has no mailbox
            // There is NO check in ethercrab for Mailbox presence, so we just have to pray that it has one and send a request
            // If there is no Mailbox to write Coe Request to, then there is also no EEPROM writes meaning we achieved our goal in a sense
            // This is why it returns OK on an error for sdo_read
            Err(_) => return Ok(()),
        };

        let eeprom_lock_toggled = match code_word {
            BECKHOFF_EEPROM_LOCK_CODEWORD => true,
            _ => false,
        };

        if !eeprom_lock_toggled {
            self.sdo_write(
                device_address,
                BECKHOFF_CODEWORD_INDEX,
                0,
                BECKHOFF_EEPROM_LOCK_CODEWORD,
            )?;
        }

        Ok(())
    }
}

pub fn type_id_to_sdo_type<T: 'static>() -> Result<SdoType, anyhow::Error> {
    let t_id = TypeId::of::<T>();
    let sdo_type: SdoType = {
        if t_id == TypeId::of::<bool>() {
            SdoType::BOOL
        } else if t_id == TypeId::of::<u8>() {
            SdoType::U8
        } else if t_id == TypeId::of::<u16>() {
            SdoType::U16
        } else if t_id == TypeId::of::<u32>() {
            SdoType::U32
        } else if t_id == TypeId::of::<i16>() {
            SdoType::I16
        } else if t_id == TypeId::of::<i32>() {
            SdoType::I32
        } else {
            SdoType::U8
        }
    };
    return Ok(sdo_type);
}