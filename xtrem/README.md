# xtrem

Rust implementation of the GRAM **XTREM / XTREM-S** weighing-module communication protocol
(v3.007), spoken over UDP. Lives in `qitech_lib` alongside `modbus` and `ethercat_hal` as a
sibling hardware-protocol crate.

## What XTREM is

An ADPD weighing module (OIML R76:2006 / EN45501:2015 certified): it drives a load cell and
publishes weight over a communication interface. It has two serial ports — UART0 is always a
plain RS232C interface; UART1 can optionally carry Wi-Fi 802.11, RS485, or wired Ethernet. This
crate implements **UDP over the network interface only**. RS232/RS485 is a different transport
carrying the same frame format and is not implemented here (see [Not implemented](#not-implemented)).

A hardware "sealing switch" locks writes to legally-relevant calibration registers and enables a
separate software-protection handshake — both exist to protect the instrument's legal-for-trade
certification, and this crate deliberately doesn't try to work around either.

## Protocol summary

Every message — request or response — is one ASCII frame:

```
STX  ID_O  ID_D  F  D_ADDRESS  D_L  DATA  LRC  ETX  [CR LF]
0x02  2     2    1      4       2   D_L    2   0x03
```

- **`ID_O` / `ID_D`** — sender/destination network address, `u8` as 2 hex chars. `ID_D = 0xFF` is
  broadcast (spec §5.1).
- **`F`** — `R`/`r` read, `W`/`w` write, `E`/`e` execute; uppercase = request, lowercase = response
  (spec §5.2).
- **`D_ADDRESS`** — the register, 4 hex chars.
- **`D_L`** — DATA length in bytes, 2 hex chars.
- **`DATA`** — raw ASCII, **not** hex-encoded. Legal range is `0x20..=0xFF`.
- **`LRC`** — XOR of every byte from `ID_O` through the last DATA byte, 2 hex chars (spec §5.6).
  Excludes STX, ETX, the LRC characters themselves, and any CR+LF.
- **CR+LF** sits outside the frame, is excluded from the LRC, and is required by the Wi-Fi/Ethernet
  module (spec §17) — it's also the serial port's factory default (register `0012h`).

### Registers this crate models

| Addr | Type | Meaning |
|---|---|---|
| `0000h` | R | Serial number (decimal ASCII, u32) — used as the stable device identity |
| `0001h` | RW | Device ID (network address) |
| `0007h` / `0008h` | R | Hardware / software version |
| `0009h` | R | Sealing switch: `'0'` unlocked, `'1'` locked |
| `0013h` | RW | Stream-mode interval, ms |
| `0100h` | R | Device state (2 hex chars): bits 0-4 weighing error, bit 5 power alarm, bits 6-7 Wi-Fi |
| `0101h`/`0102h`/`0103h` | R/RE/R | Gross / tare / net weight, 10-char fixed-width field |
| `0104h`/`0105h`/`0106h` | R | Stability / zero / zero-tracking flag |
| `0107h` | R | **Weighing register** — gross + tare + status in one 26-byte payload |
| `1010h`-`1013h` | E | Stop / start stream (weight, ADC, filtered ADC) |
| `1103h` | E | Clear tare |
| `9999h` / `EEEEh` | E | Device reset / factory reset |

`0107h`'s payload is `'W'` + 8 right-aligned digits + 2-char unit, `'T'` + same, `'S'` + 3 hex
chars (a 12-bit status word: zero / tare-on / stable / net / fixed-tare / high-res / initial-zero /
overload / negative-weight / range-2 / preset-tare, low to high bit).

**Spec erratum**, encoded correctly here: §16.1-16.3's byte tables print `D_L = "08"` for the
weight registers; the prose says `0Ah`, which is right (8 digits + 2 unit chars = 10).

### Not implemented

The §6 software-protection protocol — separate 20-byte binary framing on UART0, a 128-bit
signature settable only at the factory, engaged only when the sealing switch is LOCKed.
[`discovery::discover`](src/discovery.rs) reads register `0009h` so a sealed module is at least
diagnosable rather than silently unresponsive.

## Crate layout

```
src/
  protocol/       pure codec, no I/O — frame encode/decode, register parsing, LRC
  transport/       one shared UDP socket (XtremBus), demultiplexed by ID_O
  discovery.rs     broadcast sweep -> Vec<XtremProbe>
  devices/         XtremDevice trait + XtremScale (the weighing driver)
examples/
  discover.rs      CLI: sweep a subnet, then poll the first module found
tests/
  loopback.rs      integration tests against a fake XTREM module on 127.0.0.1
```

### `protocol` — the wire format, nothing else

[`Frame`](src/protocol/frame.rs) is the message: `id_origin`, `id_dest`, [`Function`](src/protocol/function.rs),
[`DataAddress`](src/protocol/address.rs), `data: Vec<u8>`. `Frame::encode`/`Frame::decode` are the
only places byte-level framing happens; `Frame::decode` takes a `verify_lrc: bool` because a
module can have LRC checking turned off (register `0011h`).

Register payloads are typed by [`RegisterValue::parse`](src/protocol/value.rs), which dispatches
on the [`DataAddress`](src/protocol/address.rs) — weight fields become `units::Mass`-backed
[`Weight`](src/protocol/value.rs), the `0107h` payload becomes
[`WeighingRegister { gross, tare, status }`](src/protocol/value.rs) with a `.net()` helper, flags
become `bool`, the device state byte becomes [`DeviceState`](src/protocol/status.rs) with a
`.is_healthy()` check. Unmodelled registers come back as `RegisterValue::Raw(Vec<u8>)` rather than
failing.

Every test in this layer is anchored to bytes taken verbatim from the spec's own worked examples
and its §17 UDP capture — not invented fixtures. `cargo test` in this crate re-verifies the LRC of
those captured frames on every run.

### `transport` — one socket, many modules

UDP modules don't get a socket each: they reply to whatever port is configured in their own
register `0700h`, not to the request's source port, so all traffic converges on one local port and
has to be routed by content. [`XtremBus::open`](src/transport/bus.rs) binds that socket and spawns
a receive task on the shared runtime (`common::get_async_runtime()`); [`XtremBusHandle`](src/transport/bus.rs)
is the cheap, `Clone`-able handle everything else holds. The receive task decodes each datagram and
dispatches it by `ID_O`:

- `handle.subscribe(device_id) -> mpsc::Receiver<Inbound>` — routed frames for one device (bounded
  queue; a stalled consumer loses the oldest frames rather than stalling the bus).
- `handle.events() -> broadcast::Receiver<Inbound>` — every decoded frame, for callers (discovery)
  that don't know device IDs yet.
- `handle.try_send(&frame, destination)` — non-blocking, for a synchronous control loop.
- `handle.send(&frame, destination).await` — for async callers.

`handle.stats() -> BusStats` exposes `frames_received` / `decode_errors` / `unrouted` / `dropped` /
`io_errors` — the fastest way to tell "wrong network" from "nothing's listening" when a device
doesn't answer.

### `discovery` — finding modules on a subnet

[`discover(&handle, window)`](src/discovery.rs) broadcasts a `0000h` (serial number) read to
`ID_D = 0xFF`, collects answers for `window`, then follows up per-responder (unicast, so it's safe
even when several modules share a device ID) with reads of `0009h` and `0100h`. Every XTREM ships
with device ID `01`, so a fresh multi-module install will produce ID collisions on broadcast —
`discover` dedupes by source IP and reports the collision on [`XtremProbe::id_collision`](src/discovery.rs)
rather than silently merging them. [`assign_device_id`](src/discovery.rs) resolves that by writing
a unique ID to `0001h`, one module at a time.

### `devices` — the scale driver

[`XtremDevice`](src/devices/mod.rs) is the polling contract, deliberately shaped like
`modbus::ModbusDevice` so wiring it into `machine_implementations` later is mechanical:

```rust
pub trait XtremDevice {
    fn send_next_request(&mut self) -> Result<(), anyhow::Error>;
    fn handle_response(&mut self) -> Result<(), anyhow::Error>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
```

Neither method blocks: `send_next_request` no-ops while a request is in flight (no pipelining),
`handle_response` drains whatever arrived via `try_recv`. [`XtremScale`](src/devices/xtrem_scale.rs)
implements it, with two modes:

- `ScaleMode::Poll` — repeatedly reads `0107h`. Deterministic, and the only way to tell "the link
  is alive" from "nothing's moving," since silence is otherwise ambiguous.
- `ScaleMode::Stream { interval_ms }` — writes `0013h`, executes `1011h`, then just listens.
  Cheaper on the wire, but the driver has to actively watch for the stream going stale (a module
  reboot stops it without saying so) and re-arm after ~10 missed intervals — see
  `stream_is_stale` in [`xtrem_scale.rs`](src/devices/xtrem_scale.rs).

Public surface: `reading: Option<Reading>` (`{ gross, tare, net, status, at }`), `device_state:
Option<DeviceState>`, `last_error: Option<XtremError>` (take with `.take_error()`), plus commands
`tare()`, `clear_tare()`, `zero()`, `request_device_state()`. `Drop` best-effort stops the stream
and deregisters from the bus so a dropped driver doesn't leave a module flooding the subnet.

## Using it

```rust
use std::time::Duration;
use common::get_async_runtime;
use units::mass::gram;
use xtrem::{ScaleMode, XtremDevice, XtremScale, discovery};
use xtrem::transport::{XtremBus, XtremBusConfig};

let bus = XtremBus::open(XtremBusConfig {
    bind_addr: "0.0.0.0:5555".parse()?,       // register 0700h default (module -> host)
    broadcast_addr: "192.168.4.255:4444".parse()?, // register 0701h default (host -> module)
    host_id: 0x00,
    verify_lrc: true,
    crlf: true,
})?;

// discover() is async — drive it from the shared runtime, same as examples/discover.rs does.
let probes = get_async_runtime().block_on(discovery::discover(&bus, Duration::from_secs(2)))?;
let mut scale = XtremScale::from_probe(&bus, &probes[0], ScaleMode::Poll);

// send_next_request/handle_response are non-blocking, meant for a synchronous poll loop
// (a machine's act() tick, or the plain loop below).
loop {
    scale.send_next_request()?;
    scale.handle_response()?;
    if let Some(reading) = scale.reading {
        println!("{:.1} g net", reading.net.get::<gram>());
    }
    std::thread::sleep(Duration::from_millis(10));
}
```

Run the real CLI against hardware:

```bash
cargo run --example discover -- --bind 0.0.0.0:5555 --broadcast 192.168.4.255:4444
# --no-lrc      if the module has LRC checking disabled (register 0011h)
# --stream <ms> use stream mode instead of polling
```

`--bind` must stay `0.0.0.0` — never a specific interface IP, even on a multi-homed host. The
module always replies to the broadcast address, never to the requester's unicast source IP, so a
socket bound to one specific address silently never sees the reply; [`udp::bind_socket`](src/transport/udp.rs)
now refuses to open a socket bound to anything else, for exactly this reason.

## Testing

```bash
cargo test          # protocol unit tests + tests/loopback.rs + doctest
cargo build          # from qitech_lib/ root, builds the whole workspace including this crate
```

`tests/loopback.rs` runs a fake XTREM module on `127.0.0.1` that speaks the real wire format, and
exercises the full stack end to end — discovery, polling, streaming (including the stale-stream
re-arm), tare, and multi-device `ID_O` demuxing — with no hardware required. This is the test suite
to extend when changing protocol or transport behavior; it catches the class of bug that pure
frame-codec unit tests can't (timing, concurrent devices on one bus, `Drop` cleanup).