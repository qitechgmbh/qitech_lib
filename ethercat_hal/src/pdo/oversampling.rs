use crate::pdo::{PdoObject, RxPdoObject};
use bitvec::{field::BitField, order::Lsb0, slice::BitSlice};
use ethercat_hal_derive::PdoObject as PdoObjectDerive;

/// PDO Object for oversampling analog output terminals (e.g. EL4732)
///
/// With OSFac=N, the terminal expects N consecutive i16 samples per channel
/// per EtherCAT cycle. The master fills all N slots each cycle.
///
/// N must be one of: 1, 2, 3, 4, 5, 8, 10, 16, 20, 25, 32, 40, 50, 100
/// (as defined in the EL4732 ESI DC OpModes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalogOutputOversample {
    pub samples: Vec<i16>,
}

impl AnalogOutputOversample {
    pub fn new(oversample_factor: usize) -> Self {
        Self {
            samples: vec![0i16; oversample_factor],
        }
    }

    pub fn oversample_factor(&self) -> usize {
        self.samples.len()
    }
}

impl Default for AnalogOutputOversample {
    fn default() -> Self {
        Self::new(1)
    }
}

impl PdoObject for AnalogOutputOversample {
    fn size(&self) -> usize {
        self.samples.len() * 16 // N samples × 16 bits each
    }
}

impl RxPdoObject for AnalogOutputOversample {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        for (i, &sample) in self.samples.iter().enumerate() {
            let start = i * 16;
            bits[start..start + 16].store_le(sample as u16);
        }
    }
}

/// PDO Object for oversampling terminals (EL4732, etc.)
/// Must be incremented by the master each EtherCAT cycle, or the terminal will fault.
#[derive(Debug, Clone, Default, PdoObjectDerive, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct CycleCount {
    pub value: u16,
}

impl RxPdoObject for CycleCount {
    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        bits[0..16].store_le(self.value);
    }
}

impl CycleCount {
    pub fn increment(&mut self) {
        self.value = self.value.wrapping_add(1);
    }
}
