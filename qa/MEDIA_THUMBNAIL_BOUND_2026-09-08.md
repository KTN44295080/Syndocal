# Thumbnail request ownership — 2026-09-08

Base: `d317f132ad1764ba178b295a1b442017329e4e78`.
Branch: `chatgpt/macos-artifact-validation`.
Scope: the thumbnail subset of `MEDIA-DERIVED-001` / master roadmap M2.
Implementation and review were performed directly in ChatGPT. No Codex/Works,
CLI agent or separate model was invoked for this checkpoint.

## Reproduced problem

`createMediaThumbnailController` rejected stale results but immediately started
another sequential batch whenever its source or durable E/R/H identity changed.
A controlled unresolved reader plus 50 updates produced 51 outstanding layer
requests and 51 outstanding asset requests. This measures frontend requests,
not 102 actual FFmpeg processes, device load, CPU usage or frame-rate loss.
The original output is retained in `target/qa/media-thumbnail-bound-20260908/baseline.log`.

## Change and ownership

`createLatestThumbnailBatch.ts` owns one active batch and one replaceable latest
successor for each of the two independent lanes. Superseded batches stop before
the next request and cannot publish. Reset clears successors and cached permission;
disposal retires both lanes. Already-issued promises remain owned until settlement.
Source signatures, cache identity and authority validation stay in the existing
controller. The native loader, App.tsx, project schemas and fixed UI layout are unchanged.

## Automated evidence

Node 22.22.1 / pnpm 10.9.0 on Windows:
- The production-controller gate passes with its existing authorization, authority,
  cached-source reuse, independent lanes, reset, disposal, 100 unchanged-array and
  100 equivalent-authority publication checks retained.
- The 50-update burst peaks at one outstanding request per lane and makes two
  requests per lane in total: the original and the latest successor.
- A successor batch completes all 13 assets; the retired batch never starts a
  second asset. A late failure from the retired request cannot overwrite results.
- Twenty reset/re-authorize cycles while reads remain pending do not add requests;
  a final reset discards the successor without starting it.
- TypeScript `tsc --noEmit`: exit 0. Wrapper: 243 assertions / 27 hostile fixtures.
`check:media-thumbnails` runs the focused gate and is included in `check:release`.
The changed old assertion now requires the successor to wait for the active read;
its authority rejection and eventual successor completion assertions are retained.

## Review and unresolved boundaries

This is author self-review, not independent-review evidence. Current user direction
excludes delegated review; no independent-review acceptance is claimed.
Additional pending-successor disposal/content-change/source-filter fixtures were
not appended because the tool rejected that write; existing disposal coverage did run.
The limit is per controller and per lane, not a process-wide decoder pool. An
unsettled native promise still blocks its lane: no timeout, forced cancellation,
backend process termination or physical reaping is claimed. No new threads, timers,
retries or drawing-loop work were introduced. Cache limits, waveform/proxy jobs,
backend cancellation and cold/warm performance budgets remain in MEDIA-DERIVED-001.

## Next code boundary inspected

At the base source, `get_media_asset_thumbnail` in `app/src-tauri/src/main.rs`
uses `spawn_blocking` for rendering and revalidates source/authority after await.
`get_video_layer_thumbnail` is a synchronous command taking an engine snapshot
and the shared `video_preview` renderer lock before rendering. The frontend
loaders in `createVideoRuntimeController.ts` invoke those commands and convert
frames to data URLs; they do not expose a cancellation ticket.
These observations justify examining native scheduling/lock ownership next.
They do not establish a measured UI stall, absence of all lower-level timeouts,
or a finished native cancellation design. Preserve the existing source and
post-render authority checks when separating those responsibilities.

## Final native and integration evidence

`check:release` passed after the new gate was added; its log confirms that the
50-update and 13-asset regressions actually ran. Completion-ledger, Q1-Q4 ledger
and `git diff --check` passed without changing any acceptance row to Complete.
The first build invocation stopped progressing before frontend/Cargo execution;
its remaining owned shell was terminated after process inspection. Its log is
retained. The same maintained `pnpm --dir app tauri build --no-bundle` command,
with output captured directly by a Node launcher, then exited 0. The optimized
Rust build finished in 3m29s with zero Rust warning diagnostics. A pre-change
native warning baseline was not measured; no warning delta is claimed.

Fresh artifact: `target/release/syndocal.exe`, 64,420,864 bytes,
version `1.2.0-alpha.69`, SHA-256:
`004C4E8082581346EAF0DE954FBA0F9152AA16DA80A01ECE4D6AEE37B77DCB29`.
At `2026-09-07T18:05:48.5733100Z`, the exact artifact's PID 656 owned one
responsive, maximized `Syndocal` window. Launch verification exited 0; the helper
issued zero physical-output commands, stopped its own application and the
post-check found zero remaining exact-artifact processes. This is window proof,
not native thumbnail-content, camera, audio, DMX, venue or Mac acceptance.

Evidence directory: `target/qa/media-thumbnail-bound-20260908/`; key logs are
`baseline.log`, `focused-01.log`, `wrapper.log`, `release-final.log`,
`native-build-02.log`, `native-launch-01.log` and `native-launch-01.json`.
The seven pre-existing dirty macOS validation files are outside this checkpoint.
