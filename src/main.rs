use std::time::Duration;

use ethercat::start_ethercat_thread;
pub fn main() {
    let (result, _handle) = start_ethercat_thread("eth0");
    let ecat = result.0;
    let sender = result.1;
    println!("ECAT Controller Main Addr: {:p}", ecat);


    

    // When we requested OP go here
    loop {
        // Get the latest data 
        // perhaps add a freshness atomic bool to avoid hammering the cache when theres no new data to be read ?
        let inputs = ecat.get_inputs();
       std::thread::sleep(Duration::from_micros(10000));
    }
}
