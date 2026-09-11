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

## Current-main query/event revalidation — 2026-09-10

The bounded query/event suite was rerun against current `main` at source HEAD
`50a0ed98e3f501d1676cd1809a8921e3fad76489`. Product source and the query/event
wire were unchanged by this QA-only checkpoint. The documented MSVC
14.44.35207 absolute linker was pinned and returned first by `where.exe link.exe`.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 control_plane_query -- --test-threads=1
test result: ok. 14 passed; 0 failed; 0 ignored; 1804 filtered out
```

The run covers owner-bound cursor/tamper/replay rejection, expiry and capacity,
stale/future fences, strict page bounds, serialized capture ordering,
project-replacement redaction, event backlog convergence, and explicit
retention-gap resnapshot. `AI1-SCHEMAS-001` remains Open for complete canonical
family parity, generated external schemas, external adapters, and real-client
reconnect/recovery. The real-file thumbnail recovery trial was not rerun.

## Current-main query/event revalidation — 2026-09-11

The bounded AI1 suite was rerun after the current output and snapshot
checkpoints at source HEAD `3e51a8ae4c1290d3db34ac0b6a3389ef417fe4b9`.
Evidence is preserved under `target/qa/ai1-current-main-20260911-01/`.
The exact MSVC 14.44.35207 x64 linker was initialized with
`vcvars64.bat -vcvars_ver=14.44` and returned first by `where.exe link.exe`.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 control_plane_query -- --test-threads=1
test result: ok. 14 passed; 0 failed; 0 ignored
```

The run retained the existing coverage for catalog bounds/order,
owner-bound single-use cursors, tamper/replay and expiry/capacity rejection,
stale/future fence rejection, strict page bounds, serialized capture ordering,
project-replacement redaction, event backlog convergence, exact generation
source selection, and explicit retention-gap resnapshot.

`AI1-SCHEMAS-001` remains Open. This is bounded local query/event evidence;
it does not prove every canonical family, generated external schema parity,
Remote/MCP/JSON-RPC adapters, or real-client reconnect/recovery. Physical,
Mac, signing, publication, and product-wide acceptance remain unclaimed.
