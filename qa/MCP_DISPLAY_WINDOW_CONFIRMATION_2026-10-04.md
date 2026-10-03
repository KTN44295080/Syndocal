# External MCP editor-target Display window confirmation

Branch `codex/showclock-review-20260912`, base `59139657`.

## Confirmed failure and change

The authenticated native executor disabled the controller's generic R4 dialog,
but Display `set_open` called a second, unconditional editor-monitor warning
inside the physical window core. An external request opening on the editor's
monitor therefore still waited for human input. This contradicted the user's
authorization for all external MCP R4/R5 operations without individual approval.

The [old QA executable](artifacts/display-window-before-2026-10-04.json), SHA-256
`23885b3ea23aee245e26429b5309e3501fabe02d2b116b07c87ed3a2f02faf97`,
passed exact missing-grant and stale-fence rejections, then retained the granted
open request as `pending` after the maintained 15-second status bound. No dialog
was accepted or dismissed by automation. Its own native process and credential
were cleaned up, the ordinary application identity was unchanged, and no native
panic was recorded. This is negative evidence, not native acceptance. Source
fingerprints in this historical artifact describe the candidate working tree
at observation time, not that old executable's compiled source.

The local command and immutable authenticated native executor now select a
native-only `OutputConfirmationOrigin`. The canonical controller passes it
through its internal Display request to the deeper warning seam. Local desktop
opens over the editor still require Yes; No/close retains cancellation. External
execution never invokes that OS dialog. The origin is not serialized, added to
a Tauri request, or supplied by the renderer. Existing authentication, exact
grants, single-use native execution, owner/incarnation, project/output fences,
stable monitor identity, lease/durable authorization and terminal receipts remain
on the same canonical path. The pre/post-confirmation authority rechecks remain.
No public schema, inventory, compatibility or product version changes.

The other native message dialog in `main.rs` belongs to local diagnostic export.
External R5 diagnostics uses the separate authenticated native executor's direct
capture/new-target publication path. It does not enter that local GUI dialog.

## Probe correction and failed trial

The first rebuilt [native trial](artifacts/display-window-disabled-trial-2026-10-04.json)
finished without confirmation but returned `publication_failed`: its private
Display was authored as disabled. Production
`validate_native_display_output_route` explicitly rejects a disabled Display
before rendering. The trial was not accepted as successful shell evidence.
The old negative run used that same disabled fixture; its confirmation occurred
before this downstream disabled-output rejection.

The final fixture uses an enabled, small decorated Display bound to the exact
current editor monitor, with an empty composition and no video layers/media,
fixtures or audio. Its one DMX route is explicitly disabled loopback metadata.
The ordinary reference validator requires the composition's reverse output ID;
the fixture supplies it. Earlier fixture setup failures remained temporary and
did not establish product regressions. Assertions still require a successful
typed receipt, real nonzero HWND, exact target/fence rejection, complete authored
preservation and owned retirement. No rejection was reclassified as success,
pending mutation replayed, renderer/invoke patched or acceptance timeout widened.

## Current native evidence

The maintained `--display-window` mode is exclusive with all 17 other optional
lanes; [CLI proof](artifacts/display-window-cli-2026-10-04.json) rejects overlap
before a process/profile is selected. It uses real native IPC for project setup
and separate authenticated stdio MCP for the canonical output operations. No DOM
operation, Computer Use, subagent, physical click or confirmation automation.

The [accepted Display run](artifacts/display-window-native-2026-10-04.json)
contains five specific groups plus seven maintained process/credential lifecycle
groups, all 12 passed:

- The exact current editor monitor is backend-resolved. Missing exact output
  grant and a fence captured before Arm reject without a shell.
- Granted open completes through the real native window/GPU construction path
  without additional human input. `video-output-1` has real HWND `27985380`.
- Same-request receipt replay retains the same HWND. Close/reopen completes;
  the reopened HWND is `27266932`, and final close reports no live handle.
- The complete authored checkpoint and private source bytes remain unchanged
  across Arm/open/replay/close/reopen. New replaces only the private QA project
  with empty state and denies both output domains; no Display remains.
- All maintained restart/dead-descriptor/retired-principal/credential checks
  pass; owned processes and credentials are removed, the ordinary identity
  stays unchanged, and no native panic is observed.

QA executable SHA-256:
`fa8386a50f1038a6e80ddaa62ca37c2fefa9a726881fd28906cfad31d611824b`.
Six source/harness fingerprints in the Display proof match the current files.
The native gate uses ordinary optimized Engine settings.

On that same executable, [R4/R5 and execution-time revocation](artifacts/display-window-high-risk-2026-10-04.json)
passes all 14 maintained groups: exact grant/promotion/Safe Mode gates, direct
R5 ZIP bytes/hash/no-overwrite/replay, R4 lease lifecycle and native execution
after principal revocation remain enforced. This is a bounded slice, not every
R4/R5 action or complete security acceptance.

The [separate Tap run](artifacts/display-window-tap-2026-10-04.json) passes all
eight groups. The real App callback changes engine BPM from 120 to `81.08198`;
the header reads `81`, the footer reads `Tapped BPM 81.1`, and the one-row header
remains 42px. The first tap after a long pause starts a fresh interval history
and retains the current BPM. This exercises the actual callback and native
publication/read path; it does not prove a physical mouse click. Separate probe
groups are not added to the Display assertion count.

## Validation and ordinary application

- [Pinned MSVC unit run](artifacts/display-window-tests-2026-10-04.txt):
  `control_plane_runtime::tests`, 37 passed, zero failed/ignored, 1937 filtered.
  The two new tests cover local Yes/No and zero external dialog calls, plus
  rejection of renderer-supplied origin/bypass fields. This unit configuration
  uses the recorded test-only Engine optimization overrides.
- Existing `check-output-control-runtime`, `check-video-output-window-runtime`
  and `check-agent-bridge` passed; the bridge selected all 15 groups.
- [Optimized QA build](artifacts/display-window-qa-build-2026-10-04.txt):
  `pnpm --dir app tauri build --no-bundle --config src-tauri/tauri.mcp-lifecycle.conf.json`.
- [Ordinary optimized build](artifacts/display-window-normal-build-2026-10-04.txt):
  `pnpm --dir app tauri build --no-bundle`. The maintained wrapper verified and
  stopped only the exact checkout executable. One pre-build environment attempt
  left `CARGO_TARGET_DIR` empty and failed metadata before compilation; removing
  the environment entry, rather than assigning null, corrected that setup.
  The successful ordinary build used neither isolated target nor QA receiver.
  [Passive native gate](artifacts/display-window-normal-window-2026-10-04.json)
  verifies owned PID 66772, exactly one visible responsive maximized `Syndocal`
  window and unauthenticated broker rejection. No primary debugger or
  authenticated primary mutation was used.
- Current selected Rust unit warnings: zero. QA Rust warnings/TypeScript
  diagnostics baseline/current/delta are 0/0/0 in both successful QA/ordinary
  build configurations; the existing Vite size advisory
  is 1/1/0. No warning allowance or chunk limit changed.
- Owned diff/reference/ledger/protected-file/staged review is recorded at this
  checkpoint. All five protected fingerprints remain unchanged. Q4 adds this
  bounded proof; all requirement/decision/risk states and prior evidence objects
  remain unchanged. The completion marker counts remain 27 Complete, 23 Open,
  eight Deferred; no unobserved acceptance gate is promoted.

This proves the native editor-target confirmation boundary and a small empty
window lifecycle. It does not establish content pixels/color, fullscreen,
topology/hotplug, AddDisplay, every output operation, whole Control UX, frame or
audio budgets, full AI0-AI8/bypass/security matrices, independent review, physical
devices, venue, signed updater/distribution or release acceptance. User-requested
no-subagent operation uses direct stable-diff self-review. Other-owner dirty
frontend work is preserved and included in these local builds; they are not a
frozen clean release. The broader completion ledger remains open.
