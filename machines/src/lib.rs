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

    pub fn store<T: ConvertMachineData>(
        &mut self,
        ident: MachineIdentificationUnique,
        value: &T,
    ) -> Result<(), &'static str> {
        let machine_data = value.to_machine_data()?;
        self.storage.insert(ident, machine_data);
        Ok(())
    }

    pub fn load<T: ConvertMachineData>(
        &self,
        ident: &MachineIdentificationUnique,
    ) -> Result<T, &'static str> {
        let machine_data = self.storage.get(ident).ok_or("No entry for ident")?;
        T::from_machine_data(machine_data)
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
    fn to_machine_data(&self) -> Result<MachineData, &'static str> {
        let size = std::mem::size_of::<Self>();
        if size > MAX_DATA_LEN {
            return Err("Data exceeds MAX_DATA_LEN");
        }
        let mut data = MachineData {
            type_id: TypeId::of::<Self>(),
            length: size,
            data: [0u8; MAX_DATA_LEN],
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                data.data.as_mut_ptr(),
                size,
            );
        }

        Ok(data)
    }

    fn from_machine_data(machine_data: &MachineData) -> Result<Self, &'static str> {
        if machine_data.type_id != TypeId::of::<Self>() {
            return Err("TypeId mismatch");
        }

        if machine_data.length != std::mem::size_of::<Self>() {
            return Err("Length mismatch");
        }

        unsafe { Ok(std::ptr::read(machine_data.data.as_ptr() as *const Self)) }
    }
}
