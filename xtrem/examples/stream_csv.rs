//! Stream every discovered module at a fixed interval and log every sample to a CSV file.
//!
//! Same one-bus/many-modules setup as `examples/poll_multi`, but the modules push their own
//! readings (register `0013h` + execute `1011h`) instead of answering one read per tick. At 20 ms
//! that is 50 samples/s per module with no request traffic on the wire at all.
//!
//! Every sample lands as one CSV row; the terminal keeps a live table of the connected scales and
//! their current weight. As in `poll_multi`, every module must already have a unique device ID
//! (run `examples/assign_ids` first if `discover` reports any `[COLLISION]`).
//!
//! ```text
//! cargo run -p xtrem --example stream_csv -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444
//! ```
//!
//! Ctrl-C stops the logging, but the process dies before `XtremScale::drop` can send `1010h`, so
//! the modules keep streaming into an unbound port until something talks to them again. Pass
//! `--seconds <n>` to have the run end on its own and shut the streams down cleanly.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::get_async_runtime;
use units::mass::gram;
use xtrem::transport::{XtremBus, XtremBusConfig};
use xtrem::{Reading, ScaleMode, XtremDevice, XtremScale, discovery};

/// How often the live table is repainted. Far slower than the stream itself — the CSV is the
/// record, the terminal is only there to tell you the load cells are alive.
const REDRAW_INTERVAL: Duration = Duration::from_millis(100);

/// The tick of the drive loop. Well under the stream interval, so a pushed frame is folded into
/// the driver within a tick of arriving rather than sitting in the inbox.
const TICK: Duration = Duration::from_millis(2);

struct Args {
    bind: SocketAddrV4,
    broadcast: SocketAddrV4,
    verify_lrc: bool,
    interval_ms: u16,
    out: Option<PathBuf>,
    seconds: Option<u64>,
}

fn parse_args() -> Result<Args, anyhow::Error> {
    let mut bind = None;
    let mut broadcast = None;
    let mut verify_lrc = true;
    let mut interval_ms = 20;
    let mut out = None;
    let mut seconds = None;

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--bind" => bind = Some(expect(&mut argv, "--bind")?.parse()?),
            "--broadcast" => broadcast = Some(expect(&mut argv, "--broadcast")?.parse()?),
            "--no-lrc" => verify_lrc = false,
            "--interval" => interval_ms = expect(&mut argv, "--interval")?.parse()?,
            "--out" => out = Some(PathBuf::from(expect(&mut argv, "--out")?)),
            "--seconds" => seconds = Some(expect(&mut argv, "--seconds")?.parse()?),
            other => return Err(anyhow::anyhow!("unknown argument {other:?}")),
        }
    }

    Ok(Args {
        bind: bind.ok_or_else(|| anyhow::anyhow!("--bind is required"))?,
        broadcast: broadcast.ok_or_else(|| anyhow::anyhow!("--broadcast is required"))?,
        verify_lrc,
        interval_ms,
        out,
        seconds,
    })
}

fn expect(argv: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, anyhow::Error> {
    argv.next()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
}

/// Per-scale bookkeeping the driver itself does not keep: what has already been written, and what
/// went wrong most recently.
struct Logged {
    /// The `at` of the last sample written to the CSV, so a stream that goes quiet is not logged
    /// over and over with the same reading.
    last_at: Option<Instant>,
    samples: u64,
    errors: u64,
    last_error: Option<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = parse_args()?;
    let runtime = get_async_runtime();

    let bus = XtremBus::open(XtremBusConfig {
        bind_addr: args.bind,
        broadcast_addr: args.broadcast,
        host_id: 0x00,
        verify_lrc: args.verify_lrc,
        crlf: true,
    })?;

    println!("discovering on {}...", args.broadcast);
    let probes = runtime.block_on(discovery::discover(
        &bus,
        discovery::DEFAULT_DISCOVERY_WINDOW,
    ))?;

    if probes.is_empty() {
        println!("no modules answered. bus stats: {:?}", bus.stats());
        return Ok(());
    }

    let colliding = probes.iter().filter(|p| p.id_collision).count();
    if colliding > 0 {
        return Err(anyhow::anyhow!(
            "{colliding} module(s) share a device ID - run `examples/assign_ids` first, \
             otherwise their readings will overwrite each other"
        ));
    }

    // The wall clock is sampled once and every later timestamp is derived from the monotonic
    // clock, so the CSV stays evenly spaced even if the system clock is stepped mid-run.
    let start = Instant::now();
    let start_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as f64;

    let path = args
        .out
        .unwrap_or_else(|| PathBuf::from(format!("xtrem_stream_{}.csv", start_unix_ms as u64)));
    let mut csv = BufWriter::new(File::create(&path)?);
    writeln!(
        csv,
        "unix_ms,elapsed_ms,device_id,serial,gross_g,tare_g,net_g,\
         stable,zero,net_flag,tare_active,overload,status_raw"
    )?;

    let mut scales: Vec<XtremScale> = probes
        .iter()
        .map(|probe| {
            XtremScale::from_probe(
                &bus,
                probe,
                ScaleMode::Stream {
                    interval_ms: args.interval_ms,
                },
            )
        })
        .collect();
    let mut logged: Vec<Logged> = scales
        .iter()
        .map(|_| Logged {
            last_at: None,
            samples: 0,
            errors: 0,
            last_error: None,
        })
        .collect();

    println!(
        "streaming {} scale(s) every {} ms into {}",
        scales.len(),
        args.interval_ms,
        path.display()
    );
    match args.seconds {
        Some(s) => println!("stopping after {s} s\n"),
        None => println!("ctrl-c to stop\n"),
    }
    println!(
        "  {:<4} {:>10}  {:<21} {:>12}  {:<24} {:>9}",
        "id", "serial", "address", "net", "status", "samples"
    );

    let deadline = args.seconds.map(|s| start + Duration::from_secs(s));
    let mut painted = false;
    let mut last_redraw = Instant::now() - REDRAW_INTERVAL;

    loop {
        for (scale, state) in scales.iter_mut().zip(&mut logged) {
            // In stream mode this only re-arms a stream that has gone stale; the steady state
            // sends nothing.
            scale.send_next_request()?;
            scale.handle_response()?;

            if let Some(error) = scale.take_error() {
                state.errors += 1;
                state.last_error = Some(format!("{:02X}h: {error}", scale.device_id()));
            }

            if let Some(reading) = scale.reading
                && state.last_at != Some(reading.at)
            {
                state.last_at = Some(reading.at);
                state.samples += 1;
                write_row(
                    &mut csv,
                    start,
                    start_unix_ms,
                    scale.device_id(),
                    scale.serial(),
                    &reading,
                )?;
            }
        }

        if last_redraw.elapsed() >= REDRAW_INTERVAL {
            last_redraw = Instant::now();
            redraw(&scales, &logged, painted)?;
            painted = true;
            // The table is the only progress indicator, so the file is made durable at the same
            // rate: what you see on screen is what is already on disk.
            csv.flush()?;
        }

        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        std::thread::sleep(TICK);
    }

    // Dropping the scales sends `1010h` to each module, so nothing keeps streaming after we go.
    redraw(&scales, &logged, painted)?;
    drop(scales);
    csv.flush()?;

    let total: u64 = logged.iter().map(|l| l.samples).sum();
    println!("\nwrote {total} sample(s) to {}", path.display());
    println!("bus stats: {:?}", bus.stats());
    Ok(())
}

fn write_row(
    csv: &mut impl Write,
    start: Instant,
    start_unix_ms: f64,
    device_id: u8,
    serial: Option<u32>,
    reading: &Reading,
) -> Result<(), anyhow::Error> {
    let elapsed_ms = reading.at.duration_since(start).as_secs_f64() * 1000.0;
    let status = reading.status;
    writeln!(
        csv,
        "{:.3},{:.3},{:02X},{},{:.4},{:.4},{:.4},{},{},{},{},{},{:#06X}",
        start_unix_ms + elapsed_ms,
        elapsed_ms,
        device_id,
        serial.map_or_else(String::new, |s| s.to_string()),
        reading.gross.mass.get::<gram>(),
        reading.tare.mass.get::<gram>(),
        reading.net.get::<gram>(),
        status.stable() as u8,
        status.zero() as u8,
        status.net() as u8,
        status.tare() as u8,
        status.overload() as u8,
        status.raw,
    )?;
    Ok(())
}

/// Repaint the live table in place. `painted` says whether a previous table is still on screen and
/// has to be walked back over first.
fn redraw(scales: &[XtremScale], logged: &[Logged], painted: bool) -> Result<(), anyhow::Error> {
    // One line per scale, plus the total and the error line.
    let rows = scales.len() + 2;
    let mut out = String::new();
    if painted {
        out.push_str(&format!("\x1b[{rows}A"));
    }

    let mut total = 0.0;
    let mut any_reading = false;
    for (scale, state) in scales.iter().zip(logged) {
        let (weight, status) = match scale.reading {
            Some(reading) => {
                let net = reading.net.get::<gram>();
                total += net;
                any_reading = true;
                (format!("{net:>10.1} g"), describe(&reading))
            }
            None => ("   (waiting)".to_string(), String::from("no sample yet")),
        };
        out.push_str(&format!(
            "\x1b[2K  {:02X}h  {:>10}  {:<21} {weight}  {status:<24} {:>9}\n",
            scale.device_id(),
            scale.serial().map_or_else(String::new, |s| s.to_string()),
            scale.addr().map_or_else(String::new, |a| a.to_string()),
            state.samples,
        ));
    }

    let total = if any_reading {
        format!("{total:>10.1} g")
    } else {
        "        - g".to_string()
    };
    out.push_str(&format!(
        "\x1b[2K  {:<4} {:>10}  {:<21} {total}\n",
        "all", "", ""
    ));

    let errors: u64 = logged.iter().map(|l| l.errors).sum();
    let last_error = logged
        .iter()
        .filter_map(|l| l.last_error.as_deref())
        .next_back()
        .unwrap_or("-");
    out.push_str(&format!("\x1b[2K  errors: {errors}  last: {last_error}\n"));

    let mut stdout = std::io::stdout();
    stdout.write_all(out.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

fn describe(reading: &Reading) -> String {
    let mut flags = vec![if reading.status.stable() {
        "stable"
    } else {
        "motion"
    }];
    if reading.status.zero() {
        flags.push("zero");
    }
    if reading.status.net() {
        flags.push("net");
    }
    if reading.status.overload() {
        flags.push("OVERLOAD");
    }
    if reading.status.negative_weight() {
        flags.push("NEGATIVE");
    }
    flags.join(" ")
}
