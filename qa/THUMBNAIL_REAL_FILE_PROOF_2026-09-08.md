# Real-file thumbnail verification and remaining scope — 2026-09-08

Base main: `933fab4ddd6990912d30344d990b96f31132f34b`.
Branch: `chatgpt/thumbnail-real-file-proof-20260908`.
Scope: tests of existing native real-file preparation/rendering, plus a current
remaining-work snapshot. No product behavior or command permissions are changed.

## Test boundary

The tests call the production preparation, private-copy and rendering functions,
and the actual Tauri blocking-worker dispatcher. No reader or decoded pixels are
mocked. Four-color PNGs are generated in exclusive test-owned temporary folders.
The 1344-byte silent MP4 has two known color patterns at 0 and 500 ms; its source,
encoder recipe and SHA-256 are documented in `app/src-tauri/test-assets/THUMBNAIL_FIXTURE.md`.
PNG samples must match exact RGBA bytes; lossy video RGB channels allow an absolute
error of at most 5/255 at quadrant centers. Size and pixel format are asserted.

The new cases check resized and repeated PNG decoding, actual video-frame position,
private-copy isolation after replacement of the original file, stale-catalog rejection,
new-catalog recovery and successful blocking-worker admission release.
They are native Rust application tests, NOT a shipped-EXE/WebView user-flow test.
The application main/setup function is not launched, so operator projects, persisted
machine bindings and physical outputs are not used. Nothing is substituted into
an operator's existing show. All test-created private files are owned and removed.

## Remaining product scope at this base

Sources: `SYNDOCAL_COMPLETION_LEDGER.json`, `REMAINING_WORK_2026-09-05.md`,
`SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`, and the integrated main/Mac/retry QA notes.
The machine ledger still declares 50 Open / 8 Deferred. These are wide acceptance
contracts, not 58 absent implementations or a measurable completion percentage.
The historical helper table groups them as 12 implementation/decision scopes,
19 existing-implementation reconciliation scopes, 19 external scopes and 8 deferred.
Those numbers are not refreshed implementation estimates. This checkpoint does
not automatically mark those rows Complete or reimplement accepted subsets.

| Domain | Still to establish or complete |
| --- | --- |
| Media (`MEDIA-DERIVED-001`) | Real operator/WebView import→thumbnail→display→retry/cache success; GUI-to-native cancellation and retirement order; stop latency; cache/eviction limits; waveform/proxy/analysis identity; cold/warm budgets. |
| AI/control (`AI0`–`AI8`, `AI3-*`) | Coverage beyond 47 reviewed canonical operations; authored-operation/Undo parity; consent/principal/grant and admin/health UI; crash/reply-loss recovery; adversarial and external-client acceptance. Existing registries/journals must be reused. |
| Clock/ownership (`F1`, `F2`, `SHOWCLOCK-*`) | Complete generation/owner integration; freeze clock-master/failover/fence decisions; exactly-once timed actions; two-process/two-machine fault/soak proof. |
| Timeline/video (`VIDEO-C2-C4`, `TIMELINE-*`, `VIDEO-LIVE-SOURCES`) | Take/mapping/Timeline integration; Follow/crossfade/tempo-slew and failure behavior; save/reload/Undo selection and focus; live-source identity, permission and reconnect acceptance. |
| Audio/recording (`AUDIO-AUTHORED`, `RECORDING`) | Authored audio persistence and clock/PTS policy; resampling/seek/loop/underrun/device faults; integrated recording fault/disk/crash and real-show evidence beyond accepted stop/publication subsets. |
| UI/performance (`UI-H1`–`H5`, accessibility) | Reachability and consistent status/selection/recovery throughout Setup/Edit/Control; remaining viewport/accessibility proof; representative-show snapshot clone, lock wait, delta and payload measurement. |
| Security/migration/support | Remote pairing/exposure/Origin/Host/rate/size contracts; archive/path/parser tests; versioned migration and future-schema rejection; diagnostics, redaction and recovery documentation. |
| Hardware/distribution | ASIO format/fault/soak/latency matrix; actual DMX/RDM/MIDI/OSC/DJ/capture/display behavior; M2/macOS 12 and media quality; signed clean-machine install/update/release; venue acceptance. |

Already integrated subsets must not be put back on the implementation queue:
canonical own-property rejection; bounded thumbnail requests/worker ownership;
video seek/catch-up regression repair; routing/VJ checker repair; reachable retry
controls; and the final-DMG validation/upload gate. See their domain QA records.
The successful Mac run is 34188473755 at `c4144d0`, not a run of today's main.
It does not establish M2 hardware, macOS 12 execution, Developer ID or notarization.
The ledger's older Windows-only/macOS-deferred wording needs scope reconciliation
with that later Mac work; Linux and public-distribution goals are not silently
added or removed by a passing development DMG.

Next acceptance boundary: isolate operator application data and test-project state
before attempting successful real-WebView import/render/retry. A separate browser
user-data folder alone does not isolate backend app data, machine bindings or
recovery receipts. Do not call raw thumbnail functions on an unrelated operator
project or combine separate native and browser passes into a full-flow claim.

## Executed evidence

Windows, existing default application features, exact maintained MSVC 14.44.35207
linker pin and PATH-first check. Local FFmpeg/FFprobe paths are explicit.
`cargo test -p syndocal --release --locked real_file_thumbnail -- --test-threads=1`:
4 passed / 0 failed / 0 ignored, 1804 filtered out; test runtime 0.23s.
All four tests ran. They cover the asset-thumbnail implementation and shared
layer-preview rendering method; they do not invoke the real WebView, import UI,
Tauri admission table, Layer command, or post-render project authority barrier.
The separate existing authority/admission regressions remain relevant but are not
combined into a claim that this is a complete operator-to-native success test.
`check:release`: exit 0 on this tree.

Logs and generated source patterns: `target/qa/thumbnail-real-file-20260908/`.
No EXE was rebuilt or launched; compiling the Rust test executable is not a new
application artifact. The product source, App bindings, fixed layout, Mac gate,
ASIO/NDI policy, version, installed app and old worktrees remain unchanged.
One broad combined ledger/source-read request was service-denied before execution;
its output is not evidence. The actual test/build commands above executed normally.
No Luna/Codex delegation occurred. This test-only checkpoint has author self-review,
not an independently reviewed product behavior change.

The broader `cargo test -p syndocal --release --locked thumbnail -- --test-threads=1`
then passed 21 / failed 0 / ignored 0 (1787 filtered out), including the four new
normal cases and existing ownership, cancellation and authority regressions.
The two Cargo logs contain zero Rust warning diagnostics. This is not a measured
pre-change warning delta or a full supported-platform warning matrix.
`check:completion-ledger`, `check:q1-q4-ledger` and `git diff --check` also passed.
The video fixture was separately inspected with FFprobe: MPEG-4, 96x64, 2 fps,
2 frames, 1.0 second, 1344 bytes, no audio stream. No physical device input was used.

This is a tests-and-documentation integration unit. Its four owned files are the
existing native test module, generated MP4, fixture recipe, and this checkpoint.
It adds no application startup, decode-loop, UI or transport work. Integration of
these tests does not close MEDIA-DERIVED-001 or the outstanding native UI success
and cancellation paths described above.

## Current-main source re-review — 2026-09-12

The existing retry and controller implementation was independently re-read at
current `main` source HEAD `d62eee51bdcd22050c0195a91cb451e54a6cc1d9`.
`thumbnailReadRetry.ts` admits only the two explicit transient native errors,
checks batch ownership before the first read, after the wait, and before the
single retry, and treats the second failure as terminal. The controller keeps
layer and asset lanes independently bounded, retires active work on reset or
dispose, and allows an unchanged missing asset to be explicitly retried without
continuing a retired batch. No unbounded retry, stale publication, or ownership
defect was found; no product code change was required.

| Recheck | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS — bounded retry, terminal rejection, stale retirement, explicit recovery/cache reuse, lane ownership and view bindings |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS — normal success, exact abort, stale-result rejection, cancellation failure, malformed/foreign ticket rejection |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — 50 Open + 8 Deferred authority rows preserved |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — 32 Q1 rows, 58/58 flow markers, master mirror parity |

The real-file missing → Retry → restore → native recovery flow was rerun on
current `main` after the file-moving operation was explicitly authorized. The
current source HEAD was `d62eee51bdcd22050c0195a91cb451e54a6cc1d9`; the exact
launched EXE was `target/release/syndocal.exe`, version `1.2.0-alpha.69`,
64,569,856 bytes, SHA-256
`71E861A8A716F36D09178DCF3694B86A835EA3BAF83787E86369ED0A06AF5819`.

Run evidence is in
`target/qa/native-thumbnail-failure-recovery-20260912-01/`. The isolated
application had one responsive maximized window, Standby ownership, both
lighting/video domains denied, zero physical-output enable commands, and zero
raw project-mutation commands. The test-owned PNG copy
(`1561115A3B4A825BDC64C3FC2633D4736C1EDCD8CAB7EBF1FE9AC6313FA19460`, 253
bytes) was moved to `materials/held`, produced a real missing-file failure with
an enabled `Retry Thumbnails` control, was restored byte-for-byte, and Retry
recovered a 160×90 four-quadrant image. The healthy MP4 remained displayable;
only the missing PNG asset was reread after Retry. The PNG case is therefore a
current-main real-WebView success.

An independent MP4 case was attempted in the same isolated run. The
test-owned MP4 copy (`56FCBB1725220525F53E87CCCAF6EBA670C4888C35AEEF05C381E41193648D69`,
1,344 bytes) could not be moved while the native video handle was held:
Windows returned `EBUSY` before the missing-file state was reached. The source
copy remained present and its final hash matched. This is an environment/native
handle boundary, not a successful MP4 missing→Retry→recovery proof, so the
MP4 case remains open and the overall real-file acceptance is limited to the
PNG case.

To separate the path from the native handle, the same two-case probe was
repeated as run `target/qa/native-thumbnail-failure-recovery-20260912-02`,
with both test-owned materials staged under an ASCII LocalAppData path outside
OneDrive. PNG again completed missing→Retry→recovery; MP4 again returned
`EBUSY` on the move before missing state. The external staging was removed
after its hashes and empty held directory were verified; the raw run report is
retained. This reproduces the MP4 boundary independently of the OneDrive path.

The app exited through the owned-process cleanup path and the probe listener
count was zero. The test-owned held directory was empty after cleanup. The
The normal-profile digest changed only for `agent-bridge-v1.json` in each run
(`1075de22116ccdaf19465802e9b7c4759a727b278660382f69e729788f7695e4` →
`6d82fea3f788468001395e51606aab04a11a08c988af2391f4b78dd64f98ade9` in run
01, then `6d82fea3f788468001395e51606aab04a11a08c988af2391f4b78dd64f98ade9`
→ `a5e2b5ddea04d08544d9a5991ad997e0405705de7d697c4f4b6e1d57b2e93227` in
run 02); the other recorded files were unchanged. The descriptor was not
repaired or restored, and its cause is not attributed by this test.

This evidence does not close the broader `MEDIA-DERIVED-001` row, native UI
cancellation acceptance, physical device, Mac, signing, publication, or
product-wide completion.
