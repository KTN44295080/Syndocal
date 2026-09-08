# Native thumbnail success path — 2026-09-08

Original repair base: `139b5b8608428dd473d5afef3e4a94327f3e85a4`.
Integration base: `573a09919d415c22f75dad48948f0ea3bed223cc` (parallel icon update preserved).
Branch: `chatgpt/thumbnail-native-ui-integration-20260908`.
Completed subset: real operator PNG/MP4 import, immediate library population,
real native thumbnail decoding and visible image content in an isolated Windows app.
Fault-injected retry, GUI cancellation and general media acceptance are NOT closed.

## Reproduced product defect

Before the fix, the actual import UI committed a PNG (and in the first trial an
MP4) into the native catalog. The success message and native snapshot confirmed
it, but the mounted Media Library remained at zero assets. Serial first-asset
reproduction ruled out overlapping imports; switching away and back displayed
that asset. The pre-fix native probe caught a handled TypeError at
`selectedVideoClipSlotRuntime`: `.layers.find` was called on an omitted array.
That error interrupted Solid's reactive updates during an authority-bundle commit.

`VideoClipRuntimeSnapshot.layers` and `VideoLayerTransitionRuntimeSnapshot.buses`
are Rust serde-defaulted and omitted when empty. The frontend stored the raw `{}`
runtime despite its TypeScript required-array type. This is not a mergeProps bug.
The existing component fixture started with an already-populated catalog and did
not establish this native empty-to-first-import behavior.

## Fix and compatibility

A small `videoRuntimeCollectionWire.ts` adapter normalizes only an omitted `layers`
or `buses` field. Null/non-object publications and explicitly malformed collections
are rejected, not replaced with empty data. Populated objects/arrays retain identity.
The existing controller checks authority/generation eligibility first and normalizes
before advancing the accepted watermark or notifying consumers. Invalid generation
100 cannot prevent a later valid generation 2 publication. Native IPC, serialization,
project schemas, ownership policy and UI layout are unchanged; no per-frame workers,
locks or populated-array copies were added. Array-entry semantic validation remains
with the existing native contract; this adapter validates the collection envelope.

## Executed gates and review

`check-video-runtime-polling.mjs` failed before the fix on `{}` vs `{layers:[]}`.
Afterward both clip/transition lanes passed omission, explicit-malformed rejection,
watermark retention, same-object/array preservation, original single-flight,
replacement/reset, stale-result and owner-retirement regressions.
`check:release`, VJ media-import, frontend command routing and Clip Slot B4 passed.
Routing: 133 renderer / 31 server / 28 raw / 463 facade. Release retained 515 native
commands and 18 rejected negative fixtures. Wrapper: 243 assertions / 27 hostile fixtures.
The native build ran TypeScript and Vite successfully. Final Rust warning diagnostics: 0;
no pre-change platform-wide warning delta or FPS/stop-latency measurement is claimed.

One read-only Luna/Codex CLI review received the bounded diff and new adapter inline,
with no file editing or test execution delegated. It found no blocker. Its suggested
null-result and watermark coverage already executes in the normal checker invocation
without `--baseline-ref`; no independent native acceptance was claimed by that review.

## Actual native proof

The maintained `run-tauri.mjs build --no-bundle --config <QA config>` produced a
native EXE with ONLY a unique bundle identifier override. This keeps backend settings,
recovery receipts and WebView state in a dedicated namespace; no fake readers,
permission overrides, generated fixture app entrypoint or default operator show is used.
The fresh QA output preference was Standby. Actual native status remained
StartupDenied for both lighting and video, with no video outputs. Neither output
arming nor physical-output commands were issued. The ordinary App source-file form
and Add Video Layer action imported the generated PNG and MP4 sequentially. Each
import immediately populated a real card without remounting. Explicit operator
Video selection authorized thumbnails; no additional Load click was needed here.
Actual native reads produced 160x90 displayed images; canvas samples verified the
four expected colored quadrants and opaque alpha. PNG centers were exact, video
centers differed by at most 3/255. Selecting a card retained the displayed images.
The observed IPC traffic contained two asset and two layer thumbnail requests.
This is not an exhaustive cache eviction, retry or decoder-stop test.

Final QA-identifier EXE: 64,532,992 bytes, alpha.69, SHA-256
`00C6838C6B37AC359688434C3A424760A67D66F39DA42A023B88709305673C29`.
Source hashes matched after acceptance. At `2026-09-08T07:36:26.8859085Z` the launch
report recorded one responsive maximized window, probe exit 0, owned process 67220
exited, and zero remaining loopback debug listeners. The **worktree's EXE is now a
QA-identifier artifact**, not a default-profile distribution build. Do not ship it;
rebuild without the config override before ordinary artifact use. Installed apps
and the separate main checkout's executable were not replaced.

Evidence: `target/qa/thumbnail-native-ui-20260908-01/` holds pre-fix caught-exception
and remount observations, before/after controller regression logs, fixed build log,
review, `fixed-artifact.json`, `fixed-probe.json`, `fixed-launch.json`, and screenshots.
The initial build quote error and earlier locator failures are retained as failures,
not product test successes. The final test does not use view remount to pass.

## Preservation and remaining boundary

Five ordinary top-level configuration/journal JSON files matched their initial
hashes. The sixth, the ephemeral agent-bridge descriptor, changed during the long
session: it identified PID 71980 and the separate main-checkout EXE, not any owned
QA launch. Its timestamp was outside the final QA run. No descriptor or unowned
state was restored. The initial all-six-equal audit failed and remains a failure;
this is not a claim that every operator-profile file stayed byte-identical.
A diagnostic shell JSON-decoding error printed descriptor contents; those outputs
are not copied into this public QA record or the checked-in tests.

A proposed file-move fault-injection operation was service-denied and not executed.
The accepted test was narrowed to ordinary UI imports without moving sources.
Therefore the real native missing-file→Retry→recovery path is still a separate unit.
The earlier browser recovery tests remain valid within their mock-reader scope.
GUI cancellation/stop latency, physical devices, Mac execution of this change,
signing and venue acceptance remain open. No broad completion-ledger row is closed.
Only this normal native operator import/render/display failure and its recurrence
are addressed here. Complete and review each remaining boundary independently.

## Final integration validation

Before promotion, main had advanced through the separately accepted icon update
`573a099`. Integration stopped at the old-base guard, inspected that seven-file
icon/QA delta, and cherry-picked only this four-file repair onto the new main.
Application TypeScript, scripts and crates match the reviewed repair exactly;
all new icon files were preserved. No force-push or rollback of parallel work.

The combined tree `b0132b7d9f2350d26a7ef0a1a676825c167635eb` was rebuilt with
the same identifier-only isolation config: exit 0, Rust warning diagnostics 0.
Its QA EXE SHA-256 is
`A7A8CCAECB8721EEFC00CD4107080023CDD8C78765E64F2DFF84A5005A8768BF`
(64,532,992 bytes). A new native run again imported PNG and MP4 through actual
UI actions, immediately populated both cards and verified 160x90 image pixels
without view remount, raw mutation commands or mocked native readers.
At `2026-09-08T07:51:57.8268646Z`, PID 69588 had completed the passing probe;
one responsive maximized window, owned application exit, debug listener count 0.
The final-run comparison confirmed all six ordinary top-level JSON files unchanged
within that run; this does not retroactively erase the earlier descriptor change.

Final evidence: `integrated-build.log`, `integrated-artifact.json`,
`integrated-probe.json`, `integrated-launch.json`, `integrated-preservation.json`
and `integrated-success.png` beside the earlier retained evidence. The successful
unit is normal native import/render/display plus the omitted-runtime regression;
fault-injected native retry remains the next distinct acceptance unit.
