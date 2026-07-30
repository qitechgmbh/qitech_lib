pub const MAX_PDU_DATA: usize = PduStorage::element_size(512);
pub const MAX_FRAMES: usize = 32;
pub const PDI_LEN: usize = 1024;
static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();
use crate::MAX_SUBDEVICES;

use crate::{ETHERCAT_TX_RX_SIZE, TripleBufProducer};
use crate::controller::ethercrab_funcs::{configure_oversampling, enable_dc_sync, enable_dc_sync01, read_device_identifications, sdo_read, sdo_write, write_device_identifications};
use crate::{
    ChannelRequests, ChannelResponse, Consumer, EtherCATState, Producer, SdoType,    
    get_async_runtime,
    send_response,
};
use crate::{EtherCATController, Mailbox, set_current_thread_rt_priority};
use anyhow::bail;
#[cfg(target_os = "linux")]
use common::set_irq_affinity;
use ethercrab::PduStorage;
use ethercrab::{
    MainDevice, MainDeviceConfig, RegisterAddress, RetryBehaviour, SubDeviceGroup, Timeouts,
    std::ethercat_now,
    subdevice_group::{DcConfiguration, HasDc, NoDc, Op, PreOpPdi, SafeOp},
};
use libc::{MCL_CURRENT, MCL_FUTURE, mlockall};
use spin_sleep::SpinSleeper;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};

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
        let spinner = SpinSleeper::new(200_000);

        loop {
            let state: EtherCATState = self.status.state.load(Relaxed).into();

            match state {
                EtherCATState::NoInterface => {
                    if self.interface.is_some() {
                        self.status.state.store(EtherCATState::Init.into(), Relaxed);
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
                        let pdu_tx = tx;
                        let pdu_rx = rx;
                        let interface = self.interface.clone().unwrap();
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
                                retry_behaviour: RetryBehaviour::Count(0),
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
                            Ok(group) => group,
                            Err(err) => {
                                self.status.state.store(EtherCATState::Init.into(), Relaxed);
                                send_response(
                                    msg.response_channel,
                                    ChannelResponse::ChangeState(Err(err.into())),
                                );
                                continue;
                            }
                        });
                        self.status.state.store(EtherCATState::PreOp.into(), Relaxed);
                        send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                    };
                }
                EtherCATState::PreOp => {
                    let mut i = 0;
                    let maindev = maindevice.as_ref().unwrap();
                    let mut preop_group = group.as_mut().unwrap();
                    let mut subdevice_guard = get_async_runtime().block_on(self.status.subdevices.lock());

                    for subdevice in preop_group.iter(&maindev) {
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
                    self.status.subdevice_count.store(i as u64, Relaxed);
                    let msg = match self.rx_channel.try_recv() {
                        Ok(value) => value,
                        Err(_e) => continue,
                    };

                    match msg.channel_request {
                        ChannelRequests::ChangeState(ether_catstate) => match ether_catstate {
                            EtherCATState::NoInterface => {
                                self.status.state.store(ether_catstate.into(), Relaxed);
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
                        ChannelRequests::ConfigureOversampling(
                            device_address,
                            oversampling_settings,
                        ) => {
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
                            continue;
                        }
                    }
                    let mut now = Instant::now();
                    let mut averages = Vec::new();

                    if let Some(group_ref) = group.as_ref() {
                        for _ in 0..group_ref.len() {
                            averages.push(ExponentialMovingAverage::new(64).unwrap());
                        }
                    }

                    let group_to_transition = group.take().expect("Group missing in PreOp");
                    let device_ref = maindevice.as_ref().expect("MainDevice missing");
                    let rt = get_async_runtime();
                    let res = rt
                        .block_on(async { group_to_transition.into_pre_op_pdi(device_ref).await });

                    group_preop_pdi = match res {
                        Ok(group) => group,
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to transition group into PreOpPdi: {e:?}"
                            ));
                        }
                    };

                    loop {
                        let deadline = Instant::now()
                            + Duration::from_micros(
                                self.current_config.target_cycle_time_us as u64,
                            );
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
                    self.status.state.store(EtherCATState::PreopPdi.into(), Relaxed);
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
                                let now = Instant::now(); // Moved inside
                                let res = rt.block_on(group.tx_rx_dc(device)).expect("");
                                if tick <= self.current_config.dc_config.target_dc_tick {
                                    spinner.sleep_until(now + res.extra.next_cycle_wait);
                                }

                                if tick > self.current_config.dc_config.target_dc_tick {
                                    let group_res = rt.block_on(group.into_safe_op(device));
                                    let group = group_res.map_err(|e| anyhow::anyhow!(
                                        "EtherCAT PreOp → SafeOp transition failed: {e:?}. Terminating for a clean restart."
                                    ))?;
                                    group_container = Some(GroupState::SafeOp(group));
                                } else {
                                    group_container = Some(GroupState::PreOp(group));
                                }
                            }
                            GroupState::SafeOp(group) => {
                                let device = maindevice.as_ref().unwrap();
                                // Apply the same logic here
                                let (is_all_safe, group_back, _) = rt.block_on(async {
                                    let now = Instant::now();
                                    let res = group.tx_rx_dc(device).await.expect("TX/RX");
                                    let ready = res.is_in_state(ethercrab::SubDeviceState::SafeOp);
                                    if !ready {
                                        spinner.sleep_until(now + res.extra.next_cycle_wait);
                                    }
                                    (ready, group, res.extra.next_cycle_wait)
                                });

                                if is_all_safe {
                                    let mut rx_offset = 0;
                                    let mut tx_offset = 0;
                                    let mut subdevice_guard =
                                        get_async_runtime().block_on(self.status.subdevices.lock());
                                    for (i, subdevice) in group_back.iter(device).enumerate() {
                                        let length_tx = subdevice.io_raw().inputs().len();
                                        let length_rx = subdevice.io_raw().outputs().len();

                                        subdevice_guard[i].start_tx = tx_offset;
                                        subdevice_guard[i].end_tx = tx_offset + length_tx;

                                        subdevice_guard[i].start_rx = rx_offset;
                                        subdevice_guard[i].end_rx = rx_offset + length_rx;

                                        rx_offset += length_rx;
                                        tx_offset += length_tx;
                                    }
                                    drop(subdevice_guard);
                                    break group_back;
                                } else {
                                    group_container = Some(GroupState::SafeOp(group_back));
                                }
                            }
                        }
                        tick += 1;
                    };

                    match rt.block_on(group_safe_op.request_into_op(&maindevice.as_ref().unwrap()))
                    {
                        Ok(group) => group_op = Some(group),
                        Err(e) => {
                            // request_into_op consumes the group — no retry possible.
                            // Return Err so the state-machine thread ends cleanly;
                            // the supervisor (systemd) restarts the process.
                            return Err(anyhow::anyhow!(
                                "EtherCAT SAFE-OP -> OP transition failed: {e:?}. Terminating for a clean restart."
                            ));
                        }
                    }
                    self.status.state.store(EtherCATState::Op.into(), Relaxed);
                }

                EtherCATState::Op => {
                    let group = group_op.as_ref().unwrap();
                    let maindevice = maindevice.as_ref().unwrap();
                    return futures::executor::block_on(async {
                        match &self.current_config.realtime_optimizations {
                            Some(opt) => {
                                let id = core_affinity::CoreId {
                                    id: opt.ethercat_loop_thread_core,
                                };
                                set_current_thread_rt_priority(
                                    opt.ethercat_loop_thread_priority as i32,
                                );
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

                        let mut is_all_op = false;
                        let mut not_all_op_cycles: u32 = 0;
                        let cycle_time_ns = self.current_config.target_cycle_time_us as i64 * 1000;
                        let mut integral: i64 = 0;
                        let mut error: i64;
                        let mut delta: i64;
                        // TODO Make these configurable?
                        let pgain = 0.01 as f64;
                        let igain = 0.00002 as f64;
                        // sync_offset_ns is 50% of macro cycle time(Sync1 FULL period)
                        // This essentially means we send the frame 50% into the sync1 period
                        let sync_offset_ns: u64 =
                            self.current_config.target_cycle_time_us as u64 / 2;

                        loop {
                            let cycle_start = Instant::now();
                            let res = group.tx_rx_dc(&maindevice).await.expect("TX_RX Failed");
                            delta =
                                (res.extra.dc_system_time - sync_offset_ns) as i64 % cycle_time_ns;
                            if delta > (cycle_time_ns / 2) {
                                delta = delta - cycle_time_ns
                            }
                            error = -delta;
                            // Not sure what to clamp to, if at all Clamping seemed to have a negative effect? so just keep it as is
                            integral = integral + error; //.clamp(sync_offset_ns as i64*-1 * 10, sync_offset_ns as i64 * 10);
                            // Maybe instead it makes sense to clamp offsettime?
                            let offsettime =
                                ((error as f64 * pgain) + (integral as f64 * igain)) as i64;
                            self.status.dc_sys_time
                                .store(res.extra.dc_system_time, Relaxed);
                            self.next_cycle = cycle_start
                                + Duration::from_nanos(cycle_time_ns as u64 + offsettime as u64);
                            if !is_all_op {
                                if res.all_op() {
                                    let mut subdevice_guard = self.status.subdevices.lock().await;
                                    for i in 0..self.status.subdevice_count.load(Relaxed) {
                                        subdevice_guard[i as usize].initialized = true;
                                    }
                                    self.status.all_op.store(true, Relaxed);
                                    drop(subdevice_guard);
                                    not_all_op_cycles = 0;
                                    is_all_op = true;
                                } else {
                                    spinner.sleep_until(self.next_cycle);
                                    self.status.cycle_time_us
                                        .store(cycle_start.elapsed().as_micros() as u64, Relaxed);
                                    not_all_op_cycles += 1;

                                    if not_all_op_cycles >= self.current_config.op_ramp_grace_cycles
                                    {
                                        // Read AL status now (only at timeout) for the error message.
                                        let mut erroring_subdevices: Vec<(usize, u16)> = Vec::new();
                                        for index in 0..self.status.subdevice_count.load(Relaxed) {
                                            if let Ok(subdevice) =
                                                group.subdevice(maindevice, index as usize)
                                            {
                                                let code: u16 = subdevice
                                                    .register_read(0x0134u16)
                                                    .await
                                                    .unwrap_or(0);
                                                if code != 0 {
                                                    erroring_subdevices
                                                        .push((index as usize, code));
                                                }
                                            }
                                        }
                                        bail!(
                                            "EtherCAT OP ramp timed out after {} cycles without all devices reaching OP. \
                                         AL errors at timeout: {:?}. Terminating for a clean restart.",
                                            not_all_op_cycles,
                                            erroring_subdevices
                                        );
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
                            self.status.inputs_ready.store(true, Relaxed);

                            // This gives the client side time to look at the inputs and write outputs
                            // Might be a bit too tight if running < 125us but at that point it isnt really stable anyways
                            spinner.sleep_until(self.next_cycle - Duration::from_nanos(10000));
                            match self.output_consumer.read() {
                                Some(full_buffer) => {
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
                            spinner.sleep_until(self.next_cycle);
                            self.status.cycle_time_us
                                .store(cycle_start.elapsed().as_micros() as u64, Relaxed);
                            if self.status.cycle.load(Relaxed) == u64::MAX {
                                self.status.cycle.store(0, Relaxed);
                            } else {
                                self.status.cycle.fetch_add(1, Relaxed);
                            }
                            self.status.inputs_ready.store(false, Relaxed);
                        }
                    });
                }
            }
            self.requested_state = None;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
