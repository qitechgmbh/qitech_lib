use std::time::Duration;

use ethercat::{EtherCATState, start_ethercat_thread};
pub fn main() {
    let (result, _handle) = start_ethercat_thread("eth0");
    let ecat = result.0;
    let sender = result.1;
    println!("ECAT Controller Main Addr: {:p}", ecat);
    
    let res = sender.send(ethercat::ChannelRequest::ChangeState((ethercat::EtherCATState::PreOp)));
    let _state = &ecat.state;
    
    while !matches!(EtherCATState::PreOp,_state)  {
        println!("State actually is {:?}", _state);
    }

    // When we requested OP go here
    loop {
        // Get the latest data 
        // perhaps add a freshness atomic bool to avoid hammering the cache when theres no new data to be read ?
//        let inputs = ecat.get_inputs();
       std::thread::sleep(Duration::from_micros(10000));
    }
}
