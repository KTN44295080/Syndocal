# AI3 durable terminal recovery audit — 2026-09-08

## Scope

This is a bounded audit checkpoint for `AI3-DURABLE-RECOVERY-001`. It records
the current implementation and focused regression evidence without creating a
second durable journal or claiming the complete AI3, physical re-arm, native
ingress, or hardware acceptance gates.

- Source base for this revalidation: `465ed1b5923e2fdf1550b79fd5db88271127d26a`
- Working tree before this revalidation: clean; `main` matched `origin/main`
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
- size `64,699,904` bytes
- SHA-256 `F32CE630FD7E31D3857A0F66948C05D712073A326C0A5497A14660753B025905`
- native build/one-window launch evidence: `target/qa/native-final-validation-20260909-09/native-final-validation.json`

The current native probe matched this exact executable hash and verified one
responsive maximized window, Standby ownership, snapshot shape, invalid
thumbnail IPC rejection, zero physical-output operations, exact process exit,
and no remaining debug listener. The probe does not claim a dangerous-action,
external-client, physical-resource, or venue acceptance.

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

## Current-main software revalidation

The focused software evidence was repeated on current `main` at source HEAD
`9735873ae401c84020e062d2ef53066dc6ad0abf`. No product source was changed;
the existing journal and output-lease implementation was tested as-is. The
same Windows procedure initialized Build Tools 14.44.35207, pinned the
absolute x64 linker, and verified it was first in `where.exe link.exe`.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 durable -- --test-threads=1
test result: ok. 23 passed; 0 failed; 0 ignored

cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 dsf2026 -- --test-threads=1
test result: ok. 6 passed; 0 failed; 0 ignored

cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 managed_exact_both -- --test-threads=1
test result: ok. 13 passed; 0 failed; 0 ignored
```

This revalidation strengthens the current-main software journal evidence only.
It does not close `AI3-DURABLE-RECOVERY-001`, because a real crash/restart
process drill, external-client reply-loss acceptance, physical output
retirement/re-Arm, and hardware acknowledgement remain outside this local
test boundary.

## Current-main follow-up — `c26201bc1cc4d46a2c00b52446def7ef57e14b5a`

The three focused software slices were rerun after the QA-only main
checkpoint. Product source was unchanged. The same exact Windows native
procedure was applied before all Cargo commands.

```text
cargo test -p syndocal --release --locked durable -- --test-threads=1
test result: ok. 23 passed; 0 failed; 0 ignored; 1795 filtered out

cargo test -p syndocal --release --locked dsf2026 -- --test-threads=1
test result: ok. 6 passed; 0 failed; 0 ignored; 1812 filtered out

cargo test -p syndocal --release --locked managed_exact_both -- --test-threads=1
test result: ok. 13 passed; 0 failed; 0 ignored; 1805 filtered out
```

This current-main follow-up remains bounded software evidence. It does not
close the ledger item or claim real crash/restart process acceptance,
external-client reply-loss, physical output retirement/re-Arm, or hardware
acknowledgement.
