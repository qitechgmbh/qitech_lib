pub mod coe;
pub mod controller;
pub mod debugging;
pub mod devices;
pub mod ethercat_helpers;
pub mod helpers;
pub mod interface_discovery;
pub mod io;
pub mod pdo;
pub mod shared_config;
//#[cfg(feature = "legacy_code")]
pub mod machine_ident_read;
use ethercrab::PduStorage;
use machine_ident_read::MachineDeviceInfo;
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, OnceLock, mpsc::Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex;
use triple_buffer::{Input, Output};

// A global, lazily-initialized Runtime
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
fn get_async_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Builder::new_current_thread() // Use single-threaded for determinism
            .event_interval(u32::MAX) // Never check I/O drivers automatically based on task ticks
            .global_queue_interval(u32::MAX) // Never check global queues automatically            // Optional: Limit how many events tokio processes before checking IO
            // .max_event_per_tick(64)
            .build()
            .expect("Failed to create Tokio Runtime")
    })
}

pub const BECKHOFF_VENDOR_ID: u32 = 0x2;
pub const ETHERCAT_TX_RX_SIZE: usize = 4096;
pub const MAX_SUBDEVICES: usize = 32;
pub const MAX_PDU_DATA: usize = PduStorage::element_size(512);
pub const MAX_FRAMES: usize = 32;
pub const PDI_LEN: usize = 1024;

static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();
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
        config: MasterConfiguration,
        cycle: Arc<AtomicU64>,
        cycle_time_us: Arc<AtomicU64>,
        subdevice_count: Arc<AtomicU64>,
        dc_system_time_ns: Arc<AtomicU64>,
        state: Arc<AtomicU8>,
        subdevices: Arc<Mutex<[MetaSubdevice; MAX_SUBDEVICES]>>,
        all_subdevices_operational: Arc<AtomicBool>,
        inputs_ready: Arc<AtomicBool>,
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
            input_producer: input,
            output_consumer: output,
            current_config: config,
            all_subdevices_operational,
            dc_system_time_ns,
            inputs_ready,
        }
    }
}

pub trait Consumer {
    fn read(&mut self) -> Option<&[u8]>;
    fn finish_read(&mut self);
}

pub trait Producer {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]>;
    fn publish(&mut self);
}

pub struct MockConsumer {
    pub buffer: [u8; ETHERCAT_TX_RX_SIZE],
}

pub struct MockProducer {
    pub buffer: [u8; ETHERCAT_TX_RX_SIZE],
}

impl Producer for MockProducer {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        Some(&mut self.buffer)
    }

    fn publish(&mut self) {
        // does nothing for the mock
    }
}

impl Consumer for MockConsumer {
    fn read(&mut self) -> Option<&[u8]> {
        Some(&self.buffer)
    }
    fn finish_read(&mut self) {}
}

unsafe impl Sync for Mailbox {}
unsafe impl Send for Mailbox {}

pub struct Mailbox {
    pub data: UnsafeCell<[u8; ETHERCAT_TX_RX_SIZE]>,
    pub full: AtomicBool,
}

impl Consumer for std::sync::Arc<Mailbox> {
    fn read(&mut self) -> Option<&[u8]> {
        // Cast the shared Arc pointer to a mutable reference to Mailbox
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe { (&mut *ptr).read() }
    }

    fn finish_read(&mut self) {
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe {
            (&mut *ptr).finish_read();
        }
    }
}

impl Producer for std::sync::Arc<Mailbox> {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe { (&mut *ptr).input_buffer_mut() }
    }

    fn publish(&mut self) {
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe {
            (&mut *ptr).publish();
        }
    }
}

impl Consumer for Mailbox {
    fn read(&mut self) -> Option<&[u8]> {
        // Consumer only reads if `full` is true
        if !self.full.load(Ordering::Acquire) {
            return None;
        }
        unsafe { Some(&*self.data.get()) }
    }

    fn finish_read(&mut self) {
        // We are completely done reading.
        // We store `false` to release the buffer back to the producer.
        self.full.store(false, Ordering::Release);
    }
}

impl Producer for Mailbox {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        // Producer only writes if `full` is false
        if self.full.load(Ordering::Acquire) {
            return None;
        }
        unsafe { Some(&mut *self.data.get()) }
    }

    fn publish(&mut self) {
        // We are completely done writing.
        // We store `true` to release the buffer to the consumer.
        self.full.store(true, Ordering::Release);
    }
}

pub struct TripleBufConsumer {
    pub input_consumer: Output<[u8; ETHERCAT_TX_RX_SIZE]>,
}

pub struct TripleBufProducer {
    pub output_producer: Input<[u8; ETHERCAT_TX_RX_SIZE]>,
}

impl Consumer for TripleBufConsumer {
    fn read(&mut self) -> Option<&[u8]> {
        Some(self.input_consumer.read())
    }
    fn finish_read(&mut self) {}
}

impl Producer for TripleBufProducer {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        Some(self.output_producer.input_buffer_mut())
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
    cycle: Arc<AtomicU64>,
    cycle_time_us: Arc<AtomicU64>,
    next_cycle_us: Arc<AtomicU64>,
    subdevice_count: Arc<AtomicU64>,
    state: Arc<AtomicU8>,
    subdevices: Arc<Mutex<[MetaSubdevice; MAX_SUBDEVICES]>>,
    all_op: Arc<AtomicBool>,
    dc_sys_time: Arc<AtomicU64>,
    inputs_ready: Arc<AtomicBool>,
}

impl<C, P> EtherCATAppHandle<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub fn check_all_op(&self) -> bool {
        self.all_op.load(Relaxed)
    }

    pub fn get_dc_sys_time_ns(&self) -> u64 {
        self.dc_sys_time.load(Relaxed)
    }

    pub fn get_inputs(&mut self) -> Option<&[u8]> {
        self.input_consumer.read()
    }

    pub fn finish_read(&mut self) {
        self.input_consumer.finish_read();
    }

    pub fn write_outputs(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        self.output_producer.input_buffer_mut()
    }

    pub fn send_outputs(&mut self) {
        self.output_producer.publish();
    }

    pub fn get_current_cycle(&self) -> u64 {
        self.cycle.load(Relaxed)
    }

    pub fn get_cycle_time_us(&self) -> u64 {
        self.cycle_time_us.load(Relaxed)
    }

    pub fn get_next_cycle_us(&self) -> u64 {
        self.next_cycle_us.load(Relaxed)
    }

    pub fn get_subdevice_count(&self) -> u64 {
        self.subdevice_count.load(Relaxed)
    }

    pub fn get_state(&self) -> EtherCATState {
        self.state.load(Relaxed).into()
    }

    pub fn check_inputs_ready(&self) -> bool {
        self.inputs_ready.load(Relaxed)
    }

    pub fn try_get_subdevices_vec_sync(&self) -> Result<Vec<MetaSubdevice>, anyhow::Error> {
        let unlocked = self.subdevices.blocking_lock();
        let count = self.get_subdevice_count() as usize;
        Ok(unlocked.clone()[0..count].to_vec())
    }

    pub async fn try_get_subdevices_vec(&self) -> Result<Vec<MetaSubdevice>, anyhow::Error> {
        let unlocked = self.subdevices.lock().await;
        let count = self.get_subdevice_count() as usize;
        Ok(unlocked.clone()[0..count].to_vec())
    }
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Clone)]
pub struct SdoIndex {
    index: u32,
    sub_index: u16,
}

#[cfg(feature = "mock")]
#[derive(Clone)]
pub struct TypeErasedValue {
    type_id: std::any::TypeId,
    value: Vec<u8>,
}

// Wrapper to easily refactor later on
#[cfg(not(feature = "mock"))]
#[derive(Clone)]
pub struct EtherCATThreadChannel(pub Sender<ChannelRequest>);

#[cfg(feature = "mock")]
#[derive(Clone)]
pub struct EtherCATThreadChannel {
    pub sdo_map: std::collections::HashMap<SdoIndex, TypeErasedValue>,
    pub machine_device_infos: Vec<MachineDeviceInfo>,
}

#[derive(Clone)]
pub struct EtherCATThreadResponseChannel(pub Sender<ChannelResponse>);

pub struct EtherCATControl<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub channel: EtherCATThreadChannel,
    pub app_handle: EtherCATAppHandle<C, P>,
    pub join_handle: Option<JoinHandle<Result<(), anyhow::Error>>>,
}

pub type StdEcatHandle = EtherCATAppHandle<TripleBufConsumer, Arc<Mailbox>>;
pub type MockEcatHandle = EtherCATAppHandle<MockConsumer, MockProducer>;
pub type StdEcatController = EtherCATController<Arc<Mailbox>, TripleBufProducer>;

/*Metadata for a Subdevice Contains start and end of the given subdevices pdu*/
#[derive(Clone, Copy, Debug)]
pub struct MetaSubdevice {
    pub name: [u8; 128],
    pub product_id: u32,
    pub revision: u32,
    pub vendor: u32,
    // Gives the offset at which the TxPdo starts
    pub start_tx: usize,
    pub end_tx: usize,
    // Gives the offset at which the RxPdo starts
    pub start_rx: usize,
    pub end_rx: usize,
    // Device address first one would be 0x1000, so 4096
    pub device_address: u16,
    pub initialized: bool,
}

impl MetaSubdevice {
    pub fn get_name(&self) -> Result<String, anyhow::Error> {
        let trimmed = self
            .name
            .iter()
            .take_while(|&&b| b != b'\0')
            .cloned()
            .collect::<Vec<u8>>();
        Ok(String::from_utf8(trimmed)?)
    }
}

impl Default for MetaSubdevice {
    fn default() -> Self {
        Self {
            name: [0u8; 128],
            product_id: Default::default(),
            revision: Default::default(),
            vendor: Default::default(),
            start_tx: Default::default(),
            end_tx: Default::default(),
            start_rx: Default::default(),
            end_rx: Default::default(),
            device_address: Default::default(),
            initialized: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum EtherCATState {
    NoInterface = 0,
    Boot = 1,
    Init = 2,
    PreOp = 3,
    PreopPdi = 4,
    Op = 5,
}

impl From<u8> for EtherCATState {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Boot,
            2 => Self::Init,
            3 => Self::PreOp,
            4 => Self::PreopPdi,
            5 => Self::Op,
            _ => EtherCATState::NoInterface,
        }
    }
}

impl From<EtherCATState> for u8 {
    fn from(value: EtherCATState) -> Self {
        match value {
            EtherCATState::Boot => 1,
            EtherCATState::Init => 2,
            EtherCATState::PreOp => 3,
            EtherCATState::PreopPdi => 4,
            EtherCATState::Op => 5,
            _ => 6,
        }
    }
}

#[derive(Debug)]
pub enum SdoType {
    BOOL,
    U8,
    U16,
    U32,
    I16,
    I32,
}

#[derive(Debug)]
pub struct SdoRequest {
    pub device_address: u16,
    pub index: u16,
    pub sub_index: u16,
    pub data: [u8; 4],
    pub type_flag: SdoType,
}

#[derive(Debug)]
pub struct SdoReadRequest {
    pub device_address: u16,
    pub index: u16,
    pub sub_index: u16,
    pub type_flag: SdoType,
}

/// A raw ESC register read request. Registers relevant for diagnosing state-transition failures
#[derive(Debug)]
pub struct RegisterReadRequest {
    pub device_address: u16,
    pub register: u16,
}

// LEGACY CODE HIDE BEHIND FLAG
pub struct MachineIdent {}

#[derive(Debug)]
pub enum ChannelResponse {
    SdoResponseBool(Result<bool, anyhow::Error>),
    SdoResponseU8(Result<u8, anyhow::Error>),
    SdoResponseU16(Result<u16, anyhow::Error>),
    SdoResponseU32(Result<u32, anyhow::Error>),
    SdoResponseI16(Result<i16, anyhow::Error>),
    SdoResponseI32(Result<i32, anyhow::Error>),
    SdoWriteResponse(Result<(), anyhow::Error>),
    RegisterResponseU16(Result<u16, anyhow::Error>),
    ChangeState(Result<(), anyhow::Error>),
    MachineDeviceInfoResponse(Result<Vec<MachineDeviceInfo>, anyhow::Error>),
    WriteMachineInfoResponse(Result<(), anyhow::Error>),
    EnableDCSync0Response(Result<(), anyhow::Error>),
    EnableDCSync01Response(Result<(), anyhow::Error>),
    ConfigureOversamplingResponse(Result<(), anyhow::Error>),
}

#[derive(Debug)]
pub enum ChannelRequests {
    SdoWriteRequest(SdoRequest),
    SdoReadRequest(SdoReadRequest),
    RegisterReadRequest(RegisterReadRequest),
    ChangeState(EtherCATState),
    // usize in this case is the device_address
    EnableDCSync0(usize),
    Shutdown(),
    // Legacy code, only usable when feature legacy_code is set
    ReadMachineIdent(),
    EnableDCSync01(usize, Duration),
    ConfigureOversampling(usize, Vec<(u16, u16)>),
    WriteMachineIdent(Vec<MachineDeviceInfo>),
}

pub struct ChannelRequest {
    pub channel_request: ChannelRequests,
    pub response_channel: EtherCATThreadResponseChannel,
}

/// Driver-agnostic encoder resolution returned from device configuration.
#[derive(Clone, Copy, Debug)]
pub struct EncoderResolution {
    pub increments: u32,
    pub revolutions: u32,
}

pub fn send_response(response_channel: EtherCATThreadResponseChannel, response: ChannelResponse) {
    let _res = response_channel.0.send(response);
}

#[cfg(feature = "mock")]
pub fn init_ethercat_mock(
    faked_subdevices: Vec<MetaSubdevice>,
    _machine_infos: Option<Vec<MachineDeviceInfo>>,
) -> EtherCATControl<MockConsumer, MockProducer> {
    let (_, rx) = mpsc::channel(); // wont actually get used in any way, just here to avoid handling options in the controller ...
    let mock_producer = [0u8; ETHERCAT_TX_RX_SIZE];
    let mock_consumer = [0u8; ETHERCAT_TX_RX_SIZE];

    let producer = MockProducer {
        buffer: mock_producer,
    };
    let consumer = MockConsumer {
        buffer: mock_consumer,
    };

    let producer_c = MockProducer {
        buffer: [0u8; ETHERCAT_TX_RX_SIZE],
    };
    let consumer_c = MockConsumer {
        buffer: [0u8; ETHERCAT_TX_RX_SIZE],
    };

    let channel: EtherCATThreadChannel = EtherCATThreadChannel {
        sdo_map: std::collections::HashMap::new(),
        machine_device_infos: vec![],
    };
    let app_handle = EtherCATAppHandle {
        input_consumer: consumer,
        output_producer: producer,
    };

    let mut controller = EtherCATController::new(
        producer_c,
        consumer_c,
        rx,
        None,
        MasterConfiguration::default(),
    );

    controller.subdevice_count = faked_subdevices.len();
    for i in 0..faked_subdevices.len() {
        controller.subdevices[i] = faked_subdevices[i];
    }

    let controller = Arc::new(controller);
    return EtherCATControl {
        controller,
        channel,
        app_handle,
        join_handle: None,
    };
}

#[cfg(target_os = "linux")]
pub fn set_current_thread_rt_priority(priority: i32) {
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
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn set_current_thread_rt_priority(_priority: i32) {
    eprintln!(
        "set_current_thread_rt_priority: real-time scheduling is not available on this platform"
    );
}

// Currently ignored by controller, but should be used to select driver for txrx
// XDP should in theory be most performant
// While tx_rx_blocking is supported on most platforms
// Which of these actually work depend on the controller.rs used (ethercrab supports xdp io_uring and the standard one)
#[derive(Clone, Debug)]
pub enum MasterTxRxConfig {
    TxRxBlocking,
    TxRxIoUring,
    //TX_RX_XDP
}

#[derive(Clone, Debug)]
pub struct DcConfiguration {
    pub start_delay: Duration,
    pub sync0_period: Duration,
    pub sync0_shift: Duration,
    pub target_dc_tick: usize,
}

impl Default for DcConfiguration {
    fn default() -> Self {
        Self {
            start_delay: Duration::from_millis(100),
            sync0_period: Duration::from_micros(1000),
            sync0_shift: Duration::from_micros(500),
            target_dc_tick: 300,
        }
    }
}
#[derive(Clone, Debug)]
pub struct RtOptimizationConfig {
    // Pinning to core 0 also might not be optimal on Linux
    // for optimal performance cores should not be shared,except the irq core
    pub ethercat_loop_thread_core: usize,
    pub ethercat_loop_thread_priority: i32,
    pub ethercat_io_thread_core: usize,
    pub ethercat_io_thread_priority: i32,
    // If none irq is not pinned to a core
    pub pin_irq_core: Option<usize>,
    pub lock_memory: bool,
}

impl Default for RtOptimizationConfig {
    fn default() -> Self {
        Self {
            ethercat_loop_thread_core: Default::default(),
            ethercat_loop_thread_priority: Default::default(),
            ethercat_io_thread_core: Default::default(),
            ethercat_io_thread_priority: Default::default(),
            pin_irq_core: None,
            lock_memory: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MasterConfiguration {
    /// target cycle time in Microseconds
    pub target_cycle_time_us: usize,
    pub tx_rx_config: MasterTxRxConfig,
    pub realtime_optimizations: Option<RtOptimizationConfig>,
    pub dc_config: DcConfiguration,
    pub wkc_mismatch_threshold: u32,
    pub op_ramp_grace_cycles: u32,
}

impl Default for MasterConfiguration {
    fn default() -> Self {
        Self {
            target_cycle_time_us: 1000,
            tx_rx_config: MasterTxRxConfig::TxRxIoUring,
            dc_config: DcConfiguration::default(),
            realtime_optimizations: None,
            wkc_mismatch_threshold: 5,
            op_ramp_grace_cycles: 10000,
        }
    }
}

#[cfg(not(feature = "mock"))]
pub fn init_ethercat(
    interface_name: &str,
    config: Option<MasterConfiguration>,
) -> EtherCATControl<TripleBufConsumer, Arc<Mailbox>> {
    use std::sync::atomic::AtomicU8;
    let (tx, rx) = mpsc::channel();
    let mailbox = Arc::new(Mailbox {
        data: [0u8; ETHERCAT_TX_RX_SIZE].into(),
        full: AtomicBool::new(false),
    });
    let cycle: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let cycle_time_us: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let next_cycle_us: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let subdevice_count: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let state: Arc<AtomicU8> = Arc::new(AtomicU8::new(EtherCATState::NoInterface.into()));
    let all_op: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let dc_sys_time: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let subdevices: Arc<Mutex<[MetaSubdevice; MAX_SUBDEVICES]>> =
        Arc::new(Mutex::new([MetaSubdevice::default(); MAX_SUBDEVICES]));
    let inputs_ready = Arc::new(AtomicBool::new(false));

    // input refers to ethercat tx
    // which is consumed by clientside code and produced by the controller
    let (input_producer, input_consumer) =
        triple_buffer::triple_buffer(&[0u8; ETHERCAT_TX_RX_SIZE]);

    let mut controller = match config {
        Some(conf) => EtherCATController::new(
            TripleBufProducer {
                output_producer: input_producer,
            },
            mailbox.clone(),
            rx,
            Some(interface_name.to_string()),
            conf,
            cycle.clone(),
            cycle_time_us.clone(),
            subdevice_count.clone(),
            dc_sys_time.clone(),
            state.clone(),
            subdevices.clone(),
            all_op.clone(),
            inputs_ready.clone(),
        ),
        None => EtherCATController::new(
            TripleBufProducer {
                output_producer: input_producer,
            },
            mailbox.clone(),
            rx,
            Some(interface_name.to_string()),
            MasterConfiguration::default(),
            cycle.clone(),
            cycle_time_us.clone(),
            subdevice_count.clone(),
            dc_sys_time.clone(),
            state.clone(),
            subdevices.clone(),
            all_op.clone(),
            inputs_ready.clone(),
        ),
    };

    let app_handle = EtherCATAppHandle {
        input_consumer: TripleBufConsumer { input_consumer },
        output_producer: mailbox,
        cycle,
        cycle_time_us,
        next_cycle_us,
        subdevice_count,
        state,
        subdevices,
        all_op,
        dc_sys_time,
        inputs_ready,
    };

    let channel: EtherCATThreadChannel = EtherCATThreadChannel(tx);
    let join_handle = std::thread::Builder::new()
        .name("EthercatStateMachine".into())
        .spawn(move || controller.ethercat_state_machine())
        .expect("Failed to spawn thread");
    EtherCATControl {
        channel,
        app_handle,
        join_handle: Some(join_handle),
    }
}
