use crate::{
    ETHERCAT_TX_RX_SIZE, EtherCATState, MAX_SUBDEVICES, MetaSubdevice, al_diagnostics::{TransitionLog, TransitionReport}, types::{Consumer, Producer},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering::Relaxed},
};

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
    subdevices: Arc<tokio::sync::Mutex<[MetaSubdevice; MAX_SUBDEVICES]>>,
    all_op: Arc<AtomicBool>,
    dc_sys_time: Arc<AtomicU64>,
    inputs_ready: Arc<AtomicBool>,
    transition_log: TransitionLog,
}

impl<C, P> EtherCATAppHandle<C, P>
where
    C: Consumer,
    P: Producer,
{
    pub fn new(
        input_consumer: C,
        output_producer: P,
        cycle: Arc<AtomicU64>,
        cycle_time_us: Arc<AtomicU64>,
        next_cycle_us: Arc<AtomicU64>,
        subdevice_count: Arc<AtomicU64>,
        state: Arc<AtomicU8>,
        subdevices: Arc<tokio::sync::Mutex<[MetaSubdevice; MAX_SUBDEVICES]>>,
        all_op: Arc<AtomicBool>,
        dc_sys_time: Arc<AtomicU64>,
        inputs_ready: Arc<AtomicBool>,
        transition_log: TransitionLog,
    ) -> Self {
        EtherCATAppHandle {
            input_consumer,
            output_producer,
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
        }
    }

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

    /// Every transition attempted so far with the AL status of each subdevice afterwards,
    /// oldest first. Shared state, so it stays readable after the state-machine thread ends.
    pub fn get_transition_reports(&self) -> Vec<TransitionReport> {
        self.transition_log.reports()
    }

    /// The most recent failed transition — i.e. why the master is not in OP.
    pub fn get_last_transition_failure(&self) -> Option<TransitionReport> {
        self.transition_log.last_failure()
    }
}
