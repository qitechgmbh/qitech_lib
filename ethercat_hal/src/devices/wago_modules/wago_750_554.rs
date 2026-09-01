use bitvec::field::BitField;

use crate::devices::{
    DynamicEthercatDevice, EthercatDevice, EthercatDeviceProcessing, EthercatDeviceUsed,
    EthercatDynamicPDO, Module, NewEthercatDevice, SubDeviceProductTuple,
};
use crate::io::analog_output::AnalogCurrentOutputDevice;
use units::electric_current::milliampere;
use units::f64::ElectricCurrent;

/// Wago 750-554 2-channel analog current output device
///
/// 12-bit resolution, 4-20mA
#[derive(Clone)]
pub struct Wago750_554 {
    is_used: bool,
    tx_bit_offset: usize,
    rx_bit_offset: usize,
    module: Option<Module>,
    rx_pdo: Wago750_554RxPdo,
}

impl Wago750_554 {
    /// Full scale: 0x0000 -> 4 mA, 0x7FFF -> 20 mA
    const MAX_RAW: u16 = 0x7FFF;
    /// 12-bit resolution on B3..B14; low 3 bits ignored by module
    const VALUE_MASK: u16 = 0x7FF8;
}

#[derive(Clone, Default)]
pub struct Wago750_554RxPdo {
    pub channel1: u16,
    pub channel2: u16,
}

impl AnalogCurrentOutputDevice for Wago750_554 {
    fn get_port_count(&self) -> usize {
        2
    }

    fn get_minimum_output(&self) -> ElectricCurrent {
        ElectricCurrent::new::<milliampere>(4.0)
    }

    fn get_maximum_output(&self) -> ElectricCurrent {
        ElectricCurrent::new::<milliampere>(20.0)
    }

    fn set_output_relative(&mut self, port: usize, value: f64) {
        let value = value.clamp(0.0, 1.0);
        let raw = (value * Self::MAX_RAW as f64).round() as u16 & Self::VALUE_MASK;
        match port {
            0 => self.rx_pdo.channel1 = raw,
            1 => self.rx_pdo.channel2 = raw,
            _ => (),
        }
    }
}

impl EthercatDeviceUsed for Wago750_554 {
    fn is_used(&self) -> bool {
        self.is_used
    }

    fn set_used(&mut self, used: bool) {
        self.is_used = used;
    }
}

impl DynamicEthercatDevice for Wago750_554 {}

impl EthercatDynamicPDO for Wago750_554 {
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

impl EthercatDevice for Wago750_554 {
    fn into_any_boxed(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

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
        output[base..(base + 16)].store_le::<u16>(self.rx_pdo.channel1);
        output[(base + 16)..(base + 32)].store_le::<u16>(self.rx_pdo.channel2);
        Ok(())
    }

    fn output_len(&self) -> usize {
        32
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
        input: &bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), anyhow::Error> {
        self.input(input)
    }

    fn output_checked(
        &self,
        output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), anyhow::Error> {
        self.output(output)
    }

    fn get_module(&self) -> Option<Module> {
        self.module.clone()
    }

    fn set_module(&mut self, module: Module) {
        self.tx_bit_offset = module.tx_offset;
        self.rx_bit_offset = module.rx_offset;
        self.module = Some(module)
    }
}

impl EthercatDeviceProcessing for Wago750_554 {}

impl NewEthercatDevice for Wago750_554 {
    fn new() -> Self {
        Self {
            is_used: false,
            tx_bit_offset: 0,
            rx_bit_offset: 0,
            module: None,
            rx_pdo: Wago750_554RxPdo::default(),
        }
    }
}

impl std::fmt::Debug for Wago750_554 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wago750_554")
    }
}

pub const WAGO_750_554_VENDOR_ID: u32 = 0x00000021;
pub const WAGO_750_554_PRODUCT_ID: u32 = 0x055442cd;
pub const WAGO_750_554_MODULE_IDENT: SubDeviceProductTuple =
    (WAGO_750_554_VENDOR_ID, WAGO_750_554_PRODUCT_ID);
