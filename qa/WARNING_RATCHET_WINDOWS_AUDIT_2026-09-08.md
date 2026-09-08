# Warning Ratchet Windows Audit — 2026-09-08

## Scope

This checkpoint repairs the warning-ratchet checker and its self-test for the
current Windows development environment. It also records a separately reviewed
Cargo artifact-coverage rebaseline for a target already emitted by the existing
warning command. It does not change product runtime code, warning assertions,
or release artifacts.

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
- `--rebaseline-artifacts`
  - Requires explicit ancestor base/head refs and one enforced Cargo
    configuration.
  - Permits inventory-only additions while rejecting removals, immutable field
    changes, other configuration changes, suppression loopholes, and failed or
    incomplete Cargo coverage.

The B checkpoint `b1e90813f103990dcc7f253175c5cf8dbeec1cf4` adds only the
observed `protocol/dj_link_v3_sender_contract` test artifact to
`windows-default-release` and binds its evidence commit to the reviewed A
checkpoint `db898cab30db1245ff4207282b9ba261728c1f1b`.

## Evidence

The checker implementation is A=`db898cab30db1245ff4207282b9ba261728c1f1b`;
the inventory-only rebaseline is B=`b1e90813f103990dcc7f253175c5cf8dbeec1cf4`.
The final documentation checkpoint is recorded after B→C validation.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:warnings:self-test` | PASS — `warning ratchet self-tests ok` |
| `pnpm.cmd --dir app run check:release` | PASS |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — `50 Open + 8 Deferred authority rows` |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — `32 Q1 rows`, `58/58 flow markers` |
| `node app/scripts/check-warning-ratchet.mjs --configuration windows-default-release` | BLOCKED by existing artifact inventory drift after Cargo execution |
| `node app/scripts/test-warning-ratchet.mjs` on A | PASS — `warning ratchet self-tests ok`, including artifact rebaseline negatives |
| `--rebaseline-artifacts --base-ref A --head-ref B --configuration windows-default-release` | PASS — exact MSVC 14.44.35207, Cargo completed, artifact coverage `12`, changed files `1` |
| normal `windows-default-release` gate B→C | PASS — artifact coverage `12/12`, baseline/current warnings `83/0`, identity removals `67` |

The warning-ratchet run initialized the exact local toolchain and executed
`cargo check --workspace --all-targets --release --locked
--message-format=json` successfully. It then stopped fail-closed with:

```text
Cargo artifact coverage mismatch: missing=[] unexpected=[{"package":"protocol","target":"dj_link_v3_sender_contract","targetKinds":["test"],"crateTypes":["bin"]}]
```

The target was present in the current repository history, but was absent from
the checked-in warning inventory. The initial run therefore remained a real
fail-closed failure. The explicit A→B audit then verified the inventory-only
addition against the real Cargo output; it did not edit the inventory itself.

## Boundary

No native rebuild, executable launch, device access, physical output, signing,
or publication was performed for this checker-only checkpoint. The existing
native evidence remains governed by the earlier native QA records. This record
does not close the remaining external, hardware, macOS, or product completion
rows in the handoff ledger.
