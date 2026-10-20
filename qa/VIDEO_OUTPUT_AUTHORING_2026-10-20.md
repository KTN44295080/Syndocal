# Video output authoring — Task 33 (2026-10-20)

## Status and scope

Implementation and independent source review are complete. Required validation is not fully green: the full Engine suite failed, and the broad viewport check did not reach application mount. The final native suite and workspace check passed; the complete validation results are recorded below. Real-app and hardware acceptance belongs to Claude in the authorized native acceptance environment after review.

- Worktree: `C:\Users\janua\AppData\Local\SyndocalDev\wt-task33`
- Branch: `task33-output-authoring`
- Base: `7fe67b003a9741ed79550838bf9e79e412ce93a3`
- Owner amendment: create enabled; follow current output role; apply live configuration/removal internally; no authoring confirmation dialogs or extra operator steps; deletion uses Undo.
- The handoff's older “live-output refusal” test phrase is superseded by this owner amendment.
- No application launch, release build, push, merge, stash/reset, or `.agents/` changes were performed. No warning-suppression attributes were added.
- Cargo ran only through `pcargo-here.cmd`, from this worktree, with `CARGO_TARGET_DIR` unset and debug profiles. The wrapper verified the pinned MSVC 14.44 linker.

## Canonical operations and public behavior

| Operation | Tauri route | Behavior |
| --- | --- | --- |
| `syndocal.output.external.add.v2` | `add_external_output_v2` | Create an enabled Spout, NDI or Syphon output for an existing composition. |
| `syndocal.output.video.configure.v2` | `configure_video_output_v2` | Rename, resize, retarget or change kind while preserving authored playback, mapping and composition fields. |
| `syndocal.output.video.remove.v2` | `remove_video_output_v2` | Retire the target's native resources and remove authored references in one history transaction. |

All three use OutputControl V2 requests, schema version 13, exact request/fence validation, current owner identity and a Video-capable lease, durable receipts, history and audit. Applied receipts require exactly one project revision and publication-generation successor; NoOp receipts preserve the full fence. Successful authoring preserves the ordinary global output role, output generation and safety generations. An uncertain admitted failure deliberately faults ownership and retains Pending evidence.

New outputs are enabled in both Standby and live roles. Standby creation authors the output without arming or starting a sender. Live creation can immediately send. Display creation now also works from Standby through the scoped authoring path; its existing live creation path remains in use. Create/configure/remove do not request a confirmation. The UI acquires or recovers eligible ownership internally and never adds an implicit Arm, Enable or forced lease transfer.

External dimensions are 1–8192; Display uses its existing 1–16384 and authoritative monitor identity/physical-size rules. Labels and endpoint names are trimmed and validated by UTF-8 byte length. Endpoint uniqueness is per kind with ASCII case-insensitive comparison. Backend availability is checked authoritatively and reflected in the UI. The legacy raw add/configure/remove routes remain fail-closed.

Managed Show Spout destinations retain their dedicated R4 route. Reserved names and generic Spout edits conflicting with authored reserved destinations are rejected during pure preflight, before Pending evidence or physical retirement. Ordinary unrelated output edits remain available.

## Implementation map

| Area | Source |
| --- | --- |
| Typed specs, action discriminants, validation and receipt successor rules | `crates/protocol/src/control_plane_command.rs:2018` |
| Scoped Engine candidate, compare-and-swap publication, rollback and reference cleanup | `crates/engine/src/video_output_authoring.rs:7` |
| Counted, target-resource retirement without changing global role | `crates/engine/src/lib.rs:11718` |
| Native authoring authority, project transaction, retirement and recreation | `app/src-tauri/src/video_output_authoring.rs:836` |
| Fresh physical authority and restart-required uncertainty | `app/src-tauri/src/video_output_authoring.rs:242` |
| Runtime admission policy and off-event-loop canonical wrappers | `app/src-tauri/src/main.rs:25515`, `app/src-tauri/src/main.rs:51439` |
| Dispatch, Standby Display and no-confirmation policy | `app/src-tauri/src/control_plane_runtime.rs:1413` |
| Exact-label retained Display creation/retirement ownership | `app/src-tauri/src/main.rs:13639` |
| Target-specific transport reconciliation | `crates/video/src/lib.rs:8259` and Spout/NDI/Syphon transport modules |
| Owner-aware frontend command execution | `app/src/outputControlController.ts:1710` |
| App project-intent boundary and post-operation refresh | `app/src/App.tsx:23582` |
| Capability validation and uncertainty feedback | `app/src/videoOutputAuthoring.ts:5` |
| Setup create/configure/remove panels | `app/src/components/VideoOutputCreatePanel.tsx:120`, `VideoOutputConfigPanel.tsx:21`, `VideoOutputListPanel.tsx:65` |

The three new routes are registered in the sorted preflight-only runtime dispatch list; they release outer admission/coordinator guards before the canonical executor. Their narrower native seam owns current authority. The exact runtime route union is 181.

Inventory pins: Tauri 590; frontend invokes 510; Engine sources 286; remote sources 116; MIDI/OSC/DMX sources 239; keyboard sources 33; canonical operations 80; OutputControl operations 31; legacy sources 1741; canonical sources 1774; unclassified sources 1179. Native Tauri SHA-256: `2256afa0d6a07bad401d8f7bd5947b12a0616030e4e0679cc8406bbd20429e18`.

## Authority and native-lifecycle review

Independent read-only review was performed by the control/registry lane for native authoring and Engine retirement. The frontend lane separately reviewed the transport author's scoped Spout/NDI/Syphon changes and the final route-dispatch integration. Root reviewed integration and failing-test corrections.

The review considered accidental physical starts, stale authority, lease leaks, duplicate teardown, unrelated-route preservation, Cue runtime overrides, Undo restoring a removed live output, persisted state, and Recovery:

- The project mutation reservation and Engine submission interval cover the transaction. Short lifecycle/owner/admission/coordinator/transition/registry locks follow the established order and are released before joins, HWND work, GPU work and Engine acknowledgement waits.
- Authority and lease time are refreshed before physical construction, after slow sender readiness, before first Display presentation, and before native publication. Display also rechecks cancellation and its presentation token after the final authority callback.
- Configuration uses the current live video snapshot for physical synchronization. A Cue-disabled output stays physically disabled even when its persisted authored flag is enabled; Cue opacity/blackout are preserved.
- Only the affected output's external routes or exact-label Display workers/windows are retired. Unrelated outputs, inputs and source registration survive targeted edits.
- Creation remains counted until publication or acknowledged retirement. Unpublished/failed creation and teardown remain counted in the target worker, quarantine or startup record until cleanup is acknowledged. Failed Display creation attaches retained creation leases before cleanup. The shared Video/Both control-plane owner lease remains available to other outputs; target-resource ownership is released on its own acknowledgement.
- Unknown admitted outcomes retain durable Pending and faulted admission. They do not report success, promise rollback, reuse an uncertain output ID or automatically replay physical work.
- **Recovery requires restarting Syndocal after an uncertain admitted publication.** Existing outer admission and retirement rules block same-process Recovery; no admission exception or pre-reap bypass was added. Both native and localized frontend messages state restart before Recovery.
- Undo uses existing project replacement and returns ownership to Standby. Restoring a deleted output does not silently rearm it.

## Validation

Dependency installation used `pnpm install --frozen-lockfile --prefer-offline` before frontend checks. Logs are retained under `target/task33-validation/` in this worktree; they are not committed.

| Gate | Result | Evidence |
| --- | --- | --- |
| `check --workspace --all-targets` | PASS: 0 warnings | `workspace-check-reviewed.log`, final source, 2m 17s. |
| `test -p io --lib -- --test-threads=1` | PASS: 255 passed, 4 ignored; 0 warnings | `io.log` |
| `test -p engine` | FAIL: 1170 passed, 3 failed, 14 ignored; 0 warnings | Latest of 3 full attempts, `engine-full-reviewed.log` |
| `test -p syndocal --bin syndocal -- --test-threads=4` | PASS: 2221 passed, 0 failed, 23 ignored; 0 warnings | `syndocal-full-final.log`; third full run after focused corrections. |
| Protocol suite | PASS: 285 passed; 0 warnings | `protocol-final.log` |
| Engine authoring / resource retirement | PASS: 9 authoring and 3 retirement tests | All 12 also passed in the latest full Engine run. |
| Target-specific video transport | PASS: 4 passed; 0 warnings | `video-authoring.log` |
| Native authoring / Syphon / dispatch / Display focused regression | PASS: 19 passed; 0 warnings | `syndocal-corrected-focused.log`; all also passed in final native full run. |
| NDI feature compilation | PASS: `check -p syndocal --all-targets --features ndi`; 0 warnings | `ndi-feature-check.log` |
| NDI scoped-retirement and existing worker regressions | PASS: all 5 worker tests; 0 warnings | `ndi-workers-final.log`, `--features ndi ndi_output_worker_tests -- --test-threads=1`; separate scoped-retirement test also passed in `ndi-authoring-final.log`. No real SDK sender. |
| AI0 / AI7 / frontend routing / native inventory | PASS | `ai0.log`, `ai7.log`, `frontend-routing.log`, `tauri-inventory.log` |
| Authoring / Syphon UI checks | PASS | `output-authoring-ui.log`, `syphon-ui.log` |
| Display-target UI contract | PASS | Final `check-video-display-target.mjs`: automatic ownership, Standby receipt, no confirmation, existing monitor rules. |
| Localization | PASS: 4170/4170, 0 unprotected bare user-data labels | `localization.log` |
| TypeScript | PASS | Final full frontend batch: `tsc --noEmit -p app/tsconfig.json`, `tsc.log`. |
| Broad viewport | FAILED TO COMPLETE; layout unverified | 3 pre-mount timeouts on Vite 5490 / CDP 9490. |
| Encoding / diff | PASS: final-source audit before commit | Changed files UTF-8; Rust LF without BOM; no added allow/expect; `git diff --check` clean. |

### Remaining required-gate failures

The latest full Engine run failed:

1. `tests::dj_link_stage2_worker_revalidates_playing_and_timeline_identity`: full snapshot equality differed in clock/telemetry fields.
2. `tests::max_loop_pre_wait_and_follow_dispatch_128000_occurrences_without_loss`: scheduling took 86.3707 ms against the existing 75 ms limit.
3. `tests::telemetry_counts_dmx_send_failures`: expected count 1 was 0.

An earlier full run also observed a timeline/DMX assertion failure, and focused timeline/max-loop retries passed. The final two-test isolated check (`engine-failures-isolated.log`) passed `telemetry_counts_dmx_send_failures` but still failed `dj_link_stage2_worker_revalidates_playing_and_timeline_identity` on clock/telemetry snapshot equality (1 passed, 1 failed; 0 warnings). Focused results do not turn the full gate green. These failures have not been proven harmless or baseline-only, and their assertions/thresholds were not weakened. Further full retries stopped under the parallel-work retry limit.

All three viewport attempts stopped before app mount. On the diagnostic attempt Vite was ready, `/@vite/client` loaded, and `/src/main.tsx` remained pending for about 8.16 seconds; DOMContentLoaded had not fired, and the root was empty. No layout assertion ran. The temporary diagnostic checker instrumentation was restored byte-for-byte; no viewport checker relaxation is committed. This is unresolved module-load timing, not a verified product-layout result.

Earlier native full runs exposed and led to corrections for the route-dispatch list, obsolete AddDisplay confirmation expectations, a Cue publication barrier, the fake Undo platform's redundant transition, and the Syphon fixture's publication barrier. A separate existing DJ Link full-snapshot equality failure was also observed. The final native run passed all 2221 non-ignored tests with no warnings; earlier failing runs remain in the logs.

## Native acceptance for Claude

Do these in the authorized native acceptance environment after review; this worktree agent did not launch the app.

1. Repeat the failed Engine and broad viewport gates under a stable environment. Verify all configured viewports, Japanese/English text, and all supported themes and theme tokens. Static UI tests do not replace rendered acceptance.
2. On Windows test real Display and Spout; test NDI in an NDI-enabled build; on macOS test Syphon. Confirm unavailable kinds cannot be selected and direct unavailable requests reject.
3. In Standby create each available kind: authored `enabled` must be true, the composition/endpoint/dimensions must persist, and no physical sender/window should start. Create while live and confirm delivery begins without an additional confirmation or Arm step.
4. Keep two outputs live. Rename, resize and change one endpoint; convert its supported kind; retarget a Display using a discovered monitor. Verify the old target retires exactly once, the replacement works, unrelated output remains continuous and no orphan sender/window exists.
5. Configure an output disabled or modified by a Cue. Confirm authored settings change while current enabled/opacity/blackout runtime values remain respected. Verify global role/output/safety generations do not unexpectedly change.
6. Remove a live external sender and live Display without a confirmation. Verify native cleanup, reference cleanup and one Undo step. Undo restores authored state in Standby; Redo and later normal activation must work without overlap.
7. Exercise duplicate endpoints including ASCII case variants, changed monitor topology, stale revision/epoch/owner and lease expiry during delayed startup. Rejections before admission must leave physical/project state unchanged.
8. Exercise delayed first presentation and cleanup failure with the acceptance harness. Verify fresh-authority rejection, retained counted ownership, durable Pending, no false success and the restart-before-Recovery message. Verify no physical replay of the same uncertain operation.
9. Save/reopen and compare all output configuration and composition/reference state. After restart exercise the existing Recovery workflow; Recovery and Undo must not implicitly rearm output.
10. Confirm the legacy raw mutation routes remain rejected and reserved Show Spout outputs continue to use their dedicated route.

Actual HWND retirement, monitor reconfiguration, GPU first presentation, receivers and SDK failure behavior remain unverified here. Fake backend tests establish transaction/lifecycle behavior, not hardware acceptance.
