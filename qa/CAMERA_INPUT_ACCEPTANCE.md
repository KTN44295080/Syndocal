# Syndocal camera input acceptance

Status date: 2026-08-29

Branch: `codex/syndocal-v1.2`

Product tranche: `1.2.0-alpha.33`

## Current product boundary

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
it does not replace the alpha.33 opaque-endpoint probe or sustained-output
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

The supervising exact-linker focused run used MSVC `14.44.35207`, with the
Community linker first in `where.exe`, and passed:

- capture tests: `64 passed`, `0 failed`, `2 ignored`;
- control-plane tests: `64 passed`, `0 failed`, `0 ignored`;
- full no-default-feature app suite: `1196 passed`, `0 failed`, `7 ignored`;
- full default-feature app suite: `1232 passed`, `0 failed`, `12 ignored`;
- first-party warnings: `0`.

Independent static review found no P0. Its implementation P1 findings for a
bounded FFmpeg listing-output limit and complete child-process cleanup after
every post-spawn failure are closed in source. The status response now reports
structured per-route capture faults and counts only healthy routes as active.
The 1 Hz frontend status poll removes the same faulted route from Active and
renders a specific actionable fault row; its focused checker, type check, and
localization gate pass. The sustained-4K performance boundary remains
deliberately open.

The final independent read-only rereview is GO for this source checkpoint with
no P0/P1. It remains NO-GO for native/UI/sustained-4K acceptance until the
unchecked gates below are observed on the versioned executable.

## Performance claim boundary

The capture registry owns one latest frame per camera layer, so it does not
accumulate an unbounded queue. However, the current renderer/NDI decoder clones
the RGBA frame at a fixed 60 Hz. A `4096x2160` RGBA frame is approximately
`33.75 MiB`; repeated 60 times per second this can approach `1.98 GiB/s` of
copy traffic before compositing and upload. The one-frame profile probe does
not close that performance risk.

Therefore alpha.33 may claim explicit 4K30 and 1080p60 profile selection and
probe after native evidence, but must not claim arbitrary sustained 4K stage
output or 4K60. A future shared/pooled frame handoff with generation-based
render pacing is required before broadening that claim.

## Native and hardware gates still required

- [ ] Exact alpha.33 release build with MSVC 14.44, first-party warnings 0.
- [ ] Exactly one responsive, maximized window from this checkout's release
  executable.
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
