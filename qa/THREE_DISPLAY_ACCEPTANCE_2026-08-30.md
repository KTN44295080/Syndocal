# Syndocal native three-display acceptance — 2026-08-30

## Contract

For this show, “three-display” means one Syndocal editor/operator window plus
two ordinary native `Display` output windows with distinct composition routes:

1. editor/operator display;
2. foreground Display output on the LED (`PX160 WAVE`): a dedicated
   composition with Video 1 fixed;
3. background Display output on the projector (`MPG321UX OLED`): a different composition whose Timeline route
   switches Video 2 -> MiraBox camera -> Video 2.

It does not mean three independent native output feeds in addition to the
editor. Spout remains a separate same-PC transport and does not satisfy this
native-window acceptance row. Mirroring the Main composition to both Display
windows also does not satisfy it.

## Current physical inventory

A read-only run of
`qa/harnesses/run-syndocal-three-display-show-acceptance.ps1` recorded the
current active topology under the ignored evidence directory
`target/qa/current-topology-read-only-28afa0f4dd75405fb270b0dc200c3b61`.
The run was explicitly `not-configured`, performed no UI mutation, and made no
hardware acceptance claim. Its native physical-mode inventory nevertheless
shows that the three required stable roles are present:

| Role | Friendly name | Current GDI | Physical mode | Effective DPI |
| --- | --- | --- | --- | --- |
| Editor | Pixio PX259PS | `\\.\DISPLAY2` | 1920×1080 | 96 |
| LED | PX160 WAVE | `\\.\DISPLAY5` | 1920×1080 | 144 |
| Projector | MPG321UX OLED | `\\.\DISPLAY3` | 3840×2160 | 144 |

The roles are selected by stable DisplayConfig monitor-device identity, not by
the transient GDI number, resolution, primary flag, or first match. Logical
desktop sizes are DPI-virtualized and must not be substituted for these native
physical modes.

## Distinct-content test sources

The bounded acceptance uses two visually distinct local H.264 files and the
already-authored MiraBox DirectShow identity. These are test sources only, not
show-program media:

- Foreground Video 1: `C:\Users\kouty\Downloads\06.flash back背景途中経過02.mp4`,
  SHA-256 `70C2B6C9D9F7F0F687E309C3207E9EEB78FDADCD738D1980165A643F15A45012`,
  1920x768 at 30000/1001 fps;
- Background Video 2: `C:\Users\kouty\Downloads\EtaMDr-gpyCYahbz.mp4`,
  SHA-256 `DC6E6C4B173D09A9FABDF674D75FD27ED85B1084A55F633F52F85211F4D1473F`,
  1280x720 at 30 fps;
- Background camera: the persisted MiraBox camera identity requesting MJPEG
  1920x1080 at 60 fps from the current alpha11 acceptance project.

FFmpeg signal statistics at 2 s measured luma averages 63.59 and 157.36 for
Video 1 and Video 2 respectively, so the two file sources are non-black and
visually distinguishable before native output acceptance.

## Harness status

The deterministic harness self-test passes in both PowerShell 7 and Windows
PowerShell 5.1: `90 checks / 0 failed` in each host. It proves exact executable
identity, clean checkout and source ancestry, three stable monitor identities,
exact output ID/label/HWND correlation through the app-owned observation
command, owner PID, title, responsiveness, non-minimized state, physical client
bounds, effective DPI, three consecutive stable samples, and a narrowly scoped
editor maximize operation. It never creates, moves, closes, or substitutes an
output window.

The StandardRelease authority is still pinned to the historical alpha.25
artifact. It must be rebound only after the current alpha.40 source checkpoint
is committed and the exact clean-source native artifact has been built. The
new product version, byte size, SHA-256, source commit, branch, and provenance
must then be updated together and both 90-check self-tests rerun before Apply.

The current alpha.40 process has no loopback CDP listener, so the harness
cannot obtain its app-owned output-ID/HWND observation. The 90-check harness
self-test still passes, but that proves only the harness's fail-closed logic;
it is not native three-display or content acceptance.

## Current P0 composition boundary (frozen source implementation)

The split route is now represented by the additive
`TimelineVideoLayerRef { timeline_id, layer_id }` membership. `timeline_id`
selects the exact root/Follow timeline context and `layer_id` identifies the
stable authored video layer; the runtime resolves the current ephemeral
Timeline projection without persisting a projection ID.

- Root and Follow are isolated by exact `(timeline_id, layer_id)` matching.
  There is no cross-timeline inference, numeric-only fallback, or projection
  leakage between the root and Follow routes.
- Timeline projection is composed below fixed authored layers. The Background
  route therefore retains its fixed authored-layer ordering while the Timeline
  projection supplies the Video 2 -> MiraBox -> Video 2 content; Foreground
  remains the fixed Video 1 layer only.
- Main composition is unchanged. A request that relies on routing the Timeline
  projection through Main, or that would mirror Main into both outputs, is
  rejected rather than silently transformed.
- Projects written before this P0 field existed default the missing membership
  to an empty list. No numeric layer membership is synthesized.
- Numeric-only P0 membership is an unshipped shape and fails closed; only the
  complete `{ timeline_id, layer_id }` reference is admissible.

The independent original review returned `NO-GO` for this P0 boundary. The
corrected frozen implementation and its real weighted-Follow integration
regression have now received independent `GO`: exact `(timeline_id, layer_id)`
selection excludes the target Timeline's same-numbered lane, keeps the fixed
authored layer above the selected projection, and persists neither ephemeral
projection ID.

## Source evidence and authoring separation

Supervisor evidence for the frozen implementation is protocol `200/200`,
engine focused `5/5` (including the resolver and existing media/camera
automation boundaries), the exact weighted-Follow integration regression
`1/1`, and Syndocal boundary `1/1`. TypeScript, localization `3639/3639`,
frontend invoke inventory `443`, output-control, Vite build, release gate,
`cargo fmt`/`git diff --check`, and both three-display harness self-tests pass:
PowerShell 7 `90/90` and Windows PowerShell 5.1 `90/90`. The Setup Video
containment gate passes all five viewports, including `1366x768` and
`1280x720`, with document/app scroll zero and the Advanced panel retained as an
internal scrollport. Independent UI re-review is `GO`; no typography or hit
target size was reduced. First-party warnings are `0` for the evidenced checks.

The same-PC alpha9 copy tool is historical strict alpha9 -> alpha10
Spout/Art-Net authoring only. It is not used for the alpha12 distinct Display
project. Its old schema-key list remains an explicit nonblocking P2 until that
tool is retired or updated; it is not evidence for the alpha12 composition
route.

## Remaining live sequence

- Load the updated video-switch acceptance project. The current alpha11 file
  has the fixed Spout pair, only Main composition, one MP4 plus MiraBox, and no
  native Display outputs; it is not yet the distinct-content acceptance file.
- Add exactly two fullscreen Display outputs through the canonical lease-bound
  UI, bind one to the LED stable identity and one to the projector stable
  identity, create the Foreground and Background custom compositions described
  above, and route each output to a different composition.
- Open both native windows and verify distinct output IDs, labels, HWNDs,
  monitor identities, physical bounds, DPI, responsiveness, and three stable
  samples while the editor remains maximized.
- Verify that the foreground output continues to show Video 1 while only the
  background output performs Video 2 -> MiraBox -> Video 2 at the authored
  Timeline boundaries. Capture both physical outputs together at approximately
  2 s, 5 s, and 9 s, and separately record the output render plans proving the
  distinct composition/layer sets. Also verify pause/resume and stop/retire, then reopen
  without a monitor-number fallback. A disconnected or stale identity must
  fail visibly rather than move to another display.
- Run the rebound harness with `-Apply`; retain its new, immutable evidence
  directory and record the verdict in the release and show handoff documents.

Until that sequence passes, ordinary native three-display output remains
implemented and physically eligible but not accepted for the current build.
The exact native build and the 2 s / 5 s / 9 s distinct-content captures remain
pending. Source P0 and Setup Video viewport review are `GO`; they do not by
themselves make a native or physical output claim.
