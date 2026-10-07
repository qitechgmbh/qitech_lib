# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`qitech_lib` is a Rust (edition 2024) library of building blocks for industrial machines: an EtherCAT master/HAL with device drivers (Beckhoff, WAGO, Panasonic), typed physical units, and a UDP driver for GRAM XTREM weighing modules. The API is explicitly unstable (see README).

## Commands

```bash
cargo build                       # CI builds with RUSTFLAGS="-D warnings" — warnings fail CI
cargo build --examples            # also built in CI with -D warnings
cargo fmt --all --check           # CI format check
cargo test -p ethercat_hal        # unit tests live inside ethercat_hal / xtrem sources
cargo test -p xtrem               # includes xtrem/tests/loopback.rs (fake XTREM module over UDP)
cargo test -p ethercat_hal el2522 # run tests matching a name filter
cargo run --example el2004_minimal -- <iface>   # EtherCAT examples take the NIC name as argv[1]
cargo run -p xtrem --example discover
```

The repo is a Cargo workspace whose root is also the `qitech_lib` package. Plain `cargo build`/`cargo test` at the root only act on the root package; use `--workspace` or `-p <crate>` for members. Shared dependency versions and the internal path crates are declared once in `[workspace.dependencies]` in the root `Cargo.toml`; members reference them with `<dep>.workspace = true` (adding `features = [...]` locally where needed). There is a single `Cargo.lock` at the root. CI needs `libudev-dev` installed.

`.cargo/config.toml` sets `bin/run-linux` as the runner for x86_64 Linux: every `cargo run`/`cargo test` binary is `setcap`'d (`cap_net_raw,cap_sys_nice,cap_ipc_lock`) via `sudo` and `/dev/ttyUSB*` is chowned before exec. Expect a sudo prompt; raw sockets, RT scheduling and `mlockall` need these capabilities.

## Crate layout

- **root (`src/lib.rs`)** — only re-exports `common`, `ethercat_hal`, `units`, `xtrem`. Feature `mock` forwards to `ethercat_hal/mock`.
- **`ethercat_hal`** — EtherCAT master built on a QiTech fork of `ethercrab` (pinned git rev), plus device drivers.
- **`ethercat_hal_derive`** — proc macros `RxPdo`/`TxPdo` (field attribute `#[pdo_object_index(0x....)]`). They generate `pdo::RxPdo`/`TxPdo` impls and a `coe::Configuration::write_config` that writes the PDO assignment to SDO `0x1C12`/`0x1C13` for every `Some` field. Generated code refers to `crate::...`, so they are only usable inside `ethercat_hal`.
- **`units`** — `uom` (f64) re-exports grouped by quantity.
- **`common`** — shared global multi-thread tokio runtime and Linux RT helpers (IRQ lookup/pinning via `/proc/interrupts`).
- **`xtrem`** — layered: `protocol` (pure frame codec, no I/O) → `transport` (one shared UDP socket, `XtremBus`, demuxing replies by the `ID_O` field) → `discovery` (broadcast sweep) → `devices` (`XtremScale`).

## EtherCAT architecture

`init_ethercat(iface, Option<MasterConfiguration>)` (in `ethercat_hal/src/lib.rs`) spawns the `EthercatStateMachine` thread running `EtherCATController::ethercat_state_machine` (`controller.rs`) and returns an `EtherCATControl` with:

- `app_handle: EtherCATAppHandle` — the application side of the process image. Inputs (device TxPDO) arrive via a **triple buffer** (`get_inputs`/`finish_read`); outputs (device RxPDO) go out through a single-slot **`Mailbox`** (`write_outputs` → fill → `send_outputs`). Both are raw byte arrays of `ETHERCAT_TX_RX_SIZE`. Cycle counter, state, DC system time, subdevice list, etc. are shared atomics/`Arc<Mutex<..>>`.
- `channel: EtherCATThreadChannel` — two `mpsc` senders: `ChannelRequests` (state changes, SDO read/write, DC sync config, oversampling, shutdown) handled per-state, and `DiagnosticRequest` (raw register reads / AL status) drained in every state. Responses come back on a per-request `ChannelResponse` sender.

State progression is requested by the application: `NoInterface → Init → PreOp → PreopPdi → Op` (see `EtherCATState`). SDO/CoE configuration of devices must happen in PreOp before requesting Op. Failed transitions are recorded in `al_diagnostics::TransitionLog` (`get_transition_reports`, `get_last_transition_failure`). `MasterConfiguration` holds cycle time, DC config, WKC mismatch threshold, and optional `RtOptimizationConfig` (core pinning, SCHED_FIFO priority, IRQ pinning, mlockall).

Each `MetaSubdevice` carries `start_tx..end_tx` / `start_rx..end_rx` byte ranges into the process image; the application slices those out and hands a `BitSlice<u8, Lsb0>` to the driver's `input()` / `output()`. `examples/el2004_minimal.rs` is the canonical minimal flow.

### Device drivers (`ethercat_hal/src/devices`)

- Every driver implements `EthercatDevice` + `NewEthercatDevice` (parameterless `new()`) + `EthercatDeviceProcessing` + `EthercatDeviceUsed`. Identity constants are `(vendor, product, revision)` tuples named `<DEV>_IDENTITY_<X>`.
- **Adding a driver requires registering its identity in both** `device_from_subdevice_identity` and `device_from_subdevice_identity_rc` in `devices/mod.rs` (they are parallel match tables). Use `downcast_subdevice` / `downcast_rc_refcell` to recover the concrete type.
- Simple terminals are flat files; complex ones (EL70xx steppers, EL1259) are directories with `mod.rs` + `pdo.rs` (derive-macro PDO structs) + `coe.rs` (CoE configuration). Reusable PDO objects live in `pdo/`, shared config structs in `shared_config/`, small conversion helpers in `helpers/`.
- Device-agnostic I/O traits live in `io/` (`DigitalOutputDevice`, `DigitalInputDevice`, analog in/out, encoder, …); drivers implement these so application code can stay generic.
- **WAGO**: `wago_750_354` is a coupler with up to 64 slotted `Module`s whose PDO offsets are discovered at runtime (`get_modules`, `get_pdo_offsets`). 750-xxx I/O cards implement `DynamicEthercatDevice`/`EthercatDynamicPDO` (offsets set at runtime) and report `is_module() == true`. New WAGO cards should start from `wago_modules/BOILERPLATE.rs`, which documents the steps.

### Feature flags

- `mock` (ethercat_hal) swaps `EtherCATThreadChannel` for an in-memory SDO map and enables `init_ethercat_mock`; that function still calls an older `EtherCATController::new` signature, so verify it compiles before relying on it.
- `legacy_code` is declared but currently gates nothing (the `cfg` on `machine_ident_read` in `lib.rs` is commented out).
