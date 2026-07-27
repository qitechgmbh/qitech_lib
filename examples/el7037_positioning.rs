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

    config.stm_features.operation_mode = EL70x1OperationMode::ExtendedPositionController;
    // Include StmSynchronInfoData in TxPDO so MotorLoad + MotorDcCurrent are available
    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionControl;

    config.stm_motor.max_current = 1100;

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

    for step in 0.. {
        // --- Read inputs (TxPDO: encoder + motor status from device) ---
        if let Some(input) = eth_control.app_handle.get_inputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let tx_bytes = &input[subdevice.start_tx..subdevice.end_tx];
                    let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(tx_bytes));
                    let _ = el7037.input_post_process();
                }
            }

            eth_control.app_handle.finish_read();
        }

        let enc_status = el7037.txpdo.enc_status.as_mut().expect("No ENC Status");

        println!("Counter = {}", enc_status.counter_value);

        let stm_position = el7037.rxpdo.stm_position.as_mut().expect("No STM Position");
        let stm_control = el7037.rxpdo.stm_control.as_mut().expect("No STM Control");
        stm_control.enable = true;

        // if step % 100 == 0 {
        //     stm_position.position += 5000;
        // }

        stm_position.position = step * 10;

        // if mode == 0 {
        //     if stm_position.position == enc_status.counter_value {
        //         mode = 1;
        //     }

        //     stm_position.position = 0;
        // } else {
        //     stm_position.position = 20000;
        // }

        // --- Write outputs (RxPDO: velocity + control bits to device) ---
        let _ = el7037.output_pre_process();

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

        std::thread::sleep(Duration::from_millis(10));
    }
}
