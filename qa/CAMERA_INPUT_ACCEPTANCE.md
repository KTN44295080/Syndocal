# Syndocal camera input acceptance

Status date: 2026-08-31

Branch: `codex/syndocal-v1.2`

## 2026-09-01 alpha.57 native current boundary

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

## Historical 2026-08-31 alpha.45 source/test checkpoint — camera native and hardware gates pending

At that historical checkpoint, product metadata was `1.2.0-alpha.45` on
`codex/syndocal-v1.2`. This was a source/test checkpoint only: no alpha.45
native build, launch/window verification, camera UI/profile listing, opaque
endpoint probe, sustained capture, HDMI/content observation, or other physical
hardware acceptance is claimed. The alpha.39 and earlier native/camera records
below remain immutable historical evidence and must not be relabeled as
alpha.45.

The recorded full-workspace Cargo result passed, including Syndocal
`1389 pass / 0 fail / 14 ignored` and Video `163 pass / 0 fail / 1 ignored`.
The focused output-lease keepalive gate passed `69/69`. These deterministic
source results do not prove an alpha.45 camera device, native window, capture
profile, sustained performance, or hardware result. All camera-native and
physical rows remain pending until observed against an identified alpha.45
executable and device.

## 2026-08-30 historical alpha.39 native checkpoint

The historical product checkpoint was `1.2.0-alpha.39` on branch
`codex/syndocal-v1.2`; the native build used source/docs `HEAD` and upstream
`ec93e9160da853ad181de70aee4db7b4a75fafbb`, which were equal at the build
checkpoint. This tranche repairs the Timeline
authoring-output selector's
exact UI truth: only one exact selectable occurrence is admitted, while
duplicate, missing, or ambiguous identities fail closed; status-only polling
may reapply the exact desired option without list/configuration/routing
mutation. Browser Phase A/B passed after the strict external-video status-poll
fixture was added; TypeScript, runtime, video-poll, `git diff --check`, and
`pnpm --dir app run check:release` pass. Independent Terra select review is GO
with P0/P1 `0`; the checker review is GO with P0/P1/P2 `0`. The exact MSVC
`14.44.35207` Community linker was pinned and first in `where.exe`;
`pnpm --dir app tauri build --no-bundle` exited `0` in `2m57s` with first-party
warnings `0`. The resulting `61,108,736`-byte executable reports Product/
FileVersion `1.2.0-alpha.39` and SHA-256
`7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
Exactly one checkout-owned PID `87640` is responsive with maximized `Syndocal`
window id `2033716740`; Daslight was preserved. Native alpha9 Timeline selector
reverification passed at the exact Music endpoint without a visible Backend,
Local IPC, or CUE fault. The exact sequence was Play -> Pause -> status-only
wait -> Play -> Pause -> status-only wait. The reviewed native-evidence record
was committed and pushed at `94b362bd2d733e447feabf0a0a6158699da6a2bf`, and
`HEAD`/upstream equality was verified immediately after that push.

Camera and strict Art-Net behavior are unchanged by this Timeline-output
tranche. The alpha.39 normal native build/window and Timeline selector/UI gates
are complete only; no alpha.39 camera UI/profile, capture, audible, Unity/GPU,
Art-Net, Spout, DJ, or other hardware acceptance has run. The next safe action
is operator audible confirmation. The alpha.38 artifact/PID/window and
observed `PX160` drift remain immutable historical evidence and must not be
relabeled as alpha.39.

Product tranche: historical `1.2.0-alpha.39` native checkpoint at build source/docs
`HEAD` and upstream `ec93e9160da853ad181de70aee4db7b4a75fafbb`; the historical
alpha.38 source/build authority remains
`e4ec22384675aace5ed3912ddffdcfecca190919`. The latest accepted
camera-specific native evidence remains historical alpha.37. No alpha.39 camera
UI/profile, capture, sustained-performance, or hardware evidence is claimed.

## Historical product boundary (alpha.39 and earlier)

Historical alpha.38 changes Timeline CUE transport authority and does not alter
the camera catalog, capture, or probe implementation. The historical alpha.39
normal native build/window likewise changes no camera path; no alpha.39
camera-specific UI, profile, capture, or hardware acceptance has run. The
alpha.37 camera artifact below remains historical camera evidence and must not
be relabeled as alpha.38 or alpha.39 camera or hardware acceptance.

The historical alpha.38 exact native executable is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
`61,114,368` bytes, Product/FileVersion `1.2.0-alpha.38`, SHA-256
`9E5CA0DB826D9998F7CDC76214E5CDC17597A6D4FC1CC27FEB5EF72D6E28FA37`.
`pnpm --dir app tauri build --no-bundle` used exact MSVC `14.44`, completed in
`3m12s`, and recorded first-party warnings `0`. Old checkout-owned PID `50864`
was stopped only after exact-path verification; Daslight PID `42752` was
preserved. New exact PID `55624` is responsive with one maximized `Syndocal`
window; Computer Use plus Alt+Space verified `Maximize` disabled and `Restore`
enabled. Preserve PID `55624` only as historical evidence; do not treat that
alpha.38 process as alpha.39. Do not close Daslight without operator approval.

Alpha.32 and earlier accepted a free-form camera endpoint and opened the
Windows DirectShow path at a fixed `1280x720` / `30 fps`. That path could not
truthfully select a camera's advertised 4K or high-frame-rate mode.

Alpha.33 replaces that camera entry path with native device and profile
enumeration. The UI may persist only an opaque endpoint returned by the current
native catalog. A raw device name, default camera alias, malformed endpoint,
stale selection, or selection that has not passed its exact one-frame probe is
rejected; Syndocal does not silently substitute another camera, resolution,
rate, codec, or pixel format.

The current admission envelope is:

- maximum frame size: `4096x2160`;
- maximum advertised capture rate: `120 fps`;
- profiles above `1920x1080` are admitted at no more than `30 fps` in this
  show-critical tranche;
- profiles above `1280x720` are admitted at no more than `60 fps`;
- rates above `60 fps` are capture sampling only because the current native
  Program and NDI presentation loops are capped at `60 Hz`;
- screen capture is intentionally unchanged at `1280x720` / `30 fps` in this
  tranche.

The profile probe proves that FFmpeg can open the exact current DirectShow
alternative identity and produce one complete raw frame at the requested
tuple. It does not prove a one-hour camera/output soak or independently measure
the camera's sustained cadence.

## 2026-08-29 device evidence

On the current Windows workstation, FFmpeg DirectShow option enumeration for
`Insta360 Link` advertises these relevant modes:

| Profile | Advertised maximum | Codec evidence |
| --- | ---: | --- |
| `3840x2160` | `30 fps` | MJPEG and H.264 |
| `1920x1440` | `60.0002 fps` | MJPEG and H.264 |
| `1920x1080` | `60.0002 fps` | MJPEG and H.264 |
| `1280x720` | `60.0002 fps` | MJPEG and H.264 |

No `120 fps` profile was advertised by this connected camera. Syndocal must not
invent one. A different camera may expose an at-most-720p profile up to 120 fps;
that path remains source-tested but cannot receive physical acceptance from the
current device.

As a pre-native hardware check, the same locally resolved FFmpeg executable
opened the physical camera by its display name and emitted one complete RGBA
frame to the Windows null sink for both relevant tuples: `3840x2160` / `30 fps`
MJPEG exited `0` in `2423 ms`, and `1920x1080` / `60 fps` MJPEG exited `0` in
`1983 ms`. This proves the device/driver/FFmpeg tuple is usable on this PC, but
it does not replace the current opaque-endpoint probe or sustained-output
gates below.

The direct preflight also completed `150` RGBA frames at 4K30 in `7205 ms` and
`300` RGBA frames at 1080p60 in `7023 ms`, both exit `0` with no FFmpeg error
text. Those wall times include camera open/close and represent roughly five
seconds of requested capture. They establish short device/driver decode
throughput only; they do not exercise Syndocal's 60 Hz renderer clone path.

## Source gates

The current implementation/source-review gate has established:

- exact DirectShow video-device and profile parsing, including fractional
  `60.0002 fps`;
- current-generation opaque endpoint identity and exact tuple revalidation;
- exact FFmpeg argv without a shell and without a camera scale filter;
- fail-closed unsupported-platform behavior;
- stale probe responses cannot re-enable Add Video Layer;
- post-ready capture loss evicts the last frame instead of presenting it as a
  continuing live image;
- product command registration, typed frontend invocation inventory, and
  control-plane taxonomy agree.

The historical alpha.34 camera-tranche supervising exact-linker runs used MSVC
`14.44.35207`, with the Community linker first in `where.exe`, and passed:

- capture-filtered tests: `71 passed`, `0 failed`, `2 ignored`;
- process-lifecycle tests: `3 passed`, `0 failed`, `0 ignored`;
- control-plane tests: `64 passed`, `0 failed`, `0 ignored`;
- full no-default-feature app suite, serialized: `1203 passed`, `0 failed`,
  `7 ignored`;
- full default-feature app suite, serialized: `1239 passed`, `0 failed`,
  `12 ignored`;
- first-party warnings: `0`.

The first parallel no-default run reported two unrelated coordination-test
timeouts after `1201` passes. Both failed tests passed individually with one
test thread, and the complete no-default and default suites then passed with
`--test-threads=1`; no source change was made to conceal the parallel result.

Independent static review found no P0/P1/P2. Its earlier findings for a bounded
FFmpeg listing-output limit and complete child-process cleanup after every
post-spawn failure are closed in source. A child not reaped inside the bounded
caller deadline transfers to a detached reaper; reaper spawn/send failures
retain the Child in an in-process quarantine and retry outside the quarantine
lock. Deterministic tests prove real terminal status, spawn failure, send
failure, quarantine, retry, and reap. The status response now reports
structured per-route capture faults and counts only healthy routes as active.
The 1 Hz frontend status poll removes the same faulted route from Active and
renders a specific actionable fault row; its focused checker, type check, and
localization gate pass. The sustained-4K performance boundary remains
deliberately open.

The final independent read-only rereview is GO for this source checkpoint with
no P0/P1/P2. Windows Job Object containment is not installed, so Syndocal
process exit while a deferred reaper is pending remains an explicit unverified
OS boundary. Camera UI/profile and sustained-4K acceptance remain NO-GO until
the unchecked gates below are observed on an identified alpha.45 executable.
The alpha.45 native build/window gate remains pending; the historical alpha.39
and alpha.38 native build/window gates are recorded above.

The historical 2026-08-29 post-FFmpeg-7 repair gate additionally pins exact MSVC
`14.44.35207` with the Community linker first in `where.exe` and passes the
historical `capture_catalog` set `21 passed / 0 failed / 0 ignored`, first-party
warnings `0`. The parser now accepts only complete adjacent DirectShow
video/alternative-identity pairs from one strict dshow source and requires the
terminal `Error opening input file dummy.` marker for the modern heading-free
FFmpeg 7 listing. Audio/none entries, foreign/intervening sources, malformed
identities, incomplete listings, and raw device-name disclosure fail closed.
Independent Terra xHigh rereview is GO. Native UI/profile evidence remains
unchecked below.

## Performance claim boundary

The capture registry owns one latest frame per camera layer, so it does not
accumulate an unbounded queue. However, the current renderer/NDI decoder clones
the RGBA frame at a fixed 60 Hz. A `4096x2160` RGBA frame is approximately
`33.75 MiB`; repeated 60 times per second this can approach `1.98 GiB/s` of
copy traffic before compositing and upload. The one-frame profile probe does
not close that performance risk.

Therefore alpha.37 may claim explicit 4K30 and 1080p60 profile selection and
probe after native evidence, but must not claim arbitrary sustained 4K stage
output or 4K60. A future shared/pooled frame handoff with generation-based
render pacing is required before broadening that claim.

## Historical alpha.45 native and hardware gates still required

- [x] Historical alpha.34 release build with MSVC 14.44, first-party warnings 0.
- [x] Historical alpha.35 release rebuild with MSVC 14.44, first-party
  warnings 0.
- [x] Historical alpha.36 release rebuild from clean pushed HEAD `81a7a9c`
  with MSVC 14.44, first-party warnings 0.
- [x] Historical alpha.36 window was responsive and maximized before its exact
  path was verified and PID `109972` was stopped for the next native build.
- [x] Latest accepted, now-historical alpha.37 release rebuild from clean pushed HEAD `5626a96`
  with MSVC 14.44, first-party warnings 0.
- [x] Historical proof records exactly one responsive, maximized window from this checkout's alpha.37
  release executable (PID `50864`; Computer Use system-menu proof).
- [x] Historical alpha.38 normal native build/window gate passed from source/build
  HEAD `e4ec22384675aace5ed3912ddffdcfecca190919` with exact MSVC 14.44;
  the `61,114,368`-byte executable is Product/FileVersion `1.2.0-alpha.38`
  with SHA-256
  `9E5CA0DB826D9998F7CDC76214E5CDC17597A6D4FC1CC27FEB5EF72D6E28FA37`.
  Exactly one responsive, maximized Syndocal window is PID `55624`.
- [x] Historical alpha.39 exact-MSVC normal native build, launch/maximize, and
  alpha9 Music Play/Pause/status-only selector/UI reverify are recorded above.
- [ ] Operator audible confirmation remains open and is not camera acceptance.
- [ ] Native UI lists `Insta360 Link` without accepting a raw/default name.
- [ ] Exact `3840x2160` / `30 fps` profile probe succeeds.
- [ ] Exact `1920x1080` / `60.0002 fps` profile probe succeeds.
- [ ] Camera layer start/stop is exercised for both accepted profiles.
- [ ] Disconnect after readiness removes the last frame and changes the normal
  UI from Active to a specific actionable fault without requiring manual
  `Check I/O`.
- [ ] Short sustained Preview/Program run records frame/deadline behavior at
  4K30 and 1080p60; any missed-frame limit is declared before the run.
- [ ] Physical unplug/replug recovery is accepted or remains explicitly open.
- [ ] A real 120fps camera is tested before any physical 120fps claim.
- [ ] One-hour representative camera/output soak remains a release decision
  gate and is not implied by this show-critical checkpoint.
