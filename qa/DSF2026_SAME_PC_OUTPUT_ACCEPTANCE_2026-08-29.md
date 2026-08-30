# DSF2026 same-PC output acceptance — current 2026-08-30

## 2026-08-30 alpha.42 final native checkpoint

This is the current alpha.42 native authority and supersedes the older
alpha39/alpha10 current claims below. Branch `codex/syndocal-v1.2` is at `HEAD
b1f6d760c75b430a4255ead71e5f4bb964501cf4`. The exact MSVC `14.44` native
build passed with first-party warnings `0`; the exact
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
Product/FileVersion `1.2.0-alpha.42`, `61,691,904` bytes, SHA-256
`E82B570C7BF850BB99D9DEDC529FC96ADFE952C602BA066394EA885617F17376`.

The persisted acceptance project is
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-three-display-acceptance.sdc`.
It persists exactly two strict Spout outputs and two ordinary native Display
routes. `Display 1` / LED `PX160 WAVE` (`1920x1080`) uses composition 2 with
fixed Video 1; `Display 5` / projector `MPG321UX OLED` (`3840x2160`) uses
composition 3 with Timeline Video 2 -> MiraBox -> Video 2. Foreground fixed
Video 1 works. With HDMI unplugged, the MiraBox interval is an expected
no-signal frame; black versus flat fill may vary and is not camera content
proof.

The startup gate keeps WebView2/controller creation before app-owned
maximize/F11. The desktop-window checker and final physical observation proved
that gate; the separate acceptance harness permits only its narrowly revalidated
`SW_MAXIMIZE` seam before readback. The Add receipt repair is fixed and
independently `GO`. The remaining UX
issue is the Video Output state label `Authored enabled`, localized as
`作成権を有効化`, which still looks like an operation button. Final physical
recheck passed at `1957 ms`, `5002 ms`, and `8989 ms`: `Display 1` stayed on the
same fixed Video 1 orange/red-bordered frame, while `Display 5` showed Video 2,
the expected no-signal frame, and recovered Video 2, respectively. Native route
switching/recovery, foreground independence, and both native Display windows
are `PASS`. MiraBox actual HDMI content proof, Unity/GPU/Art-Net, and other
physical hardware acceptance remain unconfirmed.

## Scope

This is the show-critical output contract for the 2026-08-30 DSF performance.
Syndocal and the Unity receiver run on the same Windows PC. Remote Art-Net and
NDI transport are deliberately outside this acceptance boundary.

The previous strict show path required a USB serial/Enttec Open DMX route. That
path is superseded for this show because the receiver consumes Art-Net and two
local Spout senders. The replacement must retain the existing output-control
lease, safety, publication, acknowledgement, and rollback boundaries; it must
not fall back to a generic unreviewed output mutation.

## 2026-08-30 alpha.39 native checkpoint (historical; superseded by alpha.42 above)

At that historical checkpoint, the product was `1.2.0-alpha.39` on branch
`codex/syndocal-v1.2`. The native build source was
`ec93e9160da853ad181de70aee4db7b4a75fafbb`; the later documentation-only
checkpoint reached clean `HEAD == upstream ==
71f93803a73f7474ae21f1d8d39eb273bcc36d67` without changing the binary. The
Timeline authoring-output selector now admits only one exact
selectable occurrence; duplicate, missing, or ambiguous identities fail closed.
Status-only polling may reapply the exact desired option without
list/configuration/routing mutation. Browser Phase A/B passed after the strict
external-video status-poll fixture was added; TypeScript, runtime, video-poll,
`git diff --check`, and `pnpm --dir app run check:release` pass. Independent
Terra select review is GO with P0/P1 `0`; the checker review is GO with P0/P1/P2
`0`. The exact MSVC `14.44.35207` Community linker was pinned and first in
`where.exe`; `pnpm --dir app tauri build --no-bundle` exited `0` in `2m57s` with
first-party warnings `0`. The exact executable
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
`61,108,736` bytes, reports Product/FileVersion `1.2.0-alpha.39`, and has
SHA-256 `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
Exactly one checkout-owned PID `87640` is responsive with maximized
`Syndocal` window id `2033716740`; Daslight was preserved. Native alpha9 UI
reverification passed without clicking the output selector or Refresh. The
exact sequence was Play -> Pause -> status-only wait -> Play -> Pause ->
status-only wait; throughout it, explicit-device and resolved output stayed at
`Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`, with advancing output
frames and no visible Backend, Local IPC, or CUE fault. The project remained
unsaved and final Timeline state was paused. The reviewed native-evidence record
was committed and pushed at `94b362bd2d733e447feabf0a0a6158699da6a2bf`, and
`HEAD`/upstream equality was verified immediately after that push.

This source change does not alter the strict Art-Net route, fixed Spout pair,
camera behavior, or their activation/retirement contracts. The alpha.39 native
build/window and selector/UI regression gates are complete only; operator audible
confirmation, audible/device selection, Unity/GPU Art-Net/Spout, DJ, camera, and
other hardware acceptance remain pending. The next safe action is operator
audible confirmation. The historical alpha.38 artifact, PID/window, and
`PX160` display drift remain immutable evidence and must not be relabeled as
alpha.39.

## 2026-08-30 Art-Net one-shot probe source checkpoint

The fixed DSF2026 Art-Net one-shot probe is now complete at the source and
review boundary, but this is not physical acceptance. The old path used a
legacy diagnostic/lease Pending record without a durable local WebView window
binding; after renderer rotation it could not prove that the same local window
still owned the in-doubt operation. The new path uses durable v2 Pending with
`window_label` captured from the authenticated local window. A v1 Pending record
without that field fails closed at migration/reconciliation. If the renderer
rotates within the same window, reconciliation is deliberately **no-send** and
records the outcome as unobservable, permanently consumes the one-shot physical
budget, and never retries the datagram. This prevents an ambiguous send from
becoming a second physical attempt.

The evidence verifier uses the current Windows UDP dynamic-port range rather
than a stale hard-coded range. The packet proof is exact Art-Net ArtDmx v14 with
physical byte `0`, wire Universe `0`, exactly 512 DMX channels, and channel 500
forced to `0`; the fixed source frame remains channel 1 = 255 and channel 5 =
255. Independent Terra xHigh re-review is `GO`. Focused source evidence is
engine DSF2026 `3/3`, Syndocal DSF2026 `6/6`, legacy migration `1/1`, and
monitor/verifier `4/4` (`41` assertions), with UI/runtime/localization checks
passing and exact MSVC `14.44` first-party warnings `0`.

These are source/deterministic proofs only: no physical UDP packet capture, no
Unity/fixture observation, and no native build of this probe checkpoint has
been performed. The probe therefore remains open in the checklist below. The
first safe resume action is to commit/push this frozen source checkpoint, run
the exact MSVC `14.44` native build and launch gate, then close Daslight/Easy
View, start the exact Unity receiver as sole UDP `6454` owner, and capture the
approved one-shot result without retrying an in-doubt operation.

## 2026-08-30 live readiness audit (pre-alpha42; superseded)

The current same-PC candidate remains
`target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha10-same-pc-output.sdc`,
`1,098,035` bytes, SHA-256
`DB1C18DCEFC79F5DC8C68589BCCAA492AF2509E932542D4A5036927B5E0814BA`.
`tools/show-structural-preflight.mjs` passes its exact DJ mapping, authored
Timeline 1 -> Timeline 2 Follow, one-source-measure transition, destination
first-measure hold, and non-finite source loop contracts. Its persisted output
surface contains one disabled Art-Net route at `127.0.0.1:6454`, wire Universe
0, and exactly two enabled 1920x1080 Spout outputs named `Syndocal Background`
and `Syndocal Foreground`. It currently contains zero video layers, an empty
Main-composition layer list, and zero Timeline video-automation rows. The fixed
senders are authored, but an MP4/Camera content switch is not. The existing
File/Camera layer plus Timeline Opacity-automation path supports a Step cut or
Linear fade once the actual media and camera identity are supplied; its focused
Timeline-automation and camera-input UI contracts pass. The non-overwriting
copy-tool test passes. These are project/source facts, not live output claims.

The last UI-verified open unsaved project was the alpha9 reference-audio source,
not the alpha10 candidate; the read-only process audit could not re-observe the
current in-memory project identity. The alpha9 file has a disabled
`EnttecOpenDmx` route and no video outputs, so that authored file cannot satisfy
this same-PC contract. Changing the open project remains an operator-visible
action and was not done during this audit.

Live process inspection found no Unity process. Daslight PID `42752` still owns
`0.0.0.0:6454`; it was deliberately preserved. The exact Unity receiver scene
is `E:\UnityProjects\Art-net-Unity\Assets\DSF2026\Scenes\DSF2026_Visualizer.unity`,
but that Unity checkout is dirty and its Build Settings point at another scene.
The safe physical sequence therefore remains: the operator explicitly closes
Daslight/Easy View, opens that exact existing scene without regenerating or
saving it, enters Play Mode, verifies Unity is the sole UDP 6454 owner, then
loads the alpha10 candidate and uses only the lease-bound Art-Net/Spout enable
actions. No Unity, GPU, Art-Net, Spout, or visible Mega PAR result was observed
in this audit.

The exact two-byte red acceptance frame is covered by a deterministic local UDP
test, while physical packet capture remains unobserved. The reviewed
lease-bound one-shot probe is now the approved source path; it is not a route
activation and cannot by itself prove a packet reached Unity. Until the exact
native build/launch and physical capture are performed, the red Mega PAR row
and Unity physical row remain open. There is no serial-DMX fallback for this
show route.

## Lighting contract

- Transport: Art-Net ArtDmx (`OpCode 0x5000`).
- Destination: `127.0.0.1:6454`.
- Daslight project universe: Universe 1.
- Art-Net wire universe and Syndocal internal universe: `0`.
- Payload: the completed 512-channel Syndocal DMX frame, byte values `0..255`.
- Address relation: DMX channel 1 is payload byte 0.
- Cadence: approximately 44 Hz (`22,727 us` engine tick), within the requested
  40–44 fps range.
- Channel 500 is unused and must remain zero on the wire. The strict show
  sender applies this fixed venue mask immediately before packet encoding, so
  an upstream non-zero byte cannot escape as payload byte 499.
- The strict show route must not admit an external DMX input/merge that can
  mutate Universe 0 after the completed show frame is established.

The imported patch evidence is `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`.
All 46 fixtures use Daslight Universe 1 and match the reference SDC names and
addresses 46/46 after normalization to internal Universe 0. Strongpoint is a
13-channel fixture with one dimmer and four RGB segments; the Daslight display
of twelve physical cells is not authoritative.

### Minimal lighting acceptance frame

All 512 bytes are zero except:

```text
payload[0] = 255  # DMX channel 1, first Mega PAR red
payload[4] = 255  # DMX channel 5, first Mega PAR dimmer
```

Acceptance requires one 530-byte ArtDmx packet at wire Universe 0 and visible
red output from the first Mega PAR in Unity. Daslight/Easy View must be closed
before Unity starts because the current receiver cannot share UDP port 6454.
Syndocal is a UDP sender and does not bind the receiver port.

## Video contract

Syndocal must keep exactly two local Spout senders active:

| Role | Exact sender name | Size | Off state |
| --- | --- | --- | --- |
| Background | `Syndocal Background` | 1920×1080 | continuous RGB black, opaque in Unity |
| Foreground | `Syndocal Foreground` | 1920×1080 | continuous RGB black, transparent in Unity |

Unity derives foreground transparency from RGB brightness and ignores source
alpha for that layer. Syndocal therefore does not need a new RGB-to-alpha
conversion, but both off states must be RGB byte-exact black. Stopping a
timeline or output must not destroy either show sender; black frames must keep
flowing. Sender destruction remains a separate output-route teardown operation.

The bundled Spout SDK registers each sender independently on its first image;
it has no transaction that can make two names externally visible in one atomic
operation. Syndocal therefore must construct and verify both exact names before
the first send, treat the pair as established only after both initial black
frames succeed, and retire both senders plus their exact engine outputs if one
side fails or the SDK changes either name. A short external observation of one
name during an asymmetric first-send failure cannot be eliminated by this SDK;
the bounded acceptance claim is immediate pair cleanup and fail-closed retry,
not impossible simultaneous registration.

Spout sender destruction is synchronous in the SDK and the worker join is the
name-reuse acknowledgement. The current SDK wrapper provides no bounded cancel
for a `SendImage` call that itself never returns. Syndocal keeps output/name
ownership fail closed while waiting, but cannot promise bounded recovery from a
driver-level infinite `SendImage` hang. This remains an external GPU/driver
liveness boundary and must not be described as an automatically recovered
case without a process restart.

The unresolved exact engine-retirement barrier is deliberately process-session
scoped. Exiting Syndocal destroys that process's SDK senders, engine instance,
and pending acknowledgement queue; restart is therefore the explicit recovery
boundary for a driver hang or crash. If the restarted project still contains
the exact authored pair, a new authenticated R4 may establish new physical
senders against that new engine instance. No claim is made that an in-process
blocked retirement survives an application restart.

The two reserved show names are mutually exclusive with generic Spout outputs.
Generic Spout creation must fail visibly while the show pair is authored,
starting, or active; a pending startup with either reserved name must be fully
joined and harvested before a fresh show-output request may reuse that name.

## Historical source evidence and open gates (superseded by alpha.42 above)

Static evidence already confirms the low-level ArtDmx encoder uses opcode
`0x5000`, wire Universe 0, a 512-byte payload, and `frame[0] -> payload[0]`.
The engine tick is approximately 44 Hz. The existing generic Spout worker runs
at 60 Hz and can render byte-exact RGB black.

Source checkpoint evidence now includes exact MSVC `14.44.35207` focused tests
with the Community linker pinned and first in `where.exe`: engine Art-Net
`7 passed / 0 failed`, Syndocal route `3 passed / 0 failed`, and protocol
control-plane `1 passed / 0 failed`; first-party warnings were `0`. The UDP
proof observes a 530-byte ArtDmx packet at wire U0 with 512 payload bytes,
`payload[0]=255`, `payload[4]=255`, and masked `payload[499]=0`. Independent
Terra xHigh review found one stale Setup I/O fixture and a missing publication
rollback proof; both were repaired before the focused rerun. Ox was not
callable, so this is the documented narrow review exception.

The alpha.35 source, carried unchanged by the current alpha.39
checkpoint, adds three independent
Universe-0 isolation fences:
activation rejects any pre-existing U0 input before creating a sender; active
strict mode rejects new U0 HTP/LTP input without inserting it; and the final
render fence skips any stale U0 merge. `ClearDmxInput(0)` remains the explicit
recovery action, and U1 plus ordinary non-strict merge remain available. Exact
MSVC 14.44 focused tests passed `12/12` with first-party warnings `0`.
Independent Terra xHigh static rereview returned GO with P0/P1 `0`. Two public
ACK/non-strict-U0 regression tests remain P2 proof debt and are not confused
with physical acceptance. The subsequent exact-linker full engine gate passed
`924 passed / 0 failed / 2 ignored`, first-party warnings `0`.

The alpha.38 normal native build/window gate is historical, from source/build
HEAD `e4ec22384675aace5ed3912ddffdcfecca190919`. With exact MSVC
`14.44`, `pnpm --dir app tauri build --no-bundle` completed in `3m12s` with
first-party warnings `0`. The exact executable
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
`61,114,368` bytes, Product/FileVersion `1.2.0-alpha.38`, SHA-256
`9E5CA0DB826D9998F7CDC76214E5CDC17597A6D4FC1CC27FEB5EF72D6E28FA37`.
Only old checkout-owned PID `50864` was stopped after exact-path verification;
Daslight PID `42752` was preserved. The new exact PID `55624` is responsive
with one maximized `Syndocal` window; Computer Use plus Alt+Space verified
`Maximize` disabled and `Restore` enabled.

This is historical alpha.38 normal native/window evidence only. It does not
prove the red Mega PAR frame, Art-Net wire behavior, either Spout sender,
Unity/GPU output, or any other physical same-PC acceptance row, and it is not
alpha.39 native evidence.

The historical committed alpha.34 Spout integration has additionally passed the
same exact linker gate for Syndocal show-Spout tests `20 passed / 0 failed`,
engine strict-pair tests `5 passed / 0 failed`, and protocol v4 command tests
`11 passed / 0 failed`, with `0` first-party warnings in each run. These tests
cover initial opaque black, post-lease live transfer, exact-name reuse,
process-session retirement reconciliation, role/project generation fences,
generic-mutator rejection, and unrelated Display coexistence. Independent
Terra xHigh final rereview returned GO with no unresolved P0/P1. The remaining
P2 proof debt is a true two-worker fake-SDK integration test for the real-time
ordering of one-sided lazy registration failure and exact retirement. The new
show modules isolate validation/lifecycle state, but the orchestration call
site in the oversized app `main.rs` and the engine command switch remain only
partially split. Neither item is promoted to physical acceptance. The final
exact-linker deterministic matrix passed: Syndocal no-default
`1205 passed / 0 failed / 7 ignored`, Syndocal default
`1263 passed / 0 failed / 12 ignored`, and engine
`920 passed / 0 failed / 2 ignored`; the remaining workspace crates also exited
successfully. First-party warnings were `0`. The first matrix attempt exposed
two separate test-maintenance defects: the new command variants were missing
from the expected command-inventory count, and an older child-Timeline test
installed an unreferenced Cue 3 state without creating Cue 3. The inventory was
updated with explicit variant assertions and the dead fixture was removed;
focused proofs and the complete matrix then passed. The alpha.38 normal
native build/window gate is historical as recorded above; Unity/GPU physical
checks remain open, while the alpha.39 native build/window and selector/UI
evidence is recorded at the top.

The alpha.35 hardening, carried unchanged by the current alpha.39
checkpoint, closes the remaining
first-physical-frame
boundary. Each fixed worker must send cached 1920×1080 opaque-black RGBA,
recheck its exact SDK name after lazy registration, and join the two-ACK plus
final R4 authority barrier before active/live handoff. Failure, timeout, name
suffix, or authority loss stops and joins both physical workers before exact
engine cleanup; an unresolved cleanup keeps the fixed-name fence fail closed
and is logged. Exact-linker focused Spout passed `13/0/0`; full Syndocal passed
no-default `1205/0/7` and default libav/Spout `1265/0/12`, with first-party
warnings `0`. Independent Terra xHigh rereview returned GO with P0/P1 `0`.
P2 remains for a single fake-worker integration test spanning every failure
variant and for an injected reaper-spawn/log-capture test; neither is promoted
to physical acceptance.

Alpha.37 strengthens the fixed pair beyond that baseline. Each worker must
complete its cached 1920x1080 opaque-black first send and revalidate the exact
post-registration SDK sender name before the pair can reach active/R4 success.
Stop/join is performed outside the show-state mutex under a state-owned
Reaping reservation, so fixed names cannot be reconstructed while old workers
still own them. If exact engine retirement is ambiguous or loses its reply,
the old pair plus prior error and a monotonic reservation receipt remain in
show state. Neither an empty/CreatePair snapshot nor a stale/substituted
receipt reopens the names; only matching `expected + prior_error +
reservation_id` with a successful exact retirement ACK does, and that repair
still requires a fresh R4 before constructing senders.

Combined exact-linker focused proof passes Show Spout `17/17`, the production
two-mutex R4 interleaving `1/1`, and generic strict-name retry `2/2`, with
first-party warnings `0`. The complete Spout-enabled Syndocal surface passes
`1276 passed / 0 failed / 12 ignored` under the same exact linker. Independent
Terra xHigh review is GO with P0/P1 `0`.
Direct pending-publish-reservation and non-synthetic post-join-cleanup-error
tests remain P2 proof debt; they are not relabeled as Unity/GPU or physical
sender acceptance.

Historical alpha.38 changes Timeline CUE transport/Guide authority only; it does not change
the strict Art-Net route, fixed Spout pair, or their activation/retirement
contracts. The latest direct Spout matrix therefore remains the historical
alpha.37 evidence above. The alpha.38 normal native/window gate is historical,
and the current alpha.39 native checkpoint does not change the strict
Art-Net or fixed Spout path. The Art-Net, Spout, and Unity/GPU physical rows
remain explicitly unaccepted.

The non-overwriting authoring tool was independently rereviewed after its
post-write cleanup was changed to fail closed: a failed post-write validation
never unlinks or renames a pathname that another process could have replaced.
The generated reference copy is
`target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha10-same-pc-output.sdc`,
`1,098,035` bytes, SHA-256
`DB1C18DCEFC79F5DC8C68589BCCAA492AF2509E932542D4A5036927B5E0814BA`.
Its alpha9 source remains SHA-256
`93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094`;
the two managed MP3 sidecars also retained their pinned hashes. The new copy
contains disabled exact Art-Net routes in both persisted route fields, exactly
two fixed Spout summaries, and Main composition output IDs `1,2`.

The following checklist records the accepted source/Spout/native gates and
keeps the final Unity/GPU physical row explicitly open:

- [x] Strict USB-serial show activation is completely replaced by exact
      `127.0.0.1:6454 / Art-Net / U0 / 512` activation.
- [x] The retired serial-show command, UI, schema, registry, tests, and
      show-only machine binding are unreachable or fail closed.
- [x] Universe 0 input/merge cannot alter the strict completed show frame in
      deterministic source tests; physical Unity output remains unchecked.
- [x] Channel 500 is deterministically forced to zero at the strict sender
      boundary and observed as zero on the wire.
- [x] The fixed channel 1 + channel 5 test frame is available and verified by
      a local UDP receiver test.
- [x] Exactly two fixed 1920×1080 Spout outputs can be created or validated by
      the reviewed output-control path.
- [x] Existing-name auto-renaming, asymmetric first-send failure, pending
      startup retry, and generic/show-output conflicts fail closed and retire
      the exact pair without leaving stale engine or transport ownership.
- [x] Timeline/output stop keeps both senders alive and publishes RGB black in
      deterministic source tests; Unity/GPU observation remains separately
      unchecked below.
- [x] The reference show is saved to a new SDC containing the Art-Net route and
      the two fixed Spout outputs; the existing alpha9 file is not overwritten.
- [x] Alpha.39 authoring-output selector admission is exact and
      fail-closed for duplicate, missing, or ambiguous identities; status-only
      polling does not mutate list/configuration/routing state. Browser Phase
      A/B passed after the strict external-video status-poll fixture was added.
- [x] Alpha.37 focused and full deterministic source gates pass with zero
      first-party warnings.
- [x] Historical native checkpoint: a warning-free alpha.36 release was built from clean
      pushed HEAD `81a7a9c` with exact MSVC 14.44 and launched from this
      checkout as exactly one responsive process.
- [x] Win32 `IsZoomed` verifies that exact alpha.36 native window as maximized
      (PID `109972`, title `Syndocal`). This is window-state proof, not
      content-level visual or Unity/GPU acceptance.
- [x] A fresh warning-free alpha.37 native release was built from clean pushed
      HEAD `5626a96` with exact MSVC 14.44. The `61,116,416`-byte executable
      has SHA-256 `7AC54394E41751126911E6DC338536B93E484A20934B4CA9A001EB8B9F3E209E`;
      exactly one checkout-owned PID `50864` is responsive with title
      `Syndocal`, and Computer Use verified the exact window is maximized.
- [x] The historical alpha.38 normal native release was built from source/build
      HEAD `e4ec22384675aace5ed3912ddffdcfecca190919` with exact MSVC 14.44.
      The `61,114,368`-byte executable has Product/FileVersion
      `1.2.0-alpha.38` and SHA-256
      `9E5CA0DB826D9998F7CDC76214E5CDC17597A6D4FC1CC27FEB5EF72D6E28FA37`;
      exactly one checkout-owned PID `55624` is responsive and maximized.
- [x] Alpha.39 exact-MSVC native build and launch/maximize checkpoint is recorded
      above: source/docs `HEAD` and upstream `ec93e9160da853ad181de70aee4db7b4a75fafbb`,
      `2m57s`, first-party warnings `0`, artifact SHA-256
      `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`, and
      one responsive maximized PID `87640` / window id `2033716740`.
- [x] Alpha.39 native alpha9 UI selector/status-only reverify passed without
      clicking the output selector or Refresh: explicit-device and resolved
      output stayed `Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`,
      with advancing output frames and no visible Backend, Local IPC, or CUE
      fault. The project remained unsaved and final Timeline state was paused.
- [ ] Operator audible confirmation and physical Unity/GPU output acceptance.
- [ ] Unity physical acceptance proves the red Mega PAR frame, both exact Spout
      sender names, 1920×1080 frames, and continuous black while stopped.
