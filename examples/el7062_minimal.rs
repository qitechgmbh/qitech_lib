/*
    EL7062 2-channel stepper motor output stage in Cyclic Synchronous Position mode (CSP).

    This example assumes a motor (200 full steps/rev, 1.8 A/phase) and an encoder
    (WEDL5541-A14, RS422 differential, 500 counts/rev after 4-fold evaluation)
    connected to channel 1, running closed-loop with commutation type 17.

    This is a SAFETY-HARDENED smoke test: conservative current, speed, acceleration
    and following-error limits are applied, and the axis is moved back and forth
    between 0 and the target position with a trapezoidal setpoint ramp, so the motor
    can be verified to spin without risking the hardware.

    usage: el7062_minimal <interface> <cycle_time_us> <target_position_increments>
    example (1 rev = 500 increments): ./target/release/examples/el7062_minimal enp4s0 1000 500
*/

use bitvec::slice::BitSlice;
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7062::{
            EL7062, EL7062_PRODUCT_ID, EL7062Port,
            pdo::{DrvControlWord, DrvStatusWord},
        },
    },
    init_ethercat, set_current_thread_rt_priority,
};
use std::{env, time::Duration};

const USAGE: &str = concat!(
    "el7062_minimal interface_name cycle_time_us target_position_increments\n",
    " example: ./target/release/examples/el7062_minimal enp4s0 1000 2000"
);

fn apply_rt() {
    let id = core_affinity::CoreId { id: 2 };
    set_current_thread_rt_priority(99);
    core_affinity::set_for_current(id);
}

/// Drive the CiA402 state machine via the control word.
/// Bits on the EL7062: 0=switch on, 1=enable voltage, 3=enable operation, 7=fault reset.
fn apply_controlword(
    el7062: &mut EL7062,
    statusword: &DrvStatusWord,
) -> Result<(), Box<dyn std::error::Error>> {
    if statusword.fault {
        el7062.set_controlword(
            EL7062Port::Ch1,
            DrvControlWord {
                fault_reset: true,
                ..Default::default()
            },
        )?;
    } else {
        // Switch on + enable voltage + enable operation.
        el7062.set_controlword(
            EL7062Port::Ch1,
            DrvControlWord {
                switch_on: true,
                enable_voltage: true,
                enable_operation: true,
                fault_reset: false,
            },
        )?;
    }
    Ok(())
}

/// Simple trapezoidal setpoint ramp in encoder increments.
///
/// Generates a smooth 3-segment profile (accelerate / cruise / decelerate) so the
/// drive is never asked to jump straight to the target position.
struct SetpointRamp {
    position: f64,
    velocity: f64,
}

/// 5 rev/s = 300 rev/min at 500 CPR.
const RAMP_MAX_VELOCITY_INCR_PER_S: f64 = 2500.0;
/// 10 rev/s² at 500 CPR (~63 rad/s²).
const RAMP_ACCEL_INCR_PER_S2: f64 = 5000.0;

impl SetpointRamp {
    fn new(initial_position: i32) -> Self {
        Self {
            position: initial_position as f64,
            velocity: 0.0,
        }
    }

    fn advance(&mut self, target: f64, dt: f64) {
        let remaining = target - self.position;
        if remaining.abs() < 1e-3 {
            self.position = target;
            self.velocity = 0.0;
            return;
        }
        let dir = remaining.signum();
        let dist = remaining.abs();
        // Cap the velocity so we can still stop within the remaining distance.
        let v_brake = (2.0 * RAMP_ACCEL_INCR_PER_S2 * dist).sqrt();
        let v_cmd = dir * RAMP_MAX_VELOCITY_INCR_PER_S.min(v_brake);
        let dv = (v_cmd - self.velocity)
            .clamp(-RAMP_ACCEL_INCR_PER_S2 * dt, RAMP_ACCEL_INCR_PER_S2 * dt);
        self.velocity += dv;

        let next = self.position + self.velocity * dt;
        // Do not overshoot the target.
        if (target - self.position).signum() != (target - next).signum() {
            self.position = target;
            self.velocity = 0.0;
        } else {
            self.position = next;
        }
    }
}

fn main() {
    let fail = format!("{}:\n{}", "Invalid arguments", USAGE);
    let interface = env::args().nth(1).expect(&fail);
    let cycle_time_us: u64 = env::args()
        .nth(2)
        .expect(&fail)
        .parse()
        .expect("cycle_time_us must be a valid u64");
    let target_position: i32 = env::args()
        .nth(3)
        .expect(&fail)
        .parse()
        .expect("target_position_increments must be a valid i32");

    let dc_config = DcConfiguration {
        // Give some headroom for dc setup to finish
        start_delay: Duration::from_millis(100),
        sync0_period: Duration::from_micros(cycle_time_us),
        sync0_shift: Duration::from_micros(cycle_time_us / 2),
        target_dc_tick: 500,
    };

    let rt = RtOptimizationConfig {
        ethercat_loop_thread_core: 3,
        ethercat_loop_thread_priority: 50,
        ethercat_io_thread_core: 3,
        ethercat_io_thread_priority: 99,
        pin_irq_core: Some(3),
        lock_memory: true,
    };

    let config = MasterConfiguration {
        target_cycle_time_us: cycle_time_us as usize,
        tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxIoUring,
        dc_config,
        realtime_optimizations: Some(rt),
        wkc_mismatch_threshold: 5,
        op_ramp_grace_cycles: 10000,
    };

    let eth_control = init_ethercat(&interface, Some(config));
    let mut eth_handle = eth_control.app_handle;

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        if matches!(eth_handle.get_state(), EtherCATState::PreOp) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    // Build the driver with the closed-loop encoder+motor configuration for channel 1.
    let mut el7062 = EL7062::new();
    el7062.configuration.channel_1.feedback.encoder_type = 1; // RS422 differential
    el7062
        .configuration
        .channel_1
        .feedback
        .encoder_increments_per_revolution = 500; // CPR (after 4x)
    el7062.configuration.channel_1.amplifier.commutation_type = 17; // stepper with encoder
    el7062.configuration.channel_1.motor.rated_current = 1800; // 1.8 A per phase
    el7062
        .configuration
        .channel_1
        .motor
        .configured_motor_current = 1800;
    el7062
        .configuration
        .channel_1
        .motor
        .motor_full_steps_per_revolution = 200;

    // Safety limits for the smoke test (tune later).
    el7062
        .configuration
        .channel_1
        .amplifier
        .enable_input_cycle_counter = true; // fault on lost cycles
    el7062.configuration.channel_1.amplifier.velocity_limitation = 300; // 1/min (rev-equivalent)
    el7062
        .configuration
        .channel_1
        .amplifier
        .following_error_window = 1000; // increments (~2 rev)
    el7062
        .configuration
        .channel_1
        .amplifier
        .following_error_timeout = 100; // ms
    el7062
        .configuration
        .channel_1
        .amplifier
        .acceleration_limitation = 2000; // 0.1 rad/s²

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    for subdevice in &subdevices {
        if subdevice.product_id == EL7062_PRODUCT_ID {
            el7062
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &el7062.get_config(),
                )
                .expect("Failed to write config");
            eth_control
                .channel
                .enable_dc_sync0(subdevice.device_address)
                .expect("Failed to enable DC Sync!");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    std::thread::sleep(Duration::from_millis(4000));
    loop {
        if eth_handle.check_all_op() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    apply_rt();

    let initial_position = {
        while !eth_handle.check_inputs_ready() {}
        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    el7062
                        .input(BitSlice::from_slice(
                            &inputs[subdevice.start_tx..subdevice.end_tx],
                        ))
                        .expect("Failed to read input");
                }
            }
        }
        el7062.get_position(EL7062Port::Ch1).unwrap_or(0)
    };

    // Ramp the setpoint instead of jumping to the target, and bounce between 0 and the target.
    let mut ramp = SetpointRamp::new(initial_position);
    let mut go_to: f64 = target_position as f64;
    let dt = cycle_time_us as f64 * 1e-6;
    let mut last_cycle = eth_handle.get_current_cycle();
    println!(
        "EL7062 CSP smoke test: cycle {}us, bouncing between 0 and {} increments",
        cycle_time_us, target_position
    );
    loop {
        while !eth_handle.check_inputs_ready() {}

        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    let input = &inputs[subdevice.start_tx..subdevice.end_tx];
                    el7062
                        .input(BitSlice::from_slice(input))
                        .expect("Failed to read input");
                    el7062
                        .input_post_process()
                        .expect("Failed to process input");
                }
            }
        }

        let statusword = el7062
            .get_statusword(EL7062Port::Ch1)
            .expect("Failed to read statusword");
        apply_controlword(&mut el7062, &statusword).expect("Failed to write controlword");

        if statusword.operation_enabled {
            ramp.advance(go_to, dt);
            el7062
                .set_target_position(EL7062Port::Ch1, ramp.position as i32)
                .expect("Failed to write target position");
            if (go_to - ramp.position).abs() < 0.5 {
                go_to = if (go_to - target_position as f64).abs() < 0.5 {
                    0.0
                } else {
                    target_position as f64
                };
            }
        }

        if let Some(outputs) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    el7062
                        .output_pre_process()
                        .expect("Failed to prepare output");
                    let output = &mut outputs[subdevice.start_rx..subdevice.end_rx];
                    el7062
                        .output(BitSlice::from_slice_mut(output))
                        .expect("Failed to write output");
                }
            }
        }
        eth_handle.send_outputs();

        let current_cycle = eth_handle.get_current_cycle();
        if current_cycle.wrapping_sub(last_cycle) >= 1000 {
            last_cycle = current_cycle;
            let position = el7062
                .get_position(EL7062Port::Ch1)
                .expect("Failed to read position");
            let following_error = el7062
                .get_following_error(EL7062Port::Ch1)
                .expect("Failed to read following error");
            println!(
                "pos={} following_error={} op_enabled={} fault={}",
                position, following_error, statusword.operation_enabled, statusword.fault
            );
        }
    }
}
