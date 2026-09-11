# Native thumbnail missing-file recovery revalidation — 2026-09-09

## Purpose

This is a current-source revalidation of the already recorded thumbnail
failure-recovery acceptance. The earlier raw acceptance used a different
checkout and executable, while the current main branch subsequently changed
the native thumbnail dispatch, cancellation, controller, and snapshot paths.
This run therefore does not treat the older evidence as proof for the current
artifact and does not change the product contract.

## Source and artifact

- Checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`
- Source HEAD: `1c30650da9ac0f356fbdbc41b07fc1f3ea9d5e26`
- Executable: `target/release/syndocal.exe`
- Product/file version: `1.2.0-alpha.69`
- Bytes: `64,699,904`
- SHA-256: `C977A1EBFBB78BEB2D3686A652D1F5CF348DB3C20C9B9D591DF0E9A9950B0197`
- Raw evidence: `target/qa/native-thumbnail-failure-recovery-20260909-01/`

The test-owned fixtures were isolated under the raw evidence directory:

- `quadrants.png`: 253 bytes,
  `1561115A3B4A825BDC64C3FC2633D4736C1EDCD8CAB7EBF1FE9AC6313FA19460`
- `two-patterns.mp4`: 1,344 bytes,
  `56FCBB1725220525F53E87CCCAF6EBA670C4888C35AEEF05C381E41193648D69`

## Native acceptance result

`probe-result-failure-recovery.json` and
`launch-result-failure-recovery.json` both report `status: pass`.

The current executable was launched as the only owned Syndocal window,
verified responsive and maximized, with output ownership in `Standby`, both
lighting/video permissions denied, and no video outputs. The probe then:

1. Imported the real test-owned PNG and MP4 through the Media Library UI.
2. Verified both 160x90 thumbnails and their expected quadrant pixels.
3. Moved only the test-owned PNG to `materials\held\` with a bounded rename,
   verified the source was absent and the moved SHA was unchanged, reloaded,
   and observed the missing-file failure with `Retry Thumbnails` enabled.
4. Verified the healthy MP4 remained displayed with the same thumbnail URL and
   pixels while the PNG was missing.
5. Restored the exact PNG, clicked `Retry Thumbnails`, verified the PNG
   recovered with the original pixels, and verified the retry reread only the
   missing asset.
6. Verified zero page errors, zero physical-output enable commands, zero raw
   project-mutation commands, exact fixture SHA restoration, application exit,
   and zero remaining debug listeners.

This proves the real native missing-file → Retry → recovery path for the
current source/EXE. It does not prove physical output, release publication,
signing/notarization, or whole-product completion.

## Current-main artifact revalidation — 2026-09-11

The already accepted PNG missing-file recovery was revalidated against the
current `main` source and a fresh identifier-only QA build. This is an
artifact/source revalidation, not a second acceptance claim for the older
run, and it did not change product source or the thumbnail contract.

- Source HEAD: `746f8bba30820ae9dd202764b14d883d736ada7b`
- Evidence: `target/qa/native-thumbnail-failure-recovery-20260911-02/`
- QA build config:
  `target/qa/native-thumbnail-failure-recovery-20260911-01-tauri.conf.json`
- QA executable: `target/release/syndocal.exe` during the run
- QA executable bytes: `64,568,320`
- QA executable SHA-256:
  `61D1C10115D751E17197191EE2D0DF8DABEF9C73D4F4C660C59CE2428D80F5B0`

The maintained Windows wrapper selected and verified the MSVC
`14.44.35207` x64 linker, and `tsc --noEmit` plus Vite completed during the
build. The real WebView probe imported fresh test-owned PNG/MP4 copies,
verified both 160x90 images, moved only the PNG copy to the held directory,
reloaded the same native process, observed the missing-file error and enabled
`Retry Thumbnails`, restored the exact PNG bytes, and used the real Retry UI.

The restored PNG pixels matched the original red/green/blue/white opaque
quadrants. The healthy MP4 URL and pixels remained unchanged, and the Retry
added exactly one asset-thumbnail request for the missing asset. The run
recorded 11 thumbnail requests total, zero page errors, zero raw project
mutation commands, zero physical-output enable commands, one responsive
maximized window, owned-process exit, and zero remaining debug listeners.
The normal profile JSON digest was unchanged; both test fixture hashes matched
their manifests after cleanup.

Independent review of this run passed `node --check` for the bridge/probe,
PowerShell launcher parsing, and a raw JSON assertion over the fault,
retry-count, pixel, output, window, and cleanup fields. The thumbnail
production modules had no diff between the previous revalidation source
`1c30650da9ac0f356fbdbc41b07fc1f3ea9d5e26` and this source. The only change in
the filtered video source was an existing test-only direct-child cancellation
fixture adjustment; it does not alter thumbnail behavior.

The ordinary non-QA executable was restored and rechecked after the isolated
run: `target/release/syndocal.exe`, 64,568,320 bytes, SHA-256
`CBC9CE9502D1173CB85A3BC28D75D0A37FD6FAA623473285F4ACBC20E5040B6E`.

This revalidation covers the PNG missing-file → Retry → recovery path on the
current main artifact. An independent MP4 missing-file move remains unclaimed
because the prior bounded move helper encountered Windows `EBUSY` after MP4
decoding; no forceful rename or byte alteration is allowed. GUI cancellation,
physical output, Mac validation, signing/notarization, release publication,
and whole-product completion remain separate boundaries.
