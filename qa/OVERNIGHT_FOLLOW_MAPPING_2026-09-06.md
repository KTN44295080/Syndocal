# Follow authority, mapping preview, and MCP diagnostics

Base `8b83ddb31f9c5ffff1982985bb2031167e052544`, branch `codex/syndocal-v1.2`,
internal product `1.2.0-alpha.69`. The user confirmed DMX BO appears functional,
reported uncertain Video BO restoration and a stale Timeline Follow read, and
authorized continued MCP-first work while asleep. Bounded implementation used
Luna max, with independent review and root integration.

## Findings and changes

- Ordinary target BO commits authored project state. The controller previously
  refreshed the engine snapshot while the independent one-second authority poll
  could still expose an old project token to the 100ms Follow read. The target
  controller now waits for canonical authority convergence before snapshot
  refresh, including draining an already-running precommit poll. It does not
  patch only receipt fields into project state.
- An exact native stale-Follow-read rejection shares one convergence attempt per
  captured project identity. Its warning is suppressed only after that identity
  is superseded. Failed/current convergence remains visible. There is no output
  retry, Follow replay, runtime clear or automatic abort.
- Yaw drag release previously removed the preview before awaiting transform
  persistence and refreshed-snapshot confirmation. It now retains the preview
  during that interval, ignores duplicate pending pointer events, and prevents
  an old completion from clearing a newer drag. Pointer cancellation before
  commit does not save. Failure returns to authoritative state. This fixes a
  demonstrated transient snapback; persistent user-project rotation acceptance
  remains separate.
- MCP adds `syndocal_get_runtime_status` (`runtime.get`): canonical project,
  lighting/video/S0 blackout bits, output summaries and timeline runtime, plus
  a separately labeled ownership observation. Empty parameters, native claim,
  authentication and existing response bounds remain enforced. No project open,
  playback or output activation command was added.

The saved native terminal receipts contain Video ON at revision 18->19 and OFF
at 19->20, both applied. A later Video ON at 22->23 is also recorded. This is
evidence of committed operations, not rendered or Unity recovery. Sanitized
local evidence is `target/qa/snapshot-cleanup-20260905/video-bo-persisted-terminal-observations.json`;
no lease identifiers or credentials are copied into this document.

## Verification

- Target controller ordering/failure/concurrent stale read/precommit poll and
  one-shot convergence checks: PASS (`check-target-blackout-controller.mjs`).
- Existing Follow runtime checks: PASS (`check-timeline-follow-runtime.mjs`).
- Mapping rotation pending preview, duplicate events, rejection, cancellation
  and stale completion checks: PASS (`check-mapping-rotation-persistence.mjs`).
  Existing mapping stage geometry checks: PASS.
- Agent bridge frontend checks and Node adapter checks: PASS; adapter 12 groups.
- Combined `pnpm --dir app exec tsc --noEmit`: PASS.
- The stale Clip Slot checker was reconciled with the current three Edit
  domains, and its CSS assertion now targets only Clip Slot rules. The checker
  passes; product routing/styles were not changed. The dated remaining-work
  ledger records the scope and evidence.
- Native `cargo test -p syndocal --locked agent_bridge -- --nocapture --test-threads=1`:
  6 passed, 1721 filtered, no compiler warnings. Used the exact MSVC 14.44.35207
  linker pin and PATH-first procedure through the maintained local native helper.
  Log: `target/qa/snapshot-cleanup-20260905/runtime-diagnostics-native-tests.log`.
- Independent review accepted runtime diagnostic, Follow/rotation and the
  receipt-convergence extraction. `check-target-blackout-authority-convergence.mjs`
  executes the extracted callback against a deferred pre-receipt poll, exact
  match, subsequent revision and unchanged-state rejection. App retains only
  dependency wiring for this behavior.
- The final Follow error policy is also inside the convergence module.
  Executable checks distinguish the exact stale-read error from unrelated
  failures, including when the captured project identity is already superseded.
  Independent final review accepted this extraction and the receipt factory.

`pnpm --dir app tauri build --no-bundle` passed: native release 2m14s, Vite 7.63s.
The wrapper verified the exact checkout executable and required MSVC linker.
Initial integration crossed the Vite threshold (App 500.29kB, then 500.01kB after
receipt extraction). Moving same-purpose Follow error policy into its module
resolved the new advisory without changing limits: final App 499.96kB. Previous
checkpoint was 499.46kB. Final native/compiler warnings and Vite advisories are
zero; baseline zero, final delta zero. This is bundle evidence, not a measured
FPS or CPU improvement. No new per-frame polling or renderer work is introduced.

Launched the exact `target/release/syndocal.exe`, PID 95484, SHA256
`611AC8337A078A3137551899841FC1C597C0E36C65E1AE4B70F89D546304A60C`.
Exactly one responsive maximized Syndocal main window was verified. The MCP
stdio probe negotiated five tools and completed fixture-list and runtime-status
reads against this native process, with zero mutations. Startup observations:
E0/R1, no fixtures/outputs, lighting/video/S0 blackout false, Follow idle,
machine output Standby/StartupDenied. An authored-video projection was absent
from this public snapshot; the optional diagnostic field was correspondingly
omitted rather than inferred from the effective bit. Ownership is a separate
read as documented, not an atomic part of the authority bundle.

Evidence under `target/qa/snapshot-cleanup-20260905/`:
`overnight-accepted-native-build.log`, `overnight-launch.json`, and
`overnight-native-mcp-probe.json`. The current Codex session discovers the
original four tools; the fresh stdio connection discovers the additive fifth
tool. No Codex credential or connection setting was changed.

## Remaining boundaries

Video restoration/isolation in Unity, actual fixture mutation in the user's
project and UI Undo/Redo require separate acceptance. The app was already exited
at investigation start; the restarted app has an empty startup project. No user
show was opened through an undocumented route and no physical output was
energized. The unrelated dirty `app/scripts/check-viewport-containment.mjs`, user
show files and Unity assets remain preserved.
