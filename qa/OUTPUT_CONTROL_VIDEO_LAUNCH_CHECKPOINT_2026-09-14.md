# H5 Video Clip Launch checkpoint — 2026-09-14

## Checkpoint identity

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `0b61c7a5` (lease-bound Video Take checkpoint)
- Scope: current-source H5 Control implementation for the direct Video Clip Launch operation, including protocol, shared video launch core, native dispatch, frontend admission, static contracts, rendered Touch viewport, and the required Windows native build/process smoke.
- This checkpoint does not close `UI-H5-CONTROL-001` or claim product-wide completion. Native button-by-button interaction, physical video output, external clients, venue/soak, accessibility, signing, publication, and the remaining Flow gates stay separate.
- No product-version bump was required; this is an internal checkpoint.

## Implementation

1. `crates/protocol/src/control_plane_command.rs` appends schema-v12 `LaunchVideoClip` with a typed `Video`/`Both` role, non-zero safe JavaScript `layer_id`, a `0..=600,000` ms fade bound, opaque output lease authority, and append-only canonical discriminant 25. Existing wire identities remain unchanged.
2. `app/src/outputControlController.ts` selects exactly one active held Video-capable lease, validates layer/fade bounds, maps the action to `launch_video_clip_output_control_v2`, and retains the exact lease/fence/receipt checks. `app/src/createVideoRuntimeController.ts` routes the production Clip Launch path through this canonical helper.
3. `app/src-tauri/src/main.rs` and `control_plane_runtime.rs` dispatch the operation through the external-admission, project-coordinator, output-transition, lease, and exact terminal receipt fences. The native callback calls a shared `launch_video_clip_core`; the legacy raw route remains a compatibility seam and does not duplicate the launch mutation.
4. Tauri/frontend inventories, canonical registry schema validation, operation counts, route SHA, adversarial/source-coverage contracts, and the OutputControl runtime contract were updated together: Tauri 539, frontend 480, legacy registry 1621, canonical source inventory 1654, canonical operations 52, OutputControl operations 25.

## Verification evidence

All commands below were run on the current checkout and their exit status was checked.

| Area | Command/result |
| --- | --- |
| Protocol wire contract | `cargo test -p protocol --locked control_plane -- --nocapture --test-threads=1` — PASS, 57 passed, 0 failed. Includes v12 schema validation, Clip Launch canonical bytes/serde, layer/fade/role/lease rejection, and existing control-plane contracts. |
| Native control-plane inventory | MSVC `14.44.35207` x64 linker; `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 control_plane::tests -- --nocapture --test-threads=1` — PASS, 31 passed, 0 failed. The authored poison-registry diagnostic is caught by its test and the process exits successfully. |
| Release static suite | `pnpm.cmd --dir app run check:release:static` — PASS. Completion ledger is 23 Open + 8 Deferred + 27 Complete = 58; Q1–Q4 mirror has 57 evidence records; Tauri inventory is 539 commands with SHA-256 `a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab` and 18 negative fixtures rejected; frontend inventory is 480. |
| OutputControl and frontend | `pnpm.cmd --dir app run check:output-control-runtime` — PASS, v12 lease-bound Video Take/Clip Launch/master and Lighting/group contracts. The no-bundle build also passed TypeScript and Vite production build for 357 modules; only the existing Vite large-chunk advisory was emitted. |
| Rendered Touch viewport | `SYNDOCAL_VIEWPORT_TRACE=1 pnpm.cmd --dir app run check:touch` — PASS, 12 records across 1920×1080, 1920×1032, 2048×1152, 1366×768, and 1280×720, including composed surfaces; no duplicate controls, overflow, or scroll failures. |
| Windows native build | Pinned `vcvars64.bat -vcvars_ver=14.44` with the exact Build Tools `14.44.35207` linker, then `pnpm.cmd --dir app tauri build --no-bundle` — PASS; generated `target/release/syndocal.exe`. |
| Native process smoke | Exact executable `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe`; before count 0, exactly one matching process (PID `42688`), title `Syndocal`, `Responding=True`, window handle `462678`; stopped only that exact process and confirmed remaining exact count 0. |
| Native artifact identity | SHA-256: `1BCD3DF94A8260CA31FF78DD71DAFF5CDB3A04CE49313E1BB43FB5DED3D71DA7`. This is an unsigned current-source build artifact, not a signed or published release. |

## Repository branch hygiene

The earlier cleanup deleted 31 local branches already merged into the current branch together with their matching `origin/*` refs. The current branch, `main`, unmerged branches, and worktree-backed branches were retained. No unmerged or active worktree branch was deleted.

## Evidence boundary and remaining work

This checkpoint proves the current-source canonical Control UI Video Clip Launch software path, shared launch core, lease/fence admission, static contracts, rendered Touch geometry, and Windows native build/process smoke. It does not prove native button-by-button interaction, physical HDMI/video output, NDI/Spout/display reception, real two-machine operation, crash/restart replay restoration, accessibility, venue/soak, signing, publication, or product-wide completion.

The Flow ledger remains **23 Open + 8 Deferred + 27 Complete = 58 markers**. `UI-H5-CONTROL-001` remains Open because the broader Cue/Clip/Take/Transition/Blackout/Arm/Take Over/recording/diagnostic and recovery workflows, plus native/external acceptance, are not all proven by this slice.

Next action: continue the next independently bounded H5 control slice only after this checkpoint is pushed; preserve the same fail-closed OutputControl authority and add separate evidence for each proven boundary.
