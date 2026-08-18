//! One-time provisioning: give every module sharing the factory-default device ID (`01h`) a
//! unique one, so each can get its own `XtremScale` on a shared `XtremBus`.
//!
//! `XtremBusHandle::subscribe` (used by `XtremScale::new`) routes incoming frames purely by the
//! `ID_O` field. Two modules answering with the same `ID_O` cannot both be attached at once — the
//! second `subscribe` call for that ID steals the route from the first (see the doc comment on
//! `XtremScale::new`). Since every XTREM ships as `01h`, a multi-scale install needs this run
//! once per new module before its `XtremScale` can coexist with the others.
//!
//! ```text
//! cargo run -p xtrem --example assign_ids -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444
//! cargo run -p xtrem --example assign_ids -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444 --dry-run
//! cargo run -p xtrem --example assign_ids -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444 --start-id 10
//! ```
//!
//! `--dry-run` prints the plan (which IP gets which new ID) without writing anything, so it's
//! safe to check before it touches real hardware. The write goes unicast to each module's own IP,
//! so a shared starting ID is not ambiguous here even though it would be over broadcast.

use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::time::Duration;

use common::get_async_runtime;
use xtrem::discovery::{self, XtremProbe};
use xtrem::transport::{XtremBus, XtremBusConfig};

struct Args {
    bind: SocketAddrV4,
    broadcast: SocketAddrV4,
    verify_lrc: bool,
    start_id: u8,
    dry_run: bool,
}

fn parse_args() -> Result<Args, anyhow::Error> {
    let mut bind = None;
    let mut broadcast = None;
    let mut verify_lrc = true;
    let mut start_id = 0x02u8;
    let mut dry_run = false;

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--bind" => bind = Some(expect(&mut argv, "--bind")?.parse()?),
            "--broadcast" => broadcast = Some(expect(&mut argv, "--broadcast")?.parse()?),
            "--no-lrc" => verify_lrc = false,
            "--start-id" => start_id = expect(&mut argv, "--start-id")?.parse()?,
            "--dry-run" => dry_run = true,
            other => return Err(anyhow::anyhow!("unknown argument {other:?}")),
        }
    }

    Ok(Args {
        bind: bind.ok_or_else(|| anyhow::anyhow!("--bind is required"))?,
        broadcast: broadcast.ok_or_else(|| anyhow::anyhow!("--broadcast is required"))?,
        verify_lrc,
        start_id,
        dry_run,
    })
}

fn expect(argv: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, anyhow::Error> {
    argv.next()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
}

/// The next device ID that is neither reserved (`00h` host / `FFh` broadcast) nor already taken.
fn next_free_id(used: &HashSet<u8>, from: u8) -> Result<u8, anyhow::Error> {
    let mut candidate = from;
    loop {
        if candidate != 0x00 && candidate != 0xFF && !used.contains(&candidate) {
            return Ok(candidate);
        }
        candidate = candidate
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ran out of device IDs (reached 0xFF)"))?;
    }
}

fn print_probes(probes: &[XtremProbe]) {
    for probe in probes {
        println!(
            "  serial {:>10}  id {:02X}h  {}  sealed {}{}",
            probe.serial,
            probe.device_id,
            probe.addr,
            probe
                .sealed
                .map_or("unknown".to_owned(), |sealed| sealed.to_string()),
            if probe.id_collision {
                "  [COLLISION]"
            } else {
                ""
            },
        );
    }
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

    println!("discovering on {}...\n", args.broadcast);
    let probes = runtime.block_on(discovery::discover(
        &bus,
        discovery::DEFAULT_DISCOVERY_WINDOW,
    ))?;

    if probes.is_empty() {
        println!("no modules answered. bus stats: {:?}", bus.stats());
        return Ok(());
    }

    println!("found {} module(s):", probes.len());
    print_probes(&probes);

    let colliding: Vec<&XtremProbe> = probes.iter().filter(|p| p.id_collision).collect();
    if colliding.is_empty() {
        println!("\nno device-ID collisions. every module already has a unique ID.");
        return Ok(());
    }

    let mut used: HashSet<u8> = probes.iter().map(|p| p.device_id).collect();
    println!(
        "\n{} module(s) share a colliding ID and will be renumbered:",
        colliding.len()
    );

    let mut plan: Vec<(SocketAddrV4, u32, u8, u8)> = Vec::new(); // (addr, serial, old_id, new_id)
    let mut next_candidate = args.start_id;
    for probe in &colliding {
        let new_id = next_free_id(&used, next_candidate)?;
        used.insert(new_id);
        next_candidate = new_id.saturating_add(1);
        plan.push((probe.addr, probe.serial, probe.device_id, new_id));
    }

    for (addr, serial, old_id, new_id) in &plan {
        println!("  {addr}  serial {serial:>10}  {old_id:02X}h -> {new_id:02X}h");
    }

    if args.dry_run {
        println!("\n--dry-run: nothing written. drop the flag to apply this plan.");
        return Ok(());
    }

    println!();
    for (addr, serial, old_id, new_id) in &plan {
        print!("writing {addr} (serial {serial}): {old_id:02X}h -> {new_id:02X}h ... ");
        runtime.block_on(discovery::assign_device_id(
            &bus,
            *old_id,
            *addr,
            *new_id,
            discovery::DEFAULT_REQUEST_TIMEOUT,
        ))?;
        println!("ok");
    }

    println!("\nre-discovering to confirm...\n");
    std::thread::sleep(Duration::from_millis(200));
    let probes = runtime.block_on(discovery::discover(
        &bus,
        discovery::DEFAULT_DISCOVERY_WINDOW,
    ))?;
    println!("found {} module(s):", probes.len());
    print_probes(&probes);

    if probes.iter().any(|p| p.id_collision) {
        println!(
            "\nstill seeing a collision - rerun this tool, or check a module didn't miss its write."
        );
    } else {
        println!("\nevery module now has a unique device ID.");
    }

    Ok(())
}
