# T27 Audio control mapping and ISF audio acceptance — 2026-10-20

## Scope and checkpoint

Implemented only T27 in `C:\Users\janua\AppData\Local\SyndocalDev\wt-task27` on `task27-audio-mapping`, based on `e1d72e2306620d9c64454abf426ded50c1261f4d`. The commit containing this note is the checkpoint; its hash is recorded in the external `task27-result.md` handoff.

The common batch rules override the repository's ordinary release/native gate: no product executable, native/real-app harness, release build, push or merge was run. Main and `.agents/` were not edited. Rust commands used the worktree-local debug target through `pcargo-here.cmd`, with `CARGO_TARGET_DIR` removed and the MSVC 14.44 linker verified. Frontend dependencies were installed first with `pnpm install --frozen-lockfile --prefer-offline`.

## Operator behavior

The existing assign menu now offers Audio for layer opacity, Video master opacity, stable video-group opacity, scalar ISF parameters, clip slots and Take.

| Area | Contract |
| --- | --- |
| Sources | Low, Mid, High, Level, Peak, Spectral flux, Spectral centroid, fast/slow spectral density, Kick, Snare and Onset strength. |
| Continuous settings | Gain 0–16; independent exponential attack/release of 0–10,000 ms; normalized output endpoints 0–1, including inverted ranges. |
| Trigger settings | Threshold crossing with 0.05 rearm hysteresis, or an onset publication. Initial/resumed/reconfigured high levels establish a baseline without firing. Onset mode hides controls it does not consume. |
| ISF controls | Float, Long and Bool scalar parameters, resolved by stable layer/stage/control identity. The existing parameter bounds and quantization are applied. Event, point and color controls are not offered as continuous Audio mappings. |
| Group identity | T27 uses the existing Audio-only VideoParam/Opacity envelope with a stable group id and no layer id. It does not implement T28's MIDI/OSC/DMX group actions. |
| Persistence | Additive Audio message plus optional settings in the existing project mapping list. A new assignment gets a stable id; replacement retains that id. Old projects omit the optional field and keep their existing behavior. |
| Editing | Assignment and removal share one serialized edit queue. Success follows a trusted mapping-save ACK. A queued or pending edit cannot continue into another project. Connected MIDI is reseated against the acknowledged list; disconnected MIDI stays disconnected. |
| Feedback | Audio has no MIDI wire message or feedback payload. The existing feedback worker is reconfigured only after the shared mapping list is durably acknowledged; superseded and old-project continuations are discarded. |
| Presentation | EN/JA strings, existing theme tokens and 14 px interactive labels. The expanded assign form scrolls within the viewport; its arrow keys retain ordinary form behavior. Group rows display the group identity rather than “Layer null”. |

Key implementation: `app/src/components/AudioControlAssign.tsx:4`, `app/src/audioControlMapping.ts:85`, `app/src/createControlInputController.ts:539` and `:616`, `app/src/controlMappingLabels.ts:24`, `crates/protocol/src/audio_mapping.rs:130`.

## Runtime ownership and Undo isolation

`app/src/audioControlRuntime.ts:33` evaluates only in the main renderer at a 50 ms minimum interval. At most 128 mappings are admitted. The existing Perform dispatcher owns the latest-value lane: one request may be in flight and only the newest pending batch is retained. A blocked trigger batch is dropped rather than building a backlog.

Continuous values use the new `set_audio_control_runtime` route under the existing owner/window binding, project coordinator, external-admission lock and published engine ACK boundary. Each request carries owner, epoch, revision, checkpoint hash and a monotonic request id. Native admission resolves persisted mapping ids itself, validates the output range and rejects arbitrary, removed, duplicate, stale or replay-modified requests. Exact retries are idempotent and do not renew the lease. The stable path does not clone/hash the entire project at 20 Hz; an external authored admission generation change forces reconciliation before another runtime write.

The engine overlays values on rendered video snapshots. It does not change authored layer/group/master/ISF values, publish a project revision, or add an Undo record. A failed published ACK restores the prior overlay. UI-authoring snapshots project the affected fields back to their authored base values, so an unrelated full-catalog edit cannot accidentally persist a modulated value. Recording and rendering continue to use the modulated snapshot.

Assignment settings remain ordinary project edits. Discrete clip/Take actions retain the existing Perform command semantics and admission rules.

| Risk considered | Mitigation and evidence |
| --- | --- |
| Undo/revision flood or checkpoint contamination | 500 engine updates preserve authored state; 64 native updates preserve Undo, epoch/revision/hash. UI snapshot regression covers subsequent manual catalog edits. |
| Late IPC, old owner, replay or pending project transaction | Native owner and full authority validation; monotonic request identity; bounded persisted-id resolution; existing coordinator lock held through published ACK. |
| Frozen renderer, stopped source or repeated status polls | 250 ms native lease and source freshness; repeated generation/sequence observations cannot renew the renderer's source timestamp. Invalid, stale or retired source clears modulation. |
| Queued dispatch across source/config/project change | Dynamic dispatch guard includes current source lease, mapping signature, project epoch and suspension version. Native also checks live generation and bounded feature-sequence lag. |
| Modal, Learn or busy-state activation | Existing in-app/modal selectors and Perform safety predicates suspend evaluation and revoke pending leases. Resume establishes a new edge baseline. |
| Threshold chatter and duplicate onsets | Hysteresis, fresh-publication checks and source/config baseline prevent repeated dispatch from a held level or duplicated publication. Clip actions precede Take in a batch. |
| Mapping removal and feedback races | Trusted serialized edit/removal barrier plus post-ACK feedback configuration. Real Solid reactive tests cover supersession, untrusted save and project replacement. |
| Removed/reordered group or ISF target | Stable ids; missing targets are rejected/ignored safely; reorder does not retarget another group or stage. |

Core code: `app/src-tauri/src/audio_control_mapping.rs:41`, `crates/engine/src/audio_control.rs:85` and `:110`, `crates/engine/src/audio_control_snapshot.rs:9`.

## ISF publication and rendering

The existing live analyzer now supplies a 512-element signed waveform and 512 normalized FFT magnitudes. Waveform resampling and FFT peak pooling reuse the existing sample window and transform. No second FFT, capture thread or unbounded audio queue was added.

The sample payload is 4 KiB plus generation/sequence/sample-rate metadata. Native consumers share the publication through Arc; UI and project JSON exclude it. The recording worker carries the samples as a fixed-size little-endian binary payload and the metadata in its request header, with truncation, bounds and finite-value checks.

ISF `audio` and `audioFFT` inputs are bounded mono width-by-one RGBA8 textures. Width defaults to 512; MAX must be an integer from 1 through 512. Smaller waveform textures use linear resampling, FFT textures keep bin-group peaks, and MAX=1 waveform uses an explicit host RMS policy. Silent waveform is byte 128; silent FFT is zero. A renderer/instance uploads only a changed publication identity or transition to silence.

Program, live Preview, display, Spout, NDI and recording receive the audio publication. Ordinary thumbnails explicitly select silence. Both public frame-only artistic APIs select the supplied audio context, including None; the integration test changes audio while the source and media PTS remain static.

Each maximum-width audio input uses 2 KiB of texture storage and counts toward the existing 16-texture limit. Existing 256 MiB per-stage and 512 MiB aggregate texture/readback accounting includes audio. Audio processing uses the existing GPU ISF stage and readback boundary even if the compositor consumes CPU pixels. There is no pure CPU ISF evaluator. Missing adapter support keeps the established explicit effect failure behavior.

See [the support matrix](ISF_SUPPORT_MATRIX_2026-10-20.md) for the complete supported subset, specification references, resource limits and historical T23 evidence. T27 implementation is in `crates/audio/src/live_features.rs:550`, `crates/video/src/isf_runtime.rs:596`, `crates/video/src/isf_pass_runtime.rs:108`, and `app/src-tauri/src/recording_audio_textures.rs:16`.

## Validation

All listed commands completed successfully. No compiler warnings were emitted in the checked Windows debug configuration. No new allow/expect attributes were added. Modified Rust is UTF-8 without BOM and uses LF; formatting is confined to new files and owned hunks. `git diff --check` passed.

Rust invocations below are arguments to `cmd /d /c C:\Users\janua\AppData\Local\SyndocalDev\tools\pcargo-here.cmd` from this worktree, with `CARGO_TARGET_DIR` unset.

| Check | Result |
| --- | --- |
| `check --workspace --all-targets` | Passed; zero compiler warnings. |
| `test -p io --lib -- --test-threads=1` | **234 passed, 4 ignored, 0 failed, 0 filtered** — required full suite. |
| Protocol audio-filtered unit tests | 15 passed. |
| Audio live-feature unit tests | 16 passed. |
| Engine `audio_control_` | 7 passed, including persistence, expiry, rejected-ACK rollback, stable ISF identity and UI-authoring projection. |
| Video `isf` | 43 passed, including five T27 audio tests. Three of the five render actual GPU pixels without an adapter-skip branch; two validate metadata/limits. |
| Video `c1_` | 27 passed, including compositor parity and the new public frame-only audio-context regression. |
| Syndocal `audio_runtime_` | 4 passed; three new native admission/Undo tests plus the existing audio storage contract. |
| Syndocal `recording_audio` | 3 passed, including bounded binary sample payload validation. |
| Syndocal `render_request_` | 1 passed; real request encoding/decoding retains audio and external-frame bytes in binary form. |
| Syndocal `live_audio_feature_` | 3 passed. |
| `node app/scripts/check-audio-control-mapping.mjs` | 18 checks passed: math, persistence shape, source mapping, identity, bounds, trigger edges, leases and the actual latest-value/Perform lanes. |
| `node app/scripts/check-midi-learn-controller.mjs` | 7 reported groups passed plus removal assertions. Includes real Solid client-runtime effects for feedback ACK/supersession/project fencing. |
| Required `check-ai0-source-coverage.mjs` | Passed with all source families and authority gates inventoried. |
| `check-ai7-adversarial-proof.mjs` | Passed. |
| `check-frontend-command-routing.mjs` | Passed: 137 renderer mutations, 31 server-authoritative mutations, 28 raw dispatches, 501 facade dispatches. |
| `check-tauri-admission-inventory.mjs` | Passed exact route set/hash, negative mutations and runtime-admission checks. |
| `check-perform-midi-routing.mjs`, `check-midi-fader-input.mjs`, `check-video-layer-groups.mjs` | Passed. |
| `check-fx-catalog.mjs` | 14 built-ins and 24 bilingual rejection cases passed; obsolete audio rejection replaced with the bounded MAX diagnostic. |
| `check-control-assign-menu-browser.mjs` | Passed at 375×640 and 1280×720, EN/JA, eight themes, keyboard behavior, form containment, Onset disclosure and contrast. Ports 5430/9430. |
| `pnpm --dir app run check:localization` | 4,155/4,155 static strings, 100%; zero unprotected user-data labels. |
| `pnpm --dir app exec tsc --noEmit` | Passed. |

The four pre-existing ignored IO tests require physical MIDI input, physical MIDI feedback ports, virtual MIDI loopback ports, or the operator-owned RB output-router environment. They were not enabled.

Re-pinned inventory: native Tauri 587; Engine 286; Remote 116; MIDI/OSC/DMX 230 (8 MIDI messages); frontend invoke manifest 507; keyboard 33; legacy total 1,726; canonical source total 1,759; canonical operation total 76. Native route SHA-256: `45651e91ffedff178454d19ec44ae76423ed26d1a3ed41ee6af8918f54d95353`.

Worktree-local ignored evidence: `target/t27-validation/` and `target/qa/t27-audio-assign/`. The narrow Japanese screenshot was also visually inspected for readable, contained controls.

### Cost evidence and remaining measurement boundary

The five audio GPU/parser tests completed in 1.16 s; the seven engine audio tests in 0.09 s; the four native audio-runtime tests in 0.12 s. These are test-group wall times, not production per-frame benchmarks. The full ISF run took 32.22 s and the C1 run 29.27 s. They prove bounded behavior and known pixels on the test adapter, not live output FPS.

The hot path remains bounded by one existing FFT publication, fixed 4 KiB sample data, at most 128 mapping values at 20 Hz, and one current GPU upload per audio input/publication. The existing GPU readback, output compositor and recording-process boundaries remain. Native multi-output load, capture-to-pixel latency and real-device onset quality still need measurement.

## Review and remaining acceptance

Three independent read-only agents reviewed runtime/authority isolation, frontend mapping/dispatch, and ISF propagation/resource ownership. Review fixes included UI-authoring snapshot contamination, source/config lease invalidation, modal detection, the two public artistic rendering entry points, trusted assignment/removal ACKs, feedback configuration races, group labeling and precise support claims. Final bounded follow-up reviews reported no remaining confirmed blockers in those areas. Root reviewed the integrated diff and executed the checks above.

Remaining acceptance belongs to Claude's authorized main-checkout/native session:

1. Start the selected real audio device, assign opacity/group/master and scalar ISF controls, then stop/restart/disconnect it; verify visible modulation, silence and authored-base restoration.
2. Exercise threshold/onset clip and Take mappings with a modal open, Learn active, project replacement and mapping removal; confirm no delayed actions.
3. Save/reopen a project and use Undo after manual edits during modulation; confirm only authored data is retained.
4. Compare Program, live Preview, display/Spout/NDI and recording using `audio.fs`, including a static image source; measure pacing and capture-to-output latency under representative load.

Limits are explicit: one selected mono mix, RGBA8 audio textures, widths at most 512, 128 Audio mappings, normalized scalar targets, no pure CPU ISF execution and no claim of full ISF conformance. Trigger evaluation uses current publications at 20 Hz; it is not a sample-accurate event recorder, and intermediate publications or triggers arriving during a busy lane can be dropped. Native-window, physical audio/MIDI/output and non-default ASIO acceptance were not performed in this worktree.
