
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`qitech_lib` is the hardware layer for QiTech machines: EtherCAT terminal drivers (Beckhoff, WAGO, Panasonic), the GRAM XTREM weighing-scale protocol over UDP, and SI unit types. The API is explicitly not stable; breaking changes are expected.

## Layout and build model

The root `Cargo.toml` is a single package (`qitech_lib`), **not** a workspace. The sibling crates are plain path dependencies, and `src/lib.rs` only re-exports them (`pub use ethercat_hal;` etc.):

- `ethercat_hal/`: EtherCAT master, state machine, all terminal drivers, PDO/CoE building blocks, io abstractions
- `ethercat_hal_derive/`: proc macros `#[derive(EthercatDevice)]`, `RxPdo`, `TxPdo`. They emit `crate::...` paths, so they only work inside `ethercat_hal`
- `xtrem/`: XTREM scale protocol (UDP, Tokio). Its `README.md` summarizes the protocol spec and which registers are modelled
- `units/`: physical quantity types on top of `uom`
- `common/`: shared helpers (Tokio runtime, IRQ/thread affinity)

Each crate declares its own dependency versions. `ethercrab` comes from QiTech's fork (`github.com/qitechgmbh/ethercrab-dc`, pinned rev) for DC support; don't swap it for the crates.io version.

## Commands

```bash
cargo build                    # CI: RUSTFLAGS="-D warnings"
cargo build --examples         # CI: RUSTFLAGS="-D warnings"
cargo fmt --all --check        # CI format check

# Because the sub-crates aren't workspace members, `cargo test -p <crate>` is refused.
# Test them through their own manifest:
cargo test --manifest-path ethercat_hal/Cargo.toml
cargo test --manifest-path ethercat_hal/Cargo.toml <test_name>
cargo test --manifest-path xtrem/Cargo.toml          # includes tests/loopback.rs (fake XTREM over UDP)

# Examples against real hardware (needs a NIC name)
cargo run --example el2004_minimal -- <interface>
cargo run --manifest-path xtrem/Cargo.toml --example discover
```

On Linux, `.cargo/config.toml` sets `bin/run-linux` as the runner for **every** `cargo run`/`cargo test` binary: it runs `sudo setcap cap_dac_override,cap_net_raw,cap_sys_nice,cap_ipc_lock` on the binary and chowns `/dev/ttyUSB*`, so expect a sudo prompt. Raw sockets and RT priority don't work without those capabilities. Unit tests don't need them; without a TTY, bypass the runner with `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=env`. CI installs `libudev-dev`.

The `mock` feature (`init_ethercat_mock`, `MockConsumer`/`MockProducer`) currently does not compile and CI doesn't build it.

## EtherCAT architecture (`ethercat_hal`)

**Threads and data flow.** `init_ethercat(interface, Option<MasterConfiguration>)` (`lib.rs`) spawns the `EthercatStateMachine` thread running `EtherCATController::ethercat_state_machine` (`controller.rs`), which in turn sets up a TX/RX thread for ethercrab's PDU loop (`setup_tx_rx_thread`; core pinning and RT priority via `RtOptimizationConfig`). It returns an `EtherCATControl` with two public halves:
- `channel` (`EtherCATThreadChannel`): requests to the master thread over `mpsc` (`ChannelRequest` / `DiagnosticRequest`, answered on a per-request response channel), e.g. `request_state_change(EtherCATState::PreOp/Op)` and SDO reads/writes. The controller only advances state when asked; `request_state_change` doesn't wait, so examples poll `get_state()`.
- `app_handle` (`EtherCATAppHandle`): the application's side of the cyclic process image. Inputs arrive through a triple buffer (`TripleBufConsumer`), outputs go out through `Arc<Mailbox>`, both behind the `Consumer`/`Producer` traits. Use `get_inputs()`/`finish_read()` and `write_outputs()`/`send_outputs()`. State, cycle counters, DC time and the subdevice list are shared via atomics / a mutex. Failed cycles in OP arrive as `OpCycleError`s via `try_recv_op_error()`; `handle_op` tolerates `max_consecutive_tx_rx_failures` transient ones in a row (`is_retryable` in `controller.rs`) and stops on anything else or when the TX/RX thread exits.

State progression is Init → PreOp → PreopPdi (with DC) → Op. Failed transitions are recorded in a `TransitionLog` (`al_diagnostics.rs`), read via `get_transition_reports()` / `get_last_transition_failure()`.

**Application loop pattern** (see `examples/*_minimal.rs`): go to PreOp then Op, call `try_get_subdevices_vec_sync()`. Each `MetaSubdevice` carries vendor/product/revision and the `start_tx..end_tx` / `start_rx..end_rx` byte ranges inside the process image. Slice the image per device, wrap it as `BitSlice<u8, Lsb0>`, and call the driver's `input()` / `output()`. Drivers don't own the bus; they only encode and decode their PDO slice.

**Drivers** (`src/devices/{beckhoff,wago,panasonic}_modules/`): every driver implements `EthercatDevice` (+ `NewEthercatDevice`, `EthercatDeviceProcessing`), usually via `#[derive(EthercatDevice)]` and `#[derive(RxPdo)]`/`#[derive(TxPdo)]` structs whose fields are `Option<PdoObject>` tagged `#[pdo_object_index(0x16xx)]`. Identity constants (`*_IDENTITY_A`, `*_PRODUCT_ID`) live in each driver; `devices/mod.rs` maps identities to drivers in `device_from_subdevice_identity(_rc)`, so register new drivers there. Other layers: `pdo/` (PDO objects shared by device families), `shared_config/` + `coe.rs` (CoE/SDO config structs, `Configuration::write_config`), `io/` (vendor-independent traits like `DigitalOutputDevice` that drivers implement), `helpers/` (counter wraparound, velocity conversion).

**WAGO is different:** the 750-354 coupler (`wago_750_354.rs`) is the EtherCAT subdevice. The 750-xxx I/O modules behind it are discovered over SDO and implement `DynamicEthercatDevice` / `EthercatDynamicPDO` with tx/rx offsets inside the coupler's PDO. To add a module, start from `devices/wago_modules/BOILERPLATE.rs` (its header lists the steps) and register its product ID in the `match ident_iom` block in `wago_750_354.rs`.

**Errors:** most fallible APIs return `anyhow::Error`; the state machine uses `EthercatErr` (`lib.rs`).
