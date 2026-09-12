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

## Non-claims and next gate

This is not LAN, native-window, two-process, two-machine, device, physical
output, automatic-failover, partition/rejoin, or venue acceptance. The next
ShowClock unit must add the bounded slew/Hold/STALE estimator and deterministic
delay/duplicate/reorder simulation before a wired-LAN adapter or UI status is
introduced.
