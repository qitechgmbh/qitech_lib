use ethercrab::{
    MainDevice, MainDeviceConfig, PduLoop, PduRx, PduStorage, PduTx, RegisterAddress,
    RetryBehaviour, SubDeviceGroup, Timeouts, TxRxResponse,
    std::ethercat_now,
    subdevice_group::{CycleInfo, DcConfiguration, HasDc, NoDc, Op, PreOpPdi, SafeOp},
};
use std::{cell::UnsafeCell, sync::{Arc, OnceLock, atomic::Ordering}};
use std::thread::Builder;
use std::{
    io::Error,
    sync::atomic::AtomicUsize,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use ta::{Next, indicators::ExponentialMovingAverage};
use tokio::{
    runtime::Runtime,
    time::{interval, sleep_until},
};

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

pub struct EtherCATController {
    pub cycle_time_us: u64,
    pub interface: Option<String>,
    pub state: EtherCATState,
    pub requested_state: Option<EtherCATState>,
    input_buffers: [UnsafeCell<[u8; ETHERCAT_TX_RX_SIZE]>; 2],
    input_read_idx: AtomicUsize,

    output_buffers: [UnsafeCell<[u8; ETHERCAT_TX_RX_SIZE]>; 2],
    output_write_idx: AtomicUsize, // Which one the "System" is writing to
}

unsafe impl Sync for EtherCATController {}
unsafe impl Send for EtherCATController {}

impl Default for EtherCATController {
    fn default() -> Self {
        Self {
            cycle_time_us: Default::default(),
            interface: Default::default(),
            state: EtherCATState::NoInterface,
            requested_state: None,
            input_buffers: [
                UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
                UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
            ],
            input_read_idx: AtomicUsize::new(0),
            output_buffers: [
                UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
                UnsafeCell::new([0u8; ETHERCAT_TX_RX_SIZE]),
            ],
            output_write_idx: AtomicUsize::new(0),            
        }
    }
}



impl EtherCATController {
    /// Read latest input blob (App side)
    pub fn get_inputs(&self) -> [u8; ETHERCAT_TX_RX_SIZE] {
        let idx = self.input_read_idx.load(Ordering::Relaxed);
     //   println!("idx: {:?}", self.input_buffers);
        let ptr = self.input_buffers[idx].get();
        unsafe { *ptr }
    }

    /// Write output commands (App side)
    pub fn set_outputs(&self, data: &[u8]) {
        let idx = self.output_write_idx.load(Ordering::Relaxed);
        let ptr = self.output_buffers[idx].get();
        unsafe {
            let buf = &mut *ptr;
            let len = data.len().min(ETHERCAT_TX_RX_SIZE);
            buf[..len].copy_from_slice(&data[..len]);
        }
        // Switch index to signal the EtherCAT thread
        self.output_write_idx.store(1 - idx, Ordering::Release);
    }
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
            let requested_state = match &self.requested_state {
                Some(requested_state) => requested_state,
                None => &EtherCATState::NoInterface,
            };
			
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
                                continue;
                            }
                        });
                        self.state = EtherCATState::PreOp;
                    };
                }
                EtherCATState::PreOp => {
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
    	    			interval(Duration::from_micros(1000))
    				});                    

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
                    let group = group_op
                                .as_ref()
                                .unwrap();

                   	let maindevice = maindevice.as_ref().unwrap();
                    loop {
                    	let res = rt.block_on(
                    		async {
        						let res = group.tx_rx_dc(&maindevice).await.expect("TX/RX");
	        					let now = tokio::time::Instant::now();
	        					(res,now)
		        			}
	        				);

        					// 1. Determine which buffer is NOT being read by the app right now
        					let read_idx = self.input_read_idx.load(Ordering::Acquire);
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
            								full_buffer[current_offset..current_offset + len].copy_from_slice(subdevice.io_raw().inputs());
	            							//println!("{:?}",full_buffer);
	            							current_offset += len;
        								} else {
            								println!("Data exceeds buffer");
            								break; 
	        							}
        							}							
        					}
        					// 3. Update the read index so the app sees the fresh buffer
        					self.input_read_idx.store(write_idx, Ordering::Release);              				
	        				rt.block_on(async {tokio::time::sleep_until(res.1 + res.0.extra.next_cycle_wait).await});

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
}}

pub fn start_ethercat_thread(interface_name: &str) -> (Arc<EtherCATController>, JoinHandle<()>) {
    let controller = Arc::new(EtherCATController {
        interface: Some(interface_name.to_owned()),
        ..Default::default()
    });

    let controller_for_thread = Arc::clone(&controller);
    let ptr = Arc::as_ptr(&controller_for_thread) as *mut EtherCATController;


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

    (controller, handle)
}