# H5 Video master checkpoint — 2026-09-14

## Checkpoint identity

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `23c5cb43` (Lighting master / group submaster checkpoint)
- Scope: current-source H5 Control implementation for the lease-bound Video master opacity operation, including protocol, native dispatch, frontend admission, viewport fixture behavior, and the required Windows native build/process smoke.
- This checkpoint does not close `UI-H5-CONTROL-001` or claim product-wide completion. The remaining H5 workflows and the physical, native interaction, accessibility, external, venue, signing, and publication gates remain separate.
- No product-version bump was required; this is an internal checkpoint.

## Implementation and repair

1. `crates/protocol/src/control_plane_command.rs` appends schema-v10 `SetVideoMaster` with a typed `Video`/`Both` role, opaque output lease authority, and a 0..=1000 milliunit bound. It uses append-only canonical discriminant 23 and a dedicated operation id, while preserving the schema-v9 lighting/group wire identities.
2. `app/src-tauri/src/main.rs` routes the operation through the existing external-admission, lease coordinator, output-transition lock, checkpoint/fence revalidation, and exact terminal receipt path. The native dispatch emits only the bounded `EngineCommand::SetVideoMasterOpacity` after the final lease candidate is committed. The retired raw `set_video_master_opacity` route remains fail-closed.
3. `app/src-tauri/src/control_plane_runtime.rs` and the canonical registry/inventory bind the operation to the Video/Both resource, exact receipt outcome, local OutputControl policy, audit/rate/consent categories, and schema-v10 admission.
4. `app/src/outputControlController.ts`, `app/src/App.tsx`, and `app/src/createVideoRuntimeController.ts` select exactly one active held Video-capable lease, validate the typed receipt and unchanged output fence, expose the canonical invoke helper, and update the viewport fixture's video master state instead of returning the previous no-op message.
5. The Tauri invoke manifest, frontend inventory, source-coverage counts, routing checker, adversarial proof, and OutputControl runtime checker were updated together. Existing Lighting and group canonical bytes and operations remain unchanged.

## Verification evidence

All commands below were run on the current checkout and their exit status was checked.

| Area | Command/result |
| --- | --- |
| Protocol wire contract | `cargo test -p protocol --locked control_plane -- --nocapture --test-threads=1` — PASS, 57 passed, 0 failed. This includes schema-current validation, Video master canonical-byte discriminant, JSON round-trip, role/value/lease rejection, and existing control-plane contracts. |
| Native control-plane inventory | Fixed MSVC 14.44.35207 x64 linker; `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 control_plane::tests -- --nocapture --test-threads=1` — PASS, 31 passed, 0 failed. The intentional poison-registry diagnostic is caught by the authored test; the test process exited successfully. |
| Release static suite | `pnpm.cmd --dir app run check:release:static` — PASS. Completion ledger remained 23 Open + 8 Deferred + 27 Complete; Q1-Q4 mirror was structurally valid; Tauri inventory was 537 commands with 18 negative fixtures rejected; frontend inventory was 478; OutputControl v10 and all included project/media/audio/video/input safety contracts passed. |
| Frontend and routing | `pnpm.cmd --dir app run check:touch` — PASS, 10 Touch/browser records across all five configured viewports and composed surfaces; no duplicate controls, overflow, or scroll failures. `pnpm.cmd --dir app run build` — PASS, 357 modules. Vite emitted the existing large-chunk advisory; no first-party compiler warning or failure was introduced. |
| Windows native build | `vcvars64.bat -vcvars_ver=14.44` with the exact Build Tools `14.44.35207` linker, then `pnpm.cmd --dir app tauri build --no-bundle` — PASS; generated `target/release/syndocal.exe` in 3m43s. |
| Native process smoke | Exact executable `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe`; before count 0, PID `41224`, exactly one matching process, title `Syndocal`, `Responding=True`, window handle `461716`; stopped only that exact process and confirmed remaining exact count 0. |
| Native artifact identity | SHA-256: `A92A9DD0C4055DEF5ECD9A1E819B1CD5024D181710A8D43CC02495FC735E90D6`. This is an unsigned current-source build artifact, not a signed or published release. |

## Repository branch hygiene

The earlier branch cleanup deleted 31 local branches already merged into the current branch together with their matching `origin/*` refs. The current branch, `main`, unmerged branches, and worktree-backed branches were retained. No unmerged or active worktree branch was deleted.

## Evidence boundary and remaining work

The software path now has a canonical lease/fence-bound Video master operation for the Video/Both roles and a working viewport fixture path. This proves neither physical video output nor native button-by-button interaction. It also does not establish physical DMX/Art-Net/sACN/NDI/Spout/display output, external clients, real two-machine operation, venue/soak, accessibility, crash/restart replay restoration, signing, publication, or product-wide completion.

The Flow ledger remains **23 Open + 8 Deferred + 27 Complete = 58 markers**. `UI-H5-CONTROL-001` remains Open because the broader Control Cue/Clip/Take/Transition/Blackout/Arm/Take Over/recording/diagnostic and recovery workflows, plus their native/external acceptance, are not proven by this bounded Video master checkpoint.

Next action: continue the next bounded H5 item only after this checkpoint is pushed; preserve the same fail-closed OutputControl authority and add separate evidence for each independently proven slice.
