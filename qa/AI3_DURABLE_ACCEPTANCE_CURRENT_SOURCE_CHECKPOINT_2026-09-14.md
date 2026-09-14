# AI3 durable acceptance current-source checkpoint — 2026-09-14

- Marker: `AI3-DURABLE-ACCEPTANCE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `4f2f259e46427a6a5d9eb56b166d36fe7af198b7`
- Product code change: none in this checkpoint

## Current-source verification

The durable recovery and dangerous-action admission seams were checked:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected |
| `pnpm.cmd --dir app run check:project-recovery-e3` | PASS — project authority and E3 recovery driver |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — output-control and Standby Sync contracts |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |

Together with `qa/AI3_DURABLE_RECOVERY_CHECKPOINT_2026-09-13.md`, this confirms
the bounded current-source journal, authority, admission, recovery, and output
ownership slices. It does not expand the existing software-only claim.

## Takeover rerun — 2026-09-14

The current-source suite was rerun after takeover. Agent Bridge passed 11
groups, bootstrap passed 4 deferred lifecycle groups, the admission inventory
passed with 539 commands and 18 rejected negative fixtures, E3 recovery passed,
and output-control/Standby Sync plus output ownership passed. No physical
output or native dangerous-action session was opened.

## Acceptance boundary

`AI3-DURABLE-ACCEPTANCE-001` remains `Open`. This checkpoint did not perform a
native dangerous-action Yes/No/Close session, external-client reply-loss or
restart acceptance, physical creation/teardown acknowledgement, or five-
display hardware run. No physical output was enabled.

## Resume procedure

Run the exact current artifact through the native dangerous-action matrix,
including reply loss, crash/restart, restart non-reclamation, physical ACK,
and five-display topology. Retain process identity, journal state, output
state, and first failure for every case before changing the Flow marker.

## Takeover continuation — current source recheck — 2026-09-14

The current checkout was rechecked at `3c75c9a0caf68cbaabc7188531d1ca21bc08f671`
after the takeover and before any physical output action. The following
read-only/source-only checks all exited `0`:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; real processor/runtime/confirmation modules; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected; SHA-256 `a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab` |
| `pnpm.cmd --dir app run check:project-recovery-e3` | PASS — project authority and E3 recovery driver |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — v2 output control, strict receipts, native-confirmation boundary, Standby Sync/lease UI |
| `pnpm.cmd --dir app run check:output-ownership` | PASS — output ownership static contract |

The current runtime still has the intended native-danger boundary: advanced
R4 actions use a parented OS Warning/Yes-No dialog; only `Yes` proceeds; a
`No` or close becomes terminal `Forbidden` before admission or mutation; and
an exact replay returns the stored terminal result without prompting again.
The deterministic Rust test covers this cancellation/replay behavior, while
the runtime contract checker verifies the production wiring. These are
implementation and source-contract results only.

No native dangerous-action dialog was opened in this continuation, and no
output, recording, device, or external client was contacted. Therefore the
marker remains `Open`; the required native No/Close observation, reply-loss
and crash/restart run, physical creation/teardown acknowledgement, and
five-display hardware acceptance are still unproven. No ledger status or
Q1/Q4 evidence count was changed from this source-only recheck.

## Takeover continuation — current-source durable-acceptance recheck — 2026-09-14

At current source HEAD `26e6d390`, the durable/recovery source contracts were
rerun:

```text
check:agent-bridge: PASS (11 groups)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
check:project-recovery-e3: PASS
check:output-control-runtime: PASS (including Standby Sync output-lease UI)
check:output-ownership: PASS
```

All six commands exited `0`. The source confirms durable journal/admission,
project recovery, output-control, ownership, and native-confirmation wiring;
it did not open a native dangerous-action dialog or perform reply-loss,
crash/restart, physical creation/teardown ACK, or five-display hardware work.

`AI3-DURABLE-ACCEPTANCE-001` remains `Open` pending the exact artifact's
native Yes/No/Close workflow, external-client reply-loss/restart matrix,
physical ACKs, and five-display hardware acceptance.

## Continuation — current artifact durable-acceptance boundary recheck — 2026-09-14

After the current release rebuild and Video desk repair at HEAD `a7d5a4d5`,
the durable/recovery source contracts were rerun:

```text
check:agent-bridge: PASS (11 groups)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
check:project-recovery-e3: PASS
check:output-control-runtime: PASS
check:output-ownership: PASS
```

All six commands exited `0`. They confirm the current journal/admission,
project-recovery, output-control, ownership, and native-confirmation source
boundaries. No native dangerous-action dialog was opened, no reply-loss or
crash/restart drill was run, no physical creation/teardown ACK was observed,
and no five-display hardware action was performed.

`AI3-DURABLE-ACCEPTANCE-001` remains `Open` pending the exact artifact's
native Yes/No/Close observation, external-client reply-loss/restart matrix,
physical ACKs, and five-display hardware acceptance.
