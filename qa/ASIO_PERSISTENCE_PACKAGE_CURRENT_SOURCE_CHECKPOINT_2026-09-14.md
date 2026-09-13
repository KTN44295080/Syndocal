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
