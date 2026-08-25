use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState, devices::el7037::EL7037_PRODUCT_ID, init_ethercat,
};
use std::{env, time::Duration};

/// Reads back the EL7037's own CoE values and compares them against the
/// defaults in the EL70x7 documentation.
///
/// This exists because CoE objects are persistent in the terminal: anything a
/// previous commissioning session (or TwinCAT) wrote stays there. An object the
/// HAL does not write is therefore whatever somebody left behind, which is
/// invisible from the code. That is how this terminal was found running with
/// `0x8014:02` (position controller Kp) at 5 against a documented default of
/// 500 - the HAL had no configuration for `0x8014` at all.
///
/// Read-only: nothing is written and the motor is never enabled.
///
/// Usage: cargo run --example el7037_coe_dump -- <network-interface>
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.get_state() {
            EtherCATState::PreOp => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let mut address = None;
    for _ in 0..50 {
        for subdevice in eth_control.controller.get_subdevices() {
            if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
                address = Some(subdevice.device_address);
            }
        }
        if address.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let address = address.unwrap_or_else(|| panic!("no EL7037 found on {interface}"));
    println!("EL7037 at address {address}\n");

    let ch = eth_control.channel.clone();

    macro_rules! row {
        ($ty:ty, $index:expr, $sub:expr, $name:expr, $default:expr) => {{
            let got = ch.sdo_read::<$ty>(address, $index, $sub);
            match got {
                Ok(v) => {
                    let v = v as i64;
                    let d: i64 = $default;
                    let flag = if v == d { "  " } else { "<-" };
                    println!(
                        "  {:04X}:{:02X}  {:<24} {:>12}   doc default {:>12} {}",
                        $index, $sub, $name, v, d, flag
                    );
                }
                Err(e) => println!(
                    "  {:04X}:{:02X}  {:<24} read failed: {e}",
                    $index, $sub, $name
                ),
            }
        }};
    }

    println!("ENC Settings (0x8000)");
    row!(bool, 0x8000, 0x08, "Disable filter", 0);
    row!(bool, 0x8000, 0x0A, "Enable micro increments", 0);
    row!(bool, 0x8000, 0x0E, "Reversion of rotation", 0);

    println!("\nSTM Motor (0x8010)");
    row!(u16, 0x8010, 0x01, "Maximal current mA", 5000);
    row!(u16, 0x8010, 0x02, "Reduced current mA", 2500);
    row!(u16, 0x8010, 0x03, "Nominal voltage mV", 5000);
    row!(u16, 0x8010, 0x04, "Coil resistance 10mOhm", 100);
    row!(u16, 0x8010, 0x05, "Motor EMF", 0);
    row!(u16, 0x8010, 0x06, "Motor fullsteps", 200);
    row!(u16, 0x8010, 0x07, "Encoder increments", 4096);
    row!(u16, 0x8010, 0x09, "Start velocity", 0);
    row!(u16, 0x8010, 0x0A, "Coil inductance 10uH", 0);
    row!(u16, 0x8010, 0x10, "Drive on delay ms", 100);
    row!(u16, 0x8010, 0x11, "Drive off delay ms", 150);

    println!("\nSTM Controller current loop (0x8011)");
    row!(u16, 0x8011, 0x01, "Kp factor (curr.)", 150);
    row!(u16, 0x8011, 0x02, "Ki factor (curr.)", 10);

    println!("\nSTM Features (0x8012)");
    row!(u8, 0x8012, 0x01, "Operation mode", 0);
    row!(u8, 0x8012, 0x05, "Speed range", 1);
    row!(u8, 0x8012, 0x08, "Feedback type", 1);
    row!(bool, 0x8012, 0x09, "Invert motor polarity", 0);
    row!(bool, 0x8012, 0x0A, "Error on step lost", 0);

    // These four are the position and velocity loop - the loop that decides
    // how a move settles, and the one that was silently misconfigured.
    println!("\nSTM Controller position/velocity loop (0x8014)");
    row!(u32, 0x8014, 0x01, "Feed forward", 100_000);
    row!(u16, 0x8014, 0x02, "Kp factor (pos.)", 500);
    row!(u32, 0x8014, 0x03, "Kp factor (velo.)", 50);
    row!(u16, 0x8014, 0x04, "Tn (velo.)", 50_000);

    println!("\nPOS Settings (0x8020)");
    row!(i16, 0x8020, 0x01, "Velocity min", 100);
    row!(i16, 0x8020, 0x02, "Velocity max", 10_000);
    row!(u16, 0x8020, 0x03, "Acceleration pos", 1000);
    row!(u16, 0x8020, 0x04, "Acceleration neg", 1000);
    row!(u16, 0x8020, 0x05, "Deceleration pos", 1000);
    row!(u16, 0x8020, 0x06, "Deceleration neg", 1000);
    row!(u16, 0x8020, 0x07, "Emergency deceleration", 100);
    row!(u16, 0x8020, 0x0B, "Target window", 10);
    row!(u16, 0x8020, 0x0C, "In-Target timeout ms", 1000);
    row!(i16, 0x8020, 0x0D, "Dead time compensation", 50);
    row!(u32, 0x8020, 0x0E, "Modulo factor", 0);
    row!(u16, 0x8020, 0x10, "Position lag max", 0);

    println!("\nPOS Features (0x8021)");
    row!(u16, 0x8021, 0x01, "Start type", 1);
    row!(bool, 0x8021, 0x15, "Emergency stop on lag", 0);
    row!(bool, 0x8021, 0x16, "Enhanced diag history", 0);

    println!("\nSTM Diag data (0xA010)  - latched conditions, all should be 0");
    row!(bool, 0xA010, 0x01, "Saturated", 0);
    row!(bool, 0xA010, 0x02, "Over temperature", 0);
    row!(bool, 0xA010, 0x03, "Torque overload", 0);
    row!(bool, 0xA010, 0x04, "Under voltage", 0);
    row!(bool, 0xA010, 0x05, "Over voltage", 0);
    row!(bool, 0xA010, 0x06, "Short circuit", 0);
    row!(bool, 0xA010, 0x08, "No control power", 0);
    row!(bool, 0xA010, 0x09, "Misc error", 0);
    row!(bool, 0xA010, 0x0A, "Config not adopted", 0);
    row!(bool, 0xA010, 0x0B, "Motor stall", 0);
    row!(u8, 0xA010, 0x11, "Actual operation mode", 0);

    println!("\n'<-' marks a value that differs from the documented default.");
}
