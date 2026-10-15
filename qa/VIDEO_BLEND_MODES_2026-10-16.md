# Layer blend modes - 2026-10-16

- Task 6 only; shared `main`, base `57f9917dbc139da663310166fbba4d3edbaef7a3`.
  Uncommitted for Claude review; existing untracked `.agents/` untouched.
- Domain: completion-flow Phase 5 Video and [the layer/composition model](SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md).
  No generators, ISF, group transforms or NDI distribution changes.
- Plan completed: append wire variants, preserve canonical CPU/table and legacy arithmetic,
  mirror both GPU shaders, expose/localize the existing menu, validate software and native Display.

## Chosen modes and compatibility

With normalized backdrop `d`, source `s` and effective source alpha `a`, RGB uses
`round/clamp(255 * (d + (B(d,s)-d)*a))`; existing destination-alpha behavior is unchanged.

| Mode | B(d,s) | Inclusion |
| --- | --- | --- |
| Overlay | `d <= .5 ? 2*d*s : 1-2*(1-d)*(1-s)` | Requested contrast blend |
| Subtract | `max(d-s,0)` | Subtraction gap in the comparison |
| Difference | `abs(d-s)` | Requested symmetric difference |
| Lighten | `max(d,s)` | Requested bright-channel selection |
| Darken | `min(d,s)` | Requested dark-channel selection |
| Soft Light | `s <= .5 ? d-(1-2*s)*d*(1-d) : d+(2*s-1)*(D(d)-d)` | Requested gentle contrast |

`D(d)=((16*d-12)*d+4)*d` for `d <= .25`, otherwise `sqrt(d)`.
The five standard artistic formulas follow [W3C separable blending](https://www.w3.org/TR/compositing-1/#blendingseparable).
Subtract means backdrop minus source, clamped before alpha interpolation.
The [comparison](SYNAPSERACK_FEATURE_COMPARISON_2026-10-15.md) observed AlphaAdd,
Inversion, Substract and Cutout names without their pixel/alpha semantics.
Subtract addresses subtraction; AlphaAdd/Inversion/Cutout and an AlphaBlend alias are omitted
because guessed semantics or aliases would imply unproved vendor equivalence.
All six specifically requested modes are included; no SynapseRack pixel-equivalence claim.

- Serde keeps PascalCase `Normal`/`Add`/`Multiply`/`Screen`; new variants are appended.
  `SoftLight` is the wire value, `Soft Light` UI copy. Version-1 projects need no migration;
  unknown/wrong-case names still reject without fallback. Old four CPU formulas are byte-identical.
- A project using a new blend mode cannot be opened by an older Syndocal, which rejects the unknown variant; an Active/Standby pair must run the same version before such a project is used.
- Canonical `blend_channel` supplies generic/axis/banded composition and `OpaqueSourceBlendTable`.
  No duplicated hot-path policy, extra frame copy, worker or allocation. Indices 0-3 stay fixed;
  4-9 use new enum order in both GPU paths. Large opaque images retain the existing lookup optimization.
- CPU Recording/NDI/Spout and composition previews retain the authored mode. Isolated source
  thumbnails intentionally retain Normal/opacity 1 (`layer_thumbnail_render.rs:54`).
- Tauri accepts the typed enum and Engine publishes it unchanged. MIDI/OSC/agent bridge expose
  no blend setter or restricted mode registry; unrelated EffectBlendMode/benchmark arrays are unchanged.
- The sole Solid menu has all ten choices and Japanese copy. A local CSS exception exposes the
  previously hidden Mixer control; content-sized rows prevent next-row pointer interception.
  Existing font/control sizes and internal scrolling are preserved.
- Independent read-only video/frontend and protocol/engine reviews found no blockers;
  supervising agent inspected the diff and evidence. No product/version bump or publication.

## Validation

Logs/captures: `target/qa/blend-modes/` (ignored). Every Cargo command used the requested
`pcargo-here.cmd`, exact Build Tools MSVC `14.44.35207` x64 linker pinned/PATH-first verified.

| Command / check | Result |
| --- | --- |
| `test -p video --features libav` | PASS: 232 passed, 10 ignored; doc tests 0 |
| `test -p video` | PASS: 208 passed, 5 ignored; doc tests 0 |
| `test -p protocol` | PASS: 267 unit + 1 API + 7 contract + 4 doc tests |
| `test -p engine` | Initial parallel: 1130 passed, 1 timing failure, 14 ignored; full serial rerun below passes |
| `test -p engine -- --test-threads=1` | PASS: 1131 passed, 14 ignored; doc tests 0; exit 0 |
| `test -p syndocal --bin syndocal` | Initial parallel: 2105 passed, 2 DJ failures, 22 ignored; isolated and full serial retries pass |
| `test -p syndocal --bin syndocal -- --test-threads=1` | PASS: 2107 passed, 22 ignored; exit 0 |
| `test -p video --features libav gpu_ -- --nocapture` | PASS: 30 passed, 4 explicitly ignored; no adapter/BC-support skips |
| `pnpm.cmd --dir app run build` | PASS: TypeScript + Vite |
| `check:video-blend-modes` | PASS: en/ja, 1920x1080 and 1280x720; all 10 options, 6 callbacks/authoritative refreshes per case |
| `check:localization` | PASS: 3941/3941, no unprotected labels |
| `check:vj-operator` | PASS after final CSS: en/ja at 1920x1080, 1366x768, 1280x720 |
| `check:frontend-invokes` / `check:frontend-command-routing` | PASS: exact 500 commands; 135 renderer/31 authoritative mutations, 28 raw/492 facade dispatches |
| `check:video-runtime-polling` / `check:tauri-build-wrapper` | PASS: stale/overlap checks; 249 wrapper assertions, 27 hostile fixtures |
| `build-nobundle.cmd` | PASS exit 0; wrapper verified/stopped only checkout PID 15620 |
| `native-gate.ps1 -Exe <checkout>/target/release/syndocal.exe` | PASS exit 0; one responsive maximized Syndocal main, showCmd=3; normal close |
| Encoding / owned-hunk rustfmt / `git diff --check` | PASS: UTF-8/no BOM, Rust LF, no added allow/expect; unrelated baseline formatting preserved |

Engine's unchanged 128k scheduling test took 101.1709ms, then 120.5387ms during compilation,
against its **75ms** budget. After compilation, the exact test passed at 39.7234ms / 188.2us dispatch
(dispatch budget 25ms); full serial suite passed. An initial short `--exact` filter selected zero tests
and was not counted; corrected fully qualified retry ran one test. No assertion changed.
Both DJ failures (`dj_link_generic_active_capacity_is_fail_closed_without_mutation`,
`dj_link_stage2_commands_require_release_authority_and_exact_correlations`) passed individually
(one test each) before the complete serial pass. Task-listed known flakes did not fail; original logs retained.

Legacy byte references remain exhaustive; all ten modes cover generic/axis/banded/table paths.
Independent f64 references cover new byte pairs and alpha/opacity boundaries. GPU grids cover every
channel pair at opacity 0/.37/1 within one level per blend. Mapped RGBA/BGRA allows two total levels
for two blends; isolated DXT1/DXT5 blend checks allow one. Unrelated multi-pass tolerances are unchanged.

Browser plugin unavailable; Playwright uses `http://127.0.0.1:5198/?syndocalViewportFixture=operator-vj`.
Identity/content/overlays/interaction/state/localization/screenshots pass; native IPC alone is mocked.
Control sizes 186.44-355.13 by 29-30px; outer overflow 0; no app/uncaught errors.
One baseline favicon 404 is separately recorded; monitor busy packets supply no fake rendered image.
Archived HEAD frontend and candidate builds both emit one existing Vite >500kB advisory (App 517.29kB),
delta 0; TypeScript 0/0 diagnostics. Current Rust tests/release build: 0 compiler warnings;
separate Rust baseline not rebuilt. No warning suppression or chunk-limit change.

## Native Display evidence

Release SHA-256: `6683ac2a8abbecbb8324120cbf27306d3e763236c9382d766d0c67906638db2a`.
The provided gate also lists a 13x13 `jp.seraf.ktn.syndocal-siw` helper, not a second Syndocal main.
A version-1 fixture supplies two 320x180 opaque PNG patterns and one local windowed 960x540 Display;
no fixtures/enabled DMX routes. Initial fixture frame_rate 0 rejected; only the fixture was corrected to 30.
Out before authority was correctly denied; ordinary owner-bound Enable receipt then applied,
and the explicitly authorized editor-monitor Display confirmation was accepted.
Some main captures retain that earlier rejection footer; later receipts/window observations prove success.
The existing process/loopback-verified debugger sets the real foreground select and dispatches its normal
input event, without mocking native code; each action verifies maximized main and actual publication
(revisions 2-7). Computer Use activates/captures the actual returned main and Display windows.

| Mode | Observed Display |
| --- | --- |
| Overlay | Strong checker contrast, dark blue/purple and bright cyan/yellow/pink |
| Subtract | Black/blue upper area, saturated green below, red/yellow right strip |
| Difference | Flat green fields with orange/yellow right strip; checker differences cancel |
| Lighten | Bright pastel checker pattern |
| Darken | Dark checker pattern retaining blue/purple areas |
| Soft Light | Gentler contrast-enhanced gradients/checkers than Overlay |

Each mode has `native-main-<Mode>.jpg`, `native-display-<Mode>.jpg`, `native-<Mode>.json`.
All six were visually inspected, differ from Normal/each other, and also changed the real Program monitor.
These captures are visual evidence; automated GPU tests supply numerical parity.
Display closed with applied receipt; empty project restored. `native-current.json` proves effective/persisted
Standby, both output domains disallowed, no layers/outputs/live Display windows. Editor remains available.

## Boundary and next action

No recording take, NDI/Spout peer, external MIDI/OSC device, physical projector, multi-display/venue,
installer or product-wide acceptance was exercised. Source thumbnails retain source-preview semantics.
Claude reviews this uncommitted diff next. Nothing staged, committed or pushed; no cache cleanup.
