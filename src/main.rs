use std::time::Duration;

use ethercat::start_ethercat_thread;
pub fn main() {
    let (ecat, _handle) = start_ethercat_thread("eth0");
    println!("ECAT Controller Main Addr: {:p}", ecat);
    loop {
        // Get the latest data 
        // perhaps add a freshness atomic bool to avoid hammering the cache when theres no new data to be read ?
        let inputs = ecat.get_inputs();
       std::thread::sleep(Duration::from_micros(10000));
    }
}
