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

/*
    This example showcases a very bare bones example to toggle the leds on an EL2004
*/
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mode = env::args().nth(2).expect("No Op-Mode given (safeop,op)");

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
    let requested_state = ethercat_hal::EtherCATState::SafeOp;
    eth_control
        .channel
        .request_state_change(requested_state)
        .expect("Channel was not ready");

    loop {
        if eth_control.controller.state == requested_state {
            break;
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    if mode == "op" {
        eth_control
            .channel
            .request_state_change(EtherCATState::Op)
            .expect("Channel was not ready");
        loop {
            if eth_control.controller.state == EtherCATState::Op {
                break;
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    let mut el2004: EL2004 = EL2004::new();
    // Expected Behaviour: In SafeOp no LED should light up on the EL2004
    // While in OP one does
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
