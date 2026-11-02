# Syndocal User Guide — NDI® Send & Receive

## Overview

In the ordinary Windows package (`Syndocal`), **NDI®** video receive (live layer source) and **NDI®** video send (composition output) are enabled out of the box with the official NDI® 6 runtime (`Processing.NDI.Lib.x64.dll`) bundled next to `syndocal.exe`.

> **Trademark Notice:** NDI® is a registered trademark of Vizrt NDI AB.
> - Official NDI® website: <https://ndi.video/>
> - Free NDI® Tools download: <https://ndi.video/tools/>

---

## Runtime Resolution Order (`crates/io/src/ndi_runtime.rs`)

When initializing NDI® discovery, reception, or transmission on Windows, Syndocal resolves `Processing.NDI.Lib.x64.dll` in strict priority order:

1. **Priority 1 — Bundled executable directory:** `<exe_dir>\Processing.NDI.Lib.x64.dll` (`std::env::current_exe()?.parent()`). In the ordinary Windows installer, this file is always present next to `syndocal.exe`, so no separate NDI Runtime installation is required.
2. **Priority 2 — Environment variables:** `NDI_RUNTIME_DIR_V6`, `NDI_RUNTIME_DIR_V5`, `NDI_RUNTIME_DIR_V4`, `NDI_RUNTIME_DIR_V3`, `NDI_RUNTIME_DIR_V2`, and `NDI_SDK_DIR\Bin\x64`.
3. **Priority 3 — Standard Windows install directories:** `C:\Program Files\NDI\NDI 6 Runtime\v6` and `C:\Program Files\NDI\NDI 6 SDK\Bin\x64`.

---

## Receiving NDI® Video into a Layer

1. Open **Control** (or **Setup → Video**) and select a video layer.
2. Choose **NDI** as the layer source type.
3. Click **Refresh NDI sources** to enumerate active NDI® senders on the local network or machine.
4. Select the desired exact NDI® endpoint from the **NDI source** dropdown and add/apply the source.
5. The receiver connects only to the selected exact endpoint identity and automatically reconnects if the sender restarts.

---

## Sending Composition Video over NDI®

1. Open **Setup → Video → New Output** (or **Video Output Config** for an existing output).
2. Under **Advanced output settings**, set **Kind** to **NDI Sender** and enter the **Endpoint** name (for example, `Syndocal Program`).
3. Select the target **Composition** and click **Add configured output** (or **Apply Output**).
4. When live video output is armed, Syndocal publishes BGRA frames over NDI® under the configured endpoint name.

---

## Verifying Real NDI® Send/Receive with Free NDI® Tools

Operators can verify live NDI® send and receive on the local machine using the free **NDI® Tools** suite (<https://ndi.video/tools/>):

1. **Receive test (`NDI Test Patterns` → Syndocal):**
   - Launch **NDI Test Patterns** (or **NDI Screen Capture**) from NDI® Tools.
   - In Syndocal's NDI source picker, click **Refresh NDI sources**, select the test pattern source, and confirm live frames appear in the layer preview.
2. **Send test (Syndocal → `NDI Studio Monitor`):**
   - Configure an **NDI Sender** output in Syndocal and enable output.
   - Launch **NDI Studio Monitor** from NDI® Tools, select the Syndocal NDI sender from the menu, and confirm live composition output is displayed.

---

## Package Separation & Licensing References

- **Ordinary Windows Package (MIT):** Bundles `Processing.NDI.Lib.x64.dll` and `licenses/Processing.NDI.Lib.Licenses.txt`.
- **GPL-ASIO (`show-asio`) Windows Package:** Built with `--no-default-features --features libav,spout,show-asio`. NDI® (`ndi` Cargo feature and `Processing.NDI.Lib.x64.dll`) is strictly excluded from the `show-asio` package to avoid combining proprietary NDI® DLL redistribution with a GPLv3 binary.
- **Official NDI® SDK Licensing Documentation:**
  - <https://docs.ndi.video/all/developing-with-ndi/sdk/licensing>
  - <https://docs.ndi.video/all/developing-with-ndi/sdk/installing-the-sdk/windows-installation>
  - `http://ndi.link/ndisdk_license`
  - `http://ndi.link/NDIRedistV6`
  - Full third-party notices: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [licenses/Processing.NDI.Lib.Licenses.txt](licenses/Processing.NDI.Lib.Licenses.txt)
