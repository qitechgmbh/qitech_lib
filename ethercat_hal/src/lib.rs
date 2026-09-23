pub mod al_diagnostics;
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
mod app_handle;
mod ethercat_control;
pub mod machine_ident_read;
mod mailbox;
mod types;
use crate::app_handle::EtherCATAppHandle;
#[cfg(not(feature = "mock"))]
use crate::controller::EtherCATController;
#[cfg(not(feature = "mock"))]
use crate::ethercat_control::EtherCATControl;
use crate::mailbox::Mailbox;
#[cfg(not(feature = "mock"))]
use crate::types::{EtherCATState, EtherCATThreadChannel, MetaSubdevice, TripleBufConsumer, TripleBufProducer};
use crate::types::{ChannelResponse, EtherCATThreadResponseChannel};
use al_diagnostics::TransitionLog;
use ethercrab::PduStorage;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc::{self};
use std::sync::{Arc, OnceLock, mpsc::Sender};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex;

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
    let (diagnostic_tx, diagnostic_rx) = mpsc::channel();
    let transition_log = TransitionLog::new();
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
            diagnostic_rx,
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
            transition_log.clone(),
        ),
        None => EtherCATController::new(
            TripleBufProducer {
                output_producer: input_producer,
            },
            mailbox.clone(),
            rx,
            diagnostic_rx,
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
            transition_log.clone(),
        ),
    };

    let app_handle = EtherCATAppHandle::new(
        TripleBufConsumer { input_consumer },
        mailbox,
        cycle,
        cycle_time_us,
        next_cycle_us,
        subdevice_count,
        state,
        subdevices,
        all_op,
        dc_sys_time,
        inputs_ready,
        transition_log,
    );

    let channel: EtherCATThreadChannel = EtherCATThreadChannel(tx, diagnostic_tx);
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
