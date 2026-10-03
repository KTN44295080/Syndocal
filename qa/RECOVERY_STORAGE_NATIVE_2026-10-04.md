# Native browser recovery preservation

Branch `codex/showclock-review-20260912`, base `d2a44d88`.
Requirement: `MIGRATION-COMPATIBILITY-001`, O4 non-destructive rejection.

The [preceding correction](RECOVERY_STORAGE_PRESERVATION_2026-10-04.md)
established production-module behavior against Map storage and rebuilt the
ordinary Windows application. This checkpoint adds a maintained native probe
for actual isolated WebView2 localStorage and the production App startup.
No product source, public operation or recovery schema changes here.

## Native failure and current result

The [pre-repair native run](artifacts/recovery-native-before-2026-10-04.json)
used the previously built QA executable, SHA-256
`fa8386a50f1038a6e80ddaa62ca37c2fefa9a726881fd28906cfad31d611824b`.
It lost `{broken-json` across the native restart and reported `Ready`.
The negative assertion was retained, and the original absent recovery key was
restored and verified after another restart. Normal application identity stayed
unchanged. This artifact was compiled before the recovery correction; source
fingerprints emitted by that run describe files present during observation,
not the source compiled into that old executable.

The [current native run](artifacts/recovery-native-final-2026-10-04.json)
passes 15 groups: seven invalid-input cases, exact original-key restoration
after restart, and seven maintained authentication/crash/graceful lifecycle
groups. The seven cases are malformed JSON, empty bytes, stored null, future
v4, unsupported v2, an invalid v3 checkpoint, and a negative v3 tombstone
serial. Each uses a new owned native process, preserves exact key bytes,
reports the fixed actionable error in the native footer DOM text, and retains
the complete empty native checkpoint, checkpoint hash and recovery journal
serial. Lighting/video gates remain closed and transport remains stopped.
Unsupported formats also require the compatible-version guidance. The private
checkpoint signature must never appear in the error.

The new `--recovery-storage` lane uses the existing process-verified backend
session, exact QA executable/profile and loopback debugger ownership checks.
It changes only its isolated profile's recovery key, exercises the production
startup after real native restarts, then restores the original bytes in a
`finally` block. It does not replace callbacks, IPC responses or storage APIs.
No DOM action, Computer Use, subagent, physical output or ordinary-profile
storage mutation was used. All 18 other optional flags reject before process
or profile access in the [CLI contract](artifacts/recovery-native-cli-2026-10-04.json).

## Build, regression and ownership evidence

- [Optimized QA build](artifacts/recovery-native-qa-build-2026-10-04.txt):
  maintained `pnpm --dir app tauri build --no-bundle --config
  src-tauri/tauri.mcp-lifecycle.conf.json`, absolute isolated Cargo target,
  opt-in existing Tap receiver and pinned PATH-first MSVC 14.44.35207.
  Rust completed in 3m; frontend in 9.76s. QA executable SHA-256:
  `180f103cdc44a7b0cf09ba1f64237567f917337cdcf9cb967832d177aa84ad1d`.
- QA Rust warnings/TypeScript diagnostics baseline/current/delta 0/0/0;
  existing Vite size advisory 1/1/0. No allowance/size limit changed.
  Node syntax/CLI checks have no compiler-warning measurement.
- [Separate current Tap regression](artifacts/recovery-native-tap-2026-10-04.json):
  eight groups pass on the same executable. The real registered App callback
  changes engine BPM from 120 to 80.15444, header to 80, footer to `Tapped BPM
  80.2`; header height remains 42px. A long pause resets history and preserves
  the tempo on the first following tap. This is backend callback proof, not a
  physical click. These eight groups are not added to the recovery count.
- [Source freeze](artifacts/recovery-native-source-freeze-2026-10-04.json)
  remained identical through the build and native runs. All five protected
  fingerprints remain unchanged; App.tsx was read and compiled, not edited.
- The ordinary executable previously built at `d2a44d88`, SHA-256
  `f3b8a847e83ed73c816d68cfdc94198784068548545fff4ed47e3a96f26b1172`,
  remains at owned PID 49536 with a responsive `Syndocal` window. Both native
  probes preserve its descriptor bytes. This harness-only checkpoint reuses
  that ordinary build; it does not claim a new ordinary build or primary
  debugger/authenticated mutation.
- Current harness syntax, owned diff/reference and both ledger checks pass.
  Native probes report no worker panic and clean up their process/credentials.

The footer observation reads production DOM text, not pixels, complete-message
visibility, keyboard accessibility or a recovery menu action. Successful
recovery/acknowledgement/discard, automatic-save/write/quota faults in actual
WebView2, verified older-generation fallback/reporting, complete O1-O4
backup/template/upgrade/restart matrices, nested parser fuzz, physical devices,
independent review and release acceptance remain open. Other-owner dirty
frontend work is preserved and included in the QA build. Q4 gains bounded
native evidence without changing requirement/risk states or the 27 Complete /
23 Open / eight Deferred completion counts.
