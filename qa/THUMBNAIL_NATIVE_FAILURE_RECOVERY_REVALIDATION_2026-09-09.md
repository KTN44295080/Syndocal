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
