# Video Physical Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `VIDEO-PHYSICAL-001` (section 8, Open)
- Q1 row: `COV-OUTPUT-LOCAL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `a44f43de4293666ddfe3506c5dee15df2c32bffe`
- Authority: `qa/M5_RELIABILITY_VALIDATION.md` and the output-control contracts

This checkpoint covers current-source video output routing, managed-window
lifecycle, bounded polling, and a current-host DirectShow camera
capture/restart slice. It does not claim physical display, HDMI output,
NDI/Spout receiver, or full video physical acceptance.

## Verification

```text
pnpm.cmd --dir app run check:video-output-routing-runtime
pnpm.cmd --dir app run check:video-output-window-runtime
pnpm.cmd --dir app run check:video-output-window-observation
pnpm.cmd --dir app run check:video-runtime-polling
```

Result: exit code 0.

- Video output routing R4 contract: PASS.
- Managed video-output window runtime: PASS, including exact-Both recovery,
  receipt rejection, singleflight, incarnation reducer, and zero legacy invokes.
- Video-output window observation contract: PASS.
- Video runtime polling: PASS for bounded current/transition requests,
  malformed-response rejection, generation retention, and zero-copy valid arrays.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun — 2026-09-14

The routing, managed-window runtime, window observation, and video polling
checks were rerun after takeover and passed. They again verified exact-Both
recovery, receipt rejection, singleflight, incarnation fencing, bounded
polling, malformed-response rejection, generation retention, and zero-copy
valid arrays. No display, HDMI output, NDI/Spout receiver, or physical output
was used in this contract rerun.

## Current-host physical camera slice — 2026-09-14

The production capture worker was exercised against the currently enumerated
`ASUS 5M webcam` through the canonical DirectShow profile endpoint. The
selected profile was the exact device alternative identity
`@device_pnp_\\?\usb#vid_636e&pid_0bda&mi_00#7&154dae8b&0&0000#{65e8773d-8f56-11d0-a3b9-00a0c9223196}\global`,
`nv12`, `1280x720`, `30/1`. FFmpeg was the explicit local
`C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin\ffmpeg.exe`.

With the pinned MSVC `14.44.35207` x64 linker, the ignored test
`camera_worker_captures_a_real_frame_and_restarts_cleanly` passed
`1 passed; 0 failed; 0 ignored`. It completed two worker start/stop attempts,
received the expected `1280x720` RGBA frame on each attempt, and rejected a
fully transparent frame. This is a partial physical capture/restart result;
it does not prove unplug/replug, HDMI/display pixels, NDI/Spout, frame drops,
one-hour, venue, or product completion.

## Unresolved acceptance

`VIDEO-PHYSICAL-001` stays Open. The required real display topology, HDMI/video
output, NDI/Spout receiver, camera fault/unplug-replug, reconnect, frame-drop,
and one-hour acceptance must be exercised with named hardware and raw
observations. The camera slice above proves only two clean current-source
capture/restart cycles; it cannot prove pixels reached a real display or
receiver.

Next action is the physical display/capture matrix with exact resolution,
receiver, source, frame-rate, reconnect, and dropped-frame records.

## Takeover continuation — current-source video-output recheck — 2026-09-14

At HEAD `d9908a8f`, video output routing, managed output-window runtime,
output-window observation, and video runtime polling all passed. The runtime
again confirmed exact-Both recovery, receipt rejection, singleflight,
incarnation fencing, malformed-response rejection, generation retention, and
zero-copy valid arrays. No display/HDMI output, NDI/Spout receiver, or new
camera capture was opened. The existing ASUS webcam two-cycle slice remains
partial evidence; `VIDEO-PHYSICAL-001` stays `Open`.

## Current HEAD output-contract recheck — 2026-09-14

At current HEAD `678e6960`, after the secondary-display native gate
checkpoint, the complete current-source video-output contract set was rerun:

```text
pnpm.cmd --dir app run check:video-output-routing-runtime
pnpm.cmd --dir app run check:video-output-window-runtime
pnpm.cmd --dir app run check:video-output-window-observation
pnpm.cmd --dir app run check:video-runtime-polling
```

All four commands exited `0`. Routing reported the R4 contract PASS;
the managed-window check passed exact-Both recovery, receipt rejection,
singleflight, incarnation reduction, and zero legacy invokes; the observation
contract passed; and polling passed bounded current/transition requests,
malformed-response rejection, generation retention, and zero-copy valid arrays.
No physical display/HDMI output, NDI/Spout receiver, camera fault/replug,
external client, or one-hour run was opened by this recheck. The result
strengthens only the current-source software sub-gate and does not change
`VIDEO-PHYSICAL-001`'s `Open` status.

## Secondary-display follow-up and current gate wiring — 2026-09-14

The earlier native-window sample recorded only two displays. A fresh
DisplayConfig dry-run after the additional display was connected found three
active displays. The exact inventory is retained at
`target/qa/three-display-inventory-20260914-f96d312348f645cdbc58fabc5b2295c6/monitors.json`:

```text
DISPLAY1 / stable \\?\DISPLAY#TMA0803#5&2a56f61f&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7} / 2560x1600 / effective DPI 192 / work 2560x1504
DISPLAY2 / stable \\?\DISPLAY#RTK0000#5&2a56f61f&0&UID261#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7} / 1920x1200 / effective DPI 96  / work 1920x1152
DISPLAY3 / stable \\?\DISPLAY#LKGF803#5&2a56f61f&0&UID281#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7} / 3840x2160 / effective DPI 144 / work 3840x2088
```

The read-only `WmiMonitorListedSupportedSourceModes` query reported one
source mode (2560x1600) for TMA0803, 1920x1080 and 1920x1200 modes for
RTK0000, and both 3840x2160 and 1920x1080 modes for LKGF803. This confirms
that LKGF803 cannot simultaneously serve as the required 4K projector and a
second independent 1920x1080 LED role.

The three-display count is therefore confirmed. The strict observer's three
role contract is not yet satisfied: the projector role matches exactly, but
the editor role requires 1920x1080@96 while DISPLAY2 is currently 1920x1200,
and the LED role requires a distinct 1920x1080@144 display while DISPLAY1 is
currently 2560x1600@192. This is a display-mode/DPI mismatch, not a missing
monitor count. No Display output was added to the user's persisted project
because the available UI path is a durable Add operation and the current source
intentionally has no canonical remove operation. No HDMI, NDI/Spout, or
external receiver was therefore driven merely to create temporary state.

The exact current-checkout executable `target/release/syndocal.exe` was
started once for a read-only native-window availability check. Windows
reported one responsive `Syndocal` process, but the Computer Use surface did
not return that native window as a targetable app. No guessed HWND, coordinate,
or UI automation fallback was used; the exact process was then stopped. This
is an automation-surface boundary, not physical output evidence.

## Acceptance-scope clarification — 2026-09-14

The exact `1920x1080` editor, distinct `1920x1080` LED, and `3840x2160`
projector role binding belongs to the stricter Near-show `SHOW-P0-4` contract
and its dedicated three-display observer. It is not a prerequisite for the
generic `VIDEO-PHYSICAL-001` matrix in the completion flow. The generic marker
can therefore use the current three-display setup for a bounded practical
display/HDMI/fullscreen test even when the displays do not match those exact
Near-show roles, provided the actual output windows are bound to exact display
identities and the observed resolution, refresh/DPI, nonblank pixels, and safe
fault/reconnect legs are recorded.

This narrows the interpretation; it does not create physical evidence. The
current host confirms three active displays, but no current app-owned output
window, fullscreen pixels, HDMI/NDI/Spout receiver, unplug/reorder/GPU-reset
observation, or one-hour frame/drop run has yet been captured. `VIDEO-PHYSICAL-
001` remains Open, while the stricter Near-show role-binding gate remains a
separate open boundary.

## Observation attempt — 2026-09-14

The requested live observation could not start because the current Computer
Use surface exposed no native applications (`cua.getState()` returned
`apps: []`), while the native helper returned `Trusted RPC service is not
configured: sky` from `sky.list_apps()`. No HWND, coordinate, focus change,
input, output creation, or physical-display claim was made. A process check
also found no currently running `syndocal.exe`. The next safe action is to
restore the native Computer Use surface, start the exact current-checkout
executable, and then capture the practical display/output observation under
the narrowed generic marker scope.

The previously unregistered current-source display-target checker was repaired
and wired into the package gate:

```text
node app/scripts/check-video-display-target.mjs                         PASS
pnpm.cmd --dir app run check:video-display-target                       PASS
pnpm.cmd --dir app run check:release:static                             PASS
```

The checker repair only made its source-boundary search CRLF-safe and aligned
its assertions with the current typed `display-authority` lease bypass. The
package gate now runs it alongside routing and output-window checks. The
current source contracts are green, but `VIDEO-PHYSICAL-001` remains Open:
real fullscreen pixels, refresh/DPI/reorder/unplug/GPU reset, HDMI/NDI/Spout
receiver observations, camera fault/replug, frame-drop records, and the
one-hour run still require named physical equipment and raw observations.

## Takeover continuation — current-source video/output recheck after upper-desk repair — 2026-09-14

At current source HEAD `8f8a9cd5`, the focused video/output contracts all
exited `0`:

```text
pnpm.cmd run check:video-runtime-polling
current clip: invokes=1, maxConcurrency=1
current transition: invokes=1, maxConcurrency=1
video runtime polling deferred-response checks passed
omitted native runtime arrays, malformed rejection, generation retention,
and zero-copy valid arrays: PASS

pnpm.cmd run check:video-output-routing-runtime
video output routing R4 contract: PASS

pnpm.cmd run check:video-display-target
video display target contract: PASS

pnpm.cmd run check:video-output-window-runtime
video output window runtime contract: PASS

pnpm.cmd run check:video-output-window-observation
video-output window observation contract passed
```

For the user's current three-display setup, the exact release executable was
started for a read-only availability check. Windows reported one responsive
process at `target/release/syndocal.exe` (PID `41156`, non-zero main window
handle). This proves process/window availability only; no guessed native UI
automation, fullscreen pixel sample, HDMI/NDI/Spout receiver, camera fault,
unplug/reorder/GPU-reset, or frame-drop/one-hour artifact was produced.
`VIDEO-PHYSICAL-001` remains `Open` pending named physical display/output
observations and the required recovery/soak matrix.

## Takeover continuation — read-only three-display harness attempt — 2026-09-14

The read-only three-display harness was rerun at the current checkpoint with
no `-Apply` and no role/artifact mutation parameters:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File qa/harnesses/run-syndocal-three-display-show-acceptance.ps1 -EvidenceSlug current-display-observation-20260914
exit code 0
verdict: not-configured
accepted: false
native_hardware_claim: false
```

The harness explicitly reported that no acceptance was evaluated because the
identity/provenance/output-role configuration was incomplete; its sample was
null. No project Add, output-window creation, HDMI/NDI/Spout send, or physical
display mutation occurred. This is a safe preflight result, not a three-display
acceptance result, and `VIDEO-PHYSICAL-001` remains `Open`.

## Current display topology recheck — 2026-09-14

The exact same read-only DisplayConfig preflight was rerun after the release
process was started:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File qa/harnesses/run-syndocal-three-display-show-acceptance.ps1 -EvidenceSlug current-three-display-preflight-20260914b
exit code 0
verdict: not-configured
accepted: false
native_hardware_claim: false
evidence: target/qa/current-three-display-preflight-20260914b-79139819d7874040858a33fc38ead5a7
```

The current active DisplayConfig inventory in `monitors.json` contains only
two connected monitors: `TMA0803` at physical `2560x1600` with effective DPI
`192`, and `RTK0000` at `1920x1200` with effective DPI `96`. The harness's
`before.sample` is `null` because no exact artifact, role identities, output
IDs, or CDP observation provider were supplied. A separate Windows Forms
enumeration also returned only `DISPLAY1` and `DISPLAY2` (its primary value is
DPI-virtualized to `1280x800`), so the third display is not treated as active
current state.

This current observation does not erase the earlier retained three-display
inventory; it records that the topology is not presently reproducible. No
display mode, project state, output window, focus, Z-order, HDMI/NDI/Spout
send, or physical device was changed. `VIDEO-PHYSICAL-001` remains `Open`.
The next safe action is to reconnect or re-enable the third display, rerun the
read-only inventory, and only then provide exact output-window identities for
the practical fullscreen/pixel and recovery matrix.

## PnP/WMI corroboration — 2026-09-14

Read-only device enumeration corroborates the two-display DisplayConfig result:
`DISPLAY\\TMA0803\\5&2A56F61F&0&UID256` and
`DISPLAY\\RTK0000\\5&2A56F61F&0&UID261` are `Present=true` with `Problem=0`.
The previously recorded third identity
`DISPLAY\\LKGF803\\5&2A56F61F&0&UID281` is currently
`Present=false` with `Problem=45`; the WMI monitor list likewise reports only
the TMA0803 and RTK0000 identities as `Active=true`. This is an OS/device
presence observation, not a diagnosis of the cable, input, or display power
state. No device enable/disable, display-setting mutation, or output action was
performed.

## Current topology recheck — 2026-09-15

The read-only three-display harness was rerun after the latest user report:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File qa/harnesses/run-syndocal-three-display-show-acceptance.ps1 -EvidenceSlug current-three-display-preflight-20260915
exit code: 0
verdict: not-configured
native_hardware_claim: false
accepted: false
```

The generated DisplayConfig inventory currently contains two connected
monitors: `DISPLAY1` at `2560x1600` with effective DPI `192` and `DISPLAY2` at
`1920x1200` with effective DPI `96`. No third connected monitor identity was
returned in this run. The harness sample was null because exact output-role
identities, an artifact, and a CDP observation provider were not configured.
No display mode, project state, output window, HDMI/NDI/Spout send, or physical
device was changed. This is a topology/preflight result only;
`VIDEO-PHYSICAL-001` remains `Open` pending the third display identity, exact
output-window binding, pixel/receiver observation, fault/reconnect matrix, and
one-hour frame/drop evidence.

## Current-host physical camera recheck — 2026-09-15

The current PnP inventory still reports `ASUS 5M webcam` as present. After
correcting the test input to the canonical opaque profile endpoint, the
production capture worker was rerun with the exact release profile and pinned
MSVC `14.44.35207` linker:

```text
SYNDOCAL_FFMPEG=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin\ffmpeg.exe
SYNDOCAL_TEST_CAMERA_ENDPOINT=syndocal-camera-v1:eyJkZXZpY2VfYWx0ZXJuYXRpdmVfbmFtZSI6IkBkZXZpY2VfcG5wX1xcXFw_XFx1c2IjdmlkXzYzNmUmcGlkXzBiZGEmbWlfMDAjNyYxNTRkYWU4YiYwJjAwMDAjezY1ZTg3NzNkLThmNTYtMTFkMC1hM2I5LTAwYTBjOTIyMzE5Nn1cXGdsb2JhbCIsImlucHV0X2Zvcm1hdCI6eyJraW5kIjoicGl4ZWxfZm9ybWF0IiwidmFsdWUiOiJudjEyIn0sIndpZHRoIjoxMjgwLCJoZWlnaHQiOjcyMCwiZnJhbWVfcmF0ZV9udW1lcmF0b3IiOjMwLCJmcmFtZV9yYXRlX2Rlbm9taW5hdG9yIjoxfQ
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 camera_worker_captures_a_real_frame_and_restarts_cleanly -- --ignored --nocapture --test-threads=1
```

```text
Finished `release` profile [optimized] target(s) in 1.08s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1928 filtered out; finished in 2.55s
```

The test completed two clean worker start/stop attempts, received the selected
`1280x720` RGBA frame shape and byte count on each attempt, and rejected a
fully transparent frame. No display/output window, HDMI/NDI/Spout receiver,
or other physical output was created. This strengthens only the physical
camera capture/restart slice; `VIDEO-PHYSICAL-001` remains `Open` pending exact
display/output identity and pixel/receiver observations, camera fault/replug,
reconnect, frame-drop, and one-hour evidence.
