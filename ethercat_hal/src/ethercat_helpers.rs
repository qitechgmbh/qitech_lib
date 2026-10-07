#[cfg(not(feature = "mock"))]
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadResponseChannel};
#[cfg(not(feature = "mock"))]
use crate::{DiagnosticRequest, DiagnosticResponse};
use crate::{
    EtherCATState, EtherCATThreadChannel, MAX_SUBDEVICES, PDI_LEN, SdoReadRequest, SdoRequest,
    al_diagnostics::SubDeviceAlStatus, get_async_runtime, machine_ident_read::MachineDeviceInfo,
};
use common::SdoType;
#[cfg(not(feature = "mock"))]
use common::{
    Configuration, EthercatResponseTypedResult, EthercatSdoBytes, SdoIndex, type_id_to_sdo_type,
};
use ethercrab::{
    DcSync, EtherCrabWireRead, EtherCrabWireSized, EtherCrabWireWrite, MainDevice, SubDeviceGroup,
};
#[cfg(not(feature = "mock"))]
use std::time::Duration;

/// Above the state machine's own snapshot deadline, so a slow bus reports results rather than
/// timing out here.
#[cfg(not(feature = "mock"))]
const DIAGNOSTIC_TIMEOUT: Duration = Duration::from_millis(1500);

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

    pub fn register_read(
        &self,
        _device_address: u16,
        _register: impl Into<u16>,
    ) -> Result<u16, anyhow::Error> {
        Err(anyhow::anyhow!(
            "register_read is not supported in mock mode"
        ))
    }

    pub fn al_status_snapshot(&self) -> Result<Vec<SubDeviceAlStatus>, anyhow::Error> {
        Err(anyhow::anyhow!(
            "al_status_snapshot is not supported in mock mode"
        ))
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

    /// Read a raw ESC register from a subdevice, e.g.
    /// `register_read(addr, RegisterAddress::AlStatusCode)`.
    ///
    /// Serviced in every master state, including `Op`.
    pub fn register_read(
        &self,
        device_address: u16,
        register: impl Into<u16>,
    ) -> Result<u16, anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<DiagnosticResponse>();

        if let Err(e) = self.1.send(DiagnosticRequest::RegisterRead {
            device_address,
            register: register.into(),
            response_channel: tx,
        }) {
            return Err(anyhow::anyhow!(e));
        }

        match rx.recv_timeout(DIAGNOSTIC_TIMEOUT) {
            Ok(DiagnosticResponse::RegisterReadResponse(result)) => result,
            Ok(_) => Err(anyhow::anyhow!("Unexpected DiagnosticResponse")),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    /// Probe a running bus. For the state a *transition* left it in, use
    /// [`EtherCATAppHandle::get_last_transition_failure`](crate::EtherCATAppHandle::get_last_transition_failure)
    /// instead — that one is recorded automatically.
    pub fn al_status_snapshot(&self) -> Result<Vec<SubDeviceAlStatus>, anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<DiagnosticResponse>();

        if let Err(e) = self.1.send(DiagnosticRequest::AlStatusSnapshot {
            response_channel: tx,
        }) {
            return Err(anyhow::anyhow!(e));
        }

        match rx.recv_timeout(DIAGNOSTIC_TIMEOUT) {
            Ok(DiagnosticResponse::AlStatusSnapshotResponse(statuses)) => Ok(statuses),
            Ok(_) => Err(anyhow::anyhow!("Unexpected DiagnosticResponse")),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
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

    pub fn bulk_sdo_write(
        &self,
        writes: Vec<SdoRequest>,
        timeout: Duration,
    ) -> Result<Vec<(SdoIndex, Option<anyhow::Error>)>, anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::BulkSdoWrite(writes),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        let res = self.0.send(req);
        match res {
            Ok(_) => (),
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let res = rx.recv_timeout(timeout);
        let response: ChannelResponse = match res {
            Ok(res) => res,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        match response {
            ChannelResponse::BulkSdoWriteResponse(result) => Ok(result),
            _ => Err(anyhow::anyhow!("Unexpected ChannelResponse")),
        }
    }

    /// Writes the device specific CoE parameters and the PDO assignment of `config`
    /// in one [`Self::bulk_sdo_write`]. Has to be called in PreOp.
    pub fn write_configuration<C: Configuration>(
        &self,
        device_address: u16,
        config: &C,
        timeout: Duration,
    ) -> Result<Vec<(SdoIndex, Option<anyhow::Error>)>, anyhow::Error> {
        let mut writes = config.get_config_coe_writes(device_address)?;
        writes.extend(config.get_sm_coe_writes(device_address)?);
        self.bulk_sdo_write(writes, timeout)
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
            data: bytes,
            type_flag: sdo_type,
            sdo_index: SdoIndex {
                index: index as u32,
                sub_index: sub_index as u16,
            },
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

/*
 Value type needs to have EtherCrabWireWriteSized at the least to be able to write with ethecrab
*/
pub fn sdo_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoRequest,
) -> Result<(), anyhow::Error> {
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();

            let res = match request.type_flag {
                SdoType::U8 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    request.data[0],
                )),
                SdoType::U16 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    u16::from_le_bytes([request.data[0], request.data[1]]),
                )),
                SdoType::U32 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    u32::from_le_bytes(request.data),
                )),
                SdoType::I16 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    i16::from_le_bytes([request.data[0], request.data[1]]),
                )),
                SdoType::I32 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    i32::from_le_bytes(request.data),
                )),
                SdoType::BOOL => {
                    let b: bool = request.data[0] == 1;
                    runtime.block_on(device.sdo_write(
                        request.sdo_index.index as u16,
                        request.sdo_index.sub_index as u8,
                        b,
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
