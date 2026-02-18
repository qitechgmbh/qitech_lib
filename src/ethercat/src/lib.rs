use ethercrab::{
    EtherCrabWireRead, EtherCrabWireSized, EtherCrabWireWrite, MainDevice, MainDeviceConfig,
    PduStorage, RegisterAddress, RetryBehaviour, SubDeviceGroup, Timeouts,
    std::ethercat_now,
    subdevice_group::{DcConfiguration, HasDc, NoDc, Op, PreOpPdi, SafeOp},
};
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::{
        Arc, OnceLock,
        atomic::Ordering,
        mpsc::{Receiver, Sender},
    },
};
use std::{
    io::Error,
    sync::atomic::AtomicUsize,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};
use tokio::{runtime::Runtime, time::interval};

use std::sync::mpsc;

// A global, lazily-initialized Runtime
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_async_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio Runtime")
    })
}

pub const ETHERCAT_TX_RX_SIZE: usize = 4096;
pub const MAX_SUBDEVICES: usize = 16;
pub const MAX_PDU_DATA: usize = PduStorage::element_size(512);
pub const MAX_FRAMES: usize = 16;
pub const PDI_LEN: usize = 1024;
static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();

#[derive(Debug)]
pub enum EtherCATState {
    NoInterface = 0,
    Boot = 1,
    Init = 2,
    PreOp = 3,
    PreopPdi = 4,
    Op = 5,
}

enum GroupState {
    PreOpNoDc(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, NoDc>),
    PreOpDc(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, HasDc>),
    SafeOp(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, SafeOp, HasDc>),
}

#[repr(align(64))]
pub struct CachePaddedAtomic(AtomicUsize);

pub struct SdoRequest<T> {
    device_address: u16,
    index: u16,
    sub_index: u16,
    value: T,
}

pub struct SdoReadRequest<T> {
    device_address: u16,
    index: u16,
    sub_index: u16,
    _p: PhantomData<T>, // need this so we can apply generics here ... little bit hacky but works great
}

// LEGACY CODE HIDE BEHIND FLAG
pub struct MachineIdent {}

// TODO: instead of using ethercrab::error:Error use something more generic like anyhow::Error
#[derive(Debug)]
pub enum ChannelResponse {
    // For simplicity every response that is unsigned -> u32
    SdoResponseU32(Result<u32, ethercrab::error::Error>),
    // And every signed  is promoted to i32
    SdoResponseI32(Result<i32, ethercrab::error::Error>),
    SdoWriteResponse(Result<(), ethercrab::error::Error>),
    ChangeState(Result<(), ethercrab::error::Error>),
}

pub enum ChannelRequests {
    // Sadly need a few variants so compiler doesnt scream here
    SdoRequestU8(SdoRequest<u8>),
    SdoRequestU16(SdoRequest<u16>),
    SdoRequestU32(SdoRequest<u32>),
    SdoRequestI16(SdoRequest<i16>),
    SdoRequestI32(SdoRequest<i32>),

    SdoReadU8(SdoReadRequest<u8>),
    SdoReadU16(SdoReadRequest<u16>),
    SdoReadU32(SdoReadRequest<u32>),
    SdoReadI16(SdoReadRequest<i16>),
    SdoReadI32(SdoReadRequest<i32>),

    MachineIdent(MachineIdent),
    ChangeState(EtherCATState),
    Shutdown(),
}

pub struct ChannelRequest {
    pub channel_request: ChannelRequests,
    pub response_channel: Option<Sender<ChannelResponse>>,
}

pub struct EtherCATController {
    pub cycle_time_us: u64,
    pub interface: Option<String>,
    pub state: EtherCATState,
    pub requested_state: Option<EtherCATState>,
    pub rx_channel: Receiver<ChannelRequest>,

    input_buffers: [UnsafeCell<[u8; ETHERCAT_TX_RX_SIZE]>; 2],
    input_read_idx: CachePaddedAtomic,

    output_buffers: [UnsafeCell<[u8; ETHERCAT_TX_RX_SIZE]>; 2],
    output_write_idx: CachePaddedAtomic, // Which one the "System" is writing to
}

unsafe impl Sync for EtherCATController {}
unsafe impl Send for EtherCATController {}

impl EtherCATController {
    /// Read latest input blob (App side)
    pub fn get_inputs(&self) -> [u8; ETHERCAT_TX_RX_SIZE] {
        let idx = self.input_read_idx.0.load(Ordering::Relaxed);
        let ptr = self.input_buffers[idx].get();
        unsafe { *ptr }
    }

    /// Write output commands (App side)
    pub fn set_outputs(&self, data: &[u8]) {
        let idx = self.output_write_idx.0.load(Ordering::Relaxed);
        let ptr = self.output_buffers[idx].get();
        unsafe {
            let buf = &mut *ptr;
            let len = data.len().min(ETHERCAT_TX_RX_SIZE);
            buf[..len].copy_from_slice(&data[..len]);
        }
        // Switch index to signal the EtherCAT thread
        self.output_write_idx.0.store(1 - idx, Ordering::Release);
    }
}

pub fn send_response(response_channel: Option<Sender<ChannelResponse>>, response: ChannelResponse) {
    match response_channel {
        Some(chan) => {
            let _ = chan.send(response);
        }
        None => (),
    };
}

/*
 Value type needs to have EtherCrabWireWrite + Copy at the least to be able to write with ethecrab
*/
pub fn sdo_write<T>(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoRequest<T>,
) -> Result<(), ethercrab::error::Error>
where
    T: EtherCrabWireWrite + Copy,
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let _res = runtime.block_on(device.sdo_write::<T>(
                request.index,
                request.sub_index as u8,
                request.value,
            ));
        }
    }
    Err(ethercrab::error::Error::UnknownSubDevice)
}

pub fn sdo_read<T>(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoReadRequest<T>,
) -> Result<T, ethercrab::error::Error>
where
    T: EtherCrabWireRead + EtherCrabWireSized + Copy,
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res =
                runtime.block_on(device.sdo_read::<T>(request.index, request.sub_index as u8));
            return res;
        }
    }
    Err(ethercrab::error::Error::UnknownSubDevice)
}

impl EtherCATController {
    pub fn ethercat_state_machine(&mut self) {
        let mut ethercat_tx_rx_handle: Result<JoinHandle<()>, Error>;
        let mut group: Option<SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>> = None;
        let mut group_preop_pdi: SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, NoDc>;
        let mut group_preop_pdi_dc: Option<
            SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, HasDc>,
        > = None;
        let mut group_op: Option<SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, Op, HasDc>> = None;
        let mut maindevice: Option<MainDevice> = None;
        println!("ECAT Controller Addr: {:p}", self);
        loop {
            match self.state {
                EtherCATState::NoInterface => {
                    if self.interface.is_some() {
                        self.state = EtherCATState::Init;
                    }
                    // Do Nothing
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
                                    ChannelResponse::ChangeState(Err(err)),
                                );
                                continue;
                            }
                        });
                        self.state = EtherCATState::PreOp;
                        send_response(msg.response_channel, ChannelResponse::ChangeState(Ok(())));
                    };
                }
                EtherCATState::PreOp => {
                    let msg = match self.rx_channel.try_recv() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    let maindev = maindevice.as_ref().unwrap();
                    let preop_group = group.as_ref().unwrap();
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
                        // enum variants suuuuuuuuck
                        ChannelRequests::SdoRequestU8(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoWriteResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::SdoRequestU16(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoWriteResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::SdoRequestU32(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoWriteResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::SdoRequestI16(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoWriteResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::SdoRequestI32(request) => {
                            let res = sdo_write(maindev, preop_group, request);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoWriteResponse(res),
                            );
                            continue;
                        }
                        ChannelRequests::SdoReadU8(request) => {
                            let res = sdo_read(maindev, preop_group, request);
                            let res_u32: Result<u32, ethercrab::error::Error> =
                                res.map(|v| v as u32);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoResponseU32(res_u32),
                            );
                            continue;
                        }
                        ChannelRequests::SdoReadU16(request) => {
                            let res = sdo_read(maindev, preop_group, request);
                            let res_u32: Result<u32, ethercrab::error::Error> =
                                res.map(|v| v as u32);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoResponseU32(res_u32),
                            );
                            continue;
                        }
                        ChannelRequests::SdoReadU32(request) => {
                            let res = sdo_read(maindev, preop_group, request);
                            let res_u32: Result<u32, ethercrab::error::Error> =
                                res.map(|v| v as u32);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoResponseU32(res_u32),
                            );
                            continue;
                        }
                        ChannelRequests::SdoReadI16(request) => {
                            let res = sdo_read(maindev, preop_group, request);
                            let res_i32: Result<i32, ethercrab::error::Error> =
                                res.map(|v| v as i32);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoResponseI32(res_i32),
                            );
                            continue;
                        }
                        ChannelRequests::SdoReadI32(request) => {
                            let res = sdo_read(maindev, preop_group, request);
                            let res_i32: Result<i32, ethercrab::error::Error> =
                                res.map(|v| v as i32);
                            send_response(
                                msg.response_channel,
                                ChannelResponse::SdoResponseI32(res_i32),
                            );
                            continue;
                        }
                        ChannelRequests::MachineIdent(machine_ident) => todo!(),
                    }

                    // Starting transition to PreopPdi
                    println!("Starting transition to PreopPdi");

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

                    println!("Moving into PRE-OP with PDI");
                    let group_to_transition = group.take().expect("Group missing in PreOp");
                    let device_ref = maindevice.as_ref().expect("MainDevice missing");
                    let rt = get_async_runtime();
                    let res = rt
                        .block_on(async { group_to_transition.into_pre_op_pdi(device_ref).await });

                    group_preop_pdi = match res {
                        Ok(group) => group,
                        Err(_) => todo!(),
                    };
                    println!("Done. PDI available. Waiting for SubDevices to align");

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
                    println!("test");
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

                                // 2. Wrap the whole sequence in one block_on so 'now' and 'sleep' share the same reactor session
                                let res = rt.block_on(async {
                                    let now = tokio::time::Instant::now(); // Moved inside
                                    let res = group.tx_rx_dc(device).await.expect("TX/RX");

                                    // Keep the logic flow, but handle the sleep here
                                    if tick <= 300 {
                                        tokio::time::sleep_until(now + res.extra.next_cycle_wait)
                                            .await;
                                    }
                                    res // return res out of the block
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
                    loop {
                        let response = rt
                            .block_on(
                                group_op
                                    .as_ref()
                                    .unwrap()
                                    .tx_rx_dc(&maindevice.as_ref().unwrap()),
                            )
                            .expect("TX/RX");
                        if response.all_op() {
                            println!("Not All OP");
                            break;
                        }
                    }

                    println!("ALL OP");
                    let group = group_op.as_ref().unwrap();

                    let maindevice = maindevice.as_ref().unwrap();
                    loop {
                        let res = rt.block_on(async {
                            let res = group.tx_rx_dc(&maindevice).await.expect("TX/RX");
                            let now = tokio::time::Instant::now();
                            (res, now)
                        });

                        // 1. Determine which buffer is NOT being read by the app right now
                        let read_idx = self.input_read_idx.0.load(Ordering::Acquire);
                        let write_idx = 1 - read_idx;

                        // 2. Get a mutable pointer to that buffer and write the data
                        let dest_ptr = self.input_buffers[write_idx].get();
                        unsafe {
                            // We get a mutable slice to the whole buffer to make sub-slicing easier
                            let full_buffer = &mut *dest_ptr;
                            let mut current_offset = 0;
                            for subdevice in group.iter(&maindevice) {
                                let len = subdevice.io_raw().inputs().len();

                                //println!("{:?}",subdevice.io_raw().inputs());
                                if current_offset + len <= ETHERCAT_TX_RX_SIZE {
                                    full_buffer[current_offset..current_offset + len]
                                        .copy_from_slice(subdevice.io_raw().inputs());
                                    //println!("{:?}",full_buffer);
                                    current_offset += len;
                                } else {
                                    println!("Data exceeds buffer");
                                    break;
                                }
                            }
                        }
                        // 3. Update the read index so the app sees the fresh buffer
                        self.input_read_idx.0.store(write_idx, Ordering::Release);
                        rt.block_on(async {
                            tokio::time::sleep_until(res.1 + res.0.extra.next_cycle_wait).await
                        });

                        /*
                        // Write Outputs
                        let out_idx = self.output_write_idx.load(Ordering::Acquire);
                        let out_read_idx = 1 - out_idx;
                        let src_ptr = self.output_buffers[out_read_idx].get();
                        unsafe {
                            // Apply commands from the buffer to the EtherCAT group
                            group.apply_output_data(&*src_ptr);
                        }*/
                    }
                }
            }
            self.requested_state = None;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

pub fn start_ethercat_thread(
    interface_name: &str,
) -> (
    (Arc<EtherCATController>, Arc<Sender<ChannelRequest>>),
    JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel();
    let controller = Arc::new(EtherCATController {
        interface: Some(interface_name.to_owned()),
        cycle_time_us: 0,
        state: EtherCATState::NoInterface,
        requested_state: None,
        rx_channel: rx,
        input_buffers: [
            UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
            UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
        ],
        input_read_idx: CachePaddedAtomic(AtomicUsize::new(0)),
        output_buffers: [
            UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
            UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
        ],
        output_write_idx: CachePaddedAtomic(AtomicUsize::new(0)),
    });
    let controller_for_thread = Arc::clone(&controller);

    let handle = std::thread::Builder::new()
        .name("EthercatStateMachine".into())
        .spawn(move || {
            // We need &mut self for the state machine.
            let ptr = Arc::as_ptr(&controller_for_thread) as *mut EtherCATController;
            unsafe {
                (&mut *ptr).ethercat_state_machine();
            }
        })
        .expect("Failed to spawn thread");

    ((controller, tx.into()), handle)
}
