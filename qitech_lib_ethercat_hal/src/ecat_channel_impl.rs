use crate::device::Configuration;
use crate::sdo::{
    EthercatResponseTypedResult, EthercatSdoBytes, SdoIndex, SdoReadRequest, SdoRequest,
};
use crate::{ChannelError, DiagnosticRequest, DiagnosticResponse, Error};
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadResponseChannel};
use crate::{EtherCATState, EtherCATThreadChannel, al_diagnostics::SubDeviceAlStatus};
use ethercrab::EtherCrabWireWrite;
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
        let sdo_request: SdoReadRequest = SdoReadRequest {
            device_address,
            index,
            sub_index: sub_index as u16,
            type_flag: T::SDO_TYPE,
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

    /// Read one 16-bit word from a subdevice's EEPROM (SII). `word_address` counts words, not
    /// bytes. Only serviced in PreOp.
    pub fn eeprom_read(&self, device_address: u16, word_address: u16) -> Result<u16, ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::EepromRead {
                device_address,
                word_address,
            },
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        let response = recv(&rx, Duration::from_millis(1000))?;
        match response {
            ChannelResponse::EepromReadResponse(result) => result,
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    /// Execute all `writes` on the master thread in one request. Returns the writes that failed;
    /// the others are still executed.
    pub fn bulk_sdo_write(
        &self,
        writes: Vec<SdoRequest>,
        timeout: Duration,
    ) -> Result<Vec<(SdoIndex, ChannelError)>, ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::BulkSdoWrite(writes),
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        match recv(&rx, timeout)? {
            ChannelResponse::BulkSdoWriteResponse(failed) => Ok(failed),
            _ => Err(ChannelError::UnexpectedResponse),
        }
    }

    /// Write the CoE parameters and the PDO assignment of `config`. Returns the writes that
    /// failed.
    pub fn write_configuration<C: Configuration>(
        &self,
        device_address: u16,
        config: &C,
        timeout: Duration,
    ) -> Result<Vec<(SdoIndex, ChannelError)>, Error> {
        let writes = config.get_coe_writes(device_address)?;
        // Devices without CoE (e.g. simple digital terminals) have nothing to write
        if writes.is_empty() {
            return Ok(vec![]);
        }
        Ok(self.bulk_sdo_write(writes, timeout)?)
    }

    /// Write one 16-bit word to a subdevice's EEPROM (SII). `word_address` counts words, not
    /// bytes. Only serviced in PreOp.
    ///
    /// Overwriting the identity or configuration area can leave the subdevice unusable, so only
    /// write to words you know are free.
    pub fn eeprom_write(
        &self,
        device_address: u16,
        word_address: u16,
        value: u16,
    ) -> Result<(), ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::EepromWrite {
                device_address,
                word_address,
                value,
            },
            response_channel: EtherCATThreadResponseChannel(tx),
        };

        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;

        let response = recv(&rx, Duration::from_millis(1000))?;
        match response {
            ChannelResponse::EepromWriteResponse(result) => result,
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
        let sdo_request = SdoRequest::new(device_address, index, sub_index, value);

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
        self.send_state_change(state)?;
        Ok(())
    }

    /// Block until the master has finished the state change. For OP, that is once every
    /// subdevice has confirmed OP. In OP the master takes no more state requests.
    pub fn change_state(&self, state: EtherCATState, timeout: Duration) -> Result<(), Error> {
        let rx = self.send_state_change(state)?;
        match recv(&rx, timeout)? {
            ChannelResponse::ChangeState(result) => Ok(result?),
            _ => Err(ChannelError::UnexpectedResponse.into()),
        }
    }

    fn send_state_change(
        &self,
        state: EtherCATState,
    ) -> Result<Receiver<ChannelResponse>, ChannelError> {
        let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
        let req: ChannelRequest = ChannelRequest {
            channel_request: crate::ChannelRequests::ChangeState(state),
            response_channel: EtherCATThreadResponseChannel(tx),
        };
        self.0.send(req).map_err(|_| ChannelError::Disconnected)?;
        Ok(rx)
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
