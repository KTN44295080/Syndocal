# ShowClock protocol software revalidation — 2026-09-12

## Scope

This checkpoint freezes the operator-approved ShowClock choices and implements
the transport-neutral admission boundary only:

- ShowClock is the canonical show-time master; device domains may follow it by
  bounded slew and may not step time backward.
- The target is two PCs on a wired LAN with manual pairing. Discovery is only an
  untrusted endpoint hint; the paired session, exact protocol version, HMAC,
  nonce, monotonic sequence, project/media hashes, and clock/fence generations
  are authoritative.
- A peer loss, crash, stale sample, or rejoin never arms output. The standby is
  in Manual Hold until an operator confirms the primary is stopped and performs
  an explicit Re-arm with an advanced fencing generation.

The implementation is in `crates/protocol/src/show_clock.rs`. It does not own
sockets, discovery, output leases, device handles, or physical output.

## Evidence

- Source base before this checkpoint: `7502c66901b83a66dbb0a4d1989a8c6973ba0723`.
- Command: `cargo test -p protocol --locked show_clock -- --nocapture`.
- Result: PASS, 3 passed, 0 failed, 0 ignored.
- Covered rejection paths: tampered sample/authentication tag, replayed and
  reordered samples, stale fencing generation, duplicate action, conflicting
  reuse of an action ID, missing operator confirmation, missing Hold, and
  non-advancing Re-arm generation.
- `sha2` was already present in the workspace lock; the protocol crate now
  declares that existing dependency directly for its HMAC-SHA256 boundary.

## Bounded admission hardening — 2026-09-12

The implementation review resumed from the clean documentation checkpoint
`1a49a3ef` and stayed within the transport-neutral protocol boundary. The
following findings were reproduced with focused regressions and fixed in
`crates/protocol/src/show_clock.rs`:

- `ShowClockPeerValidator` and `ShowClockActionReceiver` now share one peer
  context, and the authentication key is owned by a redacted Debug wrapper;
  the owners cannot print raw key bytes.
- Sample admission now retains and compares `sender_monotonic_us` in addition
  to sequence and show time. Backward timestamps are rejected atomically;
  equal timestamps are allowed and remain ordered by sequence.
- Action dedupe retains only a fixed-size SHA-256 fingerprint per action ID,
  with `SHOW_CLOCK_MAX_ACCEPTED_ACTIONS = 4096`. The receiver never evicts an
  ID; a new action at capacity fails closed without advancing admission state,
  while duplicates and conflicts remain deterministically classified.
- Manual Re-arm now receives the current clock-generation floor at
  construction and rejects a lower generation. The floor is monotonic and is
  explicitly not a claim that the fence owns the authoritative live clock;
  fencing generation must still advance and operator confirmation plus Hold
  remain mandatory.
- Session and node IDs validate again during serde deserialization, ASCII
  controls are rejected, action sequence zero reports `ZeroSequence`, and
  zero target time reports `InvalidTargetTime`. Canonical enum discriminants
  are explicit `repr(u8)` values preserving the existing v1 tags.
- Independent sample and action canonical/HMAC vectors lock the v1 domain,
  field order, little-endian encoding, and authentication output.

Evidence for this hardening:

| Command | Observed result |
| --- | --- |
| `rustfmt --check --edition 2021 crates/protocol/src/show_clock.rs` | PASS |
| `cargo test -p protocol --locked show_clock -- --nocapture --test-threads=1` | PASS — 8 focused tests, 0 failed; MSVC 14.44.35207 linker pinned and first in `where.exe link.exe` |
| `cargo test -p protocol --locked -- --test-threads=1` | PASS — 226 unit, 7 integration, 4 doctests; 0 failed/ignored; no compiler warnings in captured output |
| `git diff --check` | PASS |

The public constructor of `ShowClockManualFence` intentionally changes from
`new(fencing_generation)` to `new(clock_generation, fencing_generation)` so a
caller cannot create a fence without declaring its initial monotonic clock
floor. No production call sites existed at this checkpoint. This is a
protocol-core API hardening change, not a wire schema change.

## Non-claims and next gate

This is not LAN, native-window, two-process, two-machine, device, physical
output, automatic-failover, partition/rejoin, or venue acceptance. The next
ShowClock unit must add the bounded slew/Hold/STALE estimator and deterministic
delay/duplicate/reorder simulation before a wired-LAN adapter or UI status is
introduced.
