# Native thumbnail missing-file failure recovery — 2026-09-08

## Scope

This checkpoint covers the Windows Media Library path for a real local-file
failure: a test-owned PNG was temporarily moved out of its path, the native
thumbnail read was allowed to fail, and the restored file was recovered with
the existing `Retry Thumbnails` action. It does not add or reimplement product
code, and it does not claim overall product, hardware, venue, Mac, release,
signing, or notarization completion.

- Branch: `chatgpt/thumbnail-native-reload-20260908`
- Candidate/base HEAD: `233b82af2edce870dfa2afae71ce49becb9b5cd1`
- Evidence: `target/qa/thumbnail-native-failure-recovery-20260908-05/`
- Product code changed for this checkpoint: none
- Physical output commands: none; the isolated profile remained `Standby`, with
  lighting/video denied and zero video outputs.

## Reviewed inputs

The launched QA executable was the hash-pinned artifact already reviewed for
the native thumbnail UI/reload checkpoint:

| Item | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/syndocal.exe` | 64,532,992 | `A7A8CCAECB8721EEFC00CD4107080023CDD8C78765E64F2DFF84A5005A8768BF` |
| `materials/quadrants.png` | 253 | `1561115A3B4A825BDC64C3FC2633D4736C1EDCD8CAB7EBF1FE9AC6313FA19460` |
| `materials/two-patterns.mp4` | 1,344 | `56FCBB1725220525F53E87CCCAF6EBA670C4888C35AEEF05C381E41193648D69` |

The PNG and MP4 were fresh copies owned by this evidence directory. Only the
PNG copy was moved; no operator media, normal application data, or source
assets were moved, deleted, corrupted, or overwritten.

## Native acceptance result

`launch-failure-recovery.ps1` launched the exact hash-pinned executable and
verified one responsive, maximized `Syndocal` window. The same native process
was used for the WebView reload in the probe.

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
   button was clicked. The PNG recovered with the expected pixels. The healthy
   MP4 data URL stayed unchanged, and the retry added exactly one
   `get_media_asset_thumbnail` request for the missing asset.
5. Final PNG/MP4 hashes matched the manifest, `materials/held/` was empty, the
   normal profile JSON digest was unchanged, the owned application exited, and
   the CDP listener count returned to zero.

The machine-level result is recorded as `pass` in
`probe-result-failure-recovery.json` and `launch-result-failure-recovery.json`.

## Boundary still open

An independent MP4 missing-file move was attempted in fresh areas
`thumbnail-native-failure-recovery-20260908-02` through `-04`. After the app
had decoded the MP4 layer thumbnail, Windows returned `EBUSY`/resource busy for
the bounded, non-destructive move helper. The helper did not force a rename or
alter the bytes, and the MP4 was restored with its original hash. Therefore
MP4 missing-file failure -> Retry recovery is not claimed by this checkpoint;
normal MP4 thumbnail rendering is covered by the passed evidence above.

Remaining out-of-scope gates include GUI cancellation, Clip Grid-wide
acceptance, physical output, Mac validation, signing/notarization, and release
publication.
