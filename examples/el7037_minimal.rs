use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        el7037::{EL7037, EL7037_PRODUCT_ID, coe::EL7037Configuration, pdo::EL7037PredefinedPdoAssignment},
    },
    init_ethercat,
    io::stepper_velocity_el70x1::StepperVelocityEL70x1Device,
    shared_config::el70x7::EL70x1OperationMode,
};
use std::{env, time::Duration};

/// Minimal example: spin the motor on an EL7037 at ±1000 steps/s,
/// switching direction every 200 cycles (~2 s at 10 ms per cycle).
/// Prints encoder position and status bits each cycle.
///
/// Usage: cargo run --example el7037_minimal -- <network-interface>
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.get_state() {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    // CoE config must happen in PreOp (before Op transition)
    let mut el7037 = EL7037::new();
    let mut config = EL7037Configuration::default();

    config.stm_features.operation_mode = EL70x1OperationMode::DirectVelocity;
    // Include StmSynchronInfoData in TxPDO so MotorLoad + MotorDcCurrent are available
    config.pdo_assignment = EL7037PredefinedPdoAssignment::VelocityControlCompactWithInfoData;

    for subdevice in eth_control.controller.get_subdevices() {
        if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
            el7037
                .write_config(eth_control.channel.clone(), subdevice.device_address, &config)
                .expect("EL7037 CoE config failed");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let mut step = 0u32;

    loop {
        // --- Read inputs (TxPDO: encoder + motor status from device) ---
        if let Some(input) = eth_control.app_handle.get_inputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let tx_bytes = &input[subdevice.start_tx..subdevice.end_tx];
                    let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(tx_bytes));
                    let _ = el7037.input_post_process();

                    let state = el7037.get_input(0).unwrap();
                    let load = el7037
                        .txpdo
                        .stm_synchron_info_data
                        .as_ref()
                        .map(|d| (d.info_data_1, d.info_data_2));
                    println!(
                        "step={:4}  pos={:8}  ready={}  error={}  moving+/-={}/{}  load={}  dc_current={}mA",
                        step,
                        state.counter_value,
                        state.ready,
                        state.error,
                        state.moving_positive,
                        state.moving_negative,
                        load.map_or(0, |(l, _)| l),
                        load.map_or(0, |(_, c)| c),
                    );
                }
            }
            eth_control.app_handle.finish_read();
        }

        // Alternate direction every 200 cycles
        let target_steps_per_sec: f64 = if (step / 200).is_multiple_of(2) {
            500.0
        } else {
            -500.0
        };
        el7037.set_speed(0, target_steps_per_sec).unwrap();
        el7037.set_enabled(0, true);
        let _ = el7037.output_pre_process();

        // --- Write outputs (RxPDO: velocity + control bits to device) ---
        if let Some(output) = eth_control.app_handle.write_outputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let rx_bytes = &mut output[subdevice.start_rx..subdevice.end_rx];
                    let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(rx_bytes));
                }
            }
            eth_control.app_handle.send_outputs();
        }

        step = step.wrapping_add(1);
        std::thread::sleep(Duration::from_millis(10));
    }
}
