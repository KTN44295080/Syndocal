# Unity integration runtime follow-up

Base `53f3998430b09492dd471b1b873aecd749367cc4`, branch `codex/syndocal-v1.2`.
This checkpoint owns Syndocal changes only. Preserve the unrelated dirty
`app/scripts/check-viewport-containment.mjs` and all Unity project edits.

## Live moving-beam direction

The user compared moving Art-Net fixtures in Unity with left-fixed cyan beams
in Syndocal. Mapping's base beam used programmer faders; its live update replaced
color/intensity but did not replace Pan/Tilt or invalidate the beam cache.

Live beam direction now uses the same output-DMX precedence as live color,
including fixture-relative addresses and 8/16-bit coarse/fine expansion. Without
DMX preview, it uses live fixture attributes/defaults. Single and multi-cell beams
update together. Pan/Tilt are in the cache signature; unchanged values reuse
geometry. Mounting transforms, emitted DMX and Unity assets are unchanged.

Evidence: `node app/scripts/check-mapping-live-movement.mjs` PASS (actual browser,
live Pan/Tilt with stale programmer values, fine-only change, 8-bit expansion,
preview precedence/removal, multiple segments); `check-mapping-beam-geometry.mjs`
and `check-mapping-beam-browser.mjs` PASS. `pnpm --dir app exec tsc --noEmit` PASS.
Independent review ACCEPT. Screenshot inspected at
`target/qa/mapping-live-movement-20260905/live-pan-tilt.png`.

## Project-open output disappearance

User reported opening the project alone caused both Unity video loss and
Syndocal's 'No video outputs configured'. Preview receives `snapshot.video.outputs`
directly, without filtering. The original Unity4K project retains two configured
Spout outputs. Autosave evidence from the same source path changed from two
outputs to zero across requests recorded around23:13–23:14.

Protected evidence copied read-only to
`target/qa/snapshot-cleanup-20260905/before-spout-open-loss.json` and
`after-spout-open-loss.json` (original backups1788617598300/1788617668267).
Runtime retirement removes the old pair. Project-open currently changes callback
authority before signaling worker stop; the resulting authority-loss error can
reject project publication after old output removal. The fix and native regression
are in progress; this is not yet a successful project-reopen claim.

The user also reported BlackOut release `OutputControl forbidden` and New blocked
by retained managed Faulted authority. Managed replacement runs its keepalive
fail-stop boundary before project publication; its failure fence can cause the
same unsignaled-worker error before native route retirement. Both boundaries must
be covered. Do not weaken retained genuine-failure handling to mask this race.

Independent review rejected early-stop-before-transition (Ready cannot issue a
teardown lease) and stop-with-existing-drain (worker teardown lease is held until
join, which follows transition return). The integration must reserve all-deny
and signal both workers under the gate, then join/retire before quiescence check
and new-project publication. This is now implemented with a dedicated retirement
reservation which cannot activate outputs before exact epoch/target/state and
zero-in-flight validation. The ordinary transition API remains unchanged.

Managed fail-stop reserves its all-deny fence and signals the workers before S0
changes its safety identity; the later fence step reuses that reservation. A
poisoned registry still requests all-deny and S0 and records the failure. Callback
or input fencing failures still join outputs and never publish the new snapshot.
Genuine SDK/teardown failures remain errors; no retained Faulted bypass was added.

A final independent concurrency review found that R4/status failure harvest can
run while managed fail-stop is joining outputs. Failure harvest now observes
worker completion first and then re-reads both stop signals; either planned stop
keeps the pair with its retirement owner. That owner still reports genuine prior
SDK errors on join. Two additional state/receipt tests cover partial/all stop
signals, normal/prior SDK failure and unchanged unplanned-failure harvest.

Independent production and test review ACCEPT. Exact-MSVC commands:

- `node target/qa/recording-atomic-20260905/run-native.mjs cargo test -p engine
  --locked output_ownership_retirement_tests -- --nocapture --test-threads=1`:
  3 PASS,1051 filtered (`output-retirement-gate-tests.log`).
- Same wrapper, `cargo test -p syndocal --locked project_retirement_
  -- --nocapture --test-threads=1`: final6 PASS,1704 filtered
  (`project-retirement-harvest-final-tests.log`); actual worker loop/drop/join with fake
  SDK, real-error preservation and callback failure cleanup/no publication.
- Same wrapper, `cargo test -p syndocal --locked fenced_project_replacement
  -- --nocapture --test-threads=1`:3 PASS,1705 filtered
  (`project-replacement-existing-tests.log`).
- `node app/scripts/check-backend-operator-contract.mjs`: PASS,510 commands.

Logs are in `target/qa/snapshot-cleanup-20260905`. First-party compiler warning
baseline0/current0/delta0 for these configurations. Fake-SDK tests use
`show_control=None`; they do not exercise the strict validator/automatic pair
callback or full ProductionKeepalivePorts with a native AppHandle. Real managed
Open/New and BlackOut release remain user-driven acceptance after restart.

Separate known boundary: direct S0 toggling while a strict Spout pair is published
still invalidates its captured safety identity. Removing that check alone is
unsafe: no-Follow needs explicit hard-black materialization, and keepalive/render
revocations need a precise safe resampling classification. This tranche does not
claim that independent engage/release cycle fixed. Current reported release
failure occurred in the already-Faulted run; retest and keep this remaining work
explicit. Do not weaken the validator as a shortcut.

Follow-up implementation and acceptance status for this boundary are now owned
by `SPOUT_SAFETY_BLACKOUT_2026-09-06.md`.

Spout cadence logs from PID93684 so far contain only paused black keepalive,
not live render frames. They do not establish the reported video-stutter cause.
After native integration, the first user test is opening the unchanged Unity4K
project, then live playback for beam direction and full-resolution Spout timing.

## Native checkpoint (2026-09-06 JST)

Final `pnpm --dir app tauri build --no-bundle`: PASS2m45s, first-party warnings0
(baseline0/delta0), exact MSVC pin/PATH-first confirmed. Earlier3m52s build passed
before the last harvest fix and is superseded. Evidence:
`target/qa/snapshot-cleanup-20260905/unity-runtime-fixes-final-native-build.log`.

Launched exact checkout executable PID74892: one responsive maximized Syndocal
main window. SHA256 `85F3ACBCF9CBD77105B2A8E33EC032CC1732D3D3F6DE35F7178362947305B3B2`;
proof `unity-runtime-fixes-launch.json`. Read-only native checks show ordinary
StartupDenied/epoch0 (persisted Standby), not evidence of an armed output; owner
registration succeeded, publication intent/ACK are null, status is ready.

Temporary localhost CDP38479 and opt-in Spout timing remain enabled for user-driven
acceptance; no observer wrapper is installed. First safe next action: user opens
the unchanged4K project, root reads output count/path/ownership, then user tests
output activation and playback. After acceptance, restart without diagnostics.
No app UI/project-open/playback/output-enable action was automated. Preserve the
original4K file; no project data was rewritten by this checkpoint.
