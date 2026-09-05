# Preview clock and fixture installation orientation

Branch/base: `codex/syndocal-v1.2` / `89112d38afebbdc55fa4d98e2bb6e8de1a14f3de`.
This checkpoint continues the user's right-lower preview stutter report and adds
fixture installation alignment/targeting plus accurate 2D beam direction/angle.
Native UI actions and project editing remain user-operated. Only the exact
checkout executable may be stopped/restarted for native gates.

## Confirmed causes and implementation

- Previous checkpoint eliminated the invisible second preview controller. Its
  steady native sample still had 26.72 requests/s, approximately 23 distinct
  PTS/s and 14–15% repeated PTS. Over 56.11 wall seconds the media advanced only
  54.55 seconds. These are distinct from the resolved BUSY contention.
- Internal Timeline advancement truncated each tick's sub-millisecond duration.
  Runtime-only fractional carry now follows the admitted Timeline/transport
  identity and exact position. Authority preflight and actual advancement share
  one non-consuming delta calculation. Seek/pause/load/count-in/external clocks
  retire old carry; normal wraps preserve it; armed jumps discard it. No project
  schema or DMX frequency change.
- Preview scheduling uses fixed-origin 60 Hz slots, at least two slots per output,
  with one IPC in flight. Missed slots are dropped; timer lateness no longer moves
  every following deadline. This is a long-run 30/60 Hz budget, not a promise of
  minimum elapsed spacing or at most 60 requests in every sliding second.
- Continuous source sampling uses engine-owned, half-open validity windows for
  ordinary root Timeline file clips, including lighting-only child timelines.
  Pause, seek publication, loop, count-in, external sync, Follow, independent
  video launch, modulation and unknown source duration stay canonical. Clip,
  source and event boundaries cap interpolation; publication age is bounded to
  two DMX ticks. Snapshot and sidecar locks precede clock reads. Configuration
  changes retire the semantic fence; expiry returns BUSY while semantic changes
  clear stale frames. DJ Link reserves the sidecar before its irreversible commit.
- Old 2D `beamPoints` ignored profile optics, pitch and roll, and made every
  full-intensity fixture approximately 42 degrees wide. It also pointed zero yaw
  toward screen -Z, unlike the world +Z installation basis. The retired function
  is removed. `mappingFixtureBeam` projects the local cone onto X/Z, including
  geometry direction, head Pan/Tilt then mounting rotation. Vertical directions
  produce footprints. Geometry-specific color segments keep their own direction.
- BeamAngle is preferred; FieldAngle is labelled separately when used alone.
  Missing angles use explicitly labelled schematic 10-degree spot/moving/point,
  30-degree wash and 2-degree laser illustrations. Authored invalid angles are
  not silently replaced. Cone length is a fixed presentation extent, not a
  photometric throw-distance model. Dynamic Zoom calibration remains outside this
  illustrative profile-angle contract. Brightness controls opacity, not aperture.
- Rust Visualizer now composes local head rotation before mounting too. Geometry
  origin and model placement remain unchanged; no DMX or project mutation occurs
  in either rendering path.
- Mapping selection gains matching the active selected fixture's Yaw/Pitch/Roll
  (otherwise first selected) and aiming each installation +Z axis at XYZ with
  Roll=0. The entire target plan is validated before mutation; coincident/invalid
  targets are rejected. Existing ticketed transform persistence and partial-batch
  failure reporting are reused. This changes installation orientation, not DMX
  Pan/Tilt or per-geometry optical axes; the UI explains that distinction.

## Evidence so far

- `node app/scripts/check-mapping-installation-orientation.mjs`: PASS pure math,
  actual controller/batch behavior, validation before writes, failure reporting
  and concurrent exclusion.
- `node app/scripts/check-mapping-orientation-browser.mjs`: PASS actual component
  actions/keyboard/selection/pending/invalid input at 1280 and 640; screenshots
  inspected. Existing sizes preserved, long new actions reflow onto full rows.
- `node app/scripts/check-mapping-beam-geometry.mjs`: PASS profile angle precedence,
  missing/invalid metadata, world basis, aiming agreement, local-head composition,
  vertical projection and individual geometry direction.
- `node app/scripts/check-mapping-beam-browser.mjs`: PASS actual render model and
  beam layer with reactive pitch and three profile apertures. Root inspected
  `target/qa/mapping-beam-20260905/beam-angles.png` and `beam-vertical.png`.
  Initial fixture attempts failed because test data omitted channel_name; the
  fixture was corrected and the actual model test then passed.
- `node app/scripts/check-timeline-output-monitors.mjs`: PASS existing monitor
  safety plus late/jitter/hung/hidden/retry/project cadence regressions. Under small
  late timers, each output gets 149–150 starts per half-open five-second sample.
- `pnpm --dir app exec tsc --noEmit`: PASS at current frontend integration.
- MSVC-pinned `run-native.mjs cargo test -p engine --locked timeline_fractional_ticks
  -- --nocapture --test-threads=1`: 6 PASS after independent review found and owner
  fixed the zero-delta jump case; prior five-test run also passed. Compiler
  warnings baseline0/current0/delta0 for these runs.
- MSVC-pinned `run-native.mjs cargo test -p visualizer --locked beam_
  -- --nocapture --test-threads=1`: 3 PASS, including old GDTF orientation and new
  geometry/no-geometry head/mount and roll regressions; compiler warnings0.
- Independent reviews accepted fractional carry, fixture orientation controls,
  head/mount beam composition and the fixed-slot scheduler with its stated timing
  limits. Independent continuous-sampling review accepted the final diff after
  fixing dirty-config invalidation, clock acquisition, DJ Link reservation and
  queued/Take guards.
- MSVC-pinned `run-native.mjs cargo test -p engine --locked video_sample_
  -- --nocapture --test-threads=1`: 10 PASS, 1 deliberately ignored local-file
  test. Initial compile/clock-test failures were repaired, not waived. Windows
  Instant's backward epsilon requires an explicit future-anchor comparison.
- The ignored `video_sample_real_project_with_lighting_child_admits_plain_root_video`
  test separately PASS with `--include-ignored` and
  `SYNDOCAL_VIDEO_SAMPLE_TEST_PROJECT=C:/Users/kouty/Downloads/DSF2026_NightGirl-AV-test.sdc`.
  This reads the project without running an engine thread or physical output.
- Fractional carry 6 tests and
  `timeline_transport_authority_multiwrap_preflight_is_exact_and_atomic` 1 test
  rerun PASS after integration. Rust compiler warnings current0/delta0.
- Monitor regression and TypeScript rerun PASS after native integration.
- `pnpm --dir app tauri build --no-bundle`: PASS, release compile2m26s,
  first-party compiler/build warnings0. Wrapper verified the exact14.44 linker
  and stopped the exact checkout executable before linking. Log:
  `target/qa/snapshot-cleanup-20260905/preview-clock-fixture-aim-native-build.log`.
- New executable SHA256 `1470AD28BCD7645776FFE0CD20E4C65E3A34ADF6910508B9EFB3C64EC59A0FB0`,
  PID45852 at diagnostic launch; exactly one responsive maximized Syndocal main
  window. Evidence `preview-clock-fixture-aim-diagnostic-launch.json` in the same
  QA folder. Read-only fetch/frame observers installed on loopback38479.
  User has been asked to play NightGirl and keep the right preview visible for30s;
  playback result is pending. Remove instrumentation and relaunch normally when
  measurement finishes; this diagnostic instance is not the final handoff state.
- Native playback acceptance FAILED: the user saw `Output preview frame expired`.
  `stutter-native-continuous-running.json` captured1565 requests, one in flight,
  ~12.6Hz/output over the mixed sample, with109/110 BUSY packets. Fast sections
  still render at p95~6.56/3.29ms, but sustained slow sections decode130–250ms.
  The250ms sequential-decode gap causes repeated session reopen when two slow
  outputs exceed that per-output gap. The45ms sampling window also rejects cold
  frames after rendering. Do not claim the stutter fixed from passing unit tests.
  Read-only observers have been removed; diagnostic process remains until the
  next build. Pending repair: bounded sequential catchup and distinct fresh
  capture / semantic-boundary / delivery-age deadlines, independent review and
  new native playback measurement.
- Follow-up decoder repair: forward source-time gaps up to1s retain the live
  decoder; larger gaps and backward seeks still reopen. This bounds source-time
  catchup, not decoder wall time. Independent review ACCEPT.
  MSVC-pinned `run-gpu-native.mjs cargo test -p video --locked --features libav
  catch_up_ -- --include-ignored --nocapture --test-threads=1`: 2 PASS with
  `SYNDOCAL_GPU_TEST_VIDEO` set to the user's original foregroundMP4. Two GPU
  sessions remained open through350–400ms gaps, then reopened for1500ms and
  backward seeks. PTS/dimensions/duration and bounded pixel parity matched fresh
  software seeks; hardware errors0. Existing `session_decision_` test1 PASS.
  Compiler warnings0 for both runs. Final rebuilt-app measurement still pending.
- Delivery repair keeps the capture window at two DMX ticks, removes the
  artificial100ms semantic cap, and limits delivery to min(capture+500ms, actual
  Timeline event/clip/source/Follow/end boundary). Commands, semantic generation
  changes and ownership epochs still reject old frames. This is not permission
  to extrapolate500ms. `video_sample_`: 13 PASS/1 ignored; the ignored real-project
  fixture separately1 PASS. Compiler warnings0. Subsequent native build log is
  `preview-catchup-delivery-native-build.log`; playback proof remains pending.
  Independent delivery-deadline review ACCEPT, including publication-gap
  invalidation and overflow-safe duration subtraction after removing100ms cap.
- Follow-up `pnpm --dir app tauri build --no-bundle`: PASS3m25s, warnings0,
  exact-linker and exact-checkout stop verified in the build log. New SHA256
  `855D741177E681AF1274175C8235EC61CEBC868CA8B5BCB23995E67F3449DE44`.
  Initial observer attachment raced WebView startup; discarded that process and
  relaunched cleanly, then installed both observers sequentially. Current
  diagnostic PID15512, one responsive maximized Syndocal main window,
  `preview-catchup-delivery-diagnostic-launch.json`. User asked to show preview
  and replay; new performance measurement is pending. No checkpoint commit yet.
- Follow-up native playback measured successfully in
  `stutter-native-catchup-delivery-steady.json`: 3000 requests/~52.85s,1500/output,
  BUSY0, one IPC in flight. Foreground/background28.37/28.35 requests/s,
  duplicate PTS75/91 of1499 transitions (~5.0/6.1%), distinct advancement about
  26.95/26.63 per second; render p95 6.57/3.16ms, paint p95 .1ms. Media advanced
  52.819/52.853 seconds over essentially equal wall intervals. Two long tasks
  were observed; no sustained30fps guarantee or Unity receipt is claimed.
  No expiration reproduction during this sample. GPU slow-gap/seek regression
  provides deterministic recovery evidence; arbitrary native seeking remains
  unverified. This supersedes the earlier failed native measurement.
- Observers removed. Normal restart command was rejected by automatic approval
  review with generic `blocked by policy`, before execution. No workaround stop
  was attempted: PID15512 remains the diagnostic executable, port38479 may still
  listen until a permitted close. User informed. Next native build needs the user
  to close Syndocal if this restriction persists. The next requested UI tranche
  removes the preview's fixed180px height cap; Unity output work remains next.
- An existing `Recovery checkpoint failed: Project transaction owner does not
  match the invoking renderer window` footer was observed separately. Read-only
  tracing points to old-owner publication intents across restart / shared pane
  storage; not yet confirmed against the live intent. It may block desktop
  backup or manual publication, but the common catch does not prove browser
  recovery failed. No storage deletion, owner replacement or project save was
  attempted. Track separately from video performance.

## Next: Unity visualizer (user authorized after current checks pass)

- Existing project `E:/UnityProjects/Art-net-Unity`, scene
  `Assets/DSF2026/Scenes/DSF2026_Visualizer.unity`. Preserve existing user fixture
  placements and dirty/untracked files; do not rebuild the scene.
- Saved receivers: Art-Net `0.0.0.0:6454`, wire universe0; Spout exact names
  `Syndocal Foreground` / `Syndocal Background`. Daslight currently owns UDP6454;
  no Daslight termination or settings change performed.
- User clarified the transport is foreground3840x2160/background1920x1080,
  both16:9. Physical surfaces are5:2 with top/bottom clipping, not stretched16:9.
  Existing Unity instead strictly expects3840x1536/1920x768 and clears mismatches.
  Required receiving contract: full16:9 textures, central5:2 display crop (312px
  each vertical edge foreground,156px background). Keep background opaque and
  foreground black-key. Existing Syndocal V2 sends both1920x1080; foreground4K
  still needs a scoped implementation and output validation.
- Current NightGirl saved media/output metadata is already5:2. Preserve that
  content by placing it centrally in the16:9 sender frame with vertical black
  bars before the receiver crop. Do not stretch or crop the5:2 source twice.
- CLI scene read returned authentication-required. Connector can list/select the
  existing engine but exposes no execution tool in this session. No Unity Play,
  scene write or output start has occurred; live Unity receipt is unverified.

## Protected state and remaining gates

`app/scripts/check-viewport-containment.mjs` remains unrelated, dirty and unstaged;
SHA256 `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`.
Its old literal beamPoints signature assertion is stale after the beam change;
this file is not edited to hide it. New dedicated actual-model/geometry checks
cover the changed behavior; the entire viewport checker is not claimed passing.
No cache cleanup or project-file rewrite. Native build/start passed; user-driven
playback measurement remains necessary before claiming the stutter resolved.
Owned checkpoint is not committed/pushed yet while this acceptance is pending.
