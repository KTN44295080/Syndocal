# Timeline Follow checkpoint — 2026-09-13

This checkpoint closes `TIMELINE-FOLLOW-001` for the supported current-source
software and rendered-browser scope. The product implementation was already
present on the checkpoint base; this unit adds the independent completion
evidence and updates the authoritative ledgers. No Follow behavior was
weakened and no external device or publication claim is made.

## Implemented contract verified

- Follow admission is generation/authority fenced and captures the exact next
  Timeline and source mapping before transport mutation.
- Video crossfade uses the authored transition curve; audio follows the same
  progress, and the engine slews BPM across the admitted transition without
  splitting source/target ownership.
- Audio, video, and Lighting settlement use explicit quorum/ACK state. Late
  target failure, timeout, stale authority, ABA, rollback, abort, and receipt
  replay fail closed under the authored `Hold`, `Cut`, or `Fault` policy.
- `Trans` is emitted at transition admission and `Complete` is emitted once at
  settlement. Hold and `wait_for_pedal` retain the exact destination state and
  do not synthesize a second transport.
- The renderer exposes runtime truth, E/R/H fencing, bounded polling, abort
  receipt recovery, focus fencing, and normalized persisted immediate/hold/
  Pedal-1 controls.

## Verification

The browser plugin was unavailable in this environment. The documented regular
Playwright fallback used the installed Chrome executable and covered the
rendered Timeline performance surface at `1920x1080`, `1366x768`, `860x520`,
and `1280x720`, including Follow runtime badges, operator abort target, wheel
contracts, and the direct-resize fixture.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:timeline-follow-runtime` | PASS — E/G stale-result rejection, E/R/H abort receipt recovery, runtime visibility, focus, target, and localization |
| `pnpm.cmd --dir app run check:timeline-follow-hold-ui` | PASS — Immediate/Hold and one-measure wait-for-Pedal-1 controls, normalization, and persistence |
| `pnpm.cmd --dir app run check:timeline-advanced` | PASS — authoritative Timeline snapshot fencing and five-domain lane planning |
| `pnpm.cmd --dir app run check:timeline-cue-audio` | PASS — cue-audio fences, single-flight, latest mutation, and canonical invokes |
| `pnpm.cmd --dir app run check:timeline-loop-runtime` | PASS — loop authority, receipt, stale/reply-loss, and convergence |
| `pnpm.cmd --dir app run check:timeline-transport-runtime` | PASS — transport routing, snapshot convergence, retry/latest intent, and fail-closed pairing |
| `cargo test -p engine --release --locked -j 1 timeline_follow -- --nocapture --test-threads=1` | PASS — 27 passed, 0 failed, 0 ignored |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 timeline_follow -- --nocapture --test-threads=1` | PASS — 3 passed, 0 failed, 0 ignored |
| `pnpm.cmd --dir app run check:timeline-performance` with `PLAYWRIGHT_MODULE_PATH` and `CHROME_PATH` set | PASS — four viewport runs plus direct-resize Follow fixture |

The Cargo checks ran after `vcvars64.bat -vcvars_ver=14.44`; `where.exe
link.exe` returned the required first linker:

```text
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
```

## Boundary

This closes the current-source Follow/runtime/UI and rendered-browser slice of
`TIMELINE-FOLLOW-001`. It does not claim native renderer/GPU/display behavior,
audible devices, real A/V/Lighting synchronization, external MTC/DJ clocks,
physical output, venue/soak operation, cross-platform execution, signing,
publication, or product-wide Timeline acceptance. `COV-TIMELINE-MUSICAL-001`
therefore remains `In progress` while `TIMELINE-PERSISTENCE-001` and those
separate acceptance boundaries remain open.

`git diff --check`: PASS before commit.
