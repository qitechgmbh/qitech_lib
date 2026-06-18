use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    devices::{
        EthercatDevice, NewEthercatDevice,
        el2004::{EL2004, EL2004_PRODUCT_ID},
    },
    init_ethercat,
    pdo::basic::BoolPdoObject,
};
use std::{env, time::Duration};

/*This example showcases a very bare bones example to toggle the leds on an EL2004*/
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);
    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");

    loop {
        let val = eth_control.app_handle.get_state();
        match val {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        match eth_control.app_handle.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let subdevices = eth_control.app_handle.try_get_subdevices_vec().unwrap();
    let mut el2004: EL2004 = EL2004::new();
    // We ONLY have outputs so no need to call get_inputs

    loop {
        match eth_control.app_handle.write_outputs() {
            Some(output) => {
                for subdevice in &subdevices {
                    if subdevice.vendor == BECKHOFF_VENDOR_ID
                        && subdevice.product_id == EL2004_PRODUCT_ID
                    {
                        el2004.rxpdo.channel1 = Some(BoolPdoObject {
                            value: !el2004.rxpdo.channel1.expect("").value,
                        });
                        let _ = el2004.output(BitSlice::<u8, Lsb0>::from_slice_mut(output));
                    }
                }
                eth_control.app_handle.send_outputs();
            }
            None => (),
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
