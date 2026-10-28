//! An [`EthercatMaster`] without hardware, for tests and examples.
//!
//! Subdevices are plain [`MetaSubdevice`]s. The mock gives each one an address and lays out the
//! process image back to back in bus order, taking the PDO sizes from `end_tx - start_tx` and
//! `end_rx - start_rx` of the subdevice passed in. Every transition succeeds, and each `tx_rx`
//! hands the inputs the subdevices send to the master and the outputs from `write_outputs` to
//! the subdevices.

use crate::al_diagnostics::SubDeviceAlStatus;
use crate::sdo::EthercatSdoBytes;
use crate::{
    ChannelError, DcConfiguration, EthercatErr, EthercatMaster, MetaSubdevice, TxRxResult,
};
use std::collections::HashMap;
use std::time::Duration;

/// Configured address of the first subdevice on the bus.
pub const FIRST_ADDRESS: u16 = 0x1000;

#[derive(Default)]
pub struct MockMaster {
    subdevices: Vec<MetaSubdevice>,
    /// What the subdevices send, set by the test.
    subdevice_inputs: Vec<u8>,
    /// What the subdevices received with the last `tx_rx`.
    subdevice_outputs: Vec<u8>,
    /// The inputs as the master received them with the last `tx_rx`.
    inputs: Vec<u8>,
    /// The outputs from the last `write_outputs`, sent with the next `tx_rx`.
    outputs: Vec<u8>,
    /// SDOs that were never written read as 0.
    sdos: HashMap<(u16, u16, u8), [u8; 4]>,
    /// EEPROM words that were never written read as 0.
    eeprom: HashMap<(u16, u16), u16>,
    op_requested: bool,
    cycle_time: Duration,
    dc_system_time: u64,
}

impl MockMaster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Plugs `subdevice` in at the end of the bus and returns the address it gets.
    pub fn add_subdevice(&mut self, mut subdevice: MetaSubdevice) -> u16 {
        subdevice.device_address = self
            .subdevices
            .iter()
            .map(|subdevice| subdevice.device_address + 1)
            .max()
            .unwrap_or(FIRST_ADDRESS);
        subdevice.initialized = true;
        self.subdevices.push(subdevice);
        self.layout();
        subdevice.device_address
    }

    /// Unplugs the subdevice, together with its SDOs and EEPROM. The other subdevices keep their
    /// addresses, but their process data moves up.
    pub fn remove_subdevice(&mut self, address: u16) -> Option<MetaSubdevice> {
        let index = self
            .subdevices
            .iter()
            .position(|subdevice| subdevice.device_address == address)?;
        let subdevice = self.subdevices.remove(index);
        self.sdos
            .retain(|(sdo_address, ..), _| *sdo_address != address);
        self.eeprom
            .retain(|(eeprom_address, _), _| *eeprom_address != address);
        self.layout();
        Some(subdevice)
    }

    /// The TxPdo the subdevice sends with the next `tx_rx`.
    pub fn subdevice_inputs_mut(&mut self, address: u16) -> Option<&mut [u8]> {
        let subdevice = *self.subdevice(address).ok()?;
        Some(&mut self.subdevice_inputs[subdevice.start_tx..subdevice.end_tx])
    }

    /// The RxPdo the subdevice received with the last `tx_rx`.
    pub fn subdevice_outputs(&self, address: u16) -> Option<&[u8]> {
        let subdevice = self.subdevice(address).ok()?;
        Some(&self.subdevice_outputs[subdevice.start_rx..subdevice.end_rx])
    }

    fn subdevice(&self, address: u16) -> Result<&MetaSubdevice, ChannelError> {
        self.subdevices
            .iter()
            .find(|subdevice| subdevice.device_address == address)
            .ok_or(ChannelError::UnknownSubdevice(address))
    }

    /// Puts the process data of all subdevices back to back and clears it.
    fn layout(&mut self) {
        let (mut tx, mut rx) = (0, 0);
        for subdevice in &mut self.subdevices {
            let tx_len = subdevice.end_tx.saturating_sub(subdevice.start_tx);
            let rx_len = subdevice.end_rx.saturating_sub(subdevice.start_rx);
            subdevice.start_tx = tx;
            subdevice.end_tx = tx + tx_len;
            subdevice.start_rx = rx;
            subdevice.end_rx = rx + rx_len;
            tx += tx_len;
            rx += rx_len;
        }
        self.subdevice_inputs = vec![0; tx];
        self.inputs = vec![0; tx];
        self.subdevice_outputs = vec![0; rx];
        self.outputs = vec![0; rx];
    }
}

impl EthercatMaster for MockMaster {
    fn init(&mut self, _interface: &str) -> Result<(), EthercatErr> {
        self.op_requested = false;
        Ok(())
    }

    fn into_pre_op_pdi(&mut self, cycle_time: Duration) -> Result<(), EthercatErr> {
        self.cycle_time = cycle_time;
        Ok(())
    }

    fn configure_dc_sync(&mut self, _config: &DcConfiguration) -> Result<(), EthercatErr> {
        Ok(())
    }

    fn into_safe_op(&mut self) -> Result<(), EthercatErr> {
        Ok(())
    }

    fn request_into_op(&mut self) -> Result<(), EthercatErr> {
        self.op_requested = true;
        Ok(())
    }

    fn tx_rx(&mut self) -> Result<TxRxResult, EthercatErr> {
        self.inputs.copy_from_slice(&self.subdevice_inputs);
        self.subdevice_outputs.copy_from_slice(&self.outputs);
        self.dc_system_time += self.cycle_time.as_nanos() as u64;
        Ok(TxRxResult {
            dc_system_time: self.dc_system_time,
            next_cycle_wait: self.cycle_time,
            all_op: self.op_requested,
        })
    }

    fn get_subdevices(&self) -> Vec<MetaSubdevice> {
        self.subdevices.clone()
    }

    fn read_inputs(&self, buffer: &mut [u8]) {
        let len = buffer.len().min(self.inputs.len());
        buffer[..len].copy_from_slice(&self.inputs[..len]);
    }

    fn write_outputs(&mut self, outputs: &[u8]) {
        let len = outputs.len().min(self.outputs.len());
        self.outputs[..len].copy_from_slice(&outputs[..len]);
    }

    fn sdo_write<T: EthercatSdoBytes>(
        &mut self,
        device_address: u16,
        index: u16,
        sub_index: u8,
        value: T,
    ) -> Result<(), ChannelError> {
        self.subdevice(device_address)?;
        self.sdos
            .insert((device_address, index, sub_index), value.to_bytes());
        Ok(())
    }

    fn sdo_read<T: EthercatSdoBytes>(
        &mut self,
        device_address: u16,
        index: u16,
        sub_index: u8,
    ) -> Result<T, ChannelError> {
        self.subdevice(device_address)?;
        let bytes = self.sdos.get(&(device_address, index, sub_index));
        Ok(T::from_bytes(bytes.copied().unwrap_or_default()))
    }

    fn eeprom_read_u16(
        &mut self,
        device_address: u16,
        word_address: u16,
    ) -> Result<u16, ChannelError> {
        self.subdevice(device_address)?;
        let word = self.eeprom.get(&(device_address, word_address));
        Ok(word.copied().unwrap_or(0))
    }

    fn eeprom_write_u16(
        &mut self,
        device_address: u16,
        word_address: u16,
        word: u16,
    ) -> Result<(), ChannelError> {
        self.subdevice(device_address)?;
        self.eeprom.insert((device_address, word_address), word);
        Ok(())
    }

    fn enable_dc_sync0(&mut self, device_address: u16) -> Result<(), ChannelError> {
        self.subdevice(device_address).map(|_| ())
    }

    fn enable_dc_sync01(
        &mut self,
        device_address: u16,
        _sync1_period: Duration,
    ) -> Result<(), ChannelError> {
        self.subdevice(device_address).map(|_| ())
    }

    fn configure_oversampling(
        &mut self,
        device_address: u16,
        _oversampling_settings: &[(u16, u16)],
    ) -> Result<(), ChannelError> {
        self.subdevice(device_address).map(|_| ())
    }

    fn register_read(&mut self, device_address: u16, _register: u16) -> Result<u16, ChannelError> {
        self.subdevice(device_address).map(|_| 0)
    }

    fn al_status_snapshot(&mut self) -> Vec<SubDeviceAlStatus> {
        self.subdevices
            .iter()
            .map(|subdevice| SubDeviceAlStatus {
                device_address: subdevice.device_address,
                name: subdevice.get_name().unwrap_or_default(),
                state: None,
                error_flag: false,
                al_status_code: None,
                read_error: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subdevice(product_id: u32, tx_len: usize, rx_len: usize) -> MetaSubdevice {
        MetaSubdevice {
            vendor: 0x2,
            product_id,
            end_tx: tx_len,
            end_rx: rx_len,
            ..Default::default()
        }
    }

    #[test]
    fn process_data_round_trip() {
        let mut master = MockMaster::new();
        let input = master.add_subdevice(subdevice(1, 2, 0));
        let output = master.add_subdevice(subdevice(2, 0, 1));
        assert_eq!((input, output), (FIRST_ADDRESS, FIRST_ADDRESS + 1));

        let subdevices = master.get_subdevices();
        assert_eq!((subdevices[0].start_tx, subdevices[0].end_tx), (0, 2));
        assert_eq!((subdevices[1].start_rx, subdevices[1].end_rx), (0, 1));

        master
            .subdevice_inputs_mut(input)
            .unwrap()
            .copy_from_slice(&[0xAB, 0xCD]);
        master.write_outputs(&[0x5A]);
        master.tx_rx().unwrap();

        let mut inputs = [0u8; 2];
        master.read_inputs(&mut inputs);
        assert_eq!(inputs, [0xAB, 0xCD]);
        assert_eq!(master.subdevice_outputs(output).unwrap(), [0x5A]);
    }

    #[test]
    fn removing_keeps_addresses_and_moves_process_data_up() {
        let mut master = MockMaster::new();
        let a = master.add_subdevice(subdevice(1, 1, 0));
        let b = master.add_subdevice(subdevice(2, 1, 0));
        master.sdo_write(a, 0x8000, 1, 1u8).unwrap();

        assert_eq!(master.remove_subdevice(a).unwrap().product_id, 1);
        let subdevices = master.get_subdevices();
        assert_eq!(subdevices.len(), 1);
        assert_eq!(subdevices[0].device_address, b);
        assert_eq!((subdevices[0].start_tx, subdevices[0].end_tx), (0, 1));
        assert!(matches!(
            master.sdo_read::<u8>(a, 0x8000, 1),
            Err(ChannelError::UnknownSubdevice(_))
        ));
    }

    #[test]
    fn sdo_and_eeprom() {
        let mut master = MockMaster::new();
        let a = master.add_subdevice(subdevice(1, 0, 0));
        assert_eq!(master.sdo_read::<u16>(a, 0x8000, 1).unwrap(), 0);
        master.sdo_write(a, 0x8000, 1, 500u16).unwrap();
        assert_eq!(master.sdo_read::<u16>(a, 0x8000, 1).unwrap(), 500);
        master.eeprom_write_u16(a, 0x0100, 7).unwrap();
        assert_eq!(master.eeprom_read_u16(a, 0x0100).unwrap(), 7);
    }
}
