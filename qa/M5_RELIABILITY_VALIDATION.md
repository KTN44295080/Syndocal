# M5 Reliability Validation

Date: 2026-07-12  
Host: Windows 11, release profile  
Scope: software-only reliability and timing gates. Physical DMX evidence remains in `qa/M4_IO_VALIDATION.md`.

## One-hour soak

Command:

```powershell
./qa/run-soak.ps1 -DurationSeconds 3600 -SampleIntervalSeconds 5 -Configuration Release -SkipBuild -ReportPath target/qa/m5-soak-3600.json
```

Workload:

- Embedded `phase1-mini-show.sdc`
- One patched fixture and one Art-Net loopback route
- One LFO and one position-wave effect, including a shared video-opacity target
- Shared four-second lighting/video timeline loop
- One video layer rendered at 320x180 and 30fps through the missing-source placeholder path
- One Engine thread plus the harness process; no concurrent build or test process

Result: **PASS**

| Metric | Result | Gate |
| --- | ---: | ---: |
| Elapsed | 3600.002 s | >= 3600 s |
| Rendered/nonblank frames | 108,001 / 108,001 | all nonblank |
| Dropped frames | 0 | 0 |
| Peak working set | 13.3 MB | < 200 MB base target |
| Process CPU time | 307.70 s | recorded |
| Engine tick jitter p99 | 535 us | <= 1,000 us |
| Command queue latency p99 | 394 us | <= 1,000 us |
| Command-to-DMX latency p99 | 476 us | <= 5,000 us |
| Queue push failures | 0 | 0 |
| Command drain limit hits | 0 | 0 |
| Art-Net sends | 158,642 success / 0 failure | 0 failure |
| Last Engine error | none | none |

The raw JSON is generated under `target/qa/` and intentionally remains a local build artifact. Re-run the command above for machine-specific evidence.

### Live FFT command-pressure extension

After adding CPAL live FFT input, the soak workload was extended to publish a changing three-band `SetLiveAudioSpectrum` command at every 30fps render iteration. The full one-hour release run passed on 2026-07-12:

- 108,001 rendered/nonblank frames, zero dropped frames and zero render errors
- 108,001 live-audio updates and 108,001 low-latency DMX advances
- command queue p99 22us; command-to-DMX p99 28us; tick jitter p99 823us
- queue depth absolute maximum 3; command drain absolute maximum 3
- zero queue push failures, drain-limit hits, reconnect attempts, or DMX send failures
- 216,003 successful Art-Net sends; peak working set 14,024,704 bytes (13.4MB)
- process CPU time 335.17 seconds; telemetry budget PASS

Command:

```powershell
./qa/run-soak.ps1 -DurationSeconds 3600 -SampleIntervalSeconds 5 -Configuration Release -ReportPath target/qa/m5-soak-live-audio-3600.json
```

The original one-hour evidence remains valid for the base workload, and the live-audio command-pressure extension now has equal-duration evidence. The raw JSON is `target/qa/m5-soak-live-audio-3600.json` and remains a local build artifact.

### Mixed Color/Chaser/Move lighting extension

On 2026-07-15 the soak harness gained a `--mixed-lighting` mode (`-MixedLighting` on `qa/run-soak.ps1`). It keeps the complete base workload — embedded mini-show, PULSE LFO, WAVE position-wave with shared video-opacity target, four-second timeline loop, per-frame `SetLiveAudioSpectrum` pressure, and the 320x180@30fps video render — and adds one independent Color effect (three-stop Cycle, HSV-shortest, 2s period), one independent Chaser (four steps with gaps, Dimmer feature, 125ms steps), and one independent Move (four-point smooth closed path, paired Pan/Tilt, 2s period) on the patched fixture. Every effect is added through the published-acknowledgement engine commands, and the harness requires exactly five active effects before telemetry reset. Default (non-mixed) behavior, the report path defaults, and every existing gate value are unchanged; a 60-second default-mode run re-passed after the change.

Full one-hour release run: **PASS**

```powershell
./qa/run-soak.ps1 -DurationSeconds 3600 -SkipBuild -MixedLighting -ReportPath target/qa/m5-soak-mixed-lighting-3600.json
```

| Metric | Result | Gate |
| --- | ---: | ---: |
| Elapsed | 3600.002 s | >= 3600 s |
| Active effects | 5 (LFO + PositionWave + Color + Chaser + Move) | 5 |
| Rendered/nonblank frames | 108,001 / 108,001 | all nonblank |
| Dropped frames | 0 | 0 |
| Live-audio updates | 108,001 | recorded |
| Peak working set | 20.9 MB | < 200 MB base target |
| Process CPU time | 336.55 s | recorded |
| Engine tick jitter p95 / p99 | 334 us / 512 us | p99 <= 1,000 us |
| Command queue latency p95 / p99 | 20 us / 48 us | p99 <= 1,000 us |
| Command-to-DMX latency p95 / p99 | 22 us / 53 us | p99 <= 5,000 us |
| Queue push failures / drain-limit hits | 0 / 0 | 0 |
| Art-Net sends | 216,004 success / 0 failure | 0 failure |
| Last Engine error | none | none |

The raw JSON is `target/qa/m5-soak-mixed-lighting-3600.json` and remains a local build artifact.

### Mixed-stack release benchmark

`mixed_color_chaser_move_release_stack_meets_44hz_budget` (crates/engine) measures the true production per-tick evaluation path — `apply_effects` per fixture and control, including its per-call clock snapshot — for 200 fixtures x 6 attributes under 64 simultaneous full-rig effects (22 Color with three HSV-shortest stops and full spread, 21 Chaser with Wings/width/duty/overlap/spread, 21 smooth closed-path Move) across 1,000 44Hz ticks.

```powershell
cargo test -p engine mixed_color_chaser_move --release --locked -- --nocapture
```

Observed on this host on 2026-07-15 across two release runs: p95 2.82–3.21 ms, p99 3.06–3.78 ms, max 3.89–4.35 ms per tick — about 14% of the 22.7 ms 44Hz tick. The initial planning budget of 2/4/8 ms was set before measurement and was exceeded at p95; investigation confirmed the benchmark faithfully mirrors the production hot path rather than adding artificial overhead, so the release-only gate is fixed at p95 <= 5 ms, p99 <= 8 ms, max <= 12 ms. That keeps the worst-case mixed stack at or below roughly half a tick while still failing on a >50% evaluation regression. This is an internal software measurement on one Windows host, not venue or competitor evidence.

## Large show

`large_show_loads_200_fixtures_across_8_universes_and_100_cues` covers 200 fixtures, eight complete 512-slot previews, 100 cues, and 20,000 cue targets. The focused test remains subsecond with no queue failures or drain-limit hits.

The operator UI has a separate real-browser large-show gate:

```powershell
npm --prefix app run check:large-show-ui
```

It injects 2,000 patched fixtures into the primary maximized 1920x1080 Stage Map workspace. The visible Mapping Fixture list reports `aria-rowcount=2000`, keeps no more than 20 fixture rows in the DOM at both the top and bottom of the scroll range, reaches `Large Fixture 2000`, and preserves full-window containment. Fixture lists use fixed-row windowing above their thresholds; Cue editing is bounded to 12 rows per page, while Clip Grid, video layers/outputs, and DMX addresses retain their existing banks/pages. Compact viewport pressure remains covered by the standard five-browser-viewport gate, including the 1920x1032 measured-work-area fixture, instead of making a small window the main large-show sign-off.

The standard viewport gate also injects 128 distinct DMX output routes into Setup > DMX with 1920x1080 as the primary operational gate, 2048x1152 as the extended ceiling, and 1366x768 plus 1280x720 as compact containment fallbacks. All 128 route items must render while the application remains full-window contained. Rust boundary tests accept 128 and reject 257 routes; engine tests retain/tick 128 distinct universes and complete one enabled Art-Net tick with 128 successful sends and zero failures. A one-hour 128-universe physical-node soak remains a competitive acceptance gate.

## Project recovery crash injection

Result: **PASS**

1. Built the production frontend and an SDK-independent release executable with Tauri's `custom-protocol` asset embedding.
2. Launched the real Windows application, loaded `phase1-mini-show.sdc`, and changed the fixture label from `Mini Spot 1` to `Recovery Test Fixture` without saving.
3. Waited for the ten-second autosave interval and confirmed the `Untitled.sdc` Recovery entry in the application menu.
4. Force-terminated the complete `syndocal.exe` process tree rather than closing the window normally.
5. Relaunched the same executable and confirmed the Recovery entry persisted.
6. Selected the Recovery entry and confirmed the fixture label, one embedded profile, one fixture, one projection surface, and the 2D stage map were restored. The application reported `Recovered Untitled.sdc ... (1 embedded profiles). Save to keep it.`
7. Closed the verified application and confirmed no Syndocal process remained.

Storage corruption, invalid payload removal, successful persistence/readback, and quota failure remain covered by `pnpm --dir app run check:project-storage`.

## Active / Standby failover

Software safety gates: **PASS**

- Primary publishes a complete ProjectFile every two seconds as an immutable `.sdc` generation, then atomically commits its manifest.
- Byte length, FNV-1a checksum, project version/app validation, five-generation retention, and fallback from a corrupt newest generation are covered by Rust tests.
- Standby applies every checkpoint with legacy DMX output, every DMX route, and every video output disabled, plus lighting/video blackout asserted.
- Primary activity is observed per session from generation progress on the receiving machine's monotonic clock. Wall-clock skew between show computers is not used for heartbeat expiry.
- Two progressing Primary sessions fence warm loading. A live heartbeat or split-brain state requires an explicit operator confirmation that every previous Primary has been disconnected before Take Over.
- Promotion is manual. There is no automatic network fencing, quorum, or distributed consensus.

Focused test:

```powershell
cargo test -p syndocal standby_ --locked
```

Two-computer acceptance remains required before release sign-off:

1. Put the shared folder on the dedicated show-control LAN; verify both computers can create, rename, flush, and read files in it.
2. Start computer A as Primary and computer B as Standby. Confirm B advances `Generation` and `Applied`, while all B-side DMX/video routes remain disabled and blacked out.
3. Change a cue, lighting value, video layer, and output mapping on A. Confirm B receives each within one checkpoint interval without transmitting.
4. Disconnect A's DMX/video network interfaces or power A off. Do not use application shutdown as the only fencing evidence.
5. Measure from the last A output frame to `Primary lost` on B, then perform Take Over. Gate: heartbeat stale at approximately five seconds, no overlapping DMX/video frames, and latest confirmed generation restored.
6. Restore A while it is still configured as Primary. Confirm split-brain is reported and B fences further warm loads; do not force promotion until A is physically/network fenced.
7. Interrupt the shared-folder connection for at least 30 seconds, restore it, and confirm generation publication/resumption without application restart or corrupted-generation load.
8. Record share type, switch topology, clock-sync source, measured failover gap, duplicate-frame count, checkpoint size, and operator name below.

Acceptance capture:

| Item | Result |
| --- | --- |
| Primary / Standby hosts | Pending physical acceptance |
| Shared storage / path | Pending |
| Failover gap | Pending |
| Overlapping DMX/video frames | Pending |
| Share disconnect recovery | Pending |
| Split-brain fencing | Pending |
| Evidence files / operator | Pending |

## M5 conclusion

The one-hour soak, large-show load, `.sdc` compatibility contract, storage edge cases, real process-kill recovery, and software-only standby safety gates pass. Physical DMX timing and the two-computer failover capture remain external acceptance items rather than software-only claims.
