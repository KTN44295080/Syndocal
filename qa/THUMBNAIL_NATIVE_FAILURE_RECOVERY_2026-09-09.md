# Native thumbnail missing-file failure recovery — 2026-09-09

## Scope

This checkpoint covers the current `main` Windows native Media Library path for
a real local-file failure: a test-owned PNG was temporarily moved out of its
path, the native thumbnail read was allowed to fail, and the restored file was
recovered with the existing `Retry Thumbnails` action. It does not add or
reimplement product code, and it does not claim overall product, hardware,
venue, Mac, release, signing, or notarization completion.

- Branch: `main`
- Source HEAD: `905b2e31be2878d950de64e0dfcab1bee0ae7250`
- Evidence: `target/qa/native-thumbnail-failure-recovery-20260909-03/`
- Product code changed for this checkpoint: none
- Physical output commands: none; the isolated profile remained `Standby`,
  with lighting/video denied and zero video outputs.

## Reviewed inputs

The current `main` source was rebuilt with the maintained native wrapper and an
identifier-only QA config. No tracked Tauri configuration was changed.

| Item | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/syndocal.exe` | 64,541,696 | `580C27D683D5068F7DF1C80170D48F00DB0033C47F30BE423687CA36520DE506` |
| `materials/quadrants.png` | 253 | `1561115A3B4A825BDC64C3FC2633D4736C1EDCD8CAB7EBF1FE9AC6313FA19460` |
| `materials/two-patterns.mp4` | 1,344 | `56FCBB1725220525F53E87CCCAF6EBA670C4888C35AEEF05C381E41193648D69` |

The build command was:

```text
pnpm --dir app tauri build --no-bundle --config target/qa/native-thumbnail-failure-recovery-20260909-03-tauri.conf.json
```

It exited 0 after the wrapper selected and verified the pinned MSVC
14.44.35207 linker. The resulting executable is a QA-only artifact for this
evidence run; rebuild without the override before ordinary artifact use.

## Native acceptance result

`launch-failure-recovery.ps1` launched the exact hash-pinned QA executable and
verified one responsive, maximized `Syndocal` window. The probe used the
dedicated identifier
`jp.seraf.ktn.syndocal.qa.thumbnail.9145e649-b56e-4b28-8241-06e77d7bc1ef`,
so its backend and WebView state were isolated from the normal profile.

1. Both real test thumbnails first rendered at `160x90`; the PNG quadrants
   were verified as red, green, blue, white with opaque alpha. The MP4 frame
   was also verified against the same quadrant contract.
2. The test-owned `quadrants.png` was moved to `materials/held/` with its
   preflight hash, then the WebView was reloaded. Thumbnail IPC counts did not
   change before explicit authorization and no loaded thumbnail image was
   present.
3. After explicit Media Library authorization, the missing PNG remained
   without a loaded image, the healthy MP4 displayed, `Retry Thumbnails` was
   enabled, and `aria-busy` was `false`. No page errors were observed.
4. The exact PNG bytes were restored and the existing `Retry Thumbnails`
   button was clicked. The PNG recovered with the expected pixels. The
   healthy MP4 data URL stayed unchanged, and the retry added one thumbnail
   request for the missing asset.
5. Final PNG/MP4 hashes matched the manifest, `materials/held/` was empty, the
   normal profile JSON digest was unchanged, the owned application exited, and
   the CDP listener count returned to zero.

The machine-level result is recorded as `pass` in
`probe-result-failure-recovery.json` and `launch-result-failure-recovery.json`.
The report recorded `physicalOutputEnableCommandsIssued: 0`,
`rawProjectMutationCommandsIssued: 0`, `faultInjectionPerformed: true`, and
`retryThroughMissingFileTested: true`.

## Evidence boundary

An earlier same-day attempt under
`target/qa/native-thumbnail-failure-recovery-20260909-02/` used the normal
application identifier. Although its PNG probe output was successful, it did
not satisfy the required isolated-backend condition and is explicitly excluded
from this acceptance record; it was not used to establish success.

The independent MP4 missing-file move remains unclaimed. The prior bounded
helper encountered Windows `EBUSY`/resource-busy behavior after MP4 decoding;
no forceful rename or byte alteration is permitted. Normal MP4 thumbnail
rendering and healthy-cache preservation are covered above, but MP4
missing-file failure → Retry recovery is not claimed.

GUI cancellation, Clip Grid-wide acceptance, physical output, Mac validation,
signing/notarization, and release publication remain separate boundaries.
