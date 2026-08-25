# Syndocal historical release decision — 2026-08-12

> This decision records the `1.1.0` checkpoint and is historical evidence, not
> current authority. Pushed HEAD
> `07a9b75f25e74e58739be251aad3b3f1e98986f7` metadata remains
> `1.2.0-alpha.11`, while the dirty uncommitted worktree synchronizes
> in-progress `1.2.0-alpha.12` metadata that is not yet a reproducible
> checkpoint or native artifact. The performance is 2026-08-30; development,
> acceptance, and show preparation have a separate 2026-08-29 completion
> deadline.
> The controlled source-only DJ route, local-only non-default show-ASIO route,
> and current cleanup allowlist are defined by the current near-show and domain
> gates; this historical decision does not authorize an installer, normal
> updater, legacy adapter, ASIO fallback, or deletion. No show-ASIO artifact is
> accepted, DJ hardware remains 0/12 open, and only the 44.1 kHz bridge-only
> 3,600,031 ms ASIO run has passed; formal matched 48 kHz/native operator/
> recovery/latency acceptance remains open. Cleanup commit `7ee3b8f` proves only
> the reviewed guard; its sole current candidate is `target/debug/incremental`,
> no Apply has run, and reclaimed bytes are 0. The alpha.10 pause request was
> rescinded before promotion on 2026-08-25. Use `AGENTS.md` and
> `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` for current work. The
> post-alpha.10 snapshot remains checkpoint evidence only.

## Decision

Syndocal is **software release-candidate ready** for its documented product
boundary: a PC lighting/VJ application that uses external Art-Net visualization
instead of an embedded 3D visualizer. This is not yet a production/venue or
public-distribution sign-off. Those decisions require the external evidence
listed below.

## Software evidence at this 2026-08-12 decision checkpoint (historical)

- Daslight COLOR MAPPINGS routing is 68/68 with strict fail-closed boundaries
  documented per effect.
- The locked engine suite is green (608 passed, 2 intentionally ignored manual
  gates at the latest full run before this decision audit).
- `pnpm --dir app run check:release` passes for Syndocal 1.1.0 metadata.
- `pnpm --dir app run check:release-ui` passes the complete desktop contract,
  1920/2048/1366/1280 browser matrices, English/Japanese VJ and Timeline
  surfaces, and real native 1920 acceptance.
- Native acceptance proves maximized client 1920x1032, F11 fullscreen
  1920x1080, and Escape restoration to exactly 1920x1032.
- Japanese static UI coverage is 3131/3131 with no unprotected user-data label.
- Native close no longer uses the browser `window.confirm` surface. A fresh
  engine snapshot is checked with a bounded fail-closed timeout: clean and
  inactive closes directly, while dirty-only, runtime-only, combined and
  unknown-output states receive distinct localized operator copy. Native QA
  confirmed direct clean close, safe keep-open cancellation, and successful
  discard-and-close of a disposable unsaved edit; the static adversarial gate
  covers timeout, duplicate-close races, cleanup and exactly one approved
  native close callsite.

## External acceptance checklist

Every applicable row remains a release blocker until its evidence cell contains
the named artifacts and the result is signed by the rig or release owner.

| Area | Procedure | Pass criteria | Evidence to attach | Current status / owner |
| --- | --- | --- | --- | --- |
| Art-Net and sACN | Connect the release executable through the actual venue switch/VLAN to one node and fixture per protocol. Exercise normal looks, blackout/release, reconnect, and the representative multi-universe show while capturing packets and fixture output. | No wrong-universe, sequence gap or stale look; blackout/release and reconnect are correct; zero send failures; Engine jitter p99 <= 1,000 us, command queue p99 <= 1,000 us and command-to-DMX p99 <= 5,000 us while sustaining the 44 Hz engine cadence. | App/version/hash, node/fixture/firmware, topology and universe map, packet capture, fixture video, telemetry export, latency table, operator/date. | **Pending — rig owner.** Authority: `qa/M4_IO_VALIDATION.md`, `qa/M5_RELIABILITY_VALIDATION.md`. |
| Daslight A/B parity | Run Daslight and Syndocal sequentially from the exact same hashed `.dvc`. Capture the same named scene and authored time point from Daslight DMX Levels (and licensed Art-Net cadence/sequence when available), then capture Syndocal with `artnet-monitor.mjs` and compare with `artnet-compare.mjs`. For dynamic FX, capture at least 30 seconds and record the allowed phase error in ms and period error in percent before either run; a blank tolerance is a failed gate. | Static named-time frames pass all 512 bytes exactly. Dynamic FX remain within both predeclared tolerances for the full capture and have zero ArtDMX sequence gaps. Do not claim byte-identical Daslight parity before both requirements pass. | DVC path/size/SHA-256, Daslight version/licence/interface, scene/time/phase rule, numeric phase/period tolerances, Daslight screenshots or packet JSON, Syndocal JSON, comparator JSON, operator/date. | **Pending — parity owner.** Authority: `qa/ARTNET_EXTERNAL_VISUALIZER_ACCEPTANCE.md`. |
| Commercial external visualizer | On a second computer, bind a commercial visualizer to the intended physical adapter and universe, allow UDP 6454, patch the representative rig, then drive static and changing looks from the release executable across the real LAN for at least 20 minutes. Restart both sender and visualizer during the run. | The second computer receives the expected universe and 512-byte ArtDMX stream with zero sequence gaps, renders the expected fixture addresses/looks throughout 20 minutes, and recovers after each restart. Same-host loopback or self-NIC targeting is not acceptance. | Sender/receiver IPs and adapters, firewall rule, visualizer/version/project, fixture patch, 20-minute packet capture, rendered screenshots/video, reconnect result, operator/date. | **Pending — visualizer/rig owner.** Authority: `qa/ARTNET_EXTERNAL_VISUALIZER_ACCEPTANCE.md`. |
| USB/serial DMX and RDM | Test the published support list explicitly: Enttec USB Pro, DMXKing ultraDMX and Enttec Open DMX for output; Art-Net RDM gateway and USB Pro for RDM. Repeat output/reconnect on each applicable device. Discover at least two RDM fixtures, force a collision, remove/re-add one fixture, and exercise ACK, NACK, timeout and queued/overflow responses. | Correct DMX after reconnect; no hung worker; discovery finds each UID once; collision resolves; inventory add/remove and response handling match the physical rig. Existing Open DMX evidence is accepted only for that interface. Any unavailable device must be marked Not tested, not silently N/A. | Published support matrix, interface/gateway/fixture IDs and firmware, serial settings, discovery duration, UIDs/PIDs, collision count, response/timeout log, fixture video, operator/date. | **Pending — rig owner.** Authority: `qa/M4_IO_VALIDATION.md`. |
| MIDI and OSC | Use the production MIDI input/output path for note, CC, feedback, clock and MTC. From a physical TouchOSC device on the venue LAN, exercise mapped controls and returned state across reconnect. Before testing, record the show's maximum p95 and p99 control-to-engine/feedback round-trip in ms; blank limits are a failed gate. | Every mapping fires once with correct value/direction; feedback does not loop; clock/MTC remains locked; reconnect recovers without remapping; measured p95 and p99 stay at or below both preapproved limits. | Controller/device/app versions, mapping export, numeric p95/p99 limits, timestamped input/output log, measured p50/p95/p99 table, screen/video capture, operator/date. | **Pending — operator.** Authority: `qa/M4_IO_VALIDATION.md`. |
| Camera, media and GPU output | On the target venue GPU, run representative camera, HAP Q Alpha/HAP R, NDI and Spout inputs; multi-layer seek/reverse/loop; Preview and Program; display/NDI/Spout outputs; projection mapping; ISF stack; and recording for one hour. Include unplug/replug or sender restart. Before the run, record numeric show limits for output deadline misses, GPU temperature and A/V sync. | No unrecovered source/output loss or device-loss fault; zero unaccounted dropped frames; output and recording remain visually correct; measured deadline misses, temperature and A/V sync stay within the predeclared numeric show limits. A blank limit is a failed gate. | GPU/driver/device/media hashes, predeclared numeric limits, output resolution/rate, diagnostics export, one-hour frame/thermal log, `ffprobe` output, reconnect and projection before/after captures, operator/date. | **Pending — video/rig owner.** Authority: `qa/M4_IO_VALIDATION.md`, `qa/VJ_COMPETITIVE_AUDIT.md`. |
| Audio Reactive and Auto VJ | With the default WASAPI build, run representative show audio through at least two physical capture devices, exercise marker-to-pixel response, deterministic Auto VJ decisions, unplug/replug and one hour of combined VJ output. | Analysis p99 < 6.66 ms and max < 16.65 ms; zero overruns; capture-to-engine p95 <= 40 ms; loss-to-zero <= 250 ms; same seed/feature frames reproduce the expected Auto VJ action sequence; no unrecovered hot-plug fault. | Device/driver/rate/buffer, source and seed hashes, worker/capture/engine percentiles, physical marker video and measurements, one-hour log, Auto VJ action digest, operator/date. | **Pending — audio/video owner.** Authority: `qa/AUDIO_REACTIVE_VJ_ACCEPTANCE.md`. |
| Optional ASIO artifact | The normal installer remains WASAPI-only. If a separately named ASIO build is proposed, resolve its distribution licence/artifact obligations, then test at least two vendor drivers, reset/resync/xrun/no-callback and unplug/replug, plus matched one-hour ASIO/WASAPI and five physical-latency trials. | Distribution approval is signed; overrun 0; callback p99 < 20% and max < 50% of hardware buffer duration; capture-to-engine p95 <= 40 ms; loss-to-zero <= 250 ms; terminal faults require explicit restart; two vendors pass. Otherwise the ASIO artifact must not be published. | Licence decision/notices/source obligations, exact ASIO artifact hash, two-driver matrix, callback/xrun/fault logs, one-hour reports, five-trial latency table, operator/date. | **Pending and excluded from the default installer — release/audio owner.** Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`. |
| Venue reliability and failover | Run the release build for one hour with the actual show, network, controls and outputs. Where standby is required, sync A to B, hard-disconnect or power off A, take over on B, restore A, and verify fencing/recovery. | Zero output-send failure, queue push failure, drain-limit hit and unrecovered worker fault; no show-state corruption; Engine jitter p99 <= 1,000 us, command queue p99 <= 1,000 us and command-to-DMX p99 <= 5,000 us; heartbeat becomes stale at approximately five seconds; failover emits zero overlapping DMX/video frames and restores the latest confirmed generation. | Exact executable/project hashes, `target/qa` soak JSON, diagnostics package, topology, failure timeline/video, takeover/recovery times, duplicate-frame count, A/B logs, operator/date. | **Pending — show owner.** Authority: `qa/M5_RELIABILITY_VALIDATION.md`. |
| Cross-platform package scope | Before publishing an artifact, run the real package on a clean host for every advertised target: Windows NSIS/MSI, macOS app/DMG and Ubuntu 22.04 deb/AppImage. Install, launch, open/save a project, upgrade and uninstall; verify bundled FFmpeg/licence files and platform I/O boundaries. Ubuntu uses a retained-package manual upgrade for this release; it is not covered by the Tauri signed-updater channel. | Every advertised artifact installs, remains alive, opens/saves the compatibility project, upgrades and uninstalls cleanly with no undeclared host dependency. A platform without this evidence must be omitted from the public support matrix. Syphon remains unsupported until its separate macOS implementation boundary is closed. | Host/OS/architecture, artifact hash, install/launch/project/manual-upgrade/uninstall logs and video, dependency scan, support-matrix sign-off, tester/date. | **Pending — release owner.** Authority: `qa/M6_RELEASE_VALIDATION.md`, `qa/M4_IO_VALIDATION.md`. |
| Signed install and update | On clean Windows and macOS machines, install signed version N, publish signed N+1 through the production HTTPS endpoint, update, relaunch and verify project compatibility. Then reinstall the retained signed N artifact, restore the automatic pre-update project backup and verify the N project/version before returning to N+1. Repeat negative tests with modified artifact, wrong signature/key and non-HTTPS metadata. | Authenticode and Developer ID/notarization validate; N to N+1 succeeds; the explicit N reinstall/backup restore path succeeds; data remains usable; every tampered/wrong-key/non-HTTPS case is rejected; signing keys are access-controlled and recoverable. | Certificate/notarization output, artifact and manifest hashes/signatures, endpoint response, install/update/reinstall/restore logs and video, restored project hash/version, key-custody sign-off, tester/date. | **Pending — release owner.** Authority: `qa/UPDATE_RELEASE_RUNBOOK.md`. |
| Legal and publication | Generate the final dependency/license inventory, resolve incompatible obligations, tag the reviewed commit, build immutable artifacts, publish checksums and operator-facing release notes, and verify download contents against the tag. | Legal owner signs the inventory; tag, source, artifacts and checksums correspond exactly; release notes name supported platforms/hardware and every remaining boundary; no secret or private specimen is shipped. | Signed license report, tag/commit, CI/build provenance, artifact hashes, release-page capture, final contents audit, legal/release-owner/date. | **Pending — legal/release owner.** |

## Go/no-go rule

- **Internal demo / controlled rehearsal:** GO from the software state once the
  exact release executable is rebuilt and launched successfully.
- **Venue show:** NO-GO until every applicable hardware and one-hour reliability
  row above has attached evidence for the actual rig.
- **Public installer/update release:** NO-GO until platform signing, clean-machine
  install/update/rollback and legal checks are complete. The signed Tauri
  updater is scoped to Windows and macOS for this decision; Ubuntu, if
  advertised, uses the separately accepted manual deb/AppImage upgrade path.

No internal browser, loopback, synthetic performance or native-window result is
to be reported as a substitute for physical fixture, network, venue GPU or
distribution acceptance.
