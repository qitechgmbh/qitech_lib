use bitvec::slice::BitSlice;
use ethercat_hal::{
    EtherCATState, coe::{RX_PDO_ASSIGNMENT_REG, TX_PDO_ASSIGNMENT_REG}, devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el1259::{EL1259, EL1259_PRODUCT_ID},
    }, init_ethercat, io::multi_timestamp::{MultiTimestampEvent, MultiTimestampOutput},
};
use std::{env, time::Duration};
const INIT_DELAY_NS: u64 = 20_000_000;
/// How long each LED stays on before the next one lights up
const STEP_NS: u64 = 250_000_000;
const N_CHANNELS: usize = 8;
/// Time for one full sweep over all channels
const PERIOD_NS: u64 = STEP_NS * N_CHANNELS as u64;

#[derive(Debug, Default)]
struct Channel {
    pulse_start_ns: u64,
}

/// This example showcases a very bare bones example to cascade the leds on an EL1259
/// from channel 1 to channel 8, one LED at a time
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
            let sm_writes = el1259.get_sm_coe_writes(subdevice.device_address).unwrap();
//            println!("{:?}",sm_writes );
            let res = eth_control.channel.sdo_read::<u8>(subdevice.device_address, TX_PDO_ASSIGNMENT_REG, 0).unwrap();
            println!("tx SDO Len: {}",res);
            for i in 1..res {
                let res = eth_control.channel.sdo_read::<u16>(subdevice.device_address, TX_PDO_ASSIGNMENT_REG, i).unwrap();
                println!("{}:{} : {} {:X}",TX_PDO_ASSIGNMENT_REG,i,res,res);
            }
            let res = eth_control.channel.sdo_read::<u8>(subdevice.device_address, RX_PDO_ASSIGNMENT_REG, 0).unwrap();
            println!("rx SDO Len: {}",res);
            for i in 1..res {
                let res = eth_control.channel.sdo_read::<u16>(subdevice.device_address, RX_PDO_ASSIGNMENT_REG, i).unwrap();
                println!("{}:{} : {} {:X}",RX_PDO_ASSIGNMENT_REG,i,res,res);
            }
            let res = eth_control.channel.bulk_sdo_write(sm_writes,Duration::from_secs(10));
            println!("{:?}",res );
            eth_control
                .channel
                .enable_dc_sync0(subdevice.device_address)
                .expect("Failed to enable DC Sync!");
        }
    }

    std::thread::sleep(Duration::from_millis(1000));

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");

    'outer: loop {
        std::thread::sleep(Duration::from_millis(10));
        let res = eth_control.channel.register_read(0x1001,0x0134 as u16);
        println!("ALSTATUS: {:?} {:?}",res,eth_control.join_handle.as_ref().unwrap().is_finished() );
        
        for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
            if !subdevice.initialized {
                continue 'outer;
            }
        }

        break;
    }

    let dc_system_start_ns = eth_handle.get_dc_sys_time_ns();
    println!("DC System Start Time {} ns", dc_system_start_ns);
    // Every channel uses the same timing, shifted by one step per channel
    for (i, channel) in channels.iter_mut().enumerate() {
        channel.pulse_start_ns = dc_system_start_ns + INIT_DELAY_NS + i as u64 * STEP_NS;
    }

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    loop {
        while eth_handle.check_inputs_ready() == false {}
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
            // Once a pulse has started, schedule the same channel's pulse for the next sweep
            if channel.pulse_start_ns < eth_handle.get_dc_sys_time_ns() {
                channel.pulse_start_ns = channel.pulse_start_ns.wrapping_add(PERIOD_NS);
                let pulse_end_ns = channel.pulse_start_ns.wrapping_add(STEP_NS);

                el1259.push(
                    channel_index,
                    MultiTimestampEvent {
                        value: true,
                        dc_timestamp_ns: channel.pulse_start_ns,
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
