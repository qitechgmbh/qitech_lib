# xtrem

Rust implementation of the GRAM **XTREM / XTREM-S** weighing-module communication protocol
(v3.007), spoken over UDP. Lives in `qitech_lib` alongside `modbus` and `ethercat_hal` as a
sibling hardware-protocol crate.

The full spec is `qitech_lib/9_0_XTREM-com-protocol_R02.pdf`; a condensed version is on the
[wiki](https://github.com/qitechgmbh/control/wiki/Xtrem-Protocol). This document explains what
the crate implements, how it's laid out, and how to build a `machine_implementations` machine on
top of it. Read [`src/lib.rs`](src/lib.rs) for the authoritative module doc comment — this file
goes into more depth and won't always be kept as tightly in sync.

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
    bind_addr: "0.0.0.0:4444".parse()?,       // register 0700h default (module -> host)
    broadcast_addr: "192.168.4.255:5555".parse()?, // register 0701h default (host -> module)
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
cargo run --example discover -- --bind 0.0.0.0:4444 --broadcast 192.168.4.255:5555
# --no-lrc      if the module has LRC checking disabled (register 0011h)
# --stream <ms> use stream mode instead of polling
```

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

## Deploying against real hardware — field notes

Getting a physical unit talking over the network turned out to be the hard part, and none of it is
a code problem. If you're bringing up a new module, in order of likelihood:

1. **DHCP.** The module needs an IP before anything else works — it does not fall back to
   self-assigning a link-local address (unlike a Mac/Linux host on the same segment), so on a
   directly-wired link with no DHCP server, it just sits there indefinitely retrying `DHCPDISCOVER`
   with no fallback. Point a DHCP server at that segment (macOS Internet Sharing, or `dnsmasq`
   scoped to the interface) before doing anything else. `sudo tcpdump -i <if> -n udp` while
   power-cycling the module is the fastest way to confirm whether it's even trying.

2. **Routing, if the host has multiple interfaces.** On a multi-homed machine, sending to a
   `169.254.0.0/16` (link-local) broadcast address can silently go out the *wrong* interface — that
   subnet is scoped per-interface at the OS level, and an unscoped socket bound to `0.0.0.0` picks
   whichever interface owns the default route, not necessarily the one the module is on. Bind
   `--bind` to the specific interface address (not `0.0.0.0`) if discovery finds nothing despite
   the module clearly being connected. `route -n get <broadcast-addr>` vs.
   `route -n get -ifscope <if> <broadcast-addr>` shows the discrepancy directly.

3. **The documented UDP/TCP port defaults may not apply to your specific expansion hardware.** The
   spec's `0700h`/`0701h`/`0702h` port defaults are documented under the Wi-Fi module section
   specifically (§17). A wired-Ethernet expansion board is a physically different piece of
   hardware and may be a generic third-party serial-to-Ethernet bridge module underneath (look for
   `Server: lwIP/...` in its HTTP response headers if it has a web UI — that's the signature) with
   its own independent port configuration, possibly gated behind a login reserved for
   calibration/OEM personnel. If the documented ports don't respond (a UDP `ICMP port unreachable`
   or TCP `Connection refused` both mean "nothing is listening here," definitively — that's a real
   answer, not a transient failure), that's a hardware provisioning question, not something to
   debug further in this crate.

4. **UART0 (RS232) is always present**, independent of whatever UART1 expansion is installed, and
   speaks the exact same frame format over a USB-to-RS232 adapter instead of UDP. It doesn't touch
   whatever the network module's state is, so it's a clean fallback for validating the protocol
   against real hardware when the network path is blocked. This crate doesn't implement a serial
   transport (out of scope by design — see [`src/lib.rs`](src/lib.rs)), but `protocol::Frame` is
   fully transport-agnostic, so adding one would only mean writing a new `transport/serial.rs`
   alongside the existing UDP one, not touching the codec.

5. **Don't run a full 65535-port scan against one of these modules.** They run minimal embedded
   TCP/IP stacks (lwIP, in the one case investigated) not built for scan-level traffic volume, and
   a full sweep can make the module stop responding to *everything*, including ICMP, well before
   it's actually crashed — `ping` afterward is the fast way to confirm it's still alive. Prefer a
   targeted top-ports scan.

## Building a machine on top of this

This crate is protocol + driver only, by design — no `machine_implementations` integration exists
yet. This section documents the gap precisely so implementing it is mechanical, following
`machine_implementations/src/laser/` as the closest existing pattern (also a serial device driver
polled from a machine's `act()` loop). Read [`laser/mod.rs`](../../control/machine_implementations/src/laser/mod.rs),
[`new.rs`](../../control/machine_implementations/src/laser/new.rs),
[`act.rs`](../../control/machine_implementations/src/laser/act.rs), and
[`api.rs`](../../control/machine_implementations/src/laser/api.rs) alongside this — they're the
source of truth; the project's `creating-a-machine`/`backend` skill docs describe an older API
(`MachineNewTrait`, `MachineAct::act(&mut self, now: Instant)`) that no longer matches the code.

### The one real gap: `Hardware` has no network-device variant

[`machine_implementations/src/lib.rs`](../../control/machine_implementations/src/lib.rs) defines:

```rust
pub enum Hardware {
    Ethercat(IdentifiedEthercat),
    Modbus(IdentifiedModbus),
}
```

`try_get_serial_device_by_index::<T>()` only matches `Hardware::Modbus` — it downcasts through the
`ModbusDevice` trait object. `XtremScale` doesn't implement `ModbusDevice` (nor should it pretend
to; it's a different protocol and transport entirely), so wiring an XTREM scale into a machine
needs a third variant added first:

```rust
#[derive(Clone)]
pub struct IdentifiedXtrem {
    pub hw: Rc<RefCell<dyn XtremDevice>>,
}

pub enum Hardware {
    Ethercat(IdentifiedEthercat),
    Modbus(IdentifiedModbus),
    Xtrem(IdentifiedXtrem),
}
```

plus a `try_get_xtrem_device_by_index::<T>()` accessor mirroring `try_get_serial_device_by_index`
(same `as_any`/`as_any_mut` downcast pattern — `XtremDevice` already exposes those for exactly this
purpose). Where the resulting `Hardware::Xtrem` entries actually get constructed (discovery +
`XtremBus` lifecycle, analogous to `qitech_control/src/app_state.rs`'s
`generate_machine_hardware_from_serial` for the laser) is a `control` repo / `qitech_control`
concern, not something this crate owns — it's network-discovered, not hot-plugged off a serial
port list, so it won't reuse that exact function, only the general shape.

### Machine module layout

```
machine_implementations/src/<name>/
  mod.rs    struct, ConvertMachineData impl, get_state/emit_state/get_live_values/emit_live_values
  new.rs    impl MachineNew { fn new(hw: MachineHardware) -> Result<Self, Error> }
  act.rs    impl Machine { fn act(&mut self, reg: Option<&mut MachineDataRegistry>) -> ... }
  api.rs    socket.io events + Mutation enum + impl MachineApi
```

### `new.rs` — acquire the device

```rust
impl MachineNew for ScaleMachine {
    fn new(hw: MachineHardware) -> Result<Self, anyhow::Error> {
        let scale = hw.try_get_xtrem_device_by_index::<XtremScale>(0)?; // once the accessor above exists
        // ... build channels, namespace, initial state, matching LaserMachine::new ...
    }
}
```

### `act.rs` — the control loop tick

Identical shape to [`laser/act.rs`](../../control/machine_implementations/src/laser/act.rs):
drain `api_receiver`, call an `update()` that does `send_next_request()` +
`handle_response()` on the borrowed `XtremScale` and copies `reading`/`last_error` into the
machine's own fields, emit state on change, throttle live-value emission to ~30 Hz, and on a fatal
error (`XtremError` mapped to `MachineError`) return it from `act()` so the machine is torn down
rather than left spinning on a dead device.

```rust
impl Machine for ScaleMachine {
    fn act(&mut self, reg: Option<&mut MachineDataRegistry>) -> Result<(), MachineError> {
        if let Ok(msg) = self.api_receiver.try_recv() {
            self.act_machine_message(msg);
        }
        self.update(); // send_next_request + handle_response + copy XtremScale::reading in
        if self.did_change_state {
            self.emit_state();
        }
        // throttle live_values emission to 1/30s, matching laser/act.rs
        // store into `reg` via ConvertMachineData, matching laser/act.rs
        Ok(())
    }
    fn react(&mut self, _registry: &MachineDataRegistry) {}
    fn get_identification(&self) -> MachineIdentificationUnique { self.machine_identification_unique }
}
```

### `api.rs` — events and mutations

`LiveValuesEvent` naturally carries `net_weight: f64` (from `reading.net.get::<units::mass::gram>()`),
`gross_weight: f64`, `stable: bool` (from `reading.status.stable()`). `StateEvent` carries
`tare: f64`, `sealed: bool`, `device_state` summary. Mutations mirror the driver's commands:

```rust
enum Mutation {
    Tare,
    ClearTare,
    Zero,
}
```

dispatched in `api_mutate` by calling straight through to `XtremScale::tare()` /
`clear_tare()` / `zero()` on the borrowed device — same pattern as `Mutation::SetHigherTolerance`
calling `LaserMachine::set_higher_tolerance` in [`laser/api.rs`](../../control/machine_implementations/src/laser/api.rs).

### Registering it

```rust
// machine_implementations/src/lib.rs
pub const MACHINE_SCALE_V1: u16 = 0x00XX; // next free constant

// machine_implementations/src/registry.rs, inside MACHINE_REGISTRY
mc.register::<ScaleMachine>(vec![ScaleMachine::MACHINE_IDENTIFICATION]);

// machine_implementations/src/machine_identification.rs, in slug()
x if x == MACHINE_SCALE_V1 => "scale_v1".to_string(),
```

Then a frontend `MachineProperties` entry in `electron/src/machines/properties.ts` with
`device_roles: []` (per the `creating-a-machine` skill: valid for machines with no EtherCAT
provisioning UI — a network-discovered device has nothing to provision through the
EtherCAT-terminal-picker UI that role slots exist for).
