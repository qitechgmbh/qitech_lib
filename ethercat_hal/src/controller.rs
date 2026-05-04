use crate::{
    ChannelRequest, ChannelRequests, ChannelResponse, ETHERCAT_TX_RX_SIZE,
    EtherCATState, MAX_SUBDEVICES, MetaSubdevice, PDI_LEN, PDU_STORAGE, SdoType,
    ethercat_helpers::{enable_dc_sync, sdo_read, sdo_write},
    get_async_runtime,
    machine_ident_read::{MachineDeviceInfo, read_device_identifications},
    send_response,
};

use ethercrab::{
    MainDevice, MainDeviceConfig, RegisterAddress, RetryBehaviour, SubDeviceGroup,
    Timeouts,
    std::ethercat_now,
    subdevice_group::{DcConfiguration, HasDc, NoDc, Op, PreOpPdi, SafeOp},
};
use std::{
    sync::mpsc::Receiver,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};
use tokio::time::interval;
use triple_buffer::{Input, Output};

pub struct EtherCATController<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub cycle_time_us: u64,
    pub interface: Option<String>,
    pub machine_device_infos: Option<Vec<MachineDeviceInfo>>,

    pub subdevices: [MetaSubdevice; 256],
    pub subdevice_count: usize,

    pub state: EtherCATState,
    requested_state: Option<EtherCATState>,
    rx_channel: Receiver<ChannelRequest>,

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
        interface: Option<String>,
    ) -> Self {
        Self {
            cycle_time_us: 0,
            interface,
            subdevices: [MetaSubdevice::default(); 256],
            subdevice_count: 0,
            state: EtherCATState::NoInterface,
            requested_state: None,
            rx_channel: rx,
            input_producer: input,
            output_consumer: output,
            machine_device_infos: None,
        }
    }

    pub fn get_subdevices(&self) -> &[MetaSubdevice] {
        &self.subdevices[0..self.subdevice_count]
    }
}

pub trait Consumer {
    fn read(&mut self) -> &[u8];
}

pub trait Producer {
    fn input_buffer_mut(&mut self) -> &mut [u8; ETHERCAT_TX_RX_SIZE];
    fn publish(&mut self);
}

pub struct MockConsumer {
    pub buffer: [u8; ETHERCAT_TX_RX_SIZE],
}

pub struct MockProducer {
    pub buffer: [u8; ETHERCAT_TX_RX_SIZE],
}

impl Producer for MockProducer {
    fn input_buffer_mut(&mut self) -> &mut [u8; ETHERCAT_TX_RX_SIZE] {
        &mut self.buffer
    }

    fn publish(&mut self) {
        // does nothing for the mock
    }
}

impl Consumer for MockConsumer {
    fn read(&mut self) -> &[u8] {
        &self.buffer
    }
}

pub struct TripleBufConsumer {
    pub input_consumer: Output<[u8; ETHERCAT_TX_RX_SIZE]>,
}

pub struct TripleBufProducer {
    pub output_producer: Input<[u8; ETHERCAT_TX_RX_SIZE]>,
}

impl Consumer for TripleBufConsumer {
    fn read(&mut self) -> &[u8] {
        self.input_consumer.read()
    }
}

impl Producer for TripleBufProducer {
    fn input_buffer_mut(&mut self) -> &mut [u8; ETHERCAT_TX_RX_SIZE] {
        self.output_producer.input_buffer_mut()
    }

    fn publish(&mut self) {
        self.output_producer.publish();
    }
}

pub struct EtherCATAppHandle<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub input_consumer: C,
    pub output_producer: P,
}

impl<C, P> EtherCATAppHandle<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub fn get_inputs(&mut self) -> &[u8] {
        self.input_consumer.read()
    }

    pub fn write_outputs(&mut self) -> &mut [u8; ETHERCAT_TX_RX_SIZE] {
        self.output_producer.input_buffer_mut()
    }

    pub fn send_outputs(&mut self) {
        self.output_producer.publish();
    }
}


unsafe impl Sync for EtherCATController<TripleBufConsumer, TripleBufProducer> {}
impl EtherCATController<TripleBufConsumer, TripleBufProducer> {
    pub fn ethercat_state_machine(&mut self) {
        let mut ethercat_tx_rx_handle: Result<JoinHandle<()>, std::io::Error>;
        let mut group: Option<SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>> = None;
        let mut group_preop_pdi: SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, NoDc>;
        let mut group_preop_pdi_dc: Option<
            SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, HasDc>,
        > = None;
        let mut group_op: Option<SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, Op, HasDc>> = None;
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
                        ChannelRequests::Shutdown() => return, // We CAN safely shutdonw in Init
                        _ => continue,
                    }

                    use ethercrab::std::tx_rx_task_io_uring;
                    if self.interface.is_some() {
                        let (tx, rx, pdu) = PDU_STORAGE.try_split().expect("can only split once");
                        let pdu_tx = tx;
                        let pdu_rx = rx;
                        let interface = self.interface.clone().unwrap();

                        ethercat_tx_rx_handle = std::thread::Builder::new()
                            .name("EthercatTxRxThread".to_owned())
                            .spawn(move || {
                                tx_rx_task_io_uring(&interface, pdu_tx, pdu_rx)
                                    .expect("Failed to run TX/RX task (io_uring)");
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
                        Err(e) => continue,
                    };

                    //println!("GOT A MESSAGE: {:?}", msg.channel_request);

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
                        ChannelRequests::Shutdown() => return,
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
                        ChannelRequests::EnableDCSync0(device_address) => {
                            let res = enable_dc_sync(&mut preop_group, maindev, device_address);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::EnableDCSync0Response(res),
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
                    let mut tick_interval =
                        rt.block_on(async { interval(Duration::from_micros(1000)) });

                    // println!("Moving into PRE-OP with PDI");
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
                                start_delay: Duration::from_millis(100),
                                sync0_period: Duration::from_micros(1000),
                                sync0_shift: Duration::from_micros(500),
                            },
                        ))
                        .unwrap(),
                    );
                    self.state = EtherCATState::PreopPdi;
                }
                EtherCATState::PreopPdi => {
                    // State machine to handle transition to SafeOp with process data
                    enum GroupState {
                        PreOp(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, HasDc>),
                        SafeOp(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, SafeOp, HasDc>),
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
                                    if tick <= 300 {
                                        tokio::time::sleep_until(now + res.extra.next_cycle_wait)
                                            .await;
                                    }
                                    res
                                });

                                if tick > 300 {
                                    let group_res = rt.block_on(group.request_into_safe_op(device));
                                    let group = group_res.expect("Fail SafeOp");
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
                                    let ready = res.all_safe_op();

                                    if !ready {
                                        tokio::time::sleep_until(now + res.extra.next_cycle_wait)
                                            .await;
                                    }
                                    (ready, group, res.extra.next_cycle_wait)
                                });

                                if is_all_safe {
                                    println!("SAFE-OP");
                                    break group_back;
                                } else {
                                    group_container = Some(GroupState::SafeOp(group_back));
                                }
                            }
                        }
                        tick += 1;
                    };

                    group_op = Some(
                        rt.block_on(group_safe_op.request_into_op(&maindevice.as_ref().unwrap()))
                            .expect("SAFE-OP -> OP"),
                    );

                    println!("Started Transition to OP");
                    self.state = EtherCATState::Op;
                }
                EtherCATState::Op => {
                    let rt = get_async_runtime();
                    let group = group_op.as_ref().unwrap();
                    let maindevice = maindevice.as_ref().unwrap();

                    loop {
                        let response = rt
                            .block_on(group_op.as_ref().unwrap().tx_rx_dc(&maindevice))
                            .expect("TX/RX");
                        if response.all_op() {
                            let mut rx_offset = 0;
                            let mut tx_offset = 0;
                            let mut i = 0;

                            for subdevice in group.iter(&maindevice) {
                                let length_tx = subdevice.io_raw().inputs().len();
                                let length_rx = subdevice.io_raw().outputs().len();

                                self.subdevices[i].initialized = true;
                                self.subdevices[i].start_tx = tx_offset;
                                self.subdevices[i].end_tx = tx_offset + length_tx;

                                self.subdevices[i].start_rx = rx_offset;
                                self.subdevices[i].end_rx = rx_offset + length_rx;

                                rx_offset += length_rx;
                                tx_offset += length_tx;
                                i += 1;
                            }
                            println!("ALL OP");
                            break;
                        }
                    }

                    loop {
                        let now = Instant::now();
                        let _res = rt.block_on(async {
                            let res = group.tx_rx_dc(&maindevice).await.expect("TX/RX");
                            let now = tokio::time::Instant::now();
                            (res, now)
                        });

                        let full_buffer = self.input_producer.input_buffer_mut();
                        // We get a mutable slice to the whole buffer to make sub-slicing easier
                        let mut current_offset = 0;
                        for subdevice in group.iter(&maindevice) {
                            let len = subdevice.io_raw().inputs().len();
                            if current_offset + len <= ETHERCAT_TX_RX_SIZE {
                                full_buffer[current_offset..current_offset + len]
                                    .copy_from_slice(subdevice.io_raw().inputs());
                                current_offset += len;
                            } else {
                                println!("Data exceeds buffer");
                                break;
                            }
                        }
                        self.input_producer.publish();
                        /*rt.block_on(async {
                            tokio::time::sleep_until(res.1 + res.0.extra.next_cycle_wait).await
                        });*/

                        let full_buffer = self.output_consumer.read();
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
                        self.cycle_time_us = now.elapsed().as_micros() as u64;
                    }
                }
            }
            self.requested_state = None;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
