use ethercat::{ChannelRequest, ChannelResponse, EtherCATState, EtherCATThreadResponseChannel, start_ethercat_thread};
use ethercat_hal::devices::{NewEthercatDevice, el3024::{self, EL3024}};
use std::time::Duration;

pub fn main() {
    let (result, _handle) = start_ethercat_thread("enp101s0f4u1u2");
    let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>();
    let response_channel : EtherCATThreadResponseChannel = EtherCATThreadResponseChannel(tx);
    let ecat = result.0;
    let sender = result.1;
    let _el3024 : EL3024 = el3024::EL3024::new();
    
    loop {
        let _inputs = ecat.get_inputs();
        std::thread::sleep(Duration::from_millis(1));

        if let EtherCATState::Init = ecat.state {
            let req: ChannelRequest = ChannelRequest {
                channel_request: ethercat::ChannelRequests::ChangeState(
                    ethercat::EtherCATState::PreOp,
                ),
                response_channel: response_channel.clone(),
            };
            let _res = &sender.0.send(req);
            let _res = rx.recv();
            std::thread::sleep(Duration::from_millis(1000));
        }

        if let EtherCATState::PreOp = ecat.state {
            let req: ChannelRequest = ChannelRequest {
                channel_request: ethercat::ChannelRequests::ChangeState(
                    ethercat::EtherCATState::Op,
                ),
                response_channel: response_channel.clone(),
            };
            let _res = sender.0.send(req);
            std::thread::sleep(Duration::from_millis(1000));
        }
    }
}