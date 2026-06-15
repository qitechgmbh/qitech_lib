use bitvec::{slice::BitSlice};
use ethercat_hal::{
    EtherCATState, coe::ConfigurableDevice, devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice, el1259::{EL1259, EL1259_PRODUCT_ID}
    }, init_ethercat, io::multi_timestamp::{MultiTimestampEvent, MultiTimestampOutput}
};
use std::{env, time::Duration};

/// This example showcases a very bare bones example to toggle the leds on an EL1259
fn main() {
    let mut el1259: EL1259 = EL1259::new();

    let interface = env::args().nth(1).expect("No Interface-name given");

    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    loop {
        if matches!(eth_control.controller.state, EtherCATState::PreOp) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    for subdevice in eth_control.controller.get_subdevices() {
        if subdevice.product_id == EL1259_PRODUCT_ID {
            el1259.write_config(eth_control.channel.clone(), subdevice.device_address, &el1259.get_config()).expect("Failed to write config");
            eth_control.channel.enable_dc_sync0(subdevice.device_address).expect("Failed to enable DC Sync!");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");

    loop {
        if matches!(eth_control.controller.state, EtherCATState::Op) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    loop {
        if let Some(input) = eth_control.app_handle.get_inputs() {

            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.product_id == EL1259_PRODUCT_ID {
                    let input = &input[subdevice.start_tx..subdevice.end_tx];
                    el1259.input(BitSlice::from_slice(input)).expect("Failed to read input");
                    el1259.input_post_process().expect("Failed to process inputs");
                }
            }
        }

        let event = MultiTimestampEvent {
            value: true,
            dc_timestamp_ns: eth_control.controller.get_dc_system_time_ns().wrapping_add(Duration::from_secs(1).as_nanos() as u64),
        };
        el1259.push(0, event);

        if let Some(output) = eth_control.app_handle.write_outputs() {

            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.product_id == EL1259_PRODUCT_ID {
                    el1259.output_pre_process().expect("Failed to prepare outputs");
                    let output = &mut output[subdevice.start_rx..subdevice.end_rx];
                    el1259.output(BitSlice::from_slice_mut(output)).expect("Failed to write output");
                }
            }
        }

        eth_control.app_handle.send_outputs();

        std::thread::sleep(Duration::from_secs(1));
    }
}
