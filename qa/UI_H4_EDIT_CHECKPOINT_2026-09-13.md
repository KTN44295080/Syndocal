# UI H4 Edit checkpoint — 2026-09-13

## Checkpoint identity

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `c250df88` (H3 DJ Link Setup checkpoint)
- Scope: current-source Windows Edit workspace software, rendered-browser contracts, focused Rust release tests, and the required Windows native build/process smoke.
- This checkpoint is intentionally bounded. It does not continue into H5 Control, physical-device acceptance, or release/distribution acceptance.
- No product-version bump was required; this is an internal checkpoint.

## H4 closure scope

H4 covers the current Edit authoring path for Media and Video, Timeline authoring, Phase/Guide/loop/group/follow behavior, authored audio and source-shelf integration, scoped FX editing, Stage labels, project history and Save/Reload behavior, focus/tab contracts, import/relink entry points, and the native process gate. The result is a software/native-process checkpoint, not a claim that every Q1 row or every external acceptance gate is accepted.

The following current-source boundaries were checked:

- Media library and full Video Control reachability, thumbnail/derived-data request handling, import/relink and authority/error paths.
- Timeline advanced authoring, Follow/Hold/Cut/Complete policy, musical loop, transport, cue audio, output bus, snap, source shelf, layered viewport, and overlap containment.
- Edit Video Inspector FX scope, effect palettes, Stage labels, and shared Edit/Mixer/Live tab association.
- Project transaction, recovery, publication, open/bootstrap, history, Save/Reload, shortcut, and source-manifest contracts.
- Browser-only Timeline authority admission and exact payload/fence rejection behavior.

## Implementation and repair in this checkpoint

1. `app/src/components/VideoControlPanel.tsx` restores the `id`, `role="tabpanel"`, and `aria-labelledby` association on the full Mixer panel. The library-only route keeps its existing association; the full Control route now participates in the Edit domain tab contract.
2. `app/src/styles.css` constrains the full Video Control to the upper Edit grid row. The previous `grid-row: 1 / -1` overlay intercepted the lower persistent Inspector band, so this was a real interaction defect found by the viewport harness and fixed in product CSS.
3. `app/scripts/check-edit-video-fx.mjs` scopes ISF-layer/action assertions to the Edit Inspector. The full Video Control intentionally mounts six global FX consumers, so an unscoped count was a stale test assumption rather than a product failure.
4. `app/scripts/check-viewport-containment.mjs` updates `--edit-ia-video-only` to the current full Video Control route, measures the anchored Media rail/popover, and checks the current direct import entry. The old library-only fixture contract was removed from this focused gate.
5. `app/src-tauri/src/main.rs` aligns the sorted preflight runtime-route inventory and expected route count with the current 163-route source inventory. This repairs a stale test contract; runtime behavior was not loosened.

## Verification evidence

All commands below were run from the repository root on the current checkout. Exit status was checked for every command.

| Area | Command/result |
| --- | --- |
| Edit Video FX | `pnpm.cmd --dir app run check:edit-video-fx` — PASS at 1920x1080 and 1280x720; mounted `0/1/0`, selected-layer callback `2`, controls `64`, outer overflow `0`. |
| Edit Video IA | `PLAYWRIGHT_MODULE_PATH=C:\Users\janua\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\node_modules\playwright`, `CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe`, `pnpm.cmd --dir app run check:edit-ia-video` — PASS at 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720; all selected `1`, scroll `zero`, failed `[]`. |
| Timeline/browser contracts | `check:timeline-advanced`, `check:timeline-follow-runtime`, `check:timeline-follow-hold-ui`, `check:timeline-loop-runtime`, `check:timeline-transport-runtime`, `check:timeline-cue-audio`, `check:timeline-audio-output-bus`, `check:timeline-snap`, `check:timeline-source-shelf`, `check:timeline-layered`, `check:timeline-viewport`, and `CHROME_PATH=... pnpm.cmd --dir app run check:timeline-authority` — PASS. The authority gate covered injected-fault latch, trusted-input-only rearm, exact registration/handoff/split/move payloads, invalid-target silence, and unknown-command rejection. |
| FX and Stage | `check:fx-visual`, `check:fx-palettes`, and `check:stage-labels` — PASS. |
| Media/project software contracts | `check:media-thumbnails`, `check:media-asset-operations`, `check:media-asset-authority`, `check:bundled-library`, `check:project-storage`, `check:project-transaction`, `check:project-authority`, `check:project-recovery-e3`, `check:project-publication-e4`, `check:project-open-bootstrap`, `check:project-shortcuts`, `check:dvc-import-controller`, `check:native-thumbnail-request`, `check:vj-first-run`, and `check:vj-first-run-viewport` — PASS. The project subset records the unavailable local Daslight golden as an explicit skip, not as a pass for that external corpus. |
| Engine release subset | Exact MSVC procedure with Build Tools 14.44.35207 linker; `cargo test -p engine --release --locked -j 1 timeline_ -- --nocapture --test-threads=1` — `195 passed; 0 failed; 3 ignored`. |
| Tauri project release subset | Exact MSVC procedure with the same pinned linker; `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_ -- --nocapture --test-threads=1` — `177 passed; 0 failed; 0 ignored`. |
| Frontend build/contracts | `pnpm.cmd --dir app build` — PASS (`tsc --noEmit`, Vite: 357 modules); `check:frontend-invokes` — exact 475 commands; `check:frontend-command-routing` — 133 renderer mutations, 31 server-authoritative mutations, 28 raw dispatches, 481 facade dispatches; `check:empty-states` — PASS. Vite emitted its existing large-chunk advisory; no first-party compiler failure occurred. |
| Windows native build | `vcvars64.bat -vcvars_ver=14.44`, pinned `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=...14.44.35207...\\link.exe`, `pnpm.cmd --dir app tauri build --no-bundle` — PASS; generated `target/release/syndocal.exe`. |
| Native process smoke | Exact executable path `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe`; pre-launch exact-process count `0`; launched PID `43840`; exactly one matching process, title `Syndocal`, window handle `0x3D50A8C`, `Responding=True`, `HasExited=False`; stopped only that exact executable process; remaining exact-process count `0`. |
| Native artifact identity | SHA-256: `853303B90C95195877B6B215B4496D5CAC0C3BD4CAFCDBD44D43EF7B5936B2C9`. This is an unsigned current-source build artifact, not a published release. |

## Evidence boundary and remaining work

The browser evidence was produced with the repository's local Vite/Chrome Playwright/CDP harness; the browser plugin was unavailable in this environment. It proves rendered-browser behavior at the listed viewports, not native button-by-button behavior.

This checkpoint does not claim native NVDA, High Contrast, color-independent status, 125/150/200% scaling, IME, screen-reader, DPI, or reduced-motion acceptance. It also does not claim physical HDMI/display/GPU, DMX/Art-Net/sACN/USB/RDM, MIDI/OSC/TouchOSC, audio-interface, camera/NDI/Spout/Syphon, downstream-client, two-machine, venue, soak, signing, SBOM/notices, clean-machine installation, publication, or H5 acceptance. The known `check:output-ownership` baseline issue concerning the diagnostic route's legacy ingress inventory remains outside H4 and is carried to the next checkpoint.

The completion counts after this checkpoint are **23 Open + 8 Deferred + 27 Complete = 58 Flow markers**. H4 is the only Flow marker closed by this checkpoint; Q1 requirements remain `In progress` where their broader native, hardware, external, cross-platform, security, or release gates remain open.
