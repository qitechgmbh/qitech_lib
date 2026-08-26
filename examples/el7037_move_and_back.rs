//! Minimal example: home, then move back and forth between a target given on
//! the command line and home, indefinitely (Ctrl-C to stop). The smallest
//! program that drives the closed loop from `el7037_velocity_closed_loop.rs`
//! via `ethercat_hal::helpers::velocity_position_loop::VelocityPositionLoop` -
//! see that file for the control law, tunables and units.
//!
//! Skips its direction self-check to stay minimal; the loop's own runaway
//! guard still aborts a move driven the wrong way by backwards feedback, and
//! stops the program rather than continuing to cycle into a fault.
//!
//! Usage: `cargo run --example el7037_move_and_back -- <interface> <counts>`

use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7037::{
            EL7037, EL7037_PRODUCT_ID, coe::EL7037Configuration, pdo::EL7037PredefinedPdoAssignment,
        },
    },
    helpers::{
        el70xx_velocity_converter::EL70x1VelocityConverter,
        velocity_position_loop::{LoopEvent, VelocityPositionLoop, VelocityPositionLoopConfig},
    },
    init_ethercat,
    io::stepper_velocity_el70x1::StepperVelocityEL70x1Device,
    shared_config::el70x7::{EL70x1OperationMode, EL70x1SpeedRange, EL7037FeedbackType},
};
use std::{
    env,
    time::{Duration, Instant},
};

const CYCLE_TIME: Duration = Duration::from_nanos(250_000);
/// 4000 counts/rev over a 200 full-step motor (igus MOT-AN-S-060-005-042-L-C-AAAO).
const COUNTS_PER_FULL_STEP: f64 = 4000.0 / 200.0;
const MAX_FULL_STEPS_PER_S: f64 = 2000.0;

fn axis_config() -> EL7037Configuration {
    let mut config = EL7037Configuration::default();
    config.stm_features.operation_mode = EL70x1OperationMode::DirectVelocity;
    config.stm_features.speed_range = EL70x1SpeedRange::Steps2000;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;
    // Measured on this rig - see el7037_velocity_closed_loop.rs.
    config.encoder.reversion_of_rotation = true;

    config.stm_motor.max_current = 1100;
    config.stm_motor.reduced_current = 550;
    config.stm_motor.nominal_voltage = 24000;
    config.stm_motor.motor_coil_resistance = 175;
    config.stm_motor.motor_coil_inductance = 330;
    config.stm_motor.motor_full_steps = 200;
    config.stm_motor.encoder_increments = 4000;

    config.stm_controller_1.kp_factor = 150;
    config.stm_controller_1.ki_factor = 10;
    config.stm_controller_3.feed_forward_pos = 100_000;
    config.stm_controller_3.kp_factor_pos = 2;
    config.stm_controller_3.kp_factor_velo = 50;
    config.stm_controller_3.tn_velo = 50_000;

    config.pdo_assignment = EL7037PredefinedPdoAssignment::VelocityControlCompact;
    config
}

fn command_speed(el7037: &mut EL7037, counts_per_s: f64) {
    let steps = (counts_per_s / COUNTS_PER_FULL_STEP).clamp(-MAX_FULL_STEPS_PER_S, MAX_FULL_STEPS_PER_S);
    let converter = EL70x1VelocityConverter::new(&el7037.get_speed_range(0));
    if let Ok(mut output) = el7037.get_output(0) {
        output.velocity = converter.steps_to_velocity(steps, true);
        let _ = el7037.set_output(0, output);
    }
}

/// Cycles `axis` towards its current target until it arrives, times out or
/// runs away. Returns whether it arrived.
fn run_to_target(axis: &mut VelocityPositionLoop, el7037: &mut EL7037, cycle: &mut impl FnMut(&mut EL7037)) -> bool {
    let mut last = Instant::now();
    loop {
        cycle(el7037);
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f64().clamp(0.000_005, 0.05);
        last = now;

        let position = el7037.get_position(0) as i64;
        let stalled = el7037.txpdo.stm_status.as_ref().is_some_and(|s| s.motor_stall);
        let (speed, event) = axis.step(position, stalled, now, dt);
        command_speed(el7037, speed);

        match event {
            Some(LoopEvent::Arrived { .. }) => return true,
            Some(LoopEvent::Runaway { .. } | LoopEvent::TimedOut { .. }) => return false,
            _ => {}
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let usage = "usage: el7037_move_and_back <interface> <counts>";
    let interface = args.next().expect(usage);
    let target: i64 = args.next().expect(usage).parse().expect("counts must be an integer");

    let mut eth = init_ethercat(&interface, None);
    eth.channel.request_state_change(EtherCATState::PreOp).expect("Channel was not ready");
    while eth.app_handle.get_state() != EtherCATState::PreOp {
        std::thread::sleep(Duration::from_millis(10));
    }

    let mut el7037 = EL7037::new();
    let mut address = None;
    for _ in 0..50 {
        address = eth
            .app_handle
            .try_get_subdevices_vec_sync()
            .expect("Failed to read subdevices!")
            .iter()
            .find(|s| s.vendor == BECKHOFF_VENDOR_ID && s.product_id == EL7037_PRODUCT_ID)
            .map(|s| s.device_address);
        if address.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let address = address.unwrap_or_else(|| panic!("no EL7037 found on {interface}"));
    el7037
        .write_config(eth.channel.clone(), address, &axis_config())
        .expect("EL7037 CoE config failed");

    eth.channel.request_state_change(EtherCATState::Op).expect("Channel was not ready");
    while eth.app_handle.get_state() != EtherCATState::Op {
        std::thread::sleep(Duration::from_millis(10));
    }

    let subdevice = eth
        .app_handle
        .try_get_subdevices_vec_sync()
        .expect("Failed to read subdevices!")
        .into_iter()
        .find(|s| s.device_address == address)
        .expect("EL7037 disappeared during the Op transition");
    let (start_tx, end_tx) = (subdevice.start_tx, subdevice.end_tx);
    let (start_rx, end_rx) = (subdevice.start_rx, subdevice.end_rx);

    let mut cycle = |el7037: &mut EL7037| {
        if let Some(input) = eth.app_handle.get_inputs() {
            let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(&input[start_tx..end_tx]));
            let _ = el7037.input_post_process();
            eth.app_handle.finish_read();
        }
        let _ = el7037.output_pre_process();
        if let Some(output) = eth.app_handle.write_outputs() {
            let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(&mut output[start_rx..end_rx]));
            eth.app_handle.send_outputs();
        }
        std::thread::sleep(CYCLE_TIME);
    };

    // Wait for the drive to come up, then enable.
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        cycle(&mut el7037);
        let input = el7037.get_input(0).expect("velocity input unavailable");
        if input.ready_to_enable {
            el7037.set_enabled(0, true);
        }
        if input.ready && el7037.is_enabled(0) {
            break;
        }
        assert!(Instant::now() <= deadline, "drive never became ready");
    }

    // Home: this program's zero is wherever the shaft happens to be at startup.
    el7037.set_position(0, 0);
    println!("homed, driver stage ready");

    let mut axis = VelocityPositionLoop::new(VelocityPositionLoopConfig::default(), 0, Instant::now());
    'moves: loop {
        for (to, label) in [(target, "target"), (0, "home")] {
            let position = el7037.get_position(0) as i64;
            println!("moving to {to} ({label})");
            axis.start_move(to, position, Instant::now());
            if !run_to_target(&mut axis, &mut el7037, &mut cycle) {
                println!("did not reach {label}; stopping");
                break 'moves;
            }
            println!("arrived at {to}");
        }
    }

    // Ramp to a stop rather than dropping the drive at speed.
    while !axis.stopped() {
        cycle(&mut el7037);
        let speed = axis.ramp_down(CYCLE_TIME.as_secs_f64());
        command_speed(&mut el7037, speed);
    }
    el7037.set_enabled(0, false);
    for _ in 0..50 {
        cycle(&mut el7037);
    }
    let _ = eth.channel.request_state_change(EtherCATState::PreOp);
    println!("done");
}
