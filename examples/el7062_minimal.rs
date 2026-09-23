use bitvec::slice::BitSlice;
use ethercat_hal::{
    EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, EthercatDeviceProcessing, NewEthercatDevice,
        beckhoff_modules::el7062::{
            DmcDriveStatus, EL7062, EL7062_PRODUCT_ID, read_dmc_drive_status,
        },
    },
    init_ethercat,
    io::servo_position_el7062::ServoPositionEL7062Device,
};
use std::{env, time::Duration};

const USAGE: &str = "el7062_minimal interface_name [enable]\n
 example: ./target/release/examples/el7062_minimal enp4s0          # watch states, motor stays off
          ./target/release/examples/el7062_minimal enp4s0 enable   # actually move both motors (slow)";

/// Wall-clock interval between target steps.
///
/// The cadence is measured in wall time, NOT loop iterations: regardless of how
/// fast the EtherCAT master's OP loop spins, the commanded position never changes
/// faster than one increment per [`STEP_INTERVAL`]. 5 ms -> 200 steps/s.
const STEP_INTERVAL: Duration = Duration::from_millis(5);

/// Target change per step, in increments.
///
/// At 200 steps/s this is a sustained 200 increments/s. If the drive counts full
/// steps (200 per revolution) that is exactly 1 rev/s; with microstepping the
/// motor will turn even slower. Far below the drive's default velocity
/// limitation of 100,000 increments/s.
const STEPS_PER_UPDATE: i128 = 1;

/// Back-and-forth sweep bound in increments (= 5 revolutions at 200 steps/rev,
/// 5 s out and 5 s back at 200 steps/s).
const TRAVEL_STEPS: i128 = 1000;

/// Minimum wall-clock spacing between state/target log lines (4 lines/s max).
const PRINT_INTERVAL: Duration = Duration::from_millis(250);

/// Minimal example for the EL7062 two-channel stepper output stage in CiA 402 CSP mode.
///
/// By default the motor is left disabled (no power stage activation); add the
/// `enable` argument to actually move both channels. Motion is a very gentle
/// back-and-forth sweep: the target changes by a single increment every 5 ms of
/// wall time (200 Hz), reversing direction at ±[`TRAVEL_STEPS`]. Feedback
/// position and drive status are printed at most 4 times per second.
///
/// The drive will only leave `ReadyToSwitchOn` once the motor supply
/// (24 V / 48 V) is present and the power stage can be activated.
fn main() {
    let fail = format!("{}:\n{}", "No interface name given", USAGE);
    let interface = env::args().nth(1).expect(&fail);
    let enable_motors = env::args().nth(2).is_some_and(|v| v == "enable");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,el7062_minimal=debug,ethercat_hal=info".into()),
        )
        .init();

    // The EL7062's "DC-Synchron" OpMode declares a fixed 62500 ns SYNC0 cycle time
    // (ESI CycleTimeSync0 Factor="0") and no shift, which SafeOp enforcement reports as
    // AL error 0x0035 when violated. Our default 1 ms SYNC0 must be overridden here.
    let mut dc_config = ethercat_hal::DcConfiguration::default();
    dc_config.sync0_period = Duration::from_nanos(62_500);
    dc_config.sync0_shift = Duration::ZERO;
    let mut config = ethercat_hal::MasterConfiguration::default();
    config.dc_config = dc_config;

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

    let mut el7062 = EL7062::new();
    let mut device_address: Option<u16> = None;
    for subdevice in eth_handle.try_get_subdevices_vec_sync().unwrap() {
        if subdevice.product_id == EL7062_PRODUCT_ID {
            device_address = Some(subdevice.device_address);
            tracing::info!(
                "found EL7062 at address {} (0x{:08X})",
                subdevice.device_address,
                subdevice.product_id
            );
            el7062
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &el7062.get_config(),
                )
                .expect("Failed to write config");
            tracing::info!("EL7062 config written, enabling DC Sync0 (62.5us) + Sync1 (1ms)");
            // The EL7062 requires both SYNC0 and SYNC1 (its ESI declares
            // AssignActivate 0x700), so use DC Sync01 rather than Sync0 alone.
            eth_control
                .channel
                .enable_dc_sync01(subdevice.device_address, Duration::from_millis(1))
                .expect("Failed to enable DC Sync!");
        }
    }

    let device_address = device_address.expect("EL7062 not found on the bus");

    // SDO mailbox IO is serviced promptly in PreOp/SafeOp but is starved once the
    // Op cycle loop is running (the master cycle governor currently sprints and
    // drops mailbox responses). So snapshot the DMC unit status here, before
    // entering Op, instead of reading it live in the loop.
    let mut dmc_snapshot: [Option<DmcDriveStatus>; 2] = [None, None];
    for port in 0..2 {
        match read_dmc_drive_status(&eth_control.channel, device_address, port) {
            Ok(status) => {
                tracing::info!(
                    "EL7062 ch{port} DMC DriveStatus snapshot: ready_to_enable={} ready={} warning={} error={}",
                    status.ready_to_enable,
                    status.ready,
                    status.warning,
                    status.error
                );
                dmc_snapshot[port] = Some(status);
            }
            Err(e) => {
                tracing::warn!("EL7062 ch{port} DMC DriveStatus SDO read failed (PreOp): {e}");
            }
        }
    }

    // The DMC unit status is snapshotted once in PreOp/SafeOp: SDO mailbox
    // servicing is prompt there, but starved in the Op cycle loop (the master
    // cycle governor currently sprints/drops mailbox frames, so per-line SDO
    // reads both fail AND stall the loop ~1s each). Live SDO readout in Op
    // returns once that governor bug is fixed (see README "_Known issues_").
    // The PreOp value is representative anyway: a channel cannot be enabled
    // until `ready_to_enable` is 1, and without motor-supply detection the
    // EL7062 refuses to leave ReadyToSwitchOn in the first place.
    let mut dmc_snapshot = [DmcDriveStatus::default(); 2];
    for port in 0..2 {
        match read_dmc_drive_status(&eth_control.channel, device_address, port) {
            Ok(status) => {
                tracing::info!(
                    "EL7062 ch{port} DMC DriveStatus snapshot (PreOp): ready_to_enable={} ready={} warn={} err={}",
                    status.ready_to_enable as u8,
                    status.ready as u8,
                    status.warning as u8,
                    status.error as u8
                );
                dmc_snapshot[port] = status;
            }
            Err(e) => tracing::warn!("EL7062 ch{port} DMC DriveStatus snapshot failed: {e}"),
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        if eth_handle.check_all_op() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("EL7062 running in CSP mode");
    println!("  channels: {}", el7062.get_port_count());
    println!("  profile: ±{TRAVEL_STEPS} steps, +1 every 5 ms wall time (200 steps/s)");
    for (port, status) in dmc_snapshot.iter().enumerate() {
        match status {
            Some(status) => println!(
                "  dmc   ch{port}: ready_to_enable={} ready={} warn={} err={} mov_+={} mov_-={}",
                status.ready_to_enable as u8,
                status.ready as u8,
                status.warning as u8,
                status.error as u8,
                status.moving_positive as u8,
                status.moving_negative as u8,
            ),
            None => println!("  dmc   ch{port}: <unavailable - SDO read failed>"),
        }
    }
    if enable_motors {
        println!(
            "  WARNING: motors are ENABLED and will move. Make sure 24V/48V motor supply is present."
        );
    } else {
        println!("  motors left DISABLED (no power stage). Pass \"enable\" to move them.");
    }

    let subdevices = eth_handle.try_get_subdevices_vec_sync().unwrap();
    // Gentle back-and-forth sweep driven by the wall clock: one increment every
    // STEP_INTERVAL, reversing at ±TRAVEL_STEPS. Channel 2 is mirrored so the two
    // shafts move in opposite directions. Stepping and logging are decoupled from
    // the loop iteration rate, so a sprinting controller can never speed them up.
    let mut target_ch0: i128 = 0;
    let mut target_ch1: i128 = 0;
    let mut dir_ch0: i128 = 1;
    let mut dir_ch1: i128 = -1;
    let mut last_target_step = std::time::Instant::now();
    let mut last_print = std::time::Instant::now();
    let started = std::time::Instant::now();
    loop {
        // Read the Tx PDOs of the EL7062
        while eth_handle.check_inputs_ready() == false {}
        if let Some(input) = eth_handle.get_inputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    let input = &input[subdevice.start_tx..subdevice.end_tx];
                    el7062
                        .input(BitSlice::from_slice(input))
                        .expect("Failed to read input");
                    el7062
                        .input_post_process()
                        .expect("Failed to process input");
                }
            }
        }

        if last_target_step.elapsed() >= STEP_INTERVAL {
            last_target_step = std::time::Instant::now();
            target_ch0 += dir_ch0 * STEPS_PER_UPDATE;
            if target_ch0.abs() >= TRAVEL_STEPS {
                dir_ch0 = -dir_ch0;
            }
            target_ch1 += dir_ch1 * STEPS_PER_UPDATE;
            if target_ch1.abs() >= TRAVEL_STEPS {
                dir_ch1 = -dir_ch1;
            }
        }

        for port in 0..el7062.get_port_count() {
            let input = el7062
                .get_input(port)
                .expect("Failed to read channel snapshot");
            if input.is_in_fault {
                eprintln!("channel {port}: fault, issuing reset");
                el7062
                    .reset_fault(port)
                    .expect("Failed to reset channel fault");
            }
            el7062
                .set_enabled(port, enable_motors)
                .expect("Failed to set channel enable");
            el7062
                .set_target_position(port, if port == 0 { target_ch0 } else { target_ch1 })
                .expect("Failed to set target position");
        }

        if let Some(output) = eth_handle.write_outputs() {
            for subdevice in &subdevices {
                if subdevice.product_id == EL7062_PRODUCT_ID {
                    el7062
                        .output_pre_process()
                        .expect("Failed to prepare output");
                    let output = &mut output[subdevice.start_rx..subdevice.end_rx];
                    el7062
                        .output(BitSlice::from_slice_mut(output))
                        .expect("Failed to write output");
                }
            }
        }
        eth_handle.send_outputs();

        if last_print.elapsed() >= PRINT_INTERVAL {
            last_print = std::time::Instant::now();
            let elapsed = started.elapsed().as_secs_f64();
            for port in 0..el7062.get_port_count() {
                let input = el7062
                    .get_input(port)
                    .expect("Failed to read channel snapshot");
                let target = el7062
                    .get_target_position(port)
                    .expect("Failed to read target position");
                println!(
                    "t={elapsed:>9.3}s ch{port}: pos={:>10} target={:>10} state={:<18?} status=0x{:04X} mode={:?} follows={} enabled={}",
                    input.position,
                    target,
                    input.state,
                    input.status_word,
                    input.mode_display,
                    input.drive_follows_command,
                    input.is_enabled,
                );
            }
        }
    }
}
