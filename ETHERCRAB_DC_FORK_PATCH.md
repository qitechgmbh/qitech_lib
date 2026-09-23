# ethercrab-dc fork patch: drop stale/unclaimable frame responses

Patch applied manually to the checked-out `ethercrab-dc` git dependency. This file documents
the change so it can be re-applied after a fresh clone / `cargo clean` of the dependency.

## Why

During a failed `PreOp+PDI -> SafeOp` transition (`into_safe_op`), the main thread aborts frames
that have been sent but whose responses are still in flight (state-transition timeout). When such
a late response arrives, the receive path failed to claim the frame element (it had already been
released) and returned `PduError::InvalidIndex`, which propagated up and killed the whole TX/RX
thread via `expect("Failed to run TX/RX task (io_uring)")` in `ethercat_hal/src/controller.rs:116`.

Once the PDU loop is dead, every subsequent read times out, so `ethercat_hal`'s AL diagnostics
(`read_al_statuses`) reported every device as `unreachable (Timeout(Pdu))` and the EL7062's actual
`ALStatusCode` (the real reason SafeOp was rejected) was never captured.

## Location

- Dependency: `ethercrab-dc` git rev `e84f794db7f7ebe321a6608c7cd1808ca54a6eb0`
  (`Cargo.toml` in `ethercat_hal/`: `ethercrab = { git = "https://github.com/qitechgmbh/ethercrab-dc", rev = "e84f794..." }`)
- Checkout path: `~/.cargo/git/checkouts/ethercrab-dc-1e7b5c22e3034776/e84f794/`
- File modified: `src/pdu_loop/pdu_rx.rs`

## Change

In `receive_frame`, the lookup of the in-flight frame element:

```rust
// Before
let mut frame = self
    .storage
    .claim_receiving(frame_index)
    .ok_or(PduError::InvalidIndex(frame_index))?;

// After
// The frame element may have been released before this response arrived, e.g. the sender
// timed out or a transition was aborted while the response was still in flight. The frame
// can no longer be claimed, so drop it rather than killing the whole PDU loop.
let Some(mut frame) = self.storage.claim_receiving(frame_index) else {
    fmt::trace!(
        "Ignoring response for frame {}: frame is no longer in flight",
        frame_index
    );

    return Ok(ReceiveAction::Ignored);
};
```

This is a logic change, not just logging: a late/duplicate response is now discarded
(`ReceiveAction::Ignored`, a path the receive loop already treats as normal) instead of erroring
out the PDU loop. `claim_receiving` is a state machine transition (`Sent -> RxBusy`); when it
fails the element is gone (sender already released it), so there is nothing to hand over.

## How to re-apply after re-fetching the dependency

The edit above is not re-applied automatically. After a fresh copy of the checkout (e.g. after
a `cargo update`, removing `~/cargo/git/checkouts/ethercrab-dc-*`, or building on another
machine) redo the single `let Some(...) = ... else { return Ok(ReceiveAction::Ignored); }`
replacement in `src/pdu_loop/pdu_rx.rs`.

## How to rebuild after editing the checkout

Cargo only recompiles a git dependency when the pinned `rev` changes; edits to the checked-out
source are not always detected. Force a rebuild:

```sh
touch ~/.cargo/git/checkouts/ethercrab-dc-1e7b5c22e3034776/*/pdu_loop/pdu_rx.rs \
  ~/.cargo/git/checkouts/ethercrab-dc-1e7b5c22e3034776/*/Cargo.toml
cargo clean -p ethercrab
cargo build --example el7062_minimal
```

## Status

- Applied and compiled (passed `cargo build --example el7062_minimal`).
- Purpose of patch: keep the PDU loop alive through failed transitions so the AL diagnostics can
  report the real `ALStatusCode` of the rejecting slave. It does not fix the underlying SafeOp
  rejection; that is still under debug.