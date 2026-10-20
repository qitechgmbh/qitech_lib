use crate::device::{Configuration, Module};
use crate::sdo::{
    EthercatResponseTypedResult, EthercatSdoBytes, RX_PDO_ASSIGNMENT_REG, SdoIndex, SdoReadRequest,
    SdoRequest, TX_PDO_ASSIGNMENT_REG,
};
use crate::{ChannelError, DiagnosticRequest, DiagnosticResponse, Error};
use crate::{ChannelRequest, ChannelResponse, EtherCATThreadResponseChannel};
use crate::{EtherCATState, EtherCATThreadChannel, al_diagnostics::SubDeviceAlStatus};
use ethercrab::EtherCrabWireWrite;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

/// Above the state machine's own snapshot deadline, so a slow bus reports results rather than
/// timing out here.
const DIAGNOSTIC_TIMEOUT: Duration = Duration::from_millis(1500);
pub const MODULE_COUNT_INDEX: (u16, u8) = (0xf050, 0x00);

#[derive(Clone, Debug)]
pub struct ModulePdoMapping {
    pub offset: usize,
    pub sub_index: usize,
    pub size: usize,
    pub module_i: u32,
}
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

    /// Call after all modules have been added
    pub fn init_slot_modules(
        &self,
        modules: &mut Vec<Module>,
        device_address: u16,
    ) -> Result<(), qitech_lib_ethercat_hal::Error> {
        let tx_pdo_mappings = self.get_module_pdo_offsets(device_address, true)?;
        let rx_pdo_mappings = self.get_module_pdo_offsets(device_address, false)?;
        for m in modules.iter_mut() {
            m.tx_size = 0;
            for pdo_map in tx_pdo_mappings.iter() {
                m.tx_size += pdo_map.size;
            }

            m.rx_size = 0;
            for pdo_map in rx_pdo_mappings.iter() {
                m.rx_size += pdo_map.size;
            }

            let tx_pdo_mapping = tx_pdo_mappings
                .iter()
                .find(|map| map.module_i == m.slot.into() && map.sub_index == 0x1);

            m.tx_offset = match tx_pdo_mapping {
                Some(map) => map.offset,
                None => 0,
            };

            let rx_pdo_mapping = rx_pdo_mappings
                .iter()
                .find(|map| map.module_i == m.slot.into() && map.sub_index == 0x1);

            m.rx_offset = match rx_pdo_mapping {
                Some(map) => map.offset,
                None => 0,
            }
        }
        Ok(())
    }

    pub fn get_module_count(
        &self,
        device_address: u16,
    ) -> Result<usize, qitech_lib_ethercat_hal::Error> {
        match self.sdo_read::<u8>(device_address, MODULE_COUNT_INDEX.0, MODULE_COUNT_INDEX.1) {
            Ok(value) => Ok(value as usize),
            Err(e) => Err(e.into()),
        }
    }

    pub fn calculate_module_index(pdo_mapping: u32, is_tx: bool) -> u32 {
        let start_index = match is_tx {
            true => 0x6000 as u32,
            false => 0x7000 as u32,
        };
        let index_in_hex = ((pdo_mapping & 0xFFFF0000) >> 16) - start_index;
        if index_in_hex < 16 {
            return 0;
        } else {
            return index_in_hex / 16;
        }
    }

    pub fn get_module_pdo_offsets(
        &self,
        device_address: u16,
        get_tx: bool,
    ) -> Result<Vec<ModulePdoMapping>, qitech_lib_ethercat_hal::Error> {
        let mut vec: Vec<ModulePdoMapping> = vec![];
        let mut bit_offset = 0;
        let start_subindex = 0x2;
        let index = match get_tx {
            true => (TX_PDO_ASSIGNMENT_REG, 0x00),
            false => (RX_PDO_ASSIGNMENT_REG, 0x00),
        };

        let start_pdo_index = match get_tx {
            true => 0x1a00 as u16,
            false => 0x1600 as u16,
        };

        let start_index = match get_tx {
            true => 0x6000 as u16,
            false => 0x7000 as u16,
        };

        let count_mappings = self.sdo_read::<u8>(device_address, index.0, index.1)?;
        let pdo_index = self.sdo_read::<u16>(device_address, index.0, 1)?;
        let pdo_map_count = self.sdo_read::<u8>(device_address, pdo_index, 0)?;

        for i in 0..pdo_map_count {
            let pdo_mapping: u32 = self.sdo_read(device_address, pdo_index, 1 + i)?;
            let bit_length = (pdo_mapping & 0xFF) as u8;
            bit_offset += bit_length as usize;
        }

        let mut mappings_without_coupler: Vec<(u16, u32)> = vec![];

        for i in start_subindex..=count_mappings {
            let pdo_index = self.sdo_read(device_address, index.0, i)?;
            let pdo_map_count = self.sdo_read::<u8>(device_address, pdo_index, 0)?;

            for j in 0..pdo_map_count {
                let pdo_mapping: u32 = self.sdo_read(device_address, pdo_index, 1 + j)?;
                mappings_without_coupler.push((pdo_index, pdo_mapping));
            }
        }

        for (pdo_index, pdo_mapping) in mappings_without_coupler {
            // if distance is < 256 then its a normal mapping
            // Otherwise its a Gap
            if (pdo_index - start_pdo_index) < 256 && pdo_mapping > start_index as u32 {
                let module_i = Self::calculate_module_index(pdo_mapping, get_tx);
                let bit_length = (pdo_mapping & 0xFF) as usize;
                let sub_index = ((pdo_mapping & 0x0000FF00) >> 8) as usize;
                if module_i < 64 {
                    vec.push(ModulePdoMapping {
                        offset: bit_offset,
                        sub_index,
                        size: bit_length,
                        module_i,
                    });
                }
                bit_offset += bit_length;
            } else {
                bit_offset += pdo_mapping as usize;
            };
        }
        Ok(vec)
    }

    /// vendor_id is set on all modules
    /// device_address should be used to address the coupler that has the modules in its pdu
    /// Not sure if this generically applies to all ethercat couplers that treats their terminals as modules
    /// If you supply a ProductIdModuleMap then the call will fail if any module is found that is not mapped!
    /// If you dont supply one then the name will stay empty
    pub fn get_modules(
        &self,
        device_address: u16,
        vendor_id: u32,
        product_map: Option<&ProductIdModuleNameMap>,
    ) -> Result<Vec<Module>, qitech_lib_ethercat_hal::Error> {
        const MODULES_START_ADDR: u16 = 0x9000;
        const MODULE_IDENT_SUBINDEX: u8 = 0x0a;
        let mut modules: Vec<Module> = vec![];
        let module_count = self.get_module_count(device_address)?;

        for i in 0..module_count {
            let module_addr = MODULES_START_ADDR + (i * 0x10) as u16;
            let ident_iom =
                self.sdo_read::<u32>(device_address, module_addr, MODULE_IDENT_SUBINDEX)?;
            // For Wago the IOM well be the product ID
            let mut module = Module {
                slot: i as u16,
                belongs_to_addr: device_address,
                vendor_id,
                product_id: ident_iom,
                has_tx: false,
                has_rx: false,
                tx_offset: 0,
                rx_offset: 0,
                name: "".to_string(),
                tx_size: 0,
                rx_size: 0,
            };

            match product_map {
                Some(map) => {
                    let name = map.map.get(&ident_iom).unwrap().clone();
                    module.name = name;
                }
                None => (),
            };

            /* match ident_iom {
                // Add Module idents here:
                WAGO_750_1506_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = true;
                    module.name = "750-1506".to_string();
                }
                WAGO_750_455_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = false;
                    module.name = "750-455".to_string();
                }
                WAGO_750_501_PRODUCT_ID => {
                    module.has_tx = false;
                    module.has_rx = true;
                    module.name = "750-501".to_string();
                }
                WAGO_750_530_PRODUCT_ID => {
                    module.has_tx = false;
                    module.has_rx = true;
                    module.name = "750-530".to_string();
                }
                WAGO_750_531_PRODUCT_ID => {
                    module.has_tx = false;
                    module.has_rx = true;
                    module.name = "750-531".to_string();
                }
                WAGO_750_652_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = true;
                    module.name = "750-652".to_string();
                }
                WAGO_750_554_PRODUCT_ID => {
                    module.has_tx = false;
                    module.has_rx = true;
                    module.name = "750-554".to_string();
                }
                WAGO_750_402_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = false;
                    module.name = "750-402".to_string();
                }
                WAGO_750_430_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = false;
                    module.name = "750-430".to_string();
                }
                WAGO_750_671_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = true;
                    module.name = "750-671".to_string();
                }
                WAGO_750_672_PRODUCT_ID => {
                    module.has_tx = true;
                    module.has_rx = true;
                    module.name = "750-672".to_string();
                }
                _ => tracing::warn!("found unknown/unimplemented module: {ident_iom}"),
            }*/

            modules.push(module);
        }
        self.init_slot_modules(&mut modules, device_address)?;
        Ok(modules)
    }
}

pub struct ProductIdModuleNameMap {
    pub map: HashMap<u32, String>,
}
