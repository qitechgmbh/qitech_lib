use anyhow::bail;
use crate::EtherCATThreadChannel;
use crate::coe::{ConfigurableDevice, Configuration};
use crate::io::multi_timestamp::{MultiTimestampInput, MultiTimestampOutput};
use crate::pdo::{PredefinedPdoAssignment, RxPdo, TxPdo};
use crate::{BECKHOFF_VENDOR_ID, io::{digital_input::DigitalInputDevice, multi_timestamp::MultiTimestampEvent}};
use ethercat_hal_derive::EthercatDevice;
use std::collections::VecDeque;
use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};

mod pdo;
use pdo::{EL1259RxPdo, EL1259TxPdo};

enum State {
    ResetOn,
    ResetOff,
    Initialized,
}

/// EL1259 8-channel Multi-Timestamp Input / 8-channel Multi-Timestamp Output
/// 24V DC, 0.5A per channel
#[derive(EthercatDevice)]
pub struct EL1259 {
    rxpdo: EL1259RxPdo,
    txpdo: EL1259TxPdo,
    is_used: bool,
    input_queues: [VecDeque<MultiTimestampEvent>; 8],
    output_queues: [VecDeque<MultiTimestampEvent>; 8],
    state: State,
}

impl EthercatDeviceProcessing for EL1259 {

    fn input_post_process(&mut self) -> Result<(), anyhow::Error> {
        for channel in 0..8 {
            let txmto = self.txpdo.get_mto(channel);
            let txmti = self.txpdo.get_mti(channel);

            if txmto.output_short_circuit {
                bail!("Short circuit on channel {}", channel+1);
            }

            if txmto.output_buffer_overflow {
                bail!("Buffer overflow on output channel {}", channel+1);
            }

            if txmti.input_buffer_overflow {
                bail!("Buffer overflow on input channel {}", channel+1);
            }
        }

        for channel in 0..8 {
            let txmti = self.txpdo.get_mti_mut(channel);
            let rxmti = self.rxpdo.get_mti_mut(channel);
            if rxmti.input_order_counter == txmti.input_order_feedback {
                let events = txmti.get_events().into_iter().cloned();
                self.input_queues[channel].extend(events);
                rxmti.input_order_counter = txmti.input_order_feedback.wrapping_add(1);
            }
        }

        Ok(())
    }

    fn output_pre_process(&mut self) -> Result<(), anyhow::Error> {
        match self.state {
            State::ResetOn => {
                for channel in 0..8 {
                    self.rxpdo.get_mto_mut(channel).output_buffer_reset = true;
                }
                self.state = State::ResetOff;
            }

            State::ResetOff => {
                for channel in 0..8 {
                    self.rxpdo.get_mto_mut(channel).output_buffer_reset = false;
                }
                self.state = State::Initialized;
            }

            State::Initialized => {
                for channel in 0..8 {
                    let txmto = self.txpdo.get_mto(channel);
                    let rxmto = self.rxpdo.get_mto_mut(channel);
                    let queue = &mut self.output_queues[channel];

                    let empty_splots_in_buffer = 32 - txmto.events_in_output_buffer as usize;
                    let number_of_events_to_send = queue.len().min(empty_splots_in_buffer).min(10);

                    if number_of_events_to_send > 0 && rxmto.output_order_count == txmto.output_order_feedback {
                        let events_to_send: Vec<_> = queue.drain(0..number_of_events_to_send).collect();
                        rxmto.set_events(&events_to_send);
                        rxmto.output_order_count = txmto.output_order_feedback.wrapping_add(1);
                        rxmto.force_order = false;
                    } else {
                        rxmto.number_of_output_events = 0;
                    }
                }
            }
        }

        Ok(())
    }
}

impl DigitalInputDevice for EL1259 {

    fn get_input(&self, port: usize) -> Result<bool, anyhow::Error> {
        Ok(self.txpdo.get_mti(port).input_state)
    }

    fn get_port_count(&self) -> usize {
        8
    }
}

impl MultiTimestampInput for EL1259 {

    fn peek(&self, port: usize) -> Option<&MultiTimestampEvent> {
        self.input_queues[port].front()
    }

    fn pop(&mut self, port: usize) -> Option<MultiTimestampEvent> {
        self.input_queues[port].pop_front()
    }

    fn peek_all(&self, port: usize) -> &[MultiTimestampEvent] {
        let (front, _back) = self.input_queues[port].as_slices();
        front
    }

    fn pop_all(&mut self, port: usize) -> Vec<MultiTimestampEvent> {
        self.input_queues[port].drain(..self.input_queues[port].len()).collect()
    }

    fn get_port_count(&self) -> usize {
        8
    }
}

impl MultiTimestampOutput for EL1259 {

    fn push(&mut self, port: usize, event: MultiTimestampEvent) {
        self.output_queues[port].push_back(event);
    }

    fn push_all(&mut self, port: usize, events: &[MultiTimestampEvent]) {
        self.output_queues[port].extend(events.iter().cloned());
    }

    fn get_port_count(&self) -> usize {
        8
    }
}

impl std::fmt::Debug for EL1259 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EL1259")
    }
}

impl NewEthercatDevice for EL1259 {
    fn new() -> Self {
        Self {
            rxpdo: EL1259RxPdo::default(),
            txpdo: EL1259TxPdo::default(),
            is_used: false,
            input_queues: Default::default(),
            output_queues: Default::default(),
            state: State::ResetOn,
        }
    }
}

impl ConfigurableDevice<EL1259Configuration> for EL1259 {

    fn write_config(
        &mut self,
        channel: EtherCATThreadChannel,
        device_address: u16,
        config: &EL1259Configuration,
    ) -> Result<(), anyhow::Error>
    {
        config.write_config(channel, device_address)?;
        Ok(())
    }

    fn get_config(&self) -> EL1259Configuration {
        EL1259Configuration::default()
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct EL1259Configuration {
}

impl Configuration for EL1259Configuration {

    fn write_config(
        &self,
        channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error>
    {
        self.txpdo_assignment().write_config(channel.clone(), device_address)?;
        self.rxpdo_assignment().write_config(channel, device_address)?;
        Ok(())
    }
}

impl PredefinedPdoAssignment<EL1259TxPdo, EL1259RxPdo> for EL1259Configuration {

    fn txpdo_assignment(&self) -> EL1259TxPdo {
        EL1259TxPdo::default()
    }

    fn rxpdo_assignment(&self) -> EL1259RxPdo {
        EL1259RxPdo::default()
    }
}

pub const EL1259_VENDOR_ID: u32 = BECKHOFF_VENDOR_ID;
pub const EL1259_PRODUCT_ID: u32 = 82522194;
pub const EL1259_REVISION_A: u32 = 0;
pub const EL1259_IDENTITY_A: SubDeviceIdentityTuple =
    (EL1259_VENDOR_ID, EL1259_PRODUCT_ID, EL1259_REVISION_A);
