# Warning Ratchet Windows Audit — 2026-09-08

## Scope

This checkpoint repairs the warning-ratchet checker and its self-test for the
current Windows development environment. It does not change product runtime
code, warning baselines, Cargo assertions, or release artifacts.

## Changes

- `app/scripts/warning-ratchet-lib.mjs`
  - Accepts the installed local BuildTools MSVC 14.44.35207 linker in addition
    to the already supported Community toolset path.
  - Selects the installed local `vcvars64.bat` and derives the exact pinned
    linker from `VCToolsInstallDir`.
  - Invokes the available pnpm JavaScript entry point directly on Windows when
    possible, avoiding `cmd.exe` command-echo contamination. The `.cmd`
    invocation remains a narrow fallback.
- `app/scripts/test-warning-ratchet.mjs`
  - Uses the application package directory for the generic pnpm fixture.
  - Passes shell-sensitive fixture JavaScript through the controlled child
    environment so Windows command parsing cannot change the test case.

No entry was added to `qa/warnings/warning-inventory.json`.

## Evidence

All commands were run from the repository checkout at main `f20bd8937c86ebc5101e2b846086ad0a30d6c924`.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:warnings:self-test` | PASS — `warning ratchet self-tests ok` |
| `pnpm.cmd --dir app run check:release` | PASS |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — `50 Open + 8 Deferred authority rows` |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — `32 Q1 rows`, `58/58 flow markers` |
| `node app/scripts/check-warning-ratchet.mjs --configuration windows-default-release` | BLOCKED by existing artifact inventory drift after Cargo execution |

The warning-ratchet run initialized the exact local toolchain and executed
`cargo check --workspace --all-targets --release --locked
--message-format=json` successfully. It then stopped fail-closed with:

```text
Cargo artifact coverage mismatch: missing=[] unexpected=[{"package":"protocol","target":"dj_link_v3_sender_contract","targetKinds":["test"],"crateTypes":["bin"]}]
```

The target is present in the current repository history, but is absent from
the checked-in warning inventory. This checkpoint intentionally does not
rebaseline it and does not call the warning gate a pass. The inventory drift
must be resolved as a separately reviewed warning-baseline decision.

## Boundary

No native rebuild, executable launch, device access, physical output, signing,
or publication was performed for this checker-only checkpoint. The existing
native evidence remains governed by the earlier native QA records.
