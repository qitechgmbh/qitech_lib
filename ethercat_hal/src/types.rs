use crate::{
    ETHERCAT_TX_RX_SIZE, al_diagnostics, app_handle::EtherCATAppHandle,
    machine_ident_read::MachineDeviceInfo, mailbox::Mailbox,
};
use std::{fmt, sync::Arc, time::Duration};
use triple_buffer::{Input, Output};

#[derive(Clone)]
pub struct EtherCATThreadResponseChannel(pub crate::Sender<ChannelResponse>);
pub type StdEcatHandle = EtherCATAppHandle<TripleBufConsumer, Arc<Mailbox>>;
pub type MockEcatHandle = EtherCATAppHandle<MockConsumer, MockProducer>;

#[derive(Debug)]
pub enum EthercatErr {
    Custom(String),
    PreopTransitionFailed,
    SafeopTransitionFailed,
    OpTransitionFailed,
    WorkingCounterErr,
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
// `.0` is drained per-state, `.1` in every state.
#[cfg(not(feature = "mock"))]
#[derive(Clone)]
pub struct EtherCATThreadChannel(
    pub crate::Sender<ChannelRequest>,
    pub crate::Sender<DiagnosticRequest>,
);

#[cfg(feature = "mock")]
#[derive(Clone)]
pub struct EtherCATThreadChannel {
    pub sdo_map: std::collections::HashMap<SdoIndex, TypeErasedValue>,
    pub machine_device_infos: Vec<MachineDeviceInfo>,
}

/// Metadata for a Subdevice Contains start and end of the given subdevices pdu
#[derive(Clone, Copy)]
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

impl fmt::Display for EthercatErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EthercatErr::Custom(msg) => write!(f, "Ethercat-error: {}", msg),
            EthercatErr::PreopTransitionFailed => write!(f, "Failed to transition to PREOP state"),
            EthercatErr::SafeopTransitionFailed => {
                write!(f, "Failed to transition to SAFEOP state")
            }
            EthercatErr::OpTransitionFailed => write!(f, "Failed to transition to OP state"),
            EthercatErr::WorkingCounterErr => write!(f, "Working counter error (WKC)"),
        }
    }
}
impl std::error::Error for EthercatErr {}

impl std::fmt::Debug for MetaSubdevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaSubdevice")
            .field(
                "name",
                &self.get_name().unwrap_or("<non-uft8-name>".to_string()),
            )
            .field("product_id", &self.product_id)
            .field("revision", &self.revision)
            .field("vendor", &self.vendor)
            .field("start_tx", &self.start_tx)
            .field("end_tx", &self.end_tx)
            .field("start_rx", &self.start_rx)
            .field("end_rx", &self.end_rx)
            .field("device_address", &self.device_address)
            .field("initialized", &self.initialized)
            .finish()
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A diagnostic read, serviced from any master state.
///
/// Answered with raw `FPRD` reads needing only the `MainDevice`, so unlike [`ChannelRequests`]
/// these do not depend on holding a `SubDeviceGroup`.
#[derive(Debug)]
pub enum DiagnosticRequest {
    RegisterRead {
        device_address: u16,
        register: u16,
        response_channel: crate::Sender<DiagnosticResponse>,
    },
    AlStatusSnapshot {
        response_channel: crate::Sender<DiagnosticResponse>,
    },
}

#[derive(Debug)]
pub enum DiagnosticResponse {
    RegisterReadResponse(Result<u16, anyhow::Error>),
    AlStatusSnapshotResponse(Vec<al_diagnostics::SubDeviceAlStatus>),
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
    WriteMachineInfoResponse(Result<(), anyhow::Error>),
    EnableDCSync0Response(Result<(), anyhow::Error>),
    EnableDCSync01Response(Result<(), anyhow::Error>),
    ConfigureOversamplingResponse(Result<(), anyhow::Error>),
}

#[derive(Debug)]
pub enum ChannelRequests {
    SdoWriteRequest(SdoRequest),
    SdoReadRequest(SdoReadRequest),
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
