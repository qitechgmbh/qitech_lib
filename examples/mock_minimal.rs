use bitvec::{order::Lsb0, slice::BitSlice};
use qitech_lib_ethercat_hal::{
    EthercatMaster, MetaSubdevice, io::digital_input::DigitalInputDevice, mock::MockMaster,
};
use qitech_lib_ethercat_hal_devices::{
    EthercatDevice, IdentifiedDevice, NewEthercatDevice, beckhoff_modules::el1002::EL1002,
};
use std::time::Duration;

/// This example runs an EL1002 and a plain 8 bit output terminal on a mock master, so it needs
/// no hardware. It mirrors the inputs of the EL1002 onto the outputs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The mock assigns the address and process image offsets, from the subdevice it only takes
    // the identity and the PDO sizes (1 byte of inputs for the EL1002, 1 byte of outputs).
    let mut master = MockMaster::new();
    

    let el1002_address = master.add_subdevice(MetaSubdevice {
        vendor: EL1002::IDENTITY.vendor,
        product_id: EL1002::IDENTITY.product,
        end_tx: 1,
        ..Default::default()
    });
    let output_address = master.add_subdevice(MetaSubdevice {
        vendor: 0x2,
        product_id: 0x1234,
        end_rx: 1,
        ..Default::default()
    });

    // Take the master to OP like the state machine does
    master.init("mock0")?;
    master.into_pre_op_pdi(Duration::from_millis(1))?;
    master.into_safe_op()?;
    master.request_into_op()?;

    let subdevices = master.get_subdevices();
    for sdev in &subdevices {
        println!(" - {:?}", sdev);
    }

    let mut el1002 = EL1002::new();
    let mut inputs = [0u8; 1];
    let mut outputs = [0u8; 1];
    for iter in 0..4u8 {
        // Flip the switches on the EL1002
        master.subdevice_inputs_mut(el1002_address).unwrap()[0] = iter & 0b11;
        master.tx_rx()?;
        master.read_inputs(&mut inputs);

        let mut mirrored = 0u8;
        for subdevice in &subdevices {
            if EL1002::IDENTITY.matches(subdevice) {
                el1002.input(BitSlice::<u8, Lsb0>::from_slice(
                    &inputs[subdevice.start_tx..subdevice.end_tx],
                ))?;
                for i in 0..el1002.get_port_count() {
                    mirrored |= (el1002.get_input(i)? as u8) << i;
                }
            }
        }
        for subdevice in &subdevices {
            if subdevice.device_address == output_address {
                outputs[subdevice.start_rx..subdevice.end_rx].fill(mirrored);
            }
        }

        // The outputs reach the terminal with the next cycle
        master.write_outputs(&outputs);
        master.tx_rx()?;
        println!(
            "EL1002 inputs {:#04b} -> outputs {:#04b}",
            iter & 0b11,
            master.subdevice_outputs(output_address).unwrap()[0]
        );
    }
    Ok(())
}
