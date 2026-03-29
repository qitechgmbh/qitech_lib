pub mod coe;
pub mod controller;
pub mod debugging;
pub mod devices;
pub mod ethercat_helpers;
pub mod helpers;
pub mod io;
pub mod pdo;
pub mod shared_config;

//#[cfg(feature = "legacy_code")]
pub mod machine_ident_read;

use crate::controller::EtherCATController;
use controller::EtherCATAppHandle;
use ethercrab::PduStorage;
use machine_ident_read::MachineDeviceInfo;
use std::sync::mpsc;
use std::{
    sync::{Arc, OnceLock, mpsc::Sender},
};
use std::{thread::JoinHandle};
use tokio::runtime::Runtime;
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

// Wrapper to easily refactor later on
#[derive(Clone)]
pub struct EtherCATThreadChannel(pub Sender<ChannelRequest>);
#[derive(Clone)]
pub struct EtherCATThreadResponseChannel(pub Sender<ChannelResponse>);

/*
    Metadata for a Subdevice
    Contains start and end of the given subdevices pdu
*/
#[derive(Copy, Clone, Debug, Default)]
pub struct MetaSubdevice {
    pub product_id: u32,
    pub revision: u32,
    pub vendor: u32,
    // Gives the offset at which the TxPdo starts
    pub start_tx: usize,
    pub end_tx: usize,
    // Gives the offset at which the RxPdo starts
    pub start_rx: usize,
    pub end_rx: usize,
    // Device address (ado, i think), first one would be 0x1000, so 4096
    pub device_address: u16,
    pub initialized: bool,
}

#[derive(Debug)]
pub enum EtherCATState {
    NoInterface = 0,
    Boot = 1,
    Init = 2,
    PreOp = 3,
    PreopPdi = 4,
    Op = 5,
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

pub struct SdoRequest {
    pub device_address: u16,
    pub index: u16,
    pub sub_index: u16,
    pub data: [u8; 4],
    pub type_flag: SdoType,
}

pub struct SdoReadRequest {
    pub device_address: u16,
    pub index: u16,
    pub sub_index: u16,
    pub type_flag: SdoType,
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
    ChangeState(Result<(), anyhow::Error>),
    MachineDeviceInfoResponse(Result<Vec<MachineDeviceInfo>, anyhow::Error>),
    EnableDCSync0Response(Result<(), anyhow::Error>),
}

pub enum ChannelRequests {
    SdoWriteRequest(SdoRequest),
    SdoReadRequest(SdoReadRequest),
    ChangeState(EtherCATState),
    // usize in this case is the device_address
    EnableDCSync0(usize),
    Shutdown(),
    // Legacy code, only usable when feature enable_legacy_code is set
    ReadMachineIdent(),
}

pub struct ChannelRequest {
    pub channel_request: ChannelRequests,
    pub response_channel: EtherCATThreadResponseChannel,
}

pub fn send_response(response_channel: EtherCATThreadResponseChannel, response: ChannelResponse) {
    let _res = response_channel.0.send(response);
}

pub fn start_ethercat_thread(
    interface_name: &str,
) -> (
    (Arc<EtherCATController>,EtherCATAppHandle, EtherCATThreadChannel),
    JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel();

    let (input_producer, input_consumer) = triple_buffer::triple_buffer(&[0u8; ETHERCAT_TX_RX_SIZE]);
    let (output_producer, output_consumer) = triple_buffer::triple_buffer(&[0u8; ETHERCAT_TX_RX_SIZE]);
    
    let controller = Arc::new(EtherCATController {
        interface: Some(interface_name.to_owned()),
        cycle_time_us: 0,
        state: EtherCATState::NoInterface,
        requested_state: None,
        rx_channel: rx,        
        subdevices: [MetaSubdevice::default(); 256],
        subdevice_count: 0,
        input_producer,
        output_consumer,
    });

    let app_handle = EtherCATAppHandle {
        input_consumer,
        output_producer,
    };
    
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

    ((controller, app_handle ,EtherCATThreadChannel(tx)), handle)
}
