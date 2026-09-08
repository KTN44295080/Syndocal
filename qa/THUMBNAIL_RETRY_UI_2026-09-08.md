# Thumbnail retry UI - 2026-09-08

Base main: `f6281167925ece4ad6b8fdee74bf65ad88c2b275`.
Branch: `chatgpt/thumbnail-recovery-ui-20260908`.
Scope: reachable explicit recovery for missing thumbnails, not GUI cancellation.
Main promotion requires the remaining checks stated below.

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
A later mergeProps binding case was added but its direct Node run is pending.

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
missing-thumbnail predicate. The final capability fix and scalar type annotations
are self-reviewed, not independently re-reviewed. Review input/result are retained.

A proposed ordinary regression batch (controller, TypeScript, VJ access, routing,
release, localization, wrapper, frontend inventory and ledgers) was denied before
execution. Its individual commands must not be listed as newly passing. One
combined source/process-inspection request was also denied. No permissions or
safety settings were weakened. Only the separately executed checks documented
here are evidence; native beforeBuildCommand includes its own TypeScript check.

No native source, IPC inventory, ASIO/NDI admission, media schemas, fixed layout,
Mac tooling or product version was changed. Busy notifications occur at batch
lifetime boundaries; UI missing-state scans are memoized. There are no additional
native readers, background polling threads or per-frame copies. App view wiring
was extracted rather than raising its bundle-size warning threshold. No FPS,
latency, hard cancellation or full-media-completion claim follows from this work.

Before main integration: run the denied final regression batch, re-review the
final diff, and confirm the final native evidence below belongs to that source.
Fix only demonstrated failures; do not replay unfinished Channel/Mac branches or
weaken assertions. Main integration is separate from a signed/public release.

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

Final disposition: candidate source and evidence saved on its own branch; main
not updated. Outstanding ordinary regressions and independent re-review of the
final capability fix are the handoff, not a request to reimplement the feature.
