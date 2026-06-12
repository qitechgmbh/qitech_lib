use bitvec::field::BitField;

use crate::devices::{
    DynamicEthercatDevice, EthercatDevice, EthercatDeviceProcessing, EthercatDeviceUsed,
    EthercatDynamicPDO, Module, NewEthercatDevice, SubDeviceProductTuple,
};
use crate::io::analog_output::{AnalogOutputDevice, AnalogOutputOutput};

#[derive(Clone, Default)]
pub struct Wago750_553RxPdo {
    ao1: u16,
    ao2: u16,
    ao3: u16,
    ao4: u16,
}

#[derive(Clone)]
pub struct Wago750_553 {
    is_used: bool,
    tx_bit_offset: usize,
    rx_bit_offset: usize,
    module: Option<Module>,
    rx_pdo: Wago750_553RxPdo,
}

impl AnalogOutputDevice for Wago750_553 {
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput) {
        let raw = (value.0.clamp(0.0, 1.0) * 0x7FFF as f32) as u16;
        match port {
            0 => self.rx_pdo.ao1 = raw,
            1 => self.rx_pdo.ao2 = raw,
            2 => self.rx_pdo.ao3 = raw,
            3 => self.rx_pdo.ao4 = raw,
            _ => {}
        }
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

impl DynamicEthercatDevice for Wago750_553 {}

impl EthercatDynamicPDO for Wago750_553 {
    fn get_tx_offset(&self) -> usize {
        self.tx_bit_offset
    }

    fn get_rx_offset(&self) -> usize {
        self.rx_bit_offset
    }

    fn set_tx_offset(&mut self, offset: usize) {
        self.tx_bit_offset = offset
    }

    fn set_rx_offset(&mut self, offset: usize) {
        self.rx_bit_offset = offset
    }
}

impl EthercatDeviceUsed for Wago750_553 {
    fn is_used(&self) -> bool {
        self.is_used
    }

    fn set_used(&mut self, used: bool) {
        self.is_used = used;
    }
}

impl EthercatDevice for Wago750_553 {
    fn input(
        &mut self,
        _input: &bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn input_len(&self) -> usize {
        0
    }

    fn output(
        &self,
        output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), anyhow::Error> {
        let base = self.rx_bit_offset;
        output[base..(base + 16)].store_le::<u16>(self.rx_pdo.ao1);
        output[(base + 16)..(base + 32)].store_le::<u16>(self.rx_pdo.ao2);
        output[(base + 32)..(base + 48)].store_le::<u16>(self.rx_pdo.ao3);
        output[(base + 48)..(base + 64)].store_le::<u16>(self.rx_pdo.ao4);
        Ok(())
    }

    fn output_len(&self) -> usize {
        64
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn is_module(&self) -> bool {
        true
    }

    fn input_checked(
        &mut self,
        _input: &bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn output_checked(
        &self,
        _output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn get_module(&self) -> Option<Module> {
        self.module.clone()
    }

    fn set_module(&mut self, module: Module) {
        self.tx_bit_offset = module.tx_offset;
        self.rx_bit_offset = module.rx_offset;
        self.module = Some(module);
    }

    fn into_any_boxed(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl EthercatDeviceProcessing for Wago750_553 {}

impl NewEthercatDevice for Wago750_553 {
    fn new() -> Self {
        Self {
            is_used: false,
            tx_bit_offset: 0,
            rx_bit_offset: 0,
            module: None,
            rx_pdo: Wago750_553RxPdo::default(),
        }
    }
}

impl std::fmt::Debug for Wago750_553 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wago750_553")
    }
}

pub const WAGO_750_553_VENDOR_ID: u32 = 0x00000021;
pub const WAGO_750_553_PRODUCT_ID: u32 = 0x55343f3;
pub const WAGO_750_553_MODULE_IDENT: SubDeviceProductTuple =
    (WAGO_750_553_VENDOR_ID, WAGO_750_553_PRODUCT_ID);
