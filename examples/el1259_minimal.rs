use bitvec::slice::BitSlice;
use ethercat_hal::{
    EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        el1259::{EL1259, EL1259_PRODUCT_ID},
    },
    init_ethercat,
    io::multi_timestamp::{MultiTimestampEvent, MultiTimestampOutput},
};
use std::{env, time::Duration};

const INIT_DELAY_NS: u64 = 20_000_000;
const PULSE_DELAY_NS: u64 = 200_000;
const PULSE_WIDTH_NS: u64 = 50_000;
const BURST_DELAY_NS: u64 = 50_000_000;

const PULSES_PER_BURST: usize = 5;
const N_CHANNELS: usize = 8;

#[derive(Debug, Default)]
struct Channel {
    burst_start_ns: u64,
    burst_delay_ns: u64,
    pulse_width_ns: u64,
    pulse_delay_ns: u64,
}

/// This example showcases a very bare bones example to toggle the leds on an EL1259
fn main() {
    let mut channels: [Channel; N_CHANNELS] = Default::default();
    let mut el1259: EL1259 = EL1259::new();
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);
    let mut eth_handle = eth_control.app_handle;

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    loop {
        if matches!(eth_handle.get_state(), EtherCATState::PreOp) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
        if subdevice.product_id == EL1259_PRODUCT_ID {
            el1259
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &el1259.get_config(),
                )
                .expect("Failed to write config");

            eth_control
                .channel
                .enable_dc_sync0(subdevice.device_address)
                .expect("Failed to enable DC Sync!");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");

    'outer: loop {
        std::thread::sleep(Duration::from_millis(10));
        for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
            if !subdevice.initialized {
                continue 'outer;
            }
        }
        break;
    }

    let dc_system_start_ns = eth_handle.get_dc_sys_time_ns();
    println!("DC System Start Time {} ns", dc_system_start_ns);
    for (i, channel) in channels.iter_mut().enumerate() {
        channel.burst_start_ns = dc_system_start_ns + INIT_DELAY_NS;
        channel.burst_delay_ns = BURST_DELAY_NS * (1 << i) as u64;
        channel.pulse_width_ns = PULSE_WIDTH_NS * (1 << i) as u64;
        channel.pulse_delay_ns = PULSE_DELAY_NS * (1 << i) as u64;
    }

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    loop {
        if let Some(input) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL1259_PRODUCT_ID {
                    let input = &input[subdevice.start_tx..subdevice.end_tx];
                    el1259
                        .input(BitSlice::from_slice(input))
                        .expect("Failed to read input");
                    el1259
                        .input_post_process()
                        .expect("Failed to process input");
                }
            }
        }

        for (channel_index, channel) in channels.iter_mut().enumerate() {
            if channel.burst_start_ns < eth_handle.get_dc_sys_time_ns() {
                channel.burst_start_ns =
                    channel.burst_start_ns.wrapping_add(channel.burst_delay_ns);

                for pulse_index in 0..PULSES_PER_BURST {
                    let pulse_begin_ns = channel
                        .burst_start_ns
                        .wrapping_add(pulse_index as u64 * channel.pulse_delay_ns);
                    let pulse_end_ns = pulse_begin_ns.wrapping_add(channel.pulse_width_ns);

                    el1259.push(
                        channel_index,
                        MultiTimestampEvent {
                            value: true,
                            dc_timestamp_ns: pulse_begin_ns,
                        },
                    );
                    el1259.push(
                        channel_index,
                        MultiTimestampEvent {
                            value: false,
                            dc_timestamp_ns: pulse_end_ns,
                        },
                    );
                }
            }
        }

        if let Some(output) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL1259_PRODUCT_ID {
                    el1259
                        .output_pre_process()
                        .expect("Failed to prepare output");
                    let output = &mut output[subdevice.start_rx..subdevice.end_rx];
                    el1259
                        .output(BitSlice::from_slice_mut(output))
                        .expect("Failed to write output");
                }
            }
        }
        eth_handle.send_outputs();
    }
}
