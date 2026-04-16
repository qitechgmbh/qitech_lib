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
use controller::{EtherCATAppHandle, MockConsumer, MockProducer, TripleBufConsumer, TripleBufProducer};
use ethercrab::PduStorage;
use machine_ident_read::MachineDeviceInfo;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, OnceLock, mpsc::Sender};
use std::thread::JoinHandle;
use tokio::runtime::Runtime;
pub use controller::Consumer;
pub use controller::Producer;
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

#[derive(Hash,Eq,PartialEq,PartialOrd,Clone)]
struct SdoIndex {
    index : u32,
    sub_index : u16,
}

#[derive(Clone)]
struct TypeErasedValue {
    type_id : TypeId,
    value : Vec<u8>,
}

// Wrapper to easily refactor later on
#[cfg(not(feature = "mock"))]
#[derive(Clone)]
pub struct EtherCATThreadChannel(pub Sender<ChannelRequest>);

#[cfg(feature = "mock")]
#[derive(Clone)]
pub struct EtherCATThreadChannel{
    pub sdo_map : std::collections::HashMap<SdoIndex,TypeErasedValue>,
    pub machine_device_infos : Vec<MachineDeviceInfo>
}

#[derive(Clone)]
pub struct EtherCATThreadResponseChannel(pub Sender<ChannelResponse>);
pub struct EtherCATControl<C,P> where C : Consumer, P: Producer {
    pub controller: Arc<EtherCATController<C,P>>,
    pub channel: EtherCATThreadChannel,
    pub app_handle: EtherCATAppHandle<C,P>,
    pub join_handle: Option<JoinHandle<()>>,
}

pub type StandardEtherCATAppHandle = EtherCATAppHandle<TripleBufConsumer, TripleBufProducer>;
pub type MockEtherCATAppHandle = EtherCATAppHandle<MockConsumer,MockProducer>;
pub type StandardEtherCATController = EtherCATController<TripleBufConsumer,TripleBufProducer>;

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
    // Device address (ado, i think), first one would be 0x1000, so 4096
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

#[derive(Debug)]
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


#[cfg(feature = "mock")]
pub fn init_ethercat_mock(faked_subdevices : Vec<MetaSubdevice>, machine_infos : Option<Vec<MachineDeviceInfo>>) -> EtherCATControl<MockConsumer,MockProducer> {
    let (_, rx) = mpsc::channel(); // wont actually get used in any way, just here to avoid handling options in the controller ...
    let mock_producer = [0u8;ETHERCAT_TX_RX_SIZE];
    let mock_consumer = [0u8;ETHERCAT_TX_RX_SIZE];
    
    let producer = MockProducer{ buffer: mock_producer };
    let consumer = MockConsumer{ buffer: mock_consumer };
    
    let producer_c = MockProducer{ buffer: [0u8;ETHERCAT_TX_RX_SIZE] };
    let consumer_c = MockConsumer{ buffer: [0u8;ETHERCAT_TX_RX_SIZE] };

    let channel: EtherCATThreadChannel = EtherCATThreadChannel { sdo_map: HashMap::new(), machine_device_infos: vec![] };
    let app_handle = EtherCATAppHandle {
        input_consumer: consumer,
        output_producer: producer,
    };

    let mut controller = EtherCATController::new(
        producer_c,
        consumer_c,
        rx,
        None,
    );

    controller.subdevice_count = faked_subdevices.len();    
    for i in 0..faked_subdevices.len() {
        controller.subdevices[i] = faked_subdevices[i];
    }

    let controller = Arc::new(controller);
    return EtherCATControl { controller, channel, app_handle, join_handle: None }
}

#[cfg(not(feature = "mock"))]
pub fn init_ethercat(interface_name: &str) -> EtherCATControl<TripleBufConsumer,TripleBufProducer> {
    let (tx, rx) = mpsc::channel();

    let (input_producer, input_consumer) =
        triple_buffer::triple_buffer(&[0u8; ETHERCAT_TX_RX_SIZE]);
    let (output_producer, output_consumer) =
        triple_buffer::triple_buffer(&[0u8; ETHERCAT_TX_RX_SIZE]);

    let controller = Arc::new(EtherCATController::new(
        TripleBufProducer{output_producer:  input_producer},
        TripleBufConsumer{input_consumer: output_consumer } ,
        rx,
        Some(interface_name.to_string()),
    ));

    let app_handle = EtherCATAppHandle {
        input_consumer: TripleBufConsumer{ input_consumer },
        output_producer: TripleBufProducer { output_producer } ,
 
    };
        let channel: EtherCATThreadChannel = EtherCATThreadChannel(tx);
        let controller_for_thread = Arc::clone(&controller);
        let join_handle = std::thread::Builder::new()
            .name("EthercatStateMachine".into())
            .spawn(move || {
                // We need &mut self for the state machine.
                let ptr = Arc::as_ptr(&controller_for_thread) as *mut EtherCATController<TripleBufConsumer,TripleBufProducer>;
                unsafe {
                    (&mut *ptr).ethercat_state_machine();
                }
            })
        .expect("Failed to spawn thread");
        EtherCATControl {
            controller: controller,
            channel,
            app_handle,
            join_handle: Some(join_handle),
        }
}
