# Media-derived software and native boundary revalidation — 2026-09-12

This checkpoint records the current software evidence for `COV-MEDIA-DERIVED-001`
and the already completed native WebView probes. It does not promote a partial
thumbnail result into full media-derived acceptance.

## Source and artifact identity

- Current source/remote base: `d33dfbca55f7badbaec3d327cc622cca640f80e7` (`main`)
- `origin/main` matched before this checkpoint.
- The current product source under `app/src`, `app/src-tauri`, `crates`, and
  Cargo manifests is unchanged from the source used for the native probes;
  later commits only changed checkers and QA records.
- Exact EXE: `target/release/syndocal.exe`, version `1.2.0-alpha.69`,
  64,569,856 bytes, SHA-256
  `71E861A8A716F36D09178DCF3694B86A835EA3BAF83787E86369ED0A06AF5819`.

## Current software checks

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS — bounded lanes, retry/terminal/stale behavior, cache reuse, visible retry eligibility, disposal/reset cancellation, and view bindings |
| `pnpm.cmd --dir app run check:media-asset-operations` | PASS — phase, exact cancellation, idempotent release, owner cleanup |
| `pnpm.cmd --dir app run check:media-asset-authority` | PASS — phase order, reply loss/query, cancellation CAS, authority continuity, operator classification, paired application, empty-catalog normalization |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS — normal success, exact abort, stale-result rejection, cancellation failure, malformed/foreign ticket rejection |
| `pnpm.cmd --dir app run check:bundled-library` | PASS — bundled fixture failure/retry |
| `pnpm.cmd --dir app run check:vj-first-run` | PASS — safe first-run VJ workflow and independent Preview transport |
| `node app/scripts/check-vj-media-import-access.mjs` | PASS — guarded empty states, populated normal/mixer exclusivity, accessible names, scoped overlay CSS, and full bank/page reachability; source/policy only |

## Native evidence carried forward

The authorized real-file probe was already run twice and is not repeated here.
Both runs used the exact EXE hash above, one responsive maximized Syndocal
window, Standby ownership, denied lighting/video domains, zero physical-output
enable commands, and clean exact-process/listener cleanup.

- PNG: test-owned `quadrants.png` was moved to `held`, the real missing-file
  error and enabled `Retry Thumbnails` state appeared, the file was restored
  byte-for-byte, and Retry recovered the 160x90 four-quadrant image. This is
  accepted current-main real-WebView evidence.
- MP4: moving the test-owned `two-patterns.mp4` returned Windows `EBUSY` while
  the native video handle was held, before a missing-file state. The same
  boundary reproduced under an ASCII LocalAppData staging path. It is not a
  successful MP4 missing→Retry→recovery result and remains open.
- Evidence paths: `qa/THUMBNAIL_REAL_FILE_PROOF_2026-09-08.md` and
  `qa/THUMBNAIL_CANCELLATION_LIFECYCLE_REVALIDATION_2026-09-09.md`.

## Remaining boundary

The row remains `In progress`. The checks and PNG probe do not establish
proxy, waveform, analysis, general cache eviction, cold/warm performance
budgets, full MP4 missing-file recovery, hard-stop decoder/OS-I/O/GPU
cancellation, physical output, macOS/Linux, signing, publication, or
product-wide completion. No product source was changed in this checkpoint.
