# Native workspace/operator revalidation — 2026-09-09

- Branch: `main`
- Base before this checkpoint: `f49162f22e4c4396b199854a9fe4f47fa304126b`
- Scope: the existing Windows native workspace acceptance checker only. No
  product runtime, pane lifecycle, output route, ASIO/NDI path, or fixture was
  changed.

## Checker repair

The first run on this PC reported every pane at approximately half the
requested coordinates and size. The native process had created the windows,
but the PowerShell checker was observing Win32 geometry through DPI
virtualization at this machine's 200% display scale.

The checker now enables Per-Monitor DPI awareness before enumerating windows.
Placement coordinates and minimum outer geometry remain native-window checks;
the requested pane dimensions are validated through `GetClientRect`, because
Tauri's placement width/height are inner/client dimensions and the outer
title-bar frame is DPI-scaled. No assertion was removed or made permissive.

## Evidence

Command:

```text
pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File app/scripts/check-native-workspace-operator.ps1 -StartupTimeoutSeconds 420 -EvidenceDir target/qa/native-workspace-acceptance-20260909-06
```

Result: **PASS, 7/7 native WebView2 pane windows**.

- One isolated Syndocal process created the main window and all seven named
  pane windows.
- Every pane matched its requested placement within the existing 16 px
  tolerance: Stage `(48,64)`, Timeline `(90,98)`, Programmer `(132,132)`,
  Setup `(174,166)`, Live `(216,200)`, Mixer `(258,234)`, Touch `(300,268)`.
- Each pane reported outer `886x591` and client `860x520`; the latter is the
  requested inner size and remains inside the existing `840-900 x 500-580`
  acceptance range.
- The evidence JSON is
  `target/qa/native-workspace-acceptance-20260909-06/native-workspace-acceptance.json`.
- The package's official Windows entry was also run unchanged:
  `pnpm.cmd --dir app run check:workspace-operator-native` — exit 0, PASS
  `7/7`; its report was written to the unique temp run
  `syndocal-workspace-acceptance-20260908-175941`.
- The acceptance script's `finally` cleanup completed. No Syndocal/cargo/rustc
  process or listener on port 5191 remained after the run.
- The isolated config has bundling disabled and no physical output operation
  was issued. This is native window evidence only, not hardware, venue,
  release, signing, or physical-output acceptance.

## Remaining boundaries

This checkpoint does not close the protected workspace-operator viewport
checker, whose CDP navigation timeout remains a separate fixture/beforeunload
boundary, and does not claim the unfinished real-file recovery, GUI
cancellation latency, full-show snapshot benchmark, Mac/device/hardware,
physical output, signing, publication, or venue gates.
