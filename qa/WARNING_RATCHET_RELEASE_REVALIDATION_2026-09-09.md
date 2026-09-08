# Windows release warning-ratchet revalidation — 2026-09-09

## Scope

This checkpoint revalidates the current Windows release warning gate and its
release self-tests on `main`. It records evidence only; no product source,
warning assertion, or release policy was changed.

- Source before this checkpoint: `b215c5efada88107acadb11653f897eafb3a28a0`
- Remote comparison before the checks: `origin/main` matched
- MSVC procedure: `vcvars64.bat -vcvars_ver=14.44`; the exact
  Build Tools `14.44.35207` x64 linker was pinned and returned first by
  `where.exe link.exe`
- Native and hardware scope: no physical output, device, or external client
  was started

## Warning gate

Command:

```text
pnpm.cmd --dir app exec node scripts/check-warning-ratchet.mjs --configuration windows-default-release
```

Result: **PASS**

| Measure | Result |
| --- | ---: |
| Artifact coverage | 12 / 12 |
| Baseline warnings | 83 first-party |
| Current warnings | 0 first-party |
| Identity removals | 67 |

The gate completed with `warning ratchet ok`. No warning was hidden with an
allowlist or compiler option.

## Release self-tests

`pnpm.cmd --dir app run check:release:self-test` also passed:

- release metadata: 128 assertion groups;
- ASIO packaging boundary: 169 assertions;
- video output routing R4: pass;
- Windows candidate extractor: 43 assertions;
- verified materialization: 4 assertions;
- Windows release artifact: 140 assertions;
- strict JSON duplicate-key protection: 130 assertions.

The ASIO check correctly remained SDK-independent because `FFMPEG_DIR` was not
set. No ASIO artifact was created or treated as distributable.

## Boundary

This closes only the current Windows release warning/self-test checkpoint. It
does not claim the full completion ledger, real-file-missing → UI Retry →
restore → recovery, physical ASIO/NDI/DMX/video/audio acceptance, Mac/Linux
real-device acceptance, signing, publication, or venue acceptance. Those
boundaries remain explicitly unclaimed.
