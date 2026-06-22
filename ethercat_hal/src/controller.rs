use crate::Mailbox;
use crate::{
    ChannelRequest, ChannelRequests, ChannelResponse, Consumer, ETHERCAT_TX_RX_SIZE, EtherCATState,
    MAX_SUBDEVICES, MasterConfiguration, MetaSubdevice, PDI_LEN, PDU_STORAGE, Producer, SdoType,
    TripleBufProducer,
    ethercat_helpers::{
        configure_oversampling, enable_dc_sync, enable_dc_sync01, sdo_read, sdo_write,
    },
    get_async_runtime,
    machine_ident_read::{read_device_identifications, write_device_identifications},
    send_response,
};
#[cfg(target_os = "linux")]
use common::set_irq_affinity;
use ethercrab::{
    MainDevice, MainDeviceConfig, RegisterAddress, RetryBehaviour, SubDeviceGroup, Timeouts,
    std::ethercat_now,
    subdevice_group::{DcConfiguration, HasDc, NoDc, Op, PreOpPdi, SafeOp},
};
use std::sync::Arc;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};
use tokio::time::interval;

// Type aliases for the verbose ethercrab generics
type DefaultGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock>;
type PreOpPdiNoDcGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, PreOpPdi, NoDc>;
type PreOpPdiDcGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, PreOpPdi, HasDc>;
type SafeOpDcGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, SafeOp, HasDc>;
type OpDcGroup = SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, Op, HasDc>;

pub struct EtherCATController<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub cycle: u64,
    pub cycle_time_us: u64,
    pub next_cycle: Instant,
    pub dc_system_time_ns: u64,
    pub interface: Option<String>,
    pub subdevices: [MetaSubdevice; 256],
    pub subdevice_count: usize,
    pub state: EtherCATState,
    pub all_subdevices_operational: Arc<AtomicBool>,
    pub current_config: MasterConfiguration,
    requested_state: Option<EtherCATState>,
    rx_channel: Receiver<ChannelRequest>,
    input_producer: P,
    output_consumer: C,
}

#[cfg(target_os = "linux")]
fn set_current_thread_rt_priority(priority: i32) {
    unsafe {
        let thread_id = libc::pthread_self();
        let param = libc::sched_param {
            sched_priority: priority, // 1 to 99
        };

        // SCHED_FIFO is the standard for real-time control loops.
        // It will run until it finishes or is preempted by a higher-priority RT thread.
        let result = libc::pthread_setschedparam(
            thread_id,
            libc::SCHED_FIFO,
            &param as *const libc::sched_param,
        );
        if result != 0 {
            let err = std::io::Error::last_os_error();
            eprintln!(
                "Failed to set RT priority: {}. (Are you root / using sudo?)",
                err
            );
        } else {
            println!("Thread priority set to SCHED_FIFO with level {}", priority);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn set_current_thread_rt_priority(_priority: i32) {
    eprintln!(
        "set_current_thread_rt_priority: real-time scheduling is not available on this platform"
    );
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
        interface: Option<String>,
        config: MasterConfiguration,
    ) -> Self {
        Self {
            cycle: 0,
            next_cycle: std::time::Instant::now(),
            cycle_time_us: 0,
            dc_system_time_ns: 0,
            interface,
            subdevices: [MetaSubdevice::default(); 256],
            subdevice_count: 0,
            state: EtherCATState::NoInterface,
            all_subdevices_operational: Arc::new(AtomicBool::new(false)),
            requested_state: None,
            rx_channel: rx,
            input_producer: input,
            output_consumer: output,
            current_config: config,
        }
    }

    pub fn get_subdevices(&self) -> &[MetaSubdevice] {
        &self.subdevices[0..self.subdevice_count]
    }

    pub fn get_subdevice_count(&self) -> usize {
        self.subdevice_count
    }

    pub fn get_state(&self) -> EtherCATState {
        self.state
    }

    pub fn is_all_operational(&self) -> bool {
        self.all_subdevices_operational.load(Ordering::Acquire)
    }

    pub fn get_cycle(&self) -> u64 {
        self.cycle
    }

    pub fn get_cycle_time_us(&self) -> u64 {
        self.cycle_time_us
    }

    pub fn get_dc_system_time_ns(&self) -> u64 {
        self.dc_system_time_ns
    }
}

unsafe impl Sync for EtherCATController<Arc<Mailbox>, TripleBufProducer> {}

impl EtherCATController<Arc<Mailbox>, TripleBufProducer> {
    fn handle_sdo_read(
        maindev: &MainDevice,
        group: &mut DefaultGroup,
        request: crate::SdoReadRequest,
        response_channel: crate::EtherCATThreadResponseChannel,
    ) {
        match request.type_flag {
            SdoType::BOOL => {
                let res = sdo_read::<bool>(maindev, group, request);
                send_response(response_channel, ChannelResponse::SdoResponseBool(res));
            }
            SdoType::U8 => {
                let res = sdo_read::<u8>(maindev, group, request);
                send_response(response_channel, ChannelResponse::SdoResponseU8(res));
            }
            SdoType::U16 => {
                let res = sdo_read::<u16>(maindev, group, request);
                send_response(response_channel, ChannelResponse::SdoResponseU16(res));
            }
            SdoType::U32 => {
                let res = sdo_read::<u32>(maindev, group, request);
                send_response(response_channel, ChannelResponse::SdoResponseU32(res));
            }
            SdoType::I16 => {
                let res = sdo_read::<i16>(maindev, group, request);
                send_response(response_channel, ChannelResponse::SdoResponseI16(res));
            }
            SdoType::I32 => {
                let res = sdo_read::<i32>(maindev, group, request);
                send_response(response_channel, ChannelResponse::SdoResponseI32(res));
            }
        }
    }

    fn spawn_tx_rx_thread(
        interface: &str,
        tx: ethercrab::PduTx<'static>,
        rx: ethercrab::PduRx<'static>,
        io_failed: Arc<AtomicBool>,
        config: &MasterConfiguration,
    ) -> Result<JoinHandle<()>, std::io::Error> {
        let interface = interface.to_owned();

        #[cfg(target_os = "linux")]
        {
            use ethercrab::std::tx_rx_task_io_uring;
            let opt = config.realtime_optimizations.clone();
            let io_failed_thread = io_failed;
            std::thread::Builder::new()
                .name("EthercatTxRxThread".to_owned())
                .spawn(move || {
                    if let Some(opt) = opt {
                        let id = core_affinity::CoreId { id: opt.ethercat_io_thread_core };
                        set_current_thread_rt_priority(opt.ethercat_io_thread_priority as i32);
                        core_affinity::set_for_current(id);
                        if let Some(irq_core) = opt.pin_irq_core {
                            match set_irq_affinity(&interface, irq_core as u32) {
                                Ok(_) => println!("set irq_affinity of {} to core {}", &interface, irq_core),
                                Err(e) => println!("set_irq_affinity failed: {:?}", e),
                            }
                        }
                    }
                    if let Err(e) = tx_rx_task_io_uring(&interface, tx, rx) {
                        eprintln!("TX/RX task (io_uring) failed: {:?}. Signaling for clean restart.", e);
                        io_failed_thread.store(true, Ordering::Release);
                    }
                })
        }

        #[cfg(not(target_os = "linux"))]
        {
            use ethercrab::std::tx_rx_task;
            let _ = config;
            let io_failed_thread = io_failed;
            std::thread::Builder::new()
                .name("EthercatTxRxThread".to_owned())
                .spawn(move || {
                    get_async_runtime().block_on(async {
                        match tx_rx_task(&interface, tx, rx) {
                            Ok(task) => {
                                if let Err(e) = task.await {
                                    eprintln!("TX/RX task error: {e}. Signaling for clean restart.");
                                    io_failed_thread.store(true, Ordering::Release);
                                }
                            }
                            Err(e) => {
                                eprintln!("TX/RX task creation failed: {e}. Signaling for clean restart.");
                                io_failed_thread.store(true, Ordering::Release);
                            }
                        }
                    });
                })
        }
    }

    fn populate_subdevice_info(&mut self, maindev: &MainDevice, group: &DefaultGroup) {
        let mut i = 0;
        for subdevice in group.iter(maindev) {
            let bytes = subdevice.name().as_bytes();
            let len = bytes.len().min(127);
            self.subdevices[i].name[..len].copy_from_slice(&bytes[..len]);
            self.subdevices[i].product_id = subdevice.identity().product_id;
            self.subdevices[i].revision = subdevice.identity().revision;
            self.subdevices[i].vendor = subdevice.identity().vendor_id;
            self.subdevices[i].device_address = subdevice.configured_address();
            i += 1;
        }
        self.subdevice_count = i;
    }

    fn settle_dc_clocks(
        &self,
        group_pdi: PreOpPdiNoDcGroup,
        maindevice: &MainDevice,
    ) -> PreOpPdiDcGroup {
        let rt = get_async_runtime();
        let mut tick_interval = rt.block_on(async {
            interval(Duration::from_micros(self.current_config.target_cycle_time_us as u64))
        });

        let num_subdevices = group_pdi.iter(maindevice).count();
        let mut averages: Vec<ExponentialMovingAverage> = (0..num_subdevices)
            .map(|_| ExponentialMovingAverage::new(64).unwrap())
            .collect();

        let start = Instant::now();
        let mut now = Instant::now();

        loop {
            rt.block_on(group_pdi.tx_rx_sync_system_time(maindevice))
                .expect("TX/RX");

            if now.elapsed() >= Duration::from_millis(25) {
                now = Instant::now();
                let mut max_deviation = 0u32;
                for (s1, ema) in group_pdi.iter(maindevice).zip(averages.iter_mut()) {
                    let diff = match rt.block_on(
                        s1.register_read::<u32>(RegisterAddress::DcSystemTimeDifference),
                    ) {
                        Ok(value) => {
                            let flag = 0b1u32 << 31;
                            if value >= flag {
                                -((value & !flag) as i32)
                            } else {
                                value as i32
                            }
                        }
                        Err(ethercrab::error::Error::WorkingCounter { .. }) => 0,
                        Err(e) => {
                            println!("Failed to read DC system time: {:?}", e);
                            0
                        }
                    };
                    let ema_next = ema.next(diff as f64);
                    max_deviation = max_deviation.max(ema_next.abs() as u32);
                }
                if max_deviation < 100 {
                    println!("Clocks settled after {} ms", start.elapsed().as_millis());
                    break;
                }
            }
            rt.block_on(tick_interval.tick());
        }

        let dc_cfg = &self.current_config.dc_config;
        rt.block_on(group_pdi.configure_dc_sync(
            maindevice,
            DcConfiguration {
                start_delay: dc_cfg.start_delay,
                sync0_period: dc_cfg.sync0_period,
                sync0_shift: dc_cfg.sync0_shift,
            },
        ))
        .unwrap()
    }

    fn transition_to_safe_op(
        &mut self,
        group_dc: PreOpPdiDcGroup,
        maindevice: &MainDevice,
    ) -> SafeOpDcGroup {
        enum GroupState {
            PreOp(PreOpPdiDcGroup),
            SafeOp(SafeOpDcGroup),
        }

        let rt = get_async_runtime();
        let target_tick = self.current_config.dc_config.target_dc_tick;
        let mut container = Some(GroupState::PreOp(group_dc));
        let mut tick = 0usize;

        let group_safe_op = loop {
            match container.take().unwrap() {
                GroupState::PreOp(group) => {
                    rt.block_on(async {
                        let now = tokio::time::Instant::now();
                        let res = group.tx_rx_dc(maindevice).await.expect("TX/RX");
                        if tick <= target_tick {
                            tokio::time::sleep_until(now + res.extra.next_cycle_wait).await;
                        }
                    });

                    if tick > target_tick {
                        let group = rt.block_on(group.into_safe_op(maindevice))
                            .expect("Failed SafeOp");
                        println!("Requested SAFE-OP");
                        container = Some(GroupState::SafeOp(group));
                    } else {
                        container = Some(GroupState::PreOp(group));
                    }
                }
                GroupState::SafeOp(group) => {
                    let (is_all_safe, group_back) = rt.block_on(async {
                        let now = tokio::time::Instant::now();
                        let res = group.tx_rx_dc(maindevice).await.expect("TX/RX");
                        let ready = res.is_in_state(ethercrab::SubDeviceState::SafeOp);
                        if !ready {
                            tokio::time::sleep_until(now + res.extra.next_cycle_wait).await;
                        }
                        (ready, group)
                    });

                    if is_all_safe {
                        println!("SAFE-OP");
                        break group_back;
                    }
                    container = Some(GroupState::SafeOp(group_back));
                }
            }
            tick += 1;
        };

        let mut rx_offset = 0;
        let mut tx_offset = 0;
        for (i, subdevice) in group_safe_op.iter(maindevice).enumerate() {
            let length_tx = subdevice.io_raw().inputs().len();
            let length_rx = subdevice.io_raw().outputs().len();
            self.subdevices[i].start_tx = tx_offset;
            self.subdevices[i].end_tx = tx_offset + length_tx;
            self.subdevices[i].start_rx = rx_offset;
            self.subdevices[i].end_rx = rx_offset + length_rx;
            rx_offset += length_rx;
            tx_offset += length_tx;
        }

        group_safe_op
    }

    async fn wait_for_all_op(&mut self, group: &OpDcGroup, maindevice: &MainDevice<'_>) {
        loop {
            let cycle_start = Instant::now();
            let res = group.tx_rx_dc(maindevice).await.expect("TX/RX");
            self.dc_system_time_ns = res.extra.dc_system_time;
            self.next_cycle = cycle_start + res.extra.next_cycle_wait;

            while Instant::now() < self.next_cycle {
                std::hint::spin_loop();
            }

            if res.all_op() {
                for i in 0..self.subdevice_count {
                    self.subdevices[i].initialized = true;
                }
                self.all_subdevices_operational.store(true, Ordering::Release);
                println!("ALL OP");
                return;
            }

            let mut has_error = false;
            for index in 0..self.subdevice_count {
                let subdevice = group.subdevice(maindevice, index).expect("No subdevice at index");
                let error_code: u16 = subdevice.register_read(0x0134u16).await
                    .expect("Failed to read error code");
                if error_code != 0 {
                    has_error = true;
                    println!("Subdevice at index {} failed to Op! Error code: {:#02x}", index, error_code);
                }
            }
            if has_error {
                panic!("Failed to go into Op!");
            }
        }
    }

    async fn cyclic_io_loop(&mut self, group: &OpDcGroup, maindevice: &MainDevice<'_>) {
        loop {
            let cycle_start = Instant::now();

            if let Some(full_buffer) = self.output_consumer.read() {
                let mut current_offset = 0;
                for subdevice in group.iter(maindevice) {
                    let mut output = subdevice.outputs_raw_mut();
                    let len = output.len();
                    output.copy_from_slice(&full_buffer[current_offset..current_offset + len]);
                    current_offset += len;
                }
                self.output_consumer.finish_read();
            }

            let res = group.tx_rx_dc(maindevice).await.expect("TX_RX Failed");
            self.dc_system_time_ns = res.extra.dc_system_time;
            self.next_cycle = cycle_start + res.extra.next_cycle_wait;

            if let Some(buffer) = self.input_producer.input_buffer_mut() {
                let mut current_offset = 0;
                for subdevice in group.iter(maindevice) {
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

            while Instant::now() < self.next_cycle {
                std::hint::spin_loop();
            }

            self.cycle_time_us = cycle_start.elapsed().as_micros() as u64;
            self.cycle = self.cycle.wrapping_add(1);
        }
    }

    pub fn ethercat_state_machine(&mut self) -> Result<(), anyhow::Error> {
        let mut _ethercat_tx_rx_handle: Result<JoinHandle<()>, std::io::Error>;
        let mut group: Option<DefaultGroup> = None;
        let mut group_preop_pdi_dc: Option<PreOpPdiDcGroup> = None;
        let mut group_op: Option<OpDcGroup> = None;
        let mut maindevice: Option<MainDevice> = None;
        let io_failed = Arc::new(AtomicBool::new(false));

        loop {
            if io_failed.load(Ordering::Acquire) {
                return Err(anyhow::anyhow!(
                    "EtherCAT TX/RX task failed; terminating for a clean restart."
                ));
            }

            match self.state {
                EtherCATState::NoInterface => {
                    if self.interface.is_some() {
                        self.state = EtherCATState::Init;
                    }
                }
                EtherCATState::Boot => {}
                EtherCATState::Init => {
                    let msg = match self.rx_channel.try_recv() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    match msg.channel_request {
                        ChannelRequests::ChangeState(EtherCATState::PreOp) => (),
                        ChannelRequests::Shutdown() => return Ok(()),
                        _ => continue,
                    }

                    if let Some(ref interface) = self.interface {
                        let (tx, rx, pdu) = PDU_STORAGE.try_split().expect("can only split once");

                        _ethercat_tx_rx_handle = Self::spawn_tx_rx_thread(
                            interface,
                            tx,
                            rx,
                            io_failed.clone(),
                            &self.current_config,
                        );

                        maindevice = Some(MainDevice::new(
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
                                retry_behaviour: RetryBehaviour::Count(5),
                                dc_static_sync_iterations: 10_000,
                            },
                        ));

                        let rt = get_async_runtime();
                        let res = rt.block_on(async {
                            maindevice
                                .as_ref()
                                .unwrap()
                                .init_single_group::<MAX_SUBDEVICES, PDI_LEN>(ethercat_now)
                                .await
                        });

                        match res {
                            Ok(g) => {
                                println!("Initialized {} subdevices", g.len());
                                group = Some(g);
                                self.state = EtherCATState::PreOp;
                                send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                            }
                            Err(err) => {
                                println!("failed moving to PreOp from Init {:?}", err);
                                self.state = EtherCATState::Init;
                                send_response(
                                    msg.response_channel,
                                    ChannelResponse::ChangeState(Err(err.into())),
                                );
                                continue;
                            }
                        }
                    }
                }
                EtherCATState::PreOp => {
                    let maindev = maindevice.as_ref().unwrap();
                    let mut preop_group = group.as_mut().unwrap();
                    self.populate_subdevice_info(maindev, preop_group);

                    let msg = match self.rx_channel.try_recv() {
                        Ok(value) => value,
                        Err(_e) => continue,
                    };

                    match msg.channel_request {
                        ChannelRequests::ChangeState(ether_catstate) => match ether_catstate {
                            EtherCATState::NoInterface => {
                                self.state = ether_catstate;
                                send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                                continue;
                            }
                            EtherCATState::PreOp => continue,
                            EtherCATState::Op => (),
                            _ => continue,
                        },
                        ChannelRequests::Shutdown() => return Ok(()),
                        ChannelRequests::SdoWriteRequest(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(msg.response_channel, ChannelResponse::SdoWriteResponse(res));
                            continue;
                        }
                        ChannelRequests::SdoReadRequest(request) => {
                            Self::handle_sdo_read(maindev, preop_group, request, msg.response_channel);
                            continue;
                        }
                        ChannelRequests::ReadMachineIdent() => {
                            let res = read_device_identifications(preop_group, maindev);
                            send_response(msg.response_channel, ChannelResponse::MachineDeviceInfoResponse(res));
                            continue;
                        }
                        ChannelRequests::WriteMachineIdent(identifications) => {
                            let res = write_device_identifications(preop_group, maindev, &identifications);
                            send_response(msg.response_channel, ChannelResponse::WriteMachineInfoResponse(res));
                            continue;
                        }
                        ChannelRequests::EnableDCSync0(device_address) => {
                            let res = enable_dc_sync(&mut preop_group, maindev, device_address);
                            send_response(msg.response_channel, ChannelResponse::EnableDCSync0Response(res));
                            continue;
                        }
                        ChannelRequests::EnableDCSync01(device_address, sync1_period) => {
                            let res = enable_dc_sync01(&mut preop_group, maindev, device_address, sync1_period);
                            send_response(msg.response_channel, ChannelResponse::EnableDCSync01Response(res));
                            continue;
                        }
                        ChannelRequests::ConfigureOversampling(device_address, factor) => {
                            let res = configure_oversampling(&mut preop_group, maindev, device_address, factor);
                            send_response(msg.response_channel, ChannelResponse::ConfigureOversamplingResponse(res));
                            continue;
                        }
                    }

                    let rt = get_async_runtime();
                    let group_to_transition = group.take().expect("Group missing in PreOp");
                    let device_ref = maindevice.as_ref().expect("MainDevice missing");
                    let group_pdi = rt
                        .block_on(async { group_to_transition.into_pre_op_pdi(device_ref).await })
                        .expect("Failed into_pre_op_pdi");

                    group_preop_pdi_dc = Some(self.settle_dc_clocks(group_pdi, device_ref));
                    self.state = EtherCATState::PreopPdi;
                }
                EtherCATState::PreopPdi => {
                    let device = maindevice.as_ref().unwrap();
                    let pdi_dc = group_preop_pdi_dc.take().expect("PDI Group missing");

                    let group_safe_op = self.transition_to_safe_op(pdi_dc, device);

                    let rt = get_async_runtime();
                    match rt.block_on(group_safe_op.request_into_op(device)) {
                        Ok(g) => group_op = Some(g),
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "EtherCAT SAFE-OP -> OP transition failed: {e:?}. Terminating for a clean restart."
                            ));
                        }
                    }

                    println!("Started Transition to OP");
                    self.state = EtherCATState::Op;
                }
                EtherCATState::Op => {
                    let rt = get_async_runtime();
                    let group = group_op.as_ref().unwrap();
                    let maindev = maindevice.as_ref().unwrap();

                    rt.block_on(async {
                        if let Some(opt) = &self.current_config.realtime_optimizations {
                            let id = core_affinity::CoreId { id: opt.ethercat_loop_thread_core };
                            set_current_thread_rt_priority(opt.ethercat_loop_thread_priority as i32);
                            core_affinity::set_for_current(id);
                        }

                        self.wait_for_all_op(group, maindev).await;
                        self.cyclic_io_loop(group, maindev).await;
                    });
                }
            }
            self.requested_state = None;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
