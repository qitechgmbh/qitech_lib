//! Poll every discovered module concurrently, off one shared bus.
//!
//! One `XtremBus` (one UDP socket) serves any number of `XtremScale`s — that's what the bus was
//! built for. The only precondition is that every module already has a unique device ID (run
//! `examples/assign_ids` first if `discover` reports any `[COLLISION]`); the bus routes replies
//! by `ID_O`, so two modules sharing an ID cannot both be attached at once.
//!
//! ```text
//! cargo run -p xtrem --example poll_multi -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444
//! ```

use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

use common::get_async_runtime;
use units::mass::gram;
use xtrem::transport::{XtremBus, XtremBusConfig};
use xtrem::{ScaleMode, XtremDevice, XtremScale, discovery};

struct Args {
    bind: SocketAddrV4,
    broadcast: SocketAddrV4,
    verify_lrc: bool,
}

fn parse_args() -> Result<Args, anyhow::Error> {
    let mut bind = None;
    let mut broadcast = None;
    let mut verify_lrc = true;

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--bind" => bind = Some(expect(&mut argv, "--bind")?.parse()?),
            "--broadcast" => broadcast = Some(expect(&mut argv, "--broadcast")?.parse()?),
            "--no-lrc" => verify_lrc = false,
            other => return Err(anyhow::anyhow!("unknown argument {other:?}")),
        }
    }

    Ok(Args {
        bind: bind.ok_or_else(|| anyhow::anyhow!("--bind is required"))?,
        broadcast: broadcast.ok_or_else(|| anyhow::anyhow!("--broadcast is required"))?,
        verify_lrc,
    })
}

fn expect(argv: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, anyhow::Error> {
    argv.next()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
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

    println!("found {} module(s):", probes.len());
    for probe in &probes {
        println!(
            "  serial {:>10}  id {:02X}h  {}",
            probe.serial, probe.device_id, probe.addr
        );
    }
    println!();

    let mut scales: Vec<XtremScale> = probes
        .iter()
        .map(|probe| XtremScale::from_probe(&bus, probe, ScaleMode::Poll))
        .collect();

    println!("polling {} scale(s), ctrl-c to stop\n", scales.len());
    let mut last_print = Instant::now();
    loop {
        for scale in &mut scales {
            scale.send_next_request()?;
            scale.handle_response()?;
            if let Some(error) = scale.take_error() {
                eprintln!("  {:02X}h error: {error}", scale.device_id());
            }
        }

        if last_print.elapsed() >= Duration::from_millis(500) {
            last_print = Instant::now();
            let mut total = 0.0;
            let mut any_reading = false;
            for scale in &scales {
                match scale.reading {
                    Some(reading) => {
                        let net = reading.net.get::<gram>();
                        total += net;
                        any_reading = true;
                        print!("  {:02X}h: {net:>9.1} g", scale.device_id());
                    }
                    None => print!("  {:02X}h: (no reading yet)", scale.device_id()),
                }
            }
            if any_reading {
                println!("   total: {total:>9.1} g");
            } else {
                println!();
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}
