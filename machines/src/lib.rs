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

pub struct MachineData {
    pub type_id: TypeId,
    pub length: usize,
    pub data: [u8; MAX_DATA_LEN],
}

impl Default for MachineData {
    fn default() -> Self {
        Self {
            type_id: TypeId::of::<()>(),
            length: 0,
            data: [0u8; 2048],
        }
    }
}

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

    pub fn store<T: ConvertMachineData + Default>(
        &mut self,
        ident: MachineIdentificationUnique,
        value: &T,
    ) -> Result<(), &'static str> {
        self.storage.insert(ident, MachineData::default());
        let v = self.storage.get_mut(&ident);
        let machine_data = match v {
            Some(v) => v,
            None => return Err("Failed to insert machine_data"),
        };
        value.to_machine_data(machine_data)?;
        Ok(())
    }

    pub fn load<T: ConvertMachineData + Default>(
        &self,
        ident: &MachineIdentificationUnique,
    ) -> Result<T, &'static str> {
        let machine_data = self.storage.get(ident).ok_or("No entry for ident")?;
        let mut result: T = T::default();
        T::from_machine_data(machine_data, &mut result)?;
        Ok(result)
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

pub trait ConvertMachineData: Sized + 'static {
    fn to_machine_data(&self, data: &mut MachineData) -> Result<(), &'static str>;
    fn from_machine_data(machine_data: &MachineData, out: &mut Self) -> Result<(), &'static str>;
}
