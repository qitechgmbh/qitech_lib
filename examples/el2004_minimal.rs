use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID,
    devices::{
        EthercatDevice, NewEthercatDevice,
        el2004::{EL2004, EL2004_PRODUCT_ID},
    },
    init_ethercat,
    pdo::basic::BoolPdoObject,
};
use std::{env, time::Duration};

/*
    This example showcases a very bare bones example to toggle the leds on an EL2004
*/
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

    // During PreOp controller.subdevices is populated with name, device_address,product_id and other metadata
    eth_control
        .channel
        .request_state_change(ethercat_hal::EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.state {
            ethercat_hal::EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    eth_control
        .channel
        .request_state_change(ethercat_hal::EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.state {
            ethercat_hal::EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let mut el2004: EL2004 = EL2004::new();
    // We ONLY have outputs so no need to call get_inputs

    loop {
        let outputs = eth_control.app_handle.write_outputs();
        for subdevice in eth_control.controller.get_subdevices() {
            if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL2004_PRODUCT_ID {
                el2004.rxpdo.channel1 = Some(BoolPdoObject {
                    value: !el2004.rxpdo.channel1.expect("").value,
                });
                let _ = el2004.output(BitSlice::<u8, Lsb0>::from_slice_mut(outputs));
            }
        }
        eth_control.app_handle.send_outputs();
        std::thread::sleep(Duration::from_secs(1));
    }
}
