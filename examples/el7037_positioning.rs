use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        el7037::{
            EL7037, EL7037_PRODUCT_ID, coe::EL7037Configuration, pdo::EL7037PredefinedPdoAssignment,
        },
    },
    init_ethercat,
    shared_config::el70x7::{EL70x1OperationMode, EL7037FeedbackType},
};
use std::{env, time::Duration};

enum State {
    Reset,
    WaitForReset,
    Increment,
    Decrement,
}

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

    config.stm_features.operation_mode = EL70x1OperationMode::Automatic;
    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionControl;

    config.stm_motor.max_current = 1100;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;

    for subdevice in eth_control.controller.get_subdevices() {
        if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
            el7037
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &config,
                )
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

    let mut state = State::Reset;

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
                }
            }

            eth_control.app_handle.finish_read();
        }

        let stm_control = el7037.rxpdo.stm_control.as_mut().expect("No STM Control");
        let enc_status = el7037.txpdo.enc_status.as_mut().expect("No Encoder Status");
        let stm_position = el7037.rxpdo.stm_position.as_mut().expect("No STM Position");
        let enc_control = el7037
            .rxpdo
            .enc_control
            .as_mut()
            .expect("No Encoder Control");

        println!("POS = {}", enc_status.counter_value);

        match state {
            State::Reset => {
                stm_control.reset = true;
                enc_control.set_counter = true;
                enc_control.set_counter_value = 10000;

                state = State::WaitForReset;
            }
            State::WaitForReset => {
                stm_control.reset = false;
                enc_control.set_counter = false;

                if enc_status.counter_value == 10000 {
                    state = State::Increment;
                }
            }
            State::Increment => {
                stm_control.enable = true;
                stm_position.position = 12000;

                if enc_status.counter_value == 12000 {
                    state = State::Decrement;
                }
            }
            State::Decrement => {
                stm_control.enable = true;
                stm_position.position = 11000;

                if enc_status.counter_value == 11000 {
                    state = State::Increment;
                }
            }
        }

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
