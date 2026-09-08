# AI3 durable terminal recovery audit — 2026-09-08

## Scope

This is a bounded audit checkpoint for `AI3-DURABLE-RECOVERY-001`. It records
the current implementation and focused regression evidence without creating a
second durable journal or claiming the complete AI3, physical re-arm, native
ingress, or hardware acceptance gates.

- Source base: `27a9360b755a7f5225f4e8c8503a2b89b4f8df3d`
- Working tree before this document: clean; `main` matched `origin/main`
- Owner: this checkpoint owns only this QA record
- Product source changes: none
- Previous real-file thumbnail missing → UI Retry → recovery evidence: not rerun

## Existing implementation checked

The current main source already contains `OutputLeaseDurableReceiptJournal` in
`app/src-tauri/src/main.rs`. The bounded journal loads and validates a versioned
machine-local file, persists `Pending` before an irreversible callback, records
terminal receipt evidence atomically with the fixed DSF2026 public terminal,
retains exact replay identity, and starts the live output authority empty after
restart. It does not reconstruct the old process's lease authority.

The checked fail-closed boundaries include:

- same-request terminal replay after journal reload without a second callback;
- principal/window binding and exact request-shape conflict rejection;
- Pending/InDoubt refusal before a second physical callback;
- corrupt, oversized, unknown-field, invalid-owner/resource/change, and
  unwritable journal rejection;
- safe-abort replay guards, terminal eviction, origin retention/GC, and legacy
  replay-guard migration;
- DSF2026 one-shot consumption, explicit no-send reconciliation, and atomic
  terminal-persist-failure recovery;
- managed exact-Both renewal/relinquish/expiry identity, scope, generation,
  and route rejection.

## Focused verification

All commands ran from the repository root after `vcvars64.bat -vcvars_ver=14.44`.
The environment pinned
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to the Build Tools
`14.44.35207` x64 linker, and `where.exe link.exe` returned that exact path
first.

| Command/filter | Result |
| --- | --- |
| `cargo test -p syndocal --release --locked durable -- --test-threads=1` | 23 passed, 0 failed |
| `cargo test -p syndocal --release --locked dsf2026 -- --test-threads=1` | 6 passed, 0 failed |
| `cargo test -p syndocal --release --locked managed_exact_both -- --test-threads=1` | 13 passed, 0 failed |
| `git diff --check` | PASS for the checkpoint diff |

The current exact Windows release artifact remains:

- `target/release/syndocal.exe`
- version `1.2.0-alpha.69`
- size `64,700,928` bytes
- SHA-256 `CC984A75AB399E80C373C6B9133C5FFA6661ABA6878B4D17EF606A063F6523B3`
- native build/one-window launch evidence: `target/qa/snapshot-live-publication-20260908-01/`

That launch evidence proves only the general native artifact launch boundary:
one responsive, maximized Syndocal window, exact executable cleanup, and zero
physical-output operations. It is not dangerous-action, external-client,
physical-resource, or venue acceptance.

## Remaining boundary

`AI3-DURABLE-RECOVERY-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`; this checkpoint closes only the bounded
software journal audit/evidence slice. The full AI3 and product gates still do
not claim:

- all MIDI/OSC/Remote ingress routes are canonical and generation-fenced;
- complete project replacement retirement and explicit physical re-Arm;
- real native external-client reply-loss/restart acceptance;
- physical Art-Net/NDI/Spout/display/ASIO output or hardware ACK;
- five-display, venue-soak, Mac real-device, signing, publication, or product-
  wide completion.

No assertion was weakened and no physical output was enabled for this audit.
