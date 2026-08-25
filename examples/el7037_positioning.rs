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

#[derive(Debug)]
enum State {
    Reset,
    WaitForReset,
    WaitForReady,
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
    println!("preop");

    // CoE config must happen in PreOp (before Op transition)
    let mut el7037 = EL7037::new();
    let mut config = EL7037Configuration::default();

    config.stm_features.operation_mode = EL70x1OperationMode::PositionController;
    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionControl;

    config.encoder.reversion_of_rotation = true;
    config.stm_motor.max_current = 1100;
    // config.stm_motor.motor_coil_resistance = 175;
    // config.stm_motor.motor_coil_inductance = 330;
    config.stm_motor.encoder_increments = 4000;
    // config.pos_configuration.target_window = 100;
    // config.pos_configuration.position_lag_max = 3;
    // config.pos_configuration.velocity_min = 100;
    // config.pos_configuration.velocity_max = 1000;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;

    // The HAL writes the position loop (0x8014) with the terminal's documented
    // Kp (pos.) of 500. Measured on this axis that gain is unstable - moves run
    // 3-4.7x too far - so pin it to the value that actually converges here, the
    // same one el7037_move_steps uses. CoE is persistent, so an example that
    // left 500 behind would destabilise the next run of any other example too.
    config.stm_controller_3.kp_factor_pos = 5;

    for subdevice in eth_control.controller.get_subdevices() {
        if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
            el7037
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &config,
                )
                .expect("EL7037 CoE config failed");

            // Kp factor pos.
            // eth_control
            //     .channel
            //     .sdo_write(subdevice.device_address, 0x8014, 0x02, 5u16)
            //     .expect("KP write failed");
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

        let enc_status = el7037.txpdo.enc_status.as_mut().expect("No Encoder Status");
        let stm_status = el7037.txpdo.stm_status.as_mut().expect("No STM Status");
        let position_lag = el7037
            .txpdo
            .pos_actual_position_lag
            .as_mut()
            .expect("No Position Lag");
        let stm_control = el7037.rxpdo.stm_control.as_mut().expect("No STM Control");
        let stm_position = el7037.rxpdo.stm_position.as_mut().expect("No STM Position");
        let stm_external_position = el7037
            .txpdo
            .stm_external_position
            .as_mut()
            .expect("No STM External Position");
        let stm_internal_position = el7037
            .txpdo
            .stm_internal_position
            .as_mut()
            .expect("No STM Internal Position");
        let enc_control = el7037
            .rxpdo
            .enc_control
            .as_mut()
            .expect("No Encoder Control");

        println!(
            "{:?}: READY = {}, READY_TO_ENABLE = {}, ENC = {}, MOV_POS={}, MOVE_NEG={}, ExPOS = {}, InPOS = {}, SET_CNT_DONE = {}, POS_LAG = {}",
            state,
            stm_status.ready,
            stm_status.ready_to_enable,
            enc_status.counter_value,
            stm_status.moving_positive,
            stm_status.moving_negative,
            stm_external_position.external_position,
            stm_internal_position.internal_position,
            enc_status.set_counter_done,
            position_lag.actual_position_lag,
        );

        if stm_status.error {
            panic!("Motor Error");
        }

        match state {
            State::Reset => {
                stm_control.reset = true;
                enc_control.set_counter = true;
                enc_control.set_counter_value = 1000;

                state = State::WaitForReset;
            }
            State::WaitForReset => {
                stm_control.reset = false;

                if enc_status.set_counter_done {
                    assert_eq!(enc_status.counter_value, 1000);

                    enc_control.set_counter = false;
                    stm_control.enable = true;
                    stm_position.position = enc_status.counter_value;

                    state = State::WaitForReady;
                }
            }
            State::WaitForReady => {
                if stm_status.ready {
                    state = State::Increment;
                }
            }
            State::Increment => {
                stm_position.position = 2000;
                stm_control.enable = true;

                // state = State::Decrement;
            }
            State::Decrement => {
                if enc_status.counter_value > 4000 {
                    stm_control.enable = stm_status.ready_to_enable;
                    stm_position.position = 4000;
                } else {
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
