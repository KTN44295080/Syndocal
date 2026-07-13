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
The decorated Tauri window's actual maximized content client is 1920x1009; the
remaining 23 pixels are native title-bar chrome. Earlier notes that called
1920x1032 the native client size conflated the Windows work area with the app
content client. F11 removes both taskbar and title-bar reservations and must
produce a real 1920x1080 client.

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

`FFMPEG_DIR` must point to the LGPL shared SDK root containing `include`, `lib`
and `bin`. On the primary workstation the harness also recognizes the existing
`C:\temp\ffmpeg-n8.1-lgpl-shared` SDK. This preserves the production default
libav/Spout feature set instead of weakening the QA build.

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

The current working revision passed on 2026-07-14 03:22:33 JST. The native
client measured 1920x1009 maximized, 1920x1080 after F11, and 1920x1009 after
Esc. All three exact-window screenshots passed the non-blank visual gate:
maximized/fullscreen/restored sampled-color counts were 63/64/67, dark-pixel
ratios were 0.9860/0.9841/0.9856, and non-white ratios were
0.9992/0.9993/0.9992. The machine-readable report and inspected screenshots are
under
`%LOCALAPPDATA%\Temp\syndocal-native-acceptance-20260713-182219\native-window-acceptance.json`.
