/*
    EL7062 2-channel stepper motor output stage in Cyclic Synchronous Position mode (CSP).

    This example assumes a motor (200 full steps/rev, 1.8 A/phase) and an encoder
    (WEDL5541-A14, RS422 differential, 500 counts/rev after 4-fold evaluation)
    connected to channel 1, running open-loop with commutation type 16.

    UNITS. The EL7062 does not use one unit for everything, which is the single
    easiest thing to get wrong:

      * POSITION (0x6072, 0x6064) is 2^singleturn_bits increments per motor
        revolution. 0x8000:12 defaults to 20, so 1,048,576 increments/rev.
        0x9010:21 reports this back. 0x8008:13 only tells the terminal how many
        raw encoder counts make up one revolution; it does not change this scale.
        One full step of a 200-steps/rev motor is 5242.88 increments, so a
        "500 increment" move is 0.17 degrees - a tenth of one full step, and
        therefore no step at all.

      * VELOCITY (0xFF00, 0x6010:07) uses the separate, coarser "velocity
        encoder resolution" from 0x9010:20, which measures ~268435 per rev on
        this terminal, i.e. about 1/4 of the position increment. Converting a
        velocity with the position scale runs the motor ~4x too fast. Both
        scales are read back from the terminal and logged at startup.

    MOVEMENT PROFILES (4th argument, default csp):

      csp   Position jog. The setpoint ramps out to seed+target and back to seed
            on a trapezoidal profile limited to 5 rev/s and 10 rev/s^2, then
            repeats. With the default limits one 1-rev leg takes ~0.63 s, so the
            axis sweeps 1 rev out, 1 rev back, every ~1.27 s. Hold time is zero,
            which is why it looks continuous rather than stepping.

      csv   Constant velocity at <target> rev/s, converted with 0x9010:20.

      clock A seconds hand: 1 revolution per minute, advancing exactly 1/60 rev
            (6 degrees, 17476 increments, 3.3 full steps) once per second. The
            setpoint is a staircase, so the motion is visibly discrete. This is
            the easiest profile to check against a real clock.

    TIMING. The control step runs once per MASTER CYCLE, using the measured
    cycle time from get_cycle_time_us(). The application loop itself spins much
    faster than the master cycle - check_inputs_ready() is a level flag that
    stays set for the whole cycle window, not a new-image event - so the step is
    gated on get_current_cycle() actually changing. Un-gated, the ramp and the
    clock both run at CPU speed (measured ~200x too fast).

    This is a SAFETY-HARDENED smoke test: conservative current, speed, acceleration
    and following-error limits are applied, so the motor can be verified to spin
    without risking the hardware.

    usage: el7062_minimal <interface> <cycle_time_us> <target> [csp|csv|clock]
    example (1 rev jog): ./target/release/examples/el7062_minimal enp4s0 1000 1
    example (1 rev/s):    ./target/release/examples/el7062_minimal enp4s0 1000 1 csv
    example (clock):      ./target/release/examples/el7062_minimal enp4s0 1000 0 clock
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
            diagnostics::dump_diag_messages,
            motion::{
                DEFAULT_MAX_REV_PER_S, DEFAULT_MAX_REV_PER_S2, SetpointRamp,
                increments_per_revolution,
            },
            pdo::{DrvControlWord, EL7062PredefinedPdoAssignment},
        },
    },
    init_ethercat, set_current_thread_rt_priority,
};
use log::{debug, error, info, warn};
use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const USAGE: &str = concat!(
    "el7062_minimal interface_name cycle_time_us target [csp|csv|clock]\n",
    "  csp   = position jog: ramps <target> revs out, holds, ramps back, repeats\n",
    "  csv   = constant velocity, target in revolutions per second\n",
    "  clock = 1 revolution per minute, one 6 deg step per second (target unused)\n",
    " example: ./target/release/examples/el7062_minimal enp4s0 1000 1 csv",
);

/// The clock profile: one revolution per minute, advancing one sixtieth of a
/// revolution once per second, exactly like the seconds hand of an analogue
/// clock. `target` is ignored in this mode.
const CLOCK_STEPS_PER_REV: f64 = 60.0;
const CLOCK_STEP_PERIOD_MS: u64 = 1_000;

fn apply_rt() {
    let id = core_affinity::CoreId { id: 2 };
    set_current_thread_rt_priority(99);
    core_affinity::set_for_current(id);
}

fn main() {
    // Force early logger initialization
    env_logger::Builder::from_default_env()
        .format_timestamp_micros()
        .init();

    // Immediately log to confirm logging works
    debug!("Logger initialized successfully");

    let stop_requested = Arc::new(AtomicBool::new(false));
    {
        let flag = Arc::clone(&stop_requested);
        ctrlc::set_handler(move || {
            flag.store(true, Ordering::Relaxed);
        })
        .expect("Failed to install Ctrl-C handler");
    }

    let fail = format!("{}:\n{}", "Invalid arguments", USAGE);
    let interface = env::args().nth(1).expect(&fail);
    let cycle_time_us: u64 = env::args()
        .nth(2)
        .expect(&fail)
        .parse()
        .expect("cycle_time_us must be a valid u64");
    // Target magnitude, in revolutions: a position offset in CSP, a velocity in
    // CSV. Keeping the CLI in revolutions makes the 2^20 increment scale (and its
    // off-by-1000x misreadings) impossible to express from the command line.
    let target: f64 = env::args()
        .nth(3)
        .expect(&fail)
        .parse()
        .expect("target must be a valid f64 (revolutions, or revolutions/s in csv)");
    let mode_csv = matches!(env::args().nth(4).as_deref(), Some("csv"));
    let mode_clock = matches!(env::args().nth(4).as_deref(), Some("clock"));
    match env::args().nth(4).as_deref() {
        None | Some("csp") | Some("csv") | Some("clock") => {}
        Some(_) => {
            eprintln!("{fail}");
            std::process::exit(2);
        }
    }

    // Force flush logs
    log::logger().flush();
    info!("Starting EL7062 minimal example");
    info!("Interface: {}", interface);
    info!("Cycle time: {} µs", cycle_time_us);
    info!(
        "Command mode: {}",
        if mode_csv {
            "CSV (constant velocity)"
        } else if mode_clock {
            "CSP (clock: 1 rev/min, one 6 deg step per second)"
        } else {
            "CSP (position jog)"
        }
    );
    info!("Target: {} rev", target);

    let dc_config = DcConfiguration {
        // Give headroom for DC setup to finish
        start_delay: Duration::from_millis(100),
        // The EL7062's "DC-Synchron" OpMode declares a fixed 62500 ns SYNC0 cycle
        // time (ESI CycleTimeSync0 Factor="0"); SafeOp refuses anything else (0x0035).
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
    if mode_csv {
        el7062.configuration.pdo_assignment =
            EL7062PredefinedPdoAssignment::CyclicSynchronousVelocity;
    }
    el7062.configuration.channel_1.feedback.encoder_type = 1; // RS422 differential
    el7062
        .configuration
        .channel_1
        .feedback
        .encoder_increments_per_revolution = 500; // CPR (after 4x)
    el7062.configuration.channel_1.amplifier.commutation_type = 16; // stepper with internal counter (open-loop)
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

    // Derive the position scale from what we just asked the terminal for, rather
    // than assuming a value: every command value below is converted with it.
    // The terminal is read back after the config is written to confirm it agrees.
    let singleturn_bits = el7062.configuration.channel_1.feedback.singleturn_bits;
    let multiturn_bits = el7062.configuration.channel_1.feedback.multiturn_bits;
    assert_eq!(
        singleturn_bits as u16 + multiturn_bits as u16,
        32,
        "0x8000:12 + 0x8000:13 must be 32 or the terminal rejects the process \
         data format (diag 0x8423)"
    );
    let incr_per_rev = increments_per_revolution(singleturn_bits);
    info!(
        "Position scale: {} singleturn bits + {} multiturn bits -> {} increments per motor \
         revolution ({} increments per full step at {} steps/rev)",
        singleturn_bits,
        multiturn_bits,
        incr_per_rev,
        incr_per_rev as f64
            / el7062
                .configuration
                .channel_1
                .motor
                .motor_full_steps_per_revolution as f64,
        el7062
            .configuration
            .channel_1
            .motor
            .motor_full_steps_per_revolution,
    );
    let target_incr = (target * incr_per_rev as f64) as i32;

    // Safety limits for the smoke test (tune later).
    //
    // 0x8010:01 and 0x8010:02 both drive statusword bit 10, so only one of them
    // can be interpreted at a time. The example wants the cycle counter; leave
    // the TxPDO toggle off so bit 10 unambiguously carries the counter's low bit.
    el7062.configuration.channel_1.amplifier.enable_txpdo_toggle = false;
    el7062
        .configuration
        .channel_1
        .amplifier
        .enable_input_cycle_counter = true; // statusword bits 10/14, display only
    el7062.configuration.channel_1.amplifier.velocity_limitation = 300; // 1/min (rev-equivalent)
    el7062
        .configuration
        .channel_1
        .amplifier
        .following_error_window = incr_per_rev; // trip only if a full rev of lag
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
        "EL7062 safety limits: velocity={} rev/min, following_error_window={} increments (1 rev), \
         acceleration={} rad/s², ramp={} rev/s / {} rev/s²",
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
                    _ => format!("bad type tag '{t}'"),
                }
            };
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
                info!("  {name} ({idx:#06X}:{sub}): {}", rd(t, idx, sub));
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
                info!("  {name} ({idx:#06X}:{sub}): {}", rd(t, idx, sub));
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
                // 0xF900:18/19 are UINT32 mV, but 0x9010:39/40 answer with a
                // single byte on this terminal despite the ESI declaring them
                // UDINT, so they are read as u8.
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

    // Best-effort fault reset (CiA402 controlword bit 7) via mailbox SDO while
    // the drive is still in PreOp, so runs start from a clean state instead of
    // inheriting a latched fault from a previous session.
    if let Some(addr) = el7062_address {
        match eth_control
            .channel
            .sdo_write::<u16>(addr, 0x7010, 0x01, 0x0080)
        {
            Ok(_) => info!("Fault reset (0x7010:01) = 0x0080 sent"),
            Err(e) => warn!("Fault reset (0x7010:01) failed: {e}"),
        }
        std::thread::sleep(Duration::from_millis(200));
        match eth_control.channel.sdo_read::<u16>(addr, 0x6010, 0x01) {
            Ok(v) => info!("Ch.1 statusword after fault reset: 0x{v:04X}"),
            Err(e) => warn!("Ch.1 statusword read failed: {e}"),
        }
    }

    // Confirm the terminal actually took the position scale we are commanding
    // with. Everything downstream converts revolutions to increments using
    // `incr_per_rev`, so a mismatch here would silently scale every command.
    // `vel_incr_per_rev` is the separate, coarser unit used for velocity; it is
    // only known once the terminal has published 0x9010:20.
    let mut vel_incr_per_rev = 0u32;
    if let Some(addr) = el7062_address {
        let stb = eth_control.channel.sdo_read::<u8>(addr, 0x8000, 0x12).ok();
        let mtb = eth_control.channel.sdo_read::<u8>(addr, 0x8000, 0x13).ok();
        info!(
            "Position scale readback: 0x8000:12 singleturn bits = {:?}, 0x8000:13 multiturn bits \
             = {:?} (asked for {} / {})",
            stb, mtb, singleturn_bits, multiturn_bits
        );
        if let Some(stb) = stb {
            let readback_incr = increments_per_revolution(stb);
            if readback_incr != incr_per_rev {
                error!(
                    "Position scale mismatch: terminal reports {} singleturn bits \
                     ({} incr/rev) but this example is commanding with {} incr/rev. \
                     Commands would be off by a factor of {}.",
                    stb,
                    readback_incr,
                    incr_per_rev,
                    readback_incr as f64 / incr_per_rev as f64
                );
                return;
            }
        }
        for (idx, sub, name) in [
            (
                0x8008u16,
                0x13u8,
                "0x8008:13 encoder increments/rev (raw counts, 4-fold)",
            ),
            (
                0x9010,
                0x14,
                "0x9010:20 velocity encoder resolution (velocity incr/rev)",
            ),
            (
                0x9010,
                0x15,
                "0x9010:21 position encoder resolution increments",
            ),
            (
                0x9010,
                0x16,
                "0x9010:22 position encoder resolution revolutions",
            ),
        ] {
            match eth_control.channel.sdo_read::<u32>(addr, idx, sub) {
                Ok(v) => info!("  {}: {}", name, v),
                Err(e) => info!("  {}: read failed: {e}", name),
            }
        }

        // The EL7062 does NOT use one unit for both position and velocity.
        // Position is 2^singleturn_bits per rev (0x8000:12, 0x9010:21), but the
        // target/actual velocity in 0xFF00 / 0x6010:07 use the coarser
        // "velocity encoder resolution" from 0x9010:20, which is a factor of
        // ~4 smaller. Commanding `incr_per_rev` as a velocity therefore runs the
        // motor ~4x faster than requested, so CSV converts with this value.
        match eth_control.channel.sdo_read::<u32>(addr, 0x9010, 0x14) {
            Ok(v) if v > 0 => {
                vel_incr_per_rev = v;
                info!(
                    "Velocity scale: 0x9010:20 = {} incr/rev (position is {} incr/rev, ratio \
                     {:.3})",
                    v,
                    incr_per_rev,
                    incr_per_rev as f64 / v as f64
                );
            }
            Ok(v) => error!("0x9010:20 velocity encoder resolution is {v}, expected non-zero"),
            Err(e) => error!("0x9010:20 velocity encoder resolution read failed: {e}"),
        }
        log::logger().flush();
    }

    // Request Op state. The master thread then drives the bus through PreopPdi
    // (DC clock settling) -> SafeOp -> Op on its own.
    info!("Requesting EtherCAT state transition: -> Op");
    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Failed to request state change to Op");

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
        let position = el7062.get_position(EL7062Port::Ch1).unwrap_or(0);
        let statusword = el7062.get_statusword(EL7062Port::Ch1).unwrap_or_default();
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

    // Ramp the setpoint instead of jumping to the target. Seed the ramp and the
    // first target to the position the terminal reported at startup so enabling
    // never demands an instant large step (which trips the following-error
    // protection).
    let seed_position = initial_position as f64;
    if mode_csv {
        if vel_incr_per_rev == 0 {
            error!(
                "CSV needs the velocity unit (0x9010:20) but it could not be read, so the target \
                 velocity cannot be scaled. Re-run without 'csv', or check mailbox access."
            );
            return;
        }
        info!(
            "Starting EL7062 CSV smoke test: cycle {} µs, commanding {} rev/s = {} velocity \
             increments/s (0x9010:20 = {} incr/rev) from initial position {} increments ({:.4} \
             rev)",
            cycle_time_us,
            target,
            (target * vel_incr_per_rev as f64) as i32,
            vel_incr_per_rev,
            initial_position,
            initial_position as f64 / incr_per_rev as f64,
        );
    } else if mode_clock {
        info!(
            "Starting EL7062 CSP clock smoke test: cycle {} µs, 1 rev/min, one {:.0} deg step \
             every {} ms from initial position {} increments ({:.4} rev)",
            cycle_time_us,
            360.0 / CLOCK_STEPS_PER_REV,
            CLOCK_STEP_PERIOD_MS,
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

    // The setpoint is advanced once per MASTER CYCLE, and one master cycle is
    // `get_cycle_time_us()` long. The application loop below spins much faster
    // than that (it is only waiting on a level flag, not a new-image event), so
    // the control step is gated on the master cycle counter actually changing.
    // Skipping that gate makes the ramp run at the CPU spin rate rather than
    // real time - measured at ~200x too fast on this machine.
    let mut ramp = SetpointRamp::new(initial_position, singleturn_bits);
    let mut go_to: f64 = seed_position;
    let mut last_control_cycle = eth_handle.get_current_cycle();
    let mut control_steps: u32 = 0;
    let mut last_step_cycle = eth_handle.get_current_cycle();
    let mut step_count: u64 = 0;

    info!(
        "Starting control loop with cycle time: {} µs",
        cycle_time_us
    );

    // Track CiA402 enable transitions so we can confirm the drive enables.
    let mut prev_switched_on = false;
    let mut prev_operation_enabled = false;
    let mut prev_follows = false;
    let mut prev_warning = false;
    // Stateful fault handling: log/reset only on fault transitions, cool down
    // between enable attempts after a reset, and abort entirely after repeated
    // faults so a stuck fault never chops the motor at 1 kHz.
    let mut prev_fault = false;
    let mut fault_cooldown_cycles: u32 = 0;
    let mut fault_episodes: u32 = 0;
    let mut pending_diag_dump = false;
    const FAULT_COOLDOWN_CYCLES: u32 = 600;
    const FAULT_ABORT_AFTER_EPISODES: u32 = 3;
    // `get_current_cycle()` counts master cycles, i.e. `cycle_time_us` apart, so
    // these thresholds are milliseconds: 1000 = 1 s of status logging.
    let mut last_status_cycle = eth_handle.get_current_cycle();
    // Previous sample for the measured-velocity calculation.
    let mut last_position = initial_position;
    let mut last_position_cycle = eth_handle.get_current_cycle();
    let mut last_cycle = eth_handle.get_current_cycle();

    loop {
        if stop_requested.load(Ordering::Relaxed) {
            break;
        }
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
            .get_statusword(EL7062Port::Ch1)
            .expect("Failed to read statusword");
        if log_cycle_state {
            debug!(
                "Statusword: 0x{:04X} (ready_to_switch_on={}, switched_on={}, operation_enabled={}, drive_follows={}, fault={}, cycle_counter={}{})",
                statusword.as_raw(),
                statusword.ready_to_switch_on,
                statusword.switched_on,
                statusword.operation_enabled,
                statusword.drive_follows_command_value,
                statusword.fault,
                statusword.input_cycle_counter_high as u8,
                statusword.txpdo_toggle as u8,
            );
        }

        if statusword.fault {
            if !prev_fault {
                prev_fault = true;
                fault_episodes += 1;
                warn!(
                    "Fault detected (episode {}/{}): statusword=0x{:04X}; DiagMessages deferred \
                     to the shutdown dump, because a blocking 0x10F3 mailbox read inside this \
                     loop starves cyclic process data and trips the drive PD watchdog",
                    fault_episodes,
                    FAULT_ABORT_AFTER_EPISODES,
                    statusword.as_raw()
                );
                pending_diag_dump = true;
                el7062
                    .set_controlword(
                        EL7062Port::Ch1,
                        DrvControlWord {
                            fault_reset: true,
                            ..Default::default()
                        },
                    )
                    .expect("Failed to write fault reset");
                fault_cooldown_cycles = FAULT_COOLDOWN_CYCLES;
                if fault_episodes >= FAULT_ABORT_AFTER_EPISODES {
                    error!(
                        "Drive faulted {} times after fault resets - aborting further enable \
                         attempts to protect the setup. The diagnostic is latched in 0x10F3 \
                         (dumped at the start of the next run), e.g. 0x8404 overcurrent, \
                         0x8406 DC-link undervoltage, 0x8408/0x8409 I2T overload.",
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
                        .set_controlword(
                            EL7062Port::Ch1,
                            DrvControlWord {
                                enable_voltage: true,
                                quick_stop: true,
                                ..Default::default()
                            },
                        )
                        .expect("Failed to write shutdown word");
                }
            } else if fault_cooldown_cycles > 0 {
                fault_cooldown_cycles -= 1;
                // During the cooldown the drive is held in the CiA402 "Shutdown"
                // word (0x0006); it must not leave Switch-on-disabled again yet.
                el7062
                    .set_controlword(
                        EL7062Port::Ch1,
                        DrvControlWord {
                            enable_voltage: true,
                            quick_stop: true,
                            ..Default::default()
                        },
                    )
                    .expect("Failed to write shutdown word");
            } else {
                el7062
                    .apply_controlword(EL7062Port::Ch1, &statusword)
                    .expect("Failed to write controlword");
            }
        }

        if statusword.warning && !prev_warning {
            warn!(
                "Drive warning active (statusword=0x{:04X}); DiagMessages deferred to the \
                 shutdown dump, because a blocking 0x10F3 mailbox read inside this loop starves \
                 cyclic process data and trips the drive PD watchdog",
                statusword.as_raw()
            );
            pending_diag_dump = true;
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

        // Advance the command by exactly one master cycle's worth of time. The
        // loop spins far faster than the master cycle (check_inputs_ready is a
        // level flag that stays set for the whole cycle window), so without this
        // gate the ramp and the clock both run at CPU speed.
        let new_control_cycle = current_cycle != last_control_cycle;
        if new_control_cycle {
            // Scale dt by the master cycles that actually elapsed, not by one
            // nominal cycle. The loop body is slower than the 1 kHz master
            // cycle, so an iteration only sees every Nth cycle change; adding a
            // single nominal dt per iteration would run the whole profile at
            // 1/N of real time.
            let elapsed_cycles = current_cycle.wrapping_sub(last_control_cycle).max(1);
            last_control_cycle = current_cycle;
            let dt = elapsed_cycles as f64 * (eth_handle.get_cycle_time_us().max(1) as f64) * 1e-6;
            control_steps += 1;

            if mode_csv {
                // Velocity uses the coarser 0x9010:20 unit, not the position
                // increment. Only command once enabled, so enabling never sees a
                // stale non-zero setpoint.
                let target_v = if statusword.operation_enabled {
                    (target * vel_incr_per_rev as f64) as i32
                } else {
                    0
                };
                el7062
                    .set_target_velocity(EL7062Port::Ch1, target_v)
                    .expect("Failed to write target velocity");
            } else if mode_clock {
                // One 6 deg step per second, forever, like a seconds hand. The
                // step is small enough that the drive tracks it in one cycle
                // (1/60 rev = 17476 increments = 3.3 full steps).
                if current_cycle.wrapping_sub(last_step_cycle) >= CLOCK_STEP_PERIOD_MS {
                    last_step_cycle = current_cycle;
                    step_count += 1;
                    go_to += incr_per_rev as f64 / CLOCK_STEPS_PER_REV;
                }
                el7062
                    .set_target_position(EL7062Port::Ch1, go_to as i32)
                    .expect("Failed to write target position");
            } else {
                ramp.advance(go_to, dt);
                // Published unconditionally, not only when enabled: the ramp
                // starts at the terminal's own position, so the target is a
                // no-op before enable and the drive never sees a stale value
                // when it enables.
                el7062
                    .set_target_position(EL7062Port::Ch1, ramp.position() as i32)
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
            // Should be ~1000. Anything much lower means the loop body is
            // missing master cycles, which is what made the profile run slow.
            let control_steps_this_second = std::mem::take(&mut control_steps);
            let position = el7062
                .get_position(EL7062Port::Ch1)
                .expect("Failed to read position");

            // Measured velocity from the position delta over the real master
            // cycle count. This is exact (both are the same clock) and avoids a
            // mailbox SDO inside the cyclic loop, which fails there anyway and
            // would add jitter to a 1 kHz task.
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
                    .get_following_error(EL7062Port::Ch1)
                    .expect("Failed to read following error");

                let fe_window = el7062
                    .configuration
                    .channel_1
                    .amplifier
                    .following_error_window;
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
                     Fault={}{}",
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
                    if mode_clock {
                        format!(", clock step {} of 60", step_count % 60)
                    } else {
                        String::new()
                    },
                );
            }
        }
    }

    // Graceful stop: command Ch.1 off via PDO, drop the bus back to PreOp
    // (where mailbox SDOs are serviced again) and capture this session's diag
    // history while the drive still holds it.
    info!("Graceful stop requested; shutting the drive down...");
    if mode_csv {
        el7062
            .set_target_velocity(EL7062Port::Ch1, 0)
            .expect("Failed to write zero target velocity");
    }
    el7062
        .set_controlword(EL7062Port::Ch1, DrvControlWord::default())
        .expect("Failed to write shutdown controlword");
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

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Failed to request PreOp");
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(50));
        if eth_handle.get_state() == EtherCATState::PreOp {
            break;
        }
    }
    if eth_handle.get_state() != EtherCATState::PreOp {
        warn!(
            "Bus did not reach PreOp within 2 s (still {:?}); the DiagMessages dump below needs \
             PreOp, because mailbox SDOs are not serviced reliably in Op.",
            eth_handle.get_state()
        );
    }

    if let Some(addr) = el7062_address {
        if pending_diag_dump {
            info!(
                "Post-shutdown DiagMessages dump (includes this session's fault/warning events, \
                 latched in 0x10F3):"
            );
        } else {
            info!("Post-shutdown DiagMessages dump:");
        }
        dump_diag_messages(&eth_control.channel, addr);
    }
    info!("Clean shutdown complete");
}
