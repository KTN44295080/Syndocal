# Video full-gate software audit — 2026-09-08

## Scope

This is a bounded audit of the existing Windows video/Clip Slot/transition and
Timeline Follow software contracts. It does not add a video feature, reopen an
old candidate branch, or claim native renderer/GPU/display or physical output
acceptance.

- Source base: `e9db716504eb8d86cae9ef31c5160686903d36a0`
- Working tree before this document: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing implementation checked

The current Engine path already publishes and rolls back the complete video
state for the bounded full-gate scenario. The C3 transition bus is typed,
reversible, conflict-safe, and published. Clip Take queue sampling stays
separate from root interpolation, and Follow admission/jump termination stops
before its boundary. Existing frontend contracts retain the accepted Clip Slot
model and explicit Follow hold modes.

## Focused verification

The Rust commands ran after `vcvars64.bat -vcvars_ver=14.44`, with the exact
Build Tools MSVC `14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` and returned first by
`where.exe link.exe`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — B4 focused model/browser contract |
| `node app/scripts/check-timeline-follow-runtime.mjs` | PASS — stale E/G, E/R/H abort receipt, visibility/focus/target contracts |
| `node app/scripts/check-timeline-follow-hold-ui.mjs` | PASS — immediate/hold and one-measure wait-for-Pedal-1 static model |
| `cargo test -p engine --release --locked video_full_gate_engine_path -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked video_transition_bus_c3_is_typed -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked video_sample_clip_take_queue -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked video_sample_follow_admission -- --test-threads=1` | PASS — 1 passed |

`node app/scripts/check-edit-video-fx.mjs` was attempted but could not start
because this PC has no Playwright Chromium executable at the configured local
cache path. This is an environment prerequisite failure, not a product pass;
the browser gate remains open and no browser dependency was downloaded or
assertion weakened. No app process, physical output, device, or external client
was started by this audit.

## Remaining boundary

`VIDEO-FULL-GATE-001`, `VIDEO-C2-C4-001`, and `TIMELINE-FOLLOW-001` remain
`Open` in `qa/SYNDOCAL_COMPLETION_LEDGER.json`. This checkpoint does not prove
the complete C2/C4 reintegration, the unavailable browser/rendered UI gate,
native renderer/GPU/display behavior, 4K/three-output presentation, real show
media, or physical output. Timeline persistence, live sources, ASIO/NDI/DMX
hardware, Mac real-device, signing, publication, and product-wide acceptance
remain unclaimed.

No assertion was weakened and no runtime or physical-output behavior changed.

## Current-main software revalidation

The bounded software checks were repeated on current `main` at source HEAD
`9618c2f2a767f64f2b6b2a0d72b3b46b4cae6fe4`. No video feature, transition
contract, browser dependency, or physical-output path was changed.

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — B4 focused model/browser-contract gate |
| `node app/scripts/check-timeline-follow-runtime.mjs` | PASS — stale E/G, E/R/H, visibility/focus/target contracts |
| `node app/scripts/check-timeline-follow-hold-ui.mjs` | PASS — explicit normalized Immediate/Hold modes |
| `cargo test -p engine --release --locked video_full_gate_engine_path -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p engine --release --locked video_transition_bus_c3_is_typed -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p engine --release --locked video_sample_clip_take_queue -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p engine --release --locked video_sample_follow_admission -- --test-threads=1` | PASS — 1 passed, 0 failed |

The native Cargo commands used the exact MSVC 14.44.35207 x64 linker pin and
`where.exe link.exe` first-match check. The browser gate was attempted without
downloading dependencies:

```text
node app/scripts/check-edit-video-fx.mjs
exit 1
browserType.launch: Executable doesn't exist at
C:\Users\janua\AppData\Local\ms-playwright\chromium_headless_shell-1234\chrome-headless-shell.exe
```

No browser assertion was counted. This is an environment prerequisite
failure, not a product pass or a source assertion change. The Video/C2/C4 and
Timeline Follow parent rows remain Open for the unavailable browser/native
renderer/physical-output boundaries.

## Current-main rendered UI revalidation — 2026-09-10

The previously unavailable local browser prerequisite was supplied in the
user-level Playwright cache only; no repository dependency or product source
was changed. The browser plugin was unavailable in this environment, so the
documented regular-Playwright fallback was used. This closes the local
rendered UI prerequisite reported above, but it does not close the broader
Video/C2/C4 completion rows.

Source HEAD: `f40aa2fbc0dfe7b991f0f238ed849b99b5be29d6`.

Command:

```text
node app/scripts/check-edit-video-fx.mjs
```

Result: PASS at both required viewports.

| Viewport | Interaction/result | Page errors | Geometry |
| --- | --- | --- | --- |
| 1920x1080 | mounted `0/1/0`; selected-layer callback `2`; `add_builtin_video_isf_effect` dispatched | 0 | 64 controls; outer overflow `0/0`; inspector contained |
| 1280x720 | mounted `0/1/0`; selected-layer callback `2`; `add_builtin_video_isf_effect` dispatched | 0 | 64 controls; outer overflow `0/0`; inspector contained |

The result JSON records `shortControls: []`, `clippedControls: []`, and a
successful effect dispatch for each viewport. Screenshot evidence is retained
under `target/qa/edit-video-fx-20260905/`:

- `overview-1920x1080.png`, `controls-1920x1080.png`, `expanded-1920x1080.png`, `stack-1920x1080.png`
- `overview-1280x720.png`, `controls-1280x720.png`, `expanded-1280x720.png`, `stack-1280x720.png`
- `result-1920x1080.json`, `result-1280x720.json`

This evidence covers the local Vite/operator fixture and rendered frontend
interaction only. Native renderer/GPU/display behavior, 4K/three-output
presentation, real show media, physical output, Mac real-device acceptance,
signing/publication, and the complete C2/C4 reintegration remain unclaimed.

## Current-main video software revalidation — 2026-09-12

The bounded video/Follow software slice was rerun against current `main` at
source HEAD `c6b2f22c4670814aef192d1ef9506178e114cf81`; `origin/main` matched
before the run. No video feature, browser dependency, or physical-output path
was changed. The documented MSVC 14.44.35207 x64 linker was pinned and
returned first by `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — B4 focused model/browser-contract gate |
| `pnpm.cmd --dir app run check-timeline-follow-runtime` | PASS — stale E/G, E/R/H, visibility/focus/target, and localization contracts |
| `pnpm.cmd --dir app run check-timeline-follow-hold-ui` | PASS — explicit normalized Immediate/Hold and one-measure wait modes |
| `pnpm.cmd --dir app run check-video-runtime-polling` | PASS — deferred-response, malformed-array, generation, and zero-copy valid-array contracts |
| `cargo test -p engine --release --locked video_full_gate_engine_path -- --test-threads=1` | PASS — 1 passed, 0 failed, 0 ignored, 1082 filtered out |
| `cargo test -p engine --release --locked video_transition_bus_c3_is_typed -- --test-threads=1` | PASS — 1 passed, 0 failed, 0 ignored, 1082 filtered out |
| `cargo test -p engine --release --locked video_sample_clip_take_queue -- --test-threads=1` | PASS — 1 passed, 0 failed, 0 ignored, 1082 filtered out |
| `cargo test -p engine --release --locked video_sample_follow_admission -- --test-threads=1` | PASS — 1 passed, 0 failed, 0 ignored, 1082 filtered out |

An older documented `pnpm run check:video-clip-slot-bank` invocation was
attempted and returned `ERR_PNPM_NO_SCRIPT`; the checker exists and passed
when invoked directly by its current path. No checker or assertion was
weakened. The prior rendered-browser evidence is retained as evidence for its
recorded ancestor; this checkpoint does not claim a new browser run, native
renderer/GPU/display behavior, 4K/three-output presentation, real show media,
physical output, Mac, signing/publication, or complete C2/C4 reintegration.
`VIDEO-FULL-GATE-001`, `VIDEO-C2-C4-001`, and `TIMELINE-FOLLOW-001` remain
Open.
