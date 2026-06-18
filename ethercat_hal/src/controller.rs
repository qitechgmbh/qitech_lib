use crate::Mailbox;
use crate::{
    ChannelRequest, ChannelRequests, ChannelResponse, Consumer, ETHERCAT_TX_RX_SIZE, EtherCATState,
    MAX_SUBDEVICES, MasterConfiguration, MetaSubdevice, PDI_LEN, PDU_STORAGE, Producer, SdoType,
    TripleBufProducer,
    ethercat_helpers::{configure_oversampling, enable_dc_sync, enable_dc_sync01, sdo_read, sdo_write},
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
    sync::mpsc::Receiver,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};
use tokio::time::interval;

pub struct EtherCATController<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub cycle: u64,
    pub cycle_time_us: u64,
    pub next_cycle: Instant,
    pub interface: Option<String>,
    pub subdevices: [MetaSubdevice; 256],
    pub subdevice_count: usize,
    pub state: EtherCATState,
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
            interface,
            subdevices: [MetaSubdevice::default(); 256],
            subdevice_count: 0,
            state: EtherCATState::NoInterface,
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

    pub fn get_cycle(&self) -> u64 {
        self.cycle
    }

    pub fn get_cycle_time_us(&self) -> u64 {
        self.cycle_time_us
    }
}

unsafe impl Sync for EtherCATController<Arc<Mailbox>, TripleBufProducer> {}
impl EtherCATController<Arc<Mailbox>, TripleBufProducer> {
    pub fn ethercat_state_machine(&mut self) -> Result<(), anyhow::Error> {
        let mut _ethercat_tx_rx_handle: Result<JoinHandle<()>, std::io::Error>;
        let mut group: Option<SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock>> =
            None;
        let mut group_preop_pdi: SubDeviceGroup<
            MAX_SUBDEVICES,
            PDI_LEN,
            ethercrab::DefaultLock,
            PreOpPdi,
            NoDc,
        >;
        let mut group_preop_pdi_dc: Option<
            SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, PreOpPdi, HasDc>,
        > = None;
        let mut group_op: Option<
            SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, ethercrab::DefaultLock, Op, HasDc>,
        > = None;
        let mut maindevice: Option<MainDevice> = None;
        loop {
            match self.state {
                EtherCATState::NoInterface => {
                    if self.interface.is_some() {
                        self.state = EtherCATState::Init;
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

                    #[cfg(not(target_os = "linux"))]
                    use ethercrab::std::tx_rx_task;
                    #[cfg(target_os = "linux")]
                    use ethercrab::std::tx_rx_task_io_uring;
                    if self.interface.is_some() {
                        let (tx, rx, pdu) = PDU_STORAGE.try_split().expect("can only split once");
                        let interface = self.interface.clone().unwrap();

                        #[cfg(target_os = "linux")]
                        {
                            let pdu_tx = tx;
                            let pdu_rx = rx;
                            let opt = self.current_config.realtime_optimizations.clone();
                            _ethercat_tx_rx_handle = std::thread::Builder::new()
                                .name("EthercatTxRxThread".to_owned())
                                .spawn(move || {
                                    match opt {
                                        Some(opt) => {
                                            let id = core_affinity::CoreId {
                                                id: opt.ethercat_io_thread_core,
                                            };
                                            set_current_thread_rt_priority(
                                                opt.ethercat_io_thread_priority as i32,
                                            );
                                            // Pin to the specified core
                                            core_affinity::set_for_current(id);
                                            if let Some(irq_core) = opt.pin_irq_core {
                                                let res =
                                                    set_irq_affinity(&interface, irq_core as u32);
                                                if res.is_err() {
                                                    println!("set_irq_affinity failed: {:?}", res);
                                                } else {
                                                    println!(
                                                        "set irq_affinity of {} to core {}",
                                                        &interface, irq_core
                                                    );
                                                }
                                            }
                                        }
                                        None => (),
                                    };
                                    tx_rx_task_io_uring(&interface, pdu_tx, pdu_rx)
                                        .expect("Failed to run TX/RX task (io_uring)");
                                });
                        }

                        #[cfg(not(target_os = "linux"))]
                        {
                            let pdu_tx = tx;
                            let pdu_rx = rx;
                            _ethercat_tx_rx_handle = std::thread::Builder::new()
                                .name("EthercatTxRxThread".to_owned())
                                .spawn(move || {
                                    get_async_runtime().block_on(async {
                                        match tx_rx_task(&interface, pdu_tx, pdu_rx) {
                                            Ok(task) => {
                                                if let Err(e) = task.await {
                                                    eprintln!("TX/RX task error: {}", e);
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("TX/RX task creation failed: {}", e);
                                            }
                                        }
                                    });
                                });
                        }

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
                        group = Some(match res {
                            Ok(group) => {
                                println!("Initialized {} subdevices", &group.len());
                                group
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
                        });
                        self.state = EtherCATState::PreOp;
                        send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                    };
                }
                EtherCATState::PreOp => {
                    let maindev = maindevice.as_ref().unwrap();
                    let mut preop_group = group.as_mut().unwrap();

                    let mut i = 0;
                    for subdevice in preop_group.iter(&maindev) {
                        let bytes = subdevice.name().as_bytes();
                        let len = std::cmp::min(bytes.len(), 127);
                        // Copy the slice into the array
                        self.subdevices[i].name[..len].copy_from_slice(&bytes[..len]);
                        self.subdevices[i].product_id = subdevice.identity().product_id;
                        self.subdevices[i].revision = subdevice.identity().revision;
                        self.subdevices[i].vendor = subdevice.identity().vendor_id;
                        self.subdevices[i].device_address = subdevice.configured_address();
                        i += 1;
                    }
                    self.subdevice_count = i;
                    let msg = match self.rx_channel.try_recv() {
                        Ok(value) => value,
                        Err(_e) => continue,
                    };

                    match msg.channel_request {
                        ChannelRequests::ChangeState(ether_catstate) => match ether_catstate {
                            EtherCATState::NoInterface => {
                                self.state = ether_catstate;
                                send_response(
                                    msg.response_channel,
                                    ChannelResponse::ChangeState(Ok(())),
                                );
                                continue; // end the loop here -> go back to NoInterface state
                            }
                            EtherCATState::PreOp => continue,
                            EtherCATState::Op => (),
                            _ => continue,
                        },
                        ChannelRequests::Shutdown() => return Ok(()),
                        ChannelRequests::SdoWriteRequest(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoWriteResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::SdoReadRequest(request) => {
                            match request.type_flag {
                                SdoType::BOOL => {
                                    let res = sdo_read::<bool>(maindev, preop_group, request);
                                    send_response(
                                        msg.response_channel,
                                        ChannelResponse::SdoResponseBool(res),
                                    );
                                }
                                SdoType::U8 => {
                                    let res = sdo_read::<u8>(maindev, preop_group, request);
                                    send_response(
                                        msg.response_channel,
                                        ChannelResponse::SdoResponseU8(res),
                                    );
                                }
                                SdoType::U16 => {
                                    let res = sdo_read::<u16>(maindev, preop_group, request);
                                    send_response(
                                        msg.response_channel,
                                        ChannelResponse::SdoResponseU16(res),
                                    );
                                }
                                SdoType::U32 => {
                                    let res = sdo_read::<u32>(maindev, preop_group, request);
                                    send_response(
                                        msg.response_channel,
                                        ChannelResponse::SdoResponseU32(res),
                                    );
                                }
                                SdoType::I16 => {
                                    let res = sdo_read::<i16>(maindev, preop_group, request);
                                    send_response(
                                        msg.response_channel,
                                        ChannelResponse::SdoResponseI16(res),
                                    );
                                }
                                SdoType::I32 => {
                                    let res = sdo_read::<i32>(maindev, preop_group, request);
                                    send_response(
                                        msg.response_channel,
                                        ChannelResponse::SdoResponseI32(res),
                                    );
                                }
                            }

                            continue;
                        }
                        ChannelRequests::ReadMachineIdent() => {
                            let res = read_device_identifications(preop_group, maindev);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::MachineDeviceInfoResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::WriteMachineIdent(identifications) => {
                            let res = write_device_identifications(
                                preop_group,
                                maindev,
                                &identifications,
                            );
                            send_response(
                                msg.response_channel,
                                ChannelResponse::WriteMachineInfoResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::EnableDCSync0(device_address) => {
                            let res = enable_dc_sync(&mut preop_group, maindev, device_address);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::EnableDCSync0Response(res),
                            );
                            continue;
                        }
                        ChannelRequests::EnableDCSync01(device_address, sync1_period) => {
                            let res = enable_dc_sync01(
                                &mut preop_group,
                                maindev,
                                device_address,
                                sync1_period,
                            );
                            send_response(
                                msg.response_channel,
                                ChannelResponse::EnableDCSync01Response(res),
                            );
                            continue;
                        }
                        ChannelRequests::ConfigureOversampling(device_address, factor) => {
                            let res = configure_oversampling(
                                &mut preop_group,
                                maindev,
                                device_address,
                                factor,
                            );
                            send_response(
                                msg.response_channel,
                                ChannelResponse::ConfigureOversamplingResponse(res),
                            );
                            continue;
                        }
                    }
                    let mut now = Instant::now();
                    let start = Instant::now();
                    let mut averages = Vec::new();

                    if let Some(group_ref) = group.as_ref() {
                        for _ in 0..group_ref.len() {
                            averages.push(ExponentialMovingAverage::new(64).unwrap());
                        }
                    }

                    let rt = get_async_runtime();
                    let mut tick_interval = rt.block_on(async {
                        interval(Duration::from_micros(
                            self.current_config.target_cycle_time_us as u64,
                        ))
                    });

                    let group_to_transition = group.take().expect("Group missing in PreOp");
                    let device_ref = maindevice.as_ref().expect("MainDevice missing");
                    let rt = get_async_runtime();
                    let res = rt
                        .block_on(async { group_to_transition.into_pre_op_pdi(device_ref).await });

                    group_preop_pdi = match res {
                        Ok(group) => group,
                        Err(_) => todo!(),
                    };

                    loop {
                        rt.block_on(
                            group_preop_pdi.tx_rx_sync_system_time(&maindevice.as_ref().unwrap()),
                        )
                        .expect("TX/RX");

                        if now.elapsed() >= Duration::from_millis(25) {
                            now = Instant::now();
                            let mut max_deviation = 0;
                            for (s1, ema) in group_preop_pdi
                                .iter(&maindevice.as_ref().unwrap())
                                .zip(averages.iter_mut())
                            {
                                let diff =
                                    match rt.block_on(s1.register_read::<u32>(
                                        RegisterAddress::DcSystemTimeDifference,
                                    )) {
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
                    let device = maindevice.as_ref().unwrap();
                    group_preop_pdi_dc = Some(
                        rt.block_on(group_preop_pdi.configure_dc_sync(
                            device,
                            DcConfiguration {
                                start_delay: self.current_config.dc_config.start_delay,
                                sync0_period: self.current_config.dc_config.sync0_period,
                                sync0_shift: self.current_config.dc_config.sync0_shift,
                            },
                        ))
                        .unwrap(),
                    );
                    self.state = EtherCATState::PreopPdi;
                }
                EtherCATState::PreopPdi => {
                    // State machine to handle transition to SafeOp with process data
                    enum GroupState {
                        PreOp(
                            SubDeviceGroup<
                                MAX_SUBDEVICES,
                                PDI_LEN,
                                ethercrab::DefaultLock,
                                PreOpPdi,
                                HasDc,
                            >,
                        ),
                        SafeOp(
                            SubDeviceGroup<
                                MAX_SUBDEVICES,
                                PDI_LEN,
                                ethercrab::DefaultLock,
                                SafeOp,
                                HasDc,
                            >,
                        ),
                    }

                    let mut group_container = Some(GroupState::PreOp(
                        group_preop_pdi_dc.take().expect("PDI Group missing"),
                    ));

                    let mut tick = 0;
                    let rt = get_async_runtime();
                    let group_safe_op = loop {
                        match group_container.take().unwrap() {
                            GroupState::PreOp(group) => {
                                let device = maindevice.as_ref().unwrap();

                                // Wrap the whole sequence in one block_on so 'now' and 'sleep' share the same reactor session
                                let _res = rt.block_on(async {
                                    let now = tokio::time::Instant::now(); // Moved inside
                                    let res = group.tx_rx_dc(device).await.expect("TX/RX");
                                    if tick <= self.current_config.dc_config.target_dc_tick {
                                        tokio::time::sleep_until(now + res.extra.next_cycle_wait)
                                            .await;
                                    }
                                    res
                                });

                                if tick > self.current_config.dc_config.target_dc_tick {
                                    let group_res = rt.block_on(group.into_safe_op(device));
                                    let group = group_res.expect("Failed SafeOp");
                                    group_container = Some(GroupState::SafeOp(group));
                                    println!("Requested SAFE-OP");
                                } else {
                                    group_container = Some(GroupState::PreOp(group));
                                }
                            }
                            GroupState::SafeOp(group) => {
                                let device = maindevice.as_ref().unwrap();
                                // Apply the same logic here
                                let (is_all_safe, group_back, _) = rt.block_on(async {
                                    let now = tokio::time::Instant::now();
                                    let res = group.tx_rx_dc(device).await.expect("TX/RX");
                                    let ready = res.is_in_state(ethercrab::SubDeviceState::SafeOp);
                                    if !ready {
                                        tokio::time::sleep_until(now + res.extra.next_cycle_wait)
                                            .await;
                                    }
                                    (ready, group, res.extra.next_cycle_wait)
                                });

                                if is_all_safe {
                                    println!("SAFE-OP");
                                    // --- Calculate and map offsets here ---
                                    let mut rx_offset = 0;
                                    let mut tx_offset = 0;
                                    for (i, subdevice) in group_back.iter(device).enumerate() {
                                        let length_tx = subdevice.io_raw().inputs().len();
                                        let length_rx = subdevice.io_raw().outputs().len();

                                        self.subdevices[i].start_tx = tx_offset;
                                        self.subdevices[i].end_tx = tx_offset + length_tx;

                                        self.subdevices[i].start_rx = rx_offset;
                                        self.subdevices[i].end_rx = rx_offset + length_rx;

                                        rx_offset += length_rx;
                                        tx_offset += length_tx;
                                    }
                                    break group_back;
                                } else {
                                    group_container = Some(GroupState::SafeOp(group_back));
                                }
                            }
                        }
                        tick += 1;
                    };

                    // Use the non-blocking request_into_op + manual tx_rx_dc polling
                    // loop. The blocking `into_op()` can hit an io_uring TX/RX race on Linux.
                    match rt.block_on(group_safe_op.request_into_op(&maindevice.as_ref().unwrap()))
                    {
                        Ok(group) => group_op = Some(group),
                        Err(e) => {
                            // request_into_op consumes the group — no retry possible.
                            return Err(anyhow::anyhow!(
                                "EtherCAT SAFE-OP -> OP transition failed: {:?}",
                                e
                            ));
                        }
                    }

                    println!("Started Transition to OP");
                    self.state = EtherCATState::Op;
                }
                EtherCATState::Op => {
                    let rt = get_async_runtime();
                    let group = group_op.as_ref().unwrap();
                    let maindevice = maindevice.as_ref().unwrap();
                    rt.block_on(async {
                        match &self.current_config.realtime_optimizations {
                            Some(opt) => {
                                let id = core_affinity::CoreId {
                                    id: opt.ethercat_loop_thread_core,
                                };
                                set_current_thread_rt_priority(
                                    opt.ethercat_loop_thread_priority as i32,
                                );
                                core_affinity::set_for_current(id);
                            }
                            None => (),
                        };

                        loop {
                            let cycle_start = Instant::now();
                            let res = group_op
                                .as_ref()
                                .unwrap()
                                .tx_rx_dc(&maindevice)
                                .await
                                .expect("TX/RX");
                            self.next_cycle = cycle_start + res.extra.next_cycle_wait;

                            while Instant::now() < self.next_cycle {
                                std::hint::spin_loop();
                            }

                            if res.all_op() {
                                for i in 0..self.subdevice_count {
                                    self.subdevices[i].initialized = true;
                                }
                                println!("ALL OP");
                                break;
                            }
                        }

                        loop {
                            let cycle_start = Instant::now();

                            match self.output_consumer.read() {
                                Some(full_buffer) => {
                                    // We get a mutable slice to the whole buffer to make sub-slicing easier
                                    let mut current_offset = 0;
                                    for subdevice in group.iter(&maindevice) {
                                        let mut output = subdevice.outputs_raw_mut();
                                        let len = output.len();
                                        output.copy_from_slice(
                                            &full_buffer[current_offset..current_offset + len],
                                        );
                                        current_offset += len;
                                    }
                                    self.output_consumer.finish_read();
                                }
                                None => {}
                            };

                            let res = group.tx_rx_dc(&maindevice).await.expect("TX_RX Failed");
                            self.next_cycle = cycle_start + res.extra.next_cycle_wait;
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

                            while Instant::now() < self.next_cycle {
                                std::hint::spin_loop();
                            }

                            self.cycle_time_us = cycle_start.elapsed().as_micros() as u64;
                            if self.cycle == u64::MAX {
                                self.cycle = 0;
                            } else {
                                self.cycle += 1;
                            }
                        }
                    });
                }
            }
            self.requested_state = None;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
