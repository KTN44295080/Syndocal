# AI1 query/event audit — 2026-09-08

## Scope

This is a bounded audit of the existing AI1 query, cursor, and observation
event implementation. It records current source evidence without creating a
new API, widening the supported external-client claim, or marking the full
AI1 tranche complete.

- Source base for this revalidation: `1441f6379ca4b67c09c5b22bf735aeb4bab8b77e`
- Working tree before this revalidation: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing implementation checked

`app/src-tauri/src/control_plane_query.rs` already provides the bounded schema
catalog and capabilities query, typed query errors, owner-bound single-use
opaque cursors, expiry and process capacity limits, generation/fence-bound
snapshot paging, serialized capture, and retained observation-event paging.
Unknown/stale/altered fences and retention loss return explicit fail-closed
results; a retention gap carries `resnapshot_required` instead of silently
continuing from an invalid base.

## Focused verification

The command ran from the repository root after
`vcvars64.bat -vcvars_ver=14.44`, with the exact Build Tools MSVC
`14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` and returned first by
`where.exe link.exe`.

| Command | Result |
| --- | --- |
| `cargo test -p syndocal --release --locked control_plane_query -- --test-threads=1` | 14 passed, 0 failed |

The 14 tests cover catalog bounds/order, cursor owner binding, tamper/replay,
expiry/capacity/retirement, stale and future fences, page bounds, serialized
capture ordering, project-replacement redaction, exact query-source generation,
event backlog convergence, and explicit retention-gap resnapshot behavior.
No app process, physical output, device, or external client was started.

## Remaining boundary

`AI1-SCHEMAS-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. This checkpoint does not prove the full
AI1 requirement across every canonical query/event family, generated schema
parity, pagination payload matrix, external Remote/MCP/JSON-RPC adapters, or
real client reconnect/recovery. AI0 source coverage, AI2 authored bridge,
AI3-wide safety, external/hardware/Mac real-device, signing, publication, and
product-wide acceptance remain unclaimed.

No assertion was weakened and no runtime or physical-output behavior changed.

## Current-main software revalidation

The focused query/event tests were repeated on current `main` at source HEAD
`818235abe6c17f5571bd8a4ee2ad2132564e1ed0`. No product source or query/event
schema was changed.

```text
cargo test -p syndocal --release --locked control_plane_query -- --test-threads=1
test result: ok. 14 passed; 0 failed; 0 ignored; 1804 filtered out
```

The run used the exact MSVC 14.44.35207 x64 linker pin and the
`where.exe link.exe` first-match check. This is current-main software evidence
for the existing bounded query/event vertical only; AI1 remains Open for full
canonical-family parity, external adapters, and real-client reconnect or
recovery acceptance.
