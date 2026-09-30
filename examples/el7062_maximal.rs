/*
    EL7062 channel 1. Assumes a motor (200 full steps/rev, 1.8 A/phase) and a
    WEDL5541-A14 RS422 encoder: 500 CPR, so 0x8008:13 is 2000 after 4-fold
    evaluation.

    UNITS. Position is 2^singleturn_bits increments/rev, independent of
    0x8008:13. Velocity uses the separate coarser 0x9010:14 scale; converting a
    velocity with the position scale runs the motor ~4x too fast. Both are read
    back from the terminal and logged at startup.

    0x10F3 is read once during startup, in PreOp, never from the control loop: a
    blocking mailbox SDO there starves process data and trips the PD watchdog.

    There is no signal handling. The loop runs until the process is killed, and
    de-energising is left to the drive's PD watchdog, which fires as soon as
    cyclic exchange stops. Expect 0x8105 in the next run's 0x10F3 history.
*/

use bitvec::slice::BitSlice;
use ethercat_hal::{
    DcConfiguration, EtherCATState, MasterConfiguration, RtOptimizationConfig,
    coe::ConfigurableDevice,
    debugging::dump_dc_registers,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7062::{
            EL7062, EL7062_PRODUCT_ID, EL7062Port,
            coe::{Commutation, EncoderConfig, EncoderType, FollowingErrorMonitor},
            diagnostics::dump_diag_messages,
            motion::{DEFAULT_MAX_REV_PER_S, DEFAULT_MAX_REV_PER_S2, PositionScale, SetpointRamp},
            pdo::{DrvControlWord, EL7062PredefinedPdoAssignment, StatuswordProcessDataMonitor},
        },
    },
    init_ethercat, set_current_thread_rt_priority,
};
use log::{debug, error, info, warn};
use std::{env, time::Duration};

const USAGE: &str = concat!(
    "el7062_maximal interface_name cycle_time_us target [csp|csv|probe] [commutation_type]\n",
    "  csp   = position jog: ramps <target> revs out, holds, ramps back, repeats\n",
    "  csv   = constant velocity, target in revolutions per second\n",
    "  probe = write config + read back + dump 0x10F3 in PreOp, then exit. Never requests\n",
    "          Op, never enables the axis, so no current can flow. For encoder-based\n",
    "          commutation types. Defaults to commutation type 17.\n",
    "  commutation_type = 16 (internal step counter), 17 (incremental encoder),\n",
    "                      18 (FOC). Default 16, or 17 for probe. 18 also needs\n",
    "                      commutation determination (0x8010:63), which is not done here.\n",
    " example: ./target/release/examples/el7062_maximal enp4s0 1000 1 csv\n",
    " example: ./target/release/examples/el7062_maximal enp4s0 1000 0 probe",
);

fn apply_rt() {
    let id = core_affinity::CoreId { id: 2 };
    set_current_thread_rt_priority(99);
    core_affinity::set_for_current(id);
}

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_micros()
        .init();

    debug!("Logger initialized successfully");

    let fail = format!("{}:\n{}", "Invalid arguments", USAGE);
    let interface = env::args().nth(1).expect(&fail);
    let cycle_time_us: u64 = env::args()
        .nth(2)
        .expect(&fail)
        .parse()
        .expect("cycle_time_us must be a valid u64");
    // Revolutions on the CLI, so the increment scale cannot be misapplied.
    let target: f64 = env::args()
        .nth(3)
        .expect(&fail)
        .parse()
        .expect("target must be a valid f64 (revolutions, or revolutions/s in csv)");
    let mode_arg = env::args().nth(4);
    let mode_csv = matches!(mode_arg.as_deref(), Some("csv"));
    let mode_probe = matches!(mode_arg.as_deref(), Some("probe"));
    match mode_arg.as_deref() {
        None | Some("csp") | Some("csv") | Some("probe") => {}
        Some(_) => {
            eprintln!("{fail}");
            std::process::exit(2);
        }
    }
    // 16 = step counter, 17 = incremental encoder, 18 = FOC. `probe` defaults to
    // 17, the motion modes to 16.
    let commutation_type: Commutation = match env::args().nth(5).as_deref() {
        Some("16") => Commutation::StepperWithInternalCounter,
        Some("17") => Commutation::StepperWithEncoder,
        Some("18") => Commutation::StepperFocWithEncoder,
        Some(other) => {
            eprintln!("commutation_type must be 16, 17 or 18, got {other}");
            std::process::exit(2);
        }
        None if mode_probe => Commutation::StepperWithEncoder,
        None => Commutation::StepperWithInternalCounter,
    };

    log::logger().flush();
    info!("Starting EL7062 maximal example");
    info!("Interface: {}", interface);
    info!("Cycle time: {} µs", cycle_time_us);
    info!(
        "Command mode: {}",
        if mode_probe {
            format!(
                "PROBE (config + readback + 0x10F3 dump in PreOp, no Op, no enable, commutation \
                 type {commutation_type})"
            )
        } else if mode_csv {
            "CSV (constant velocity)".to_string()
        } else {
            "CSP (position jog)".to_string()
        }
    );
    if !mode_probe {
        info!("Target: {} rev", target);
    }

    let dc_config = DcConfiguration {
        // Give headroom for DC setup to finish
        start_delay: Duration::from_millis(100),
        // The OpMode declares a fixed SYNC0 cycle time; SafeOp refuses anything
        // else (0x0035).
        sync0_period: Duration::from_nanos(62_500),
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

    info!("Waiting for EtherCAT master to stabilize...");
    std::thread::sleep(Duration::from_millis(1000));
    let mut eth_handle = eth_control.app_handle;

    info!("Waiting for EtherCAT master to reach Init state...");
    let mut current_state = eth_handle.get_state();
    while !matches!(current_state, EtherCATState::Init) {
        debug!("Current EtherCAT state: {:?}", current_state);
        std::thread::sleep(Duration::from_millis(100));
        current_state = eth_handle.get_state();
    }
    info!("EtherCAT master reached Init state");
    log::logger().flush();

    info!("Requesting EtherCAT state transition: -> PreOp");
    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Failed to request state change to PreOp");

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

    let mut el7062 = EL7062::new();
    if mode_csv {
        el7062.configuration.pdo_assignment =
            EL7062PredefinedPdoAssignment::CyclicSynchronousVelocity;
    }
    el7062.configuration.channel_1.feedback.encoder = EncoderConfig::wired(
        EncoderType::Rs422Differential,
        // After 4-fold evaluation: 500 CPR is 2000. The raw 500 scales 4x slow.
        2000,
    )
    .expect("encoder resolution must be non-zero");
    // Encoder counts the opposite way to the motor on this setup. Without this,
    // commutation 17 closes the loop with positive feedback and runs away.
    el7062
        .configuration
        .channel_1
        .feedback
        .invert_feedback_direction = true;
    el7062.configuration.channel_1.amplifier.commutation = commutation_type;
    el7062.configuration.channel_1.motor.rated_current = 1800; // 1.8 A per phase
    el7062
        .configuration
        .channel_1
        .motor
        .configured_motor_current = 1000;
    el7062
        .configuration
        .channel_1
        .motor
        .motor_full_steps_per_revolution = 200;

    debug!(
        "EL7062 configuration: encoder_type={:?}, CPR={}, commutation={:?}",
        el7062
            .configuration
            .channel_1
            .feedback
            .encoder
            .encoder_type(),
        el7062
            .configuration
            .channel_1
            .feedback
            .encoder
            .increments_per_revolution()
            .unwrap_or(0),
        el7062.configuration.channel_1.amplifier.commutation
    );

    // Every command value below is converted with this, and the terminal is read
    // back afterwards to confirm it agrees.
    let scale = el7062.configuration.channel_1.feedback.position_scale;
    let incr_per_rev = scale.increments_per_revolution();
    let full_steps_per_rev = el7062
        .configuration
        .channel_1
        .motor
        .motor_full_steps_per_revolution;
    info!(
        "Position scale: {} singleturn bits + {} multiturn bits -> {} increments per motor \
         revolution ({} increments per full step at {} steps/rev)",
        scale.singleturn_bits(),
        scale.multiturn_bits(),
        incr_per_rev,
        scale.increments_per_step(full_steps_per_rev),
        full_steps_per_rev,
    );
    let target_incr = (target * incr_per_rev as f64) as i32;

    // 0x8010:01 and 0x8010:02 both drive statusword bit 10, so only the cycle
    // counter is enabled.
    el7062.configuration.channel_1.amplifier.statusword_monitor =
        StatuswordProcessDataMonitor::InputCycleCounter; // statusword bits 10/14
    el7062.configuration.channel_1.amplifier.velocity_limitation = 300; // 1/min
    // Trip if a full revolution of lag persists for 100 ms.
    el7062.configuration.channel_1.amplifier.following_error =
        FollowingErrorMonitor::enabled(incr_per_rev, 100)
            .expect("following error window must be a usable threshold");
    el7062
        .configuration
        .channel_1
        .amplifier
        .acceleration_limitation = 2000;

    info!(
        "EL7062 safety limits: velocity={} rev/min, following_error_window={} increments (1 rev), \
         acceleration={} rad/s², ramp={} rev/s / {} rev/s²",
        el7062.configuration.channel_1.amplifier.velocity_limitation,
        el7062
            .configuration
            .channel_1
            .amplifier
            .following_error
            .window(),
        el7062
            .configuration
            .channel_1
            .amplifier
            .acceleration_limitation as f64
            / 10.0,
        DEFAULT_MAX_REV_PER_S,
        DEFAULT_MAX_REV_PER_S2,
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

            let addr = subdevice.device_address;
            let rd = |t: char, index: u16, sub: u8| -> String {
                let chan = &eth_control.channel;
                match t {
                    'b' => chan
                        .sdo_read::<u8>(addr, index, sub)
                        .map(|v| format!("0x{v:02x} ({v} dec)"))
                        .unwrap_or_else(|e| format!("read failed: {e}")),
                    'w' => chan
                        .sdo_read::<u16>(addr, index, sub)
                        .map(|v| format!("0x{v:04x} ({v} dec)"))
                        .unwrap_or_else(|e| format!("read failed: {e}")),
                    'd' => chan
                        .sdo_read::<u32>(addr, index, sub)
                        .map(|v| format!("0x{v:08x} ({v} dec)"))
                        .unwrap_or_else(|e| format!("read failed: {e}")),
                    'x' => chan
                        .sdo_read::<bool>(addr, index, sub)
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|e| format!("read failed: {e}")),
                    _ => format!("bad type tag '{t}'"),
                }
            };
            // Confirms the SDO writes in write_config actually landed. `expected`
            // is what write_config sent, so a disagreement is a failed write.
            let h1 = |v: u8| format!("0x{v:02x} ({v} dec)");
            let h2 = |v: u16| format!("0x{v:04x} ({v} dec)");
            let h4 = |v: u32| format!("0x{v:08x} ({v} dec)");
            let fb = &el7062.configuration.channel_1.feedback;
            let amp = &el7062.configuration.channel_1.amplifier;
            let mot = &el7062.configuration.channel_1.motor;
            let fb_11 = Some(h4(fb.device_type));
            let fb_12 = Some(h1(scale.singleturn_bits()));
            let fb_13 = Some(h1(scale.multiturn_bits()));
            let fb_14 = Some(h2(fb.observer_bandwidth_hz));
            let fb_15 = Some(h1(fb.observer_feed_forward));
            let fb_801 = Some(fb.invert_feedback_direction.to_string());
            let fb_812 = Some(h2(fb.encoder.encoder_type().as_raw()));
            let fb_813 = fb.encoder.increments_per_revolution().map(h4);
            let mon = amp.statusword_monitor;
            let amp_01 = Some(mon.enable_txpdo_toggle().to_string());
            let amp_02 = Some(mon.enable_input_cycle_counter().to_string());
            let amp_31 = Some(h4(amp.velocity_limitation));
            let amp_50 = Some(h4(amp.following_error.window()));
            let amp_51 = Some(h2(amp.following_error.timeout()));
            let amp_64 = Some(h1(amp.commutation.as_raw()));
            let amp_73 = Some(h4(amp.acceleration_limitation));
            let mot_12 = Some(h4(mot.rated_current));
            let mot_33 = Some(h4(mot.motor_full_steps_per_revolution));
            let mot_34 = Some(h4(mot.configured_motor_current));
            info!("Config readback (what the drive actually stored):");
            for (t, idx, sub, name, expected) in [
                // 0x8000 / 0x8008, El7062FeedbackConfiguration
                ('d', 0x8000u16, 0x11u8, "device type", fb_11),
                ('b', 0x8000, 0x12, "singleturn bits", fb_12),
                ('b', 0x8000, 0x13, "multiturn bits", fb_13),
                ('w', 0x8000, 0x14, "observer bandwidth [Hz]", fb_14),
                ('b', 0x8000, 0x15, "observer feed forward", fb_15),
                ('x', 0x8008, 0x01, "invert feedback direction", fb_801),
                ('w', 0x8008, 0x12, "encoder type (1=RS422, 0=off)", fb_812),
                ('d', 0x8008, 0x13, "encoder incr/rev (after 4-fold)", fb_813),
                // 0x8010, El7062AmplifierConfiguration
                ('x', 0x8010, 0x01, "enable TxPDO toggle", amp_01),
                ('x', 0x8010, 0x02, "enable input cycle counter", amp_02),
                ('d', 0x8010, 0x31, "velocity limitation [1/min]", amp_31),
                ('d', 0x8010, 0x50, "following error window [incr]", amp_50),
                ('w', 0x8010, 0x51, "following error timeout [ms]", amp_51),
                ('b', 0x8010, 0x64, "commutation type (16=internal)", amp_64),
                ('d', 0x8010, 0x73, "acceleration limitation", amp_73),
                // 0x8011, El7062MotorConfiguration
                ('d', 0x8011, 0x12, "rated current [mA]", mot_12),
                ('d', 0x8011, 0x33, "motor fullsteps/rev", mot_33),
                ('d', 0x8011, 0x34, "configured motor current [mA]", mot_34),
            ] {
                match expected {
                    Some(expected) => info!(
                        "  {name} ({idx:#06X}:{sub:#04x}): {}  [wrote {expected}]",
                        rd(t, idx, sub)
                    ),
                    // 0x8008:13 is skipped by write_config when the encoder is
                    // disabled, so reading it back would confirm nothing.
                    None => {
                        info!("  {name} ({idx:#06X}:{sub:#04x}): not written (encoder disabled)")
                    }
                }
            }
            info!("SM sync params readback:");
            for (t, idx, sub, name) in [
                ('w', 0x1C32u16, 0x01u8, "SM2 sync mode"),
                ('d', 0x1C32, 0x02, "SM2 cycle [ns]"),
                ('d', 0x1C32, 0x03, "SM2 shift [ns]"),
                ('d', 0x1C32, 0x0A, "SM2 Sync0 cycle [ns]"),
                ('w', 0x1C33, 0x01, "SM3 sync mode"),
                ('d', 0x1C33, 0x02, "SM3 cycle [ns]"),
                ('d', 0x1C33, 0x03, "SM3 shift [ns]"),
            ] {
                info!("  {name} ({idx:#06X}:{sub:#04x}): {}", rd(t, idx, sub));
            }
            info!("PDO assignment readback:");
            for (t, idx, sub, name) in [
                ('b', 0x1C13u16, 0x00u8, "SM3 PDO assignment count"),
                ('w', 0x1C13, 0x01, "SM3 PDO 1 (TxPdo)"),
                ('w', 0x1C13, 0x02, "SM3 PDO 2 (TxPdo)"),
                ('w', 0x1C13, 0x03, "SM3 PDO 3 (TxPdo)"),
                ('b', 0x1C12, 0x00, "SM2 PDO assignment count"),
                ('w', 0x1C12, 0x01, "SM2 PDO 1 (RxPdo)"),
                ('w', 0x1C12, 0x02, "SM2 PDO 2 (RxPdo)"),
                ('w', 0x1C12, 0x03, "SM2 PDO 3 (RxPdo)"),
                ('b', 0x1A00u16, 0x00u8, "0x1A00 entry count"),
                ('d', 0x1A00, 0x01, "0x1A00:1 mapping [idx:sub:bits]"),
                ('d', 0x1A01, 0x01, "0x1A01:1 mapping [idx:sub:bits]"),
                ('d', 0x1A06, 0x01, "0x1A06:1 mapping [idx:sub:bits]"),
                ('b', 0x1600u16, 0x00u8, "0x1600 entry count"),
                ('d', 0x1600, 0x01, "0x1600:1 mapping [idx:sub:bits]"),
                ('d', 0x1606, 0x01, "0x1606:1 mapping [idx:sub:bits]"),
                ('d', 0x1601, 0x01, "0x1601:1 mapping [idx:sub:bits]"),
            ] {
                info!("  {name} ({idx:#06X}:{sub:#04x}): {}", rd(t, idx, sub));
            }

            info!("DMC drive status readback (Ch.1):");
            for (sub, name) in [
                (0x11u8, "ready_to_enable (0x6060:17)"),
                (0x12, "ready (0x6060:18)"),
                (0x13, "warning (0x6060:19)"),
                (0x14, "error (0x6060:20)"),
            ] {
                match eth_control
                    .channel
                    .sdo_read::<bool>(subdevice.device_address, 0x6060, sub)
                {
                    Ok(v) => info!("  {}: {}", name, v),
                    Err(e) => info!("  {}: read failed: {e}", name),
                }
            }
            match eth_control
                .channel
                .sdo_read::<u32>(subdevice.device_address, 0x6060, 0x37)
            {
                Ok(v) => info!(
                    "  DMC error ID (0x6060:55): 0x{:08X}{}",
                    v,
                    if v != 0 {
                        " (DMC fault active)"
                    } else {
                        " (no error)"
                    }
                ),
                Err(e) => info!("  DMC error ID (0x6060:55): read failed: {e}"),
            }
            match eth_control
                .channel
                .sdo_read::<u16>(subdevice.device_address, 0x6010, 0x12)
            {
                Ok(v) => info!("  Info data 1 / DC link voltage (0x6010:18): {} mV", v),
                Err(e) => info!("  Info data 1 / DC link voltage (0x6010:18): read failed: {e}"),
            }
            match eth_control
                .channel
                .sdo_read::<u8>(subdevice.device_address, 0x6010, 0x03)
            {
                Ok(v) => info!("  Modes of operation display (0x6010:03): {} (8=CSP)", v),
                Err(e) => info!("  Modes of operation display (0x6010:03): read failed: {e}"),
            }
            match eth_control
                .channel
                .sdo_read::<u8>(subdevice.device_address, 0x10F3, 0x00)
            {
                Ok(v) => info!("  DiagMessages (0x10F3:0) size/count: {}", v),
                Err(e) => info!("  DiagMessages (0x10F3:0): read failed: {e}"),
            }
            match eth_control
                .channel
                .sdo_read::<u8>(subdevice.device_address, 0x10F3, 0x02)
            {
                Ok(v) => info!("  DiagMessages (0x10F3:2) latest index: {}", v),
                Err(e) => info!("  DiagMessages (0x10F3:2): read failed: {e}"),
            }
            dump_diag_messages(&eth_control.channel, subdevice.device_address);
            for (idx, sub, name) in [
                (0xF900u16, 0x12u8, "DC link voltage (0xF900:18) [mV]"),
                (0xF900, 0x13, "Supply voltage Up (0xF900:19) [mV]"),
                (
                    0x9010,
                    0x27,
                    "Output stage safety state (0x9010:39) [0=safe,1=ready]",
                ),
                (
                    0x9010,
                    0x28,
                    "Actual motor brake state (0x9010:40) [0=applied,1=released]",
                ),
            ] {
                // 0x9010:39/40 answer with one byte despite the ESI declaring
                // them UDINT.
                let res = if idx == 0x9010 {
                    eth_control
                        .channel
                        .sdo_read::<u8>(subdevice.device_address, idx, sub)
                        .map(|v| v as u32)
                } else {
                    eth_control
                        .channel
                        .sdo_read::<u32>(subdevice.device_address, idx, sub)
                };
                match res {
                    Ok(v) => info!("  {}: {}", name, v),
                    Err(e) => info!("  {}: read failed: {e}", name),
                }
            }

            let sync1_period = Duration::from_millis(1);
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

    let el7062_address = subdevices
        .iter()
        .find(|s| s.product_id == EL7062_PRODUCT_ID)
        .map(|s| s.device_address);

    // No SDO fault reset: 0x7010:01 is read-only here. Fault reset goes through
    // CiA402 controlword bit 7, which is in the TxPDO.

    // Everything downstream converts revolutions with `incr_per_rev`, so a scale
    // mismatch would silently scale every command.
    let mut vel_incr_per_rev = 0u32;
    if let Some(addr) = el7062_address {
        let stb = eth_control.channel.sdo_read::<u8>(addr, 0x8000, 0x12).ok();
        let mtb = eth_control.channel.sdo_read::<u8>(addr, 0x8000, 0x13).ok();
        // Hex and decimal together, so a sub-index is not read as a bit count.
        let opt_hex = |v: Option<u8>| match v {
            Some(v) => format!("0x{v:02x} ({v} dec)"),
            None => "unreadable".to_string(),
        };
        info!(
            "Position scale readback: 0x8000:0x12 singleturn bits = {}, 0x8000:0x13 multiturn \
             bits = {} (asked for {} / {})",
            opt_hex(stb),
            opt_hex(mtb),
            opt_hex(Some(scale.singleturn_bits())),
            opt_hex(Some(scale.multiturn_bits()))
        );
        // Rebuild as a PositionScale so an out-of-range value is reported rather
        // than silently scaling to 1 incr/rev.
        match stb.zip(mtb).map(|(s, m)| PositionScale::new(s, m)) {
            Some(Ok(readback)) => {
                let readback_incr = readback.increments_per_revolution();
                if readback_incr != incr_per_rev {
                    error!(
                        "Position scale mismatch: terminal reports {} singleturn bits \
                         ({} incr/rev) but this example is commanding with {} incr/rev. \
                         Commands would be off by a factor of {}.",
                        readback.singleturn_bits(),
                        readback_incr,
                        incr_per_rev,
                        readback_incr as f64 / incr_per_rev as f64
                    );
                    return;
                }
            }
            Some(Err(e)) => {
                error!("Position scale readback: terminal reports an unusable scale: {e}");
                return;
            }
            None => {}
        }
        for (idx, sub, name) in [
            (
                0x8008u16,
                0x13u8,
                "0x8008:13 encoder increments/rev (after 4-fold evaluation)",
            ),
            (
                0x9010,
                0x14,
                "0x9010:14 velocity encoder resolution (velocity incr/rev)",
            ),
            (
                0x9010,
                0x15,
                "0x9010:15 position encoder resolution increments",
            ),
            (
                0x9010,
                0x16,
                "0x9010:16 position encoder resolution revolutions",
            ),
        ] {
            match eth_control.channel.sdo_read::<u32>(addr, idx, sub) {
                Ok(v) => info!("  {}: {}", name, v),
                Err(e) => info!("  {}: read failed: {e}", name),
            }
        }

        // Shown for comparison only: 0x9010:15 tracks the position scale, not
        // the physical encoder resolution, so it cannot confirm the 0x8008:13 write.
        let phys_incr = eth_control.channel.sdo_read::<u32>(addr, 0x8008, 0x13).ok();
        let pos_incr = eth_control.channel.sdo_read::<u32>(addr, 0x9010, 0x15).ok();
        let pos_rev = eth_control.channel.sdo_read::<u32>(addr, 0x9010, 0x16).ok();
        match (phys_incr, pos_incr, pos_rev) {
            (Some(enc), Some(inc), Some(rev)) => {
                let scale_incr = incr_per_rev;
                if inc == scale_incr {
                    info!(
                        "0x9010:15 reports {} incr/rev, which is the process-data position scale \
                         (2^{} = {}), as expected -- it is not the physical encoder resolution and \
                         says nothing about whether 0x8008:13 = {} is correct.",
                        inc,
                        scale.singleturn_bits(),
                        scale_incr,
                        enc
                    );
                } else if inc == enc {
                    warn!(
                        "0x9010:15 = {} coincidentally equals 0x8008:13 = {}. Expected the position \
                         scale {}. Check 0x8000:12.",
                        inc, enc, scale_incr
                    );
                } else {
                    warn!(
                        "0x9010:15 = {} matches NEITHER the position scale {} nor the physical \
                         encoder resolution {}. Check 0x8000:12 and 0x8008:12/13. \
                         0x9010:16 reports {} rev.",
                        inc, scale_incr, enc, rev
                    );
                }
            }
            (phys, pi, pr) => warn!(
                "Could not compare encoder resolutions (0x8008:13={phys:?}, 0x9010:15={pi:?}, \
                 0x9010:16={pr:?}); the encoder scaling check did not run."
            ),
        }

        // CSV converts with this, not with the position increment.
        match eth_control.channel.sdo_read::<u32>(addr, 0x9010, 0x14) {
            Ok(v) if v > 0 => {
                vel_incr_per_rev = v;
                info!(
                    "Velocity scale: 0x9010:14 = {} incr/rev (position is {} incr/rev, ratio \
                     {:.3})",
                    v,
                    incr_per_rev,
                    incr_per_rev as f64 / v as f64
                );
            }
            Ok(v) => error!("0x9010:14 velocity encoder resolution is {v}, expected non-zero"),
            Err(e) => error!("0x9010:14 velocity encoder resolution read failed: {e}"),
        }
        log::logger().flush();
    }

    if mode_probe {
        // Stays in PreOp: the axis is never enabled and no current can flow.
        info!(
            "PROBE: commutation type {} written and read back in PreOp.",
            commutation_type
        );
        let Some(addr) = el7062_address else {
            error!("PROBE: no EL7062 subdevice on the bus, nothing to probe.");
            return;
        };
        dump_diag_messages(&eth_control.channel, addr);
        log::logger().flush();
        return;
    }

    // The master ramps PreopPdi -> SafeOp -> Op on its own.
    info!("Requesting EtherCAT state transition: -> Op");
    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to request state change to Op");

    if let Some(report) = eth_handle.get_last_transition_failure() {
        warn!("Previous transition failed: {}", report);
    }

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

        // While wedged in PreopPdi the EL7062 may be refusing SafeOp; AL status
        // code 0x0134 says why.
        if attempts % 8 == 0 {
            match eth_control.channel.al_status_snapshot() {
                Ok(statuses) => {
                    let any_faulty = statuses.iter().any(|s| s.is_faulty());
                    log::log!(
                        if any_faulty {
                            log::Level::Warn
                        } else {
                            log::Level::Debug
                        },
                        "AL snapshot during state ramp:"
                    );
                    for s in &statuses {
                        log::log!(
                            if s.is_faulty() {
                                log::Level::Warn
                            } else {
                                log::Level::Debug
                            },
                            "  {s}"
                        );
                    }
                }
                Err(e) => debug!("AL snapshot failed: {}", e),
            }
            if let Some(addr) = el7062_address {
                match eth_control.channel.register_read(addr, 0x0134u16) {
                    Ok(code) => debug!("EL7062 @{} AL status code (0x0134): 0x{:04x}", addr, code),
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

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();

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
        let position = el7062.axis(EL7062Port::Ch1).position().unwrap_or(0);
        let statusword = el7062
            .axis(EL7062Port::Ch1)
            .statusword()
            .unwrap_or_default();
        info!(
            "Initial position: {} increments ({:.4} rev), statusword=0x{:04X} (ready_to_switch_on={}, switched_on={}, operation_enabled={}, fault={})",
            position,
            position as f64 / incr_per_rev as f64,
            statusword.as_raw(),
            statusword.ready_to_switch_on,
            statusword.switched_on,
            statusword.operation_enabled,
            statusword.fault,
        );
        position
    };

    // Seeded from the terminal's own position, so enabling never demands an
    // instant step.
    let seed_position = initial_position as f64;
    if mode_csv {
        if vel_incr_per_rev == 0 {
            error!(
                "CSV needs the velocity unit (0x9010:14) but it could not be read, so the target \
                 velocity cannot be scaled. Re-run without 'csv', or check mailbox access."
            );
            return;
        }
        info!(
            "Starting EL7062 CSV smoke test: cycle {} µs, commanding {} rev/s = {} velocity \
             increments/s (0x9010:14 = {} incr/rev) from initial position {} increments ({:.4} \
             rev)",
            cycle_time_us,
            target,
            (target * vel_incr_per_rev as f64) as i32,
            vel_incr_per_rev,
            initial_position,
            initial_position as f64 / incr_per_rev as f64,
        );
    } else {
        info!(
            "Starting EL7062 CSP jog smoke test: cycle {} µs, ramping {} rev ({} increments) out \
             and back around initial position {} increments ({:.4} rev)",
            cycle_time_us,
            target,
            target_incr,
            initial_position,
            initial_position as f64 / incr_per_rev as f64,
        );
    }

    let mut ramp = SetpointRamp::new(initial_position, scale);
    let mut go_to: f64 = seed_position;
    let mut last_control_cycle = eth_handle.get_current_cycle();
    let mut control_steps: u32 = 0;

    info!(
        "Starting control loop with cycle time: {} µs",
        cycle_time_us
    );

    let mut prev_switched_on = false;
    let mut prev_operation_enabled = false;
    let mut prev_follows = false;
    let mut prev_warning = false;
    // Log/reset on fault transitions, cooldown between enable attempts, abort
    // after repeated faults.
    let mut prev_fault = false;
    let mut fault_cooldown_cycles: u32 = 0;
    let mut fault_episodes: u32 = 0;
    const FAULT_COOLDOWN_CYCLES: u32 = 600;
    const FAULT_ABORT_AFTER_EPISODES: u32 = 3;
    // These thresholds are master cycles, i.e. milliseconds only at 1 kHz.
    let mut last_status_cycle = eth_handle.get_current_cycle();
    let mut last_position = initial_position;
    let mut last_position_cycle = eth_handle.get_current_cycle();
    let mut last_cycle = eth_handle.get_current_cycle();
    // Sampled per master cycle, not per loop iteration, and reported once the
    // window fills.
    const COUNTER_BURST_CYCLES: usize = 2000;
    let mut counter_burst: Vec<u16> = Vec::with_capacity(COUNTER_BURST_CYCLES);
    let mut counter_burst_started = false;
    let mut counter_burst_reported = false;
    let mut counter_burst_last_cycle = eth_handle.get_current_cycle();
    let mut counter_burst_start_cycle = 0u64;

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

        let current_cycle = eth_handle.get_current_cycle();
        let log_cycle_state = current_cycle.wrapping_sub(last_status_cycle) >= 1_000;
        if log_cycle_state {
            last_status_cycle = current_cycle;
        }

        let statusword = el7062
            .axis(EL7062Port::Ch1)
            .statusword()
            .expect("Failed to read statusword");
        // Bit 10 is decoded through the configured monitor, from stored config
        // rather than a mailbox access.
        let monitor = el7062.axis(EL7062Port::Ch1).statusword_monitor();
        if log_cycle_state {
            debug!(
                "Statusword: 0x{:04X} (ready_to_switch_on={}, switched_on={}, operation_enabled={}, drive_follows={}, fault={}, monitor={:?}, bit10={}, cycle_counter={:?})",
                statusword.as_raw(),
                statusword.ready_to_switch_on,
                statusword.switched_on,
                statusword.operation_enabled,
                statusword.drive_follows_command_value,
                statusword.fault,
                monitor,
                statusword.bit10(monitor) as u8,
                statusword.input_cycle_counter(monitor),
            );
        }

        // `check_inputs_ready()` is a level flag, so the loop body runs many
        // times per master cycle and re-reads the same frame. Gate on the master
        // cycle, or the burst captures one frame hundreds of times.
        if counter_burst.len() < COUNTER_BURST_CYCLES {
            if !counter_burst_started && statusword.operation_enabled {
                counter_burst_started = true;
                counter_burst_start_cycle = current_cycle;
                counter_burst_last_cycle = current_cycle;
                debug!(
                    "Capturing the next {COUNTER_BURST_CYCLES} statuswords, one per master \
                     cycle (arming at master cycle {current_cycle})"
                );
            }
            if counter_burst_started && current_cycle != counter_burst_last_cycle {
                counter_burst_last_cycle = current_cycle;
                counter_burst.push(statusword.as_raw());
            }
        } else if !counter_burst_reported {
            counter_burst_reported = true;
            let n = counter_burst.len();
            // The window only means something if it advanced one master cycle per
            // sample: N samples must span exactly N cycles.
            let span = current_cycle.saturating_sub(counter_burst_start_cycle);
            debug!(
                "burst covered master cycles {}..={} = {span} cycles for {n} samples",
                counter_burst_start_cycle, current_cycle,
            );
            if span != n as u64 {
                warn!(
                    "Counter burst is INVALID: {n} samples span {span} master cycles \
                     ({:.2} samples per cycle, expected 1.00). The loop did not track the \
                     master cycle, so this window sampled stale frames. Ignore the bit \
                     statistics below.",
                    n as f64 / span.max(1) as f64
                );
            }
            // Beckhoff's descriptions of the counter's location disagree
            // (0x8010:02 vs 0x6010:01), so measure all three candidate bits.
            let mut bit_changes = [0u32; 3];
            for (idx, (name, shift)) in [
                ("bit10  (0x8010:02 counter low)", 10u32),
                ("bit13  (0x6010:01 'input cycle counter')", 13),
                ("bit14  (0x8010:02 claims high bit)", 14),
            ]
            .into_iter()
            .enumerate()
            {
                let mut changes: Vec<usize> = Vec::new();
                let mut ones = 0u32;
                for (i, &raw) in counter_burst.iter().enumerate() {
                    if (raw >> shift) & 1 == 1 {
                        ones += 1;
                    }
                    if i > 0 && (raw >> shift) & 1 != (counter_burst[i - 1] >> shift) & 1 {
                        changes.push(i);
                    }
                }
                bit_changes[idx] = changes.len() as u32;
                let shown: Vec<String> = changes.iter().take(12).map(|i| format!("{i}")).collect();
                debug!(
                    "{name}: set in {ones}/{n} samples, {} transitions at cycles [{}]{}",
                    changes.len(),
                    shown.join(","),
                    if changes.len() > 12 { ", ..." } else { "" }
                );
            }
            let counter_at = |raw: u16| ((raw >> 13) & 1) << 1 | ((raw >> 10) & 1);
            let mut advance = [0u32; 4];
            let mut counter_changes: Vec<usize> = Vec::new();
            for (i, &raw) in counter_burst.iter().enumerate() {
                if i > 0 {
                    let d = (counter_at(raw) + 4 - counter_at(counter_burst[i - 1])) % 4;
                    advance[d as usize] += 1;
                    if d != 0 {
                        counter_changes.push(i);
                    }
                }
            }
            debug!(
                "composed counter (bit13<<1 | bit10): advance +0={} +1={} +2={} +3={} over {} \
                 sample pairs; it moved on {} of them",
                advance[0],
                advance[1],
                advance[2],
                advance[3],
                n - 1,
                counter_changes.len()
            );
            // A per-cycle counter predicts ~100% low-bit and ~50% high-bit
            // change rate; the high bit's rate is what identifies it.
            let high_rate = 100.0 * bit_changes[1] as f64 / (n - 1).max(1) as f64;
            let low_rate = 100.0 * bit_changes[0] as f64 / (n - 1).max(1) as f64;
            debug!(
                "a per-cycle counter predicts ~100% low-bit change rate and ~50% high-bit \
                 change rate; measured {low_rate:.0}% and {high_rate:.0}%."
            );
        }

        if statusword.fault {
            if !prev_fault {
                prev_fault = true;
                fault_episodes += 1;
                warn!(
                    "Fault detected (episode {}/{}): statusword=0x{:04X}",
                    fault_episodes,
                    FAULT_ABORT_AFTER_EPISODES,
                    statusword.as_raw()
                );
                el7062
                    .axis(EL7062Port::Ch1)
                    .set_controlword(DrvControlWord {
                        fault_reset: true,
                        ..Default::default()
                    })
                    .expect("Failed to write fault reset");
                fault_cooldown_cycles = FAULT_COOLDOWN_CYCLES;
                if fault_episodes >= FAULT_ABORT_AFTER_EPISODES {
                    error!(
                        "Drive faulted {} times after fault resets - aborting further enable \
                         attempts.",
                        fault_episodes
                    );
                }
                log::logger().flush();
            }
        } else {
            if prev_fault {
                info!("Fault cleared");
                log::logger().flush();
            }
            prev_fault = false;
            if fault_episodes >= FAULT_ABORT_AFTER_EPISODES {
                if fault_cooldown_cycles > 0 {
                    fault_cooldown_cycles -= 1;
                }
                if fault_cooldown_cycles == 0 {
                    el7062
                        .axis(EL7062Port::Ch1)
                        .set_controlword(DrvControlWord {
                            enable_voltage: true,
                            quick_stop: true,
                            ..Default::default()
                        })
                        .expect("Failed to write shutdown word");
                }
            } else if fault_cooldown_cycles > 0 {
                fault_cooldown_cycles -= 1;
                // Hold the CiA402 Shutdown word (0x0006) during the cooldown.
                el7062
                    .axis(EL7062Port::Ch1)
                    .set_controlword(DrvControlWord {
                        enable_voltage: true,
                        quick_stop: true,
                        ..Default::default()
                    })
                    .expect("Failed to write shutdown word");
            } else {
                el7062
                    .axis(EL7062Port::Ch1)
                    .apply_controlword(&statusword)
                    .expect("Failed to write controlword");
            }
        }

        if statusword.warning && !prev_warning {
            warn!(
                "Drive warning active (statusword=0x{:04X})",
                statusword.as_raw()
            );
            log::logger().flush();
        }
        prev_warning = statusword.warning;

        if statusword.operation_enabled && !prev_operation_enabled {
            info!("Drive enabled: operation_enabled");
            log::logger().flush();
        } else if statusword.switched_on && !prev_switched_on {
            info!("Drive switched on");
            log::logger().flush();
        }
        if statusword.drive_follows_command_value && !prev_follows {
            info!(
                "Drive follows command value ({} active)",
                if mode_csv { "CSV" } else { "CSP" }
            );
            log::logger().flush();
        }
        prev_switched_on = statusword.switched_on;
        prev_operation_enabled = statusword.operation_enabled;
        prev_follows = statusword.drive_follows_command_value;

        // The loop body spins far faster than the master cycle, so gate the
        // profile on the cycle changing or it runs at CPU speed.
        let new_control_cycle = current_cycle != last_control_cycle;
        if new_control_cycle {
            // Scale dt by the master cycles that actually elapsed: the body is
            // slower than the cycle, so one nominal dt per iteration would run
            // the profile at 1/N of real time.
            let elapsed_cycles = current_cycle.wrapping_sub(last_control_cycle).max(1);
            last_control_cycle = current_cycle;
            let dt = elapsed_cycles as f64 * (eth_handle.get_cycle_time_us().max(1) as f64) * 1e-6;
            control_steps += 1;

            if mode_csv {
                // Uses 0x9010:14, not the position increment. Zero until enabled,
                // so enabling never sees a stale non-zero setpoint.
                let target_v = if statusword.operation_enabled {
                    (target * vel_incr_per_rev as f64) as i32
                } else {
                    0
                };
                el7062
                    .axis(EL7062Port::Ch1)
                    .set_target_velocity(target_v)
                    .expect("Failed to write target velocity");
            } else {
                ramp.advance(go_to, dt);
                // Published unconditionally: the ramp starts at the terminal's
                // own position, so this is a no-op before enable.
                el7062
                    .axis(EL7062Port::Ch1)
                    .set_target_position(ramp.position() as i32)
                    .expect("Failed to write target position");

                if (go_to - ramp.position()).abs() < 0.5 {
                    go_to = if (go_to - seed_position).abs() < 0.5 {
                        seed_position + target_incr as f64
                    } else {
                        seed_position
                    };
                }
            }
        }

        if let Some(outputs) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    if log_cycle_state {
                        debug!(
                            "Writing output to subdevice: Product ID={:#X}, Device Address={}",
                            subdevice.product_id, subdevice.device_address
                        );
                    }
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

        if current_cycle.wrapping_sub(last_cycle) >= 1000 {
            last_cycle = current_cycle;
            // ctrl_steps is expected to be ~1000; much lower means the loop body
            // is missing master cycles.
            let control_steps_this_second = std::mem::take(&mut control_steps);
            let position = el7062
                .axis(EL7062Port::Ch1)
                .position()
                .expect("Failed to read position");

            // Position delta over the real master cycle count: same clock, and
            // no mailbox SDO inside the cyclic loop.
            let measured_rev_per_s =
                if last_position_cycle != 0 && current_cycle > last_position_cycle {
                    (position as f64 - last_position as f64)
                        / (current_cycle - last_position_cycle) as f64
                        / incr_per_rev as f64
                        * 1000.0
                } else {
                    f64::NAN
                };
            last_position = position;
            last_position_cycle = current_cycle;

            if mode_csv {
                info!(
                    "Cycle {}: Position={} incr ({:.3} rev), measured={:.3} rev/s (commanded \
                     {:.3} rev/s), Operation Enabled={}, Fault={}",
                    current_cycle,
                    position,
                    position as f64 / incr_per_rev as f64,
                    measured_rev_per_s,
                    target,
                    statusword.operation_enabled,
                    statusword.fault
                );
            } else {
                let following_error = el7062
                    .axis(EL7062Port::Ch1)
                    .following_error()
                    .expect("Failed to read following error");

                let fe_window = el7062
                    .configuration
                    .channel_1
                    .amplifier
                    .following_error
                    .window();
                if (following_error as i64).abs() > fe_window as i64 {
                    warn!(
                        "Following error exceeded threshold! Error={} incr ({:.3} rev), \
                         Threshold={} incr",
                        following_error,
                        following_error as f64 / incr_per_rev as f64,
                        fe_window
                    );
                }

                info!(
                    "Cycle {}: Position={} incr ({:.3} rev), target={:.3} rev, setpoint={:.3} rev \
                     at {:.3} rev/s, measured={:.3} rev/s, cycle_us={}, ctrl_steps={}, \
                     Following Error={} incr, Internal Limit Active={}, Operation Enabled={}, \
                     Fault={}",
                    current_cycle,
                    position,
                    position as f64 / incr_per_rev as f64,
                    go_to / incr_per_rev as f64,
                    ramp.position() / incr_per_rev as f64,
                    ramp.velocity() / incr_per_rev as f64,
                    measured_rev_per_s,
                    eth_handle.get_cycle_time_us(),
                    control_steps_this_second,
                    following_error,
                    statusword.internal_limit_active,
                    statusword.operation_enabled,
                    statusword.fault,
                );
            }
        }
    }
}
