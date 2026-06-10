use bitvec::{slice::BitSlice};
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig, coe::ConfigurableDevice, devices::{
        EthercatDevice, NewEthercatDevice, el1259::{EL1259, EL1259_PRODUCT_ID}
    }, init_ethercat
};
use std::{env, time::Duration};

/// This example showcases a very bare bones example to toggle the leds on an EL1259
fn main() {
    let mut el1259: EL1259 = EL1259::new();

    let interface = env::args().nth(1).expect("No Interface-name given");

    let cycle_time_us: u64 = 1000;
    let dc_config = DcConfiguration {
        start_delay:  Duration::from_millis(100),
        sync0_period:  Duration::from_micros(cycle_time_us / 2),
        sync0_shift:  Duration::from_micros(cycle_time_us),
        target_dc_tick:  100,
    };

    let rt = RtOptimizationConfig {
        ethercat_loop_thread_core: 2,
        ethercat_loop_thread_priority: 99,
        ethercat_io_thread_core: 3,
        ethercat_io_thread_priority: 99,
        pin_irq_core: Some(3),
    };

    let config = MasterConfiguration {
        target_cycle_time_us: cycle_time_us as usize,
        tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxBlocking,
        dc_config,
        realtime_optimizations: Some(rt),
    };

    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    loop {
        let val = eth_control.controller.get_state();
        match val {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
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
        match eth_control.controller.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    loop {
        if let Some(input) = eth_control.app_handle.get_inputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.product_id == EL1259_PRODUCT_ID {
                    let input = &input[subdevice.start_tx..subdevice.end_tx];
                    el1259.input(BitSlice::from_slice(input)).expect("Failed to read input");
                }
            }
        }

        // logic

        if let Some(output) = eth_control.app_handle.write_outputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.product_id == EL1259_PRODUCT_ID {
                    let output = &mut output[subdevice.start_rx..subdevice.end_rx];
                    el1259.output(BitSlice::from_slice_mut(output)).expect("Failed to write output");
                }
            }

            eth_control.app_handle.send_outputs();
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
