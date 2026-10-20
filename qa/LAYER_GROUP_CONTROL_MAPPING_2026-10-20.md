# T28 — Layer-group control mapping (2026-10-20)

Worktree: `C:\Users\janua\AppData\Local\SyndocalDev\wt-task28`
Branch: `task28-group-mapping`
Base: `e1d72e2306620d9c64454abf426ded50c1261f4d`

## Behavior and compatibility

The existing Perform mapping path now supports group opacity, solo, bypass, blend, and enabled. Both the Perform group strip and the group inspector expose all five controls to the existing Learn and Assign menu. The obsolete unsupported-Learn notice is removed. Target labels, controls, and new admission errors have English and Japanese text.

Mappings carry the typed `video_group_id`, never a row index or a group name. Reorder and rename preserve the target; removing a group leaves its saved mapping inert. MIDI, OSC, and DMX mapping JSON defaults an absent group field to None and omits None when serialized, preserving legacy JSON shape without a schema-version bump. Native and renderer admission reject missing, zero, unsafe (> 2^53 - 1), or mixed-identity group targets.

| Target | MIDI / OSC / DMX behavior | Beat division |
| --- | --- | --- |
| Opacity | Absolute continuous value, clamped to 0–1 | Unavailable, matching existing continuous-target policy |
| Blend | Absolute value selects one of the existing ten blend modes | Unavailable |
| Solo / bypass / enabled | Note/OSC pulses toggle; CC, pitch-bend, explicit OSC booleans, and DMX levels set state, including false/zero | Toggles through the existing scheduler |

MIDI note release does not retrigger a group toggle. Relative MIDI and native MIDI feedback remain unsupported for these renderer-owned group targets, as for comparable Perform targets.

## Runtime and Undo decisions

- Group writes use the existing ticketed `apply_video_effect_catalog_authoritative` command; no Tauri route or input-event variant was added.
- Each group/property lane retains the newest value with 200 ms minimum spacing. All mapped group catalog writes additionally share one serialized lane. The limit is per property, not a global five-updates-per-second guarantee.
- The catalog is constructed after native-operation preflight from the latest authored snapshot. This preserves pending unrelated catalog changes and uses the latest revision/checkpoint.
- Project epoch, input-source lease, Learn/modal suspension, group existence, and the FX mutation gate are rechecked at the queued/deferred boundaries. Stale work is dropped.
- Continuous opacity/blend changes use the existing 750 ms idle Undo window, keyed by owner, window, incarnation, stable group ID, and property. Coalescing requires exactly one supported field change in exactly one unchanged group identity, with no other catalog/composition change. Flags, membership, reorder, transform, cross-group, and mixed edits do not coalesce.

## Main implementation references

Paths below are relative to the worktree above.

| Area | File:line |
| --- | --- |
| Protocol actions and optional stable IDs | `crates/protocol/src/lib.rs:7900`, `crates/protocol/src/lib.rs:8267` |
| Native MIDI and OSC event routing | `crates/io/src/midi.rs:2132`, `crates/io/src/osc.rs:1262` |
| Native MIDI / shared OSC-DMX admission | `app/src-tauri/src/main.rs:31404`, `app/src-tauri/src/main.rs:31619` |
| Learn identity and typed payload parsing | `app/src/controlMappingLearn.ts:134`, `app/src/controlMappingPerform.ts:133` |
| Latest-value dispatcher and deferred catalog patch | `app/src/performControlDispatcher.ts:120`, `app/src/performControlDispatcher.ts:279`, `app/src/createVideoRuntimeController.ts:264` |
| Shared modal/source guards and Beat integration | `app/src/App.tsx:22036` |
| Learnable inspector and Perform group strip | `app/src/components/VideoGroupInspector.tsx:22`, `app/src/components/VideoPerformMatrix.tsx:218` |
| Group target labels and Japanese localization | `app/src/controlMappingLabels.ts:69`, `app/src/uiLocalization.ts:4425` |
| Native Undo policy | `app/src-tauri/src/project_undo_grouping.rs:50`, `app/src-tauri/src/main.rs:12376` |
| Inventory pins | `app/src-tauri/src/control_plane.rs:2683`, `app/src-tauri/src/control_plane.rs:3119`, `app/src-tauri/src/control_plane.rs:3554`, `crates/io/src/control_plane.rs:348`, `app/scripts/check-ai0-source-coverage.mjs:154` |
| New regression suites | `app/scripts/check-group-control-mapping.mjs:1`, `crates/protocol/src/video_group_mapping_tests.rs:1`, `crates/io/src/video_group_midi_tests.rs:1` |

MIDI action sources increase 60→65, OSC action sources 55→60, combined I/O sources 229→239, legacy sources 1722→1732, canonical sources 1755→1765, and unclassified sources 1168→1178. Canonical operations remain 76. Tauri routes remain 586; frontend sources remain 506.

## Validation evidence

Frontend dependencies were installed in this worktree with `pnpm install --frozen-lockfile --prefer-offline`. Browser checks used Vite 5440 / CDP 9440. Every Cargo invocation used the worktree-local target and `pcargo-here.cmd` with MSVC 14.44.35207 PATH-first validation, debug profile, and CARGO_TARGET_DIR unset.

| Check | Observed result |
| --- | --- |
| `pnpm exec tsc --noEmit` | Passed |
| `pnpm run check:midi-learn` | Passed, including MIDI Learn controller, Perform routing, and the new group mapping suite |
| `node app/scripts/check-group-control-mapping.mjs` | Passed, including burst thinning, catalog serialization, project/source/modal/deletion fences, and preflight/FX races |
| `check:video-layer-groups`, `check:video-runtime-polling` | Passed |
| `check:perform-layout` | Passed: six EN/JA viewport cases plus five runtime-focus cases; actual group Assign menus exercised at 1280×720 in both languages |
| `check:localization` | Passed: 4145/4145 static Japanese coverage; zero unprotected user-data findings |
| `node app/scripts/check-ai0-source-coverage.mjs` | Passed; 239 I/O sources, existing route counts preserved |
| `test -p protocol video_group -- --test-threads=1` | 4 passed, 0 failed; selected group tests actually ran |
| `test -p io --lib -- --test-threads=1` | Full library: 243 passed, 0 failed, 4 existing environment/device tests ignored |
| `test -p syndocal --bin syndocal perform_input_validation_tests -- --test-threads=1` | 9 passed, 0 failed |
| `test -p syndocal --bin syndocal project_undo_grouping -- --test-threads=1` | 10 passed, 0 failed |
| Additional `test -p syndocal --bin syndocal control_plane -- --test-threads=1` | Last run: 119 passed, 1 failed on the stale unclassified-source pin (actual 1178, expected 1168). Corrected the pin to 1178 afterward; no fourth suite execution under the parallel-task retry limit. This suite is not claimed green on the final tree. |
| Diff / Rust encoding | `git diff --check` passed; all 16 changed/new Rust files UTF-8, LF, no BOM; no new allow/expect attributes |

Current Rust compiler warnings: 0 in the protocol, I/O, and app debug-test configurations actually compiled, including the last inventory run. A separate baseline was not compiled; baseline and warning delta are unmeasured. pnpm also reported an ignored-esbuild-build-script dependency warning.

Native test logs are under `target/task28-protocol-tests.log`, `task28-io-tests.log`, `task28-native-validation.log`, `task28-native-undo.log`, and `task28-native-inventory-verified.log`. The last log records the pre-correction inventory failure. Browser evidence is under `target/qa/vj-perform-ui/browser`; the Japanese 1280×720 group-inspector image was visually inspected.

## Review and remaining acceptance

Independent read-only review covered identity/serde compatibility, native input semantics, malformed targets, queued command invalidation, stale catalog overwrites, FX preflight races, and Undo isolation. The remaining inventory-count correction was also reviewed. The source-count assertions retain exact inventory checks rather than weakening the contract.

First follow-up: rerun the additional app `control_plane` suite after the final 1178 assertion correction. Native GUI and real MIDI/OSC/DMX/Beat end-to-end acceptance remain with the merging/main-checkout agent. No release build, native/real-app harness, `syndocal.exe` launch, main-checkout edit, push, or merge was performed.
