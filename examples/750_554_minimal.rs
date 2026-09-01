use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    EtherCATState,
    devices::{
        EthercatDevice, NewEthercatDevice,
        wago_modules::{
            wago_750_354::{WAGO_750_354_IDENTITY_A, Wago750_354},
            wago_750_455::Wago750_455,
            wago_750_554::Wago750_554,
        },
    },
    init_ethercat,
    io::{analog_input::AnalogInputDevice, analog_output::AnalogCurrentOutputDevice},
};
use std::{env, time::Duration};

fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);
    let mut eth_handle = eth_control.app_handle;

    // Transition to PreOp
    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        match eth_handle.get_state() {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    println!(
        "Found {:?} EtherCAT terminals",
        eth_handle.get_subdevice_count()
    );

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    for sdev in &subdevices {
        println!(" - {:?}", sdev);
    }

    // Find the Wago 750-354 coupler subdevice
    let coupler_subdev = subdevices
        .iter()
        .find(|s| {
            s.vendor == WAGO_750_354_IDENTITY_A.0 && s.product_id == WAGO_750_354_IDENTITY_A.1
        })
        .expect("No Wago 750-354 coupler found on the bus");

    // Initialize the coupler and discover its bus modules
    let mut coupler = Wago750_354::new();
    let modules =
        Wago750_354::initialize_modules(eth_control.channel.clone(), coupler_subdev.device_address)
            .expect("Failed to initialize coupler modules");
    for module in &modules {
        println!(
            "  Found module: {} (product_id: 0x{:08x})",
            module.name, module.product_id
        );
    }
    for module in modules {
        coupler.set_module(module);
    }
    coupler.init_slot_modules(eth_control.channel.clone(), coupler_subdev.device_address);

    // Find the 750-554 and 750-455 slots
    let ao_slot = coupler
        .slot_devices
        .iter()
        .position(|s| {
            s.as_ref().map_or(false, |d| {
                d.as_any().downcast_ref::<Wago750_554>().is_some()
            })
        })
        .expect("No Wago 750-554 found: is it registered in init_slot_modules?");
    let ai_slot = coupler
        .slot_devices
        .iter()
        .position(|s| {
            s.as_ref().map_or(false, |d| {
                d.as_any().downcast_ref::<Wago750_455>().is_some()
            })
        })
        .expect("No Wago 750-455 found: is it registered in init_slot_modules?");

    println!("750-554 in slot {ao_slot}, 750-455 in slot {ai_slot}, starting ramp...");

    // Transition to Op
    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to go into OP");
    loop {
        match eth_handle.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    let coupler_subdev = subdevices
        .iter()
        .find(|s| {
            s.vendor == WAGO_750_354_IDENTITY_A.0 && s.product_id == WAGO_750_354_IDENTITY_A.1
        })
        .expect("No Wago 750-354 coupler found on the bus");

    // Main loop: ramp AO 1 up and AO 2 down over 16 steps, read back via the 455
    let ramp_steps: u32 = 16;
    for step in 0..=u32::MAX {
        let normalized = (step % ramp_steps) as f64 / (ramp_steps - 1) as f64;

        // Set the analog outputs
        {
            let slot_dev = coupler.slot_devices[ao_slot]
                .as_mut()
                .expect("No device in AO slot");
            let ao = slot_dev
                .as_any_mut()
                .downcast_mut::<Wago750_554>()
                .expect("AO slot is not a Wago 750-554");
            ao.set_output_relative(0, normalized);
            ao.set_output_relative(1, 1.0 - normalized);
        }

        // Write outputs to the EtherCAT bus
        if let Some(outputs) = eth_handle.write_outputs() {
            let subdevice_outputs = &mut outputs[coupler_subdev.start_rx..coupler_subdev.end_rx];
            coupler
                .output(BitSlice::<u8, Lsb0>::from_slice_mut(subdevice_outputs))
                .expect("Failed to write Rx PDO");
        }
        eth_handle.send_outputs();

        // Let the DAC and ADC settle
        std::thread::sleep(Duration::from_millis(250));

        // Read inputs from the EtherCAT bus
        if let Some(inputs) = eth_handle.get_inputs() {
            let subdevice_inputs = &inputs[coupler_subdev.start_tx..coupler_subdev.end_tx];
            coupler
                .input(BitSlice::<u8, Lsb0>::from_slice(subdevice_inputs))
                .expect("Failed to read Tx PDO");
        }

        // Read back the measured current from the 455
        let slot_dev = coupler.slot_devices[ai_slot]
            .as_ref()
            .expect("No device in AI slot");
        let ai = slot_dev
            .as_any()
            .downcast_ref::<Wago750_455>()
            .expect("AI slot is not a Wago 750-455");

        let cmd1 = 4.0 + normalized * 16.0;
        let cmd2 = 4.0 + (1.0 - normalized) * 16.0;

        match (ai.get_input(0), ai.get_input(1)) {
            (Ok(a), Ok(b)) => {
                let m1 = 4.0 + a.normalized * 16.0;
                let m2 = 4.0 + b.normalized * 16.0;
                println!(
                    "step {:3}: AO1 {:.1} mA -> AI1 {:.1} mA | AO2 {:.1} mA -> AI2 {:.1} mA",
                    step, cmd1, m1, cmd2, m2
                );
            }
            _ => println!("step {:3}: read error", step),
        }
    }
}
