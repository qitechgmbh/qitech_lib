use std::{any::TypeId, collections::HashMap};
const MAX_DATA_LEN: usize = 2048; // Adjust to your largest struct size

#[derive(Clone, PartialEq, Eq, Copy, Debug, Hash)]
pub struct MachineIdentificationUnique {
    pub machine_ident: MachineIdentification,
    pub serial: u32,
}

#[derive(Clone, PartialEq, Eq, Copy, Debug, Hash)]
pub struct MachineIdentification {
    pub vendor: u16,
    pub machine: u16,
}

impl MachineIdentificationUnique {
    pub fn as_u64(&self) -> u64 {
        // Pack 16 + 16 + 32 bits into one 64-bit integer
        ((self.machine_ident.vendor as u64) << 48)
            | ((self.machine_ident.machine as u64) << 32)
            | (self.serial as u64)
    }
}

#[repr(align(64))]
pub struct MachineData {
    pub type_id: TypeId,
    pub length: usize,
    pub data: [u8; MAX_DATA_LEN],
}

/*
    How should the raw data look like?
    Initial state should be all zeroes
    Endianness: Little endian
    first 4 bytes: vendor,machine,serial
    then 4 bytes data length: 256 for example
    then the data: of 256 bytes

*/
pub struct MachineDataRegistry {
    // Each slot is a fixed-size buffer
    pub storage: HashMap<MachineIdentificationUnique, MachineData>,
}

impl MachineDataRegistry {
    pub fn zero_entry(&mut self, ident: MachineIdentificationUnique) {
        match self.storage.get_mut(&ident) {
            Some(m) => {
                m.length = 0;
                m.data.fill(0);
            }
            None => (),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MachineError {
    RecoverableFailure(String),
    IrrecoverableFailure(String), // irrecoverable
}

pub trait Machine {
    fn act(&mut self, machine_data: Option<&mut MachineDataRegistry>) -> Result<(), MachineError>;
    fn react(&mut self, registry: &MachineDataRegistry);
    fn get_identification(&self) -> MachineIdentificationUnique;
}
