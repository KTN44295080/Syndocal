# ASIO Persistence and Package Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-PERSISTENCE-PACKAGE-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `4aeacb32778b906ba9edc788198d1a717c089832`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`, `qa/ASIO_SDK_PIN.json`

This checkpoint revalidates current-source selection persistence and package
separation. It does not claim final native UI telemetry or public package
acceptance.

## Verification

```text
pnpm.cmd --dir app run check:live-audio
pnpm.cmd --dir app run check:asio-packaging
```

Result: exit code 0.

- Live-audio fail-closed lifecycle, availability, explicit selection
  persistence, presentation, and request ordering: PASS.
- ASIO packaging boundary: `169 assertions`, PASS.
- The package proof retains exact normal-package resources, rejects ASIO bridge
  payloads/globs and retired names, checks SDK-pin immutability, and rejects
  distribution approval drift.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun — 2026-09-14

The live-audio lifecycle and ASIO packaging checks were rerun after takeover.
Both passed; packaging reported 169 assertions. The run again verified
selection persistence and normal-package ASIO rejection without producing a
licensed/public ASIO artifact, installer, updater overlay, or final package.

## Unresolved acceptance

`ASIO-PERSISTENCE-PACKAGE-001` stays Open. The final gate still requires native
telemetry on the current artifact, stale/ambiguous device locking on the
operator PC, exact feature identity, and a final package review after the
licensing decision. The current `distribution_approved: false` state correctly
keeps the public installer/updater from carrying the bridge, but that safety
boundary is not a completed ASIO release package.

Next action is the approved artifact's native persistence/package rehearsal,
with exact executable/bridge hashes, restart and stale-device logs, and package
inventory attached.

## Takeover continuation — current-source persistence/package recheck — 2026-09-14

At HEAD `e43f2148`, the live-audio selection/persistence/lifecycle check passed
and the ASIO package-separation self-test passed `169` assertions. The normal
package continues to reject the ASIO bridge payload and retired names while
the SDK pin remains protected. No licensed/public ASIO artifact, native stale
device rehearsal, installer, updater overlay, or final package review was
performed. `ASIO-PERSISTENCE-PACKAGE-001` remains `Open`.

## Takeover continuation — current-source persistence/package recheck — 2026-09-14

At current source HEAD `fd82d5cd`, the focused current-source contracts were
rerun:

```text
pnpm.cmd --dir app run check:live-audio
live audio fail-closed lifecycle, availability contract, selection persistence,
presentation, and request ordering ok

pnpm.cmd --dir app run check:live-audio-ipc-v1
live audio IPC v1 exact request mapping and fail-closed checks ok

pnpm.cmd --dir app run check:asio-packaging
ASIO packaging boundary self-test passed: 169 assertions
```

These results revalidate selection persistence and the normal-package boundary:
the exact resource inventory remains enforced, ASIO bridge payloads/globs and
retired names are rejected, and SDK-pin/distribution-approval drift is rejected.
No public or licensed ASIO artifact was created.

The native release-artifact self-test was not used as acceptance evidence in
this continuation: it emitted its candidate-extractor (`43 assertions`) and
materialization (`4 assertions`) sub-results but made no further progress and
was stopped before a successful process exit. No installer, updater overlay,
or package artifact was produced by that attempt.

`ASIO-PERSISTENCE-PACKAGE-001` remains `Open`. Completion still requires the
approved artifact's native telemetry, stale/ambiguous device-lock rehearsal,
exact executable/bridge identity, licensing decision, and final package review.
