pub mod helpers;
pub mod controller;
use crate::controller::EtherCATController;

use ethercrab::{
    PduStorage, SubDeviceGroup, subdevice_group::{HasDc, NoDc, PreOpPdi, SafeOp},
};
use std::{
    cell::UnsafeCell,
    sync::{
        Arc, OnceLock,
        mpsc::Sender,
    },
};
use std::{
    sync::atomic::AtomicUsize,
    thread::JoinHandle,
};
use tokio::runtime::Runtime;
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

// Wrapper to easily refactor later on
#[derive(Clone)]
pub struct EtherCATThreadChannel(pub Sender<ChannelRequest>);
#[derive(Clone)]
pub struct EtherCATThreadResponseChannel(pub Sender<ChannelResponse>);

/*
    Metadata for a Subdevice 
    Contains start and end of the given subdevices pdu
*/
#[derive(Copy,Clone,Debug,Default)]
pub struct MetaSubdevice {
    pub product_id : u32,
    pub revision : u32,
    pub vendor : u16,
    // Gives the offset at which the TxPdo starts
    pub start_tx : usize,
    pub end_tx : usize,
    // Gives the offset at which the RxPdo starts
    pub start_rx : usize,
    pub end_rx : usize,

    pub initialized : bool,
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

enum GroupState {
    PreOpNoDc(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, NoDc>),
    PreOpDc(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, PreOpPdi, HasDc>),
    SafeOp(SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN, SafeOp, HasDc>),
}

#[repr(align(64))]
pub struct CachePaddedAtomic(AtomicUsize);

pub enum SdoType {
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
    pub data: [u8;4],
    pub type_flag : SdoType,
}

pub struct SdoReadRequest {
    pub device_address: u16,
    pub index: u16,
    pub sub_index: u16,
    pub type_flag : SdoType,
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
    SdoWriteRequest(SdoRequest),
    SdoReadRequest(SdoReadRequest),
    MachineIdent(MachineIdent),
    ChangeState(EtherCATState),
    Shutdown(),
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
    (Arc<EtherCATController>, EtherCATThreadChannel),
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
        subdevices: [MetaSubdevice::default();256],
        subdevice_count: 0,
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

    ((controller, EtherCATThreadChannel(tx) ), handle)
}
