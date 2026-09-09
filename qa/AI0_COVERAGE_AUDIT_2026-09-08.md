# AI0 coverage audit — 2026-09-08

## Scope

This is a bounded audit of the existing source-inventory gates for
`AI0-COVERAGE-001`. It records current checker evidence without changing the
registry, increasing the covered-operation claim, or treating a Tauri/frontend
inventory as complete coverage of every mutation source.

- Source base for this revalidation: `f264ab07c0f89d69e5eee7c54b0a5f8ec476ef0f`
- Working tree before this revalidation: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing gates and results

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:completion-ledger` | PASS — 50 Open + 8 Deferred authority rows |
| `pnpm --dir app run check:q1-q4-ledger` | PASS — 32 Q1 rows, 29/29 Q0 domains, 10/10 source contracts, 58/58 Flow markers |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 516 exact native Tauri commands, SHA-256 `5120894f36feb82ac58fffd4db20739d80ae1b1a1c556fc95838a1708cbb8eea`, 18 negative fixtures rejected |
| `node app/scripts/check-frontend-command-routing.mjs` | PASS — renderer 133, server-authoritative 31, raw dispatch 28, facade dispatch 464 |
| `cargo test -p protocol --release --locked control_plane_registry_v2 -- --test-threads=1` | PASS — 13 passed, 0 failed; 203 filtered |

The checks use the current source and preserve strict unknown-command,
duplicate, frozen-count, frozen-hash, malformed-syntax, and routing assertions.
No assertion was removed or weakened.

## What this proves

The current registered Tauri inventory is synchronized with its frozen native
admission policy, and the audited frontend command routes resolve through the
existing authority/facade boundaries. Ledger and Q1–Q4 mirror structure remain
machine-valid.

## Remaining boundary

`AI0-COVERAGE-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. The full AI0 requirement still needs a
mechanical, fail-closed inventory and classification of every relevant Tauri,
Engine, Remote, MIDI/OSC, shortcut, audio-analysis/BPM, native-output-window,
and UI mutation source. The current checks do not by themselves prove that
all of those sources are covered or that an unclassified mutation cannot
reach a physical/output path. Downstream AI1–AI8 completion and external,
hardware, Mac real-device, signing, publication, and product-wide acceptance
remain unclaimed.

This checkpoint contains no runtime change, no physical-output action, and no
new feature implementation.

## Current-main software revalidation

The bounded gates were repeated on current `main` at source HEAD
`818235abe6c17f5571bd8a4ee2ad2132564e1ed0`. No product source or inventory
count was changed.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — 50 Open + 8 Deferred authority rows |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — 32 Q1 rows, 29/29 Q0 domains, 10/10 source contracts, 58/58 Flow markers |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 516 exact commands, SHA-256 `5120894f36feb82ac58fffd4db20739d80ae1b1a1c556fc95838a1708cbb8eea`, 18 negative fixtures rejected |
| `node app/scripts/check-frontend-command-routing.mjs` | PASS — renderer 133, server-authoritative 31, raw 28, facade 464 |
| `cargo test -p protocol --release --locked control_plane_registry_v2 -- --test-threads=1` | PASS — 13 passed, 0 failed; 204 filtered |

The protocol test used the exact MSVC 14.44.35207 x64 linker pin and
`where.exe link.exe` first-match check. The current-main revalidation does not
expand the coverage claim: AI0 still requires a fail-closed inventory and
classification of every relevant non-Tauri mutation source.
