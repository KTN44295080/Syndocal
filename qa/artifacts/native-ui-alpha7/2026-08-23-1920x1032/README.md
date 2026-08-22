# Syndocal 1.2.0-alpha.7 native UI evidence — 2026-08-23

## Build identity

- Branch: `codex/syndocal-v1.2`
- HEAD at evidence capture: `da02e3fd20190b1e8656023c09ec532f3b9016d7`
- Product/File version: `1.2.0-alpha.7`
- Executable: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Size: `57,099,264` bytes
- SHA-256: `00253F26A8D3A933172D7B07923E430B455CA18F98E439CE45C5B62405F86EF4`

Immediately before the build, only a process resolving to the exact executable
path above was eligible for termination and the exact-path count was verified as
zero. The required native command passed:

```text
pnpm --dir app tauri build --no-bundle
```

The successful command ran under the Visual Studio 2022 Community amd64 developer
environment (`VsDevCmd.bat -arch=amd64`) with the installed shared FFmpeg SDK:

```text
FFMPEG_DIR=C:\Users\kouty\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-8.1.2-full_build-shared
INCLUDE=%FFMPEG_DIR%\include;%INCLUDE%
LIB=%FFMPEG_DIR%\lib;%LIB%
PATH=%FFMPEG_DIR%\bin;%PATH%
```

Result: **PASS**. This is the alpha.7 native release executable used for every
screenshot in this directory.

## Process and native-window proof

The exact executable launched as PID `100260`. Inspection found exactly one
exact-path process and exactly one visible main window titled `Syndocal`, with
`Responding=True`. The same PID predates all eight captures. It was explicitly
maximized before native UI actions.

- Primary native pass: maximized `1920x1032` client; screenshots `01`–`07`.
- Supplemental extended-width pass: the same responsive PID, `IsZoomed=True`,
  window rect `1913,-371,3975,747`, client `2048x1104`; screenshot `08`.

The 1920 pass exercised the empty Lighting desk, closed/open Video import surface,
and Timeline normal, expanded, expanded-tools, and restored states. The UI remained
contained and the lower panes/actions shown in the captures remained reachable.
The 2048 capture is supplemental evidence for the expanded Timeline/Sources layout;
it does not replace the primary 1920 acceptance.

Native UI was **not verified** at `2560x1440`, `1280x720`, or `3840x2160` in this
run. Browser viewport results must not be reported as native evidence for those
sizes.

## Screenshot manifest

| File | Native client | Evidence | SHA-256 |
| --- | ---: | --- | --- |
| `01-lighting.png` | 1920x1032 | Lighting desk | `43FDA449DE5F4D66F69E1BFC491ED4749FDEEE78DA660286C6F162E48AE3B27D` |
| `02-video-closed.png` | 1920x1032 | Video desk, import controls closed | `BB0ABF31ABE8ACC3B434C9542655123F042525FC68D135DA233BD5C33C66F70E` |
| `03-video-import-open.png` | 1920x1032 | Video desk, import controls open | `B00EA5E8D5DA29A8D090935259326ADD985853DE5084D67810874C71CFDB338A` |
| `04-timeline-normal-sources.png` | 1920x1032 | Timeline normal layout with Sources | `3F8E85B28AA6F0ECB1BC180A4B4C3FEDDA14BEA443B45AD98A4E1E8EB9936D62` |
| `05-timeline-expanded.png` | 1920x1032 | Timeline expanded layout | `432330B329E2E5ACE49B2430760473A4ED3033CE578B2C1433878F0C20D1ECAC` |
| `06-timeline-expanded-tools.png` | 1920x1032 | Timeline expanded tool disclosure | `845C79D4B2D43E63A17A85B3C76078282AB6E32C6AE9691F06849AED782BD196` |
| `07-timeline-restored.png` | 1920x1032 | Timeline restored layout | `213DA0C89F127FF902186F49613371821B22420A1B20E1C224DFA4B3F2559594` |
| `08-timeline-2048x1104.png` | 2048x1104 | Supplemental expanded Timeline/Sources layout | `1A63999597966E58BC3973266A584BD0FC4AC2622981DBFD3B1F37AE5616D7C3` |

## Browser evidence and known unrelated red gate

The integrated Lighting/Video/Timeline geometry and screenshot browser matrix
passed at exactly these four viewports:

- `3840x2160`: PASS
- `2560x1440`: PASS
- `1920x1080`: PASS
- `1280x720`: PASS

These are automated browser/render results, separate from the native client proof
above. `960x640` remains the configured product minimum but is not a substitute
for this four-size acceptance matrix. Historical `860x520` and `1366x768`
checks remain supplemental only and are not counted as current acceptance.
The final browser checker SHA-256 is
`EF569F545CF2FCC0368CF8A576552DF3849B315B5F23D8CFF17320F7AE645E81`;
it separately captures closed/open Video states, verifies restored Timeline
focusability/inert removal, and rejects hidden horizontal topbar scrolling.

`pnpm --dir app run check:edit-live` remains an existing, unrelated red gate. A
2026-08-23 confirmation exited `1` at all five of its own viewports (`1920x1080`,
`1920x1032`, `2048x1152`, `1366x768`, and `1280x720`) on the same two checks:
`editBadgeUsesSceneIdentity` and `sharedHeaderAndContextTabsStayCompact`. Its core
EDIT/LIVE write, Undo/Redo, reachability, and zero-scroll observations continued to
run, but this evidence does not relabel that command as passing and does not make
the failure part of the alpha.7 native screenshot acceptance.
