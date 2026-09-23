use crate::MasterConfiguration;
use crate::MetaSubdevice;
use crate::RtOptimizationConfig;
use crate::TripleBufProducer;
use crate::al_diagnostics::TransitionLog;
use crate::al_diagnostics::TransitionReport;
use crate::al_diagnostics::fallback_addresses;
use crate::al_diagnostics::read_al_statuses;
use crate::ethercat_helpers::configure_oversampling;
use crate::ethercat_helpers::enable_dc_sync01;
use crate::types::ChannelRequest;
use crate::types::ChannelRequests;
use crate::types::Consumer;
use crate::types::DiagnosticRequest;
use crate::types::DiagnosticResponse;
use crate::types::EthercatErr;
use crate::types::Producer;
use crate::types::SdoType;
use crate::{
    ChannelResponse, ETHERCAT_TX_RX_SIZE, EtherCATState, MAX_SUBDEVICES,
    PDI_LEN, PDU_STORAGE,
    al_diagnostics::EtherCATTransition,
    ethercat_helpers::{enable_dc_sync, sdo_read, sdo_write},
    get_async_runtime,
    machine_ident_read::{read_device_identifications, write_device_identifications},
    send_response,
};
use crate::{Mailbox, set_current_thread_rt_priority};
use anyhow::bail;
#[cfg(target_os = "linux")]
use common::set_irq_affinity;
use ethercrab::Command;
use ethercrab::std::ethercat_now;
use ethercrab::{
    MainDevice, MainDeviceConfig, RegisterAddress, RetryBehaviour, SubDeviceGroup, Timeouts,
    subdevice_group::{DcConfiguration, HasDc, NoDc, Op, PreOpPdi},
};
use libc::{MCL_CURRENT, MCL_FUTURE, mlockall};
use spin_sleep::SpinSleeper;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::Receiver;
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};
use tokio::sync::Mutex;

type PreopGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock>;
type PreopPdiNoDcGroup =
    SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, PreOpPdi, NoDc>;
type PreopPdiDcGroup =
    SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, PreOpPdi, HasDc>;
type OpGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, Op, HasDc>;
enum PreopResult {
    Preop(PreopGroup),
    PreopPdiDc(PreopPdiDcGroup),
}

pub struct EtherCATController<C, P>
where
    C: Consumer,
    P: Producer,
{
    cycle: Arc<AtomicU64>,
    cycle_time_us: Arc<AtomicU64>,
    subdevice_count: Arc<AtomicU64>, // maybe a Mailbox<Status> or smth like that makes more sense?
    next_cycle: Instant,
    interface: Option<String>,
    subdevices: Arc<Mutex<[MetaSubdevice; MAX_SUBDEVICES]>>,
    state: Arc<AtomicU8>,
    all_subdevices_operational: Arc<AtomicBool>,
    inputs_ready: Arc<AtomicBool>,
    dc_system_time_ns: Arc<AtomicU64>,
    current_config: MasterConfiguration,
    requested_state: Option<EtherCATState>,
    rx_channel: Receiver<ChannelRequest>,
    /// Separate from `rx_channel` so it can be drained in every state without swallowing a
    /// `ChangeState`/`Shutdown` the current arm cannot honour.
    diagnostic_channel: Receiver<DiagnosticRequest>,
    transition_log: TransitionLog,
    input_producer: P,
    output_consumer: C,
}

impl<C, P> EtherCATController<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub fn new(
        input: P,
        output: C,
        rx: Receiver<ChannelRequest>,
        diagnostic_channel: Receiver<DiagnosticRequest>,
        interface: Option<String>,
        config: MasterConfiguration,
        cycle: Arc<AtomicU64>,
        cycle_time_us: Arc<AtomicU64>,
        subdevice_count: Arc<AtomicU64>,
        dc_system_time_ns: Arc<AtomicU64>,
        state: Arc<AtomicU8>,
        subdevices: Arc<Mutex<[MetaSubdevice; MAX_SUBDEVICES]>>,
        all_subdevices_operational: Arc<AtomicBool>,
        inputs_ready: Arc<AtomicBool>,
        transition_log: TransitionLog,
    ) -> Self {
        Self {
            cycle,
            cycle_time_us,
            interface,
            subdevice_count,
            next_cycle: std::time::Instant::now(),
            subdevices,
            state,
            requested_state: None,
            rx_channel: rx,
            diagnostic_channel,
            transition_log,
            input_producer: input,
            output_consumer: output,
            current_config: config,
            all_subdevices_operational,
            dc_system_time_ns,
            inputs_ready,
        }
    }
}

impl EtherCATController<Arc<Mailbox>, TripleBufProducer> {
    /// The subdevices to probe, as `(configured_address, name)`. Falls back to reconstructed
    /// addresses during `init_single_group`, the one transition that runs before enumeration.
    async fn diagnostic_devices(&self, maindevice: &MainDevice<'_>) -> Vec<(u16, String)> {
        let count = self
            .subdevice_count
            .load(std::sync::atomic::Ordering::Relaxed) as usize;
        if count == 0 {
            return fallback_addresses(maindevice);
        }

        let subdevices = self.subdevices.lock().await;
        subdevices[..count.min(MAX_SUBDEVICES)]
            .iter()
            .map(|meta| {
                let name = meta
                    .get_name()
                    .unwrap_or_else(|_| format!("{:#06x}", meta.device_address));
                (meta.device_address, name)
            })
            .collect()
    }

    /// File a [`TransitionReport`] with an AL snapshot taken right after `result` was produced.
    /// On failure the returned error carries the rendered snapshot.
    ///
    /// Takes an already-evaluated `Result` so it covers the group-consuming transitions too:
    /// by now the group may be dropped, and the snapshot needs only `maindevice`.
    pub(crate) async fn record<T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        maindevice: &MainDevice<'_>,
        started: Instant,
        result: Result<T, E>,
    ) -> Result<T, EthercatErr> {
        let error = result.as_ref().err().map(|e| format!("{e:?}"));
        let succeeded = error.is_none();
        let devices = self.diagnostic_devices(maindevice).await;

        let report = TransitionReport {
            transition,
            succeeded,
            error,
            duration: started.elapsed(),
            statuses: read_al_statuses(maindevice, &devices).await,
        };

        if succeeded {
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
    pub(crate) async fn transition<T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        maindevice: &MainDevice<'_>,
        op: impl Future<Output = Result<T, E>>,
    ) -> Result<T, EthercatErr> {
        let started = Instant::now();
        let result = op.await;
        self.record(transition, maindevice, started, result).await
    }

    /// Run a per-cycle operation, recording it only if it fails. Snapshotting every success
    /// would put two extra PDUs per subdevice on the wire every cycle.
    pub(crate) async fn guard<T, E: fmt::Debug>(
        &self,
        transition: EtherCATTransition,
        maindevice: &MainDevice<'_>,
        op: impl Future<Output = Result<T, E>>,
    ) -> Result<T, EthercatErr> {
        let started = Instant::now();
        match op.await {
            Ok(value) => Ok(value),
            Err(e) => {
                self.record(transition, maindevice, started, Err::<T, E>(e))
                    .await
            }
        }
    }

    /// Answer at most one pending diagnostic read. Called from every state arm; the one-per-
    /// iteration cap keeps a busy client from starving the OP loop.
    pub(crate) async fn service_diagnostic_request(&self, maindevice: &MainDevice<'_>) {
        let Ok(request) = self.diagnostic_channel.try_recv() else {
            return;
        };

        match request {
            DiagnosticRequest::RegisterRead {
                device_address,
                register,
                response_channel,
            } => {
                let result = Command::fprd(device_address, register)
                    .receive::<u16>(maindevice)
                    .await
                    .map_err(|e| anyhow::anyhow!("register read failed: {e:?}"));
                let _ = response_channel.send(DiagnosticResponse::RegisterReadResponse(result));
            }
            DiagnosticRequest::AlStatusSnapshot { response_channel } => {
                let devices = self.diagnostic_devices(maindevice).await;
                let statuses = read_al_statuses(maindevice, &devices).await;
                let _ =
                    response_channel.send(DiagnosticResponse::AlStatusSnapshotResponse(statuses));
            }
        }
    }
}

fn setup_tx_rx_thread(
    interface: String,
    opt: Option<RtOptimizationConfig>,
) -> Option<MainDevice<'static>> {
    let (tx, rx, pdu) = PDU_STORAGE.try_split().expect("can only split once");
    let pdu_tx = tx;
    let pdu_rx = rx;

    let _ethercat_tx_rx_handle = std::thread::Builder::new()
        .name("EthercatTxRxThread".to_owned())
        .spawn(move || {
            match opt {
                Some(opt) => {
                    let id = core_affinity::CoreId {
                        id: opt.ethercat_io_thread_core,
                    };
                    set_current_thread_rt_priority(opt.ethercat_io_thread_priority as i32);
                    // Pin to the last core (e.g., Core 3 on a 4-core system)
                    core_affinity::set_for_current(id);
                    #[cfg(target_os = "linux")]
                    if let Some(irq_core) = opt.pin_irq_core {
                        let res = set_irq_affinity(&interface, irq_core as u32);
                        if res.is_err() {
                            eprintln!("set_irq_affinity failed performance may be degraded");
                        }
                    }
                }
                None => (),
            };

            #[cfg(not(target_os = "linux"))]
            use ethercrab::std::tx_rx_task;
            #[cfg(target_os = "linux")]
            use ethercrab::std::tx_rx_task_io_uring;

            #[cfg(target_os = "linux")]
            tx_rx_task_io_uring(&interface, pdu_tx, pdu_rx)
                .expect("Failed to run TX/RX task (io_uring)");
            #[cfg(not(target_os = "linux"))]
            get_async_runtime().block_on(async {
                match tx_rx_task(&interface, pdu_tx, pdu_rx) {
                    Ok(task) => {
                        task.await.expect("TX/RX task failed");
                    }
                    Err(e) => panic!("Failed to create TX/RX task: {e}"),
                }
            });
        });

    let maindevice = Some(MainDevice::new(
        pdu,
        Timeouts {
            state_transition: Duration::from_millis(20000),
            pdu: Duration::from_micros(30_000),
            eeprom: Duration::from_millis(100),
            wait_loop_delay: Duration::from_millis(0),
            mailbox_echo: Duration::from_millis(100),
            mailbox_response: Duration::from_millis(1000),
        },
        MainDeviceConfig {
            retry_behaviour: RetryBehaviour::Count(0),
            dc_static_sync_iterations: 10_000,
        },
    ));
    return maindevice;
}

/*
    if ecat OP state
*/
fn handle_channel_requests(
    msg: ChannelRequest,
    maindev: &MainDevice<'_>,
    mut preop_group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock>,
) -> bool {
    match msg.channel_request {
        ChannelRequests::ChangeState(ether_catstate) => {
            return match ether_catstate {
                EtherCATState::NoInterface => {
                    //self.state.store(ether_catstate.into(), Relaxed);
                    send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                    false
                }
                EtherCATState::Op => true,
                _ => false,
            };
        }
        ChannelRequests::Shutdown() => true,
        ChannelRequests::SdoWriteRequest(request) => {
            let res = sdo_write(maindev, preop_group, request);
            send_response(msg.response_channel, ChannelResponse::SdoWriteResponse(res));
            false
        }
        ChannelRequests::SdoReadRequest(request) => {
            match request.type_flag {
                SdoType::BOOL => {
                    let res = sdo_read::<bool>(maindev, preop_group, request);
                    send_response(msg.response_channel, ChannelResponse::SdoResponseBool(res));
                }
                SdoType::U8 => {
                    let res = sdo_read::<u8>(maindev, preop_group, request);
                    send_response(msg.response_channel, ChannelResponse::SdoResponseU8(res));
                }
                SdoType::U16 => {
                    let res = sdo_read::<u16>(maindev, preop_group, request);
                    send_response(msg.response_channel, ChannelResponse::SdoResponseU16(res));
                }
                SdoType::U32 => {
                    let res = sdo_read::<u32>(maindev, preop_group, request);
                    send_response(msg.response_channel, ChannelResponse::SdoResponseU32(res));
                }
                SdoType::I16 => {
                    let res = sdo_read::<i16>(maindev, preop_group, request);
                    send_response(msg.response_channel, ChannelResponse::SdoResponseI16(res));
                }
                SdoType::I32 => {
                    let res = sdo_read::<i32>(maindev, preop_group, request);
                    send_response(msg.response_channel, ChannelResponse::SdoResponseI32(res));
                }
            }

            false
        }
        ChannelRequests::ReadMachineIdent() => {
            let res = read_device_identifications(preop_group, maindev);
            send_response(
                msg.response_channel,
                ChannelResponse::MachineDeviceInfoResponse(res),
            );
            false
        }
        ChannelRequests::WriteMachineIdent(identifications) => {
            let res = write_device_identifications(preop_group, maindev, &identifications);
            send_response(
                msg.response_channel,
                ChannelResponse::WriteMachineInfoResponse(res),
            );
            false
        }
        ChannelRequests::EnableDCSync0(device_address) => {
            let res = enable_dc_sync(&mut preop_group, maindev, device_address);
            send_response(
                msg.response_channel,
                ChannelResponse::EnableDCSync0Response(res),
            );
            false
        }
        ChannelRequests::EnableDCSync01(device_address, sync1_period) => {
            let res = enable_dc_sync01(&mut preop_group, maindev, device_address, sync1_period);
            send_response(
                msg.response_channel,
                ChannelResponse::EnableDCSync01Response(res),
            );
            false
        }
        ChannelRequests::ConfigureOversampling(device_address, oversampling_settings) => {
            let res = configure_oversampling(
                &mut preop_group,
                maindev,
                device_address,
                &oversampling_settings,
            );
            send_response(
                msg.response_channel,
                ChannelResponse::ConfigureOversamplingResponse(res),
            );
            false
        }
    }
}

fn dc_static_sync(
    main_device: &MainDevice<'_>,
    group_preop_pdi: PreopPdiNoDcGroup,
    cycle_time: u64,
    spinner: SpinSleeper,
) -> PreopPdiNoDcGroup {
    let rt = get_async_runtime();
    let mut now = Instant::now();
    let mut averages = Vec::new();
    for _ in 0..group_preop_pdi.len() {
        averages.push(ExponentialMovingAverage::new(64).expect("Should never fail"));
    }

    loop {
        let deadline = Instant::now() + Duration::from_micros(cycle_time);
        let _res = rt.block_on(group_preop_pdi.tx_rx_sync_system_time(main_device));
        if now.elapsed() >= Duration::from_millis(25) {
            now = Instant::now();
            let mut max_deviation = 0;
            for (s1, ema) in group_preop_pdi.iter(&main_device).zip(averages.iter_mut()) {
                let diff = match rt
                    .block_on(s1.register_read::<u32>(RegisterAddress::DcSystemTimeDifference))
                {
                    Ok(value) => {
                        let flag = 0b1u32 << 31;
                        if value >= flag {
                            // Strip off negative flag bit and negate value as normal
                            -((value & !flag) as i32)
                        } else {
                            value as i32
                        }
                    }
                    Err(ethercrab::error::Error::WorkingCounter { .. }) => 0,
                    Err(_e) => 0,
                };

                let ema_next = ema.next(diff as f64);
                max_deviation = max_deviation.max(ema_next.abs() as u32);
            }
            if max_deviation < 300 {
                break;
            }
        }
        spinner.sleep_until(deadline);
    }
    return group_preop_pdi;
}

impl EtherCATController<Arc<Mailbox>, TripleBufProducer> {
    fn apply_op_optimizations(&self) -> Result<(), anyhow::Error> {
        match &self.current_config.realtime_optimizations {
            Some(opt) => {
                let id = core_affinity::CoreId {
                    id: opt.ethercat_loop_thread_core,
                };
                set_current_thread_rt_priority(opt.ethercat_loop_thread_priority as i32);
                core_affinity::set_for_current(id);
                if opt.lock_memory {
                    let flags = MCL_CURRENT | MCL_FUTURE;
                    let result = unsafe { mlockall(flags) };
                    if result != 0 {
                        bail!("Warning: Memory locking failed! Result: {}", result,);
                    }
                }
            }
            None => (),
        };
        Ok(())
    }

    fn handle_preop(
        &self,
        group_opt: Option<PreopGroup>,
        maindevice: &MainDevice<'_>,
        spinner: SpinSleeper,
    ) -> Result<PreopResult, EthercatErr> {
        let mut i = 0;
        let mut subdevice_guard = get_async_runtime().block_on(self.subdevices.lock());
        let rt = get_async_runtime();
        let mut group = group_opt.unwrap();
        for subdevice in group.iter(maindevice) {
            let bytes = subdevice.name().as_bytes();
            let len = std::cmp::min(bytes.len(), 127);
            // Copy the slice into the array
            subdevice_guard[i].name[..len].copy_from_slice(&bytes[..len]);
            subdevice_guard[i].product_id = subdevice.identity().product_id;
            subdevice_guard[i].revision = subdevice.identity().revision;
            subdevice_guard[i].vendor = subdevice.identity().vendor_id;
            subdevice_guard[i].device_address = subdevice.configured_address();
            i += 1;
        }
        drop(subdevice_guard);
        self.subdevice_count.store(i as u64, Relaxed);
        get_async_runtime().block_on(self.service_diagnostic_request(maindevice));

        let msg = match self.rx_channel.try_recv() {
            Ok(value) => value,
            Err(_e) => return Ok(PreopResult::Preop(group)),
        };

        let should_not_restart_loop = handle_channel_requests(msg, maindevice, &mut group);

        match should_not_restart_loop {
            true => (),
            false => return Ok(PreopResult::Preop(group)),
        };

        let mut group_preop_pdi: PreopPdiNoDcGroup = rt.block_on(self.transition(
            EtherCATTransition::PreOpToPreOpPdi,
            maindevice,
            group.into_pre_op_pdi(maindevice),
        ))?;

        group_preop_pdi = dc_static_sync(
            maindevice,
            group_preop_pdi,
            self.current_config.target_cycle_time_us as u64,
            spinner,
        );

        // A bad DC config shows up as InvalidDcSyncConfiguration (0x0030).
        let group_preop_pdi_dc = rt
            .block_on(self.transition(
                EtherCATTransition::ConfigureDcSync,
                maindevice,
                group_preop_pdi.configure_dc_sync(
                    maindevice,
                    DcConfiguration {
                        start_delay: self.current_config.dc_config.start_delay,
                        sync0_period: self.current_config.dc_config.sync0_period,
                        sync0_shift: self.current_config.dc_config.sync0_shift,
                    },
                ),
            ))
            .expect("msg");
        self.state.store(EtherCATState::PreopPdi.into(), Relaxed);
        return Ok(PreopResult::PreopPdiDc(group_preop_pdi_dc));
    }

    fn handle_preop_pdi(
        &self,
        group_opt: Option<PreopPdiDcGroup>,
        maindevice: &MainDevice<'_>,
        spinner: SpinSleeper,
    ) -> Result<OpGroup, EthercatErr> {
        let rt = get_async_runtime();
        let group = group_opt.unwrap();
        // Needs to run atleast once
        let res = rt.block_on(self.guard(
            EtherCATTransition::TxRx(EtherCATState::PreopPdi),
            maindevice,
            group.tx_rx_dc(maindevice),
        ))?;
        spinner.sleep_until(Instant::now() + res.extra.next_cycle_wait);
        let group = rt.block_on(self.transition(
            EtherCATTransition::PreOpPdiToSafeOp,
            maindevice,
            group.into_safe_op(maindevice),
        ))?;

        // Apply the same logic here
        let now = Instant::now();
        let res = rt.block_on(self.guard(
            EtherCATTransition::TxRx(EtherCATState::PreopPdi),
            maindevice,
            group.tx_rx_dc(maindevice),
        ))?;

        let is_all_safe = res.is_in_state(ethercrab::SubDeviceState::SafeOp);
        if !is_all_safe {
            spinner.sleep_until(now + res.extra.next_cycle_wait);
        }

        if is_all_safe {
            let mut rx_offset = 0;
            let mut tx_offset = 0;
            let mut subdevice_guard = get_async_runtime().block_on(self.subdevices.lock());
            for (i, subdevice) in group.iter(maindevice).enumerate() {
                let length_tx = subdevice.io_raw().inputs().len();
                let length_rx = subdevice.io_raw().outputs().len();

                subdevice_guard[i].start_tx = tx_offset;
                subdevice_guard[i].end_tx = tx_offset + length_tx;

                subdevice_guard[i].start_rx = rx_offset;
                subdevice_guard[i].end_rx = rx_offset + length_rx;

                rx_offset += length_rx;
                tx_offset += length_tx;
            }
        };

        let group_op = rt.block_on(self.transition(
            EtherCATTransition::SafeOpToOpRequest,
            maindevice,
            group.request_into_op(maindevice),
        ))?;
        self.state.store(EtherCATState::Op.into(), Relaxed);
        return Ok(group_op);
    }

    async fn handle_op(
        &mut self,
        group_opt: Option<OpGroup>,
        maindevice: &MainDevice<'_>,
        spinner: SpinSleeper,
    ) -> Result<(), EthercatErr> {
        let mut is_all_op = false;
        let mut not_all_op_cycles: u32 = 0;
        let ramp_started = Instant::now();
        let cycle_time_ns = self.current_config.target_cycle_time_us as i64 * 1000;
        let mut integral: i64 = 0;
        let mut error: i64;
        let mut delta: i64;
        // TODO Make these configurable?
        let pgain = 0.01 as f64;
        let igain = 0.00002 as f64;
        // sync_offset_ns is 50% of macro cycle time(Sync1 FULL period)
        // This essentially means we send the frame 50% into the sync1 period
        let sync_offset_ns: u64 = (self.current_config.target_cycle_time_us as u64 * 1000) / 2;
        let group = group_opt.unwrap();

        loop {
            // Instant::now() on Almost any modern linux pc compiles down to one instruction if pc is not older than 2006 or so
            let cycle_start = Instant::now();
            let res = self
                .guard(
                    EtherCATTransition::TxRx(EtherCATState::Op),
                    maindevice,
                    group.tx_rx_dc(maindevice),
                )
                .await?;
            delta = (res.extra.dc_system_time - sync_offset_ns) as i64 % cycle_time_ns;
            if delta > (cycle_time_ns / 2) {
                delta = delta - cycle_time_ns
            }
            error = -delta;
            // Not sure what to clamp to, if at all Clamping seemed to have a negative effect? so just keep it as is
            integral = integral + error;
            // Maybe instead it makes sense to clamp offsettime?
            let offsettime = ((error as f64 * pgain) + (integral as f64 * igain)) as i64;
            self.dc_system_time_ns
                .store(res.extra.dc_system_time, Relaxed);
            self.next_cycle = cycle_start
                + Duration::from_nanos(
                    (cycle_time_ns + offsettime.clamp(cycle_time_ns * -1, cycle_time_ns)) as u64,
                );

            if !is_all_op {
                if res.all_op() {
                    let mut subdevice_guard = self.subdevices.lock().await;
                    for i in 0..self.subdevice_count.load(Relaxed) {
                        subdevice_guard[i as usize].initialized = true;
                    }
                    self.all_subdevices_operational.store(true, Relaxed);
                    drop(subdevice_guard);
                    not_all_op_cycles = 0;
                    is_all_op = true;
                } else {
                    spinner.sleep_until(self.next_cycle);
                    self.cycle_time_us
                        .store(cycle_start.elapsed().as_micros() as u64, Relaxed);
                    not_all_op_cycles += 1;

                    if not_all_op_cycles >= self.current_config.op_ramp_grace_cycles {
                        self.record(
                            EtherCATTransition::OpRamp,
                            maindevice,
                            ramp_started,
                            Err::<(), _>(format!(
                                "not all subdevices reached OP within \
                                                 {not_all_op_cycles} cycles"
                            )),
                        )
                        .await?;
                    }
                    continue;
                }
            }

            match self.input_producer.input_buffer_mut() {
                Some(buffer) => {
                    // We get a mutable slice to the whole buffer to make sub-slicing easier
                    let mut current_offset = 0;
                    for subdevice in group.iter(&maindevice) {
                        let len = subdevice.io_raw().inputs().len();
                        if current_offset + len <= ETHERCAT_TX_RX_SIZE {
                            buffer[current_offset..current_offset + len]
                                .copy_from_slice(subdevice.io_raw().inputs());
                            current_offset += len;
                        } else {
                            break;
                        }
                    }
                    self.input_producer.publish();
                }
                None => {}
            }
            self.inputs_ready.store(true, Relaxed);

            // Inside the window already reserved for the client side, so a
            // register read costs the cycle nothing it was going to use.
            self.service_diagnostic_request(maindevice).await;

            // This gives the client side time to look at the inputs and write outputs
            // Might be a bit too tight if running < 125us but at that point it isnt really stable anyways
            spinner.sleep_until(self.next_cycle - Duration::from_nanos(10000));
            match self.output_consumer.read() {
                Some(full_buffer) => {
                    let mut current_offset = 0;
                    for subdevice in group.iter(&maindevice) {
                        let mut output = subdevice.outputs_raw_mut();
                        let len = output.len();
                        output.copy_from_slice(&full_buffer[current_offset..current_offset + len]);
                        current_offset += len;
                    }
                    self.output_consumer.finish_read();
                }
                None => {}
            };
            spinner.sleep_until(self.next_cycle);
            self.cycle_time_us
                .store(cycle_start.elapsed().as_micros() as u64, Relaxed);
            if self.cycle.load(Relaxed) == u64::MAX {
                self.cycle.store(0, Relaxed);
            } else {
                self.cycle.fetch_add(1, Relaxed);
            }
            self.inputs_ready.store(false, Relaxed);
        }
    }

    pub fn ethercat_state_machine(&mut self) -> Result<(), anyhow::Error> {
        let mut _ethercat_tx_rx_handle: Result<JoinHandle<()>, std::io::Error>;
        let mut group: Option<SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock>> =
            None;
        let mut group_preop_pdi_dc: Option<
            SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, PreOpPdi, HasDc>,
        > = None;
        let mut group_op: Option<
            SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, Op, HasDc>,
        > = None;
        let mut maindevice: Option<MainDevice> = None;
        let spinner = SpinSleeper::new(200_000);

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
                            _ => continue,
                        },
                        ChannelRequests::Shutdown() => return Ok(()), // We CAN safely shutdonw in Init
                        _ => continue,
                    }

                    if self.interface.is_some() {
                        maindevice = setup_tx_rx_thread(
                            self.interface.clone().expect("Should be some"),
                            self.current_config.realtime_optimizations.clone(),
                        );
                        let rt = get_async_runtime();
                        let maindev = maindevice.as_ref().unwrap();

                        let res = rt.block_on(self.transition(
                            EtherCATTransition::InitGroup,
                            maindev,
                            maindev.init_single_group::<MAX_SUBDEVICES, PDI_LEN>(ethercat_now),
                        ));

                        group = Some(match res {
                            Ok(group) => group,
                            Err(err) => {
                                self.state.store(EtherCATState::Init.into(), Relaxed);
                                send_response(
                                    msg.response_channel,
                                    ChannelResponse::ChangeState(Err(err.into())),
                                );
                                continue;
                            }
                        });
                        self.state.store(EtherCATState::PreOp.into(), Relaxed);
                        send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                    }
                }
                EtherCATState::PreOp => {
                    let res = self.handle_preop(group, maindevice.as_ref().unwrap(), spinner)?;
                    match res {
                        PreopResult::Preop(preop_group) => group = Some(preop_group),
                        PreopResult::PreopPdiDc(preop_pdi_dc) => {
                            group = None;
                            group_preop_pdi_dc = Some(preop_pdi_dc);
                        }
                    }
                }
                EtherCATState::PreopPdi => {
                    group_op = Some(self.handle_preop_pdi(
                        group_preop_pdi_dc,
                        maindevice.as_ref().unwrap(),
                        spinner,
                    )?);
                    group_preop_pdi_dc = None;
                }

                EtherCATState::Op => {
                    let maindevice = maindevice.as_ref().unwrap();
                    return futures::executor::block_on(async {
                        self.apply_op_optimizations()?;
                        return Ok(self.handle_op(group_op, maindevice, spinner).await?);
                    });
                }
            }
            self.requested_state = None;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
