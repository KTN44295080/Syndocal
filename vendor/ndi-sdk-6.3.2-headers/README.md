# NDI 6.3.2 SDK headers (build-time only)

These are the unmodified C headers from the Windows NDI 6 SDK, version 6.3.2
(the SDK whose `Processing.NDI.Lib.x64.dll` matches the pinned runtime in
`qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json`). Each header states that the MIT
license in its own preamble applies to that file; that notice is retained in
every file. Copyright (C) 2023-2026 Vizrt NDI AB.

They exist so hosted Windows CI can compile the `ndi` feature without
downloading the NDI SDK, whose download requires registration (owner
decision 2026-11-05). `app/scripts/synthesize-ndi-sdk.ps1` verifies
`SHA256SUMS`, generates the import library from the `PROCESSINGNDILIB_API`
declarations in these headers (the proprietary runtime binary is never
inspected), and lays out `Include`, `Lib\x64` and `Bin\x64` for
`grafton-ndi`. The headers are never packaged into a Syndocal artifact.
Local builds may keep using an installed NDI 6 SDK.

NDI® is a registered trademark of Vizrt NDI AB. https://ndi.video/
