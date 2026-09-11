# Remote/Touch software revalidation — 2026-09-12

This checkpoint records bounded current-main software evidence for
`COV-REMOTE-TOUCH-001`. It does not claim LAN, peer-device, DJ Link hardware,
RDM/TOD, or TouchOSC acceptance.

## Source and scope

- Source base: `6b88ec8a353f6233f6f6dd606ecad84036c7aa50` (`main`)
- `origin/main` matched before the checkpoint; no product source was changed.
- The checks exercise existing Touch/DJ Link contracts and the existing
  fail-closed Web Remote/DJ Link transport tests. No permissive fallback,
  new route, or new feature was added.

## Focused evidence

| Check | Result |
| --- | --- |
| `node app/scripts/check-dvc-touch-feature-preset.mjs` | PASS — 10 assertions |
| `node app/scripts/check-workspace-navigation-controller.mjs` | PASS — history back/forward, guard, branch, rejection/throw rollback, project fencing, stale state isolation, traversal/route/epoch races, and cleanup |
| `node app/scripts/check-dj-track-mapping-policy.mjs` | PASS |
| `node app/scripts/check-dj-link-runtime.mjs` | PASS — frontend/runtime contract |
| `cargo test -p io --release --locked remote_ -- --test-threads=1` | PASS — 67 passed, 0 failed, 1 ignored; 115 filtered out |

The native Rust run used `vcvars64.bat -vcvars_ver=14.44` and the exact
Build Tools x64 linker returned first by `where.exe link.exe`:
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.

The executed `io` tests cover missing/malformed/wrong pairing PINs,
Host/Origin mismatch, exact `/dj-link` path separation, token authentication,
replay/conflict/rollback, process-fence and listener-restart behavior, invalid
frame rejection, rate limiting, connection capacity, stale generation
rejection, ACK failure handling, and worker reaping. The ignored test requires
`SYNDOCAL_RB_OUTPUT_TEST_ROOT` and a local Node runtime; it is not counted as
hardware or peer acceptance.

## Remaining boundary

`COV-REMOTE-TOUCH-001` remains `In progress`. No LAN client exercise, TCP
listener exposure proof, NIC ambiguity resolution, token/bind restoration
proof, CAS mapping, Pedal/Agent, TouchOSC/iPad, or RDM/TOD physical
acceptance was performed. The row therefore does not claim remote exposure
safety or DJ Link hardware acceptance.
