# Spout full-resolution composition cadence

Base: `codex/syndocal-v1.2` at `80bbdb42dee07b8a6f8e81e4fb82f79e8fc2dfc6`.

## User acceptance and baseline

The user reports that opening the original Unity4K project restored video, and
that Syndocal's moving-head beams now follow the movement. Read-only native
inspection on PID99440 confirms both authored Spout outputs (3840x2160 and
1920x1080), Both/Ready output ownership, and no ownership error. The earlier
zero-output recovery image remains distinct from the unchanged original file;
the exact earlier UI recovery action was not established.

Live playback with opt-in sender diagnostics identifies composition before send
as the dominant cost. Weighted aggregate of non-keepalive windows:

| Output | Windows / renders | Render mean | Loop cadence mean | Send-authority mean |
| --- | --- | --- | --- | --- |
| Foreground 3840x2160 | 83 / 622 | 282.222 ms | 286.815 ms | 2.741 ms |
| Background 1920x1080 | 88 / 2432 | 71.460 ms | 73.934 ms | 1.538 ms |

Evidence: `target/qa/snapshot-cleanup-20260905/spout-timing-99440-{1,2}.log`
and `spout-cadence-before-summary.json`. These are sender-side measurements,
not receiver delivery/FPS proof. SDK frame/fps counters are not used as delivery
evidence. The sources are 30000/1001 fps, with 5:2 content fitted to 16:9 outputs.

## Bounded change and remaining acceptance

The existing identity compositor optimization requires an unchanged transform
and equal source/output sizes. The 5:2 fit therefore uses the general CPU pixel
sampler. Optimize unrotated Normal, opacity-one layers with default color/FX by
sharing horizontal sample coordinates and avoiding identity pixel adjustments;
retain the original general path for other artistic conditions. Preserve full
resolution, crop, pixel-center sampling, and alpha blending. No output ownership,
Spout protocol, Unity surface, or persisted project changes are in scope.

## Focused validation

The implementation owner ran the exact-MSVC wrapper
`node target/qa/recording-atomic-20260905/run-native.mjs` for video crate tests:
`axis_aligned_composite` (3 PASS, benchmark ignored), `cpu_compositor_`
(12 PASS), and release `axis_aligned_composite_release_cost` with `--ignored`
(1 PASS). First-party compiler warnings: baseline 0 / current 0 / delta 0.
Results were returned in tool output, not a saved compiler log.

Exact invocations after the wrapper prefix:

```text
cargo test -p video --locked axis_aligned_composite -- --nocapture --test-threads=1
cargo test -p video --release --locked axis_aligned_composite_release_cost -- --ignored --nocapture --test-threads=1
cargo test -p video --locked cpu_compositor_ -- --nocapture --test-threads=1
```

Filtered counts respectively 170 / 173 / 162; all exit 0. No physical device
or Unity output is opened by these tests.

The release benchmark measures only blending an opaque 5:2 source into a full
16:9 destination, eight iterations per condition. Median old/new cost:
3840x2160 225.252 / 4.091 ms; 1920x1080 56.738 / 1.299 ms.
It does not establish full-renderer cost or Unity delivery cadence.

Root inspected the stable production diff. An independent read-only reviewer
accepted sampling/crop/alpha parity, bounded eligibility, preserved validation,
and differential test coverage, with no revision requests.

## Native integration checkpoint

`pnpm --dir app tauri build --no-bundle` PASS (release compilation 3m45s,
frontend build 8.79s). Exact checkout PID99440 was verified and stopped by the
maintained wrapper; exact MSVC 14.44.35207 pin and PATH-first checks passed.
First-party warnings baseline 0 / current 0 / delta 0. Log:
`target/qa/snapshot-cleanup-20260905/spout-compositor-native-build.log`.

Launched exact checkout executable PID83812 and verified one responsive,
maximized Syndocal main window. SHA256:
`703C076EEAC4D562B93134B6813AA26F31261C42D495E82D48451BBE6431A08E`.
Evidence: `spout-compositor-launch.json` in the same directory. Temporary
localhost CDP38479 and bounded Spout timing remain enabled for read-only
measurements; no UI, project-open, output-enable or playback action was automated.

User was asked to open the original Unity4K file with Ctrl+O, enable Spout and
play the same timeline. Full-renderer before/after timings and Unity smoothness
remain unverified. After acceptance restart without diagnostic environment.
The separate direct Safety BlackOut lifecycle boundary recorded in
`SYNDOCAL_UNITY_RUNTIME_FIXES_2026-09-05.md` remains outside this optimization.
Preserve unrelated `app/scripts/check-viewport-containment.mjs`.

## User acceptance

After checkpoint `dbc7657`, the user reported success in Unity and explicitly
accepted the video issue as resolved. Live-only windows (zero keepalive frames)
on PID83812 show full-renderer foreground mean 60.279 ms (780 renders/25 windows)
and background 14.386 ms (2574 renders/25 windows), versus baseline 282.222 and
71.460 ms. Loop cadence means are 65.250 and 19.528 ms. This is a substantial
improvement and user visual acceptance, not a claim of 30 fps foreground
delivery. Evidence: `spout-cadence-after-live-only-summary.json` and raw
`spout-timing-83812-{1,2}.log` in the same QA evidence directory. The broader
`spout-cadence-after-summary.json` includes mixed keepalive windows and must not
be interpreted as active-playback cadence.

The video-stutter issue is closed at the user's requested boundary. Next bounded
work is the already-recorded direct BlackOut/Spout lifecycle defect; do not resume
further video-performance changes without new evidence or a user request.
