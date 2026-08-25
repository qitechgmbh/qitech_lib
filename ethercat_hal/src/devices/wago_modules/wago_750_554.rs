use bitvec::field::BitField;

use crate::devices::{
    DynamicEthercatDevice, EthercatDevice, EthercatDeviceProcessing, EthercatDeviceUsed,
    EthercatDynamicPDO, Module, NewEthercatDevice, SubDeviceProductTuple,
};
use crate::io::analog_output::{AnalogOutputDevice, AnalogOutputOutput};
use units::electric_current::milliampere;
use units::f64::ElectricCurrent;

/// Full scale of the output word: 0x0000 -> 4 mA, 0x7FFF -> 20 mA.
const WAGO_750_554_MAX_RAW: u16 = 0x7FFF;

/// The value is represented with 12 bit resolution on B3..B14,
/// the three least significant bits (B0..B2) are not parsed by the module.
const WAGO_750_554_VALUE_MASK: u16 = 0x7FF8;

const WAGO_750_554_MIN_MA: f64 = 4.0;
const WAGO_750_554_MAX_MA: f64 = 20.0;

#[derive(Clone, Debug)]
pub enum Wago750_554Port {
    AO1,
    AO2,
}

impl From<Wago750_554Port> for usize {
    fn from(value: Wago750_554Port) -> Self {
        match value {
            Wago750_554Port::AO1 => 0,
            Wago750_554Port::AO2 => 16,
        }
    }
}

#[derive(Clone, Default)]
pub struct Wago750_554RxPdo {
    pub ao1: u16,
    pub ao2: u16,
}

#[derive(Clone)]
pub struct Wago750_554 {
    is_used: bool,
    tx_bit_offset: usize,
    rx_bit_offset: usize,
    module: Option<Module>,
    rx_pdo: Wago750_554RxPdo,
}

impl Wago750_554 {
    /// Converts a normalized value (0.0 ..= 1.0) into the raw output word.
    fn normalized_to_raw(normalized: f32) -> u16 {
        let normalized = normalized.clamp(0.0, 1.0);
        let raw = (normalized * WAGO_750_554_MAX_RAW as f32).round() as u16;
        raw & WAGO_750_554_VALUE_MASK
    }

    /// Converts a raw output word back into a normalized value (0.0 ..= 1.0).
    fn raw_to_normalized(raw: u16) -> f32 {
        (raw & WAGO_750_554_VALUE_MASK) as f32 / WAGO_750_554_MAX_RAW as f32
    }

    /// Currently written raw output word of the given port.
    pub fn get_raw(&self, port: usize) -> Result<u16, anyhow::Error> {
        match port {
            0 => Ok(self.rx_pdo.ao1),
            1 => Ok(self.rx_pdo.ao2),
            _ => Err(anyhow::anyhow!("port {} doesnt exist on Wago750_554", port)),
        }
    }

    /// Currently written output of the given port as a normalized value.
    pub fn get_output(&self, port: usize) -> Result<AnalogOutputOutput, anyhow::Error> {
        Ok(AnalogOutputOutput(Self::raw_to_normalized(
            self.get_raw(port)?,
        )))
    }

    /// Sets the output of the given port as an absolute current.
    /// Values outside of 4 mA ... 20 mA are clamped.
    pub fn set_current(
        &mut self,
        port: usize,
        current: ElectricCurrent,
    ) -> Result<(), anyhow::Error> {
        if port >= self.get_port_count() {
            return Err(anyhow::anyhow!("port {} doesnt exist on Wago750_554", port));
        }
        let ma = current.get::<milliampere>();
        let normalized = (ma - WAGO_750_554_MIN_MA) / (WAGO_750_554_MAX_MA - WAGO_750_554_MIN_MA);
        self.set_output(port, AnalogOutputOutput(normalized as f32));
        Ok(())
    }

    /// Currently written output of the given port as an absolute current.
    pub fn get_current(&self, port: usize) -> Result<ElectricCurrent, anyhow::Error> {
        let normalized = Self::raw_to_normalized(self.get_raw(port)?) as f64;
        let ma = WAGO_750_554_MIN_MA + normalized * (WAGO_750_554_MAX_MA - WAGO_750_554_MIN_MA);
        Ok(ElectricCurrent::new::<milliampere>(ma))
    }
}

impl AnalogOutputDevice for Wago750_554 {
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput) {
        let raw = Self::normalized_to_raw(value.0);
        match port {
            0 => self.rx_pdo.ao1 = raw,
            1 => self.rx_pdo.ao2 = raw,
            _ => (),
        }
    }

    fn get_port_count(&self) -> usize {
        2
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
        // The status byte of this module is always zero and is not mapped.
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
