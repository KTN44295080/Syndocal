# DSF2026 same-PC output acceptance — current 2026-09-02

## 2026-09-02 alpha.60 native launch checkpoint

The current source checkpoint is `140bebcabeec4f574fa37bc30e03e1b294ecbc32`
on branch `codex/syndocal-v1.2`, pushed and upstream-equal. The exact Community
MSVC `14.44.35207` pinned no-bundle build passed in `1m48s` with first-party
warnings `0`; the only build notice was the existing Vite large-chunk advisory.
The resulting `target/release/syndocal.exe` is ProductVersion and FileVersion
`1.2.0-alpha.60`, `62,435,840` bytes, SHA-256
`29045BC40227F823E0A2259113E2BECC246B665AEC161E5A14111D9CB0E4EA1C`.

Exactly one process resolved to this checkout's executable (PID `53420`),
reported `Responding=True`, and exposed the maximized main `Syndocal` window.
Read-only host enumeration found the persisted FTDI Open-DMX selection on
`COM3`, the `Music (Elgato Virtual Audio)` endpoint, four display modes, and an
`Unknown MiraBox Video Capture` device entry. These are presence observations
only; they do not prove live USB-DMX frames, audible PROGRAM/CUE, display
pixels/60fps, MiraBox content, or Unity Art-Net/Spout.

The current alpha.60 engine also completed the explicit physical serial demo
against `COM3` with the pinned Community MSVC 14.44 linker:
`SYNDOCAL_PHYSICAL_MASTER=10`, `SYNDOCAL_PHYSICAL_SECONDS=15`, and
`physical_serial_rainbow_demo_drives_master_dimmer_and_rgb_cells` returned
`1 passed / 0 failed` in `15.02s`, with no serial send failure and at least
`439` successful sends observed during the run. This proves the current engine
serial write path reached the selected COM port; fixture illumination remains
an operator visual check and is not inferred from the test.

With the same pinned MSVC gate and the explicit WinGet FFmpeg root, the current
alpha.60 native Timeline smoke passed: `phase1_smoke_project_sample_timeline`
returned `2 passed / 0 failed` and
`phase1_smoke_project_sample_sends_cue_to_artnet_loopback` returned
`1 passed / 0 failed`. This proves the deterministic Timeline-to-event and
Timeline-to-Art-Net loopback routes only; it does not prove physical DMX,
audible PROGRAM/CUE, display pixels, or Unity reception.

The deterministic three-display observation harness is pinned to this exact
artifact and source identity; its seam suite passes `96/0`. It remains
observation-only and cannot claim native display acceptance without app-owned
output observation and explicit monitor identities. The next safe action is one
bounded operator/native acceptance pass covering S0/USB-DMX, Display windows,
Timeline UI loop/release/Follow, and PROGRAM/CUE; external Unity, DJ-Link, camera, ASIO, and
installer/updater rows remain open.

## Historical 2026-09-01 alpha.57 native boundary

The current source checkpoint is `1545bedd189c1f14b9656551ca5046cb1d02023c`
on branch `codex/syndocal-v1.2`. Exact Community MSVC `14.44.35207`
`pnpm --dir app tauri build --no-bundle` passed in `5m29s`, with first-party
warnings `0`. The resulting `target/release/syndocal.exe` is ProductVersion
and FileVersion `1.2.0-alpha.57`, `62,310,912` bytes, SHA-256
`ACB6A29A90F5D9CE5D57A406AC15488F870515B917B4654720F92332C86E3F2B`.
Exactly one checkout-owned process, PID `82632`, exposes one responsive,
visible, maximized `Syndocal` window. This accepts native build/start only;
USB-DMX, camera/MiraBox, audible PROGRAM/CUE, external DJ/pedal, Unity/Spout,
and three-display pixels remain physical/external gates. Older evidence below
is immutable and must not be relabeled as alpha.57 acceptance.

## Historical 2026-09-01 alpha.51 Timeline authority / pedal-wait native and S0 checkpoint

At that historical checkpoint, product metadata was `1.2.0-alpha.51` on
`codex/syndocal-v1.2`, which was clean and pushed/upstream-equal at
`4f67dbf108dc17825979731435f844a5cb57311c` before this evidence-only update.
The exact Community MSVC
`14.44.35207` no-bundle build passed in `3m39s`; the pinned Community linker
was first in `where.exe` and first-party Rust warnings were `0`. The artifact
is `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.51`, `62,419,456` bytes, SHA-256
`A8D6A397FF9D6838D16C1F796FC9681BFD64A475FEE71A49CB3AABB413EA998B`.
Exactly one responsive maximized window was verified at PID `62920`.

Timeline mutation ACK data is now treated as runtime-free authored data. Live
state is accepted only from an atomic `get_project_authority_bundle` matching
the ACK's exact epoch/revision/checkpoint hash, active Timeline identity, bank
membership, and post-receipt read generation. Missing, stale, superseded, or
mismatched convergence remains visibly blocked; generic snapshots cannot
unlock it and no non-idempotent retry is inferred.

The `人生オーバー` Follow destination remains `惑う星` with
`destination_start_mode: wait_for_pedal`. DJ-Link v3 is unchanged: the wait is
encoded with the existing Running/loop-off/hold-off truth plus exact Timeline,
play-session, timeline-owner, and release correlation. No
`waitingForPedalStart` wire key was added. The companion `rb-output` checkout
remains clean/upstream-equal at
`59df968d91bca71a327ef2a57ee5ab15de9f9947`.

Exact MSVC 14.44 focused/broad source gates currently pass: I/O DJ-Link
`43 passed / 1 ignored`, Syndocal DJ-Link `120 passed / 1 ignored`, Engine
Timeline-bank `5/5`, plus TypeScript, Timeline authority/follow checkers,
release metadata, format, and diff checks. First-party warning count is `0`.
Independent Terra xHigh final reviews of both corrected boundaries returned
GO.

The alpha.51 project was reopened, the exact persisted `COM3` FTDI device was
reconfirmed as this machine's Open-DMX binding, and the worker was armed under
S0. The native surface reported output enabled, S0 safety blackout,
Open-DMX worker active, and latest all-zero frame queued. The attached F3200A
remains all-512-zero-only until a separately reviewed safe nonzero test. This
closes only the bounded software-worker/initial-zero receipt; electrical
waveform and fixture response remain unverified. Per operator priority,
USB-DMX is blocking; Art-Net and Spout are non-blocking and remain explicitly
unverified. From the first successful observation before `04:35:10 JST`
through `04:40:07 JST`, exact PID `62920` stayed responsive, the latest zero
frame remained queued, and stderr contained no serial/output fault; this is a
bounded greater-than-`4m57s` alpha.51 S0 continuity observation.

The current native Reference Audio + two Video Switch Timeline was then run
from the start through natural transport completion at `04:47`-`04:49 JST`
while S0 remained latched. The transport returned to stopped/play-enabled,
PID `62920` remained responsive, the post-run I/O surface still reported the
Open-DMX worker running with the latest S0 zero frame queued, and stderr added
no serial/output fault. This is bounded native Timeline/USB-worker coexistence
evidence.

The authored `人生オーバー` Timeline (`219,506 ms`) was then selected in the
same native alpha.51 process. Its authored `136,941`-`138,353 ms` loop wrapped
the running playhead from approximately `138,318 ms` back into the loop at
`137,662 ms`. After a direct seek beyond the loop, natural completion stopped
at exact `219,506 / 219,506 ms`. The state then remained stable for more than
five seconds with active Timeline `人生オーバー`, Follow `保持`, correlated
route `人生オーバー -> 惑う星`, and start reason `自然再生境界`; `惑う星`
did not auto-start. This closes the native loop and natural
completion-to-pedal-hold boundary. Actual Pedal 1 start remains externally
blocked by the absent DJ peer (`trust_network_absent`).

The post-Follow Setup > I/O surface still reported the exact local Open-DMX
route, S0 armed, `250000 baud`, the worker running, and the latest S0 zero frame
queued. At `05:09:48 JST`, exact PID `62920` remained responsive, Windows
reported `USB Serial Port (COM3)` / FTDI / PnP status `OK`, and stderr had no
new serial/output fault. This extends the bounded alpha.51 S0 coexistence
observation from before `04:35:10 JST` to greater than `34m38s`. It does not
claim electrical waveform or fixture receipt, and the attached F3200A remains
all-512-zero-only.

## Historical 2026-09-01 alpha.50 USB-DMX continuous S0 checkpoint

The exact Community MSVC `14.44.35207` no-bundle build passed with the
Community linker first in `where.exe`, the same absolute linker pinned for
Cargo, and first-party Rust warnings `0`. The artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.50`, `62,416,384` bytes, SHA-256
`C73065B089D7AE7672213FB2334882A033AA5B5936EF23A15D4B40F4AF8BEE84`.
Exactly one responsive maximized Syndocal window was verified (PID `22100`).

Alpha.49 later faulted after its approximately-five-minute observation with
`Open DMX worker stopped: failed to write whole buffer`; its earlier healthy
statement is historical only and is not current continuous-health evidence.
The root cause was the Open-DMX `2 ms` Windows COM write timeout: shorter than
one `513`-byte, `250000` baud, `8N2` frame (`22.764 ms`) and therefore able to
return a zero or partial write under FTDI backpressure. Alpha.50 centralizes a
`100 ms` Open-DMX write timeout for both normal and verified-direct opens;
Enttec USB Pro retains its separate `2 ms` timeout. A zero-byte write remains a
terminal `WriteZero`: S0 stays latched, the worker is not retained or retried,
and later live bytes are rejected.

The same authored project (`1,114,510` bytes, SHA-256
`5926A36FDD8E0251A2B67904A93490D3E3B3F54E323E24D3FAC26B31E7546F46`)
was recovered in alpha.50. The one-button `Prepare show DMX` flow completed
`Both`, exact machine-local `COM3` binding, Art-Net loopback, S0, and Open-DMX
arm. The selected and confirmed PnP instance was
`FTDIBUS\VID_0403+PID_6001+A&A5D719&0&8\0000`. Backend activation was retained
only after the initial all-zero BREAK/MAB/`write_all`/`flush` receipt.

From the `2026-09-01 02:45:22 JST` observation through
`2026-09-01 02:55:36 JST` (`10 min 14 s`), the exact alpha.50 process remained
responsive and the UI continuously ended at `S0 armed`, Open-DMX worker
active, latest zero frame queued, and no worker fault. Stderr contained only
the unrelated DJ-Link `trust_network_absent` line. This passes the bounded
software-worker continuity checkpoint under S0. It does not prove an
electrical waveform, fixture receipt, a nonzero frame, or the F3200A visual
result; all 512 physical channels remained in the all-zero-only boundary.

Focused evidence passed: IO serial `25/25`, Engine show-serial `23/23`,
Syndocal show-serial `5/5`, release metadata, formatting, diff, and the native
no-bundle build. The standard Vite `>500 kB` chunk notice is the only frontend
build notice; first-party Rust warnings are `0`. Independent read-only review
reported GO with no P0/P1 finding.

## Historical 2026-09-01 alpha.49 native one-button checkpoint

The exact Community MSVC `14.44.35207` no-bundle build passed with
first-party Rust warnings `0`; the Community linker was first in `where.exe`
and Git was second. The artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.49`, `62,397,440` bytes, SHA-256
`2152272B75A59D3342DE4AD64D33638564FC9D2038ED38D56F7C65B64F42177C`.
Exactly one responsive maximized Syndocal window was verified (PID `71980`).

The project is
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-artnet-probe-acceptance.sdc`,
`1,114,510` bytes, SHA-256
`5926A36FDD8E0251A2B67904A93490D3E3B3F54E323E24D3FAC26B31E7546F46`.

The one-button flow completed all four stages: `Both` role, exact `COM3` PnP
binding, Art-Net `127.0.0.1:6454` / wire Universe `0` enable, and S0 plus
Open-DMX arm. Backend activation acknowledged only after the initial physical
all-zero BREAK/MAB/`write_all`/`flush` transaction. Alpha.49 recorded an
approximately-five-minute healthy observation before the later terminal
WriteZero. That interval is historical only and was superseded by alpha.50;
it is not current continuous-health evidence. It proved the initial guarded
transaction, not an electrical waveform or fixture visual result.

| Surface | Alpha.49 observation | Acceptance boundary |
| --- | --- | --- |
| USB-DMX | Exact `COM3` binding, S0, and guarded initial all-zero transaction acknowledged; worker stayed active. | Electrical waveform and fixture receipt are not in scope/are pending user confirmation. |
| Art-Net / Unity | Route enabled at `127.0.0.1:6454`, wire U0. | Listener/route readiness only; no current Art-Net datagram capture or external node proof. |
| Displays | Display 1 `1920x1080`, Display 5 `3840x2160`, plus editor live. | Window presence only; current content/pixel acceptance remains pending. |
| Spout / camera | MiraBox absent; Spout pixel route unresolved. | No camera or Spout pixel acceptance claimed. |
| Timeline | `人生オーバー` -> `惑う星` wait-for-Pedal-1 configuration present. | Natural-boundary transition and live Pedal 1 start receipt pending. |

Focused evidence: serial code `23/23`, protocol `205/205`, TypeScript, fmt,
diff, and `check:release` pass. The only reported frontend notice is the
standard Vite chunk-size warning (`>500k`); first-party warning count is `0`.
The alpha.49 implementation and QA checkpoint was committed as
`f7e412c284e6222f85c57f81c7ebf52d7e8995a9` and pushed to
`origin/codex/syndocal-v1.2`. This acceptance update is the documentation-only
successor at the current branch tip; verify `HEAD == origin/codex/syndocal-v1.2`
before resuming.

## Historical 2026-09-01 alpha.48 stage-4 stale-fence checkpoint

Alpha.48 was `62,375,424` bytes, SHA-256
`749AF5590D2694937F9CAE62471AFF87446E87C455137270D98C38582A82B6DD`.
The stage-3 stale-fence fix crossed to stage 4, then serial arm failed before
publication because `GetFinalPathNameByHandleW` received an invalid COM handle
(`0x80070057`). No USB worker or physical write occurred. This was a
pre-physical-apply fail-closed rejection; alpha.49 supersedes it.

## Historical 2026-08-31 alpha.46 native one-button pre-publication checkpoint

The required exact MSVC `14.44.35207` native no-bundle build completed with
first-party Rust warnings `0`. The resulting
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` reports
Product/FileVersion `1.2.0-alpha.46`, is `62,290,432` bytes, and has SHA-256
`04A36C709BEF41E8BE4A561C37835EE6FDAAD8F3EC25586666DFB4D6C2B8F768`.
Exactly one responsive maximized window was launched with
`DSF2026-show-alpha42-artnet-probe-acceptance.sdc`; that project is
`1,114,470` bytes with SHA-256
`35EE42976B97C9570139A37AF09F82E1B1D97B0F1BD1CC5F6D9DB83F53EA2F0A`.

The one-button `Prepare show DMX` run completed output role `Both` and the exact
machine-local binding stages. MiraBox was unplugged, and its input-only absence
did not block `Both`. The operator accepted the native Art-Net confirmation,
but stage 3 then failed before publication. Exact stderr identifies the cause
as managed output-lease renewal advancing the lease while the confirmation was
open: the submitted candidate ended `Err(StaleGeneration)`. No Art-Net socket,
USB-DMX worker, S0 transition, or physical frame was created by that rejected
stage. The UI's generic `publication_failed / physical output state is unknown`
wording is therefore not a physical in-doubt result for this observation.

At that historical checkpoint, the source fix still needed to serialize managed
exact-`Both` ordinary authorization with the keepalive renewal lane while
preserving stale rejection for unmanaged or mismatched leases. Alpha.49
supersedes that stage-3 boundary. The F3200A remains all-zero-only.

## 2026-09-01 current show transport boundary (acceptance pending)

The venue's physical lighting primary is an explicitly selected,
machine-local USB-DMX route. The `.sdc` carries logical output settings only;
the show PC must enumerate and confirm the exact current USB device/protocol
from Setup > DMX. COM/PnP identity is machine-local, and missing, stale,
ambiguous, or changed identity fails closed. The completed internal Universe 0
frame is mirrored to USB-DMX for the fixtures and simultaneously to Unity as
Art-Net ArtDmx at `127.0.0.1:6454`, wire Universe `0`, exactly `512` channels,
approximately `40–44 fps`, with channel 500 forced to zero. The exact
FT232R/COM3 USB worker uses its empirically stable `22,764us + 8ms` period
(about `32.5 fps`) and retains the latest completed U0 frame; it does not claim
one physical USB delivery per 44Hz engine tick. This is not an Open-DMX
universal maximum, and any faster USB rate needs fresh waveform and fixture
evidence. Video remains the two local Spout senders `Syndocal Background` and
`Syndocal Foreground`.

| Boundary | Historical wording | Current wording | Reason |
| --- | --- | --- | --- |
| Physical lighting | Art-Net was the only show route; serial DMX was outside it | Explicit machine-local USB-DMX is primary; Art-Net is the simultaneous Unity mirror | Matches venue wiring while retaining the Unity visualizer |
| Video | Local fixed Spout pair | Same fixed Spout pair | Spout is still the local video transport |

An independently configured generic serial route may fail without stopping an
unrelated Art-Net/sACN route. The selected USB-DMX route in this strict show is
different: a USB identity, write, or flush fault is show-wide and must engage
global S0 plus an Art-Net zero/blackout; live Art-Net must not continue from a
stale frame. This fault response is a source contract whose physical result is
still pending acceptance.

The following rows are still **pending**. Queue acceptance and worker completion
are intermediate observations, not electrical wire or fixture proof:

### Current acceptance checklist

- [x] Explicitly select and confirm the venue USB-DMX device/protocol on the
      show PC: `COM3`, PnP instance
      `FTDIBUS\VID_0403+PID_6001+A&A5D719&0&8\0000`, historical alpha.50
      artifact above.
- [x] Engage S0 and record the initial all-zero USB-DMX frame queue plus the
      enabled Art-Net U0 route. This is not electrical or datagram delivery proof.
- [x] For the selected Open-DMX worker, observe completion of BREAK, MAB,
      `write_all`, and `flush`; backend activation acknowledged only after this
      transaction. Worker completion is not electrical-wire proof.
- [x] Keep the alpha.50 worker active under S0 for `10 min 14 s` with no
      worker fault or repeat of the alpha.49 WriteZero. This is bounded
      software-worker continuity, not electrical or fixture proof.
- [ ] Release S0 and observe the same completed internal U0 frame sent to the
      Art-Net mirror and retained by the USB latest-frame worker; retain the
      live-frame receipt separately. This is not a per-engine-tick USB-wire
      delivery claim.
- [ ] Capture the USB-DMX electrical wire and the Art-Net datagram independently;
      confirm USB frame channel 500 / `payload[499]` is `0`.
- [ ] Confirm the Art-Net packet is ArtDmx `0x5000`, wire U0, 512 channels, and
      `payload[499]=0`; a local loopback capture is not venue-wire proof.
- [ ] **Current attached fixture boundary:** the F3200A laser (34ch, DMX
      address 1) is all-512-zero-only. Do not release S0 or send the historical
      Mega PAR `ch1=255` / `ch5=255` frame until the operator has confirmed a
      safe beam path and an exact reviewed F3200A channel test. The Mega PAR
      red/blackout check is deferred to that later Mega PAR rig.
- [ ] Keep the exact output lease healthy for more than its 60-second TTL while
      navigating away from the output panel; backend keepalive is not yet
      accepted.

## Historical 2026-08-31 alpha.45 operator-run output-control checkpoint (observed; physical acceptance pending)

The operator-run artifact was identified as
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, Product/FileVersion
`1.2.0-alpha.45`, `62,272,000` bytes, SHA-256
`34EAC7C71E8EC392A3C51A429A313C573D096AC86DE3499EB6EA0A454641E89B`. At the
same readback, the source checkout was branch `codex/syndocal-v1.2`,
`HEAD 85787a87aa95ded22f80f11386a6603ef0c675bb`, and upstream
`85787a87aa95ded22f80f11386a6603ef0c675bb` (equal; the worktree is dirty).
This records the artifact and repository identity used for the observation; it
is not a clean-tree/source-binding attestation or a fresh native build gate.

The following observations are recorded exactly as runtime/operator evidence.
They do not promote queue status, a local listener, or a dark fixture to
physical delivery proof:

| Surface | alpha.45 observation | Acceptance boundary |
| --- | --- | --- |
| Legacy Spout reset | The legacy pair reset was observed successful: runtime output count changed `4 -> 2`, while `Display 1` and `Display 5` remained present. | Runtime inventory only; Spout receiver/pixel and external display acceptance remain unverified. |
| Art-Net / Unity | Art-Net was enabled and a Unity listener at `127.0.0.1:6454` was observed. | This is local loopback/listener evidence only; the physical Art-Net node target IP is still unresolved and no physical-node acceptance is claimed. |
| `Both` enable | Enable was rejected while preparing the combined route with `External video output route synchronization failed while preparing Both`; OutputControl returned `publication_failed` / `physical output state is unknown`. | A camera-input-start failure is suspected, not proven; do not call it the root cause or retry an in-doubt publication blindly. |
| USB-DMX `COM3` arm | The arm ended `publication_failed`; the worker stopped and no physical zero receipt was captured. | USB electrical/fixture acceptance was not performed. `S0` remained engaged and the observed DMX status was `0/0`, which is logical/runtime state only. |
| Attached fixture | The F3200A remained dark in the observation. It is the reviewed boundary of a 34-channel laser, and nonzero output remains prohibited. | Keep all 512 payload bytes zero; darkness is not a physical zero-wire receipt and no nonzero probe is released. |
| One-button fix | The frontend + backend one-button fix remains in progress. | No native build, launch/re-run, or acceptance of that fix exists yet. |

Current status is therefore `publication_failed` with the physical output state
unknown. Preserve S0 and the all-zero F3200A boundary. The next safe action is
to finish the one-button source fix, then perform the required native run and
independently identified USB/Art-Net observations; until then, every physical
output row below remains pending.

## 2026-08-30 alpha.42 final native checkpoint (historical; superseded by the boundary above)

This is the historical alpha.42 native authority and superseded the older
alpha39/alpha10 claims below at that time. It is not the current source or
physical-show authority. Branch `codex/syndocal-v1.2` was at `HEAD
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

## 2026-08-30 alpha.42 Art-Net probe-only derivative

The fixed generator
`qa/harnesses/derive-dsf2026-alpha42-artnet-probe.mjs` created the ignored QA
artifact
`C:\Users\kouty\Documents\KDMX\target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha42-artnet-probe-acceptance.sdc`
once, from the exact current alpha42 three-display source
`DSF2026-show-alpha42-three-display-acceptance.sdc` (`1,120,320` bytes,
SHA-256
`2D8B4D760E51009344D5A3195A39A61D0674F993027CD440A49F6F7D8F1F355C`). The
derived artifact is `1,120,306` bytes with SHA-256
`4130599CEB73F2D97C7BB22DBCAD6A9187BD02F53DAC3946491BEB32E33776D9`.

The publication is byte-preserving except for exactly these two parsed paths:
`snapshot.dmx_outputs[0].protocol` and `snapshot.output.protocol`, each changed
from `EnttecOpenDmx` to `ArtNet`. Both routes remain `enabled:false`,
`127.0.0.1:6454`, wire Universe `0`, `serial_port:""`, and baud `250000`.
Strict Spout/Display/timeline content and every other JSON value are unchanged
by deep-diff verification. The generator fails closed on fixed source identity,
schema/count, literal-count, reparse, existing-target, and overwrite-boundary
violations; production CLI paths are fixed and the target is create-new only.

This is a probe-only current-alpha42 derivative, not a production replacement
and not physical acceptance. Unity and UDP `6454` are recorded as prepared for
the eventual one-shot run, but this generation/verification invoked no UDP send
and the one-shot remains unsent/unconsumed. Focused Node tests, `node --check`,
and `git diff --check` pass. With the exact MSVC `14.44.35207` linker pinned and
first in `where.exe`, the Syndocal packet/route tests pass `2/2` and the Engine
no-route-mutation test passes `1/1`; first-party warnings are `0`. No native
release build or physical receiver/fixture observation is claimed here.

Recorded focused commands (all exit `0`) are
`node qa/tests/derive-dsf2026-alpha42-artnet-probe.test.mjs`,
`node --check qa/harnesses/derive-dsf2026-alpha42-artnet-probe.mjs`,
`node --check qa/tests/derive-dsf2026-alpha42-artnet-probe.test.mjs`, and
`git diff --check` on the owned tracked documentation paths. The Rust commands
are `cargo test -p syndocal show_artnet_acceptance_probe::tests -- --nocapture
--test-threads=1` and `cargo test -p engine
dsf2026_probe_engine_boundary_sends_one_exact_u0_packet_without_route_mutation
-- --nocapture --test-threads=1` under that exact linker gate.

## Historical alpha.42 scope (superseded)

This is the show-critical output contract for the 2026-08-30 DSF performance.
Syndocal and the Unity receiver run on the same Windows PC. Remote Art-Net and
NDI transport are deliberately outside this acceptance boundary.

At that historical alpha.42 checkpoint, the strict path required Art-Net for
lighting and two local Spout senders; USB serial was explicitly outside the
show boundary. The 2026-08-31 source boundary above supersedes that transport
role decision only: USB-DMX is now the physical fixture primary and Art-Net is
the simultaneous Unity mirror. The existing output-control lease, safety,
publication, acknowledgement, and rollback boundaries remain required; no
generic unreviewed output mutation is admitted.

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

The candidate recorded in that historical audit was
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
activation and cannot by itself prove a packet reached Unity or a USB fixture.
The alpha.42 probe had no serial-DMX fallback; that is historical only. Until
the current native build/launch and physical capture are performed, the USB
fixture, Art-Net/Unity, and red Mega PAR rows remain open.

## Current lighting contract

- Physical primary: explicitly selected machine-local USB-DMX device/protocol;
  its COM/PnP identity is not persisted in `.sdc`.
- Unity mirror transport: Art-Net ArtDmx (`OpCode 0x5000`) to
  `127.0.0.1:6454`.
- Daslight project universe: Universe 1.
- Art-Net wire universe and Syndocal internal universe: `0`.
- Payload on both routes: the same completed 512-channel Syndocal DMX frame,
  byte values `0..255`.
- Address relation: DMX channel 1 is payload byte 0.
- Art-Net cadence: approximately 44 Hz (`22,727 us` engine tick), within the
  requested 40–44 fps range.
- USB-DMX cadence: the exact FT232R/COM3 Open-DMX default is `30,764 us`
  (`22,764 us` frame wire time plus an empirically required `8 ms` guard), or
  about 32.5 fps. It retains the latest completed U0 frame and does not promise
  a physical USB delivery on every Art-Net/engine tick. This is exact-rig
  evidence, not an adapter-family maximum; a 36–40 fps trial is acceptance-only
  and may not become the default without new waveform and fixture evidence.
- Channel 500 is unused and must remain zero on both routes: USB-DMX frame
  `payload[499]` and Art-Net wire payload `payload[499]` are forced to `0` at
  their respective output boundaries, so an upstream non-zero byte cannot
  escape either route.
- The strict show route must not admit an external DMX input/merge that can
  mutate Universe 0 after the completed show frame is established.

The imported patch evidence is `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`.
All 46 fixtures use Daslight Universe 1 and match the reference SDC names and
addresses 46/46 after normalization to internal Universe 0. Strongpoint is a
13-channel fixture with one dimmer and four RGB segments; the Daslight display
of twelve physical cells is not authoritative.

### Current F3200A laser acceptance boundary

The currently attached fixture is an F3200A laser in 34-channel mode at DMX
address 1. Until the operator confirms beam-path safety and an exact reviewed
F3200A channel test, the only permitted physical acceptance frame is all 512
payload bytes zero. Do not reuse the historical Mega PAR red/dimmer mapping:
`payload[0]=255` / `payload[4]=255` is neither reviewed nor safe for this
laser.

The deferred Mega PAR rig may later use its separately reviewed red frame, but
that is a different fixture-specific acceptance record. For either future
nonzero test, acceptance requires separately recorded USB worker write/flush
completion, USB electrical-wire observation, and one 530-byte ArtDmx packet at
wire Universe 0; queue acknowledgement or worker completion alone is not
electrical-wire or fixture-delivery proof. Daslight/Easy View must be closed
before Unity starts because the current receiver cannot share UDP port 6454;
Syndocal is a UDP sender and does not bind it.

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
control-plane `1 passed / 0 failed`; first-party warnings were `0`. The local
loopback UDP proof observes a 530-byte ArtDmx packet at wire U0 with 512
payload bytes, `payload[0]=255`, `payload[4]=255`, and masked `payload[499]=0`;
it is not an electrical venue-wire capture. Independent
Terra xHigh review found one stale Setup I/O fixture and a missing publication
rollback proof; both were repaired before the focused rerun. Ox was not
callable, so this is the documented narrow review exception.

The alpha.35 source, carried unchanged by the historical alpha.39
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

The alpha.35 hardening, carried unchanged by the historical alpha.39
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
and the historical alpha.39 native checkpoint does not change the strict
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

The following **historical alpha.42 checklist** records the accepted
source/Spout/native gates and keeps its final Unity/GPU physical row explicitly
open. It does not supersede the current USB-DMX checklist above:

- [x] Strict USB-serial show activation is completely replaced by exact
      `127.0.0.1:6454 / Art-Net / U0 / 512` activation.
- [x] The retired serial-show command, UI, schema, registry, tests, and
      show-only machine binding are unreachable or fail closed.
- [x] Universe 0 input/merge cannot alter the strict completed show frame in
      deterministic source tests; physical Unity output remains unchecked.
- [x] Channel 500 is deterministically forced to zero at the historical strict
      sender boundary; `payload[499]=0` was observed in a local loopback UDP
      packet test. This is not an electrical venue-wire capture.
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
