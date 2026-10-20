# T27b: current-main integration — 2026-10-20

## Scope

Worktree: `C:\Users\janua\AppData\Local\SyndocalDev\wt-task27`; branch: `task27-audio-mapping`.

This integrates accepted T27 commit `a570f4870de346f93f3ad2fec12847d00a192bef` with main `7fe67b003a9741ed79550838bf9e79e412ce93a3`. The merge base is `e1d72e2306620d9c64454abf426ded50c1261f4d`. Main had advanced beyond the handoff's `3f426c4b` and also contained T29 exact-slot Preview and T30 Syphon. Those changes are preserved. The integration commit is recorded in the external `task27b-result.md` handoff.

Repository changes are confined to this worktree. No push, merge into main, product `syndocal.exe` launch, native/real-app acceptance harness, release build, stash, reset, or `.agents/` edit was performed. The permitted Cargo native unit-test binary is distinct from launching the product application.

## Conflict resolutions

| Files | Resolution |
| --- | --- |
| `check-ai0-source-coverage.mjs`, `check-frontend-command-routing.mjs`, native and IO `control_plane.rs` | Combine main's expanded inventories with T27's Audio delta; regenerate the native route SHA from the actual sorted route declarations. |
| `App.tsx` | Keep Audio request identity and runtime publication together with main's group patch dispatcher, modal/Learn guard, and T29 audition wiring. |
| `VideoGroupInspector.tsx`, `VideoPerformMatrix.tsx` | Preserve main's typed group mapping controls, T31 canvas, and T29 matrix behavior. These two files match the merged main versions exactly. |
| `uiLocalization.ts` | Retain both the Audio and group-canvas dictionaries, and main's canonical group/NDI patterns. Remove the interim Audio group label pattern. |
| Native `main.rs` | Keep T29's transport-locked exact-slot Preview snapshot and freshness checks. Capture the Audio texture publication in that tuple and pass it to the audio-aware Preview renderer. |

The merge had nine conflicted files, including the additional native Preview conflict caused by T29.

## One group target for every source

Audio group opacity now uses `VideoGroupOpacity` with a numeric, typed `video_group_id`. MIDI, OSC, DMX and Audio share the same control metadata and stable target identity. The intermediate `audio_only` flag, its hardware filters, the menu's MIDI-eligibility exception, and the `VideoParam`/string `group_id` label branch are removed. Neither group control component contains `data-control-map-unsupported`.

The protocol requires a positive JavaScript-safe group ID and rejects competing target identities. Other Audio actions cannot carry a video group ID. The frontend applies matching group-shape checks when creating assignments and evaluating saved mappings. Group Solo, Bypass, Blend and Enabled retain their existing hardware/Beat behavior and are not continuous Audio destinations.

Persistence keeps numeric group identity and stable Audio assignment IDs. Reassigning the same group keeps its ID; assigning another group keeps separate mappings. **No migration is needed:** T27's temporary string-group representation never shipped on main. Existing shipped projects retain their additive optional-field compatibility.

Deleted group, layer, clip or ISF targets remain in the saved mapping list, but a current-target filter excludes them from live Audio batches. This prevents one retained dangling assignment from rejecting unrelated valid controls. The existing queued-work guard reads the same filtered getter again at execution, so a deletion revokes already queued values. Native atomic target validation remains the final fence for races.

Continuous Audio still uses `set_audio_control_runtime`, not an authored group catalog patch. The native regression verifies rendered group modulation while authored UI/persistence opacity, epoch/revision/checkpoint, publication/history generations, and Undo/Redo remain unchanged.

## Rendering integration

T29 exact-slot Preview keeps its staged asset, trim/loop/reverse, draft isolation and stale-frame rejection semantics. Audio remains per-render context rather than part of the slot's persisted identity.

The new Syphon effect-context constructor now forwards `snapshot.live_audio_textures.as_deref()`. A native regression checks both the borrowed publication identity and the return to silence. Program, Preview, NDI, Spout, Follow and recording retain their Audio propagation. T31 group effects continue through the shared renderer.

## Final inventory

These are main's values plus T27's delta, verified against production declarations and executable inventory gates.

| Inventory | Main | T27 delta | Integrated |
| --- | ---: | ---: | ---: |
| MIDI message variants | 7 | 1 | 8 |
| Native Tauri routes | 587 | 1 | 588 |
| Frontend invoke manifest | 507 | 1 | 508 |
| MIDI/OSC/DMX operations | 239 | 1 | 240 |
| Engine sources | 285 | 1 | 286 |
| Remote sources | 116 | 0 | 116 |
| Keyboard sources | 33 | 0 | 33 |
| Legacy sources | 1,734 | 4 | 1,738 |
| Canonical sources | 1,767 | 4 | 1,771 |
| Canonical operations | 77 | 0 | 77 |
| Runtime route admission class | 178 | 1 | 179 |
| Unclassified local sources | 1,178 | 3 | 1,181 |

Native route SHA-256: `8658023cdba7321344369ccd4488c82cdc3f325dcdefb4e5f1c94c9601fafbbe`.

MIDI action variants are 65; OSC actions are 60. AI7's embedded frontend-count expectations and the MIDI feedback source-contract test were updated to reflect the integrated inventory and both Audio and renderer-owned feedback exclusions.

## Review

Three independent read-only reviews covered frontend mapping and tests, protocol/native authority, and video/Preview/Syphon propagation. The resulting fixes included the missing Syphon Audio context and filtering retained deleted targets. Follow-up review also strengthened the frontend regression to count forbidden authored writes explicitly and await actual master-only runtime publication after group deletion. Final reviews reported no remaining confirmed blocker.

## Validation

Rust commands used the worktree-local debug target via `C:\Users\janua\AppData\Local\SyndocalDev\tools\pcargo-here.cmd` with `CARGO_TARGET_DIR` unset. The wrapper verified MSVC 14.44. No compiler warnings were emitted. Frontend dependencies were installed first with `pnpm install --frozen-lockfile --prefer-offline`.

| Check | Final result |
| --- | --- |
| `check --workspace --all-targets` | Passed; zero warnings. |
| `test -p io --lib -- --test-threads=1` | 255 passed; 4 ignored; 0 failed. |
| `test -p protocol` | 297 passed; 0 ignored; 0 failed. |
| `test -p engine audio_control` | 7 passed; 0 ignored; 0 failed. |
| `test -p video isf` | 43 passed; 0 ignored; 0 failed. |
| `test -p syndocal --bin syndocal -- --test-threads=4` | 2,207 passed; 23 ignored; 0 failed. |
| `test -p video c1_audio_frame_only_apis_select_and_clear_audio_on_static_input` | 1 passed; 0 ignored; 0 failed. |
| `check-ai0-source-coverage.mjs` | Passed actual source inventories; 348 UI literal invokes; 6 fail-closed authority gates. |
| `check-ai7-adversarial-proof.mjs` | Passed. |
| `check-frontend-command-routing.mjs` | Passed: 137 renderer mutations, 31 server-authoritative mutations, 28 raw dispatches, 503 facade dispatches. |
| `check-tauri-admission-inventory.mjs` | Passed exact 588-route set/hash; inventory/runtime negative cases rejected. |
| `check:midi-learn` | Passed controller, routing, cross-transport and group regressions. |
| `check-group-control-mapping.mjs` | Passed identity, persistence, coalescing, catalog serialization and safety checks. |
| `check-audio-control-mapping.mjs` | 19 checks passed, including actual runtime publication after group deletion and zero authored writes. |
| `check:video-layer-groups` and `check-video-group-canvas.mjs` | Passed. |
| `check:perform-layout` | 6 EN/JA viewport cases and 5 runtime-focus/persistence cases passed. |
| `check-control-assign-menu-browser.mjs` | Passed 1280x720 and 375x640, EN/JA, 8 themes, keyboard/containment/contrast and the real canonical group control across MIDI/OSC/DMX/Audio. |
| `check:localization` | 4,179/4,179 static strings; zero unprotected user-data labels. |
| `tsc --noEmit` | Passed. |

The final browser runs were sequential on ports 5430/9430. An initial Perform pass used its dynamic Vite-port default; both browser gates were rerun with the dedicated ports explicitly set.

Initial verification exposed stale AI7 count and MIDI feedback expectations, then three native assertions for runtime routes and unclassified source counts. These were reconciled with the actual inventories, and the affected gates plus the full native suite were rerun successfully. The first native run already passed the new group-authority and Syphon-context tests and all six T29 slot-Preview tests.

Final evidence index: `target/t27b-validation/final-validation-summary.json`. The authoritative final Rust logs are `workspace-final.log` and `syndocal-full-final.log`; frontend rerun logs end in `-final.log`. Browser evidence is under `target/t27b-validation/perform-layout/` and `assign-menu/`.

The four IO ignored cases require operator-owned MIDI/loopback or RB fixtures. The native suite retains 23 ignored entries for operator fixtures, physical sessions, benchmarks, long recordings and subprocess-only helpers. No operator/hardware acceptance opt-in was enabled.

## Remaining acceptance boundary

Windows debug tests and headless browser evidence do not establish real-device Audio/MIDI/OSC/DMX behavior, live output latency or production multi-output load. macOS Syphon CI and a real Syphon client remain outside this Windows integration task. No product executable or native acceptance harness was launched.
