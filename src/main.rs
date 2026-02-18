use std::time::Duration;

use ethercat::{ChannelRequest, ChannelResponse, EtherCATState, start_ethercat_thread};




pub fn business_logic() {

}


pub fn main() {
    let (result, _handle) = start_ethercat_thread("enp101s0f4u1u2");
    let (tx, rx) = std::sync::mpsc::channel::<ChannelResponse>(); 

    let ecat = result.0;
    let sender = result.1;
    println!("ECAT Controller Main Addr: {:p}", ecat);

    // When we requested OP go here
    loop {
        // Get the latest data 
        // perhaps add a freshness atomic bool to avoid hammering the cache when theres no new data to be read ?
        let inputs = ecat.get_inputs();
        std::thread::sleep(Duration::from_millis(1));

        if let EtherCATState::Init = ecat.state {
            let req : ChannelRequest = ChannelRequest{
                channel_request: ethercat::ChannelRequests::ChangeState(ethercat::EtherCATState::PreOp), 
                response_channel: Some(tx.clone()), 
            };

            let res = sender.send(req);
            println!("sent request");
           // println!("{:?}",res);
            let res = rx.recv();
            println!("hello {:?}",res);
            std::thread::sleep(Duration::from_millis(1000));
        }

        if let EtherCATState::PreOp = ecat.state {
           let req : ChannelRequest = ChannelRequest{
                channel_request: ethercat::ChannelRequests::ChangeState(ethercat::EtherCATState::Op), 
                response_channel: Some(tx.clone()), 
            };
            let res = sender.send(req);
            println!("{:?}",res);
            std::thread::sleep(Duration::from_millis(1000));
        }
    }
}
