# Native thumbnail cancellation revalidation on current main — 2026-09-12

## Scope

This checkpoint validates the existing native thumbnail cancellation contract against the current `main` executable. It is not the real-file-missing/retry/recovery experiment and does not repeat that experiment.

## Identity and setup

- Source branch: `main`
- Source HEAD: `8812a7ff0e1d2e679bb66f4eb61de6302919498b`
- Executable: `target/release/syndocal.exe`
- Product version: `1.2.0-alpha.69`
- Executable size: `64,569,856` bytes
- Executable SHA-256: `144CD12F9B247575DE4E40FC966BEA703D74206B159C736AFA5A6FE66957C9E3`
- Probe run directory: `target/qa/native-thumbnail-cancel-20260912-02/`
- Probe listener: loopback port `51542`, verified as owned by the exact executable process
- Probe input: existing test-only `long-av-sync-30m.mp4`; no normal/user project or physical output was used

The existing cancellation probe was copied to the new run directory without changing product code. Only its expected executable hash was updated to the current rebuilt EXE. The probe uses the real WebView Tauri `Channel` and `invoke` path; it does not mock native replies.

## Result

The probe exited with code `0` and wrote `native-thumbnail-cancel.json` with `status: pass`:

- exactly one `Syndocal` window, responsive and maximized;
- startup ownership `Standby`, with `lighting_allowed=false` and `video_allowed=false`;
- `get_snapshot` returned `fixtures`, `cues`, and `video.outputs` arrays;
- an unknown route was rejected;
- layer thumbnail cancellation: acknowledged ticket and cancelled result;
- asset thumbnail cancellation: acknowledged ticket and cancelled result;
- `physicalOutputOperations=0`;
- exact application process exited and loopback listener count was `0` after cleanup.

## Nonclaims

This is current-source native cancellation evidence only. It does not claim hard-stop cancellation of synchronous decoder/OS/GPU work, missing-file recovery, MP4 EBUSY recovery, cache-cold performance, device/hardware acceptance, venue soak, macOS/Linux, signing, publication, or product completion.
