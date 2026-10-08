use crate::{ChannelError, DiagnosticRequest, DiagnosticResponse};
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadResponseChannel};
use crate::{
    EtherCATState, EtherCATThreadChannel, SdoReadRequest, SdoRequest,
    SdoType, al_diagnostics::SubDeviceAlStatus, machine_ident_read::MachineDeviceInfo,
};
use ethercrab::EtherCrabWireWrite;
use std::any::TypeId;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

/// Above the state machine's own snapshot deadline, so a slow bus reports results rather than
/// timing out here.
const DIAGNOSTIC_TIMEOUT: Duration = Duration::from_millis(1500);

/// Wait for the master thread's answer to a request.
fn recv<T>(rx: &Receiver<T>, timeout: Duration) -> Result<T, ChannelError> {
    rx.recv_timeout(timeout).map_err(|e| match e {
        RecvTimeoutError::Timeout => ChannelError::Timeout(timeout),
        RecvTimeoutError::Disconnected => ChannelError::Disconnected,
    })
}

pub trait EthercatResponseTypedResult: Sized {
    fn from_bool(_v: bool) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "bool",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_u8(_v: u8) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "u8",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_u16(_v: u16) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "u16",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_i16(_v: i16) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "i16",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_u32(_v: u32) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "u32",
            std::any::type_name::<Self>(),
        ))
    }
    fn from_i32(_v: i32) -> Result<Self, ChannelError> {
        Err(ChannelError::SdoTypeMismatch(
            "i32",
            std::any::type_name::<Self>(),
        ))
    }
}

macro_rules! impl_ethercat_typed_result {
    ($t:ty, $func:ident) => {
        impl EthercatResponseTypedResult for $t {
            fn $func(v: $t) -> Result<Self, ChannelError> {
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

impl EtherCATThreadChannel {
    pub fn sdo_read<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
    ) -> Result<T, ChannelError>
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

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;
        let response = recv(&rx, Duration::from_millis(500))?;

        let res: Result<T, ChannelError> = match response {
            ChannelResponse::SdoResponseBool(r) => T::from_bool(r?),
            ChannelResponse::SdoResponseU8(r) => T::from_u8(r?),
            ChannelResponse::SdoResponseU16(r) => T::from_u16(r?),
            ChannelResponse::SdoResponseU32(r) => T::from_u32(r?),
            ChannelResponse::SdoResponseI16(r) => T::from_i16(r?),
            ChannelResponse::SdoResponseI32(r) => T::from_i32(r?),
            _ => Err(ChannelError::UnexpectedResponse),
        };
        return res;
    }

    /// Read a raw ESC register from a subdevice, e.g.
    /// `register_read(addr, RegisterAddress::AlStatusCode)`.
    ///
    /// Serviced in every master state, including `Op`.
    pub fn register_read(
        &self,
        device_address: u16,
        register: impl Into<u16>,
    ) -> Result<u16, ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<DiagnosticResponse>();

        self.1
            .send(DiagnosticRequest::RegisterRead {
                device_address,
                register: register.into(),
                response_channel: tx,
            })
            .map_err(|_| ChannelError::Disconnected)?;

        match recv(&rx, DIAGNOSTIC_TIMEOUT)? {
            DiagnosticResponse::RegisterReadResponse(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    /// Probe a running bus. For the state a *transition* left it in, use
    /// [`EtherCATAppHandle::get_last_transition_failure`](crate::EtherCATAppHandle::get_last_transition_failure)
    /// instead — that one is recorded automatically.
    pub fn al_status_snapshot(&self) -> Result<Vec<SubDeviceAlStatus>, ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<DiagnosticResponse>();

        self.1
            .send(DiagnosticRequest::AlStatusSnapshot {
                response_channel: tx,
            })
            .map_err(|_| ChannelError::Disconnected)?;

        match recv(&rx, DIAGNOSTIC_TIMEOUT)? {
            DiagnosticResponse::AlStatusSnapshotResponse(statuses) => Ok(statuses),
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn read_device_identifications(&self) -> Result<Vec<MachineDeviceInfo>, ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::ReadMachineIdent(),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        let response = recv(&rx, Duration::from_millis(5000))?;
        match response {
            ChannelResponse::MachineDeviceInfoResponse(machine_device_infos) => {
                machine_device_infos
            }
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn write_machine_device_info_eeprom(
        &self,
        info: Vec<MachineDeviceInfo>,
    ) -> Result<(), ChannelError> {
        use crate::ChannelRequests;

        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: ChannelRequests::WriteMachineIdent(info),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;
        let response = recv(&rx, Duration::from_millis(5000))?;
        match response {
            ChannelResponse::WriteMachineInfoResponse(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn sdo_write<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
        value: T,
    ) -> Result<(), ChannelError>
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

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        let response = recv(&rx, Duration::from_millis(500))?;
        match response {
            ChannelResponse::SdoWriteResponse(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn request_state_change(&self, state: EtherCATState) -> Result<(), ChannelError> {
        let (tx, _rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::ChangeState(state),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        let _res = self.0.send(req);
        Ok(())
    }

    pub fn enable_dc_sync0(&self, device_address: u16) -> Result<(), ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::EnableDCSync0(device_address.into()),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        let response = recv(&rx, Duration::from_millis(500))?;

        match response {
            ChannelResponse::EnableDCSync0Response(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn enable_dc_sync01(
        &self,
        device_address: u16,
        sync1_period: Duration,
    ) -> Result<(), ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::EnableDCSync01(
                device_address.into(),
                sync1_period,
            ),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        let response = recv(&rx, Duration::from_millis(500))?;

        match response {
            ChannelResponse::EnableDCSync01Response(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn configure_oversampling(
        &self,
        device_address: u16,
        oversampling_settings: Vec<(u16, u16)>,
    ) -> Result<(), ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req = ChannelRequest {
            channel_request: crate::ChannelRequests::ConfigureOversampling(
                device_address.into(),
                oversampling_settings,
            ),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;
        let response = recv(&rx, Duration::from_millis(500))?;
        match response {
            ChannelResponse::ConfigureOversamplingResponse(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    pub fn set_mut_beckhoff_eeprom_lock_active(
        &self,
        device_address: u16,
    ) -> Result<(), ChannelError> {
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

pub fn type_id_to_sdo_type<T: 'static>() -> Result<SdoType, ChannelError> {
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