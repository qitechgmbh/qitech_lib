use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::coe::ConfigurableDevice;
use ethercat_hal::devices::el3024::{EL3024_IDENTITY_A, EL3024Configuration};
use ethercat_hal::ethercat_helpers::{sdo_read_helper, sdo_write_helper};
use ethercat_hal::{ChannelRequest, ChannelResponse, EtherCATState, EtherCATThreadResponseChannel, start_ethercat_thread};
use ethercat_hal::devices::{EthercatDevice, NewEthercatDevice, el1008::{self, EL1008, EL1008_IDENTITY_A}, el2004::{EL2004, EL2004_IDENTITY_A}, el3024::{self, EL3024}};
use std::time::Duration;

pub fn main() {
    let (result, _handle) = start_ethercat_thread("enp101s0f3u1u2");
    let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
    let response_channel : EtherCATThreadResponseChannel = EtherCATThreadResponseChannel(tx);
    let ecat = result.0;
    let sender = result.1;

    let mut el1008 : EL1008 = EL1008::new();
    let mut el2004 : EL2004 = EL2004::new();
    let mut el3024 : EL3024 = EL3024::new();
    let mut initialized = false;

    loop {
        let _inputs = ecat.get_inputs();
        std::thread::sleep(Duration::from_millis(1));

        if let EtherCATState::Init = ecat.state {
            let req: ChannelRequest = ChannelRequest {
                channel_request: ethercat_hal::ChannelRequests::ChangeState(
                    ethercat_hal::EtherCATState::PreOp,
                ),
                response_channel: response_channel.clone(),
            };
            let _res = sender.clone().0.send(req);
            let _res = rx.recv();
            std::thread::sleep(Duration::from_millis(1000));
        }

        if let EtherCATState::PreOp = ecat.state {
            for dev in ecat.subdevices {
                if dev.device_address == 0 {
                    break;
                }
                println!();
                
                if dev.product_id == EL3024_IDENTITY_A.1 && !initialized {
                    el3024.write_config(sender.clone(), dev.device_address, &EL3024Configuration::default());
                    println!("Configuring EL3024");
                    initialized = true;
                }

                let res = sdo_write_helper::<u32>(sender.clone(), dev.device_address, 0xF008, 0, 0x12345678);
                let res = sdo_read_helper::<u32>(sender.clone(), dev.device_address, 0xF008, 0);
                println!("res {:?}",res)
            }




            let req: ChannelRequest = ChannelRequest {
                channel_request: ethercat_hal::ChannelRequests::ChangeState(
                    ethercat_hal::EtherCATState::Op,
                ),
                response_channel: response_channel.clone(),
            };
            let _res = sender.clone().0.send(req);
            std::thread::sleep(Duration::from_millis(1000));
        }

        if let EtherCATState::Op = ecat.state {
          //  println!("entered op with {} subdevices",ecat.subdevice_count);
            
            let inputs = ecat.get_inputs();
            let mut outputs = ecat.get_outputs();

            for dev in ecat.subdevices {
                if !dev.initialized {
                    break;
                }

                let input_slice = &inputs[dev.start_tx..dev.end_tx];
                let input_bits = BitSlice::<u8, Lsb0>::from_slice(input_slice);
                
                let output_slice = &mut outputs[dev.start_rx..dev.end_rx];
                let output_bits = BitSlice::<u8, Lsb0>::from_slice_mut(output_slice);

                if dev.product_id == EL1008_IDENTITY_A.1 {
                    let _res = el1008.input(input_bits);
                }

                if dev.product_id == EL3024_IDENTITY_A.1 {
                    let _res = el3024.input(input_bits);
                    let val = match el3024.txpdo.ai_standard_channel1 {
                        Some(ref v) => v,
                        None => continue,
                    };
                }

                if dev.product_id == EL2004_IDENTITY_A.1 {
                    let chan = &mut el2004.rxpdo.channel2;
                    match chan {
                        Some(c) => c.value = true,
                        None => (),
                    };
                    let _res = el2004.output(output_bits);
                    ecat.finish_write();
                }
              //  println!("cycle time: {:?}", ecat.cycle_time_us);
            }
        }
    }
}