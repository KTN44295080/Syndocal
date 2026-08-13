# Media Asset T1 A7 focused-gate evidence

Date: 2026-08-13 JST

Scope: current checkout after the Media Asset A1-A6 implementation commits and the shared Edit/Video UI correction. This record proves the A7 automated gate only. It does not prove A8 native workflows, non-Windows execution, physical hardware, clean-machine distribution, or whole-product completion.

## Source checkpoint

- Branch: `codex/syndocal-v1.0`
- A1 backend terminal transaction: `aad9172866206963ea2ef5e49795bc95ec3a2091`
- Preview staging authority fence: `682b9087c5e23b8f3123e1fce29027edfda6919b`
- A2 frontend authoritative wiring: `14eeeb212ed07a6dfadc70e9d8efde5b39333ee4`
- A3 legacy IPC compatibility: `4173e35765a6f256ba40691e8883d70707ffeb4f`
- A6 complete Bootstrap rollback proof: `bb6aef763cb3545b804d7ca1ae5877e93b273c7b`
- A4 cross-platform coherence implementation: `8cb0459496dbe0bd150090ebf41e77b66f228d66`
- A5 availability/reaper hardening: `68a983d1a80c6327a88a004314bee04fe2bf2afd`
- A6 six-command idempotency proof: `bdb700813eaedce9f0e94a14cf5ef84b6cdb61b3`
- Deterministic native first-hash pause seam: `dd91db8bb7bb2b9424308afe674bffa02029c2fc`
- Media Library/native readiness UI: `10fdf0594ea46c01de710b578a7b7723e1233766`
- Shared Edit/Video grid correction: `be9c456b8a4fd8ffe7f53e9342c9d9bcec66d199`
- Localization correction discovered by this gate: `2f77981dd178ff4433483e2cd453746fd2035510`

The checkout also contains shared/user-owned dirty work outside this focused sequence. It was preserved and was not folded into the localization commit.

## Ordered A7 results

All commands were run from `C:\Users\kouty\Documents\KDMX` in the roadmap order.

| Step | Command | Result | Selected proof |
| ---: | --- | --- | --- |
| 1 | `cargo fmt --check --package protocol --package engine --package syndocal` | PASS | exit 0 |
| 2 | `cargo test -p protocol media_asset --locked -- --nocapture` | PASS | 4 passed, 0 failed, 56 filtered |
| 3 | `cargo test -p engine media_asset --locked -- --nocapture` | PASS | 9 passed, 0 failed, 658 filtered |
| 4 | `cargo test -p syndocal media_asset_operation --locked -- --nocapture` | PASS | 15 passed, 0 failed, 604 filtered |
| 5 | `cargo test -p syndocal media_asset_hash --locked -- --nocapture` | PASS | 2 passed, 0 failed, 617 filtered |
| 6 | `cargo test -p syndocal media_asset_finalize --locked -- --nocapture` | PASS | 2 passed, 0 failed, 617 filtered |
| 7 | `cargo test -p syndocal media_asset_availability --locked -- --nocapture` | PASS | 7 passed, 0 failed, 612 filtered |
| 8 | `cargo test -p syndocal media_asset_relink_ --locked -- --nocapture` | PASS | 6 passed, 0 failed, 613 filtered |
| 9 | `cargo test -p syndocal media_asset_authoritative --locked -- --nocapture` | PASS | 26 passed, 0 failed, 593 filtered |
| 10 | `cargo test -p syndocal media_asset_commit_ --locked -- --nocapture` | PASS | 9 passed, 0 failed, 610 filtered |
| 11 | `cargo check -p syndocal --locked` | PASS | exit 0 |
| 12 | `pnpm --dir app exec tsc --noEmit` | PASS | exit 0 |
| 13 | `node app/scripts/check-media-asset-authority.mjs` | PASS | phase order, reply-loss/query, cancel CAS, E/R/H continuity, operator classification, paired apply, empty-catalog normalization |
| 14 | `node app/scripts/check-vj-first-run.mjs` | PASS | safe first-run workflow and independent Preview transport |
| 15 | `node app/scripts/check-vj-media-import-access.mjs` | PASS | empty/populated/mixer import predicates, names, scoped CSS, bank/page reachability |
| 16 | `node app/scripts/check-backend-operator-contract.mjs` | PASS | 405 backend commands, 321 literal frontend calls, 163 transactional mutations |
| 17 | `pnpm --dir app run check:localization` | PASS after correction | first run exposed 3166/3174; focused eight-label fix produced 3174/3174 and zero bare user-data labels |
| 18 | scoped `git diff --check` for Protocol, Engine, Syndocal, Media frontend, harnesses, and localization | PASS | exit 0; repository LF-to-CRLF notice only |

No filter selected zero tests.

## Expected diagnostic output

- The Engine allocator maximum test intentionally catches the allocator-exhaustion panic; the test itself passed.
- The authoritative post-publication bookkeeping test intentionally injects and catches a receipt-bookkeeping panic; the test itself passed.
- Existing dead-code warnings remain in Engine and Syndocal. No new warning was treated as proof of failure.
- The VJ media-import static gate explicitly does not replace browser-rendered or native reachability evidence.

## Native build checkpoint after A7

The exact checkout executable was resolved and the one matching running process was terminated before linking.

- Command: `pnpm --dir app tauri build --no-bundle`
- Result: PASS
- Frontend modules: 248 transformed
- Release executable: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Executable SHA-256: `41FE507275C21916F36894A4D4E5DF4D9CEF537E5223A778302B83CCE93FAE07`

This build starts A8 but does not complete it.

## A8 status and non-claims

A8 remains in progress. The following are not yet accepted for this exact executable:

- empty picker cancellation;
- populated catalog-only import;
- 12-file catalog-only import;
- one File layer and one Still layer;
- ordinary mixed success/failure and Bootstrap fail-fast;
- cancel during the first hash;
- project replacement during prepare;
- Missing, HashMismatch, and Relink of all references;
- first-run Bootstrap success;
- restart/load without source-file touch;
- explicit Save and reload equivalence.

The broader product roadmap remains incomplete. In particular, generic Begin reply-loss, PATCH/Stage authority, output replacement fencing, Clip Slots, derived media data, Audio/recording, ShowClock, migration/security/performance/hardware, and release/distribution gates remain open.
