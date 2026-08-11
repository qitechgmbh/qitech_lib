//! Isolates two hypotheses `discover` can't distinguish on its own: wrong ports, and whether
//! this module actually answers protocol-level broadcast (`ID_D = 0xFF`) reads at all.
//!
//! Sends a *unicast* read (by default, register `0101h` to device `01`, mirroring the known-good
//! `prototype-ff01/machine/src/scales.rs` request) straight at a known IP — no discovery sweep,
//! no `ID_D = 0xFF`. If this gets an answer but `discover` doesn't, the module's firmware isn't
//! honoring broadcast addressing and `discover()` needs to be bypassed for this hardware.
//!
//! ```text
//! cargo run -p xtrem --example probe_direct -- --target 192.168.4.87:4444 --bind 192.168.4.1:5555
//! cargo run -p xtrem --example probe_direct -- --target 192.168.4.87:4444 --bind 192.168.4.1:5555 --device-id 2
//! ```

use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

use common::get_async_runtime;
use xtrem::protocol::DataAddress;
use xtrem::transport::{Destination, XtremBus, XtremBusConfig};

struct Args {
    bind: SocketAddrV4,
    target: SocketAddrV4,
    device_id: u8,
    verify_lrc: bool,
}

fn parse_args() -> Result<Args, anyhow::Error> {
    let mut bind = None;
    let mut target = None;
    let mut device_id = 0x01u8;
    let mut verify_lrc = true;

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--bind" => bind = Some(expect(&mut argv, "--bind")?.parse()?),
            "--target" => target = Some(expect(&mut argv, "--target")?.parse()?),
            "--device-id" => device_id = expect(&mut argv, "--device-id")?.parse()?,
            "--no-lrc" => verify_lrc = false,
            other => return Err(anyhow::anyhow!("unknown argument {other:?}")),
        }
    }

    Ok(Args {
        bind: bind.ok_or_else(|| anyhow::anyhow!("--bind is required"))?,
        target: target.ok_or_else(|| anyhow::anyhow!("--target is required"))?,
        device_id,
        verify_lrc,
    })
}

fn expect(argv: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, anyhow::Error> {
    argv.next()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
}

fn main() -> Result<(), anyhow::Error> {
    let args = parse_args()?;

    let bus = XtremBus::open(XtremBusConfig {
        bind_addr: args.bind,
        // Unused here since every send is Destination::Unicast, but XtremBusConfig requires it.
        broadcast_addr: SocketAddrV4::new(std::net::Ipv4Addr::BROADCAST, args.target.port()),
        host_id: 0x00,
        verify_lrc: args.verify_lrc,
        crlf: true,
    })?;

    println!(
        "bound {}, unicasting device {:02X}h reads to {} (register 0101h, gross weight)",
        bus.local_addr()?,
        args.device_id,
        args.target,
    );

    let mut events = bus.events();
    let runtime = get_async_runtime();

    for attempt in 1..=5 {
        let frame = bus.read_frame(args.device_id, DataAddress::GrossWeight);
        runtime.block_on(bus.send(&frame, Destination::Unicast(args.target)))?;
        println!("[{attempt}/5] sent, waiting up to 1s for a reply...");

        let deadline = Instant::now() + Duration::from_secs(1);
        let got_reply = runtime.block_on(async {
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return false;
                }
                match tokio::time::timeout(remaining, events.recv()).await {
                    Ok(Ok(inbound)) => {
                        println!(
                            "  reply from {}: ID_O={:02X}h function={:?} address={:?} data={:?}",
                            inbound.from,
                            inbound.frame.id_origin,
                            inbound.frame.function,
                            inbound.frame.address,
                            String::from_utf8_lossy(&inbound.frame.data),
                        );
                        return true;
                    }
                    _ => return false,
                }
            }
        });

        if got_reply {
            println!("\nmodule answered a direct unicast read. bus stats: {:?}", bus.stats());
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    println!(
        "\nno reply after 5 attempts. bus stats: {:?}\n\
         if this stays silent while a raw socket test (no library) also gets nothing back on\n\
         these ports, the port numbers or the module's listening state are still the problem,\n\
         not broadcast addressing.",
        bus.stats()
    );
    Ok(())
}
