# F2 local output ownership checkpoint

Date: 2026-09-13
Branch: `codex/showclock-review-20260912`

## Result

`F2-OUTPUT-OWNERSHIP-001` is complete at the current-source local ownership
boundary. The implementation has one bounded lease authority for `Lighting`,
`Video`, and `Both` resource sets, with `Standby` represented by the existing
all-deny machine role. It covers generation-fenced acquire/renew/recover,
expiry to `HeldOrphaned`, owner-incarnation/ABA rejection, atomic forced
transfer, explicit `relinquish_output_lease`, process-restart non-reclamation,
bounded receipts/audit/rate admission, project-orphan retirement, and the
managed exact-`Both` keepalive path.

The local output integration preserves the required order: output transition
and failure fences are held through DMX/video/native-window preparation and
teardown acknowledgement. Display, NDI, Spout, native-window, Standby, and
Take Over paths remain fail-closed when ownership or teardown acknowledgement
is absent. The pre-existing source contract and focused native tests remain the
authoritative implementation evidence; this checkpoint adds the F2-specific
current-source checker and ledger trace.

## Verification

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:f2-output-ownership:self-test` | PASS — 3 assertions |
| `pnpm.cmd --dir app run check:f2-output-ownership` | PASS — lease/role/retirement source contract |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS |
| `pnpm.cmd --dir app run check:safety-blackout-runtime` | PASS |
| Current-source `output_lease` focused Rust tests under pinned MSVC 14.44.35207 | PASS — 32 passed, 0 failed, 1848 filtered out, no first-party warnings |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — 41 Open + 8 Deferred + 9 Complete |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — 58/58 Flow markers and mirror parity |
| `pnpm.cmd --dir app run check:release:self-test` | PASS |
| `pnpm.cmd --dir app run check:release:static` | PASS |

The F2 completion is deliberately a software/local boundary. It does not
claim physical GPU/driver/HDMI behavior, fixture output, Art-Net/sACN, RDM/TOD,
external NDI/Spout/Syphon peers, real native UI acceptance, distributed
two-machine exclusivity, or venue operation. Those remain separately tracked
External acceptance markers and are not closed by source tests.
