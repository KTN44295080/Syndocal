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

## Large show

`large_show_loads_200_fixtures_across_8_universes_and_100_cues` covers 200 fixtures, eight complete 512-slot previews, 100 cues, and 20,000 cue targets. The focused test completed in 0.06 seconds with no queue failures or drain-limit hits.

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

## M5 conclusion

The one-hour soak, large-show load, `.sdc` compatibility contract, storage edge cases, and real process-kill recovery all pass. Physical DMX timing remains an external M4 hardware evidence item rather than an M5 software blocker.
