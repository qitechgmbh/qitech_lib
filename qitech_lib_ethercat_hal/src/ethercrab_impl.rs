use crate::{
    ChannelError, EthercatMaster, MAX_SUBDEVICES, PDI_LEN, PDU_STORAGE, RtOptimizationConfig,
    get_async_runtime,
    sdo::{SdoReadRequest, SdoRequest, SdoType},
    set_current_thread_rt_priority,
};
use ethercrab::{
    DcSync, EtherCrabWireRead, EtherCrabWireSized, MainDevice, MainDeviceConfig, RetryBehaviour, SubDeviceGroup, Timeouts, std::ethercat_now,
};
use libc::group;
use std::time::Duration;
use ethercrab::{
    subdevice_group::{
        
        Init, PreOp, PreOpPdi, SafeOp, Op, NoDc, HasDc
    },    
    DefaultLock,
};

/// An enum representing every possible SubDeviceGroup configuration and state 
/// across both DC (HasDc) and non-DC (NoDc) variants.
pub enum EcrabGroup<
    const MAX_SUBDEVICES: usize, 
    const MAX_PDI: usize, 
> {
    Init(),    
    PreOpNoDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, PreOp, NoDc>),
    PreOpHasDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, PreOp, HasDc>),
    PreOpPdiNoDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, PreOpPdi, NoDc>),
    PreOpPdiHasDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, PreOpPdi, HasDc>),
    SafeOpNoDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, SafeOp, NoDc>),
    SafeOpHasDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, SafeOp, HasDc>),
    OpNoDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, Op, NoDc>),
    OpHasDc(SubDeviceGroup<MAX_SUBDEVICES, MAX_PDI, DefaultLock, Op, HasDc>),
}

// TODO Add handle to tx rx
pub struct EtherCrabMaster<'a, const MAX_SUBDEVICES: usize, const MAX_PDI: usize> {
    maindevice: Option<MainDevice<'a>>,    
    group: EcrabGroup<MAX_SUBDEVICES, MAX_PDI>,
    outputs : [u8;MAX_PDI],
    inputs : [u8;MAX_PDI],
}


impl<'a> EtherCrabMaster<'a, MAX_SUBDEVICES, PDI_LEN> {
    fn new() -> Self {
        EtherCrabMaster { maindevice: None, group: EcrabGroup::Init(), outputs:[0u8;PDI_LEN], inputs: [0u8;PDI_LEN] }
    }

    fn setup_tx_rx_thread(
        interface: String,
        opt: Option<RtOptimizationConfig>,
    ) -> Option<MainDevice<'static>> {
        let (tx, rx, pdu) = PDU_STORAGE.try_split().expect("can only split once");
        let pdu_tx = tx;
        let pdu_rx = rx;

        let _ethercat_tx_rx_handle = std::thread::Builder::new()
            .name("EthercatTxRxThread".to_owned())
            .spawn(move || {
                match opt {
                    Some(opt) => {
                        let id = core_affinity::CoreId {
                            id: opt.ethercat_io_thread_core,
                        };
                        set_current_thread_rt_priority(opt.ethercat_io_thread_priority as i32);
                        // Pin to the last core (e.g., Core 3 on a 4-core system)
                        core_affinity::set_for_current(id);
                        #[cfg(target_os = "linux")]
                        if let Some(irq_core) = opt.pin_irq_core {
                            use qitech_lib_common::set_irq_affinity;
                            if let Err(e) = set_irq_affinity(&interface, irq_core as u32) {
                                tracing::warn!(
                                    "set_irq_affinity failed, performance may be degraded: {e}"
                                );
                            }
                        }
                    }
                    None => (),
                };

                #[cfg(not(target_os = "linux"))]
                use ethercrab::std::tx_rx_task;
                #[cfg(target_os = "linux")]
                use ethercrab::std::tx_rx_task_io_uring;

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

        let maindevice = Some(MainDevice::new(
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
        return maindevice;
    }

    fn into_preop(&mut self)  -> Result<(), crate::EthercatErr> {
        match self.group {
            EcrabGroup::Init() => {
                let rt = get_async_runtime();
                let group = rt.block_on(self.maindevice.as_ref().expect("").init_single_group::<MAX_SUBDEVICES, PDI_LEN>(ethercat_now)).unwrap();
                self.group = EcrabGroup::PreOpNoDc( group );
                Ok(())
            },
            _ => Ok(()),
        }
    }
}




impl<'a> EthercatMaster for EtherCrabMaster<'a, MAX_SUBDEVICES, PDI_LEN> {
    fn init(&mut self, interface: &str) -> Result<(), crate::EthercatErr> {
        self.maindevice = Self::setup_tx_rx_thread(interface.clone().to_owned(), None);
        Ok(())
    }

    fn into_pre_op_pdi(&mut self, cycle_time: Duration) -> Result<(), crate::EthercatErr> {
        let old_group = std::mem::replace(&mut self.group, EcrabGroup::Init());
        match old_group {
            EcrabGroup::PreOpNoDc(sub_device_group) => {
                let pdi_group = get_async_runtime().block_on(sub_device_group.into_pre_op_pdi(self.maindevice.as_ref().unwrap())).unwrap();
                self.group = EcrabGroup::PreOpPdiNoDc( pdi_group );
                Ok(())
            },
            _ => {
                Ok(())
            },  
        }
    }

    fn configure_dc_sync(
        &mut self,
        config: &crate::DcConfiguration,
    ) -> Result<(), crate::EthercatErr> {

    }

    fn into_safe_op(&mut self) -> Result<(), crate::EthercatErr> {
        
    }

    fn request_into_op(&mut self) -> Result<(), crate::EthercatErr> {
        Ok(())
    }

    fn tx_rx(&mut self) -> Result<crate::TxRxResult, crate::EthercatErr> {
    
    }

    fn get_subdevices(&self) -> Vec<crate::MetaSubdevice> {
    
    }

    fn read_inputs(&self, buffer: &mut [u8]) {
    
    }

    fn write_outputs(&mut self, outputs: &[u8]) {
    
    }

    fn sdo_write(&mut self, request: SdoRequest) -> Result<(), ChannelError> {
    
    }

    fn sdo_read(&mut self, request: &SdoReadRequest) -> Result<[u8; 4], ChannelError> {
    
    }

    fn eeprom_read_u16(
        &mut self,
        device_address: u16,
        word_address: u16,
    ) -> Result<u16, ChannelError> {
    
    }

    fn eeprom_write_u16(
        &mut self,
        device_address: u16,
        word_address: u16,
        word: u16,
    ) -> Result<(), ChannelError> {
    
    }

    fn enable_dc_sync0(&mut self, device_address: u16) -> Result<(), ChannelError> {
        Ok(())
    }

    fn enable_dc_sync01(
        &mut self,
        device_address: u16,
        sync1_period: Duration,
    ) -> Result<(), ChannelError> {
        Ok(())
    }

    fn configure_oversampling(
        &mut self,
        device_address: u16,
        oversampling_settings: &[(u16, u16)],
    ) -> Result<(), ChannelError> {
        Ok(())
    }

    fn register_read(&mut self, device_address: u16, register: u16) -> Result<u16, ChannelError> {
        Ok(())
    } 

    fn al_status_snapshot(&mut self) -> Vec<crate::al_diagnostics::SubDeviceAlStatus> {
        Ok(())
    }
}

/*
 Value type needs to have EtherCrabWireWriteSized at the least to be able to write with ethecrab
*/
pub fn sdo_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoRequest,
) -> Result<(), ChannelError> {
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();

            let res = match request.type_flag {
                SdoType::U8 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    request.data[0],
                )),
                SdoType::U16 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    u16::from_le_bytes([request.data[0], request.data[1]]),
                )),
                SdoType::U32 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    u32::from_le_bytes(request.data),
                )),
                SdoType::I16 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    i16::from_le_bytes([request.data[0], request.data[1]]),
                )),
                SdoType::I32 => runtime.block_on(device.sdo_write(
                    request.sdo_index.index as u16,
                    request.sdo_index.sub_index as u8,
                    i32::from_le_bytes(request.data),
                )),
                SdoType::BOOL => {
                    let b: bool = request.data[0] == 1;
                    runtime.block_on(device.sdo_write(
                        request.sdo_index.index as u16,
                        request.sdo_index.sub_index as u8,
                        b,
                    ))
                }
            };
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(request.device_address))
}

pub fn sdo_read<T>(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    request: SdoReadRequest,
) -> Result<T, ChannelError>
where
    T: EtherCrabWireRead + EtherCrabWireSized,
{
    for device in group.iter(maindevice) {
        if device.configured_address() == request.device_address {
            let runtime = get_async_runtime();
            let res: Result<T, ethercrab::error::Error> =
                runtime.block_on(device.sdo_read::<T>(request.index, request.sub_index as u8));
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(request.device_address))
}

pub fn eeprom_read(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    device_address: u16,
    word_address: u16,
) -> Result<u16, ChannelError> {
    for device in group.iter(maindevice) {
        if device.configured_address() == device_address {
            let runtime = get_async_runtime();
            let res = runtime.block_on(device.eeprom_read::<u16>(maindevice, word_address));
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(device_address))
}

pub fn eeprom_write(
    maindevice: &MainDevice,
    group: &SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    device_address: u16,
    word_address: u16,
    value: u16,
) -> Result<(), ChannelError> {
    for device in group.iter(maindevice) {
        if device.configured_address() == device_address {
            let runtime = get_async_runtime();
            let res =
                runtime.block_on(device.eeprom_write_dangerously(maindevice, word_address, value));
            return Ok(res?);
        }
    }
    Err(ChannelError::UnknownSubdevice(device_address))
}

pub fn enable_dc_sync(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
) -> Result<(), ChannelError> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_dc_sync(DcSync::Sync0);
                return Ok(());
            }
        }
        return Err(ChannelError::UnknownSubdevice(device_address as u16));
    })
}

pub fn enable_dc_sync01(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
    sync1_period: Duration,
) -> Result<(), ChannelError> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_dc_sync(DcSync::Sync01 { sync1_period });
                return Ok(());
            }
        }
        Err(ChannelError::UnknownSubdevice(device_address as u16))
    })
}

pub fn configure_oversampling(
    group: &mut SubDeviceGroup<MAX_SUBDEVICES, PDI_LEN>,
    maindevice: &MainDevice,
    device_address: usize,
    oversampling_settings: &[(u16, u16)],
) -> Result<(), ChannelError> {
    let rt = get_async_runtime();
    rt.block_on(async {
        for mut subdevice in group.iter_mut(maindevice) {
            if subdevice.configured_address() == device_address as u16 {
                subdevice.set_oversampling(oversampling_settings);
                return Ok(());
            }
        }
        Err(ChannelError::UnknownSubdevice(device_address as u16))
    })
}
