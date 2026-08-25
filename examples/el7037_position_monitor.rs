use bitvec::{order::Lsb0, slice::BitSlice};
use ethercat_hal::{
    BECKHOFF_VENDOR_ID, EtherCATState,
    coe::ConfigurableDevice,
    devices::{
        EthercatDevice, NewEthercatDevice,
        el7037::{
            EL7037, EL7037_PRODUCT_ID, coe::EL7037Configuration, pdo::EL7037PredefinedPdoAssignment,
        },
    },
    init_ethercat,
    shared_config::el70x7::{EL70x1OperationMode, EL7037FeedbackType},
};
use std::{
    env,
    fmt::Write as _,
    io::Write as _,
    time::{Duration, Instant},
};

/// Read-only example: the motor is never enabled, the example only reads the
/// positioning information the EL7037 reports and shows it as a panel that
/// refreshes in place instead of scrolling.
///
/// The `PositionControl` PDO assignment gives all four position sources:
/// encoder counter (0x6000:11), encoder position as the position controller
/// sees it (0x6010:15), internal microstep position (0x6010:14) and the
/// position lag (0x6020:23).
///
/// Because `enable` is never set, the motor stays de-energized and the values
/// only change when the shaft is turned by hand. `ready` staying off and
/// `ready to enable` being on is the expected state for a disabled drive.
///
/// Usage: cargo run --example el7037_position_monitor -- <network-interface>
fn main() {
    let interface = env::args().nth(1).expect("No Interface-name given");
    let mut eth_control = init_ethercat(&interface, None);

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
    println!("preop");

    // CoE config must happen in PreOp (before Op transition)
    let mut el7037 = EL7037::new();
    let mut config = EL7037Configuration::default();

    config.stm_features.operation_mode = EL70x1OperationMode::PositionController;
    config.pdo_assignment = EL7037PredefinedPdoAssignment::PositionControl;

    // Encoder/motor parameters have to match the hardware for the internal and
    // external position to be comparable. No current flows either way, the
    // drive is never enabled below.
    config.encoder.reversion_of_rotation = true;
    config.stm_motor.max_current = 1100;
    config.stm_motor.encoder_increments = ENCODER_INCREMENTS as u16;
    config.stm_features.feedback_type = EL7037FeedbackType::Encoder;

    // The HAL writes the position loop (0x8014) with the terminal's documented
    // Kp (pos.) of 500. Measured on this axis that gain is unstable - moves run
    // 3-4.7x too far - so pin it to the value that actually converges here, the
    // same one el7037_move_steps uses. CoE is persistent, so an example that
    // left 500 behind would destabilise the next run of any other example too.
    config.stm_controller_3.kp_factor_pos = 5;

    for subdevice in eth_control.controller.get_subdevices() {
        if subdevice.vendor == BECKHOFF_VENDOR_ID && subdevice.product_id == EL7037_PRODUCT_ID {
            el7037
                .write_config(
                    eth_control.channel.clone(),
                    subdevice.device_address,
                    &config,
                )
                .expect("EL7037 CoE config failed");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    loop {
        match eth_control.controller.get_state() {
            EtherCATState::Op => break,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    // Static header, printed once above the panel that redraws below it.
    println!("\nEL7037 position monitor");
    println!("interface {interface} · PositionControl PDO · motor is never enabled");
    println!("turn the shaft by hand to see the values move · Ctrl-C to quit\n");

    let start = Instant::now();
    let mut screen = Screen::new();
    let mut cycle = 0u32;

    // Raw counters are u32 and wrap; accumulate wrap-corrected totals relative
    // to the first sample so hand-turning past a wrap stays readable.
    let mut prev_counter: Option<u32> = None;
    let mut counter_total: i64 = 0;
    let mut last_frame = (Instant::now(), 0i64);
    let mut have_data = false;

    loop {
        // --- Read inputs (TxPDO: encoder + motor status from device) ---
        if let Some(input) = eth_control.app_handle.get_inputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let tx_bytes = &input[subdevice.start_tx..subdevice.end_tx];
                    let _ = el7037.input(BitSlice::<u8, Lsb0>::from_slice(tx_bytes));
                    have_data = true;
                    // No `input_post_process()`: it maintains the counter wrapper
                    // for the compact encoder PDO, which this assignment does not
                    // contain. The wrap handling below replaces it.
                }
            }

            eth_control.app_handle.finish_read();
        }

        // Until the first TxPDO has landed the PDO structs still hold their
        // defaults. Drawing those would show a position of zero and then a jump
        // to the real value, which reads as if the motor had moved.
        if !have_data {
            std::thread::sleep(CYCLE_TIME);
            continue;
        }

        let sample = Sample::read(&el7037);

        if let Some(prev) = prev_counter {
            counter_total += wrapping_delta(prev, sample.counter_value);
        }
        prev_counter = Some(sample.counter_value);

        if cycle.is_multiple_of(REFRESH_EVERY) {
            let (last_instant, last_total) = last_frame;
            let elapsed = last_instant.elapsed().as_secs_f64();
            let steps_per_second = if elapsed > 0.0 {
                (counter_total - last_total) as f64 / elapsed
            } else {
                0.0
            };
            last_frame = (Instant::now(), counter_total);

            screen.draw(&render(
                &sample,
                start.elapsed().as_secs_f64(),
                counter_total,
                steps_per_second,
            ));
        }

        // --- Write outputs (RxPDO: control bits to device) ---
        // Everything stays passive: `enable` is never set, so the motor is not
        // energized. `output_pre_process()` is skipped on purpose, it would
        // auto-acknowledge drive errors via `stm_control.reset`, and this
        // example is only supposed to observe them.
        let stm_control = el7037.rxpdo.stm_control.as_mut().expect("No STM Control");
        stm_control.enable = false;
        stm_control.reset = false;
        stm_control.reduce_torque = false;

        let enc_control = el7037
            .rxpdo
            .enc_control
            .as_mut()
            .expect("No Encoder Control");
        enc_control.set_counter = false;
        enc_control.set_counter_value = 0;

        // Keep the setpoint on the measured position so the RxPDO never carries
        // a stale target that could be acted on the moment somebody enables the
        // drive in a modified copy of this example.
        let stm_position = el7037.rxpdo.stm_position.as_mut().expect("No STM Position");
        stm_position.position = sample.counter_value;

        if let Some(output) = eth_control.app_handle.write_outputs() {
            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.vendor == BECKHOFF_VENDOR_ID
                    && subdevice.product_id == EL7037_PRODUCT_ID
                {
                    let rx_bytes = &mut output[subdevice.start_rx..subdevice.end_rx];
                    let _ = el7037.output(BitSlice::<u8, Lsb0>::from_slice_mut(rx_bytes));
                }
            }
            eth_control.app_handle.send_outputs();
        }

        cycle = cycle.wrapping_add(1);
        std::thread::sleep(CYCLE_TIME);
    }
}

/// Encoder increments per revolution, used for the `revolutions` row and
/// written to the terminal as 0x8010:07.
const ENCODER_INCREMENTS: u32 = 4000;

const CYCLE_TIME: Duration = Duration::from_millis(10);

/// Redraw every 20th cycle, ~5 frames per second. Slow enough that the digits
/// stay readable, fast enough to follow the shaft by hand.
const REFRESH_EVERY: u32 = 20;

/// Everything the panel shows, copied out of the PDOs so the borrow on the
/// device ends before the RxPDO is written further down.
struct Sample {
    counter_value: u32,
    external_position: u32,
    internal_position: u32,
    position_lag: i32,
    ready_to_enable: bool,
    ready: bool,
    warning: bool,
    error: bool,
    motor_stall: bool,
    moving_positive: bool,
    moving_negative: bool,
    counter_overflow: bool,
    counter_underflow: bool,
    digital_input_1: bool,
    digital_input_2: bool,
}

impl Sample {
    fn read(el7037: &EL7037) -> Self {
        let enc_status = el7037.txpdo.enc_status.as_ref().expect("No Encoder Status");
        let stm_status = el7037.txpdo.stm_status.as_ref().expect("No STM Status");

        Self {
            counter_value: enc_status.counter_value,
            external_position: el7037
                .txpdo
                .stm_external_position
                .as_ref()
                .expect("No STM External Position")
                .external_position,
            internal_position: el7037
                .txpdo
                .stm_internal_position
                .as_ref()
                .expect("No STM Internal Position")
                .internal_position,
            // 0x6020:23 is signed on the terminal, the PDO object stores it as u32
            position_lag: el7037
                .txpdo
                .pos_actual_position_lag
                .as_ref()
                .expect("No Position Lag")
                .actual_position_lag as i32,
            ready_to_enable: stm_status.ready_to_enable,
            ready: stm_status.ready,
            warning: stm_status.warning,
            error: stm_status.error,
            motor_stall: stm_status.motor_stall,
            moving_positive: stm_status.moving_positive,
            moving_negative: stm_status.moving_negative,
            counter_overflow: enc_status.counter_overflow,
            counter_underflow: enc_status.counter_underflow,
            digital_input_1: stm_status.digital_input_1,
            digital_input_2: stm_status.digital_input_2,
        }
    }
}

/// Signed distance between two u32 position counters, wrap-around safe as long
/// as less than 2^31 increments happen between two samples.
fn wrapping_delta(prev: u32, current: u32) -> i64 {
    current.wrapping_sub(prev) as i32 as i64
}

const RULE: &str = "──────────────────────────────────────────────────────────";

fn render(sample: &Sample, seconds: f64, travelled: i64, steps_per_second: f64) -> String {
    let mut f = String::new();

    let _ = writeln!(f, "{RULE}");
    let _ = writeln!(
        f,
        "  encoder counter     0x6000:11  {:>14}",
        sample.counter_value
    );
    let _ = writeln!(
        f,
        "  external position   0x6010:15  {:>14}",
        sample.external_position
    );
    let _ = writeln!(
        f,
        "  internal position   0x6010:14  {:>14}",
        sample.internal_position
    );
    let _ = writeln!(
        f,
        "  position lag        0x6020:23  {:>14}",
        sample.position_lag
    );
    let _ = writeln!(f, "{RULE}");
    let _ = writeln!(f, "  travelled                      {travelled:>+14} steps");
    let _ = writeln!(
        f,
        "  revolutions                    {:>+14.3} rev",
        travelled as f64 / ENCODER_INCREMENTS as f64
    );
    let _ = writeln!(
        f,
        "  speed                          {steps_per_second:>+14.1} steps/s"
    );
    let _ = writeln!(f, "{RULE}");

    // Two columns of flags: drive status on the left, motion and I/O on the
    // right. The flag cells are always two visible characters wide, so the
    // padding stays correct even with the color escapes mixed in.
    let left = [
        ("ready to enable", sample.ready_to_enable, false),
        ("ready", sample.ready, false),
        ("warning", sample.warning, true),
        ("error", sample.error, true),
        ("motor stall", sample.motor_stall, true),
    ];
    let right = [
        ("moving +", sample.moving_positive, false),
        ("moving -", sample.moving_negative, false),
        ("counter overflow", sample.counter_overflow, true),
        ("counter underflow", sample.counter_underflow, true),
        ("digital input 1", sample.digital_input_1, false),
        ("digital input 2", sample.digital_input_2, false),
    ];

    for i in 0..left.len().max(right.len()) {
        match left.get(i) {
            Some(&(label, on, alert)) => {
                let _ = write!(f, "  {label:<18}{}", flag(on, alert));
            }
            // 18 label columns plus the 2 of the flag cell
            None => {
                let _ = write!(f, "  {:<20}", "");
            }
        }
        if let Some(&(label, on, alert)) = right.get(i) {
            let _ = write!(f, "    {label:<18}{}", flag(on, alert));
        }
        let _ = writeln!(f);
    }

    let _ = writeln!(f, "{RULE}");
    let _ = write!(f, "  running {seconds:.1} s");

    f
}

/// Two visible characters: `ON` when set, a dimmed `--` when not. Flags that
/// signal trouble turn red so they stand out in the still panel.
fn flag(on: bool, alert: bool) -> String {
    match (on, alert) {
        (true, true) => "\x1b[1;31mON\x1b[0m".to_string(),
        (true, false) => "\x1b[1mON\x1b[0m".to_string(),
        (false, _) => "\x1b[2m--\x1b[0m".to_string(),
    }
}

/// Draws a block of lines in place: every frame moves the cursor back to the
/// top of the previous block and overwrites it, so the panel stays still
/// instead of scrolling the terminal.
struct Screen {
    lines: usize,
}

impl Screen {
    fn new() -> Self {
        Self { lines: 0 }
    }

    fn draw(&mut self, frame: &str) {
        let mut out = String::new();

        if self.lines > 0 {
            let _ = write!(out, "\x1b[{}A\r", self.lines);
        }

        let mut drawn = 0;
        for line in frame.lines() {
            // \x1b[K clears whatever the previous frame left on this line
            let _ = writeln!(out, "{line}\x1b[K");
            drawn += 1;
        }
        // A shorter frame than last time would leave stale lines behind
        for _ in drawn..self.lines {
            let _ = writeln!(out, "\x1b[K");
            drawn += 1;
        }

        print!("{out}");
        let _ = std::io::stdout().flush();
        self.lines = drawn;
    }
}
