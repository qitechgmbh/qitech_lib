pub struct MachineIdentificationUnique {
	pub vendor : u16,
	pub machine : u16,
	pub serial : u32,
}
pub trait MachineData{}
pub trait Machine {
	fn act(&mut self, machine_data : &mut MachineData);
	fn get_identification(&self) -> MachineIdentificationUnique;
}