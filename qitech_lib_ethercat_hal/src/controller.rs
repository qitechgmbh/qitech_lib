use crate::ChannelError;
use crate::ChannelRequest;
use crate::EtherCATController;
use crate::EtherCATThreadResponseChannel;
use crate::EthercatErr;
use crate::EthercatMaster;
use crate::Mailbox;
use crate::TripleBufProducer;
use crate::sdo::SdoRequest;
use crate::{
    ChannelRequests, ChannelResponse, EtherCATState, get_async_runtime, sdo::SdoType, send_response,
};

use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

/// Small func so code doesnt repeat too much
fn sdo_write<T>(master: &mut T, request: SdoRequest) -> Result<(), ChannelError>
where
    T: EthercatMaster,
{
    match request.type_flag {
        SdoType::BOOL => {
            let b: bool = request.data[0] == 1;
            master.sdo_write::<bool>(
                request.device_address,
                request.sdo_index.index as u16,
                request.sdo_index.sub_index as u8,
                b,
            )
        }
        SdoType::U8 => master.sdo_write::<u8>(
            request.device_address,
            request.sdo_index.index as u16,
            request.sdo_index.sub_index as u8,
            request.data[0],
        ),
        SdoType::U16 => master.sdo_write(
            request.device_address,
            request.sdo_index.index as u16,
            request.sdo_index.sub_index as u8,
            u16::from_le_bytes([request.data[0], request.data[1]]),
        ),
        SdoType::U32 => master.sdo_write(
            request.device_address,
            request.sdo_index.index as u16,
            request.sdo_index.sub_index as u8,
            u32::from_le_bytes(request.data),
        ),
        SdoType::I16 => master.sdo_write(
            request.device_address,
            request.sdo_index.index as u16,
            request.sdo_index.sub_index as u8,
            i16::from_le_bytes([request.data[0], request.data[1]]),
        ),
        SdoType::I32 => master.sdo_write(
            request.device_address,
            request.sdo_index.index as u16,
            request.sdo_index.sub_index as u8,
            u32::from_le_bytes(request.data),
        ),
    }
}

/// Handle a request in PreOp. Returns the response channel of an accepted request to go to OP,
/// which is answered once the transition has finished.
fn handle_channel_requests<T>(
    master: &mut T,
    msg: ChannelRequest,
) -> Option<EtherCATThreadResponseChannel>
where
    T: EthercatMaster,
{
    match msg.channel_request {
        ChannelRequests::ChangeState(ether_catstate) => {
            let res = match ether_catstate {
                EtherCATState::Op => return Some(msg.response_channel),
                EtherCATState::NoInterface | EtherCATState::PreOp => Ok(()),
                to => Err(EthercatErr::InvalidStateChange {
                    from: EtherCATState::PreOp,
                    to,
                }),
            };
            send_response(msg.response_channel, ChannelResponse::ChangeState(res));
            None
        }
        // Not yet implemented
        ChannelRequests::Shutdown() => None,
        ChannelRequests::SdoWriteRequest(request) => {
            let res = sdo_write(master, request);
            send_response(msg.response_channel, ChannelResponse::SdoWriteResponse(res));
            None
        }
        ChannelRequests::SdoReadRequest(request) => {
            match request.type_flag {
                SdoType::BOOL => {
                    let res = master.sdo_read::<bool>(
                        request.device_address,
                        request.index as u16,
                        request.sub_index as u8,
                    );
                    send_response(msg.response_channel, ChannelResponse::SdoResponseBool(res));
                }
                SdoType::U8 => {
                    let res = master.sdo_read::<u8>(
                        request.device_address,
                        request.index as u16,
                        request.sub_index as u8,
                    );
                    send_response(msg.response_channel, ChannelResponse::SdoResponseU8(res));
                }
                SdoType::U16 => {
                    let res = master.sdo_read::<u16>(
                        request.device_address,
                        request.index as u16,
                        request.sub_index as u8,
                    );
                    send_response(msg.response_channel, ChannelResponse::SdoResponseU16(res));
                }
                SdoType::U32 => {
                    let res = master.sdo_read::<u32>(
                        request.device_address,
                        request.index as u16,
                        request.sub_index as u8,
                    );
                    send_response(msg.response_channel, ChannelResponse::SdoResponseU32(res));
                }
                SdoType::I16 => {
                    let res = master.sdo_read::<i16>(
                        request.device_address,
                        request.index as u16,
                        request.sub_index as u8,
                    );
                    send_response(msg.response_channel, ChannelResponse::SdoResponseI16(res));
                }
                SdoType::I32 => {
                    let res = master.sdo_read::<i32>(
                        request.device_address,
                        request.index as u16,
                        request.sub_index as u8,
                    );
                    send_response(msg.response_channel, ChannelResponse::SdoResponseI32(res));
                }
            }
            None
        }
        ChannelRequests::EepromRead {
            device_address,
            word_address,
        } => {
            let res = master.eeprom_read_u16(device_address, word_address);
            send_response(
                msg.response_channel,
                ChannelResponse::EepromReadResponse(res),
            );
            None
        }
        ChannelRequests::EepromWrite {
            device_address,
            word_address,
            value,
        } => {
            let res = master.eeprom_write_u16(device_address, word_address, value);
            send_response(
                msg.response_channel,
                ChannelResponse::EepromWriteResponse(res),
            );
            None
        }
        ChannelRequests::EnableDCSync0(device_address) => {
            let res = master.enable_dc_sync0(device_address as u16);
            send_response(
                msg.response_channel,
                ChannelResponse::EnableDCSync0Response(res),
            );
            None
        }
        ChannelRequests::EnableDCSync01(device_address, sync1_period) => {
            let res = master.enable_dc_sync01(device_address as u16, sync1_period);
            send_response(
                msg.response_channel,
                ChannelResponse::EnableDCSync01Response(res),
            );
            None
        }
        ChannelRequests::ConfigureOversampling(device_address, oversampling_settings) => {
            let res = master.configure_oversampling(device_address as u16, &oversampling_settings);
            send_response(
                msg.response_channel,
                ChannelResponse::ConfigureOversamplingResponse(res),
            );
            None
        }
        ChannelRequests::BulkSdoWrite(sdo_requests) => {
            let mut failed_writes = vec![];
            for write in sdo_requests {
                let sdo_index = write.sdo_index;
                if let Err(e) = sdo_write(master, write) {
                    failed_writes.push((sdo_index, e));
                }
            }
            send_response(
                msg.response_channel,
                ChannelResponse::BulkSdoWriteResponse(failed_writes),
            );
            None
        }
    }
}

/// The message passed to `panic!`, if it was a string.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message.to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

impl EtherCATController<Arc<Mailbox>, TripleBufProducer> {
 fn handle_preop<T: EthercatMaster>(
        &self,
        master: &mut T,    
    ) -> Result<(), EthercatErr> {
        
        let mut i = 0;
        let mut subdevice_guard = get_async_runtime().block_on(self.subdevices.lock());
        for subdevice in master.get_subdevices() {
            subdevice_guard[i] = subdevice;            
            i += 1;
        }
        drop(subdevice_guard);

        self.subdevice_count.store(i as u64, Relaxed);                
        let msg = match self.rx_channel.try_recv() {
            Ok(value) => value,
            Err(_e) => return Ok(()),
        };        
        
        let should_not_restart_loop = handle_channel_requests(master,msg);        

        self.state.store(EtherCATState::PreopPdi.into(), Relaxed);

        return Ok(());
    }

    pub fn ethercat_state_machine<T: EthercatMaster>(
        &mut self,
        master: &mut T,
    ) -> Result<(), EthercatErr> {
        // Catch panics here instead of installing a global panic hook, which would replace the
        // application's own. The controller is not touched again after a panic, so it is fine
        // that its state may be inconsistent.
        let result = match catch_unwind(AssertUnwindSafe(|| self.run_state_machine(master))) {
            Ok(result) => result,
            Err(payload) => {
                let message = panic_message(payload.as_ref());
                tracing::error!("EtherCAT state machine panicked: {message}");
                self.all_subdevices_operational.store(false, Relaxed);
                self.inputs_ready.store(false, Relaxed);
                self.state.store(EtherCATState::NoInterface.into(), Relaxed);
                Err(EthercatErr::Panicked(message))
            }
        };
        // The state machine only returns on failure, so tell a waiting state change why.
        if let (Err(err), Some(response_channel)) = (&result, self.pending_state_change.take()) {
            send_response(
                response_channel,
                ChannelResponse::ChangeState(Err(err.clone())),
            );
        }
        result
    }

    fn run_state_machine<T: EthercatMaster>(&mut self, master: &mut T) -> Result<(), EthercatErr> {
        loop {
            let state: EtherCATState = self.state.load(Relaxed).into();
            match state {
                EtherCATState::NoInterface => {
                    if self.interface.is_some() {
                        self.state.store(EtherCATState::Init.into(), Relaxed);
                    }
                }
                EtherCATState::Boot => {
                    // Do Nothing
                }
                EtherCATState::Init => {
                    let msg = match self.rx_channel.try_recv() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    match msg.channel_request {
                        ChannelRequests::ChangeState(ether_catstate) => match ether_catstate {
                            EtherCATState::PreOp => (),
                            to => {
                                let err = EthercatErr::InvalidStateChange {
                                    from: EtherCATState::Init,
                                    to,
                                };
                                send_response(
                                    msg.response_channel,
                                    ChannelResponse::ChangeState(Err(err)),
                                );
                                continue;
                            }
                        },
                        // Not implemented yet
                        ChannelRequests::Shutdown() => continue,
                        _ => continue,
                    }

                    if self.interface.is_some() {
                        let i = 0;
                        let subdevice_guard = get_async_runtime().block_on(self.subdevices.lock());
                        self.subdevice_count.store(i as u64, Relaxed);
                        drop(subdevice_guard);
                        self.state.store(EtherCATState::PreOp.into(), Relaxed);
                        send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                    }
                }
                EtherCATState::PreOp => {}
                EtherCATState::PreopPdi => {}
                EtherCATState::Op => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
