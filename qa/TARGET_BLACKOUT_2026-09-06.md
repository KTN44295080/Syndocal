# Target blackout separation

Base `ab05fceba4c533353ffc7b06eae3ec37e9369e27`, branch `codex/syndocal-v1.2`, internal product `1.2.0-alpha.69`.

## Behavior and responsibility

DMX, VID, and ALL now target the existing authored lighting, video, or both blackout bits. Previously DMX/ALL invoked the emergency S0 latch and video presentation also read effective DMX blackout. Emergency S0 remains independent, overrides both outputs, and has an explicit release indicator. Full Lock retains safer-direction emergency engage and forbids release.

A new additive `set_blackout` action uses the existing exact Both lease; target specifies effect scope, not a relaxed lease match. It never starts an inactive output to perform blackout. Legacy setter commands remain retired. Existing opcodes are unchanged; new opcode is 20. Native admission and schema inventories include the new command.

The UI controller owns pending interaction and delegates typed IPC. Native authorization and project publication are separate small modules; the engine owns atomic bits and publication acknowledgement. The candidate project image is verified before checkpoint/history and durable receipt commit. A queued request can expire; an admitted request waits for its definitive result rather than reporting failure before late application. No new render-loop polling or per-frame project clone is introduced. DMX-only changes no longer invalidate the video presentation token. Follow cannot blend old lighting values over blackout. Shared Spout/NDI and Native Display predicates use video blackout plus S0.

`authored_blackout` and `safety_blackout_engaged` are derived UI fields, also carried by snapshot deltas. Persisted S0 remains false. Project load still uses existing authored blackout truth. The user test project and Unity files are unchanged.

## Verification

- Engine target tests: 6 passed, including atomic targets, expiry/publication rollback, held submission, definitive ACK, Follow, and S0 preservation.
- Protocol focused strict-wire test: passed; existing opcodes and target/enabled/lease shapes checked.
- Frontend OutputControl, target controller, snapshot delta, safety controller, operator policy, invoke inventory (451), and TypeScript checks: passed.
- Native `cargo test -p syndocal --locked blackout -- --nocapture --test-threads=1`: 20 passed, including all target/direction persisted images, public receipt replay/conflict, NoOp, S0, delta transport and shared video fence.
- Native `cargo test -p syndocal --locked control_plane::tests -- --nocapture --test-threads=1`: 30 passed (also selects authored-control-plane tests). Exact frozen inventory: native 511, engine 279, frontend 451, canonical sources 1596; native SHA-256 `a7f349ccf6bc270ada37f32744bc598198452693c7b5c242a7e2a9af2e670d02`.
- Windows test commands used `node target/qa/recording-atomic-20260905/run-native.mjs` with MSVC 14.44.35207 absolute linker pin and PATH-first verification. Logs are under `target/qa/snapshot-cleanup-20260905/target-blackout-*-tests.log`.
- Earlier failed attempts found missing schema/inventory integration, stale persisted fence and test-fixture issues. They were corrected; only the final successful runs above are acceptance evidence.
- `pnpm --dir app tauri build --no-bundle`: passed, Vite 6.60s / native release 2m44s, first-party warnings 0 (baseline 0, delta 0). Wrapper verified and stopped only checkout PID 103936 before building.
- Launched `target/release/syndocal.exe`, PID 71960, SHA-256 `6223FB5A1EB09A650439439793533F8C152C17FF5565E63A4C4C2BDAD5158071`. Exactly one responsive maximized Syndocal main window. Normal launch, no temporary CDP or timing flag. Evidence: `target/qa/snapshot-cleanup-20260905/target-blackout-native-build.log` and `target-blackout-launch.json`.
- Unity/manual target isolation and a real UI Undo/redo round trip remain unverified. Shared fence tests cover Spout and NDI kinds with a fake SDK send; no live NDI transport acceptance is claimed.

## Next manual acceptance

Open the original Unity-4K-test project and start its configured outputs through the normal output controls. Check DMX ON/OFF leaves both video panels visible, VID leaves lighting running, and ALL blanks both, then restores each. Repeat while paused. Ordinary buttons do not clear independent S0; use the explicit safety release if S0 is engaged. No project or output operation was automated during this checkpoint.

Independent source review accepted the final persisted publication change. Root inspected the integration and verified the test results. Compiler warnings for final engine/native tests: baseline 0, current 0, delta 0. The native adapter verifies and commits history images; a manual Ctrl+Z/redo round trip in the running app is not claimed.

Exact frontend commands: `node app/scripts/check-output-control-runtime.mjs`, `node app/scripts/check-target-blackout-controller.mjs`, `node app/scripts/check-blackout-snapshot-delta.mjs`, `node app/scripts/check-safety-blackout-runtime.mjs`, `node app/scripts/check-workspace-operator.mjs`, `node app/scripts/check-frontend-tauri-invokes.mjs`, and `pnpm --dir app exec tsc --noEmit`. Preserve unrelated `app/scripts/check-viewport-containment.mjs`; no cleanup or version bump is part of this fix.

## Follow-up: dispatch policy ordering

Base `eef26a4fa8b36f517aac3e93c1f9a7bd582ea61c`, same branch and internal product version.
The user pressing DMX BO received `set_blackout_output_control_v2 has no reviewed
dispatch-fence policy`. Its entry existed, but had been inserted immediately
after `release_blackout_output_control_v2` in a binary-searched sorted slice.
That broke lookup before the native output handler ran. The frontend conservatively
wrapped the rejection as a lost reply; that message did not establish a hardware failure.

Moved the entry after `set_asio_output_test`, before `set_display_output_window_open_v2`.
No output semantics, authorization, or recovery path changed. The existing test
already checks ordering and resolution of every runtime command, but was omitted
from the previous blackout checkpoint's selected tests. Its stale count 156 was
updated to 157: 50 outer-fenced routes plus 107 preflight/inner-authority routes,
matching the current admission inventory including the newly added blackout command.

Exact MSVC wrapper command:
`node target/qa/recording-atomic-20260905/run-native.mjs cargo test -p syndocal --locked synchronous_project_runtime_routes_hold_the_identity_fence_through_dispatch -- --nocapture --test-threads=1`.
Before: failed the sorted-slice assertion. After: 1 PASS, 1726 filtered, including
all 157 runtime policy resolutions. Logs: `blackout-dispatch-before.log` and
`blackout-dispatch-after.log` under `target/qa/snapshot-cleanup-20260905/`.
Independent review found this was the only ordering violation across the four
related lookup slices; the minimal entry move and existing coverage are sufficient.

`pnpm --dir app tauri build --no-bundle` PASS, native release 2m28s / Vite 9.59s.
The wrapper verified and stopped exact-checkout PID 102504 and pinned the required
MSVC linker. Relaunched exact executable as PID 101560, one responsive maximized
Syndocal main window, SHA256
`818ED47D0FE66D3D52F988329900BB68F4430D5A7C3E932D094A0E9EB5AF0699`.
Evidence: `blackout-dispatch-build.log` and `blackout-dispatch-launch.json` in the
same QA directory. Native test/build warnings: baseline 0/current 0/delta 0.
Real user DMX/VID/ALL isolation in Unity and UI Undo/Redo remain unverified;
no physical-output action was automated. Next action is the user's DMX BO retry
in their project, then the target-isolation acceptance above.
