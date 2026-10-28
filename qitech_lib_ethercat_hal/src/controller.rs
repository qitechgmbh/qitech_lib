use crate::ChannelError;
use crate::ChannelRequest;
use crate::Consumer;
use crate::EtherCATController;
use crate::EtherCATThreadResponseChannel;
use crate::EthercatErr;
use crate::EthercatMaster;
use crate::MAX_SUBDEVICES;
use crate::Mailbox;
use crate::Producer;
use crate::TripleBufProducer;
use crate::al_diagnostics::{EtherCATTransition, TransitionReport};
use crate::sdo::SdoRequest;
use crate::{
    ChannelRequests, ChannelResponse, DiagnosticRequest, DiagnosticResponse, EtherCATState,
    sdo::SdoType, send_response,
};

use std::any::Any;
use std::fmt;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};

/// How long to wait between steps while no process data is exchanged.
const IDLE_POLL: Duration = Duration::from_millis(1);

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
    /// Runs the state machine on `master` until it fails, sleeping between steps for as long as
    /// [`step`](Self::step) says. For realtime timing, call `step` from your own loop instead.
    pub fn ethercat_state_machine<M: EthercatMaster>(
        &mut self,
        master: &mut M,
    ) -> Result<(), EthercatErr> {
        // Catch panics here instead of installing a global panic hook, which would replace the
        // application's own. The controller is not touched again after a panic, so it is fine
        // that its state may be inconsistent.
        let result = match catch_unwind(AssertUnwindSafe(|| {
            loop {
                let wait = self.step(master)?;
                std::thread::sleep(wait);
            }
        })) {
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

    /// Advances the state machine by one step, in OP by one cycle. Returns how long to wait
    /// before the next step: the master's cycle wait once process data is exchanged.
    pub fn step<M: EthercatMaster>(&mut self, master: &mut M) -> Result<Duration, EthercatErr> {
        self.service_diagnostic_request(master);

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
            EtherCATState::Init => self.handle_init(master),
            EtherCATState::PreOp => self.handle_preop(master)?,
            EtherCATState::PreopPdi => return self.handle_preop_pdi(master),
            EtherCATState::Op => return self.handle_op(master),
        }
        Ok(IDLE_POLL)
    }

    fn handle_init<M: EthercatMaster>(&mut self, master: &mut M) {
        let Ok(msg) = self.rx_channel.try_recv() else {
            return;
        };

        match msg.channel_request {
            ChannelRequests::ChangeState(EtherCATState::PreOp) => (),
            ChannelRequests::ChangeState(to) => {
                let err = EthercatErr::InvalidStateChange {
                    from: EtherCATState::Init,
                    to,
                };
                send_response(msg.response_channel, ChannelResponse::ChangeState(Err(err)));
                return;
            }
            // Shutdown is not implemented yet, the rest needs PreOp
            _ => return,
        }

        let Some(interface) = self.interface.clone() else {
            return;
        };
        if let Err(err) = self.transition(EtherCATTransition::InitGroup, master, |m| {
            m.init(&interface)
        }) {
            send_response(msg.response_channel, ChannelResponse::ChangeState(Err(err)));
            return;
        }

        self.publish_subdevices(master);
        self.state.store(EtherCATState::PreOp.into(), Relaxed);
        send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
    }

    fn handle_preop<M: EthercatMaster>(&mut self, master: &mut M) -> Result<(), EthercatErr> {
        let Ok(msg) = self.rx_channel.try_recv() else {
            return Ok(());
        };
        let Some(op_request) = handle_channel_requests(master, msg) else {
            return Ok(());
        };
        self.pending_state_change = Some(op_request);

        let cycle_time = Duration::from_micros(self.current_config.target_cycle_time_us as u64);
        self.transition(EtherCATTransition::PreOpToPreOpPdi, master, |m| {
            m.into_pre_op_pdi(cycle_time)
        })?;

        // A bad DC config shows up as InvalidDcSyncConfiguration (0x0030).
        let dc_config = self.current_config.dc_config.clone();
        self.transition(EtherCATTransition::ConfigureDcSync, master, |m| {
            m.configure_dc_sync(&dc_config)
        })?;

        self.state.store(EtherCATState::PreopPdi.into(), Relaxed);
        Ok(())
    }

    fn handle_preop_pdi<M: EthercatMaster>(
        &mut self,
        master: &mut M,
    ) -> Result<Duration, EthercatErr> {
        // Needs to run atleast once
        self.guard(
            EtherCATTransition::TxRx(EtherCATState::PreopPdi),
            master,
            |m| m.tx_rx(),
        )?;
        self.transition(EtherCATTransition::PreOpPdiToSafeOp, master, |m| {
            m.into_safe_op()
        })?;
        let res = self.guard(
            EtherCATTransition::TxRx(EtherCATState::PreopPdi),
            master,
            |m| m.tx_rx(),
        )?;

        // The process image ranges are known from SafeOp on
        self.publish_subdevices(master);

        self.transition(EtherCATTransition::SafeOpToOpRequest, master, |m| {
            m.request_into_op()
        })?;
        self.op_ramp_started = Instant::now();
        self.op_ramp_cycles = 0;
        self.state.store(EtherCATState::Op.into(), Relaxed);
        Ok(res.next_cycle_wait)
    }

    fn handle_op<M: EthercatMaster>(&mut self, master: &mut M) -> Result<Duration, EthercatErr> {
        let all_op = self.all_subdevices_operational.load(Relaxed);
        self.inputs_ready.store(false, Relaxed);

        // The outputs the client wrote since the last cycle go out with this one
        if all_op {
            if let Some(outputs) = self.output_consumer.read() {
                master.write_outputs(outputs);
                self.output_consumer.finish_read();
            }
        }

        let res = self.guard(EtherCATTransition::TxRx(EtherCATState::Op), master, |m| {
            m.tx_rx()
        })?;
        self.dc_system_time_ns.store(res.dc_system_time, Relaxed);

        if !all_op {
            if !res.all_op {
                self.op_ramp_cycles += 1;
                if self.op_ramp_cycles >= self.current_config.op_ramp_grace_cycles {
                    let message = format!(
                        "not all subdevices reached OP within {} cycles",
                        self.op_ramp_cycles
                    );
                    self.record(
                        EtherCATTransition::OpRamp,
                        master,
                        self.op_ramp_started,
                        Err::<(), _>(message),
                    )?;
                }
                return Ok(res.next_cycle_wait);
            }

            let count = self.subdevice_count.load(Relaxed) as usize;
            let mut subdevice_guard = self.subdevices.blocking_lock();
            for subdevice in &mut subdevice_guard[..count] {
                subdevice.initialized = true;
            }
            drop(subdevice_guard);
            self.all_subdevices_operational.store(true, Relaxed);
            if let Some(response_channel) = self.pending_state_change.take() {
                send_response(response_channel, ChannelResponse::ChangeState(Ok(())));
            }
        }

        if let Some(buffer) = self.input_producer.input_buffer_mut() {
            master.read_inputs(buffer);
            self.input_producer.publish();
        }
        self.inputs_ready.store(true, Relaxed);
        self.cycle.fetch_add(1, Relaxed);
        Ok(res.next_cycle_wait)
    }

    /// Copies the master's subdevices into the list the client reads.
    fn publish_subdevices<M: EthercatMaster>(&self, master: &M) {
        let subdevices = master.get_subdevices();
        let count = subdevices.len().min(MAX_SUBDEVICES);
        let mut subdevice_guard = self.subdevices.blocking_lock();
        subdevice_guard[..count].copy_from_slice(&subdevices[..count]);
        drop(subdevice_guard);
        self.subdevice_count.store(count as u64, Relaxed);
    }

    /// File a [`TransitionReport`] with an AL snapshot taken right after `result` was produced.
    /// On failure the returned error carries the rendered snapshot.
    fn record<M: EthercatMaster, T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        master: &mut M,
        started: Instant,
        result: Result<T, E>,
    ) -> Result<T, EthercatErr> {
        let error = result.as_ref().err().map(|e| format!("{e:?}"));
        let report = TransitionReport {
            transition,
            succeeded: error.is_none(),
            error,
            duration: started.elapsed(),
            statuses: master.al_status_snapshot(),
        };

        if report.succeeded {
            tracing::debug!("{report}");
        } else {
            tracing::error!("{report}");
        }

        let message = report.to_string();
        self.transition_log.push(report);

        result.map_err(|_| EthercatErr::Custom(message))
    }

    /// Run a one-shot state transition, recording it either way. Recording successes gives the
    /// next failure its "before" state for free, and shows how far startup got.
    fn transition<M: EthercatMaster, T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        master: &mut M,
        op: impl FnOnce(&mut M) -> Result<T, E>,
    ) -> Result<T, EthercatErr> {
        let started = Instant::now();
        let result = op(master);
        self.record(transition, master, started, result)
    }

    /// Run a per-cycle operation, recording it only if it fails. Snapshotting every success
    /// would put two extra PDUs per subdevice on the wire every cycle.
    fn guard<M: EthercatMaster, T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        master: &mut M,
        op: impl FnOnce(&mut M) -> Result<T, E>,
    ) -> Result<T, EthercatErr> {
        let started = Instant::now();
        match op(master) {
            Ok(value) => Ok(value),
            Err(e) => self.record(transition, master, started, Err::<T, E>(e)),
        }
    }

    /// Answer at most one pending diagnostic read. Called in every state; the one-per-step cap
    /// keeps a busy client from starving the OP loop.
    fn service_diagnostic_request<M: EthercatMaster>(&self, master: &mut M) {
        let Ok(request) = self.diagnostic_channel.try_recv() else {
            return;
        };

        match request {
            DiagnosticRequest::RegisterRead {
                device_address,
                register,
                response_channel,
            } => {
                let result = master.register_read(device_address, register);
                let _ = response_channel.send(DiagnosticResponse::RegisterReadResponse(result));
            }
            DiagnosticRequest::AlStatusSnapshot { response_channel } => {
                let statuses = master.al_status_snapshot();
                let _ =
                    response_channel.send(DiagnosticResponse::AlStatusSnapshotResponse(statuses));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::panic_message;
    use crate::mock::MockMaster;
    use crate::{EtherCATState, MetaSubdevice, init_ethercat_with};
    use std::panic::catch_unwind;
    use std::time::{Duration, Instant};

    const TIMEOUT: Duration = Duration::from_secs(1);

    #[test]
    fn state_machine_runs_on_mock_master() {
        let mut master = MockMaster::new();
        let input = master.add_subdevice(MetaSubdevice {
            end_tx: 1,
            ..Default::default()
        });
        master.add_subdevice(MetaSubdevice {
            end_rx: 1,
            ..Default::default()
        });
        master.subdevice_inputs_mut(input).unwrap()[0] = 0xAB;

        let mut control = init_ethercat_with(master, "mock0", None);
        assert!(control.change_state(EtherCATState::Op, TIMEOUT).is_err());
        control.change_state(EtherCATState::PreOp, TIMEOUT).unwrap();
        assert_eq!(control.get_subdevice_count(), 2);
        control.sdo_write(input, 0x8000, 1, 7u16).unwrap();
        assert_eq!(control.sdo_read::<u16>(input, 0x8000, 1).unwrap(), 7);

        control.change_state(EtherCATState::Op, TIMEOUT).unwrap();
        assert_eq!(control.get_state(), EtherCATState::Op);
        assert!(control.check_all_op());
        assert!(control.get_last_transition_failure().is_none());

        let started = Instant::now();
        while control.get_inputs().map(|inputs| inputs[0]) != Some(0xAB) {
            assert!(started.elapsed() < TIMEOUT, "inputs never arrived");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn panic_message_reads_str_and_string_payloads() {
        let payload = catch_unwind(|| panic!("static message")).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "static message");

        let payload = catch_unwind(|| panic!("formatted {}", 42)).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "formatted 42");

        let payload = catch_unwind(|| std::panic::panic_any(7u32)).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "non-string panic payload");
    }
}
