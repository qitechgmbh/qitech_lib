//! On-hardware smoke test: sweep a subnet for XTREM modules, then poll the first one.
//!
//! ```text
//! cargo run --example discover -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444
//! ```
//!
//! `--bind` is the port the modules send to (register `0700h`), `--broadcast` is the subnet
//! broadcast address plus the port they listen on (register `0701h`). Both default to the
//! factory settings. `--no-lrc` is for modules with LRC checking switched off (register
//! `0011h`), and `--stream <ms>` uses stream mode instead of polling.

use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

use common::get_async_runtime;
use units::mass::gram;
use xtrem::transport::{
    DEFAULT_DEVICE_LOCAL_PORT, DEFAULT_DEVICE_REMOTE_PORT, XtremBus, XtremBusConfig,
};
use xtrem::{ScaleMode, XtremDevice, XtremScale, discovery};

struct Args {
    bind: SocketAddrV4,
    broadcast: SocketAddrV4,
    verify_lrc: bool,
    mode: ScaleMode,
}

fn parse_args() -> Result<Args, anyhow::Error> {
    let mut args = Args {
        bind: SocketAddrV4::new(std::net::Ipv4Addr::UNSPECIFIED, DEFAULT_DEVICE_REMOTE_PORT),
        broadcast: SocketAddrV4::new(std::net::Ipv4Addr::BROADCAST, DEFAULT_DEVICE_LOCAL_PORT),
        verify_lrc: true,
        mode: ScaleMode::Poll,
    };

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--bind" => args.bind = expect_value(&mut argv, "--bind")?.parse()?,
            "--broadcast" => args.broadcast = expect_value(&mut argv, "--broadcast")?.parse()?,
            "--no-lrc" => args.verify_lrc = false,
            "--stream" => {
                let interval_ms = expect_value(&mut argv, "--stream")?.parse()?;
                args.mode = ScaleMode::Stream { interval_ms };
            }
            other => return Err(anyhow::anyhow!("unknown argument {other:?}")),
        }
    }
    Ok(args)
}

fn expect_value(
    argv: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, anyhow::Error> {
    argv.next()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
}

fn main() -> Result<(), anyhow::Error> {
    let args = parse_args()?;

    let bus = XtremBus::open(XtremBusConfig {
        bind_addr: args.bind,
        broadcast_addr: args.broadcast,
        host_id: 0x00,
        verify_lrc: args.verify_lrc,
        crlf: true,
    })?;

    println!(
        "listening on {}, probing {}",
        bus.local_addr()?,
        args.broadcast
    );

    let probes = get_async_runtime().block_on(discovery::discover(
        &bus,
        discovery::DEFAULT_DISCOVERY_WINDOW,
    ))?;

    if probes.is_empty() {
        println!("no modules answered. bus stats: {:?}", bus.stats());
        return Ok(());
    }

    println!("found {} module(s):", probes.len());
    for probe in &probes {
        println!(
            "  serial {:>10}  id {:02X}h  {}  sealed {}  state {:?}{}",
            probe.serial,
            probe.device_id,
            probe.addr,
            probe
                .sealed
                .map_or("unknown".to_owned(), |sealed| sealed.to_string()),
            probe.state,
            if probe.id_collision {
                "  [DEVICE ID COLLISION]"
            } else {
                ""
            }
        );
    }

    let mut scale = XtremScale::from_probe(&bus, &probes[0], args.mode);
    println!(
        "\npolling serial {} at {} in {:?} mode, ctrl-c to stop\n",
        probes[0].serial, probes[0].addr, args.mode
    );

    let mut last_print = Instant::now();
    let mut last_reading_at = None;
    loop {
        scale.send_next_request()?;
        scale.handle_response()?;

        if let Some(error) = scale.take_error() {
            eprintln!("error: {error}");
        }

        // Poll mode re-reads 0107h every tick, so report it on our own 2 Hz timer. Stream
        // mode only gets a new reading every `interval_ms` (the module's own push rate, not
        // ours) — print exactly when one lands instead of re-printing a stale reading or
        // lagging a fresh one behind a fixed clock.
        let should_print = match (args.mode, scale.reading) {
            (ScaleMode::Stream { .. }, Some(reading)) => Some(reading.at) != last_reading_at,
            _ => last_print.elapsed() >= Duration::from_millis(500),
        };

        if should_print {
            last_print = Instant::now();
            last_reading_at = scale.reading.map(|reading| reading.at);
            match scale.reading {
                Some(reading) => println!(
                    "{:>10.2} g net  (gross {:>10.2} g, tare {:>8.2} g){}{}{}",
                    reading.net.get::<gram>(),
                    reading.gross.mass.get::<gram>(),
                    reading.tare.mass.get::<gram>(),
                    if reading.status.stable() {
                        "  stable"
                    } else {
                        ""
                    },
                    if reading.status.zero() { "  zero" } else { "" },
                    if reading.status.overload() {
                        "  OVERLOAD"
                    } else {
                        ""
                    },
                ),
                None => println!("no reading yet. bus stats: {:?}", bus.stats()),
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}
