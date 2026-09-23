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
    DcConfiguration, EtherCATState, EtherCATThreadChannel, MasterConfiguration,
    RtOptimizationConfig,
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
use log::{debug, error, info, warn};
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

fn dump_dc_registers(channel: &EtherCATThreadChannel, addr: u16) {
    let pairs: [(u16, &str); 11] = [
        (0x0980, "AssignActivate(classic)"),
        (0x0981, "DcSyncActive(module)"),
        (0x0984, "StartTime-classic-lo"),
        (0x0985, "StartTime-classic-hi"),
        (0x0988, "Sync0Cycle-classic-lo"),
        (0x0989, "Sync0Cycle-classic-hi"),
        (0x098C, "Sync1Cycle-classic-lo"),
        (0x098D, "Sync1Cycle-classic-hi"),
        (0x0990, "StartTime-module-lo"),
        (0x09A0, "Sync0Cycle-module-lo"),
        (0x09A4, "Sync1Cycle-module-lo"),
    ];
    for (reg, label) in pairs {
        match channel.register_read(addr, reg) {
            Ok(v) => debug!("  {label} @0x{reg:04X} = 0x{v:04x}"),
            Err(e) => debug!("  {label} @0x{reg:04X} read failed: {e}"),
        }
    }
}

/// Drive the CiA402 state machine via the control word.
/// Bits on the EL7062: 0=switch on, 1=enable voltage, 3=enable operation, 7=fault reset.
fn apply_controlword(
    el7062: &mut EL7062,
    statusword: &DrvStatusWord,
) -> Result<(), Box<dyn std::error::Error>> {
    if statusword.fault {
        warn!("Fault detected! Applying fault reset to EL7062 (Channel 1)");
        el7062.set_controlword(
            EL7062Port::Ch1,
            DrvControlWord {
                fault_reset: true,
                ..Default::default()
            },
        )?;
    } else {
        // Switch on + enable voltage + enable operation.
        debug!("Applying controlword: switch_on, enable_voltage, enable_operation");
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
    // Force early logger initialization
    env_logger::Builder::from_default_env()
        .format_timestamp_micros()
        .init();

    // Immediately log to confirm logging works
    debug!("Logger initialized successfully");

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

    // Force flush logs
    log::logger().flush();
    info!("Starting EL7062 minimal example");
    info!("Interface: {}", interface);
    info!("Cycle time: {} µs", cycle_time_us);
    info!("Target position: {} increments", target_position);

    let dc_config = DcConfiguration {
        // Give headroom for DC setup to finish
        start_delay: Duration::from_millis(100),
        sync0_period: Duration::from_micros(cycle_time_us),
        sync0_shift: Duration::ZERO,
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

    info!("Initializing EtherCAT master on interface: {}", interface);
    debug!("Creating EtherCAT master configuration");

    debug!(
        "MasterConfiguration: target_cycle_time_us={}, tx_rx_config={:?}, wkc_mismatch_threshold={}",
        cycle_time_us,
        ethercat_hal::MasterTxRxConfig::TxRxIoUring,
        5
    );
    debug!(
        "DC Configuration: start_delay={:?}, sync0_period={:?}, sync0_shift={:?}",
        dc_config.start_delay, dc_config.sync0_period, dc_config.sync0_shift
    );
    debug!(
        "RT Optimization: ethercat_loop_thread_core={}, ethercat_loop_thread_priority={}",
        rt.ethercat_loop_thread_core, rt.ethercat_loop_thread_priority
    );

    let config = MasterConfiguration {
        target_cycle_time_us: cycle_time_us as usize,
        tx_rx_config: ethercat_hal::MasterTxRxConfig::TxRxIoUring,
        dc_config,
        realtime_optimizations: Some(rt),
        wkc_mismatch_threshold: 5,
        op_ramp_grace_cycles: 10000,
    };
    log::logger().flush();

    debug!("EtherCAT master configuration created");
    log::logger().flush();

    debug!("Initializing EtherCAT master with interface: {}", interface);
    let eth_control = init_ethercat(&interface, Some(config));
    info!("EtherCAT master initialized");
    log::logger().flush();

    // Ensure the master is fully initialized
    info!("Waiting for EtherCAT master to stabilize...");
    std::thread::sleep(Duration::from_millis(1000));
    let mut eth_handle = eth_control.app_handle;

    // Wait for Init state
    info!("Waiting for EtherCAT master to reach Init state...");
    let mut current_state = eth_handle.get_state();
    while !matches!(current_state, EtherCATState::Init) {
        debug!("Current EtherCAT state: {:?}", current_state);
        std::thread::sleep(Duration::from_millis(100));
        current_state = eth_handle.get_state();
    }
    info!("EtherCAT master reached Init state");
    log::logger().flush();
    
    // Request PreOp state
    info!("Requesting EtherCAT state transition: -> PreOp");
    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Failed to request state change to PreOp");
    
    // Wait for PreOp state
    info!("Waiting for PreOp state...");
    let mut attempts = 0;
    let max_attempts = 100; // ~100 seconds timeout (100 * 1000ms)
    while attempts < max_attempts {
        current_state = eth_handle.get_state();
        debug!("Current EtherCAT state: {:?}", current_state);
        if matches!(current_state, EtherCATState::PreOp) {
            info!("EtherCAT master reached PreOp state");
            log::logger().flush();
            break;
        }
        attempts += 1;
        std::thread::sleep(Duration::from_millis(1000));
    }
    
    if attempts >= max_attempts {
        error!("Timeout waiting for PreOp state! Check device configuration.");
        return;
    }

    info!("Configuring EL7062 driver for Channel 1");

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

    debug!(
        "EL7062 configuration: encoder_type={}, CPR={}, commutation_type={}",
        el7062.configuration.channel_1.feedback.encoder_type,
        el7062
            .configuration
            .channel_1
            .feedback
            .encoder_increments_per_revolution,
        el7062.configuration.channel_1.amplifier.commutation_type
    );

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

    info!(
        "EL7062 safety limits: velocity={} rev/min, following_error_window={} increments, acceleration={} rad/s²",
        el7062.configuration.channel_1.amplifier.velocity_limitation,
        el7062
            .configuration
            .channel_1
            .amplifier
            .following_error_window,
        el7062
            .configuration
            .channel_1
            .amplifier
            .acceleration_limitation as f64
            / 10.0
    );

    info!("Checking for EtherCAT subdevices...");
    log::logger().flush();

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    info!("Detected {} subdevices", subdevices.len());

    let mut el7062_found = false;
    for subdevice in &subdevices {
        debug!(
            "Subdevice: Product ID={:#X}, Device Address={}",
            subdevice.product_id, subdevice.device_address
        );
        log::logger().flush();

        if subdevice.product_id == EL7062_PRODUCT_ID {
            el7062_found = true;
            info!(
                "EL7062 device detected at address: {}",
                subdevice.device_address
            );
        }
    }

    if !el7062_found {
        error!("EL7062 device not found! Check power and connection.");
        log::logger().flush();
        return;
    }

    if subdevices.is_empty() {
        error!("No EtherCAT subdevices detected! Check network connection.");
        log::logger().flush();
        return;
    }

    for subdevice in &subdevices {
        if subdevice.product_id == EL7062_PRODUCT_ID {
            info!(
                "Configuring subdevice: Product ID={:#X}, Device Address={}",
                subdevice.product_id, subdevice.device_address
            );

            el7062
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &el7062.get_config(),
                )
                .expect("Failed to write config");
            info!("EL7062 configuration written successfully");

            let sm = |index: u16, sub: u8| -> String {
                match eth_control
                    .channel
                    .sdo_read::<u32>(subdevice.device_address, index, sub)
                {
                    Ok(v) => format!("0x{v:08x} ({v} dec)"),
                    Err(e) => format!("read failed: {e}"),
                }
            };
            info!("SM sync params readback:");
            for (idx, sub, name) in [
                (0x1C32u16, 0x01u8, "SM2 sync mode"),
                (0x1C32, 0x02, "SM2 cycle [ns]"),
                (0x1C32, 0x03, "SM2 shift [ns]"),
                (0x1C32, 0x0A, "SM2 Sync0 cycle [ns]"),
                (0x1C33, 0x01, "SM3 sync mode"),
                (0x1C33, 0x02, "SM3 cycle [ns]"),
                (0x1C33, 0x03, "SM3 shift [ns]"),
            ] {
                info!("  {name} ({idx:#06X}:{sub}): {}", sm(idx, sub));
            }

            let sync1_period = Duration::from_nanos(31250);
            eth_control
                .channel
                .enable_dc_sync01(subdevice.device_address, sync1_period)
                .expect("Failed to enable DC Sync!");
            info!(
                "DC Sync01 enabled for subdevice {} (sync1_period={:?})",
                subdevice.device_address, sync1_period
            );
        }
    }

    // Request Op state. The master thread then drives the bus through PreopPdi
    // (DC clock settling) -> SafeOp -> Op on its own.
    info!("Requesting EtherCAT state transition: -> Op");
    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to request state change to Op");

    let el7062_address = subdevices
        .iter()
        .find(|s| s.product_id == EL7062_PRODUCT_ID)
        .map(|s| s.device_address);

    if let Some(report) = eth_handle.get_last_transition_failure() {
        warn!("Previous transition failed: {}", report);
    }

    // Wait for Op state. The master reports PreopPdi (and then SafeOp) while it
    // settles the distributed clocks, so poll until all subdevices report Op.
    info!("Waiting for Op state (master passes through PreopPdi -> SafeOp -> Op)...");
    let mut attempts = 0;
    let max_attempts = 400; // ~100 seconds timeout (400 * 250ms)
    loop {
        current_state = eth_handle.get_state();
        let all_op = eth_handle.check_all_op();
        debug!(
            "EtherCAT state: {:?}, all subdevices OP: {}",
            current_state, all_op
        );
        if all_op {
            info!("EtherCAT master and all subdevices reached Op state");
            log::logger().flush();
            break;
        }

        // While wedged in PreopPdi the EL7062 may be refusing SAFE-OP; its AL
        // status / AL status code (0x0130 / 0x0134) say why. The controller
        // services these probes even during the PreopPdi cycle loop.
        if attempts % 8 == 0 {
            match eth_control.channel.al_status_snapshot() {
                Ok(statuses) => {
                    let any_faulty = statuses.iter().any(|s| s.is_faulty());
                    log::log!(
                        if any_faulty { log::Level::Warn } else { log::Level::Debug },
                        "AL snapshot during state ramp:"
                    );
                    for s in &statuses {
                        log::log!(
                            if s.is_faulty() { log::Level::Warn } else { log::Level::Debug },
                            "  {s}"
                        );
                    }
                }
                Err(e) => debug!("AL snapshot failed: {}", e),
            }
            if let Some(addr) = el7062_address {
                match eth_control.channel.register_read(addr, 0x0134u16) {
                    Ok(code) => debug!(
                        "EL7062 @{} AL status code (0x0134): 0x{:04x}",
                        addr, code
                    ),
                    Err(e) => debug!("EL7062 AL status code read failed: {}", e),
                }
                dump_dc_registers(&eth_control.channel, addr);
            }
            if let Some(report) = eth_handle.get_last_transition_failure() {
                warn!("Last failed transition: {}", report);
            }
            log::logger().flush();
        }

        attempts += 1;
        if attempts >= max_attempts {
            error!("Timeout waiting for Op state! Check device configuration.");
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    apply_rt();

    info!("Reading initial position from EL7062");
    let initial_position = {
        while !eth_handle.check_inputs_ready() {}
        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    debug!(
                        "Reading input from subdevice: Product ID={:#X}, Device Address={}",
                        subdevice.product_id, subdevice.device_address
                    );
                    el7062
                        .input(BitSlice::from_slice(
                            &inputs[subdevice.start_tx..subdevice.end_tx],
                        ))
                        .expect("Failed to read input");
                    el7062
                        .input_post_process()
                        .expect("Failed to process input");
                }
            }
        }
        let position = el7062.get_position(EL7062Port::Ch1).unwrap_or(0);
        let statusword = el7062
            .get_statusword(EL7062Port::Ch1)
            .unwrap_or_default();
        info!(
            "Initial position: {} increments, statusword=0x{:04X} (ready_to_switch_on={}, switched_on={}, operation_enabled={}, fault={})",
            position,
            statusword.as_raw(),
            statusword.ready_to_switch_on,
            statusword.switched_on,
            statusword.operation_enabled,
            statusword.fault,
        );
        position
    };

    // Ramp the setpoint instead of jumping to the target, and bounce between 0 and the target.
    info!(
        "Starting EL7062 CSP smoke test: cycle {} µs, bouncing between 0 and {} increments",
        cycle_time_us, target_position
    );

    let mut ramp = SetpointRamp::new(initial_position);
    let mut go_to: f64 = target_position as f64;
    let dt = cycle_time_us as f64 * 1e-6;
    let mut last_cycle = eth_handle.get_current_cycle();

    info!(
        "Starting control loop with cycle time: {} µs",
        cycle_time_us
    );

    // Track CiA402 enable transitions so we can confirm the drive enables.
    let mut prev_switched_on = false;
    let mut prev_operation_enabled = false;
    let mut prev_follows = false;

    loop {
        while !eth_handle.check_inputs_ready() {}

        if let Some(inputs) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    debug!(
                        "Processing input for subdevice: Product ID={:#X}, Device Address={}",
                        subdevice.product_id, subdevice.device_address
                    );
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
        debug!(
            "Statusword: 0x{:04X} (ready_to_switch_on={}, switched_on={}, operation_enabled={}, drive_follows={}, fault={})",
            statusword.as_raw(),
            statusword.ready_to_switch_on,
            statusword.switched_on,
            statusword.operation_enabled,
            statusword.drive_follows_command_value,
            statusword.fault,
        );

        if statusword.fault {
            error!("Fault detected! Statusword: 0x{:04X}", statusword.as_raw());
        }

        apply_controlword(&mut el7062, &statusword).expect("Failed to write controlword");

        if statusword.operation_enabled && !prev_operation_enabled {
            info!("Drive enabled: operation_enabled");
            log::logger().flush();
        } else if statusword.switched_on && !prev_switched_on {
            info!("Drive switched on");
            log::logger().flush();
        }
        if statusword.drive_follows_command_value && !prev_follows {
            info!("Drive follows command value (CSP active)");
            log::logger().flush();
        }
        prev_switched_on = statusword.switched_on;
        prev_operation_enabled = statusword.operation_enabled;
        prev_follows = statusword.drive_follows_command_value;

        if statusword.operation_enabled {
            ramp.advance(go_to, dt);
            debug!(
                "Setpoint ramp updated: target={}, current_position={}, velocity={}",
                go_to, ramp.position, ramp.velocity
            );

            el7062
                .set_target_position(EL7062Port::Ch1, ramp.position as i32)
                .expect("Failed to write target position");
            debug!("Target position set: {} increments", ramp.position as i32);

            if (go_to - ramp.position).abs() < 0.5 {
                go_to = if (go_to - target_position as f64).abs() < 0.5 {
                    0.0
                } else {
                    target_position as f64
                };
                debug!("Target position toggled: {}", go_to);
            }
        }

        if let Some(outputs) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    debug!(
                        "Writing output to subdevice: Product ID={:#X}, Device Address={}",
                        subdevice.product_id, subdevice.device_address
                    );
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

            if following_error
                > el7062
                    .configuration
                    .channel_1
                    .amplifier
                    .following_error_window as i32
            {
                warn!(
                    "Following error exceeded threshold! Error={}, Threshold={}",
                    following_error,
                    el7062
                        .configuration
                        .channel_1
                        .amplifier
                        .following_error_window
                );
            }

            info!(
                "Cycle {}: Position={}, Following Error={}, Operation Enabled={}, Fault={}",
                current_cycle,
                position,
                following_error,
                statusword.operation_enabled,
                statusword.fault
            );
        }
    }
}
