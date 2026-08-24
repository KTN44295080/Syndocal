# Native Window Acceptance

Date: 2026-07-14
Primary host: Windows, 1920x1080 monitor

## Policy

Syndocal is a full-window show-control desk. Compact browser runs are retained
for containment and reachability, but they are not the visual or operational
sign-off surface.

The release UI order is:

1. Browser visual and interaction gate at 1920x1080.
2. Native Windows gate in an isolated Tauri application: maximized client at
   least 1920x1000, F11 fullscreen client exactly 1920x1080, then Esc restoring
   the exact original maximized client size.
3. Browser measured-work-area fixture at 1920x1032 and extended ceiling at
   2048x1152.
4. Compact containment fallbacks at 1366x768 and 1280x720.

On this workstation the 1920x1080 monitor has a 1920x1032 Windows work area.
Since T25-E (2026-07-30) the main window is frameless (`decorations: false`)
with the integrated top bar acting as the title bar, so the maximized content
client equals the full work area: 1920x1032. The pre-T25-E decorated client
was 1920x1009 (23 pixels of native title-bar chrome); that value is historical
only. F11 removes the taskbar reservation as well and must produce a real
1920x1080 client; Esc must restore the exact 1920x1032 maximized client.

## Commands

```powershell
pnpm --dir app run check:release-ui
```

This is the primary release UI aggregate. On Windows it runs the desktop
window contract, the complete browser viewport suite, focused timeline
viewport checks, and the real native maximized/F11/Esc acceptance.

Sign-off evidence must come from the current checkout's isolated QA build or
from the release artifact built from the same revision. An already-running
older binary, including one that still opens as a small window, is useful only
for diagnosis or comparison and can never satisfy the primary gate.

For headless/browser-only diagnosis:

```powershell
pnpm --dir app run check:release-ui:browser
```

For the native gate alone:

```powershell
pnpm --dir app run check:native-window
```

The native launcher uses the separate identifier
`jp.seraf.ktn.syndocal.qa.native-acceptance`, title
`Syndocal QA - Native 1920 Acceptance`, Vite port 5187, and a Cargo target in
the OS temporary directory. It never selects or sends keys to the normal
Syndocal or Daslight windows. The launcher closes only its own QA process tree.

`FFMPEG_DIR` must point to a complete shared SDK. The harness validates all
three directories and the files needed by the current `ffmpeg-next` feature set:
`include/libavcodec/avcodec.h`, `include/libavformat/avformat.h`,
`include/libavutil/avutil.h`,
`include/libswscale/swscale.h`; the matching MSVC import libraries
`avcodec.lib`, `avformat.lib`, `avutil.lib`, and `swscale.lib`; and non-empty
runtime DLLs under `bin`. When versioned `.def` metadata is present, the DLL
major must match it; otherwise the validator requires a non-empty DLL from the
corresponding library family.

When `FFMPEG_DIR` is unset, the harness first discovers a complete installed
Gyan FFmpeg Shared SDK under the user's WinGet package directory. It may then
inspect `C:\temp\ffmpeg-n8.1-lgpl-shared` and its versioned children as a fallback,
but directory existence alone is never sufficient. An incomplete candidate is
reported with its missing headers, import libraries, or runtime DLLs and the
gate fails closed if no complete SDK can be selected. This preserves the
production default libav/Spout feature set instead of weakening the QA build.
SDK discovery and completeness do not certify a redistribution license. The
legacy `C:\temp\ffmpeg-n8.1-lgpl-shared` directory name is not license evidence;
public artifacts must still pass the separate third-party licensing gate for the
exact FFmpeg binaries they distribute.

The 2026-08-25 fallback hardening checkpoint passed PowerShell parsing and
function-level validation in both native scripts: the helper implementations
were byte-identical; an unset variable selected the complete WinGet SDK; an
explicit complete SDK was accepted; the known partial SDK was rejected; and
versioned `.def` metadata rejected a stale-major runtime DLL. Independent Ox
and Terra reviews found no P0/P1 after the fixes. The obsolete partial trees
`C:\temp\ffmpeg-n8.1-lgpl-shared` and
`C:\temp\syndocal-ffmpeg-sdk-20260813` were moved to the Recycle Bin (4 files
and 806,696 bytes each; 1,613,392 bytes total, recoverable). The complete
217-file / 296,016,041-byte WinGet SDK remained intact at the path above.

## Evidence

Each run writes three client screenshots and
`native-window-acceptance.json` under a new
`%TEMP%\syndocal-native-acceptance-*` directory. The JSON records monitor and
work-area dimensions, exact maximized/fullscreen/restored client dimensions,
tolerance and screenshot paths. Generated evidence stays outside the source
tree.

The gate fails if the QA window is not maximized, the monitor is not the
required 1920x1080 primary surface, maximized content is smaller than
1920x1000, F11 is not exactly 1920x1080, or Esc does not return to the exact
original maximized dimensions.

## Latest verified run

The T25-E frameless revision passed supervisor native acceptance on 2026-07-30
(WebView2 CDP + Win32 window-rect evidence): maximized client 1920x1032, F11
fullscreen client 1920x1080, Esc restore to the pre-F11 client. Frameless
window operations were verified natively in the same run: top-bar drag region
moved the window by exact commanded deltas (three drags, cross-checked against
`GetWindowRect` because `window.screenX` is stale in WebView2), double-click
on the bar toggled maximize both ways, the 40x40 minimize/maximize/close
controls worked (`IsIconic` true on the `Tauri Window` class handle after
minimize; close exited the process cleanly through the `CloseRequested` path),
and an 8-direction edge resize grew the restored window 1280x800 -> 1380x850
with exact deltas.

The prior decorated-window run passed on 2026-07-14 03:22:33 JST. The native
client measured 1920x1009 maximized, 1920x1080 after F11, and 1920x1009 after
Esc. All three exact-window screenshots passed the non-blank visual gate:
maximized/fullscreen/restored sampled-color counts were 63/64/67, dark-pixel
ratios were 0.9860/0.9841/0.9856, and non-white ratios were
0.9992/0.9993/0.9992. The machine-readable report and inspected screenshots are
under
`%LOCALAPPDATA%\Temp\syndocal-native-acceptance-20260713-182219\native-window-acceptance.json`.
