# Syndocal 2026-09-02 show completion handoff

Status date: 2026-09-03 JST
Source checkpoint: branch `codex/syndocal-v1.2`; blackout-release source commit
`5ae65a37fc428760f2032ef30ea9b2ccd84bcd4a`; serial-smoke evidence commit
`734a2c9`; final HEAD equals upstream `origin/codex/syndocal-v1.2` (verified
after this handoff update).

This is the concise authoritative resume note for the final show-critical tranche. It supersedes chat-only status, but it does not supersede the detailed acceptance documents named below.

## 2026-09-03 alpha.68 blackout release query-race source checkpoint

The DMX blackout-off path now consumes the validated active-Both receipt from
its single bounded Enable recovery instead of trusting an immediately repeated
fail-fast lease read. Release remains backend-fenced and fails closed if that
receipt or current owner/lease CAS is not valid. ASIO advanced configuration
starts collapsed in Setup while Normal WASAPI keeps its primary route visible.
Focused output-control and safety-blackout checkers (both included in the
`check:release` aggregate), TypeScript, Node syntax, and diff checks are the
pre-native gates and pass. The safety checker follows
the extracted `executeBlackoutRelease` controller boundary rather than
requiring App to inline the R4 operation. The
exact pinned Community MSVC `14.44.35207` no-bundle native build passed in
`3m20s`; the native artifact identity is the exact release evidence for this
checkpoint: `target/release/syndocal.exe`, Product/FileVersion
`1.2.0-alpha.68`, `63,520,256` bytes, SHA-256
`0B30AC14FD84F28A628522024AD573ADECE5DD1210FC84A80971F3A3F24FA08F`.
The current exact artifact process is PID `74264`, supplying exactly one responsive, visible, maximized Syndocal
window (`showCmd=3`). First-party warnings are `0` (only the existing Vite
large-chunk advisory); physical USB-DMX, Art-Net/Unity, fixture, camera/MiraBox,
Spout, and audio acceptance remain explicitly unverified external rows.

The pinned native test gate also passed both blackout regressions:
`native_display_safety_blackout_engage_release_cycle_rejects_stale_authority`
and `managed_blackout_release_projection_drops_the_stale_public_generation`,
each `1 passed / 0 failed` with `1666` filtered tests. This validates the
stale-authority fence path, not physical fixture illumination.

The current alpha.68 Engine physical serial smoke then ran against the
connected FTDI `COM3` device with the pinned Community MSVC 14.44 linker:
`SYNDOCAL_PHYSICAL_MASTER=10`, `SYNDOCAL_PHYSICAL_SECONDS=15`, and
`physical_serial_rainbow_demo_drives_master_dimmer_and_rgb_cells` returned
`1 passed / 0 failed` in `15.02s`, with `439` successful sends, no serial send
failure, and preview RGB `(54,73,255)`. This proves Engine-to-COM writes only;
fixture illumination and downstream DMX reception remain separate physical
checks.

## Historical 2026-09-03 alpha.67 ASIO setup race source checkpoint

ASIO setup requests are invalidated before publishing Normal, preventing a
late ASIO response from overwriting the operator's selected Normal view or
returning it to Locked. Blackout clearing still recovers the canonical local
Both output lease exactly once when the safety latch has no active lease. The
audio-output controller checker, output-control checker, TypeScript, Node
syntax, release metadata, and scoped diff checks pass with first-party
warnings `0`. The exact pinned Community MSVC `14.44.35207` no-bundle native
build passed in `3m21s`; `target/release/syndocal.exe` is Product/FileVersion
`1.2.0-alpha.67`, SHA-256
`3D7BDF45B25189FD9F61694F7523C00125AB71A02E0622D222A2A0D113454CDE`.
PID `65324` supplied exactly one responsive, visible, maximized Syndocal
window. Physical USB-DMX, Art-Net/Unity, fixture, camera/MiraBox, Spout, and
audio acceptance remain explicitly unverified external rows.

The current alpha.67 Engine physical serial smoke then ran against the
connected FTDI `COM3` device with the pinned Community MSVC 14.44 linker:
`SYNDOCAL_PHYSICAL_MASTER=10`, `SYNDOCAL_PHYSICAL_SECONDS=15`, and
`physical_serial_rainbow_demo_drives_master_dimmer_and_rgb_cells` returned
`1 passed / 0 failed` in `15.02s`, with `439` successful sends, no serial
send failure, and preview RGB `(54,73,255)`. This is Engine-to-COM evidence
only; fixture illumination and downstream DMX reception remain separate
physical checks.

## Historical 2026-09-02 alpha.66 DMX blackout release source checkpoint

Blackout clearing now recovers the canonical local Both output lease exactly
once when the safety latch is engaged but no active lease remains after a
restart or expiry. Existing active Both leases release directly; active split,
foreign, or ambiguous ownership fails closed with an actionable Setup → I/O
message, and an already-clear request is a no-op. The output-control checker,
TypeScript, Node syntax, and scoped diff checks pass with first-party warnings
`0`. The exact pinned Community MSVC `14.44.35207` no-bundle native build
passed in `3m28s`; `target/release/syndocal.exe` is Product/FileVersion
`1.2.0-alpha.66`, `63,515,136` bytes, SHA-256
`E8A7C67137501BF2FF41F003587CE1B256E95D0D53BD74FE9E2F7604A71B331F`.
PID `54896` supplied exactly one responsive, visible, maximized Syndocal
window (`showCmd=3`). Physical USB-DMX, Art-Net/Unity, fixture, camera/MiraBox,
Spout, and audio acceptance remain explicitly unverified external rows.

## Historical 2026-09-02 alpha.65 DMX operator-copy source checkpoint

The Setup → I/O → DMX routine surface now keeps only the Show DMX action and
logical route rows in view. DMX route facts replace the long protocol and
safety prose; the compact table appears only when requested. Long
protocol/safety explanations and raw diagnostic detail no longer occupy the
operator surface and are recorded in application logs.
The closed Diagnostics disclosure still exposes the explicit maintenance
actions required by the existing acceptance harness, without changing their
backend fences or fail-closed rules.

Focused DMX contracts, output-control runtime checks, TypeScript, Node syntax,
and scoped diff checks pass with first-party warnings `0`. The exact pinned
Community MSVC `14.44.35207` no-bundle build passed in `3m23s`; the resulting
`target/release/syndocal.exe` is Product/FileVersion `1.2.0-alpha.65`,
`63,520,256` bytes, SHA-256
`8068D4C80F2E8A9A5C29CFBB15D79858A14ECA47DF74954CC24D5383D3B1BE05`.
PID `90464` supplied exactly one responsive, visible, maximized Syndocal
window (`showCmd=3`). Physical USB-DMX, Art-Net/Unity, fixture,
camera/MiraBox, Spout, and audio acceptance remain explicitly unverified
external rows.

Current alpha.65 COM3 smoke: the pinned MSVC 14.44 Engine test with
`SYNDOCAL_PHYSICAL_MASTER=10` and `SYNDOCAL_PHYSICAL_SECONDS=15` returned
`1 passed / 0 failed` in `15.02s`, with no serial send failure, at least `439`
successful sends, and non-zero cell preview RGB `(54,73,255)`. Keep this as
Engine-to-COM evidence; do not promote it to fixture illumination or wire
reception acceptance without an operator observation.

## 2026-09-02 alpha.64 fixture-count source checkpoint

Alpha.64 advances the product prerelease ordinal for the fixture-pack count
correction. Verified and bundled Patch section headers now count fixture
families, and Cached / offline counts fixture entries; selectable Mode counts
remain on individual fixture rows. The current viewport gate follows the
merged Setup → Lighting → Patch browser and removes the stale Library-panel
assumption. Source parent is the upstream-equal alpha.63 checkpoint
`5503913be77a8bf665a886f4ff9c898165b20fa0`.

Focused checker (`10` assertions), fixture-catalog helpers (`69` assertions),
all five Patch browser viewports, TypeScript, Node syntax, and scoped diff
checks pass with first-party warnings `0`. The exact pinned Community MSVC
`14.44.35207` no-bundle native build passed in `3m26s` with first-party
warnings `0` (only the existing Vite large-chunk advisory). The resulting
`target/release/syndocal.exe` is Product/FileVersion `1.2.0-alpha.64`,
`63,524,352` bytes, SHA-256
`F5E039B902BE4F8F0FD7AB17CE4CFF8FA88230268E5CE38FACA86FE6C4C03F6E`.
PID `38416` supplied exactly one responsive, visible, maximized Syndocal
window (`showCmd=3`). Physical USB-DMX, Art-Net, fixture illumination,
camera/MiraBox, Spout/Unity, and audible output remain explicitly unverified
external rows.

## 2026-09-02 alpha.63 DMX operator-surface source checkpoint

The normal DMX setup surface now keeps protocol and safety prose closed by
default. `Protocol & safety details` and `Individual DMX diagnostics` remain
explicit disclosures for maintenance and acceptance work; the routine path
shows the Show DMX action, current Art-Net/USB-DMX route state, and concise
status only. The backend safety fences, exact device selection, one-shot probe
rules, and debug logging are unchanged. This source checkpoint is based on
upstream-equal parent `08f700b51b2ffc5917796dbf6a5ee4370ce7499e`.

Focused UI contracts and TypeScript pass with first-party warnings `0`.
The exact pinned Community MSVC `14.44.35207` no-bundle native build passed in
`3m47s` with first-party warnings `0` (only the existing Vite large-chunk
advisory). `target/release/syndocal.exe` is Product/FileVersion
`1.2.0-alpha.63`, `63,520,256` bytes, SHA-256
`4263754F08529FBF509C631DBACF4D48A179F54707B94853B03FF851E379DD72`.
PID `66028` provided exactly one responsive, visible, maximized Syndocal
window (`showCmd=3`). Physical USB-DMX, Art-Net receiver, and fixture
illumination remain separate hardware acceptance rows.

## 2026-09-02 alpha.62 camera 4K/60 source checkpoint

The camera catalog now admits advertised high-resolution profiles through
source parent `4d8fb69fb67169bf1032c45260355b87aa219476` at the current
upstream-equal checkpoint and through
`4096x2160` at `60 fps`, while keeping `4K/120` fail-closed and retaining
device-advertised 120fps capture for bounded lower resolutions. The one-frame
probe remains mandatory and presentation remains capped at `60 Hz`. This
checkpoint does not claim sustained 4K performance or physical camera output.
The exact pinned Community MSVC `14.44.35207` no-bundle build passed in
`2m48s` with first-party warnings `0` (only the existing Vite large-chunk
advisory). `target/release/syndocal.exe` is Product/FileVersion
`1.2.0-alpha.62`, `63,520,256` bytes, SHA-256
`F5477B506E9867DACE350E717CA7759B3CE8883D04A1DF22E77485F4A96FFA04`.
The executable was then launched from this checkout as PID `82876`; exactly
one matching `Syndocal` process was responsive and visible with a maximized
main window (`showCmd=3`). USB-DMX, camera/MiraBox, Spout/Unity, and physical
display pixels remain unverified.

## 2026-09-02 alpha.61 native/managed-output checkpoint

Alpha.61 is the current native candidate, based on upstream-equal parent
`d34a6011ff1c768f2f3a75bd8ec55900403da21b` on `codex/syndocal-v1.2`.
managed-output focused `28/0/0` passed with first-party warnings `0`; the
USB-DMX route remains an explicitly separate runtime and hardware acceptance
row. This top boundary is the release-authority marker; the newer native
Timeline, three-display, and Open-DMX evidence below records the current
operator/runtime state.

## 2026-09-02 normal ASIO availability checkpoint (native complete)

The normal Windows application now includes the ASIO loader and command surface
in its default feature set: `libav`, `spout`, and `asio`. Setup → I/O → Audio
therefore exposes a normal `ASIO` backend alongside Normal WASAPI; selecting it
still requires an explicit driver, sample rate, buffer, channel routing, and
Revalidate/Start sequence. No driver is selected automatically. The UI now
labels the backend simply `ASIO`, and a missing optional bridge has an explicit
Return-to-Normal recovery path instead of leaving the operator trapped in a
Locked view.

The SDK-derived `syndocal_asio_bridge.dll` remains an explicit same-host QA
payload only (`qa/ASIO_SDK_PIN.json` keeps `distribution_approved: false`) and
is not part of the approved normal installer/updater payload. The setup-only
return path is also explicit: when ASIO is selected but the native router is
still `Normal`, the Return-to-Normal action is enabled and resets only the
local view, without dispatching a redundant native Normal-selection command.

## 2026-09-02 alpha.61 normal-UI diagnostics boundary (source checkpoint)

The normal Audio setup surface no longer renders ASIO preflight, solo, or
bounded test-tone controls. Those controls were diagnostic operations, not
ordinary output configuration, so they are retained only as backend Tauri
commands for QA/maintenance and are never wired into the production
`AudioOutputPanel`. Backend invocations now emit a compact `[audio-output][debug]`
request/result record to stderr; no test control or diagnostic disclosure is
shown in the normal UI. PROGRAM/CUE routing, device selection, lifecycle state,
and explicit Return-to-Normal remain ordinary setup controls.

The diagnostic-boundary source parent was commit `c528aa2` on
`codex/syndocal-v1.2`. The exact pinned Community MSVC 14.44 no-bundle native
build for that parent passed in `2m25s`;
the release contract chain, TypeScript, focused audio checker, and fixed-linker
Cargo check/test are green. The final executable/runtime evidence is recorded
below. That parent evidence is historical; the alpha.61 build evidence is
recorded below and is not a release tag.

After the documentation repair, `pnpm --dir app run check:release` passed again
with first-party warnings `0` (release metadata, ASIO packaging `169`, ASIO v3
`22`, Timeline audio/loop/watermark/bootstrap, video routing/window observation,
and camera contracts). The matching `check:release:self-test` also passed:
release metadata `125`, Windows artifact self-test `140`, and strict JSON `130`;
the unapproved bridge was staged outside `target/release` for the packaging
checks and restored with its original SHA-256.

The alpha.61 native candidate then rebuilt after the diagnostics-boundary,
fixture-pack count, and Setup Lighting filter-clarity changes. The pinned
Community MSVC `14.44.35207` no-bundle build passed in `2m00s` with first-party
warnings `0` (only the existing Vite large-chunk advisory). The artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.61`, `63,520,256` bytes, SHA-256
`872ED7DDCE00935DB8E13197966F97526A5DCB28D6074CFFA91CC39E5C4117C4`.
It is running as PID `84596`; exactly one exact-checkout process is responsive
and maximized (`showCmd=3`). The verified fixture-pack header now reports
unique fixture-family count rather than summed mode count. Setup Lighting's
fixture-type strip now says `All Types` and no longer renders the ambiguous
red/white four-block glyph; the strip remains a filter independent of the
current fixture selection, with explicit accessible label and tooltip. This is
native build/window evidence, not physical USB-DMX, audible, pixel, camera,
DJ-Link, or pedal acceptance.

## 2026-09-02 alpha.61 Setup Lighting empty-selection clarity

The small red/white four-block mark shown in Setup → Lighting when no fixture
is picked was the old visual glyph for the `All Types` fixture-type filter. It
was not a fixture state, DMX state, or an implicit fixture selection. The
normal filter now renders the explicit `All Types` label, keeps the filtered
fixture count, adds `aria-label="All fixture types"` and a tooltip, and removes
the ambiguous glyph in both active and legacy selection surfaces. Type-specific
filter buttons retain their existing behavior and now have explicit button
semantics as well.

Focused validation passed: `node app/scripts/check-mapping-empty-context.mjs`,
`pnpm --dir app exec tsc --noEmit`, the audio panel/control checkers, GDTF
profile-action checker, and `git diff --check` (exit `0`; only Git's expected
LF→CRLF notices). The full `pnpm --dir app run check:release` chain also
passed: ASIO packaging `169`, ASIO v3 `22`, Timeline audio/loop/watermark/
bootstrap, video routing/window observation, and camera contracts. The
unapproved ASIO bridge was staged outside `target/release` and restored with
unchanged SHA-256 `06132C2CC8C54F7D89947F8BB1E5320226986BCD8155069AFEC034C6E841B9AC`.

## 2026-09-02 normal ASIO native/runtime evidence

The native artifact built from the source content of `c528aa2` is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
`63,518,208` bytes, SHA-256
`21588CF7F603792D026DF7041863DD02000C9136EC8A0CA81AF66ECE999E08A4`.
The same-host QA bridge beside it is `926,208` bytes, SHA-256
`06132C2CC8C54F7D89947F8BB1E5320226986BCD8155069AFEC034C6E841B9AC`;
it is deliberately not an approved normal package payload.

The final executable was launched from the exact checkout path. Exactly one
responsive `Syndocal` process is running (PID `55688`), with the main window
maximized. On the native UI, Setup → I/O → Audio exposed the `ASIO` backend,
enumerated 8 drivers, and selected `TOPPING Pro USB Audio Device`. Native
status was schema/ABI 3, backend `asio-sdk-v3-rt`, `state=ready`,
`routerState=Normal`, `catalogGeneration=1`, `profileReady=true`. Capabilities
were 6 output channels, native `i32`, rates `44.1/48/88.2/96/176.4/192 kHz`,
and fixed buffers `8..2048` with preferred `128` frames. Return-to-Normal was
enabled in this setup-only Normal-router state; after activation it restored
`normal-wasapi` / `Ready`, and the native status remained `ready` / `Normal`.

This proves the normal binary's ASIO loader/command surface, explicit driver
enumeration, capabilities, and recoverable setup UI. It does not claim a
bundled licensed bridge, audible output, long-soak/XRUN or unplug recovery,
USB-DMX fixture illumination, Art-Net/Spout/Unity receiver pixels, MiraBox
frames, or DJ-Link/pedal ACKs. Those remain open external acceptance rows.

## 2026-09-02 alpha.60 WDM audio device enumeration and selection

Setup → I/O → Audio was opened on the running process. The Timeline authoring
route enumerated the current Windows WDM catalogue (15 endpoints, including
`Music (Elgato Virtual Audio)`, the E2x2 Playback pairs, `スピーカー (2-
TOPPING USB DAC)`, and the JBL device) rather than relying on a fixed device
name. Selecting the exact `Music (Elgato Virtual Audio)` endpoint and pressing
the explicit Windows-output refresh left the route at `実行中` with no alert;
the UI retained the exact endpoint in the device select. A short Timeline
play/pause from `0 ms` advanced to `1177 ms` and paused at `1266 ms` while the
route remained configured. This proves enumeration, exact selection, and
native route lifecycle only; actual audible delivery and the operator's
headphone level remain a separate hardware acceptance row.

## 2026-09-02 alpha.60 current native Timeline playback acceptance

The already-running exact alpha.60 process was exercised through the native
Edit → Timeline surface with the authored
`target\\qa\\DSF2026-show-alpha55-dual-file-display.sdc` project. The surface
reported two authored lanes (`Reference Audio` and `Lighting`), one
`PROGRAM · Audio Clip` spanning `0–214032 ms`, and two lighting scene blocks
(`all_white` and `all_max`, each `138353 ms` start / `81153 ms` block). The
timeline also reported the enabled loop region `136941–138353 ms`.

At the current process/PID `68032`, Play advanced the native scrubber from
`66254 ms` to `67357 ms` in approximately one second while the live status
reported `タイムライン再生中`, `照明 · 2 シーン`, `灯体46`, `DMX経路1 / 1`, and
`映像出力2 / 2`. Pause then settled at `67445 ms` and the status changed to
`タイムライン停止`. A bounded loop probe sought to `136700 ms`; samples
`137330 → 138148 → 137818 ms` demonstrated wrap inside the enabled loop
region, then Pause settled at `137862 ms`. A natural-completion probe sought
to `218900 ms`; the scrubber reached the exact `219506 ms` duration and stayed
there with `タイムライン停止` across subsequent samples, with no automatic
next-track start observed. This is current native UI/runtime evidence for
transport, loop wrap, authored scene presence, and natural stop; it does not
claim audible endpoint delivery, physical fixture response, display pixels,
MiraBox frames, Unity/Spout, or DJ-Link/pedal ACKs.

## 2026-09-02 alpha.60 native three-display window acceptance

With the current clean checkout and the rebuilt alpha.60 executable, the
read-only/apply acceptance runner returned `accepted=true` after three
consecutive stable samples. Evidence is retained at
`target\\qa\\alpha60-three-display-current-aef9067c38c940b382c4fea897cd4941`.
The exact artifact was `1.2.0-alpha.60`, `62,486,528` bytes,
SHA-256 `D31AB72AA54800BA93AA8305F467CD084979823885887F17535C65D5F842F0A3`;
the running process was PID `68032` from the exact checkout path.

The verified three-screen topology is:

- editor `Syndocal` on stable identity `\\?\\DISPLAY#PXO2500#5&eb37e8d&1&UID4355#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` / `DISPLAY2`, client `1920x1032`, effective DPI `96`, maximized;
- output `3` / `Display 1` on stable identity `\\?\\DISPLAY#PXO1560#5&2c959af3&0&UID768#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` / `DISPLAY5`, exact physical client `1920x1080`, effective DPI `144`, title `Syndocal Output - Display 1`;
- output `4` / `Display 5` on stable identity `\\?\\DISPLAY#MSI3DD2#5&eb37e8d&1&UID4357#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` / `DISPLAY3`, exact physical client `3840x2160`, effective DPI `144`, title `Syndocal Output - Display 5`.

All three windows were visible, responsive, owned by PID `68032`, and matched
their explicit output IDs/titles and monitor identities in every sample. This
accepts real native window creation, placement, and geometry for the editor +
two Display outputs. The harness deliberately reports
`native_hardware_claim=false`: it does not sample pixels, decoder quality, a
60-fps budget, MiraBox frames, Unity/Spout delivery, or audience-facing
content. Those remain separate external acceptance rows. The current process
is left with both Display windows open and the USB-DMX worker in safe S0.

A subsequent read-only strict sample after the documentation checkpoint also
accepted the same topology with current clean HEAD
`a7e3d249936ea714487dd1509c79ae6c0c819e4b`, exact artifact hash/size/version,
PID `68032`, maximized editor, and the two app-owned output HWNDs. No output
window was replaced or duplicated.

The final read-only strict sample was rerun against the current clean HEAD
`f1187143b5884ca60618d9157a73ad56fbc0ca67` after the documentation-only
checkpoint commits. It returned `accepted=true` with the same artifact
`1.2.0-alpha.60`, PID `68032`, editor/output identities, and output IDs `3`
and `4`. Evidence is retained at
`target\\qa\\alpha60-three-display-current-head-f1187143-c2e61a9d4229447baed19c2594fbbb4e`.
The harness still reports `native_hardware_claim=false`; this is a current
HEAD/window topology proof only and does not add pixel, 60-fps, MiraBox,
Unity/Spout, audible, DJ-Link, pedal, or physical fixture claims.

After the receiver/pixel spot check was recorded, the same strict three-sample
reader was rerun against clean HEAD `1396352a777f8be28cb443c33b4f3c7d7fc1393c`
with the same exact alpha.60 artifact and stable monitor identities. It
returned `accepted=true`; evidence is retained at
`target\\qa\\alpha60-current-head-1396352-df05d7ee018f4e598d98d2e4aed4aeda`.
The editor was maximized and exactly the two app-owned output windows (IDs 3
and 4) remained visible/responsive. This refresh is still topology/window
evidence only and does not change the native-hardware claim boundary.

## 2026-09-02 alpha.60 post-build Open-DMX barrier and restart checkpoint

The final source adjustment closes a late-cleanup race in the bounded Open-DMX
sender open. The process-wide single-flight barrier now remains set until a
timed-out worker's result has been received and, when a sender exists, its
bounded shutdown has completed successfully. A timely result releases the
barrier in the caller; a late constructor error releases it only after the
reaper observes that error; a disconnected worker or cleanup failure keeps the
barrier latched and requires process restart. No second driver open, fallback
sender, or permissive retry is allowed. The regression test also attempts a
second open while the first late open is still pending and requires the
single-flight rejection before allowing a post-reaper retry.

Focused verification after this adjustment passed: exact Community MSVC
14.44 `show_serial_dmx_tests` `21 passed / 0 failed / 0 ignored` with
first-party warnings `0`, `cargo fmt --all -- --check`, TypeScript, and the
full `check:release` contract chain (ASIO packaging `169` assertions, ASIO v3
`22`, timeline/audio/video/camera contracts). The fixed-linker no-bundle
build passed in `2m32s`; only the existing Vite large-chunk advisory was
emitted. Artifact:
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.60`, `62,486,528` bytes, SHA-256
`D31AB72AA54800BA93AA8305F467CD084979823885887F17535C65D5F842F0A3`.

The rebuilt executable was relaunched once with the authored
`target\qa\DSF2026-show-alpha55-dual-file-display.sdc` project. Exactly one
exact-checkout `Syndocal` process (PID `68032`) was responsive and its main
window was maximized. The persisted FTDI Open-DMX binding was
`selected_and_present` on `USB Serial Port (COM3)` with the exact Windows
device instance identity retained.

The first post-restart one-click Prepare attempt was rejected at the final
stage with `forbidden`; no output was applied. After refreshing the same
binding/route state, one deliberate retry and its explicit native safety
confirmation completed. Four one-second status samples then remained stable:
`active=true`, `zeroFrameQueued=true`,
`zeroFramePhysicalWriteCompleted=true`, `faulted=false`, and
`artnetMirrorLive=true` (route status revision `3`). The worker is intentionally
left in S0 with the latest all-zero frame queued; this proves native-to-Windows
Open-DMX physical-write/queue state only, not fixture illumination, cable
continuity, or audience output. The current external acceptance rows remain
USB fixture visual response, display pixel/60fps, MiraBox live content, Unity
Art-Net/Spout delivery, audible PROGRAM/CUE, DJ-Link/pedal ACKs, and physical
ASIO routing.

This source/doc checkpoint is committed and pushed on
`codex/syndocal-v1.2` (verify `HEAD == origin/codex/syndocal-v1.2` before any
new mutation). It is not a release tag. The running process is retained in
safe S0 for operator Timeline work; the external acceptance rows above remain
deliberately open until their physical or receiver-side evidence is captured.

## 2026-09-02 alpha.60 same-PC Art-Net receiver and two-output pixel spot check

With the running alpha.60 process still in safe S0, the repository-owned
external monitor captured the current Timeline route at
`target\\qa\\artnet-alpha60-playback-probe-20260902.json`. The bounded
12-second capture accepted `528` ArtDmx datagrams, rejected `0`, and recorded
`528` U0 frames with a `512`-byte DMX payload, protocol version `14`, and a
maximum inter-frame gap of `24 ms` (about `44 fps`). The first accepted raw
packet has the canonical `Art-Net\\0` / `0x5000` header, wire universe `0`,
and a `512`-byte payload. All payload bytes, including channel 500, were zero
because S0 remained engaged. This closes current same-PC UDP receiver,
universe/length/period, and safe-zero mirror evidence only; it does not claim
non-zero Art-Net, Unity delivery, or fixture response.

Read-only screen captures of the two already-open output windows are retained
at `target\\qa\\display1-alpha60-20260902.png` (1280x720,
SHA-256 `FFDFCA3A438AA3A1E2B80BB1EF589D13B6956B7243316D16DA995A992C84BEF6`)
and `target\\qa\\display5-alpha60-20260902.png` (2560x1440,
SHA-256 `008F6459434582A2B7FB92C05DD67DD48D20A6D90E95F76C6BB0C624F23F99B7`).
The captures have different pixel content (Display 1 is predominantly black;
Display 5 is a four-colour test composition), proving independent visible
content on the two native output windows. The captures are a content spot
check, not a 60-fps or audience-pixel acceptance measurement.

## 2026-09-02 alpha.60 camera enumeration boundary

The native Edit -> Video -> camera import surface was exercised against the
current machine. Device refresh failed closed with a bounded FFmpeg/DirectShow
timeout while enumerating `Insta360 Link`; no camera profile was exposed after
the failed catalog refresh. `OBS Studio` was running during this attempt and
may own one or more capture endpoints. A fresh PnP inventory does contain
`MiraBox Video Capture` (USB VID `1BCF`, PID `2C99`), alongside `NDI Webcam
Video` and `Insta360 Link`. No camera process was force-terminated and no false
profile or 4K/60 acceptance was recorded. Retry after the camera provider is
released and the intended capture device is connected; then select an exact
advertised profile and require one complete-frame probe.

An independent DirectShow option listing for the present `MiraBox Video
Capture` succeeded. Its advertised maximum is `1920x1080` at `60.0002 fps`;
it advertises no 3840x2160/4K mode. The product can accept higher-resolution
profiles from a different capture device within its bounded 4096x2160/120-fps
catalog policy, but this particular MiraBox cannot provide 4K. With the HDMI
source disconnected, a one-frame 1080p60 probe did not complete within the
bounded command window and was terminated; no frame-quality claim is made.

## 2026-09-02 alpha.60 Windows delivery self-tests

The SDK-independent release chain was rerun on clean HEAD `d8c6b7b`:
`pnpm --dir app run check:release` passed (ASIO packaging `169`, ASIO v3 `22`,
Timeline/audio/video/camera contracts), with the documented `FFMPEG_DIR is not
set` informational line and zero first-party warnings. The deterministic
Windows delivery checks also passed: `windows-candidate-extractor.mjs
--self-test` (`43` assertions), its verified-materialization self-test (`4`),
and `check-windows-release-artifacts.mjs --self-test` (`140`). These are
packaging-boundary proofs only. A real NSIS/MSI/updater candidate was not built
because the current product is an alpha prerelease and the signed release
manifest/key inputs are intentionally absent; no installer or updater release
claim follows from these self-tests.

## Detailed alpha.60 native/managed-output evidence (superseded source snapshot)

Alpha.60 is the current native candidate, based on upstream-equal parent
`9ae8276cc6fbf807b3517770f22ee788e050f85f` on
`codex/syndocal-v1.2`. The current exact alpha.59 process reproduced a narrow
post-confirmation race: the keepalive manager advanced the healthy exact Both
lease from generation `6` to `8` (and later `15`), while blackout release and
existing Display reopen submitted the older UI generation. Both rejected
before physical publication with `StaleGeneration`; the USB route remained in
safe S0 zero output, `faulted=false`.

The source fix recognizes only those two short routes after confirmation.
Exact manager-owned Both authority uses the existing private serialized
authorization; blackout release holds the guard through the bounded Engine S0
callback, while Display reopen drops it immediately after durable registry
authorization and before native window/GPU work. Add Display and every
unrelated route are unchanged. Faulted, foreign, wrong-resource,
wrong-project, and stale-fence states cannot fall back to ordinary authority.

Exact Community MSVC `14.44.35207` full-feature managed-output focused `28/0/0`
passes with first-party warnings `0`; formatter and scoped diff gates
pass. The local-only alpha55 two-video specimen is also exact: it derives from
alpha54 `1,112,448` bytes / SHA-256
`D20A7891C1D2DBB6E1E41BA1291C4F1D9C13E348237ABD16F20FF059CA3D1347`,
changes exactly `16` paths, preserves two Display outputs and three
compositions, and publishes `1,112,438` bytes / SHA-256
`F57A98966CD58FE040D2AC3C2A5B41D7CA53F15BE2A6C5A9C117EC918A574459`.
Its separate 3840x2160 media file is `43,807,726` bytes / SHA-256
`D51DCD2B4F55CB7A6FAE3F34E9B4D93E6BAC24B0B29A6BE500AB4B848D323E29`.
Absolute paths keep this a same-PC QA artifact, not a portable release bundle;
USB-DMX remains a runtime-only acceptance row.

The alpha.60 checkpoint `140bebcabeec4f574fa37bc30e03e1b294ecbc32` is committed,
pushed, and upstream-equal. The exact Community MSVC `14.44.35207` pinned
no-bundle build passed in `1m48s` with first-party warnings `0`; only the
existing Vite large-chunk advisory was emitted. Artifact:
`target/release/syndocal.exe`, Product/FileVersion `1.2.0-alpha.60`,
`62,435,840` bytes, SHA-256
`29045BC40227F823E0A2259113E2BECC246B665AEC161E5A14111D9CB0E4EA1C`.
It is running with `target/qa/DSF2026-show-alpha55-dual-file-display.sdc` as
PID `53420`; this is the sole exact-checkout process, responsive, with the
main window maximized (`showCmd=3`). Read-only enumeration found FTDI
Open-DMX `COM3`, `Music (Elgato Virtual Audio)`, four monitor modes, and an
`Unknown MiraBox Video Capture` entry.

The current alpha.60 engine physical serial demo then ran against `COM3` with
the pinned Community MSVC 14.44 linker (`master=10`, `seconds=15`):
`1 passed / 0 failed` in `15.02s`, no serial send failure, and at least `439`
successful sends observed. This is engine-to-COM write evidence only; fixture
illumination still requires an operator visual check.

The current alpha.60 native Timeline smoke then passed under the same pinned
MSVC linker and explicit WinGet FFmpeg root: the automation/event route test
returned `2 passed / 0 failed`, and the Art-Net loopback cue route returned
`1 passed / 0 failed`. This is deterministic native/runtime route evidence;
it does not establish a physical fixture, audible endpoint, display pixels, or
Unity receiver.
The companion Engine `show_serial_dmx` protection suite returned `23 passed / 0
failed`, covering S0-first writes, the shared 512-channel Art-Net/USB buffer,
and fail-stop/recovery invariants.

These process/window, device-presence, and Timeline-smoke observations are not
signal or pixel acceptance. One bounded native/operator pass still must run the
show-DMX action and S0 release, confirm continuous USB-DMX, open both Display
outputs and collect their current 600-frame windows, exercise Timeline loop/
release/Follow in the UI, and read the selected PROGRAM/CUE endpoint. MiraBox
live content, Unity Art-Net/Spout, DJ-Link/pedal ACKs, show-ASIO physical
routing, and installer/updater inspection remain explicitly unverified.

The StandardRelease three-display observation harness is pinned to the same
alpha.60 artifact/source identity; its deterministic seam suite passes `96/0`.
It does not create outputs or claim signal/pixel success and cannot perform a
real display acceptance until the app-owned loopback CDP output observation
and three explicit monitor identities are available.

Source gates now recorded: full-feature managed `28/28`, no-default managed
`28/28`, control-plane inventory `28/28`, TypeScript, release metadata, and
alpha54/alpha55 derivation checks all pass. The full Rust suite is
`1415/1/14` (pass/fail/ignored); its only failure is the external local
`DSF2026.dvc` `4chPar`/`Dimmer` mismatch, retained as fail-closed evidence.
The regenerable Cargo dev profile was cleaned after a `60,082`-file/
`151.2 GiB` dry-run; release and QA artifacts were not removed. The next safe
action is one bounded operator/native acceptance pass; no hardware or pixel
claim may be inferred from the process and device-presence evidence above.

## 2026-09-02 alpha.60 bounded Open-DMX completion pass

The source checkpoint began at upstream-equal `8bdd72417330e9ad3abb68a09125d383d8f25ba2`
on `codex/syndocal-v1.2`. The show-critical change adds a bounded, process-wide
single-flight around the Windows Open-DMX sender constructor. PnP/HANDLE and
driver calls now have a five-second caller deadline; a late constructor result
is owned by a bounded cleanup reaper, S0 remains engaged, and a second open is
refused until the late result returns (or the process is restarted). No fallback
sender or permissive retry is introduced. The previous path could hold the
engine activation/UI indefinitely inside an uncooperative driver call; the new
path fails closed with visible S0-retaining error text. The deterministic
regression is
`bounded_show_serial_dmx_sender_open_times_out_and_blocks_stacked_opens`.

Final source gates for this change: `cargo fmt --all -- --check` PASS;
`pnpm --dir app exec tsc --noEmit` PASS; `pnpm --dir app run check:release` PASS
(ASIO packaging 169 assertions, ASIO v3 22 assertions, timeline/audio/video/
camera contracts); exact Community MSVC `14.44.35207` engine
`show_serial_dmx_tests` PASS `21/0/0` with first-party warnings `0`;
`node --check` and `git diff --check` pass (Git only reports LF-to-CRLF
conversion notices).

The final exact native build used the required Community linker as the first
`where.exe link.exe` result and passed
`pnpm --dir app tauri build --no-bundle` in `2m30s`, with first-party Rust
warnings `0` and only the existing Vite large-chunk advisory. Artifact:
`target/release/syndocal.exe`, Product/FileVersion `1.2.0-alpha.60`,
`62,486,528` bytes, SHA-256
`2AA951CD29772B42B267C1A8512DCE2D3BC09692067C2004889398D342CE95A2`.
The exact checkout process was relaunched once after the build; exactly one
responsive, maximized `Syndocal` window was observed.

Final native/operator observation on the same machine: persisted FTDI
Open-DMX binding `COM3` was `selected_and_present`. The one-click **Prepare
show DMX** flow completed after its explicit OS confirmation. It reported
`active=true`, `zeroFrameQueued=true`,
`zeroFramePhysicalWriteCompleted=true`, `faulted=false`, and
`artnetMirrorLive=true`; after the explicit DMX blackout release, four
one-second samples remained `active=true`, `liveFrameQueued=true`,
`zeroFramePhysicalWriteCompleted=true`, `faulted=false`, and
`artnetMirrorLive=true`. This is native-to-Windows Open-DMX physical-write and
continuous queue evidence; it is not a fixture, cable, or audience-output
claim. The release/prepare control intentionally retains one explicit safety
confirmation so an unattended click cannot change live output.

The same final binary was restarted once more and the complete prepare path
was repeated: `selected_and_present`/COM3, S0 zero receipt complete, active
worker, then S0 release with live-frame queueing. No native diagnostic logs
remain in source. The final runtime was left with the USB worker active and
S0 clear so the operator can continue Timeline work. Display pixel/60fps,
MiraBox live content, Unity receiver delivery, audible PROGRAM/CUE endpoint,
DJ-Link/pedal ACKs, and show-ASIO physical routing remain external acceptance
rows; prior local display-routing evidence does not promote those claims.

Checkpoint commit `32b8c78d4729c17c5fd570b6e7ef9e61b10f5509` is committed and
pushed to `origin/codex/syndocal-v1.2`; HEAD and upstream are equal. The
post-push tree contains no owned dirty files. The next safe action is ordinary
operator Timeline work; do not infer the explicitly unverified external rows
above from the native route evidence.

## 2026-09-01 alpha.59 native and local hardware checkpoint

Source checkpoint `9f1925cf98fd36dbdb8059a2b50143feb300f641` is
committed, pushed, and upstream-equal on `codex/syndocal-v1.2`. The exact
Community MSVC `14.44.35207` no-bundle build passed in `2m55s` with first-party
warnings `0`. The artifact is
`target/release/syndocal.exe`, Product/FileVersion `1.2.0-alpha.59`,
`62,367,744` bytes, SHA-256
`EF18B7A92913B9BA72A74CAA93864106DC834065135E8703542851C11ED7E2F7`.
PID `56736` is the sole exact-checkout process. Its main `Syndocal` window is
responsive and maximized (`showCmd=3`, native outer rect
`-8,-8,1928,1040`).

The one-click show-DMX flow completed against the current FTDI Open-DMX
adapter on `COM3`. Authoritative machine binding is
`selected_and_present`; initial S0 status proved `active=true`,
`zeroFrameQueued=true`, `zeroFramePhysicalWriteCompleted=true`,
`workerShutdownCompleted=false`, and `faulted=false`. After explicit native
Yes confirmation for blackout release, the same worker reported
`active=true`, `liveFrameQueued=true`, `faulted=false`, and
`artnetMirrorLive=true`; the engine preview carried the completed all-white
frame (including ch1-3 and ch5 at 255). This proves application-to-Windows
Open-DMX physical-write completion and continuous live-frame queueing. Fixture
illumination is still an operator visual acceptance row until the operator
confirms the attached luminaire remained lit.

The authored `人生オーバー` Timeline was exercised through its first pedal
boundary. Starting at `137000ms` remained inside the configured
`136941..138353ms` four-beat loop (observed `137255ms` after more than one loop
duration). The normal Break/loop-release control then advanced beyond B to
`153986ms`; USB-DMX remained active/live/non-faulted throughout. Playback was
paused and returned to `0ms`; the current safe operator state is Timeline
stopped at the start, S0 clear, and live USB-DMX active.

Both persisted ordinary Display outputs were opened concurrently under exact
Ready/Both ownership. Output 3 is Display 1 at `1920x1080`, composition 2
`Foreground Video 1`, containing file-backed `logo-anim-dark`; output 4 is
Display 5 at `3840x2160`, composition 3 `Background Video2 Camera`, containing
the exact 1920x1080@60 camera source selection. Display 5 passed its current
600-frame validation window (`11.357ms` average, `17.445ms` max, one deadline
miss, decoder errors `0`). Display 1 is live and decodes without errors, but
its current strict 60fps window remains NO-GO: `12.982ms` average,
`35.504ms` max, `180/600` deadline misses, validation `failed`. Thus distinct
two-screen routing and live window creation are accepted locally; full
three-display 60fps acceptance is not.

Cleanup remains fail-closed. Inventory was approximately
`184,901,787,243` bytes under `target` and `545,338,492` bytes under
`app/node_modules`. The tracked cleanup self-test passed `110` assertions, but
the plan rejected with `WriterOwnershipTopologyUnverifiable` for a missing
positive parent PID. No deletion ran and reclaimed bytes are `0`.

Open external rows: operator visual confirmation of the lit USB-DMX fixture;
Display 1 60fps budget repair/retest; audible PROGRAM/CUE confirmation on the
selected real endpoints; MiraBox live-image acceptance; Unity Art-Net/Spout;
DJ-Link real ACK/reconnect/pedals; show-ASIO physical routing; and installer/
updater inspection. Do not promote route state, queue acceptance, or the local
camera selection into those external claims.

## 2026-09-01 alpha.59 source candidate before native acceptance

Alpha.59 is being prepared from upstream-equal parent
`951f377735b742f8353add01926f47aafe1521d9` on
`codex/syndocal-v1.2`. The observed restart failure was not a USB serial or
fixture problem: quick setup sent Arm again while an exact active Both lease
was already being kept alive, and the redundant mutation lost with
`StaleGeneration`. The new path reuses exactly one `held_active` Both lease only
after authoritative Ready/Both ownership verification. A sole unavailable or
orphaned lease uses canonical enable; ambiguous, mixed, multiple, or
wrong-resource state stops before any device or route mutation. There is no
raw retry or permissive fallback.

The foreground video path also gains an exact identity-layer compositor fast
path and a bounded thread-local libav scaler cache; failed contexts are
discarded. Performance reporting no longer lets an old lifetime history stand
in for the current route. It preserves lifetime counters separately and starts
a new validation epoch on output/monitor/route/composition/media/geometry,
authority, seek/transport, loop/Follow, settlement, revoked-present, or render
error. Each epoch requires 60 warmup frames and 600 successful measured frames
before a 60fps result is published.

Source evidence so far: DMX quick-setup checker, Node syntax, and TypeScript
pass; exact Community MSVC 14.44 video proof passes libav `171/0/1 ignored`,
default `163/0/1 ignored`, and metrics focused `5/0/0`; first-party warnings
are `0`. Decoder/compositor and metrics independent Terra xHigh review is GO
with P0/P1 `0`; review caught and closed stale PASS on zero extent,
window-size error non-reporting, and the identity-change warmup off-by-one.
TypeScript and the complete release checker pass. Commit/push, alpha.59 native
build, and physical USB-DMX, audible audio, and three-display 60fps
revalidation remain open. Do not treat this source candidate as native or
hardware acceptance. The next native action must first terminate only the
resolved exact-checkout `target/release/syndocal.exe`, then use the pinned
MSVC 14.44 no-bundle build.

## 2026-09-01 alpha.58 display-authority source candidate

Alpha.58 is the current source candidate on `codex/syndocal-v1.2`, advanced because
alpha.57's first Display-present attempt failed despite exact persisted output
and monitor identities. The root cause was the first-present guard comparing
the full `VideoSnapshot`, treating ordinary playhead/transition progress as an
authority change. Alpha.58 removes only that volatile equality while retaining
exact output identity, ownership/lease, project and safety blackout, paired
configuration-token, and final pre-present token fences.

Exact Community MSVC `14.44.35207` focused proof passes `8/8` for
`native_display_`, warnings `0`; the output-control runtime checker, Node syntax,
formatter, and scoped diff check pass. The regression uses the production
first-frame prepare seam, waits for an actual same-token playhead snapshot
change, and admits exactly one present. Existing semantic output mutation
coverage remains zero-present. Independent Terra xHigh rereview is GO with
P0/P1 `0`; its P2-only notes are a narrower explicit `position_ms` assertion
and the pre-existing final-token-load-to-GPU-call TOCTOU that requires a
separate engine-side presentation permit.
Source checkpoint `a0b9b00e4871739ab3765849c58846d30aee2d9b` produced the exact
alpha.58 artifact in a pinned native build: Product/FileVersion
`1.2.0-alpha.58`, `62,313,472` bytes, SHA-256
`11BB7116E75EDA3B264DD81D94C14DA4040BF027656D11115A8E228AE53DCC31`,
`2m43s`, first-party warnings `0`. PID `8720` exposes one responsive,
maximized main window. Pre-open observations for output 3 and output 4 were
both `live_open=false`; one click opened Display 1/output 3 with stable HWND
`189075418`, then one click opened Display 5/output 4 with stable HWND
`377557514` while output 3 remained live. Three Syndocal windows were present
(main, output 1, output 5). Display 1 visibly rendered the fixed foreground
logo. The `Video Switch Acceptance` Timeline drove Display 5 through Video 2,
the expected black/no-signal MiraBox interval while that device was
disconnected, and Video 2 again at approximately 1s, 5s, and 9s. Background
route switching is proven, but camera content is not. Latest UI status is
Display 1 `Window open / 1920x1080 / 94.3ms / 60 FAIL` and Display 5
`Window open / 3840x2160 / 12.8ms / 60 FAIL`; Display 5 initially reached
`10.4ms / 60 PASS`, then recorded a `1854.23ms` maximum and accumulated late
frames during route switching. Full three-display performance acceptance
remains blocked. Stderr had no
display error, only DJ Link `trust_network_absent`; USB-DMX, audible audio,
MiraBox/camera, and Unity/Spout remain unverified external gates.
Display 1 changed foreground frames after 2.5s, proving animated presentation
while Timeline ran. Timeline transport was paused and restarted for the
background sequence above while the main window remained maximized at
1920x1032.
The clean descendant checkpoint `72e8d99e116ecf35efd4768b4a7254b3937ba096`
passed three consecutive strict monitor/window identity samples. Evidence is
`target/qa/alpha58-live-3display-61f07c8789fc4a0bb1d0a606b119e20d-2368fe4d7ccf437584362a82874c2e9a`;
the verdict is `accepted` with `native_hardware_claim=false`. Do not promote
that window-identity verdict into the still-open performance or physical gates.

## 2026-09-01 historical alpha.57 media-admission native checkpoint

The preceding product metadata was `1.2.0-alpha.57` on `codex/syndocal-v1.2`, based on
upstream-equal parent `73df6c9db759ada763ab36a17fb55576fabf7fed`. The old
Timeline Media Library could show a file-backed asset without proving that its
current machine-local bytes matched the project identity. Alpha.57 adds
localized per-asset and batch Verify actions and routes both click and drag
through one fail-closed admission predicate. Missing, unreadable, hash-mismatched,
unknown, or not-yet-inspected media cannot be placed; no filename or device-name
fallback is accepted.

The native project-load path now hashes file-backed candidate media before the
project mutation locks are taken, then publishes the complete availability set
only under the exact post-ACK video-presentation authority. It stages B
privately, publishes the B snapshot, commits the B map under that snapshot
guard, and only then acknowledges. Publication failure preserves exact A;
duplicate, foreign, missing, or token-mismatched batches are zero-mutation.
Cross-lock access uses one `snapshot -> availability` order. Independent Terra
xHigh review is GO with P0/P1/P2 `0`; review first found and then verified the
closure of early-B exposure and an ABBA deadlock. Final exact MSVC 14.44 engine
proof is focused `7/7` and full `1002 passed / 0 failed / 2 ignored` in
`236.48s`, warnings `0`. Frontend reruns pass Timeline external DnD, Timeline
Source Shelf at all five supported viewports, and TypeScript, warnings `0`.
The no-default app project-load bootstrap test is `1/1`; its sole warning is
pre-existing in unchanged `control_plane_runtime.rs`, so modified files remain
at warnings `0`.

The local-only complete-show alpha53 derivative now starts the fixed foreground
composition in an exact full-duration `[0, 3008)` loop. Its current identity is
`target/qa/DSF2026-show-alpha53-complete-show.sdc`, `1,112,369` bytes, SHA-256
`46D79EEA2A0562D4CB385F4BA9AA6E721FC741D081533D95DA073EB2099F016E`.
Generation tests, strict verify-only, and the show structural preflight pass.
The superseded black-first-frame artifact was retained recoverably under its
identity-bearing `.old-black-first-frame.72B51207.sdc` name. This remains a
same-PC QA specimen, not a portable release bundle.

The source checkpoint is pushed and upstream-equal at
`1545bedd189c1f14b9656551ca5046cb1d02023c`. Immediately before build, only
exact checkout-owned alpha.56 PID `54360` was terminated. The first build
attempt failed before output because `FFMPEG_DIR` was unset; it is not evidence.
After explicitly supplying the existing FFmpeg 8.1.2, LLVM, and ASIO SDK paths,
the exact Community MSVC 14.44 no-bundle build passed in `5m29s`, warnings `0`.
Exact alpha.57 artifact: ProductVersion/FileVersion `1.2.0-alpha.57`,
`62,310,912` bytes, SHA-256
`ACB6A29A90F5D9CE5D57A406AC15488F870515B917B4654720F92332C86E3F2B`.
PID `2440` is the sole checkout process; its `Syndocal` window is responsive,
visible, and maximized, its command line carries the exact alpha53 project, and
the output-owner enable was confirmed. The current distinct persisted targets
are output 3 (`PX160 WAVE`, identity
`1ca92e95cf40014697261a29800d02f50dd5777f092470d3973d25949ee0fa92`) and
output 4 (`MPG321UX OLED`, identity
`7353124ed8458eadca4625722d80fe4f6ac7206861f040de0294130e3d1fdb06`). Strict
post-restart window observation reported both targets closed. One deliberate
Display 1 open briefly created `Syndocal Output - Display 1`, then failed and
closed with raw native error `Native Display output 3 project/video authority
changed before present`; Display 1/5 runtime acceptance remains blocked pending
that fix, and no retry was performed.

USB-DMX, exact MiraBox, and audible PROGRAM/CUE were not reverified. Unity/
Spout and three-display pixel acceptance remain open external gates. These
boundaries must remain open in the next handoff.

## 2026-09-01 alpha.56 terminal loop/Follow source checkpoint

Current metadata is `1.2.0-alpha.56` on `codex/syndocal-v1.2`. The source commit
is the commit containing this section; verify exact upstream equality after push
before beginning the native build. The old strict root-loop release could expose
loop OFF before natural-terminal transport and Follow admission converged, with
late capacity, preparation, audio, acknowledgement, or publication failure able
to separate worker-local state from the published receipt. Alpha.56 preflights
all transport and click-schedule successors, converges terminal and Follow inside
one receipt-bearing worker turn, and restores exact A plus the Follow presenter
on every fallible post-mutation boundary. Invalid, stale, expired, exhausted, or
poisoned state remains visibly fail-closed; no compatibility path or persistence
schema was added.

Exact MSVC 14.44 source evidence: strict release `16/16`, `dj_link_` `40/40`,
`timeline_follow_` `27/27`, and full engine `997 passed / 0 failed / 2 ignored`
in `225.50s`; first-party warnings `0`. Timeline transport, loop,
runtime-watermark, TypeScript, release, formatter, and diff gates pass.
Independent Terra xHigh rereview is GO with P0/P1 `0`; Ox was unavailable and
this is the narrow documented exception. P2 proof strengthening remains for an
explicit stale shared-A assertion and terminal-fixture poison coverage.

The source checkpoint is pushed at
`3185fe52a631b0b5855b9e5996ad131db74e8e81`. Immediately before the native
build, PID `56892` was resolved to this checkout's exact alpha.55 executable and
only that process was terminated. Exact MSVC 14.44 remained first and absolutely
pinned; `pnpm --dir app tauri build --no-bundle` completed in `4m30s`, warnings
`0`. Exact alpha.56 artifact: `target/release/syndocal.exe`, ProductVersion and
FileVersion `1.2.0-alpha.56`, `62,292,480` bytes, SHA-256
`4BCC1B33A73586C3CED2FB66217732FD1CF1BBF957529559E0DD7ED67435ABAE`.
PID `54360` is the only checkout-owned Syndocal process; its single window is
responsive and maximized. The retargeted StandardRelease observer passes `96/96`
under PowerShell 7 and Windows PowerShell 5.1, with both scripts accepted by both
parsers. No output window was created, re-armed, or moved.
Independent Terra xHigh observer rereview is GO with P0/P1 `0`; its sole P2 is
future explicit negative inventory for retired alpha.55 authority tokens. Those
tokens are absent from the current runner/test files.

Physical USB-DMX, Art-Net/Unity, Spout, display pixels, MiraBox, audible
PROGRAM/CUE, and real DJ/pedal gates remain unverified. First safe physical action
is to reconnect the exact device under test, confirm its machine-local identity,
and execute only its bounded acceptance checklist against this alpha.56 process.
Post-build inventory: `target` `158,092,212,050` logical bytes / `112,200`
files; `app/node_modules` `545,338,492` bytes / `3,704` files. No eligible
exact-target cleanup harness exists; no deletion was attempted and reclaimed
bytes are `0`.

## 2026-09-01 historical alpha.55 runtime snapshot/native checkpoint

Current metadata is `1.2.0-alpha.55` on `codex/syndocal-v1.2`, based on clean
upstream-equal parent `c0885cf8b8d4c6bc54207b4f3dfbd1676f79a413`. The exact
reviewed implementation/source/native checkpoint is commit
`2c4fe33b06eba339ec246c7ba02dd3e01ffcc1f6`. This follow-up records that
post-commit identity; verify the documentation commit itself equals upstream
after the shared push. The show
project failure was a representation bug: native runtime Timeline watermarks
were intentionally excluded from `.sdc`, while renderer ingress incorrectly
required them inside that persisted snapshot. Alpha.55 carries a mandatory
runtime-only sidecar on every native full/delta/authority/poll/canonical read,
validates exact transport/loop/Follow coherence, and applies it through one
ingress path. Runtime state is stripped from storage, recovery, dirty-state,
and publication signatures, so authored project persistence stays unchanged.

The mapping stage also clears stale local hover when the pointer or click moves
to empty stage space; this removes unsolicited fixture labels without changing
selection or drag semantics. All five required mapping viewport checks pass.
Runtime-watermark, loop, transport, authority, project transaction/storage,
mapping-live-snapshot, TypeScript, Rust format, diff, and `check:release` gates
pass. Focused exact-linker Rust proof is `2/2`, warnings `0`; independent Terra
xHigh review is GO with P0/P1 `0`. Ox was unavailable and is recorded as the
narrow exception.

The exact Community MSVC `14.44.35207` linker was pinned and first. The required
native no-bundle build completed in `5m16s`, warning-free. Exact executable:
`target/release/syndocal.exe`, `62,292,480` bytes,
ProductVersion/FileVersion `1.2.0-alpha.55`, SHA-256
`8CBD6A5875CFAD8BFABC1838BE7E4C32FA367017E9267F97453B756706B63CA1`.
PID `56892` is the only checkout-owned Syndocal process and exposes one
responsive window with `準備完了`. Capture succeeded; input activation failed
closed after one clean Computer Use reinitialization, so no synthetic
pointer/keyboard action was used. The exact executable was then invoked with
alpha52 through the official single-instance command-line open path. PID
`56892` remained the only process, the project loaded visibly, and the retired
missing-runtime-watermark error did not recur. Read-only derivation,
show-structural preflight, and Timeline block/loop gates pass. Both reference
MP3 files and both current MP4 files exist and were independently hashed.

The exact local-only project opened in this checkpoint is
`C:\Users\kouty\Documents\KDMX\target\qa\DSF2026-show-alpha52-three-display-current-media.sdc`
(`1,112,316` bytes, SHA-256
`27484E18DE3FFBB19829D19A90459AB4D847209BF272E629CAE37AF3DD38EA11`).
Its foreground `logo-anim-dark.mp4` and background `EtaMDr-gpyCYahbz.mp4`
paths both exist; the previous missing-foreground note is superseded. Two
configured Display routes and three compositions are visible after load, but
both output windows are `ProjectSwapDisarmed`, so they must be deliberately
re-armed before physical pixel acceptance. USB-DMX is currently absent and
Unity owns UDP 6454, so no unattended output probe was fired. MiraBox is also
absent; do not substitute another camera for its persisted identity.

The current native project was then changed through the same official
single-instance path to
`C:\Users\kouty\Documents\KDMX\target\qa\DSF2026-show-alpha53-complete-show.sdc`.
The same PID remained sole owner, the native status displayed that exact path,
and no runtime-watermark error appeared. This is the show candidate with the
two mirrored Scene Block durations restored to `81,153 ms`; open is accepted,
but Play/loop/Follow/pedal behavior is not yet physically observed.

Windows currently enumerates the full WASAPI output catalogue, including
`Music (Elgato Virtual Audio)`. The Setup-owned Cue Audio runtime/browser gate
and output-bus UI gate pass. Alpha52 persists the Reference Audio layer muted
to prevent the rehearsal MP3 from doubling the live DJ audio; unmute that lane
only for rehearsal listening. Audible PROGRAM/CUE remains a physical gate.
Next operator actions are therefore: connect and re-confirm the exact USB-DMX
device, re-arm the two Display outputs and observe their physical content,
reconnect exact MiraBox for the camera interval, then exercise Timeline
Play/A-B loop/Follow/Pedal and audible PROGRAM/CUE.

The StandardRelease three-display observer has been retargeted to the exact
alpha.55 artifact/source identity. PowerShell 7 and Windows PowerShell 5.1 each
pass `96/96`; independent Terra xHigh review is GO with P0/P1/P2 `0`. After
this tracked checkpoint is committed, it can observe three stable samples of
already-open outputs. It never creates, re-arms, or moves those outputs.

The tracked alpha.55 observer/documentation checkpoint is pushed as
`feb300674a417e8b35b52abb80b95e8aad0dde64` on `codex/syndocal-v1.2`. The
authoritative resume state is the commit containing this note after its
upstream-equal push; the exact post-push equality must be rechecked before any
physical acceptance action.

Never stage or modify the untracked protected project
`DSF2026-show-alpha51-usb-final.sdc` (`1,116,223` bytes; SHA-256
`7031196A6527431FB8D625F420FE5D3E442DD2AA35D2ED86D57AF9EDF69890A7`).
The exact root-relative filename is explicitly ignored; this prevents
accidental staging and lets a committed tracked tree satisfy the strict
three-display clean-check without moving or rewriting the protected file.
Generated inventory: `target` `147,037,274,593` bytes / `106,330` files;
`app/node_modules` `545,338,492` bytes / `3,704` files. Cleanup reclaimed `0`
bytes because no exact-target cleanup harness has the required tracked tests
and independent review.

## 2026-09-01 historical alpha.54 strict loop native checkpoint

Historical candidate metadata was `1.2.0-alpha.54` on
`codex/syndocal-v1.2`, based on upstream-equal parent
`067d0bbb246ef87d0a6f13f73899d7d0eaba9e01`. The reviewed implementation,
source, and native checkpoint is pushed as
`c742af8c778f41ddc0c8dd7d99d44b5d5d5f9a12` on the same upstream branch. Root Timeline loop ON/OFF,
half, and double now use a dedicated exact-authority request/receipt lane and
canonical snapshot convergence. MIDI and OSC use the same checked engine
primitive, with toggle decided inside the worker so two queued edges cannot be
lost. All full/delta/poll/canonical renderer snapshots pass the same transport,
loop, and follow generation watermark. Retired fire-and-forget Tauri/engine
routes are unreachable.

The first full engine run exposed one Guide regression: loop OFF created
`Break` under the predecessor transport authority and the authority commit then
retired it. Alpha.54 reissues `Break` only after the successor commits. The
focused regression passed and the complete engine rerun is `987 passed / 0
failed / 2 ignored`. Protocol full is `217/217`; focused Syndocal lifecycle is
`2/2`.

The renderer extraction produces `App-BytuNPGE.js` at `497,905` bytes with no
Vite chunk advisory. Command dispatch remains statically bound and performs
invocation-time route/authority capture. App.tsx itself is still oversized; a
further source split is a P2 maintainability item, not an unreported completion
claim.

The first native launch exposed and blocked on a stale reviewed-admission
fingerprint. The strict loop clean break removed two old routes and added two
new routes, leaving count `509` unchanged but changing the exact name set. The
frozen fingerprint is now the reviewed current value
`d806e8380462507a590fdd795d5bb21fe9c1bd2bd16f65da725d103af2aa7486`.
The new commit is an inner-authority RuntimeMutation with explicit preflight
dispatch policy; its query is ReadOnly. Canonical registry metadata now proves
both loop operations as R0 authoritative-runtime operations with exact terminal
receipts and the same token-bucket/fail-closed contract as transport. The two
retired direct loop names are asserted absent.

Final gates: control-plane `28/28`, dispatch-fence `1/1`, frontend invoke
inventory `449`, frontend routing `131` renderer / `31` server-authoritative,
all five extracted-module checker migrations, strict loop, shared snapshot
watermark, Timeline block/loop, TypeScript, Rust format, and diff checks pass.
The strict-loop and snapshot-watermark checkers are first-class package scripts
inside `check:release`, which the cross-platform CI workflow executes. Their
source assertions now prove the actual integration-module and shared-ingress
delegation instead of relying on explanatory App comments. Final independent
Terra xHigh re-review is `GO`, with P0/P1/P2 `0`.
The Windows release warning ratchet is baseline `0`, current `0`; no first-party
or third-party warning remains. Exact Community MSVC `14.44.35207` was pinned
and first for every Rust/native gate.

The final no-bundle executable is ProductVersion/FileVersion
`1.2.0-alpha.54`, `62,271,488` bytes, SHA-256
`9C5D9350D8CF0B615507C1CF02256B540C192294B8D5EA382DC89824996F0693`.
Exactly one checkout-owned process, PID `72444`, has one responsive maximized
`Syndocal` window (`1920x1032`). Its status is `準備完了`; neither the owner
registration inventory failure nor the missing loop dispatch-policy failure is
present.

Ox was unavailable, so independent Terra xHigh lanes are the documented narrow
exception. The protected user project `DSF2026-show-alpha51-usb-final.sdc`
remains untracked and must not be staged or changed. USB-DMX is absent; Art-Net
was not probed because Unity owns UDP 6454; Spout pixels, physical three-display
content, and audible PROGRAM/CUE remain unverified for alpha.54. Immutable
alpha.53 artifacts/evidence remain historical and must not be retargeted.

Workspace inventory at this checkpoint is `148,970,288,441` logical bytes /
`121,234` files; `target` is `146,215,879,267` bytes / `105,061` files. No files
were deleted and reclaimed bytes are `0`, because no tracked cleanup harness
with focused safety tests and an independent exact-target review is eligible.
Do not substitute manual broad deletion for that missing safety proof.

## 2026-09-01 alpha.53 source/native checkpoint (three-display placement accepted)

Alpha.53 was the preceding source checkpoint on `codex/syndocal-v1.2`, based on
upstream-equal parent `9be5479b23b417049c078e24a7e6c3b172fc720e`. The exact
source commit and alpha.53 executable identity are recorded below. That build
evidence does not itself accept Timeline UI, three-display, or physical output
behaviour.

The old Timeline transport path settled Play/Pause from the command receipt while
an older generic snapshot could still be in flight, and a queued action could
query authority after a project replacement. The new path captures exact
project epoch/revision/checkpoint/read-generation scope per enqueue, partitions
queue groups by that scope, revalidates before query/send/retry/receipt/canonical
apply/queue advance, and settles success only after authority-bound canonical
snapshot convergence. Invalid, stale, replaced, or cross-scope work fails closed
without mutating the newer project. Independent review reports P0/P1 `0`; the
remaining P2 evidence limitation is that the App-level delayed-full-versus-
canonical race is source-contract checked rather than exercised by a dedicated
App async harness.

The old overlapping-lighting representation could collapse simultaneous Scene
Blocks into one marker and later prototypes could grow a lane without bound or
leave a hidden selected marker in the roving focus model. The new representation
uses at most eight deterministic rails (`258 px` maximum lane height), exact
track/layer/member identity, and the complete Scene Block subtree on every visible
rail. Overflow stays visible through the focusable count/Inspector badge but does
not create hidden marker DOM. One overflow-filtered collection now owns rendering,
the sole `tabindex=0`, and keyboard traversal. Independent review reports
P0/P1/P2 `0`.

An Alt-isolated Split could also return a fresh item before the parent-owned
selection validator observed the newly applied snapshot, causing the new audio
item to be transiently rejected and its focus to disappear. Local selection now
settles first, App-owned validation runs in the next microtask, and focus waits a
bounded maximum of seven animation frames for the new DOM. Every retry is fenced
by the captured project epoch and active Timeline ID and stops after unmount, so
an old action cannot focus a same-ID item in a replacement project/Timeline.
Independent Terra xHigh review is GO with P0/P1 `0`; P2 is limited to the lack of
an explicit test for a user intentionally focusing another control during that
short same-Timeline retry window. No authority or mutation is affected.

The exact alpha.53 local three-display QA derivative is generated and verified
by `qa/harnesses/derive-dsf2026-alpha53-complete-show.mjs`. It preserves two
ordinary Display outputs and three compositions, restores the four mirrored
Scene Block duration fields to `81153 ms`, and starts the fixed foreground
layer in an exact full-duration `[0, 3008)` loop. Its output
`target/qa/DSF2026-show-alpha53-complete-show.sdc` is `1,112,369` bytes,
SHA-256
`46D79EEA2A0562D4CB385F4BA9AA6E721FC741D081533D95DA073EB2099F016E`.
The superseded black-first-frame artifact was moved recoverably to
`target/qa/DSF2026-show-alpha53-complete-show.old-black-first-frame.72B51207.sdc`;
it was not deleted and is not an accepted current specimen.
It is a same-PC exact QA specimen only: fixed `C:\Users\kouty` and absolute SDC
media paths make portable/release use fail closed until explicit relink and
re-verification.

Source gates pass with first-party warnings `0`: release metadata and its ASIO,
audio, project bootstrap, video-output, and camera subgates; Timeline transport;
Timeline overlap (`10k 22.0 ms`, rails `11.6 ms`); TypeScript; Vite frontend build;
local derivative verification; syntax and diff checks. The Timeline performance
browser gate passes all `1920x1080`, `1366x768`, `860x520`, and `1280x720`
viewport contracts plus direct resize. One earlier run failed the Alt-isolated
audio focus assertion and exposed the ordering issue above. After the bounded,
scope-fenced focus fix, the isolated full rerun passed; concurrently running
orphan copies of the same browser fixture were terminated by exact command/PID
before that acceptance run. The Vite build retains one existing chunk-size
advisory (`511.01 kB` App chunk); no first-party source warning was added.

The final transport correction was pushed and verified upstream-equal at
`27f45d1689f415a9423e431d1bf9bf285c0bd634`. The observed native convergence
error came from a wire projection mismatch: nested runtime transport fields are
intentionally `serde(skip)` for persistence and therefore reached the renderer
as absent values. The corrected authority bundle exposes numeric top-level
epoch/generation from the same captured engine snapshot while leaving nested
project serialization unchanged. Focused checker, TypeScript, formatter, and
the Rust serializer test (`1 passed`, `0 failed`, `1416 filtered`) pass with
first-party warnings `0`; independent Terra xHigh review reports P0/P1 `0`.
The separate terminal/loop/follow end-boundary race remains P1 future work; the
App delayed-full/canonical dynamic-harness gap remains P2.

Its final native no-bundle build passed in `3m12s` under
`vcvars64.bat -vcvars_ver=14.44`, with
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` pinned to
`C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\link.exe`
and that same linker first in `where.exe link.exe`. The successful build set
`FFMPEG_DIR=C:\\Users\\kouty\\AppData\\Local\\Microsoft\\WinGet\\Packages\\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\\ffmpeg-8.1.2-full_build-shared`,
`LIBCLANG_PATH=C:\\Program Files\\LLVM\\bin`,
`CPAL_ASIO_DIR=C:\\Users\\kouty\\Documents\\KDMX\\target\\asio-sdk-2.3.4\\ASIOSDK`,
and `SYNDOCAL_ASIO_SDK_ARCHIVE_PATH=C:\\Users\\kouty\\Documents\\KDMX\\target\\ASIO-SDK_2.3.4_2025-10-15.zip`.
The resulting exact `target/release/syndocal.exe` is ProductVersion/FileVersion
`1.2.0-alpha.53`, `62,424,576` bytes, SHA-256
`69E89678FFF38CD50F631BCA2E36D1FAFDB38FC0CB49F6C98254E715CC187178`.
Rust first-party warnings were `0`; the existing Vite `511.02 kB` App-chunk
advisory remains. The exact executable launched as responsive PID `45336`.

Maximized native Timeline Play converged to epoch `1`, generation `3`, playing
`true`, position `18699`; Pause converged to generation `4`, playing `false`,
position `47058`, stable after `2.2 s`. The previous error did not recur.
Reference Audio was unmuted; explicit CUE resolved to
`Music (Elgato Virtual Audio)` with lifecycle `running`, live callback, fault
count `0`, and no last error. This is runtime proof, not a listening claim.

The normal UI re-armed the lighting/video lease (`held_active`) and reopened
only existing output ID `3` / `Display 1` / `1920x1080` / Foreground and output
ID `4` / `Display 5` / `3840x2160` / Background. Both currently report
`live_open=true`, `OwnedByMachineRole`, and no last error. Exact
StandardRelease identity/placement sampling remains the next action; this
paragraph is not that acceptance. The first Apply attempt rejected before
display sampling because a successful Git environment guard leaked Boolean
`True` into the branch resolver. HEAD, branch, ancestor, and clean helpers now
discard only the guard success output; guard exceptions still fail closed. The
new scalar-output regression contract passes `91/91` in both Windows PowerShell
and PowerShell 7, and independent Terra xHigh review reports P0/P1/P2 `0`.

Two subsequent Apply attempts rejected before sampling with the same stale
ancestry diagnostic. Live process evidence showed CDP listener PID `81660` as a
stable direct child of exact Syndocal PID `45336`; the reported PID `74648` was
instead Syndocal's already-exited parent. The old helper walked past the exact
Syndocal trust root toward PID zero. It now receives the exact checkout PID,
includes that PID and stops immediately, while failure before reaching it,
zero termination, cycles, and excessive depth remain fail-closed. Existing
listener-first, uniqueness, exact-PID containment, loopback-listener, and single
strict-reader checks remain intact. Windows PowerShell and PowerShell 7 each
pass `95/95`; both parsers pass; scoped diff checking has only LF-to-CRLF Git
notices. Independent Terra xHigh review is GO with P0/P1 `0` and a P2-only note
for the existing PID-reuse race plus no dedicated expected-listener-equals-root
selftest. The next safe action is to commit/push this harness checkpoint, then
perform exactly one new StandardRelease Apply with the new clean HEAD.

That clean-head Apply reached CDP successfully, then rejected before sampling
because the PowerShell WebSocket evaluator leaked successful ConnectAsync and
SendAsync `VoidTaskResult` values into its output stream. They combined with the
otherwise valid app-owned observation into an array; the live observation still
reported only output IDs `3` and `4`, both open. The two completion values are
now explicitly suppressed with `[void]`, while transport exceptions, response
validation, and disposal remain fail-closed. A fake-WebSocket dynamic regression
requires exactly one typed result and a static contract requires both completion
calls to remain suppressed. Windows PowerShell and PowerShell 7 each pass
`96/96`; both parsers and scoped diff checking pass. Independent Terra xHigh
review is GO with P0/P1 `0`; P2 is limited to no committed per-operation
Connect/Send/Receive failure-disposal test, which the reviewer nevertheless
probed independently. This checkpoint was committed and pushed before the next
StandardRelease Apply retry.

The final StandardRelease Apply is accepted at evidence HEAD
`f3b4fc4a162262c718a45e736eb457f95506e819`. Exact evidence directory:
`target/qa/alpha53-complete-show-final-f3b4fc4-2009889d5a224f21a20be0eb045d6ee0`.
`SHA256SUMS.txt` independently re-hashes every evidence file. `final.json`
contains three consecutive stable samples with `accepted=true` and
`native_hardware_claim=false`; `operation.json` is `performed=false`,
`kind=none`. Each sample binds exact responsive PID `45336`, alpha.53 exe
identity, artifact source `27f45d1689f415a9423e431d1bf9bf285c0bd634`,
and current harness HEAD. Editor HWND `252514478` is maximized on `DISPLAY2`;
output ID `3` / Display 1 is HWND `81333538`, full `1920x1080` on `DISPLAY5`;
output ID `4` / Display 5 is HWND `79695962`, full `3840x2160` on `DISPLAY3`.
The app-owned reader reports both exact outputs `live_open=true`, with unchanged
titles, owner PID, stable monitor identities, DPI, and full physical bounds in
all three samples. This closes native three-display placement/ownership only,
not physical content visibility or any external hardware claim.

Final read-only inventory reports the whole workspace at `138,877,050,747`
logical bytes across `124,921` files and the shared `target` tree at
`135,000,724,819` logical bytes across `98,490` files. The reviewed recurring
cleanup eligibility is not currently satisfied, so no cleanup Apply or ad-hoc
deletion ran; reclaimed bytes remain `0`. Release/QA evidence, dependency trees,
and user-authored files were preserved.

Fresh USB-DMX, Art-Net, Spout, ASIO-device, and physical listening acceptance
remain open. USB-DMX is absent on this PC; alpha.51 `all_white`/COM3 behavior is
historical evidence only. The protected user project
`DSF2026-show-alpha51-usb-final.sdc` remains untracked and unchanged at
`1,116,223` bytes, SHA-256
`7031196A6527431FB8D625F420FE5D3E442DD2AA35D2ED86D57AF9EDF69890A7`; it must
not be staged, moved, modified, or deleted.

## 2026-09-01 alpha.52 show-critical UI/audio checkpoint

Alpha.52 closes the operator-path defects recorded below and is the current
source/native checkpoint on `codex/syndocal-v1.2` (source parent
`78669f51ef38e2cb1a7a131066cf259940021e46`). The clean boundaries are:

- Machine audio routing is edited only in `SETUP > I/O > Audio`. It enumerates
  the current Windows output endpoints, permits arbitrary exact PROGRAM and CUE
  selection, preserves a missing saved endpoint as a disabled warning, and
  rejects ambiguous duplicate exact names. Timeline exposes status and a Setup
  navigation action only. Mount refresh and settings mutations share one FIFO
  gate, including rejection recovery and disposal barriers.
- Timeline primary selection is an App-owned tagged reference covering lighting,
  audio, video, lighting automation, and video automation. Inspector renders all
  five kinds and clears only a genuinely stale selection. An initial/constant
  null selection no longer erases local or Alt-isolated selections, and an
  automation context-menu selection no longer expands linked items twice.
- An exact drop lane is authoritative. The lane picker appears only for multiple
  valid candidates or stale state; sole-candidate click placement resolves
  automatically and ambiguity fails visibly.
- Lighting scenes, audio, and video use duration blocks. The A-B loop is a clipped
  orange interval with start/end edges. A zero-duration legacy lighting item stays
  explicitly unsupported rather than receiving an invented duration.
- `人生オーバー` natural completion holds `惑う星` at zero with
  `destination_start_mode=wait_for_pedal` and
  `hold_first_destination_measure=false`; it does not auto-play the destination.
- `qa/specimens/DSF2026-show-alpha10-reference-audio.sdc` is a tracked structural
  test fixture only. The retired `--require-content-artifact` flags now fail
  closed. The sole local operational bundle is the canonical SDC plus its two
  sidecars and approved-identity manifest under
  `target/qa/dsf2026-show-authored-20260901-canonical`; moving it to another PC
  still requires explicit relink and re-verification because media paths are
  absolute.

Focused evidence passes: TypeScript, localization `3684/3684`, Timeline cue-audio
runtime/browser/panel tests, project-transaction recovery, Timeline source-shelf
static/browser and external DnD contracts, Timeline performance and slim viewport
matrices, block/loop checker, show authoring/structural preflight, rehearsal/same-PC
copy tests, release metadata, JavaScript syntax, and diff checks. Independent
implementation reviews report no P0/P1 finding. Focused first-party warning count
is `0`.

The exact Community MSVC `14.44.35207` gate ran
`pnpm --dir app tauri build --no-bundle` with the absolute linker pinned and first
in `where.exe link.exe`. The resulting
`target/release/syndocal.exe` is product/file version `1.2.0-alpha.52`,
`62,419,968` bytes, SHA-256
`FE29DD658EE3D02661DEFCBF62027F9E99F9014D27A42202FCA7253F25FEF966`.
The native build added no Rust warning; Vite emitted one existing chunk-size
advisory for the `509.43 kB` App chunk. Exactly one responsive maximized Syndocal
window was verified from that exact path at PID `21900`.

No fresh physical USB-DMX, Art-Net, Spout, three-display, ASIO-device, or audio
endpoint listening acceptance was performed for alpha.52. Earlier operator
confirmation that the USB-DMX `all_white` scene held continuously remains bounded
historical evidence only. The current `target` inventory is `129,949,179,997`
bytes; cleanup was not run because no tracked cleanup harness with the required
focused safety proof and independent exact-target review is eligible. The
untracked user project `DSF2026-show-alpha51-usb-final.sdc` is protected and was
not staged, modified, or deleted.

## 2026-09-01 immediate P0 UI and authored-Timeline completion contract

The operator rejected the temporary zero-duration flag representation as a
finished Timeline UI. The required clean boundary is now explicit:

- Lighting scenes, video, and audio are authored and presented as duration
  blocks. A zero-duration legacy lighting cue may remain readable for migration,
  but it must not be the normal authoring result or the completed visual model.
- An enabled Timeline loop is shown as its exact start/end interval with a
  distinct orange region/overlay. It is not represented by a point flag.
- Move, resize, fade, snap, Undo/Redo, accessibility labels, and runtime timing
  must continue to use the authoritative block interval. A cosmetic-only flag
  replacement that diverges from persisted or runtime duration is not accepted.
- `C:\Users\kouty\Downloads\dance.dvc` remains the evidence source for a real
  imported multi-lane block Timeline. The raw DVC and the previously rejected
  intermediate SDC must not be overwritten.

The operator also reproduced a show-blocking project replacement defect in the
current alpha.51 native process: Project > New displayed the unsaved-change
warning, but selecting `Discard and Continue` did not create a new project. The
same session displayed `Retrying pending project transaction commit
acknowledgement recovery before the next mutation.` This is not an intentional
operator lock. Treat it as P0: the exact pending terminal/acknowledgement must
settle before the one requested replacement is dispatched; cancellation must
remain non-mutating; failure must stay visible and must not double-dispatch or
silently discard state. New/load/import/recovery replacement paths must use one
consistent project-replacement barrier.

The same alpha.51 session exposed a second P0 operator-path defect in Timeline
audio routing. `SETUP > I/O > Audio` showed only the Normal WASAPI backend and
sent the operator to a hidden Timeline tool. That Timeline selector then showed
only the persisted `Music (Elgato Virtual Audio) (missing)` row and no current
Windows output devices. The required boundary is one authoritative machine
audio-routing surface in `SETUP > I/O > Audio`: enumerate the current WDM
outputs on entry/refresh, allow arbitrary exact PROGRAM/authoring and CUE
selection, keep a missing persisted endpoint as a disabled warning without
hiding live candidates, and reject duplicate display names as ambiguous. The
Timeline may show the active route/fault status, but it must not own a second
editable device configuration. No default-device or name-only fallback is
permitted when an explicit endpoint becomes stale.

This requirement is part of the broader UI cleanup rule for the final tranche:
configuration belongs in Setup, current context should resolve an unambiguous
Timeline/lane automatically, and the UI should ask for an explicit selection
only when more than one valid target remains. In particular, drag/drop onto an
exact Timeline lane must not first require the separate `Select a lane`
control; accessible click placement may auto-use the sole unlocked matching
lane and must fail visibly when several matching lanes remain ambiguous.

The authoritative Follow values for the show are:

```json
"destination_start_mode": "wait_for_pedal",
"hold_first_destination_measure": false
```

Current alpha.51 runtime/project data already uses these values, but the
DSF2026 generator, its authoring test, the structural preflight, and its test
still encode the retired auto-start/first-measure-hold model. They must be
corrected in this tranche so regeneration cannot reintroduce automatic playback
of `惑う星`.

Physical USB-DMX evidence from the current session is bounded but positive: the
operator played the existing `all_white` scene and observed the attached F3200A
remain continuously lit, then reported that the signal was stable. This closes
continuous visible delivery for that exact live scene/device session. It does
not establish exact F3200A channel semantics, the full production patch, Art-Net,
Spout, three-display pixels, or a fresh DVC-imported Timeline run. The hardware
may leave this checkout before the remaining software tranche, so do not relabel
source/native tests as a second physical acceptance.

Current three-display audit boundary: alpha.51 project structure routes distinct
foreground/background compositions to two native Display outputs, but the
foreground media file is absent and alpha.51 pixel acceptance is not complete.
The earlier alpha.42 observation is historical evidence only.

Before any alpha.51 process replacement for the next native build, preserve and
recheck the latest valid project auto-backup. The latest inspected backup at
this checkpoint was
`C:\Users\kouty\AppData\Local\jp.seraf.ktn.syndocal\project-backups\backup-1788213670697.json`,
`1,188,944` bytes, SHA-256
`355BF852512FEA57E516DD609FC3CB61E8269222FE9DD6B466E9CB85DDD6D37C`.
It contains the project envelope and points back to the current alpha.51 SDC;
its identity must be refreshed immediately before terminating the checkout-owned
native process.

## 2026-09-01 alpha.51 USB-first final priority and Display diagnostic checkpoint

The operator fixed the remaining completion priority to the physical show path:
USB-DMX first; Art-Net, Spout, Unity, and the two native Display windows are
non-blocking for this checkpoint. Before this documentation-only delta, branch
`codex/syndocal-v1.2` was clean and upstream-equal at
`87f37a90c13ca9575f3838daebcde1cf2cd87fa1`; no product source was changed for
the Display diagnostic. The exact alpha.51 process remained the
single responsive checkout-owned `syndocal.exe` at PID `62920`. At
`05:19:43 JST` it had run for `55m15s`; Windows still reported the persisted
`COM3` FTDI device `FTDIBUS\VID_0403+PID_6001+A&A5D719&0&8\0000` as `OK`, and
stderr contained no serial/output fault. Together with the authoritative UI
readback below, this extends the bounded S0/Open-DMX software-worker
observation while preserving all 512 channels at zero. Electrical-waveform and
F3200A DMX-receipt indication remain external and are not inferred from the
fixture staying dark.

The attempted reopen of native `Display 1` failed before publication with
`Native Display output 3 presentation rejected: NotFresh { freshness: Error }`.
Independent static review found that the live auto-backup still references the
test-only foreground source
`C:\Users\kouty\Downloads\06.flash back背景途中経過02.mp4`, while that exact file
is currently absent. The last recorded content hash was
`70C2B6C9D9F7F0F687E309C3207E9EEB78FDADCD738D1980165A643F15A45012`.
No retry or permissive fallback was added: a persistent missing-media fault
must remain zero-present and should eventually report the exact missing path
and a restore/relink action. The native window closed, the USB-DMX worker and
S0 authority were unaffected, and this three-display test remains non-blocking
under the operator's USB-first boundary.

## 2026-09-01 alpha.51 Timeline authority / pedal-wait native and S0 checkpoint

The source was clean and pushed/upstream-equal on `codex/syndocal-v1.2` at
HEAD `4f67dbf108dc17825979731435f844a5cb57311c` before this evidence-only
checkpoint update; product metadata is
`1.2.0-alpha.51`. The exact Community MSVC `14.44.35207` no-bundle build passed
in `3m39s` with the pinned Community linker first in `where.exe` and
first-party Rust warnings `0`. The resulting artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.51`, `62,419,456` bytes, SHA-256
`A8D6A397FF9D6838D16C1F796FC9681BFD64A475FEE71A49CB3AABB413EA998B`.
Exactly one responsive maximized `Syndocal` window was verified at PID `62920`.

The current tranche prevents runtime-free Timeline mutation ACKs from
overwriting live transport truth. Only an atomic authority-bound full bundle
matching exact epoch/revision/hash, active Timeline, bank membership, and
post-receipt read generation may converge the result. Pending/blocked state
fences root and child Timeline transport/edit paths; stale or unverifiable
state stays fail-closed without an implicit retry.

Natural completion of `人生オーバー` targets paused-at-zero `惑う星` pedal
wait. The DJ peer wire remains exact v3: existing Running + loop/hold off and
the exact inherited play-session/timeline-owner/release correlation; the
internal wait flag and Follow source/target/generation receipt are not new wire
fields. `rb-output` remains clean and upstream-equal at
`59df968d91bca71a327ef2a57ee5ab15de9f9947`.

Focused/broad exact-MSVC 14.44 evidence currently passes with first-party
warnings `0`: I/O DJ-Link `43/1 ignored`, Syndocal DJ-Link `120/1 ignored`,
Engine Timeline bank `5/5`, TypeScript, authority/follow checkers, release
metadata, format, and diff checks. Independent Terra xHigh final reviews of
both the Timeline-authority and wait-for-pedal boundaries returned GO.

USB-DMX is the show-blocking output route. The alpha.51 project was reopened,
the exact persisted `COM3` FTDI identity was reconfirmed for this machine, and
the Open-DMX worker was armed successfully under S0. The authoritative native
surface reported output enabled, S0 safety blackout, Open-DMX worker active,
and the latest all-zero frame queued. Art-Net/Spout are operator-waived as
non-blocking for this checkpoint. F3200A remains all-zero-only until a reviewed
safe nonzero acceptance; this checkpoint proves the bounded software worker
and initial zero-write receipt, not electrical waveform or fixture response.
From the first successful observation before `04:35:10 JST` through the final
`04:40:07 JST` recheck, PID `62920` stayed responsive, the worker remained
active with the latest zero frame queued, and stderr added no serial/output
fault (only unrelated DJ-Link `trust_network_absent`). This is a bounded
greater-than-`4m57s` alpha.51 S0 continuity observation.

At `04:47`-`04:49 JST`, the reopened native project also ran its current
Reference Audio + two Video Switch Timeline from the start through natural
transport completion while S0 remained latched. The transport returned to a
play-enabled/stopped state; PID `62920` remained responsive; the post-run I/O
surface still reported the Open-DMX worker running with the latest S0 zero
frame queued; stderr added no serial/output fault. This proves bounded native
Timeline execution does not interrupt the S0 USB worker.

At approximately `05:00`-`05:09 JST`, the same alpha.51 native process selected
the authored `人生オーバー` Timeline (`219,506 ms`), armed its authored
`136,941`-`138,353 ms` loop, and observed a real wrap from approximately
`138,318 ms` back into the `137,662 ms` loop interval. The playhead was then
seeked beyond that loop for the separate natural-boundary proof. Natural
completion stopped at exact `219,506 / 219,506 ms`; for more than five seconds
the active Timeline remained `人生オーバー`, Follow remained `保持`, the
correlated route remained `人生オーバー -> 惑う星`, and the start reason was
`自然再生境界`. `惑う星` did not auto-start. This closes the native source
loop and natural completion-to-pedal-hold behavior. A real Pedal 1 receipt that
starts `惑う星` remains external because the DJ peer is absent under
`trust_network_absent`.

Immediately after that proof, Setup > I/O still reported exact
`Enttec Open DMX · このPC`, `S0アーム済み`, `250000 baud`, the alpha.51
Open-DMX worker running, and the latest S0 zero frame queued. At
`05:09:48 JST`, Windows still reported exact `USB Serial Port (COM3)` / FTDI /
PnP status `OK`, PID `62920` was responsive, and stderr contained no new
serial/output fault. Measured from the first successful observation before
`04:35:10 JST`, this extends the bounded alpha.51 worker/Timeline coexistence
observation to greater than `34m38s`. It remains queue/worker evidence, not an
electrical waveform or fixture-response claim; F3200A remains all-zero-only.

## 2026-09-01 alpha.50 USB-DMX continuous S0 checkpoint

The current artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.50`, `62,416,384` bytes, SHA-256
`C73065B089D7AE7672213FB2334882A033AA5B5936EF23A15D4B40F4AF8BEE84`.
The exact Community MSVC `14.44.35207` no-bundle build passed with the pinned
Community linker first in `where.exe` and first-party Rust warnings `0`.
Exactly one responsive maximized window was verified (PID `22100`).

Alpha.49 later stopped with `failed to write whole buffer`, so its earlier
approximately-five-minute healthy statement is historical only. Alpha.50
replaces the too-short Open-DMX `2 ms` Windows COM write timeout with one
centralized `100 ms` builder used by both the normal and verified-direct
Open-DMX opens. The separate Enttec USB Pro timeout remains `2 ms`. A zero or
partial write is still terminal and fail-closed: no retry, S0 remains latched,
and later live frames are rejected.

The saved show project remained `1,114,510` bytes / SHA-256
`5926A36FDD8E0251A2B67904A93490D3E3B3F54E323E24D3FAC26B31E7546F46`.
The one-button `Prepare show DMX` flow completed `Both`, exact `COM3` machine
binding, Art-Net loopback, S0, and Open-DMX arm against PnP instance
`FTDIBUS\VID_0403+PID_6001+A&A5D719&0&8\0000`. Backend activation was retained
only after its initial all-zero BREAK/MAB/`write_all`/`flush` receipt.

From `2026-09-01 02:45:22 JST` through `02:55:36 JST` (`10 min 14 s`), the
exact process remained responsive; the final authoritative UI surface showed
S0 armed, the Open-DMX worker active, its latest zero frame queued, and no
worker fault. Stderr contained only unrelated DJ-Link
`trust_network_absent`. This closes the bounded software-worker continuity
checkpoint under S0. It does not close USB electrical waveform, F3200A
receipt/visual response, nonzero output, Art-Net datagram, Unity/Spout pixels,
MiraBox, production DJ-Link peer, Timeline Follow/Pedal 1, or three-display
pixel-content acceptance. The attached F3200A remains all-512-zero-only.

Focused gates passed: IO serial `25/25`, Engine show-serial `23/23`, Syndocal
show-serial `5/5`, release metadata, formatting, diff, and native no-bundle
build. First-party Rust warnings are `0`; the standard Vite `>500 kB` chunk
notice remains. Independent read-only review reported GO with no P0/P1.

## Historical 2026-09-01 alpha.49 native one-button / output checkpoint

The exact Community MSVC `14.44.35207` no-bundle build completed with
first-party Rust warnings `0`. The release artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.49`, `62,397,440` bytes, SHA-256
`2152272B75A59D3342DE4AD64D33638564FC9D2038ED38D56F7C65B64F42177C`.
The pinned Community linker was first in `where.exe` (Git linker second). One
responsive maximized Syndocal window was verified (PID `71980`).

The authored project is
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-artnet-probe-acceptance.sdc`,
`1,114,510` bytes, SHA-256
`5926A36FDD8E0251A2B67904A93490D3E3B3F54E323E24D3FAC26B31E7546F46`.

The one-button `Prepare show DMX` flow completed all four UI stages: `Both`
role, exact `COM3` machine-local PnP binding, Art-Net
`127.0.0.1:6454` / wire Universe `0` enable, and S0 engagement with Open-DMX
arm. Backend activation acknowledged only after the initial physical all-zero
BREAK/MAB/`write_all`/`flush` transaction. Alpha.49 recorded an
approximately-five-minute healthy observation before its later terminal
WriteZero. That observation is historical only and is superseded by alpha.50;
it is not current continuous-health evidence or electrical/fixture proof.

The identified live display arrangement was Display 1 `1920x1080`, Display 5
`3840x2160`, plus the editor window (three screens). This records live windows,
not current pixel/Spout acceptance. Unity listener and Art-Net route status are
local readiness evidence only; no current Art-Net datagram capture is claimed.
MiraBox was absent, and Unity pixel/Spout acceptance remains open.

The project contains the wait-for-pedal destination configuration from
`人生オーバー` to `惑う星`; natural-boundary transition and Pedal 1 live start
receipt remain pending and must be captured separately from source tests.

Focused evidence: serial code `23/23`, protocol `205/205`, TypeScript, fmt,
diff, and `check:release` pass. The only reported frontend notice is the
standard Vite chunk-size warning (`>500k`); no first-party warning was added.
The alpha.49 implementation and QA checkpoint was committed as
`f7e412c284e6222f85c57f81c7ebf52d7e8995a9` and pushed to
`origin/codex/syndocal-v1.2`. This handoff update is the documentation-only
successor at the current branch tip; verify `HEAD == origin/codex/syndocal-v1.2`
before resuming.

Workspace cleanup was intentionally not run at this checkpoint. The bounded
size inventory of the large shared `target` tree did not complete within the
show-critical time box, and no independently reviewed exact recurring-delete
target set was available. Current release/QA evidence, user-authored project
files, dependency stores, and build trees were preserved; no reclaimed-byte
claim is made.

## Historical 2026-09-01 alpha.48 stage-4 stale-fence checkpoint

Alpha.48 artifact was `62,375,424` bytes, SHA-256
`749AF5590D2694937F9CAE62471AFF87446E87C455137270D98C38582A82B6DD`.
The previous stage-3 stale-fence fix crossed to stage 4, then the serial arm
failed before publication because `GetFinalPathNameByHandleW` received an
invalid COM handle (`0x80070057`). No USB worker or physical write occurred.
This is a pre-physical-apply fail-closed rejection, not external delivery
evidence; alpha.49 supersedes it.

## Historical 2026-08-31 alpha.46 native / managed-lease race checkpoint

The exact MSVC `14.44.35207` no-bundle build completed with first-party Rust
warnings `0`. The exact release artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, Product/FileVersion
`1.2.0-alpha.46`, `62,290,432` bytes, SHA-256
`04A36C709BEF41E8BE4A561C37835EE6FDAAD8F3EC25586666DFB4D6C2B8F768`.
One responsive maximized Syndocal window opened the identified alpha.42
Art-Net-probe acceptance project. Its current byte identity is `1,114,470`
bytes / SHA-256
`35EE42976B97C9570139A37AF09F82E1B1D97B0F1BD1CC5F6D9DB83F53EA2F0A`.

`Prepare show DMX` completed the `Both` and machine-local binding stages with
MiraBox unplugged; input-only capture absence no longer blocks `Both`. After
the operator accepted the native stage-3 confirmation, the exact stderr record
showed `Err(StaleGeneration)`: the managed keepalive renewed the same exact
`Both` lease while confirmation was open. This rejection occurred before the
Art-Net socket, S0 transition, USB worker, or physical write. The generic UI
`publication_failed / physical output state is unknown` text overstates this
particular pre-publication safe rejection.

At that historical checkpoint, implementation work was narrowly closing that
TOCTOU by serializing an exact managed-`Both` authorization against keepalive
renewal without rebasing public/unmanaged stale requests. Alpha.49 supersedes
that stage-3 boundary; physical USB-DMX and Art-Net delivery still require the
explicit evidence rows below, and the F3200A remains all-zero-only.

## Historical 2026-08-31 alpha.45 source/test checkpoint — native and external gates pending

The KDMX product metadata at that checkpoint was `1.2.0-alpha.45` on branch
`codex/syndocal-v1.2`. This is a source/test checkpoint only. No alpha.45
native build, launch/window verification, saved/reopened show artifact,
physical USB-DMX or Art-Net/Spout output, camera/audio observation, production
DJ-Link peer exchange, or other external/hardware acceptance is claimed. The
alpha.44 source, alpha.43 native, and alpha.42 display records below remain
immutable historical evidence and are not rebound to alpha.45.

The recorded full-workspace Cargo result passed, including Syndocal
`1389 pass / 0 fail / 14 ignored` and Video `163 pass / 0 fail / 1 ignored`.
The focused output-lease keepalive gate passed `69/69`. These deterministic
source results do not close native, external, physical, audible, or hardware
gates. The alpha.45 native build/window, real-device, production peer, and
show-output rows remain pending until observed against identified artifacts,
devices, and configuration.

The current DJ-Link wire is exclusively `syndocal-envelope-v3` with the exact
`{v:3,type,agentId,sessionId,sequence,eventId,payload}` envelope. Flat, v1, and
v2 adapters are retired and rejected without fallback; any older v2 wording in
the historical rows below is not current executable guidance.

## Historical 2026-08-31 alpha.45 operator-run output-control checkpoint — failure boundary

The operator-run artifact was identified as
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, Product/FileVersion
`1.2.0-alpha.45`, `62,272,000` bytes, SHA-256
`34EAC7C71E8EC392A3C51A429A313C573D096AC86DE3499EB6EA0A454641E89B`. The
repository readback was branch `codex/syndocal-v1.2`, `HEAD
85787a87aa95ded22f80f11386a6603ef0c675bb`, upstream
`85787a87aa95ded22f80f11386a6603ef0c675bb` (equal; worktree dirty). This is
artifact/repository identity for the operator observation, not a clean-source
binding or a completed alpha.45 native build/launch gate.

| Surface | Recorded observation | Boundary |
| --- | --- | --- |
| Legacy Spout reset | Reset succeeded at runtime: output count `4 -> 2`; `Display 1` and `Display 5` were preserved. | Runtime inventory only; no Spout receiver or external-display acceptance. |
| Art-Net / Unity | Art-Net enabled; Unity listener observed at `127.0.0.1:6454`. | Local listener only; physical Art-Net node IP remains unresolved and physical-node acceptance is open. |
| `Both` enable | Rejected with `External video output route synchronization failed while preparing Both`; OutputControl returned `publication_failed` / physical output state unknown. | Camera-input-start failure is only a suspicion, not a confirmed cause. |
| USB `COM3` arm | Returned `publication_failed`; worker stopped; no physical zero receipt. | USB physical acceptance was not performed. `S0` stayed engaged and DMX was observed `0/0` (logical/runtime only). |
| F3200A | 34-channel laser remained dark. | Nonzero output is prohibited; retain all-zero only and do not infer a physical zero receipt from darkness. |
| One-button fix | Frontend + backend fix is still in progress. | No native verification or acceptance yet. |

This is a failed/unknown publication boundary, not a completed output
checkpoint. Keep S0 engaged and the F3200A all-zero prohibition in force. The
first safe resume action is to complete the one-button source fix, then run the
required native verification and independently capture the identified USB and
Art-Net paths; do not retry the in-doubt publication before that.

## 2026-09-01 current show output execution / physical boundaries pending

Current transport decision: explicitly select and confirm the machine-local
USB-DMX device/protocol on the show PC; mirror each completed internal U0 frame
to USB-DMX and to Unity via Art-Net ArtDmx (`127.0.0.1:6454`, wire U0, 512
channels). USB frame channel 500 / `payload[499]` and Art-Net `payload[499]`
must both remain `0`; keep the two local Spout senders for video.

Current alpha.50 evidence closes the exact artifact/device identity, initial
S0 queue, initial Open-DMX BREAK/MAB/`write_all`/`flush` completion, and a
fault-free `10 min 14 s` worker run under S0. Still pending: release S0 only after
an exact reviewed fixture-safe test, observe the completed-U0 live mirror,
capture USB electrical wire and Art-Net datagrams independently, and verify
the operator-visible fixture result. The currently attached F3200A laser (34ch, DMX
address 1) is all-512-zero-only: do not release S0 or send the historical Mega
PAR red `ch1=255` / `ch5=255` frame until the operator confirms beam-path
safety and an exact reviewed F3200A channel test. Mega PAR red/blackout is
deferred to a later Mega PAR rig. Queue acceptance or worker completion is not
wire or fixture proof. A fault in this strict USB mirror is show-wide: engage
global S0 and Art-Net zero/blackout. Generic independent serial-route failures
remain isolated from unrelated network routes.

Known nonblocking P2 telemetry boundary: `crates/engine/src/lib.rs:31772-31781`
samples `effective_safety_blackout` before the route send, then
`:31965-31980` publishes `live_frame_queued` and its detail from that sample. If
S0 engages between those points, the same tick can report a queued live mirror
although `send_dmx_frame_at_safety_boundary` at `:64828-64856` rechecks the
logical authority and physical latch and queues zero. The physical/action gate
therefore remains fail-closed; this is a status-truth refinement, not a wire or
fixture claim. Do not widen the sender API in this freeze; resolve it in a
dedicated telemetry tranche with a deterministic interleave proof.

The historical alpha.34–alpha.42 Art-Net-only wording is superseded because
the venue fixtures use USB-DMX; Art-Net remains the simultaneous Unity path.

## Historical 2026-08-31 alpha.44 Timeline UX/Undo bounded pause checkpoint

Authority is branch `codex/syndocal-v1.2`; the pre-checkpoint local/upstream
base was `eaef0508df944dafdee08552acb6be28a74c9c46`, and the authoritative source
checkpoint is the commit containing this note after its upstream-equal push.
At that historical checkpoint, product metadata was `1.2.0-alpha.44`. The tranche contains the documented alpha.44
DVC persistence work plus Timeline Source Shelf density, Scene Block
readability, wheel navigation, adaptive ruler/fixed Grid snap, edge guides, and
authoritative root Scene Block history. It also moves the 409-KB
`live_audio_input_tests` module out of the 5.36-MB `main.rs` source. Source
commit `07622a5` was pushed; immediately afterward the excluded 5,363,529-byte
backup and 3,783-byte one-off split script were deleted, reclaiming 5,367,312
bytes. No build cache or user-authored project was removed.

Independent Terra xHigh review is GO for snap/grid, the repaired Audio/Video
Grid-versus-Edges boundary, Undo production timing, and control-gate cleanup.
Supervisor evidence passes exact-MSVC Engine `3/3`, Syndocal root Scene Block
history `2/2`, first-party warnings `0`, snap, Scene Block helpers, TypeScript,
release metadata/package contracts, and Timeline performance at
1920/1366/860/1280 widths. Source Shelf placement/parity passes at 1920x1080,
1920x1032, and 2048x1152. Compact 1366/1280 reruns and the repaired control
gate's four-viewport aggregate were deliberately not started/finished after the
operator requested this bounded pause.

The first safe source action is to run Source Shelf at 1366x768 and 1280x720,
then the repaired control four-viewport aggregate. Do not add implementation
before those gates are recorded. The first safe native action still requires
operator confirmation that alpha.43 PID `80968` no longer owned needed in-memory
work; path-verify and stop only that executable, run the mandatory MSVC 14.44
no-bundle build, launch exactly one maximized alpha.44 window, and manually
exercise move, RATE/WINDOW resize, Fade In/Out, guide, Undo, and Redo. Physical
DMX/Art-Net/Spout/audio/capture/DJ acceptance remains open.

Cleanup note: a command-line-verified orphan headless Chrome tree rooted at PID
`18032`, profile `C:\TEMP\syndocal-control-upper-workspaces-oDGZYD`, was stopped
without touching interactive Chrome or Syndocal. Recursive removal of that
already-inactive temporary profile was blocked by the execution safety layer,
so the directory may remain and must not be confused with a live gate. No active
process still referenced it at this checkpoint.

Operational pause state: source commit `07622a5` and cleanup-note commit
`0268bb5` were pushed; immediately before this final note the branch and
upstream were equal at `0268bb51dc754fb6e89a07a921fbdafda11bf14d`, with a
clean Git status. The final authority is the pushed commit containing this
paragraph. Existing stashes are preserved unchanged:
`stash@{0}: WIP on codex/syndocal-v1.2: e9209d6 checkpoint: validate Syndocal
1.2.0-alpha.9` and `stash@{1}: On fable/open-dmx-pacing: orphaned open-dmx
pacing WIP (preserved by Fable 2026-08-10 before preset lane)`. No delegated or
browser-gate process remains. Exact checkout release PID `80968` is still
responsive at `target/release/syndocal.exe` and was not stopped.

The pause inventory is `138,356` workspace files totaling `176,049,291,948`
bytes (`163.96 GiB`); `target` is `122,738` files totaling `173,320,737,881`
bytes (`161.42 GiB`). Only the verified 5,367,312-byte split backup/script set
was removed. No build/cache tree had a reviewed recurring deletion target at
this checkpoint, so no cache deletion was attempted; current release, debug,
ASIO, QA, and user-authored evidence remain preserved.

## Historical 2026-08-31 alpha.44 DVC save/reopen source authority

At that historical checkpoint, product metadata was synchronized at `1.2.0-alpha.44`. The old
current-schema validator required a global lighting-effect summary even when a
Cue carried the complete owned effect request. The new path accepts that
single-authority form and still rejects zero IDs, duplicate per-Cue IDs,
missing global definitions for parameterless targets, and malformed
fixture/group/attribute/video references.

With the exact Community MSVC `14.44.35207` linker pinned and first,
`cargo test -p protocol --locked` passed `214/214`; the default-feature exact
`dance.dvc` import -> Engine normalization -> Save JSON -> fresh Engine reopen
test passed `1/1`; and the DVC suite passed `109/0/3 ignored`. First-party
warnings were `0`. `check:release` passed. Independent Terra xHigh reviews are
GO with no P0/P1. The external regression retains the exact 18 lanes, 194
Lighting events, one audio clip, `201090 ms`, 29 cue-owned targets, empty global
effect registry, and nonempty Timeline bank across save/reopen.

Native alpha.44 is still pending because the exact alpha.43 Syndocal window has
an unsaved `Untitled.sdc*` raw import open and the operator is actively viewing
its child Timeline. Do not stop that process until the work is saved or the
operator explicitly releases it. The first safe resume action is then the
path-verified exact-process stop, mandatory alpha.44 no-bundle build, one-window
launch, and native reopen of a separately saved candidate. Preserve the failed
alpha.43 evidence file; do not overwrite it. Physical DMX/Art-Net/Spout/audio,
capture, and production DJ acceptance remain open.

## Historical 2026-08-31 alpha.43 native/DVC resume authority

Native build source/docs were clean and upstream-equal at
`eaef0508df944dafdee08552acb6be28a74c9c46`. Only the exact checkout-owned
alpha.42 Syndocal PID `115592` was stopped after path verification; Rekordbox,
the local DJ Agent, and Unity were preserved. The pinned Community MSVC
`14.44.35207` linker was first in `where.exe`; the no-bundle build completed in
`3m50s`, first-party warnings `0`. The alpha.43 executable is 61,696,512 bytes,
File/ProductVersion `1.2.0-alpha.43`, SHA-256
`3B96560DEFB2E66E03A4D09DAA63825EACE5842674F0CB50FEB62E457228A3DC`.
`check:release` passed. Exactly one checkout-owned PID `80968` was responsive,
and the verified Syndocal window was maximized before UI actions.

The raw `C:\Users\kouty\Downloads\dance.dvc` was imported in that native
window. The report showed fixtures 46, profiles 12, groups 15, cues 56, scene
blocks 194, audio clips 1, and missing audio files 1. In EDIT / LIGHTING,
`TIMELINE / New Scene` `#8.1` visibly showed `TIMELINE` plus `TL`; the same-name
`ber / New Scene` `#15.2` remained `STATIC`. The raw importer classification is
accepted. The intermediate
`target/qa/dance-dvc-import-20260831.sdc` (1,442,222 bytes, SHA-256
`A56DBA4FE15A60B47D6CD6AEE495587980C6ECE46D703D07BF3E5C51D654500F`)
is not accepted: alpha.43 reopen failed closed with `Cue 4 references missing
lighting effect 1`. Do not use or overwrite it as a show project. First safe
code action is a focused importer-save/project-open reference trace; first safe
operator action remains relinking the one missing audio only after a corrected
save/reopen artifact exists. Physical DMX/Art-Net/Spout/audio and production DJ
acceptance remain open.

## Historical 2026-08-31 alpha.43 source-integration handoff

The pushed alpha.43 implementation/QA checkpoint is branch
`codex/syndocal-v1.2` at
`eaef0508df944dafdee08552acb6be28a74c9c46`, recorded with local
`HEAD`/upstream equality. The implementation was introduced at
`8d8c5461f314d85e0be64f4ab7bd4f1a857de619`; that historical checkpoint includes
its pushed documentation follow-up. That committed source checkpoint
advances the product consistently to `1.2.0-alpha.43` across workspace
Cargo/lock, frontend package, Tauri
configuration, release-check metadata, README, macOS bundle artifact naming,
and Windows workflow artifact naming. It also contains the Timeline
follow-hint wrapping change and the control-upper-workspaces fresh-child
viewport harness work in `app/src/styles.css`,
`app/scripts/check-control-upper-workspaces-browser.mjs`, and new
`app/scripts/run-control-upper-workspaces-browser.mjs`. `app/package.json`
contains both the version and harness command changes and is a shared
integration file; do not separately overwrite or commit either half.

`pnpm --dir app run check:release` and
`pnpm --dir app run check:release:self-test` are `PASS`. The final
control-upper-workspaces runner passed `3840x2160`, `2560x1440`, `1920x1080`,
and `1280x720` in that exact order with zero CDP runtime exceptions, errors,
warnings, or harness errors. Timeline performance, context-menu, DJ Link,
localization (`3644/3644`, zero unprotected labels), TypeScript, Cargo format,
and diff checks passed. Independent Terra xHigh adversarial review returned GO
after Vite/CDP endpoint identity, fail-closed input, and cleanup proof were
closed. This paragraph records the historical source-only gate; native identity
is now governed by the alpha.43 native/DVC authority above, and no physical
acceptance is implied.

The bounded show artifacts are the alpha9 reference-audio/content artifact
(`1,095,864` bytes, SHA-256
`93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094`),
the alpha.42 three-display acceptance artifact (`1,120,320` bytes, SHA-256
`2D8B4D760E51009344D5A3195A39A61D0674F993027CD440A49F6F7D8F1F355C`),
and the alpha.42 Art-Net probe-only derivative (`1,120,306` bytes, SHA-256
`4130599CEB73F2D97C7BB22DBCAD6A9187BD02F53DAC3946491BEB32E33776D9`).
The legacy alpha4 artifact is retired and must not be loaded as current. These
three bounded artifacts are evidence inputs, not an approved production
operator artifact; performance/deployment use remains fail-closed until exact
route and hardware acceptance completes. The alpha.42 executable identity,
Art-Net wire boundary, and native three-display observations in the following
sections remain immutable and do not prove alpha.43. Production DJ
configuration/token/Show-LAN and HW-4 `0/12` remain pending. The standalone
Rekordbox local physical F14 beat transition and F13 HPF -> ChannelFader fade ->
Stop edge remain operator-run pending work.

The alpha9 file is pinned only as a content artifact. The tracked gate is
`node qa/tests/author-dsf2026-show.mjs --require-content-artifact`; the retired
`--require-final-artifact` and `--test-only-missing-final-artifact` flags are
unknown and fail closed. Its disabled `EnttecOpenDmx` route persists
`serial_port=""` at `250000`. USB-DMX port identity is machine-local and must be
enumerated and deliberately selected on the show PC; historical `COM3` is not
an authored production requirement.

The exact `C:\Users\kouty\Downloads\dance.dvc` input is `268,702` bytes,
SHA-256 `33E1EAD49F59B511D6A812FD06F506A89FF23721AE6CD99A927FDC89877B6B65`.
`TIMELINE / New Scene` `#8.1` is the imported child Timeline (18 lanes, 194
resolved Lighting blocks, one audio block, `201090 ms`); `ber / New Scene`
`#15.2` is a separate correct Static scene. The exact external acceptance
passed `1/1`, first-party warnings `0`, under the pinned MSVC 14.44 gate. The
audio path is unavailable and must be relinked; the Lighting blocks are intact.

Next safe action: keep the current raw DVC import unsaved, preserve the rejected
intermediate SDC as evidence, and diagnose the missing Lighting-effect
reference through the import -> save -> project-open path. Do not continue to
physical output from that rejected intermediate artifact.

## Historical 2026-08-30 alpha.43 source, process, and external-boundary checkpoint

The pushed implementation/QA checkpoint at that historical time was branch `codex/syndocal-v1.2` at
`eaef0508df944dafdee08552acb6be28a74c9c46`, exactly equal to
`origin/codex/syndocal-v1.2`. The fixed alpha.42 Art-Net derivative generator
and tests were introduced at `42cb17ce3ae0272a43f7671da70361d3f2520e98`.
The DJ production-authority documentation checkpoint was introduced at
`d7449d02dccc686bb99d7b6878a147262868f0b0`.

The external DJ Agent authority is the clean, upstream-equal
`C:\Users\kouty\Desktop\rb-output` checkout on `beta-v1.1.2` at
`59df968d91bca71a327ef2a57ee5ab15de9f9947`, product source `1.1.12`.
Its standalone process remains PID `55684`, listening only on
`127.0.0.1:8787` and UDP `127.0.0.1:22346`; Rekordbox `7.2.18` remains the
responsive PID `49440`. The mode is explicitly `REKORDBOX LOCAL TEST / NO
SYNDOCAL`, Syndocal is disabled in that mode, and the latest observed Deck 1
state is `no-track` / `track-not-loaded`. Physical Hook-measured F14 `2 -> 1`
beat and F13 HPF -> ChannelFader fade -> Stop still require the operator to
load/play Deck 1 and press the pedals. The fresh self-launch/cold-launch
acceptance also remains pending. These local checks do not replace production
strict-v3 provisioning, LAN, token, HELLO/ACK/STATE_SYNC, reconnect, or
Timeline acceptance; HW-4 remains exactly `0/12`.

The exact checkout-owned alpha.42 PID `115592` was path-verified and stopped
for the alpha.43 build. The historical exact checkout-owned alpha.43 PID
`80968` was responsive; its identity is recorded in the native/DVC authority
above.
Unity remains responsive as PID `112488` and is the sole observed UDP `6454`
owner. The Art-Net derivative one-shot is unsent, physical receiver/fixture
observation is open, MiraBox HDMI-content proof is open, audible rehearsal
audio and ASIO real-device acceptance are open, and the production DJ show
configuration/token and Show-LAN proof are not present on this PC.

A read-only logical-size inventory of `target`, `qa`, `app`, and `.pnpm-store`
was bounded at 30 seconds and stopped without a complete result. No cleanup
target has newly passed the recurring exact-path/adversarial gate, the generated
Art-Net artifact is current QA evidence, no deletion ran, and reclaimed bytes
remain `0`.

## 2026-08-30 alpha.42 final native three-display checkpoint

This is immutable historical alpha.42 native evidence and does not override the
current alpha.45 source/test checkpoint above. Branch `codex/syndocal-v1.2` was at `HEAD
b1f6d760c75b430a4255ead71e5f4bb964501cf4`. The exact MSVC `14.44` native
build passed with first-party warnings `0`; the exact
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
Product/FileVersion `1.2.0-alpha.42`, `61,691,904` bytes, SHA-256
`E82B570C7BF850BB99D9DEDC529FC96ADFE952C602BA066394EA885617F17376`.

The persisted acceptance project is
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-three-display-acceptance.sdc`.
It persists exactly two strict Spout outputs and two ordinary native Display
outputs. `Display 1` / LED `PX160 WAVE` (`1920x1080`) uses composition 2 with
fixed Video 1; `Display 5` / projector `MPG321UX OLED` (`3840x2160`) uses
composition 3 with Timeline Video 2 -> MiraBox -> Video 2. Foreground fixed
Video 1 works. The HDMI input is unplugged, so the MiraBox interval is expected
no-signal; black versus flat fill may vary and is not camera content proof.

The startup gate keeps WebView2/controller creation before app-owned
maximize/F11. The desktop-window checker and final physical observation proved
that gate; the separate acceptance harness permits only its narrowly revalidated
`SW_MAXIMIZE` seam before readback. The Add receipt repair is fixed and
independently `GO`. A remaining UX
issue is the Video Output state label `Authored enabled`, localized as
`作成権を有効化`, which still looks like an operation button. Final physical
recheck passed at `1957 ms`, `5002 ms`, and `8989 ms`: `Display 1` stayed on the
same fixed Video 1 orange/red-bordered frame, while `Display 5` showed Video 2,
the expected no-signal frame, and recovered Video 2, respectively. Native route
switching/recovery, foreground independence, and both native Display windows
are `PASS`. MiraBox actual HDMI content proof, Unity/GPU/Art-Net, and other
physical hardware acceptance remain unconfirmed.

### 2026-08-30 alpha.42 Art-Net probe-only derivative

The ignored QA artifact
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-artnet-probe-acceptance.sdc`
was created once by
`qa/harnesses/derive-dsf2026-alpha42-artnet-probe.mjs` from the exact
`DSF2026-show-alpha42-three-display-acceptance.sdc` source identity:
`1,120,320` bytes,
SHA-256 `2D8B4D760E51009344D5A3195A39A61D0674F993027CD440A49F6F7D8F1F355C`.
The derivative is `1,120,306` bytes,
SHA-256 `4130599CEB73F2D97C7BB22DBCAD6A9187BD02F53DAC3946491BEB32E33776D9`.

The exact deep diff is limited to
`snapshot.dmx_outputs[0].protocol` and `snapshot.output.protocol`, both
`EnttecOpenDmx` -> `ArtNet`; both routes otherwise remain disabled at
`127.0.0.1:6454`, wire U0, empty serial port, and baud `250000`. Spout,
Display, and Timeline content remains unchanged. The generator is fixed-path,
strict-schema/count, exact-literal, reparse-safe, and create-new/no-overwrite;
its focused Node tests, `node --check`, and `git diff --check` pass. Under the
exact pinned and `where.exe`-first MSVC `14.44.35207` linker, the Syndocal
packet/route tests pass `2/2` and the Engine no-route-mutation test passes `1/1`;
first-party warnings are `0`.

This is a probe-only current-alpha42 derivative, not a production replacement
or physical acceptance. Unity/UDP `6454` is recorded as prepared for the
eventual physical run, but the generation/verification sent no UDP and the
one-shot remains unsent/unconsumed. No native build, receiver capture, or
fixture observation is claimed.

Recorded focused commands (all exit `0`) are
`node qa/tests/derive-dsf2026-alpha42-artnet-probe.test.mjs`, both generator
and test `node --check` commands, and `git diff --check` on the owned tracked
documentation paths, plus the focused Syndocal `show_artnet_acceptance_probe::tests`
and Engine `dsf2026_probe_engine_boundary_sends_one_exact_u0_packet_without_route_mutation`
Cargo tests under the exact linker gate; first-party warning count is `0`.

## 2026-08-30 alpha.42 three-display source checkpoint (superseded by final native checkpoint above)

- The source checkpoint starts from branch `codex/syndocal-v1.2` at
  `64f3cab58dca0ff0b524c925ae895415a7c56f6a`; the checkpoint commit is the
  commit containing this entry. The Windows main window now creates WebView2
  while windowed, maximizes only after controller creation, and admits native
  F11 only after that boundary. The acceptance harness can separately perform
  only its narrowly revalidated editor `SW_MAXIMIZE` seam before strict
  readback; it does not create or reposition either output window.
- The bounded OutputControl receipt repair accepts a generation-changing
  `Authorized` result only for Display Add and Display-window-open, only for
  exact logical `Both = [Lighting, Video]`, and only for
  `HeldOrphaned G -> HeldActive G+1` or expired
  `HeldActive G -> HeldActive G+2`. Assign, Arm, Spout, Art-Net, wrong resource,
  phase, and delta shapes remain rejected. Independent Terra xHigh review is
  `GO` with P0/P1/P2 `0`. Supervisor protocol proof is `201/201` unit tests and
  `4/4` doc-tests with `RUSTFLAGS=-D warnings`; first-party warnings are `0`.
- The saved acceptance project is
  `C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-three-display-acceptance.sdc`.
  It persists exactly two strict Spout outputs and two ordinary native Display
  outputs: LED `PX160 WAVE` routes fixed Foreground Video 1, while projector
  `MPG321UX OLED` routes Background Timeline Video 2 -> MiraBox -> Video 2.
  Preliminary local seek observations at approximately 2 s and 9 s showed the
  intended Video 2/projector and foreground Video 1 arrangement. HDMI input was
  intentionally unplugged, so the MiraBox interval's black/no-signal frame is
  expected and is not HDMI-content acceptance. At that source checkpoint the
  final physical 2 s / 5 s / 9 s recheck was pending; its result is recorded in
  the final native checkpoint above.
- `pnpm --dir app run check:desktop-window` passes. The three-display harness
  self-test passes in PowerShell 7 and Windows PowerShell 5.1 at `90/90` in
  each host. The next action after this source checkpoint is an exact
  MSVC `14.44.35207` native release build from the committed source, followed
  by reopen/readback of this project and the same physical 2 s / 5 s / 9 s
  sequence. No final alpha.42 EXE identity or post-restart physical acceptance
  is claimed by this source checkpoint.

## Historical source authority (alpha.39; superseded by alpha.42 final native checkpoint above)

- The historical alpha.39 native checkpoint was on branch `codex/syndocal-v1.2`; the
  build source/docs `HEAD` and upstream were both
  `ec93e9160da853ad181de70aee4db7b4a75fafbb`. This tranche repairs the Timeline
  authoring-output selector's
  exact UI truth: exactly one selectable occurrence is admitted, while
  duplicate, missing, or ambiguous identities fail closed; status-only polling
  may reapply the exact desired option without list/configuration/routing
  mutation. Browser Phase A/B passed after the strict external-video
  status-poll fixture was added; TypeScript, runtime, video-poll,
  `git diff --check`, and `pnpm --dir app run check:release` pass. Independent
  Terra select review is GO with P0/P1 `0`, and the checker review is GO with
  P0/P1/P2 `0`. First-party warnings are `0` only for the evidenced non-native
  checks. The exact MSVC `14.44.35207` Community linker was pinned and first in
  `where.exe`; `pnpm --dir app tauri build --no-bundle` exited `0` in `2m57s`
  with first-party warnings `0`. The exact executable
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
  `61,108,736` bytes, reports Product/FileVersion `1.2.0-alpha.39`, and has
  SHA-256 `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
  Exactly one checkout-owned PID `87640` is responsive with maximized
  `Syndocal` window id `2033716740`; Daslight was preserved. The reviewed
  native-evidence record was committed and pushed at
  `94b362bd2d733e447feabf0a0a6158699da6a2bf`, and `HEAD`/upstream equality was
  verified immediately after that push.

  Native alpha9 UI reverification passed without clicking the output selector or
  Refresh. The exact sequence was Play -> Pause -> status-only wait -> Play ->
  Pause -> status-only wait; throughout it, explicit-device and resolved output
  stayed at `Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`, with
  advancing output frames and no visible Backend, Local IPC, or CUE fault. The project
  remained unsaved and final Timeline state was paused. Read-only settings were
  `route=explicit_device`, `device_name=Music (Elgato Virtual Audio)`,
  fingerprint `A2D9603C75ED1A6ECBC37F0FE851AAD56C632DFF31314544F2F606C220C9479C`,
  `click_gain=1`, and `guide_gain=0.85`. The opened alpha9 project remained
  unchanged with SHA-256 `93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094`
  and last write `2026-08-29T05:53:14.5317317Z`. The native build/window
  and selector/UI regression gates are complete only; audible/device selection,
  operator audible confirmation, dedicated Show-ASIO, Unity/GPU Art-Net/Spout,
  DJ, camera, and other hardware acceptance remain open. After the current
  native show-UI evidence below is committed and pushed and `HEAD`/upstream
  equality is reverified, the next safe action is operator audible
  confirmation.

### 2026-08-30 Art-Net one-shot probe source checkpoint

The fixed DSF2026 Art-Net one-shot probe is complete at the source and review
boundary, not at the physical boundary. The old diagnostic/lease path could
leave a Pending operation identified only by a renderer principal, so a new
renderer after rotation could not prove same-window ownership. The new durable
v2 Pending record stores `window_label`; a v1 Pending record without that field
fails closed during migration/reconciliation. Same-window renderer rotation
reconciliation is explicitly no-send, marks the physical result unobservable,
permanently consumes the one-shot budget, and never retries.

The verifier now derives the current Windows UDP dynamic-port range. The exact
packet proof is ArtDmx v14, physical byte `0`, wire Universe `0`, 512 DMX
channels, and channel 500 = `0` (with the fixed source look ch1/ch5 = `255`).
Independent Terra xHigh re-review is `GO`. Focused evidence is engine DSF2026
`3/3`, Syndocal DSF2026 `6/6`, legacy migration `1/1`, and monitor/verifier
`4/4` (`41` assertions); UI/runtime/localization checks pass and exact MSVC
`14.44` first-party warnings are `0`.

No physical UDP packet capture, Unity/fixture acceptance, or native build of
this probe checkpoint has occurred. Do not mark the probe or same-PC physical
output complete. The first safe resume action is the exact MSVC `14.44` native
build/launch from the frozen source checkpoint; then close Daslight/Easy View,
  start the exact Unity receiver as sole UDP `6454` owner, and perform one
  approved probe capture without retrying an in-doubt operation.

### 2026-08-30 native three-display composition source checkpoint (superseded)

The distinct native Display route is frozen at source for review. Its custom
composition membership uses `TimelineVideoLayerRef { timeline_id, layer_id }`.
Root and Follow resolve only their exact timeline/layer pair, so Timeline
projection cannot leak across transports. Timeline projection is composed below
fixed authored layers; the Background route keeps the authored layer ordering
around the Video 2 -> MiraBox -> Video 2 projection, while Foreground contains
only fixed Video 1. Main composition is unchanged, and any Main-based split or
mirror request is rejected. A pre-P0 project missing the field defaults to an
empty membership list; unshipped numeric-only P0 values fail closed.

The independent original review was `NO-GO`; the corrected implementation and
its real 50%-weighted Follow integration regression now have independent
`GO`. The regression proves exact root-Timeline selection when root and Follow
target share a lane number, fixed-layer ordering, and no ephemeral projection
ID persistence. Supervisor evidence is protocol `200/200`, engine focused
`5/5`, weighted-Follow integration `1/1`, Syndocal boundary `1/1`, TypeScript,
localization `3639/3639`, frontend invoke inventory `443`, output-control,
Vite, release, fmt/diff, and three-display harness self-tests PowerShell 7
`90/90` and Windows PowerShell 5.1 `90/90`; first-party warnings are `0`.

Setup Video also passes the five-viewport containment gate, including
`1366x768` and `1280x720`: document/app scroll remains zero, Advanced details
remain reachable through their internal scrollport, and no typography or hit
target was reduced. Independent viewport/UI re-review is `GO`.

The same-PC alpha9 copy tool is historical strict alpha9 -> alpha10
Spout/Art-Net authoring and is not used for the alpha12 distinct Display
project. Its old schema-key list remains explicit nonblocking P2 debt until the
tool is retired or updated. Physical native build and the 2 s / 5 s / 9 s
two-output content acceptance remain pending. The first safe action is now the
exact native build and live sequence from this source-reviewed checkpoint.

### 2026-08-30 alpha.41 native-admission repair checkpoint (historical)

The first alpha.40 launch from `d39383d` was responsive and maximized, but the
UI immediately reported that project transaction owner registration was not in
the reviewed Tauri admission inventory. The artifact was 61,690,368 bytes with
SHA-256 `389664BBF9FA01315CC2D504E0375F137587E481650A951E8C75541B4CD1D86F`.
No authoring mutation or Display Add was attempted after the failure.

Root cause was an old frozen count/hash for 502 Tauri handlers after
`set_video_composition_timeline_layers` became the 503rd production handler.
Because the admission registry validates atomically, the mismatch correctly
failed the entire table closed and made every route appear unreviewed. The
alpha.41 source repair freezes the exact 503-route hash, explicitly classifies
the new route as the same `RendererTicketedProjectMutation` class as
`set_video_composition_layers`, and asserts the exact mapping. It also updates
the stale generated Engine inventory test from 271 to 273 and asserts the two
new variants; runtime Engine behavior is unchanged.

Add Display failures now keep the stable localized summary and optionally show
a closed local diagnostic with redacted/bounded raw detail, captured monitor
identity, and dimensions. The helper redacts authorization, tokens, passwords,
and tested API-key spellings before its 2048-character cap, removes control and
Bidi characters, wraps long values, and provides explicit clear. An operation
epoch/flight token prevents kind ABA and stale monitor selection/discovery from
committing an old result. No storage, console, network, or telemetry path was
added.

Supervisor evidence at that historical alpha.41 checkpoint: exact MSVC 14.44 Cargo control-plane `28/28` and Engine
inventory `1/1`, backend operator contract 503/131, Display checker PASS,
frontend routing 443/131/31, frontend invoke inventory 443, localization
`3644/3644`, TypeScript/Vite PASS, release gate PASS, fmt/diff PASS,
first-party warnings 0. The routing checker names the three new Art-Net probe
routes as non-project mutations and the new Timeline-composition route as the
only renderer-ticketed addition. Independent admission, Engine, and final
diagnostic reviews are GO with no P0/P1. Product metadata was advanced at that
historical checkpoint to
`1.2.0-alpha.41`; no alpha.41 native artifact or hardware claim exists yet.
The next safe action is commit/push this source checkpoint, terminate only the
exact checkout-owned alpha.40 process immediately before build, perform the
exact-linker alpha.41 native build, launch it with isolated loopback CDP and log
capture, verify one responsive maximized window, then author/save alpha12 and
perform each physical Display Add once.

The checkpoint's read-only storage inventory is `target`
`119,395,192,153` logical bytes / `100,307` files, `app/node_modules`
`545,338,492` bytes, `tools/asio-bridge/target` `1,774,985,879` bytes, and
`app/dist` `5,221,332` bytes. The reviewed recurring-cleanup eligibility is not
satisfied, so no deletion ran and reclaimed bytes are `0`.

  A subsequent bounded historical native show-UI pass used the same exact responsive,
  maximized alpha.39 PID `87640` / window id `2033716740` and left the opened
  project paused and unsaved. Right-clicking its Timeline Audio Clip opened the
  compact `操作` disclosure; expanding it exposed the four nested groups
  `選択`, `クリップボード`, `タイミング`, and `レーン`. An outside left click
  dismissed the menu both before and after hierarchy expansion. In
  Setup I/O, the DJ Link disclosure was scrolled to its lower content while the
  upper Web Remote card remained visible and directly selectable; switching to
  Web Remote succeeded without restoring the disclosure scroll position first.
  This is native visual evidence for context-menu dismissal/hierarchy and
  Web Remote/DJ Link reachability only, not DJ/network/remote-server acceptance.

### 2026-08-30 read-only live-audit checkpoint (pre-alpha42; historical)

The read-only FOH/show audit at `2026-08-30 07:10–07:14 JST` ran on branch
`codex/syndocal-v1.2` with `HEAD == @{upstream} ==
71f93803a73f7474ae21f1d8d39eb273bcc36d67`. It changed no files, processes,
network state, UI state, build output, commit, or push; first-party warnings
were `0` because no compile/build ran. It updates live blockers only and does
not change the completion claims above.

- DJ Link is disconnected: bound `イーサネット 4` / interface GUID prefix
  `19578d89...` is `Disconnected` at `0 bps`, and `192.168.50.1` is
  `Deprecated`. No TCP/9100 listener or peer session appeared in the
  read-only socket snapshot; `.50.2` was only a stale ARP neighbor. The current
  peer identity is v1.1.11 / `a13d7bff...`; the older v1.1.10 launch identity
  is stale. HW-4 remains `0/12`.
- Normal audio persists `route=explicit_device` to `Music (Elgato Virtual
  Audio)` (fingerprint
  `A2D9603C75ED1A6ECBC37F0FE851AAD56C632DFF31314544F2F606C220C9479C`), but
  current topology was not UI-confirmed and audible output was not verified.
  Alpha9 production reference layers are muted by design (candidate SHA-256
  `93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094`);
  the rehearsal sibling is unmuted for listening (SHA-256
  `AC9133AFD2C9AAD022674B45222447115F624175B2C5E20DE1E803ABAA9778CE`).
  No alpha.39 ASIO artifact/profile existed at that historical checkpoint;
  TOPPING/CUE physical
  output remains unverified.
- The last UI-verified loaded unsaved project was alpha9; this read-only audit
  did not re-observe the current in-memory project identity. The alpha9 file
  has disabled `EnttecOpenDmx` and no video outputs. The separate alpha10
  same-PC candidate (`1,098,035` bytes, SHA-256
  `DB1C18DCEFC79F5DC8C68589BCCAA492AF2509E932542D4A5036927B5E0814BA`) has a
  disabled Art-Net `127.0.0.1:6454` U0 route and enabled exact `1920x1080`
  Spout senders `Syndocal Background` / `Syndocal Foreground`. Its exact shape
  is `video.media_assets=2`, `video.layers=0`, Main `layer_ids=[]`, and
  `timeline.video_automations=0`; the pinned alpha9 source/media hashes were
  retained and structural preflight passed. Fixed Spout
  output exists, but actual MP4/Camera switching content is not authored. The
  operation contract is existing File/Camera video layer plus Timeline Opacity
  automation (`Step cut` / `Linear fade`); no concrete asset or camera
  identity was supplied, so no physical/content proof exists.
- Authored release-visible `all_white` (Cue 1) and `all_max` (Cue 2) are both
  Timeline 1 `Lighting` events at `138353 ms`, immediately at loop end
  `loop_region.b_ms=138353`.
- Unity is not running. Daslight PID `42752` owns UDP `6454` and must not be
  stopped without the operator. The exact scene
  `E:\UnityProjects\Art-net-Unity\Assets\DSF2026\Scenes\DSF2026_Visualizer.unity`
  has not been played; physical DMX/GPU/Spout remains unverified. The reviewed
  lease-bound one-shot red probe now exists in the source path, but no native
  build or physical packet/capture/Unity acceptance has been performed.

Safe next actions are operator-owned: connect the DJ wired peer and capture the
v1.1.11 state-sync sequence; audible-test the unmuted rehearsal sibling;
manually close Daslight/Easy View, verify UDP `6454` is empty, open the exact
Unity scene and press Play, then use lease-bound Art-Net/Spout enable actions
and verify the fixed senders. Do not unmute the production alpha9 candidate or
claim the red probe until the exact native build/launch and one physical
packet/capture acceptance are recorded; an in-doubt probe must not be retried.

Representative read-only commands/evidence:

```powershell
git branch --show-current
git rev-parse HEAD
git rev-parse '@{upstream}'
git status --short
Get-CimInstance Win32_Process
Get-NetTCPConnection
Get-NetUDPEndpoint
Get-NetAdapter -IncludeHidden
Get-NetIPAddress -AddressFamily IPv4
Get-NetNeighbor -InterfaceIndex 3 -AddressFamily IPv4
Get-FileHash target/release/syndocal.exe -Algorithm SHA256
cmdkey /list
git -C C:\Users\kouty\Desktop\rb-output status --short
node tools/show-structural-preflight.mjs target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha10-same-pc-output.sdc
```

- Historical alpha.38 native authority remains immutable. The exact MSVC
  `14.44` native gate ran `pnpm --dir app tauri build --no-bundle` in `3m12s`
  with first-party warnings `0`. The resulting exact executable
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
  `61,114,368` bytes, reports Product/FileVersion `1.2.0-alpha.38`, and has
  SHA-256 `9E5CA0DB826D9998F7CDC76214E5CDC17597A6D4FC1CC27FEB5EF72D6E28FA37`.
  Its pushed source/build HEAD was
  `e4ec22384675aace5ed3912ddffdcfecca190919`. Immediately before the build,
  exact-path verification stopped only old checkout-owned PID `50864`;
  Daslight PID `42752` was path-verified and preserved. Exact PID `55624` is
  responsive with one `Syndocal` window, and Computer Use plus Alt+Space
  verified that it is maximized (`Maximize` disabled, `Restore` enabled).
  This proves alpha.38 native build/window only; its binary must not be
  relabeled as alpha.39. The alpha.38 authoring run completed
  Click/Guide-enabled Play -> Pause -> Play -> Pause without a CUE fault, but
  after Pause the authoring-output `<select>` displayed `3 - PX160 WAV...`
  while settings JSON and resolved output remained `Music (Elgato Virtual
  Audio)`. Preserve that drift as historical fail-closed evidence; alpha.39
  native/UI reverification was pending at that checkpoint and is now complete
  as recorded above. Audible confirmation remains open.

- The first alpha.38 native resume action was exercised on PID `55624` with the
  saved reference
  `target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha9-reference-audio.sdc`
  opened from the recent-projects menu. Settings JSON remained
  `route=explicit_device` and `device_name=Music (Elgato Virtual Audio)`. With
  Click, one-measure mode, and Guide enabled, Play -> Pause -> Play -> Pause
  tracked the Timeline bottom playing/paused state; the authoring monitor stayed
  `実行中`, `rev1`, with frames `9452160` (before start) -> `180000` (re-Play)
  -> `157440`/`1645920` (after Pause while output-frame publication continued).
  Resolved output remained `制作モニター出力: Music (Elgato Virtual Audio)` and
  no Backend fault, Local IPC error, or Cue fault appeared. This is native
  UI/runtime regression evidence only; audible confirmation is still open.

- Historical alpha.37 source train is `1.2.0-alpha.37` on branch
  `codex/syndocal-v1.2`. Checkpoint
  `1636aeb440c718c628a953b58e4f0c59d4874e35` and native-build source HEAD
  `5626a9636003462a23daf0f3de67af3cc5060e29` are pushed; its pre-tranche base was upstream-equal HEAD
  `901e4e09f829705ff488511c53e50f51a511e088`. The checkpoint integrates
  Timeline CUE anchor linearization and the
  strict fixed-name Show Spout first-black/reaping/retirement-ACK state
  machines. Exact MSVC 14.44 combined focused gates pass Timeline CUE `49/49`,
  media-audio `46/46`, Show Spout `17/17`, production R4 interleaving `1/1`,
  and generic strict-name retry `2/2`, with first-party warnings `0`.
  Complete exact-linker integration passes `1211/0/7 ignored` without default
  features and `1276/0/12 ignored` with Spout; TypeScript and the Vite
  production build also pass, and exact ASIO Timeline CUE passes `59/59`.
  `check:release` passes after synchronizing product metadata to alpha.37.
  Independent Terra xHigh production review is GO for both diffs; audio is
  P0/P1/P2 `0`, while Spout retains two explicit P2 proof-only rows for direct
  pending-publish reservation and non-synthetic post-join cleanup error.
  Combined Terra xHigh review initially returned NO-GO for a release-only
  `dead_code` warning in the deterministic source-token field. That complete
  seam is now `cfg(test)` with no fake production read; release check is
  warning-free and final rereview is GO with P0/P1/P2 `0`. A first post-fix
  full Spout run saw one transient `Busy` in the unrelated
  production-remote-stop test; its exact filter then passed three consecutive
  runs and a fresh full Spout run passed `1276/0/12`.
  The alpha.37 native build/window gate is now complete as recorded below.
  Audible `Music (Elgato Virtual Audio)`, Unity/GPU, camera hardware,
  installer/updater, dedicated Show-ASIO, DJ hardware, and final
  show-completion acceptance remain open.

- From clean pushed HEAD `5626a9636003462a23daf0f3de67af3cc5060e29`,
  the exact MSVC `14.44.35207` Community linker was pinned and first in
  `where.exe`; Git's incompatible linker remained second. `pnpm --dir app
  tauri build --no-bundle` completed in `3m04s` with first-party warnings `0`.
  `target/release/syndocal.exe` is `61,116,416` bytes, reports
  Product/FileVersion `1.2.0-alpha.37`, and has SHA-256
  `7AC54394E41751126911E6DC338536B93E484A20934B4CA9A001EB8B9F3E209E`.
  The exact-path pre-build gate stopped only historical checkout-owned
  alpha.36 PID `109972`; Daslight PID `42752` was path-verified and preserved.
  An initial Computer Use accessibility-window launch attempt left no process;
  direct launch of the same exact executable then succeeded. Exactly one
  checkout-owned alpha.37 PID `50864` is responsive with title `Syndocal`.
  Computer Use verified Restore enabled and Maximize disabled in its system
  menu, proving the exact window is maximized. The visible runtime status
  `trust_network_absent` keeps DJ Link auto-start and DJ readiness explicitly
  open. This closes native build/window only, not physical acceptance.

- Alpha.36 source checkpoint `ffdb289da1a3980883807a83b8074f0247ab3ea9`
  (`Localize same-PC show controls for alpha.36`) is pushed to
  `origin/codex/syndocal-v1.2`; immediately after push, local `HEAD` and
  upstream were exactly equal and the only dirty path was this handoff update.
  Product metadata is synchronized at `1.2.0-alpha.36`. Focused localization
  passed `3632/3632` (`100.0%`) with unprotected user-data labels `0`; the
  exact `Syndocal Background` and `Syndocal Foreground` identifiers have direct
  `data-no-localize` source assertions. Release metadata, output-control R4,
  video routing R4, the exact 439-command frontend invoke inventory,
  TypeScript/Vite production build, Timeline context-menu, five I/O
  remote-scroll viewports, format, and diff gates passed. Independent Terra
  xHigh rereview is GO with P0/P1/P2 `0`.

- From clean pushed HEAD `81a7a9cccd93d40c3f40a697bc3aad6d3616aded`,
  the exact MSVC `14.44.35207` Community linker was pinned and first in
  `where.exe`; Git's incompatible linker remained second. `pnpm --dir app
  tauri build --no-bundle` completed in `3m30s` with first-party warnings `0`.
  `target/release/syndocal.exe` is `61,049,856` bytes, reports
  Product/FileVersion `1.2.0-alpha.36`, and has SHA-256
  `150C30FE218C6C32A50169EFE7638AD0C5D1CDD8DCFDF5BCFCCEB6434DD4E79E`.
  The path-verified historical alpha.35 PID `72448` alone was stopped before
  the build. At that historical checkpoint, exactly one checkout-owned
  alpha.36 PID `109972` was responsive with title `Syndocal`, and Win32
  `IsZoomed` verified its exact window was maximized. This proves the
  historical native/window gate only; no Unity/GPU, audible
  device, DJ, or camera hardware acceptance is inferred.

- Historical alpha.35 source checkpoint `c7218c112b296652fa77f88c67e861c18d6aea1b`
  (`Harden same-PC show output isolation`) is pushed to
  `origin/codex/syndocal-v1.2`; immediately after push, local `HEAD`, upstream,
  and that hash were equal and the worktree was clean. The exact MSVC 14.44
  alpha.35 native build then ran from clean pushed HEAD
  `31a577cfaad54c0ea981a0fc98a4254b0f5cb8db`, after path-verifying and stopping
  only the historical checkout-owned alpha.34 process PID `85492`. It completed
  in `3m36s`, first-party warnings `0`; `target/release/syndocal.exe` is
  `61,045,760` bytes, Product/FileVersion `1.2.0-alpha.35`, SHA-256
  `1144DBFA1AAD3E46E88F6F3A026218B8B7AB3C3E9A234270B75EDB41213A0E7A`.
  Exactly one checkout-owned PID `72448` is responsive. Computer Use did not
  expose the window as targetable and explicit-path recovery reported an
  unready accessibility window-opened handler; maximized visual proof remains
  open.

- Historical alpha.38 product metadata is `1.2.0-alpha.38`; source/build HEAD is
  `e4ec22384675aace5ed3912ddffdcfecca190919`. The normal native artifact and
  responsive/maximized window identity are recorded above. Alpha.37 and earlier
  processes are historical execution evidence; the exact alpha.38 PID `55624`
  is historical native authority only. At that historical checkpoint, alpha.39
  was the then-current native checkpoint, not current source authority. Its
  native checkpoint used build source/docs `HEAD` and upstream
  `ec93e9160da853ad181de70aee4db7b4a75fafbb`. The reviewed native-evidence
  record is pushed at `94b362bd2d733e447feabf0a0a6158699da6a2bf`; the exact
  alpha.39 artifact and responsive maximized window are recorded at the top of
  this handoff.
  The show boundary is same-PC
  only: exact Art-Net `127.0.0.1:6454` plus local Spout; remote Art-Net and NDI
  are outside this acceptance scope. Alpha.35 rejects pre-existing U0 input
  before strict activation, rejects active U0 HTP/LTP input without insertion,
  and skips stale U0 merge at the final render fence. U1 and ordinary
  non-strict merge remain supported. Exact MSVC 14.44 focused proof passed
  `12/12`; the full engine gate passed `924/0/2`; first-party warnings were `0`.
  Independent Terra xHigh rereview returned GO with P0/P1 `0`. Two additional
  public-ACK/non-strict-U0 tests remain P2 proof debt. The strict Spout pair now
  requires each worker's first physical frame to be cached 1920×1080 opaque
  black, exact post-send SDK names, both first-black ACKs, and a final R4
  authority revalidation before active/live handoff. Focused Spout passed
  `13/0/0`; full Syndocal passed no-default `1205/0/7` and default libav/Spout
  `1265/0/12`; TypeScript, production build, release, format, and diff gates
  passed with first-party warnings `0`. Independent Terra xHigh rereview is GO
  with P0/P1 `0`. The alpha.35 native build passed as recorded above; physical
  Unity observation remains pending.

- Alpha.38 post-build storage evidence records `110,353,719,296` free bytes on
  `C:`. A fresh read-only traversal reports `target` as `94,214,143,142`
  logical bytes across `86,760` files, `app/node_modules` as
  `545,338,492` bytes across `3,704` files, `tools/asio-bridge/target` as
  `1,774,985,879` bytes across `5,560` files, and `app/dist` as `5,203,962`
  bytes across `306` files. Git reports one `568 KiB` temporary-object garbage
  item. No deletion target is currently accepted for this checkpoint
  (`Candidates=[]`); no deletion ran and reclaimed logical bytes remain `0`.

- The complete same-PC Art-Net/camera/fixed-Spout source checkpoint is a
  **historical** checkpoint, clean and pushed at
  `138f6c3e7bd536c10a589bc644bb9bc6df269f7a` on `codex/syndocal-v1.2`. At that
  time its show-output clean break replaced the retired show-only USB/Enttec
  activation with a payloadless, locally confirmed Art-Net route fixed at
  `127.0.0.1:6454`, wire U0, 512 bytes, and no serial field. That old transport
  decision is superseded by the current boundary above: USB-DMX is the physical
  show-lighting primary and Art-Net is the simultaneous Unity mirror. The
  historical sender still masks DMX channel 500 to zero immediately before
  packet encoding.
- Exact MSVC `14.44.35207` focused evidence after the final rollback/UI repair:
  camera catalog `21/21`, engine Art-Net `7/7`, Syndocal Art-Net `3/3`, and
  protocol control-plane `1/1`; failures and ignored tests are zero and
  first-party warnings are zero. The historical proof observes ch1/ch5 at
  payload bytes 0/4 and ch500 zero at payload byte 499 in a local loopback
  packet, not an electrical venue-wire capture. Independent Terra xHigh review
  is GO after closing the stale Setup I/O fixture and publication-failure
  rollback proof. Ox was unavailable for this narrow review exception.
- The historical alpha.34 tranche established the fixed two-Sender Spout
  state/validation baseline. Historical alpha.35 superseded it with the strict
  first-physical-black and post-send exact-name barrier recorded above;
  focused tests, independent review, and the native build passed. Physical
  Unity/GPU observation remains open and is not inferred from source evidence.
- The non-overwriting authoring tool received independent Terra xHigh GO after
  post-write cleanup was changed to retain an unverifiable target for explicit
  quarantine/manual removal instead of risking pathname-based deletion. It
  created `DSF2026-show-alpha10-same-pc-output.sdc` as a separate `1,098,035`
  byte file with SHA-256
  `DB1C18DCEFC79F5DC8C68589BCCAA492AF2509E932542D4A5036927B5E0814BA`.
  Alpha9 and both managed MP3 sidecars retained their pinned hashes.
- C: reached zero free bytes during the focused build. A dry run proved that
  Cargo profile `dev` alone owned `205,503` generated files / `400.0 GiB`.
  `cargo clean --profile dev` removed only those regenerable development
  artifacts, retained `target/release`, QA evidence, source, and authored SDC
  files, and restored `188,021,350,400` bytes free. The subsequent exact-linker
  focused builds regenerated their required graph successfully.
- The authoritative same-PC output gate is
  [qa/DSF2026_SAME_PC_OUTPUT_ACCEPTANCE_2026-08-29.md](DSF2026_SAME_PC_OUTPUT_ACCEPTANCE_2026-08-29.md).
  Remote Art-Net/NDI is deliberately out of the current acceptance scope.

- KDMX checkout: `C:\Users\kouty\Documents\KDMX`
- Branch: `codex/syndocal-v1.2`
- Last pushed alpha.30 source checkpoint: `9c9a96da6955f1ee0098468f30e16221ccaf779e` (`feat: add hybrid ASIO program and WDM cue routing`). It was pushed to `origin/codex/syndocal-v1.2` with exact upstream equality. No alpha.30 native artifact is accepted yet.
- Historical alpha.31 source authority: `602b96a8fcb0de3fd3a3e281324550fe1d7b5630` on `origin/codex/syndocal-v1.2`. It contains the Timeline authoring monitor plus the reviewed normal-build cfg correction.
- Pushed pre-alpha.32 Timeline-audio component checkpoint, included by the
  historical alpha.32 integration: `fedf6c48fbab59b3f0c643da402fcc026ee74fbe`
  on `origin/codex/syndocal-v1.2` (`Synchronize Timeline audio varispeed clocks`).
  Its own product metadata was still alpha.31; the synchronized alpha.32
  product identity begins with the subsequent version checkpoint. Child
  Timeline PROGRAM/CUE now uses one canonical 250..=4000 millirate for
  position, Rodio speed, drift, and inverse Sink seek; nested fractional rates
  are rounded once, ambiguous/invalid rates refuse audio attachment, and a
  negative `source_offset_ms` remains silent until its exact root-output
  boundary. Lighting keeps its established position path.
- The checkpoint's independent Terra xHigh rereview is `GO` with no P0/P1/P2. Supervisor evidence is exact MSVC 14.44 with the Community linker first and absolutely pinned, engine Timeline audio `31 passed / 0 failed / 0 ignored`, frontend Timeline-audio contract PASS, `cargo fmt --all -- --check` PASS, `git diff --check` PASS, and first-party warnings 0. The implementer also recorded ASIO media playback `80/80` and protocol nonserialization `1/1`, both warnings 0.
- The Timeline-audio component checkpoint itself had no new native or hardware acceptance. Its preserved pre-checkpoint process PID `73380` was later path-verified and terminated immediately before the alpha.32 native build; it was never relabelled as current evidence.
- The Windows candidate-gate source checkpoint is pushed at `53d70baacbc2c9ed3f719eaf2e67aab1766cf487`. Metadata completes all hash and updater-signature checks before any EXE inspector; candidate and manual three-root inspectors receive only verified-byte `wx` copies; NSIS/MSI/outer/nested archive tools likewise receive only pre/post-hashed copies. Independent review is implementation `GO`, while actual RC acceptance remains `NO-GO` until real signed bundles and Windows runtime identity evidence exist. Supervisor self-tests passed `125/43/4/140` plus `check:release`; Node's non-handle-atomic pathname and unprovable owner/DACL boundary is explicitly limited to fresh single-writer staging.
- The synchronized alpha.32 product identity is pushed at `5d40874c629f26d6e011252622b2886d39d8d40b`. From that clean pushed HEAD, the exact MSVC 14.44 Community linker was pinned and first in `where.exe`; `pnpm --dir app tauri build --no-bundle` passed in `2m 52s` with first-party warnings 0. `target/release/syndocal.exe` is `60,713,984` bytes, Product/FileVersion `1.2.0-alpha.32`, SHA-256 `B04CE351A456C82715383A4430401F3DA4D824EC8813B32894ABE1BF98FBD90D`. With the verified local FFmpeg runtime `bin` inherited, exactly one checkout-owned process launched as PID `89524`, is responsive with title `Syndocal`, and Computer Use verified the exact window is maximized. Audible and physical acceptance remain open.
- The alpha.34 integration includes the alpha.33 camera-capture source tranche, which replaces the old free-form DirectShow route fixed at `1280x720` / `30 fps` with an explicit current-generation device/profile catalog, opaque endpoint identity, and an exact one-frame probe before Add. Its envelope is maximum `4096x2160`; profiles above `1920x1080` admit no more than `30 fps`, profiles above `1280x720` no more than `60 fps`, and capture rates up to `120 fps` only when advertised. Output presentation remains at most `60 Hz`; screen capture stays `1280x720` / `30 fps`. The exact supervisor source gates passed capture `64/0/2` and control-plane `64/0`, with MSVC 14.44 and first-party warnings 0. Independent review found P0 `0`; source fixes close bounded listing memory, child cleanup, and automatic fault-row visibility, while sustained-4K performance remains open. The native alpha.34 build passed; visual UI and hardware probes remain pending. The acceptance authority is [qa/CAMERA_INPUT_ACCEPTANCE.md](CAMERA_INPUT_ACCEPTANCE.md).
- Alpha.32 cleanup inventory was refreshed read-only. The four known build/cache roots still exist; a new exact-byte traversal of `target` exceeded the bounded 50-second window and was stopped without changing files. The latest completed exact `target` inventory therefore remains `428,244,808,551` logical bytes. The reviewed recurring-cleanup harness is not approved for Apply, so no deletion ran and reclaimed bytes remain 0.
- The documentation checkpoint containing this record is accepted only after
  its commit is pushed, `HEAD` equals `origin/codex/syndocal-v1.2`, and the
  primary worktree is clean; the close procedure rechecks all three conditions.
  Three detached alpha18/19 release-gate worktrees are clean. The
  separate `C:\Users\kouty\Documents\KDMX-asio-persistence` worktree retains
  its pre-existing owned changes in `app/scripts/check-backend-operator-contract.mjs`,
  `check-frontend-command-routing.mjs`, `check-live-audio-input.mjs`,
  `app/src-tauri/src/control_plane.rs`, `app/src-tauri/src/main.rs`,
  `app/src/App.tsx`, `app/src/tauri-invoke-manifest.json`,
  `app/src/tauriInvokeCommands.ts`, `app/src/types.ts`,
  `qa/ASIO_INPUT_ACCEPTANCE.md`, `qa/harnesses/README.md`, and
  `qa/harnesses/check-asio-build.ps1`; none was touched by this checkpoint.
- Preserved stashes are `stash@{0}` (`e9209d6` alpha.9 validation WIP) and
  `stash@{1}` (orphaned open-DMX pacing WIP). Neither was applied or modified.
- Historical alpha.29 source checkpoint: `54a4ffcce0e2029d9f0aecc713ae4436228a4d3c`, committed as `fix: stabilize ASIO output lifecycle` and pushed to `origin/codex/syndocal-v1.2` on 2026-08-29 JST.
- DJ Agent checkout: `C:\Users\kouty\Desktop\rb-output`
- Branch: `beta-v1.1.2`
- DJ Agent committed HEAD and upstream: `a13d7bff59db5e7c00e19655f87c69db7cb52005` on `beta-v1.1.2`; its worktree was clean at the recorded checkpoint.
- The DJ Agent operator-return path received independent source-review `GO` with no P0/P1/P2. The external full regression passed `506 tests / 504 passed / 0 failed / 2 skipped` with first-party warnings 0. DJ-PC pull/restart, strict preflight, active runtime version, real ACK, and physical pedal acceptance remain external gates.
- At that historical checkpoint, KDMX product metadata was `1.2.0-alpha.37`.
  Alpha.32 source integration
  and its native build, launch, and maximized-window gate remain historical.
  The alpha.34 same-PC output/camera native build, alpha.35 U0/Spout-hardening
  native build, and alpha.36 source/native/window authority are also
  historical checkpoints after that source bump. The alpha.37 native
  authority is exact checkout PID `50864`, built from clean pushed HEAD
  `5626a9636003462a23daf0f3de67af3cc5060e29` and verified responsive and
  maximized. Real-device audition, physical output routing,
  camera hardware, DJ acceptance, show completion, real installer/updater
  inspection, and the dedicated show-ASIO artifact remain open. The camera
  acceptance authority is [qa/CAMERA_INPUT_ACCEPTANCE.md](CAMERA_INPUT_ACCEPTANCE.md).

### 2026-08-29 historical alpha.34 same-PC output and camera integration

The product identity is `1.2.0-alpha.34`; source and native build identity are
fixed, while visual and hardware acceptance remain open. The old camera route accepted a free-form
DirectShow endpoint at fixed `1280x720` / `30 fps`. The new source path uses an
explicit current-generation DirectShow device/profile catalog, persists only an
opaque endpoint identity, and requires an exact one-frame probe before Add. The
envelope is maximum `4096x2160`; profiles above `1920x1080` admit no more than
`30 fps`, profiles above `1280x720` no more than `60 fps`, and capture rates up
to `120 fps` only when advertised. Output presentation remains capped at
`60 Hz`; screen capture stays `1280x720` / `30 fps`. The acceptance authority is
[qa/CAMERA_INPUT_ACCEPTANCE.md](CAMERA_INPUT_ACCEPTANCE.md).

On 2026-08-29, the connected `Insta360 Link` advertised `3840x2160` at
`30 fps`; `1920x1440`, `1920x1080`, and `1280x720` at `60.0002 fps`; and no
`120 fps` profile. The final exact supervisor source gates passed
capture-filtered `71 / 0 / 2`, process-lifecycle `3 / 0 / 0`, control-plane
`64 / 0 / 0`, full no-default `1203 / 0 / 7`, and full default
`1239 / 0 / 12` under MSVC `14.44.35207`, with first-party warnings `0`.
The first parallel no-default run exposed two unrelated coordination-test
timeouts (`1201 / 2 / 7`); both passed individually and the full serialized
suite passed without a source change for those tests. Independent Terra xHigh
rereview returned GO with P0/P1/P2 `0`; source fixes close bounded listing
memory, post-spawn child cleanup, deferred reaper/quarantine ownership, and
automatic stale Active-row replacement. App exit before a deferred reaper
finishes remains an explicit unverified non-Job-Object OS boundary. Sustained-4K
performance remains open.

Direct FFmpeg preflight completed 150 RGBA frames at 4K30 and 300 at 1080p60,
both exit `0`. The exact Syndocal profile probe still proves one frame only.
Sustained 4K remains unverified:
the current RGBA `Vec` clone at `60 Hz` may approach `1.98 GiB/s` of copy
traffic. Native alpha.34 UI and hardware probes remain pending; no visual or
hardware completion is claimed here.

At pushed checkpoint `fd0d40698ae849a0f327fa7769b06b2af182c82f`, the
camera/Art-Net independent source rereview was GO with P0/P1/P2 `0`. That
checkpoint's full app regression passed `1203/0/7` without default features
and `1239/0/12` with default libav/Spout features. Those counts predate the
current fixed two-Sender Spout integration and are not its full-regression
evidence.

The historical committed alpha.34 Spout integration has focused exact-linker
evidence of Syndocal show-Spout `20/0/0`, engine strict-pair `5/0/0`, and
protocol v4 command `11/0/0`, all with first-party warnings `0`. It holds both
senders on opaque black until the durable output lease commit, binds every
send to the exact output-ownership generation/epoch and project callback
epoch, blocks generic mutation of the fixed pair, and preserves unresolved
engine-retirement identity until a later R4 reconciles it. That barrier is
process-session scoped: process exit destroys the SDK senders, engine instance,
and pending ACK queue and is the explicit recovery boundary for an infinite
driver call. Independent Terra xHigh final rereview returned GO with no
unresolved P0/P1. P2 proof debt remains for a true two-worker fake-SDK timing
test, and the new show modules only partially reduce the existing oversized
app/engine orchestration files. Final exact-linker regression passed Syndocal
no-default `1205/0/7`, Syndocal default `1263/0/12`, and engine `920/0/2`;
the remaining workspace crates exited successfully and first-party warnings
were `0`. The initial matrix found only a stale expected command count and an
older unreferenced Cue 3 test fixture. Explicit command-variant assertions and
removal of the dead fixture closed both; focused reruns and the full matrix then
passed. A fresh exact-linker native build from clean pushed HEAD `138f6c3`
completed in `3m45s` with first-party warnings `0`. The resulting
`target/release/syndocal.exe` is `61,039,104` bytes, Product/FileVersion
`1.2.0-alpha.34`, SHA-256
`1FCB899E2B118B94F92B5D87ECD7A5EA3FFE32D33448319FE697D39841FA642F`.
Exactly one checkout-owned process, PID `85492`, launched and remained
responsive. Computer Use did not expose that native window as targetable after
one explicit-path recovery attempt, so maximized-window proof and Unity/GPU
observation remain pending; no visual native acceptance is inferred from the
responsive process alone.

Frontend production build, typed command/output-control checks,
`check:release`, localization/IPC inventory, format, diff, and full Rust gates
pass for the frozen source checkpoint. No native or physical output claim is
promoted from these deterministic results.

### 2026-08-29 historical alpha.30 hybrid PROGRAM/CUE source checkpoint

- The dirty alpha.30 source/UI tree now carries two explicit delivery modes:
  `CueDelivery::SameAsio` keeps PROGRAM and CUE on one selected ASIO stream and
  shared clock; `CueDelivery::ExplicitWdm` keeps PROGRAM on ASIO while the
  explicitly named WDM endpoint, including its enumerated topology fingerprint,
  owns CUE.
- Timeline CUE clips, generated Click/Guide, and the explicit CUE test path
  are wired through the same selected logical CUE route. Missing, ambiguous,
  stale, changed, or failed endpoint/session state is visible and fail-closed;
  no ASIO/PROGRAM/default-device fallback or CUE leakage to PROGRAM is allowed
  by the source contract. Session/generation fences cover activation,
  publication, timeline preparation, and retirement.
- Exact MSVC 14.44 Community-linker gates finished with first-party warnings 0.
  The focused ASIO media-audio gate passed `64/64`; the final full
  `cargo test -p syndocal --features asio -- --nocapture --test-threads=1`
  passed `1435 / 0 failed / 12 ignored`. `cargo fmt --all -- --check` and
  `git diff --check` passed (line-ending notices only).
- TypeScript, Vite, frontend command routing, audio-control `74` static plus
  `57` runtime assertions, audio-panel `53`, localization `3615/3615`, and
  `check:release` passed. Release checking included packaging `169`, ASIO v3
  `22`, and Timeline output-bus `11` assertions. Independent Terra xHigh source
  review returned GO with no P0/P1; physical WDM audition and independent-clock
  observation remain external.
- No alpha.30 native build/launch/window, real-device audition, physical output
  routing, or show completion has been verified. The oversized audio-runtime
  extraction from `app/src-tauri/src/main.rs` is deferred until after show
  acceptance because changing ownership/lifecycle boundaries before the show
  is a pre-show risk; no module-split completion is claimed.
- The next show-critical source item is a separate Timeline-authoring monitor:
  while the output router is Normal, media-library Timeline clips plus generated
  Guide/Click must share one explicitly selected WDM endpoint such as
  `Music (Elgato Virtual Audio)`. Entering show ASIO must retire that authoring route;
  missing, ambiguous, or changed endpoints must remain silent and fail closed.

### 2026-08-29 historical alpha.31 Timeline-authoring monitor checkpoint

- The Normal route now has an explicit Timeline-authoring monitor. `FollowProgram`
  preserves the existing Normal behavior; `ExplicitDevice` sends every Timeline
  media clip (logical PROGRAM or CUE) and generated Guide/Click material to one
  operator-selected WDM endpoint. The endpoint is arbitrary and machine-local,
  with exact name/topology revalidation before publication and preparation.
- Missing, ambiguous, stale, changed, or failed endpoint/session state remains
  visible and silent. There is no default-device, PROGRAM, or Show-ASIO fallback.
  Entering Show ASIO retires all Normal-authoring Timeline sinks, even when no
  normal PROGRAM stream is open; returning to authoring requires an explicit
  Normal route and output selection.
- Exact MSVC 14.44 Community-linker gates completed with first-party warnings
  0. The ASIO-enabled full suite passed `1456 discovered / 1444 passed / 0
  failed / 12 ignored`; focused media-audio passed `72/72`. TypeScript, Vite,
  release metadata/checking, audio-output control/panel, Timeline-audio,
  command-routing, and localization checkers passed.
- The normal alpha.31 native no-bundle build passed from clean pushed HEAD
  `602b96a` using exact MSVC 14.44. `target/release/syndocal.exe` is
  `60,756,480` bytes with SHA-256
  `6F9BF17A2802A2FC5F8935E8EFFEB0C57CA4A3A21B245C68BFA62315152C7F4A`.
  Exactly one launched process was responsive. The exact native window was
  maximized through its verified process handle. Product-path enumeration found
  exactly one selectable `Music (Elgato Virtual Audio)` endpoint with topology
  fingerprint
  `A2D9603C75ED1A6ECBC37F0FE851AAD56C632DFF31314544F2F606C220C9479C`.
  The exact selection was persisted at
  `%LOCALAPPDATA%\jp.seraf.ktn.syndocal\timeline-cue-audio-settings.json` while
  Syndocal was stopped, followed by a successful relaunch; audible output is
  not yet verified. The responsive maximized process at that checkpoint was PID `100320`.
  Physical output, venue
  routing, DJ Link, serial DMX, reconnect, and show completion remain external
  gates. Ox was unavailable for this tranche; under the documented narrow
  exception, an independent Terra xHigh review returned GO with no P0/P1.
- The oversized audio-runtime extraction from `app/src-tauri/src/main.rs` is
  intentionally deferred until after show acceptance; no module-split
  completion is claimed. The next safe action is the real endpoint audition.
- A subsequent source-only I/O clarity delta keeps the backend selector visible
  in Normal WASAPI, hides Show-ASIO-only lifecycle/channel/preflight controls,
  identifies PROGRAM as the Windows default output, and directs arbitrary WDM
  Timeline media/Guide/Click routing to Timeline tools -> Timeline authoring
  monitor -> Explicit Device. Its focused panel checker passed `58` assertions,
  localization passed `3616/3616` with zero unprotected labels, TypeScript
  passed, and independent Terra xHigh review returned GO with no P0/P1/P2.
  Native visual acceptance of this delta remains open.
- The first normal native attempt exposed a non-ASIO-only compile defect: the
  ASIO-gated `route_gate` declaration had one unconditional `drop`. The new
  path applies the identical cfg to that drop, preserving ASIO lock order while
  removing the undefined name from normal builds. The one-line fix is commit
  `602b96a`; an independent Terra xHigh rereview returned GO with no P0/P1.

### 2026-08-29 historical alpha.29 ASIO lifecycle safety checkpoint

- `AsioReady` can now return explicitly to Normal or be invalidated by profile
  reselection. The exact Ready ticket is cancelled first, leaving `Locked`
  with no operation/session; a validated Start is then rejected until an
  explicit revalidation. Reselect/Revalidate/Start/Stop/Normal selection are
  serialized by one lifecycle lock. Active or otherwise ambiguous states
  remain rejected without mutation.
- The show-ASIO first-party warning inventory is zero without warning
  suppression. Test-only router/result/proof helpers are compiled only for
  tests; production generation and live-audio fences remain present.
- Independent adversarial review returned GO with P0/P1 zero. Its only code
  observation was a stale lock comment, corrected before this commit.
- Exact MSVC 14.44 preflight and pinned Community linker were used. The
  show-ASIO release check completed with first-party warnings 0. The complete
  show-ASIO application test run discovered 1409 tests and finished 1397
  passed / 0 failed / 12 ignored. The frontend audio-output contract passed
  59 static assertions and 28 runtime assertions; TypeScript/Vite transformed
  302 modules. `check:release` passed including packaging 169, ABI v3 22, and
  Timeline output-bus 11 assertions. `cargo fmt --all -- --check` and
  `git diff --check` passed (Git line-ending notices only).
- `pnpm --dir app tauri build --no-bundle` completed successfully with
  first-party warnings 0 and produced the alpha.29 release executable.
  Native launch acceptance is still open: the exact process was responsive
  and setup tracing reached the end of the Tauri setup callback, but only the
  internal 16x16 single-instance window was exposed and no user-facing
  `Syndocal` window appeared. The temporary trace instrumentation was removed.
  This checkpoint therefore makes no native-window GO claim.
- The user has explicitly expanded the output requirement beyond the original
  one-device contract: PROGRAM and CUE must be independently assignable to
  physical output devices. Immediate acceptance target is TOPPING E2x2 ASIO
  outputs 1/2 for PROGRAM and a separately selected headphone/WDM endpoint for
  all CUE sources. Same-ASIO shared-clock mode remains supported; hybrid mode
  must state that it uses two clock domains, must never fall back or leak CUE
  to PROGRAM, and must fail closed on missing or ambiguous endpoints. This
  hybrid path is not implemented or hardware-accepted at the alpha.29
  checkpoint.

### 2026-08-29 historical alpha.28 integrated source checkpoint

- The previously open application integration is now present on the working
  tree. PROGRAM/CUE render, output runtime, Timeline output/transport, output
  router, normal-output boundary, bridge-v3, and preflight logic are split into
  dedicated modules. This historical checkpoint did not extract the remaining
  `TimelineCueAudioRuntime`/`MediaAudioPlayback` ownership from the already
  oversized `main.rs`; that larger application-runtime split is still deferred
  because changing its lifecycle boundary before the show is a pre-show risk.
  The only final `main.rs` repair at this checkpoint was a deterministic
  test-startup fence; production behavior was not loosened.
- The final independent ASIO drain review is `GO` with no P0/P1. Stop fences
  the callback context before join, Fault uses one state-lock interval, stale
  queued Test/Solo blocks are silenced through the two-callback drain, and
  arbitrary invalid-width/partial/non-finite blocks fail to full-width silence.
  Supervisor evidence is `165 passed / 0 failed` for exact `asio_` focused
  tests with first-party warnings 0.
- The full parallel Syndocal show-ASIO suite passed from a fresh rerun after
  repairing two test-only races: `1366` discovered, `1359 passed`, `0 failed`,
  `7 ignored`, first-party warnings 0. The repairs use a test-only router slot
  rather than competing for the production process owner, and wait for the
  engine startup snapshot before atomic no-mutation comparisons. Focused proof
  after repair was `25/25` router, `48/48` Timeline CUE, `3/3` ASIO Timeline
  Play, and `1/1` for each DJ rejection boundary.
- The final accepted default workspace gate ran with the CI-equivalent
  `--test-threads=1` setting and passed `2708` tests with `0` failures and `17`
  explicit hardware/GPU/long-duration ignores. First-party warnings were 0.
  A deliberately broader parallel `--all-features` diagnostic is not counted
  as acceptance: it first lacked the NDI runtime DLL path and later exposed
  unrelated parallel native/socket-process instability. The exact `io` suite
  passed serially (`166 passed / 0 failed / 2 ignored`), and the accepted full
  workspace rerun is the serial result above.
- Release metadata and frontend contracts passed: ASIO packaging `169`, ABI v3
  `22`, Timeline PROGRAM/CUE UI `11`, project-open bootstrap, video output
  routing/window observation, and TypeScript/Vite production build with `302`
  transformed modules. `cargo fmt --all -- --check`, three Show-ASIO script
  syntax checks, and `git diff --check` pass; first-party warnings remain 0.
- The local-only artifact authority is now split into artifact source commit
  `S`, evidence HEAD `E`, and source branch `B`. The checker rejects dirty or
  unpushed evidence, a wrong branch, non-ancestor `S`, merges, D/R/T changes,
  non-allowlisted evidence paths, and source mutation followed by revert. Its
  schema is a clean-break v3 with manifest `commit=S` and `sourceBranch=B`.
  The final source identity contains `70` exact paths, including
  `app/src/uiLocalization.ts`; every Git authority query rejects repository,
  index, object, and config overrides and disables replacement objects.
  Supervisor self-tests pass: checker `155`, runtime staging `16`, build
  orchestrator `43`, and three-display harness `90/90`, with native/Cargo/
  process actions explicitly not run by those self-tests. Independent final
  rereview is `GO` with no P0/P1. PATH-resolved Git executable pinning remains
  non-blocking P2 hardening.
- Exact normal native acceptance passed after removing two release-only
  dead-code warnings with correct cfg boundaries: `pnpm --dir app tauri build
  --no-bundle`, first-party warnings 0. The exact executable is 60,599,296
  bytes, SHA-256
  `3080A3664B4657D6B6D6EFE76448DBE13912CA118262E78C0AE02B8DC6E85C6B`.
  Exactly one process at this checkout path exposed exactly one responsive
  `Syndocal` window and was verified maximized. It was then terminated by exact
  resolved path before the final tests. This does not claim normal installer/
  updater inspection or the separate show-ASIO artifact.
- No MOTU M4 stream, M32/DL16 route, or USB-DMX physical output is claimed yet.
  In the **2026-08-28 alpha.30 machine snapshot**, this machine exposed no MOTU
  M4 device and no serial port; that observation is historical and does not
  describe the current show PC. Those physical rows must be run on the equipped
  show system. The integrated source
  checkpoint is committed and pushed at
  `32a092267f55d32185b2cf9cc123f92067614ec1`.

## Accepted source boundaries

### Pedals and DJ Link source boundaries (not physical acceptance)

- Pedal 1 / F13 owns the current loop toggle in Timeline-control mode.
- Pedal 2 / F14 owns loop-half in both modes: Rekordbox MIDI loop-half in DJ-control mode and `DJ_TIMELINE_LOOP_HALF` for an active Syndocal Timeline loop in Timeline-control mode.
- Pedal 3 / F15 owns Timeline `+4 bars` only in Timeline-control mode.
- F13 DJ release starts HPF and emits the correlated `DJ_RELEASE` on the same edge. The local Rekordbox action then completes HPF, ChannelFader fade, stop, and reset independently of Syndocal delivery.
- Stage 2 commands require exact Timeline/play-session/release authority and revalidate current playing Timeline identity in the engine worker before mutation.
- Re-enabling a completed loop after position B re-enters at A; disabling does not jump.

Focused source evidence preserved in the committed alpha.28 source checkpoint:

- protocol DJ Link: 14 passed, 0 failed, first-party warnings 0.
- engine DJ Link: 30 passed, 0 failed, first-party warnings 0.
- Syndocal DJ Link: 119 passed, 0 failed, 1 ignored live-network test, first-party warnings 0.
- Independent Stage 2 adversarial review: GO.
- DJ Agent v1.1.10 independent adversarial review: GO. Supervisor focused rerun: 88 passed, 0 failed; `git diff --check` passed before commit.

### Imported fixture stage layout

- `960 sound waves strongpoint` is authoritative as four physical cells and four logical RGB segments. Daslight's twelve displayed cells are the known three-row duplication bug, not twelve physical emitters.
- The Strongpoint collapse requires exact normalized profile identity, twelve raw DVC cells, four logical segments, and the exact three-row duplication pattern. A fixture label cannot trigger the exception; unrelated twelve-cell profiles remain twelve cells.
- Mega Bar remains eight physical cells and eight logical RGBA segments, with separate global dimmer and strobe controls.
- Physical-only layouts remain physical-only; the frontend does not infer color roles from control names.
- Unknown persisted layout fields and duplicate PATCH beam indices fail closed.

Focused source evidence preserved in the committed alpha.28 source checkpoint:

- protocol stage-layout validation: 13 passed, 0 failed, first-party warnings 0.
- DVC stage-layout unit tests: 5 passed, 0 failed, first-party warnings 0.
- duplicate PATCH beam rejection: 1 passed.
- synthetic exact topology import: 1 passed.
- operator-owned `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` acceptance: 1 passed with Strongpoint 4/4, Mega Bar 8/8, and wristband layout absent.
- TypeScript build, Strongpoint browser contract, mapping viewport conformance, stage-label, mapping geometry, and fixture live-color checks: passed.
- Independent physical-layout adversarial review: GO.

### Show authoring and safe publication

- The DSF2026 authoring tool uses a Windows fail-closed safe-write path with exclusive create, reparse rejection, identity revalidation, flush, exact-length verification, and cleanup on failure.
- Independent safe-write review: GO; focused tests 3 passed.
- Fresh native import/save evidence uses the exact operator-owned source `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`. The report is exact at `fixtures=46`, `profiles=12`, `fixture_groups=15`, `cues=2`, `Converted=93`, `Approximate=4`, `Skipped=0`, and `Unsupported=1` for one Daslight hardware binding. The saved base has exact stage layouts: six Mega Bar layouts with 8 cells and 8 logical segments each, plus three Strongpoint layouts with 4 cells and 4 logical segments each.
- Historical alpha.27 Native Save As output artifact: `target/qa/dsf2026-native-alpha27/DSF2026-imported-alpha27.sdc`, `1,079,564` bytes, SHA-256 `B21165A70A41A4036153359E579E1433C2739EC1C3EDCC0C46B94F513238DFB1`.
- Alpha.15 is retired as a current authoring base. Its artifacts remain historical evidence only and must not be supplied to the pinned authoring CLI.
- At that historical checkpoint, the former alpha3-alpha8 candidates were superseded by the reviewed alpha9 reference-audio candidate documented below. The authority at the top of that historical note bounded alpha9 to reference-audio/content evidence and granted no production deployment approval.
- The current supervisor rerun passed the authored-show test, DJ Link frontend/runtime contract, localization `3556/3556` with 0 unprotected labels, stage-label contract, fixture-limit degree contract, Timeline context-menu browser contract, I/O disclosure scroll contract, Strongpoint segment browser contract, Stage Settings viewport contract at five sizes, TypeScript/Vite production build, release metadata, Tauri wrapper self-test, and the `frontend-typescript-vite-windows` warning ratchet. First-party warnings were zero.
- Historical alpha.27 native release evidence: the exact MSVC 14.44 linker gate produced `target/release/syndocal.exe` at `60,314,624` bytes with SHA-256 `CEBB44C713043CCE885D87E3651464F3756A5CEE2D1700728D412AC7B18C48EC`.
- Historical alpha.27 process evidence: exactly one checkout-owned process was responsive after that build, PID `87732`, HWND `124064278`, and the verified Syndocal window was maximized. This is not alpha.28 native acceptance.
- This handoff claims no DJ HELLO/ACK exchange, physical pedal acceptance, or physical DMX-output acceptance; those remain unverified.
- `target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha3.sdc` passed the current structural Timeline preflight, including the one-measure transition and indefinite pedal-release holds.
- That alpha3 artifact is not the final show file because its base predates the newly imported fixture `stage_layout`. Do not deploy it as the final stage-layout authority.

### 2026-08-28 reference-audio authoring delta

- The two operator-supplied MP3 files are now accepted only as rehearsal/reference material through the ordinary Media Library -> Timeline Audio Clip path. They are not click/guide assets and are not armed for show playback.
- The authoring helper preserves both originals, copies byte-identical files beside the generated candidate under ASCII names, records SHA-256/byte size/duration, and refuses overwrite or divergent existing bytes.
- Each authored song Timeline receives one ordinary Audio layer with one `media_asset_id` clip. The layer is muted by default and requires an explicit operator unmute before rehearsal playback.
- At that historical checkpoint, the operator-saved alpha9 candidate `target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha9-reference-audio.sdc` superseded the earlier alpha9 byte identity. It is `1,095,864` bytes with SHA-256 `93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094` and passes the structural preflight. Measured media durations remain `214032 ms` for 人生オーバー and `273432 ms` for 惑う星; source and managed sidecar SHA-256 values match. The authority at the top of that historical note did not approve it as the production operator artifact.
- Rehearsal playback now uses a distinct sibling `DSF2026-show-alpha9-rehearsal-reference-audio.sdc`, `1,092,410` bytes with SHA-256 `AC9133AFD2C9AAD022674B45222447115F624175B2C5E20DE1E803ABAA9778CE`. Its `956`-byte approval manifest has SHA-256 `ADA244FE095B48BC67DE6181F1D9EFE55131EF720A4D2B5EEB91714A7DA85AED`. The reviewed fail-closed creator pins the production source and both MP3 identities, requires that exact manifest, creates exclusively, and permits only the three synchronized `Reference Audio` layer changes from `muted=true` to `muted=false`. Independent post-publication audit confirmed no other semantic JSON differences, both source and copy passed all ten structural checks, and the source candidate and sidecars remained byte-identical. The rehearsal copy has not been loaded into the running native app or audibly accepted.
- Windows publication holds verified non-reparse parent/leaf handles across source and sidecar revalidation through candidate flush. Parent/leaf substitution, same-hash reparse substitution, partial write, and flush failure fail closed without candidate or temporary-directory residue.
- Focused authoring regression passed with first-party warnings 0, including explicit no-reference assertions for zero Timeline Audio Clips and zero added Media Library assets. The final independent Terra xHigh review is `GO` with no P0/P1.
- Alpha9 loaded successfully into the exact native process through the single-instance project-forwarding path and reported 46 fixtures plus the expected 12 embedded profiles and mappings. Timeline Audio Clip playback remains unverified.
- A cold command-line launch with alpha9 exposed an open startup race: `Project authority changed before mutation (expected epoch 0 revision 0)`. Starting Syndocal first and forwarding the project path then loaded successfully. This race is not accepted or hidden; fix it in the next narrow tranche before cold-start deployment is claimed.

### 2026-08-28 machine-local USB-DMX delta (historical source evidence)

- The project retains only the logical serial DMX route. COM/PnP identity is stored in machine-local state and is selected from a current-device dropdown, then persisted only by an explicit Confirm action.
- Missing, stale, ambiguous, renumbered, and A->B->A physical identity changes fail closed. The exact opened Windows handle identity is revalidated rather than trusting a selected port label.
- The S0 blackout epoch is linearized ahead of activation/live ticks. The same physical write gate is held from selected-frame authority through BREAK, MAB, `write_all`, and `flush`; S0-first therefore permits zero live frames, while worker-first permits at most the already-started live frame followed only by zero frames.
- The deterministic worker proof uses the real `EnttecOpenDmxSender` worker and real Open-DMX write sequence with a fake `SerialPort`, including the public `safety_blackout_engage_published` interleave and non-deadlock assertion.
- Focused Rust evidence: `cargo check -p engine`, serial DMX 14/14, engine DMX 29/29, show route 4/4, machine binding 3/3, serial A->B->A 1/1, first-party warnings 0. Frontend output-control, production build, and exact command routing/invoke gates passed.
- Final independent adversarial review is `GO` with no P0/P1/P2. Physical USB-DMX/fixture output remains open.
- Final source gates after the active/bank normalization repair: exact MSVC 14.44 workspace Rust `2683 passed / 0 failed / 17 ignored` across `2700` executed tests, followed by the final exact-gated Syndocal rerun `1199 passed / 0 failed / 12 ignored` across `1211` tests; frontend production build; release/ASIO packaging `169` assertions; Tauri wrapper `231` assertions plus `27` hostile fixtures; exact frontend route/invoke inventories; Stage label, Strongpoint, fixture-limit degree, and remote disclosure contracts. First-party warnings remained 0; `cargo fmt --all -- --check`, `git diff --check`, and `pnpm --dir app run check:release` passed.

### ASIO PROGRAM/CUE output gate added 2026-08-28

- `qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md` is now the authoritative output gate. ASIO output is a show-critical requirement, not optional polish.
- Adopted architecture C: retain the exact input/Reactive Capture ABI/schema v2
  surface only, and use the exact v3 output/full-duplex surface as the sole
  current ASIO playback path in the same canonical bridge DLL. ASIO playback
  uses one v3 session; a parallel v2 session on the same driver is forbidden
  and must fail busy.
- PROGRAM stereo and CUE mono are project-level logical buses. Physical PROGRAM L/R, CUE, optional Spare, driver identity, sample rate, format, and buffer remain machine-local. Missing bus data migrates one way to PROGRAM; CUE never falls back to PROGRAM.
- Two logical Rodio mixers feed a non-realtime renderer and bounded preallocated interleaved SPSC. The ASIO callback only copies a complete block or outputs a complete silent block and latches terminal Fault. PROGRAM/CUE share one device and clock only in `SameAsio`; `ExplicitWdm` intentionally uses independent ASIO and WDM clock domains.
- DSF2026 acceptance mapping is MOTU M4 Output 1/2 = PROGRAM L/R and Output 3 = CUE at exact 48 kHz, with Output 4 optional Spare. This is a selectable profile, not MOTU-specific code.
- Device/rate/mapping conflict, disconnect, XRUN, reset/resync, buffer/rate change, callback gap, or underflow must stop output without WASAPI/default-device/rate fallback. Explicit revalidation and Start are required.
- Independent Terra xHigh review added three implementation-blocking P0 boundaries: quiesce and join the legacy `FollowProgram` CUE/normal Rodio output before v3 Start and stay silent on Start failure/Fault; freeze exact v3 callback/queue/lifetime semantics before code; and prove arbitrary non-contiguous/reordered physical mappings with all unselected channels zero. It also requires exact queue/race injection tests and updates every show-ASIO export checker from the retained v2-input-only set to the retained v2-input-nine plus current v3-output-nine set.
- The acceptance contract was committed and pushed at `e583141cc60decff7c062db21a39f69241f894c8`. At that historical contract checkpoint, the `1.2.0-alpha.28` implementation was still uncommitted; the authoritative 2026-08-29 integrated checkpoint above supersedes that state.
- Integrated source and independent review are complete. A historical pre-alpha.30 normal no-bundle build completed, but its user-facing window gate did not; alpha.30 has no native build/window evidence. Dedicated show-ASIO artifact/loader proof, normal installer/updater inspection, authoritative non-default Timeline speed synchronization, physical output proof, and M32/DL16 routing proof remain open.

### 2026-08-28 alpha.28 ASIO implementation checkpoint (superseded source snapshot)

This subsection preserves the earlier partial snapshot. The 2026-08-29
integrated source checkpoint above was authoritative for that historical
source status; the current alpha.45 source/test status is at the top.

- Source authority remains branch `codex/syndocal-v1.2` at committed/pushed parent
  `8153ebb37a9517aad91c0da6ad06de9a80db2a1a`. The `1.2.0-alpha.28` source below
  is a shared uncommitted working checkpoint; production code integration,
  native acceptance, and physical acceptance remain pending, so no deployment
  or physical acceptance follows from it.
- Portable Timeline Audio Clips now carry only `PROGRAM | CUE`. Missing legacy
  values become PROGRAM, explicit invalid/future values reject, and root,
  child, import, save/reload, split, lane move, and sink source identity retain
  the logical bus. A stale full authoring/bank publication cannot overwrite a
  newer dedicated bus mutation. Bank preservation is keyed by
  `(TimelineId, clipId)`, so two Timelines that reuse a clip ID cannot
  contaminate one another. Independent Terra xHigh rereview is GO for this
  engine boundary; exact MSVC 14.44 focused evidence is 15/15 Timeline audio,
  1/1 strict protocol bus rejection, 1/1 root and child invoke-wire checks,
  1/1 sink retirement, and 11/11 static UI assertions, with first-party
  warnings 0.
- The v3 bridge is an actual ASIO SDK output/full-duplex implementation, not a
  CPAL callback placeholder. It owns explicit driver load/init, exact tuple
  negotiation, one `ASIOCreateBuffers` set, output/full-duplex callbacks,
  `ASIOStart`, terminal faulting, and drained Stop/Close. Full-duplex input and
  output must share the exact sample rate and fixed buffer before driver open;
  both mismatch directions reject without a handle, session, result, or false
  `actualInput`. Independent Terra xHigh rereview is GO with P0/P1 0. Final
  bridge evidence is default 31/31, ASIO 34/34 plus one physical ignored test,
  clippy `-D warnings` in both graphs, and exact v2 nine plus v3 nine exports.
  The current bridge DLL is 926,208 bytes with SHA-256
  `B21713C5C3F4A3ECA1C3A12769E05076136A43D40D7FBF225DDE525D6792A294`.
- The isolated `AudioOutputRouter` core received independent Terra xHigh GO.
  It provides process-wide ownership, sealed/ticket-bound source creation and
  recovery, late-prepare retirement, retryable exact resource teardown,
  monotonic callback drain, explicit Ready cancellation and return to Normal,
  and fail-closed identity exhaustion. Its standalone proof is 12/12 with
  first-party warnings 0. It is not yet a claim that the production app has
  routed every Rodio constructor through the owner.
- Prior focused ASIO preflight evidence is 17/17, 11/11, and 15/15, with
  first-party warnings 0. This is preflight/source evidence only; it does not
  close production code integration, the alpha.28 native build, or hardware
  acceptance.
- The show artifact source identity now includes the application owner/cue
  runtime, both v2/v3 loaders, router and tests, render/runtime modules, bridge
  build script, shared lease, ABI/native/SDK FFI and C++ callback sources,
  headers, tests, manifests, and trusted helpers. Critical runtime mutation is
  rejected before DLL inspection. The checker self-test currently passes 53
  assertions; the exact identity count is 55 after adding the split app render
  and runtime modules, the normal-output lease, the compact Audio control/UI
  integration and its focused checkers, plus the complete fifteen-file bridge
  identity.
- Current SDK-independent alpha.28 gates: TypeScript/Vite production build PASS
  (`302` modules), and `check:release` PASS including synchronized release
  metadata, ASIO packaging `169` assertions, v3 contract `22`, Timeline
  PROGRAM/CUE bus `11`, and existing video route/window gates. These checks do
  not establish complete production integration or native/hardware acceptance.
- At this superseded alpha.28 snapshot, the following work was still open:
  production app code integration, including
  v3 lifecycle wiring, two
  Rodio mixers drained by one non-RT worker into a preallocated bounded SPSC,
  app-owned mutable transport generation, complete normal/FollowProgram/
  ExplicitDevice quiesce, machine-local IPC and compact Audio UI, independent
  app/UI review, exact normal and show-ASIO native builds, real loader smoke,
  and all MOTU M4/M32/DL16 physical rows.
- Cleanup inventory was refreshed read-only at this checkpoint: `target`
  `94,214,143,142` logical bytes across `86,760` files, `app/node_modules`
  `545,338,492` bytes, `tools/asio-bridge/target` `1,774,985,879` bytes, and
  `app/dist` `5,204,539` bytes. Git also reported one approximately `568 KiB`
  garbage object. The accepted recurring deletion conditions are not currently
  satisfied, so no cleanup Apply or ad-hoc deletion ran and reclaimed bytes
  remain `0`.

## Current alpha.45 required acceptance (2026-08-31; historical rows retained)

The alpha.45 source/test checkpoint above is current. The recorded
full-workspace Cargo result passed, including Syndocal `1389 pass / 0 fail /
14 ignored` and Video `163 pass / 0 fail / 1 ignored`; the focused output-lease
keepalive gate passed `69/69`. These deterministic source results do not close
native, external, physical, audible, or hardware gates. Completed alpha39/alpha42
build facts in the rows below remain historical evidence; the current open
execution and physical rows are not closed by those facts.

1. The historical alpha.39 native checkpoint is recorded from source/docs `HEAD` and
   upstream `ec93e9160da853ad181de70aee4db7b4a75fafbb`: exact MSVC `14.44.35207`
   pinned/where-first, build exit `0` in `2m57s`, warnings `0`, artifact
   `61,108,736` bytes with SHA-256
   `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
2. The historical exact launch checkpoint is complete: one responsive/maximized
   `Syndocal` window (PID `87640`, id `2033716740`) was verified and Daslight
   was preserved. The reviewed native-evidence record was committed/pushed at
   `94b362bd2d733e447feabf0a0a6158699da6a2bf`, followed by verified
   `HEAD`/upstream equality.
3. Historical alpha9 native UI reverification is complete without clicking the output
   selector or Refresh. The exact sequence was Play -> Pause -> status-only wait
   -> Play -> Pause -> status-only wait; throughout it, explicit-device and
   resolved output stayed at `Music (Elgato Virtual Audio)`, lifecycle `実行中`,
   `rev1`, with advancing output frames and no visible Backend, Local IPC, or CUE fault.
   The project remained unsaved and final Timeline state was paused; the
   historical alpha.38 drift to `3 - PX160 WAV...` was not observed.
4. Ask the operator to audibly confirm Timeline media and Guide/Click through
   the selected `Music (Elgato Virtual Audio)` endpoint in Normal mode, then
   verify ASIO Start retires the authoring route and stale/missing/ambiguous
   devices remain silent.
5. Build and inspect the normal NSIS/MSI/updater artifacts so the complete default-distribution ASIO-free gate is measured, not inferred only from source packaging tests.
6. Build and verify the exact local-only show-ASIO artifact from the clean
   pushed source checkpoint: exact 18 exports, v3 S/E/B manifest and source
   hashes, real loader Start/Stop/Fault smoke, and one responsive maximized
   Syndocal window.
7. Verify the now-implemented canonical non-default Timeline speed synchronization on the selected real PROGRAM/CUE devices; source and deterministic gates are closed, but physical playback remains open.
8. The DJ-Link PC may remain stopped during local audio work. Before final acceptance, pull the committed DJ Agent checkpoint without exposing the token and confirm strict preflight, active runtime version, real ACK, reconnect snapshot recovery, and physical Pedal 1/2/3 behavior.
9. For the current show boundary, explicitly select and confirm the machine-local USB-DMX device/protocol, then verify the all-512-zero S0 frame reaches both USB-DMX and the strict same-PC Art-Net mirror at `127.0.0.1:6454`, wire U0, 512 bytes, including ch500 zero. The attached F3200A laser remains zero-only until beam-path safety and an exact reviewed F3200A channel test are confirmed; do not use the historical Mega PAR ch1/ch5 red frame here. That nonzero Mega PAR proof is deferred to its later rig. The older alpha.42 statement that generic serial DMX was outside this route is historical and superseded above. Close Daslight/Easy View manually before Unity because the receiver port cannot be shared.
10. For the immediate split-device target, verify TOPPING E2x2 ASIO PROGRAM on Outputs 1/2 and the explicitly selected WDM headphone endpoint for CUE. If the venue instead supplies the preferred multichannel route, perform MOTU M4 at exact 48 kHz and M32/DL16 physical acceptance as recorded in the detailed gate.
11. Update this handoff with physical evidence and exact artifact/device identities. USB-DMX, Art-Net/Unity, real ACK, ASIO device, M32 routing, reconnect, installer/updater inspection, lease keepalive, and dedicated show-ASIO acceptance remain explicitly unverified until observed; queue acceptance alone is not wire/fixture proof.

## Historical first safe resume actions (superseded; use the current USB-DMX checklist above)

- Do not regenerate the final show from alpha3 or deploy superseded alpha4-alpha8 reference candidates; alpha9 is the reviewed reference-audio candidate.
- The exact alpha.38 PID `55624` and its maximized window are historical
  evidence only; preserve the artifact identity and the `PX160` display drift
  record, but never treat that process as alpha.39. Preserve Daslight PID
  `42752` unless the operator explicitly authorizes closing it. The DJ-Link PC
  may remain stopped until the final integration gate.
- Alpha.35-alpha.37 source/full-regression, independent review, and native build
  gates are historical. Alpha.38 native/build/window evidence is also
  historical at source/build HEAD `e4ec22384675aace5ed3912ddffdcfecca190919`.
  Alpha.39 automated source gates and independent reviews are complete; the
  native build/window checkpoint used source/docs `HEAD` and upstream
  `ec93e9160da853ad181de70aee4db7b4a75fafbb`, with exact MSVC `14.44.35207`
  pinned/where-first, build exit `0` in `2m57s`, first-party warnings `0`,
  artifact SHA-256 `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`,
  and one responsive maximized PID `87640` / window id `2033716740`.
  Native alpha9 UI selector/status-only reverification also passed without
  clicking the output selector or Refresh; explicit/resolved output stayed
  `Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`, with advancing
  frames and no visible Backend, Local IPC, or CUE fault. The reviewed
  native-evidence record is pushed at `94b362bd2d733e447feabf0a0a6158699da6a2bf`.
  Keep Unity/GPU
  Art-Net, Spout, camera, DJ, hardware, audible, and dedicated Show-ASIO
  acceptance explicitly pending until observed and recorded against their
  acceptance documents.
- Historical alpha.38 first safe resume action was completed: the saved alpha9
  reference was opened, Click/Guide-enabled Play -> Pause -> Play -> Pause
  tracked Timeline state without a CUE fault, and the authoring monitor stayed
  `実行中`, `rev1`. This is historical alpha.38 native UI/runtime evidence
  only, not alpha.39 native evidence; audible confirmation remains open.
  After Pause, the authoring-output `<select>` drifted to `3 - PX160 WAV...`
  while settings JSON and resolved output remained Music. Keep this only as the
  historical alpha.38 fail-closed record; do not treat the alpha.38 process or
  this mismatch as historical alpha.39 acceptance. Alpha.39 selector/UI
  reverification is recorded above; the next safe action is operator audible
  confirmation. Do not reuse or relabel historical alpha.31/
  alpha.32/alpha.35/alpha.36/alpha.37/alpha.38 artifacts.
- Preserve the operator-owned DVC, all token material outside the checkout, and existing QA artifacts.

## 2026-09-02 alpha.60 ASIO recovery and restart checkpoint

- The current regular artifact was rebuilt from the working tree with the exact
  Community MSVC `14.44.35207` x64 linker and the explicit WinGet FFmpeg
  directory. The build command was `pnpm --dir app tauri build --no-bundle`;
  it completed successfully in `2m50s`, with first-party warnings `0`. The
  wrapper printed and verified the pinned linker first, followed by Git's
  `usr\\bin\\link.exe`. Artifact identity after the rebuild is
  `target/release/syndocal.exe`, `62,486,528` bytes, SHA-256
  `F16104950048CEAC9706D4010F4E8DEE2C2CD69D5767B4EBD135C6AA62AC6EDB`,
  Product/FileVersion `1.2.0-alpha.60`.
- Root cause of the reported “cannot leave ASIO” state was a capability
  mismatch: the normal MIT/WASAPI artifact intentionally does not register
  the separate `get_asio_output_status` / Show-ASIO command set, while the
  renderer still allowed the Show-ASIO selection to become a locked dead end.
  `audioOutputControl.ts` now recognizes the exact missing-command boundary,
  exposes `Show ASIO is not available in this build; choose Normal WASAPI.`,
  and makes the backend selector available again once the failed probe settles.
  Returning to Normal resets stale ASIO/CUE/preflight state without dispatching
  an unsupported native command. `AudioOutputPanel.tsx` no longer disables the
  backend selector merely because the view is Show-ASIO; it remains disabled
  only during Active/Fault/busy states.
- Focused evidence after the change: Audio controller `78` assertions with
  runtime regression `59`, Audio panel `58`, TypeScript `pnpm --dir app exec
  tsc --noEmit` exit `0`, and scoped `git diff --check` exit `0` (only the
  repository's LF-to-CRLF notices). A maximized, responsive exact-checkout
  window was relaunched as PID `31248` from
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`. In the real
  UI, selecting Show-ASIO produced the actionable unavailable-build message;
  selecting Normal WASAPI returned the card to `Ready · Normal WASAPI is
  selected.`. The selected Timeline WDM endpoint remained
  `Music (Elgato Virtual Audio)`.
- After launch, the dedicated safer-direction S0 command was applied once and
  acknowledged (`syndocal.safety.blackout.engage.v1`, audit sequence `1`),
  leaving the final state blackout `true`, Timeline stopped at position `0`,
  and no nonzero physical output operation. The project loaded was
  `target/qa/DSF2026-show-alpha55-dual-file-display.sdc`. Art-Net remains the
  configured same-PC logical route (`127.0.0.1:6454`, wire U0, 512ch), but the
  current runtime is `ProjectSwapDisarmed` with zero sends; USB-DMX COM3 is
  selected/present but its worker is stopped. This is a safe software state,
  not a fixture or cable acceptance claim.
- Existing external evidence remains bounded: strict two-display acceptance
  passed for Display 1 `1920x1080` at the 60-fps budget and Display 5
  `3840x2160` rendered with a measured 60-fps-budget failure; MiraBox advertises
  MJPEG `1920x1080@60` maximum (no 4K mode); Art-Net monitor proved only the
  safe-zero U0 wire shape/period. Unity/Spout, audible WDM playback, DJ-Link /
  pedal, nonzero USB-DMX fixture output, physical ASIO, and installer/updater
  acceptance remain unverified.
- This checkpoint is source/runtime/UI only. The three changed source files
  (`app/src/audioOutputControl.ts`, `app/src/components/AudioOutputPanel.tsx`,
  `app/scripts/check-audio-output-control.mjs`) and this handoff document must
  be committed and pushed together before calling the checkpoint complete.
