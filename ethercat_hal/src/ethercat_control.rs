use ethercrab::EtherCrabWireWrite;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::al_diagnostics::{SubDeviceAlStatus, TransitionReport};
use crate::ethercat_helpers::{EthercatResponseTypedResult, EthercatSdoBytes};
use crate::machine_ident_read::MachineDeviceInfo;
use crate::mailbox::Mailbox;
use crate::types::{Consumer, Producer};
use crate::{
    ETHERCAT_TX_RX_SIZE, EtherCATAppHandle, EtherCATState, EtherCATThreadChannel,
    MetaSubdevice, TripleBufConsumer,
};

pub struct EtherCATControl<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub channel: EtherCATThreadChannel,
    pub app_handle: EtherCATAppHandle<C, P>,
    pub join_handle: Option<JoinHandle<Result<(), anyhow::Error>>>,
}

impl EtherCATControl<TripleBufConsumer, Arc<Mailbox>> {
    pub fn sdo_read<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
    ) -> Result<T, anyhow::Error>
    where
        T: EthercatSdoBytes + EthercatResponseTypedResult,
    {
        self.channel.sdo_read(device_address, index, sub_index)
    }

    pub fn register_read(
        &self,
        device_address: u16,
        register: impl Into<u16>,
    ) -> Result<u16, anyhow::Error> {
        self.channel.register_read(device_address, register)
    }

    pub fn al_status_snapshot(&self) -> Result<Vec<SubDeviceAlStatus>, anyhow::Error> {
        self.channel.al_status_snapshot()
    }

    pub fn read_device_identifications(&self) -> Result<Vec<MachineDeviceInfo>, anyhow::Error> {
        self.channel.read_device_identifications()
    }

    pub fn write_machine_device_info_eeprom(
        &self,
        info: Vec<MachineDeviceInfo>,
    ) -> Result<(), anyhow::Error> {
        self.channel.write_machine_device_info_eeprom(info)
    }

    pub fn sdo_write<T: 'static>(
        &self,
        device_address: u16,
        index: u16,
        sub_index: u8,
        value: T,
    ) -> Result<(), anyhow::Error>
    where
        T: EtherCrabWireWrite + EthercatSdoBytes,
    {
        self.channel
            .sdo_write(device_address, index, sub_index, value)
    }

    pub fn request_state_change(&self, state: EtherCATState) -> Result<(), anyhow::Error> {
        self.channel.request_state_change(state)
    }

    pub fn enable_dc_sync0(&self, device_address: u16) -> Result<(), anyhow::Error> {
        self.channel.enable_dc_sync0(device_address)
    }

    pub fn enable_dc_sync01(
        &self,
        device_address: u16,
        sync1_period: Duration,
    ) -> Result<(), anyhow::Error> {
        self.channel.enable_dc_sync01(device_address, sync1_period)
    }

    pub fn configure_oversampling(
        &self,
        device_address: u16,
        oversampling_settings: Vec<(u16, u16)>,
    ) -> Result<(), anyhow::Error> {
        self.channel
            .configure_oversampling(device_address, oversampling_settings)
    }

    pub fn set_mut_beckhoff_eeprom_lock_active(
        &self,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        self.channel
            .set_mut_beckhoff_eeprom_lock_active(device_address)
    }

    pub fn check_all_op(&self) -> bool {
        self.app_handle.check_all_op()
    }

    pub fn get_dc_sys_time_ns(&self) -> u64 {
        self.app_handle.get_dc_sys_time_ns()
    }

    pub fn get_inputs(&mut self) -> Option<&[u8]> {
        self.app_handle.get_inputs()
    }

    pub fn finish_read(&mut self) {
        self.app_handle.finish_read();
    }

    pub fn write_outputs(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        self.app_handle.write_outputs()
    }

    pub fn send_outputs(&mut self) {
        self.app_handle.send_outputs();
    }

    pub fn get_current_cycle(&self) -> u64 {
        self.app_handle.get_current_cycle()
    }

    pub fn get_cycle_time_us(&self) -> u64 {
        self.app_handle.get_cycle_time_us()
    }

    pub fn get_next_cycle_us(&self) -> u64 {
        self.app_handle.get_next_cycle_us()
    }

    pub fn get_subdevice_count(&self) -> u64 {
        self.app_handle.get_subdevice_count()
    }

    pub fn get_state(&self) -> EtherCATState {
        self.app_handle.get_state()
    }

    pub fn check_inputs_ready(&self) -> bool {
        self.app_handle.check_inputs_ready()
    }

    pub fn try_get_subdevices_vec_sync(&self) -> Result<Vec<MetaSubdevice>, anyhow::Error> {
        self.app_handle.try_get_subdevices_vec_sync()
    }

    pub async fn try_get_subdevices_vec(&self) -> Result<Vec<MetaSubdevice>, anyhow::Error> {
        self.app_handle.try_get_subdevices_vec().await
    }

    /// Every transition attempted so far with the AL status of each subdevice afterwards,
    /// oldest first. Shared state, so it stays readable after the state-machine thread ends.
    pub fn get_transition_reports(&self) -> Vec<TransitionReport> {
        self.app_handle.get_transition_reports()
    }

    /// The most recent failed transition — i.e. why the master is not in OP.
    pub fn get_last_transition_failure(&self) -> Option<TransitionReport> {
        self.app_handle.get_last_transition_failure()
    }
}
