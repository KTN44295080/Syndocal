# Thumbnail retry UI - 2026-09-08

Base main: `f6281167925ece4ad6b8fdee74bf65ad88c2b275`.
Branch: `chatgpt/thumbnail-recovery-ui-20260908`.
Scope: reachable explicit recovery for missing thumbnails, not GUI cancellation.
Main promotion was completed after the remaining checks stated below passed.

## Behavior and responsibilities

Previously both existing Load Thumbnails buttons disappeared once access was
allowed, including after a failed read. The controller could retry, but those
operator paths no longer exposed that action. Both surfaces now distinguish
initial loading, active work, retryable local-source gaps and completed caches.
Active work disables repeated clicks and has aria-busy; after failure a Retry
Thumbnails button uses the existing authorization/reload callback. Successful
cache entries are not refetched. Empty/live-only Media Library catalogs do not
advertise file-thumbnail recovery. A missing native capability disables initial
loading and does not expose a misleading retry action.

The latest-batch owner reports its actual busy lifetime without releasing an
outstanding native promise. Layer and asset requests remain independent: retrying
an idle lane does not retire the other active lane. Source/authority changes,
reset and disposal retain their generation/authority fences. Busy after reset
means old work is still owned, not that cancellation or stopping was completed.
The controller provides reactive thumbnail view getters; App only merges those
ports with its existing per-panel props using Solid mergeProps.

## Automated and browser evidence

Evidence root: `target/qa/thumbnail-recovery-20260908/`.
The existing production-controller regressions passed after independent busy
observation and idle-only reload were implemented. Added executed cases cover
30 repeated requests, the unaffected active lane during retry, one latest queued
successor, reset/disposal ownership, old-response rejection and cache reuse.
The later mergeProps binding case is now covered by the executed controller
regression and passed.

`browser-capability.log` exited 0 on the final capability-gated components and
reactive view bindings. The dedicated Vite/Playwright fixture mounts the real
VideoClipGridPanel, VideoControlPanel and thumbnail controller. At 1280x720 and
1920x1080 it verifies no reads before explicit authorization, disabled repeat
clicks, loading status, independent lanes, Enter-key asset retry, layer retry,
success-cache reuse, reset and unavailable-native behavior. The retry button's
observed dimensions were 180x30; no product font/control size was reduced.
The final screenshot was inspected for the 1280x720 fixture. These are real DOM
interactions with deferred test readers, not full native rendering/device proof.

Earlier browser attempts failed because test data omitted layer.state and
clip-slot props. Those logs are retained. `browser-03` and `browser-final` passed
before the capability case was added; neither is substituted for the final run.
The fixture is under scripts/fixtures, not in the production app entrypoint.
Its server binds only loopback, and its own browser/server are closed on exit.
No application device, external show or operator project is used by these tests.

## Review, performance and remaining gates

One Codex CLI/gpt-5.6-luna review was run in read-only mode on inline source; it
was not used to edit, run tests or evade a denied operation. The reviewer found
an enabled dead retry in non-Tauri mode. Native capability now gates the action,
and the final browser run proves disabled/no-read/no-dead-retry behavior. It also
recommended excluding live-layer gaps from file retry, which is reflected in the
missing-thumbnail predicate. At handoff time the final capability fix and scalar
type annotations were self-reviewed; the resume validation below supplies the
independent read-only re-review. Review input/result are retained.

A proposed ordinary regression batch was initially denied before execution;
that is historical only. The batch was later executed in the resume validation
below. One combined source/process-inspection request was also denied. No
permissions or safety settings were weakened.

No native source, IPC inventory, ASIO/NDI admission, media schemas, fixed layout,
Mac tooling or product version was changed. Busy notifications occur at batch
lifetime boundaries; UI missing-state scans are memoized. There are no additional
native readers, background polling threads or per-frame copies. App view wiring
was extracted rather than raising its bundle-size warning threshold. No FPS,
latency, hard cancellation or full-media-completion claim follows from this work.

Before main integration: the final regression batch, final-diff review and
source/native hash cross-check below must remain green. Fix only demonstrated
failures; do not replay unfinished Channel/Mac branches or weaken assertions.
Main integration is separate from a signed/public release.

## Resume validation - 2026-09-08

The candidate remained clean at `577b4aa0ddd81c227f77807d1e53a337a7d432c7`,
equal to `origin/chatgpt/thumbnail-recovery-ui-20260908`, with main still at
`f6281167925ece4ad6b8fdee74bf65ad88c2b275`. A read-only independent review of
the final diff confirmed the capability gate, File/StillImage-only missing
predicates, disabled/aria-busy busy state, Solid `mergeProps` getter wiring,
and unchanged authority/reset/disposal fences. No source fix was required.

The previously unexecuted batch is recorded in
`target/qa/thumbnail-recovery-20260908/resume-01/command-results.json`; all
11 commands exited 0:

`check:media-thumbnails`, `tsc --noEmit`,
`check-vj-media-import-access`, `check:frontend-command-routing`,
`check:release`, `check:localization`, `check:tauri-build-wrapper`,
`check:frontend-invokes`, `check:completion-ledger`, `check:q1-q4-ledger`,
and `git diff --check`.

The six source hashes in `final-source-hashes.json` were recomputed and all
matched. The existing final native artifact remained the same EXE:
64,532,992 bytes, SHA-256
`113545610241AC6AC2957CB0C355572EA59572775E5ABE68B3A4870B38398505`.
The existing native launch/IPC evidence remains applicable to that exact
artifact and records exit 0, responsive/maximized one-window state, selected
snapshot and missing-layer/asset rejection cases, rejected excluded
cancellation IPC, zero project/physical-output commands, owned process exit,
and zero remaining debug listeners.

No product source, native IPC, ASIO/NDI, fixed layout, Mac, version or
unfinished Channel API scope was changed during resume validation. Main
integration is permitted after this QA checkpoint is committed and the remote
heads are rechecked.

## Final native evidence

The final `pnpm --dir app tauri build --no-bundle` exited 0. Its frontend step
executed `tsc --noEmit && vite build` successfully on the final capability fix.
App output was 499.83 kB, below the unchanged 500 kB warning threshold; the earlier
500.02 kB warning disappeared through getter extraction, not suppression.
The final log contains zero Rust warning diagnostics. No pre-change full native
warning matrix was rerun; this is the measured configuration, not a global delta.

Earlier native attempts are retained: first failed because this new checkout
lacked its seven runtime DLLs; those were then copied only after matching the
repository's exact pinned hashes/sizes. The next failed on four callback-parameter
types after mergeProps extraction, corrected with explicit string/number types.
A later successful build preceded the capability fix and was not used for final
acceptance. `native-build-final.log` and `native-build-result-final.json` identify
the final run. `final-source-hashes.json` still matched immediately before launch.

Final EXE: `target/release/syndocal.exe`, 64,532,992 bytes, version alpha.69.
SHA-256: `113545610241AC6AC2957CB0C355572EA59572775E5ABE68B3A4870B38398505`.
At `2026-09-08T05:36:26.4631453Z`, the owned PID 44820 had one responsive,
maximized Syndocal window. The real-WebView probe read the snapshot, got the
specific missing-layer/asset rejection twice per lane, and rejected the excluded
cancellation IPC. Probe/launcher exited 0, the application exited and no debug
listener remained. Evidence is in `target/qa/thumbnail-recovery-native-20260908/`.

The native probe issued no project-mutation or physical-output commands. It does
not demonstrate successful file decoding or retry through the complete native
operator facade; recovery interaction evidence above is the component/browser
fixture. GUI cancellation, running-decoder-stop latency, physical devices, Mac
execution of this UI change, signing and venue acceptance remain outside scope.

Final disposition: the candidate and its QA checkpoint were fast-forwarded to
main after validation. The final main SHA is
`edb677292639f9b5f6cea75ad8cbad2857c56f2c`, equal to `origin/main`. The
remaining boundaries are not a request to reimplement this feature.
