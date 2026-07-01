use machines::{ConvertMachineData, MachineData};
use postcard::from_bytes;
use postcard::to_slice;
use serde::{Deserialize, Serialize};
use std::any::TypeId;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MockData {
    bytes_0_255: Vec<u8>,
    str_data: String,
    stack_bytes: [u8; 32],
    max_u64: u64,
    min_u64: u64,
}

/*
    You have to supply your own implementation when converting from and to the given type
    you can do it with json,yaml,bincode whatever best fits your use case
*/
impl ConvertMachineData for MockData {
    fn to_machine_data(&self, data: &mut MachineData) -> Result<(), &'static str> {
        let serialized_bytes =
            to_slice(self, &mut data.data).map_err(|_| "Postcard serialization failed")?;
        data.type_id = TypeId::of::<Self>();
        data.length = serialized_bytes.len();
        Ok(())
    }

    fn from_machine_data(machine_data: &MachineData, out: &mut Self) -> Result<(), &'static str> {
        if machine_data.type_id != TypeId::of::<Self>() {
            return Err("Typeid Mismatch");
        }
        if machine_data.length == 0 {
            return Err("Empty buffer data");
        }
        let deserialized: Self =
            from_bytes(&machine_data.data).map_err(|_| "Postcard deserialization failed")?;
        *out = deserialized;
        Ok(())
    }
}

fn main() {
    let mut mock = MockData {
        stack_bytes: [0u8; 32],
        max_u64: u64::MAX,
        min_u64: u64::MIN,
        bytes_0_255: vec![],
        str_data: "str".to_owned(),
    };
    for i in 0..255 {
        if i < 32 {
            mock.stack_bytes[i as usize] = i;
        }
    }
    println!("before: {:?}", mock);
    let mut machine_data = MachineData::default();
    mock.to_machine_data(&mut machine_data).unwrap();
    let mut reconstructed_mock = MockData::default();
    let _ = MockData::from_machine_data(&machine_data, &mut reconstructed_mock);
    println!("reconstructed_mock: {:?}", reconstructed_mock);
}
