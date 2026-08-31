# Syndocal native three-display acceptance — 2026-08-31

## 2026-09-01 alpha.51 missing test-media diagnostic — non-blocking for USB-DMX

The alpha.51 saved/live state still assigns `Display 1` / output `3` to the
foreground test source
`C:\Users\kouty\Downloads\06.flash back背景途中経過02.mp4`. That exact file is
now absent (`Test-Path=false`), although the later background test file
`EtaMDr-gpyCYahbz.mp4` remains present. One canonical Display 1 reopen attempt
therefore returned, before publication,
`Native Display output 3 presentation rejected: NotFresh { freshness: Error }`
and the just-created window closed. No repeated attempt was made after the
physical state became unknown.

Static tracing confirms that the renderer's detailed media error is currently
flattened by the native presentation admission boundary to `NotFresh { Error }`.
Retrying that result would conceal a persistent missing-file fault; no retry or
fallback was implemented. Restoration of the exact hashed test file or an
explicit operator relink to an available test source is required before the
distinct-content physical row can be re-run. A future UX repair may preflight
only the active resolved source before `WindowBuilder` and return the exact
path plus a restore/relink action, while preserving zero presents, exact
authority fences, and the existing strict Add first-frame gate. This does not
affect the separately verified USB-DMX/S0 worker and is non-blocking under the
operator's current USB-first completion boundary.

## 2026-08-31 alpha.45 source/test checkpoint — native and physical acceptance pending

The current product metadata is `1.2.0-alpha.45` on branch
`codex/syndocal-v1.2`. This is a source/test checkpoint only: no alpha.45
native build, launch/window verification, saved/reopened three-display project,
HDMI/MiraBox content observation, Unity/GPU/Art-Net/Spout observation, or other
physical hardware acceptance is claimed. The alpha.42 executable and display
observations below remain immutable historical evidence and must not be
relabeled as alpha.45.

The recorded full-workspace Cargo result passed, including Syndocal
`1389 pass / 0 fail / 14 ignored` and Video `163 pass / 0 fail / 1 ignored`.
The focused output-lease keepalive gate passed `69/69`. These deterministic
source results do not prove an alpha.45 native artifact, responsive window,
physical display signal, HDMI input, or hardware route. Native, external, and
physical three-display rows remain pending until observed against an identified
alpha.45 executable.

## 2026-08-30 historical alpha.42 final native checkpoint

This is immutable historical alpha.42 native evidence; it does not override the
alpha.45 source/test status above. Branch `codex/syndocal-v1.2` was at `HEAD
b1f6d760c75b430a4255ead71e5f4bb964501cf4`. The exact MSVC `14.44` native
build passed with first-party warnings `0`; the exact
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
Product/FileVersion `1.2.0-alpha.42`, `61,691,904` bytes, SHA-256
`E82B570C7BF850BB99D9DEDC529FC96ADFE952C602BA066394EA885617F17376`.

The persisted acceptance project is
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-three-display-acceptance.sdc`.
It contains exactly two strict Spout senders and two ordinary native Display
routes: `Display 1` / LED `PX160 WAVE` at `1920x1080` uses composition 2 with
fixed Video 1, while `Display 5` / projector `MPG321UX OLED` at `3840x2160`
uses composition 3 with Timeline Video 2 -> MiraBox -> Video 2. The foreground
fixed Video 1 path works. With HDMI unplugged, the MiraBox interval is an
expected no-signal frame; black versus flat fill may vary and is not camera
content proof.

The startup gate retains WebView2/controller creation before app-owned
maximize/F11. The desktop-window checker and final physical observation proved
that gate; the separate acceptance harness permits only its narrowly revalidated
`SW_MAXIMIZE` seam before readback. The Add receipt repair is fixed and
independently `GO`. The remaining UX
issue is the Video Output state label `Authored enabled`, localized as
`作成権を有効化`, which still looks like an action button. Final physical
2 s / 5 s / 9 s recheck passed: at `1957 ms`, `5002 ms`, and `8989 ms`,
`Display 1` remained on the same fixed Video 1 orange/red-bordered frame, while
`Display 5` showed Video 2, the expected no-signal frame, and recovered Video 2,
respectively. Native route switching/recovery, foreground independence, and
the two native Display windows are `PASS`. MiraBox actual HDMI content proof,
Unity/GPU/Art-Net, and other physical hardware acceptance remain unconfirmed.

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

## Historical alpha.42 route-transaction source checkpoint (superseded source snapshot)

The alpha.42 source checkpoint is based on branch `codex/syndocal-v1.2` at
parent `6456412f3569e5c47aa6b18b870e996d9ddd9b0a`; the checkpoint commit is the
commit containing this entry. Alpha.41 could acknowledge a Display route B and
then publish a predecessor or follower persistence mutation in the same worker
cycle. A later publication failure could therefore restore an image that was
not the transaction's exact A, which is why the first physical route attempt
failed closed and was reconciled back to Main rather than retried.

Alpha.42 gives every persistent engine submission one fail-closed submission
gate. The Display route transaction holds its narrow submission token from the
exact A read through the acknowledged route B, exact B checkpoint verification,
history update, and durable output-lease receipt. The route command is also a
worker drain barrier, so a following mutation enters a later publication cycle.
Gate poison returns the specific
`PersistenceMutationSubmissionAuthority` error before enqueue; it is never
silently recovered. The preallocated Fixture PATCH enqueue path is covered by
the same gate.

Independent Terra xHigh re-review is `GO` after the original `NO-GO` test
finding was repaired. The deterministic regression registers a real follower
inside the gate, forces the real route publication-failure seam, proves exact
Video A restoration while that follower remains blocked, and releases it only
after the route token drops. Supervisor reruns under the exact pinned/first
MSVC `14.44.35207` linker pass: persistence submission `2/2`, engine route
barrier `3/3`, Syndocal video-output assignment `3/3`, `cargo fmt --check`,
`git diff --check`, and `pnpm --dir app run check:release`. First-party warnings
are `0`; the intentional caught poison regression prints its expected panic-hook
line only.

This is the pre-build source-only snapshot and does not override the final
native checkpoint above. No alpha.42 native artifact, physical route, distinct
foreground/background content, Timeline Video 2 -> MiraBox -> Video 2 sequence,
or saved/reopened alpha.42 project is claimed here. The next safe action is the
exact native release build from this committed source, followed by one canonical
route attempt per distinct non-editor monitor and strict readback after each.

## Pre-build physical inventory (superseded by final native checkpoint above)

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
  1920x1080 at 60 fps from the alpha42 acceptance project above.

FFmpeg signal statistics at 2 s measured luma averages 63.59 and 157.36 for
Video 1 and Video 2 respectively, so the two file sources are non-black and
visually distinguishable before native output acceptance.

## Pre-build harness status (superseded by final native checkpoint above)

The deterministic harness self-test passes in both PowerShell 7 and Windows
PowerShell 5.1: `90 checks / 0 failed` in each host. It proves exact executable
identity, clean checkout and source ancestry, three stable monitor identities,
exact output ID/label/HWND correlation through the app-owned observation
command, owner PID, title, responsiveness, non-minimized state, physical client
bounds, effective DPI, three consecutive stable samples, and a narrowly scoped
editor maximize operation. It never creates, moves, closes, or substitutes an
output window.

The StandardRelease authority is rebound to the exact alpha.42 artifact above:
Product/FileVersion `1.2.0-alpha.42`, `61,691,904` bytes, SHA-256
`E82B570C7BF850BB99D9DEDC529FC96ADFE952C602BA066394EA885617F17376`,
source branch `codex/syndocal-v1.2`, and source HEAD
`b1f6d760c75b430a4255ead71e5f4bb964501cf4`. Both PowerShell hosts pass the
rebound self-test at `90/90`.

The final physical content observations above used the verified native windows,
not a CDP-enabled harness `-Apply`; that separate acceptance claim remains open
until a clean checkpoint is launched with an isolated loopback CDP endpoint.

## Alpha.40 admission failure and alpha.41 repair checkpoint

The exact alpha.40 artifact was built and launched from `d39383d` before live
authoring. It is `61,690,368` bytes, has SHA-256
`389664BBF9FA01315CC2D504E0375F137587E481650A951E8C75541B4CD1D86F`, and
opened exactly one responsive maximized Syndocal window. The first native
screen then reported that `register_project_transaction_owner` was not in the
reviewed admission inventory. No project mutation or Display Add was attempted
after that failure.

The old path froze 502 Tauri routes before
`set_video_composition_timeline_layers` was registered. The count/hash mismatch
invalidated the complete admission table, so even an already classified route
such as `register_project_transaction_owner` failed closed. The alpha.41 source
path freezes the exact 503-route inventory at SHA-256
`0ed47baf1361c273562b24ccd704ac52f421603f0a1f011fb76ed0e2a2bbe996`, classifies
the new route beside `set_video_composition_layers` as
`RendererTicketedProjectMutation`, and asserts that exact route/class pair.
The adjacent generated Engine inventory tripwire is also corrected from 271
to 273 with explicit presence assertions for
`SendDsf2026ArtNetAcceptanceProbe` and
`SetVideoCompositionTimelineLayers`; this is test-only and changes no runtime
behavior.

Because the previous Add surface collapsed all unknown native failures to a
generic sentence, alpha.41 retains that short localized operator summary but
adds a closed, local-only diagnostic disclosure. It records the captured
monitor identity, physical dimensions, and a one-line 2048-character error
after control/Bidi removal and secret redaction. Authorization/Bearer/Basic,
token/secret/credential/password, and all tested `apikey`, `api_key`,
`api-key`, `API Key`, and URL-query spellings are redacted before the cap. A
flight token and epoch prevent Display -> NDI -> Display ABA or stale monitor
discovery/selection results from restoring an old failure. The operator can
clear the disclosure explicitly, and its long values wrap inside the pane.

Supervisor evidence is Syndocal control-plane `28/28`, Engine inventory `1/1`,
backend operator contract 503 commands / 131 transactional mutations, Display
target contract PASS, frontend routing 443 / 131 / 31, frontend invoke inventory
443, localization `3644/3644`, TypeScript/Vite PASS, release gate PASS,
`cargo fmt --all -- --check`, and `git diff --check`; exact MSVC
`14.44.35207` was pinned and first for the Cargo gates, with first-party
warnings `0`. Independent admission, Engine-inventory, and final raw-diagnostic
reviews are `GO` with P0/P1 `0`. This is source acceptance only: alpha.41 still
requires a clean pushed native build, CDP-enabled QA launch, alpha12 authoring,
two canonical Display Adds, and the 2 s / 5 s / 9 s physical proof.

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

## Historical pre-alpha42 remaining live sequence (superseded by the alpha.42 native checkpoint above)

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

Until that sequence passed, ordinary native three-display output remained
implemented and physically eligible but not accepted for the alpha.42 build.
The exact native build and the 2 s / 5 s / 9 s distinct-content captures remain
pending. Source P0 and Setup Video viewport review are `GO`; they do not by
themselves make a native or physical output claim.
