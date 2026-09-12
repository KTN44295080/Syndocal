# AI3 durable recovery checkpoint — 2026-09-13

## Scope

This checkpoint closes the supported software marker
`AI3-DURABLE-RECOVERY-001`: crash-safe durable terminal state for the output
lease boundary. It does not close native ingress, physical project replacement
or re-Arm, external-client acceptance, hardware acknowledgement, or the
product-wide AI3 exit.

## Existing implementation and current verification

`app/src-tauri/src/main.rs` contains the versioned
`OutputLeaseDurableReceiptJournal`. It persists `Pending` before an
irreversible callback, records terminal receipts atomically, reloads exact
replay identity, rejects corrupt/oversized/unknown or unwritable state, and
starts live output authority empty after restart. A pre-crash lease is not
reclaimed from process-local state.

The current source was verified at `cfc65bcf34288d56615090d0d26c8b5644611311`
with the documented Windows Build Tools `14.44.35207` environment. The exact
x64 linker was pinned at:

```text
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
```

`where.exe link.exe` returned that path first. Focused release tests passed:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 durable -- --test-threads=1
test result: ok. 23 passed; 0 failed; 0 ignored

cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 dsf2026 -- --test-threads=1
test result: ok. 6 passed; 0 failed; 0 ignored

cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 managed_exact_both -- --test-threads=1
test result: ok. 13 passed; 0 failed; 0 ignored
```

The existing current-main process-boundary drill is recorded in
`qa/AI3_DURABLE_TERMINAL_RECOVERY_2026-09-08.md`; it observed restart as
`unknown`, rejected changed-shape reuse as `request_conflict`, retained
Standby with lighting/video denied, and performed zero physical-output
operations. No physical output was enabled in this checkpoint.

## Result and boundary

The Flow row is now checked and the completion ledger represents this marker as
`Complete`. The companion AI3 native ingress, physical re-Arm, and durable
external acceptance markers remain Open because their required native,
external-client, and physical evidence is not present.
