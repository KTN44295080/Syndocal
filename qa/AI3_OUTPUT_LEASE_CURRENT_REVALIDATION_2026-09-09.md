# AI3 output-lease current-main revalidation — 2026-09-09

## Scope

This checkpoint revalidates the existing AI3 output-lease authority and
durable-boundary implementation on the current `main` source. It does not add
an output feature, alter the machine-local physical gate, or claim external
client, hardware, venue, or product-wide completion.

- Source HEAD: `9f4982b2e445c387e246057b610aa6e4452c5366`
- Product source changes in this checkpoint: none
- Test-owned files or project data changed: none
- Application process, physical output, external client, and device: not started

## Verification

The Windows native procedure was applied before Cargo: Visual Studio Build
Tools `14.44.35207` x64 was initialized, the exact absolute linker was pinned
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and `where.exe link.exe`
returned it first.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked output_lease -- --nocapture --test-threads=1
Finished `release` profile [optimized] target(s) in 5m 59s
test result: ok. 133 passed, 0 failed, 0 ignored, 0 measured; 1682 filtered out
```

The focused set covered the pure lease transitions, canonical resource and
owner-incarnation validation, expiry/orphan/recover/relinquish/transfer,
overlap and capacity rejection, request receipts and shape conflicts, rate
limits, process-restart non-reclamation, durable receipt reload, physical
commit-boundary refusal, exact-Both keepalive, poisoned-lock fail-closed
paths, worker timeout/reap ownership, stale callback fencing, and managed
cleanup/retry boundaries.

Several tests deliberately panic inside injected poison seams; the panic
messages are caught by those tests and the aggregate result above is the
authoritative pass result. No assertion was weakened.

## Boundary

This proves the current software-level output-lease regression surface only.
`AI3-DURABLE-RECOVERY-001` remains open for crash/restart acceptance beyond
the bounded local journal, canonical ingress across every legacy adapter,
physical project replacement and re-Arm, external-client reply-loss, and real
Art-Net/NDI/Spout/display/ASIO hardware evidence. It does not claim AI4
principal/grant/consent services, physical output, Mac execution, signing,
publication, or whole-product completion.
