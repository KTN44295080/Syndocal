# VJ Live Monitor Acceptance

Updated: 2026-07-13

## Implemented contract

Control > VJ Desk now exposes two continuously refreshed buses instead of a manually refreshed reference image:

- `PREVIEW` renders the layer staged with its `P` button.
- `PROGRAM` renders the currently selected video output, including its composition routing, opacity, blackout and projection mapping.
- The UI requests 320x180 JPEG frames at a 10 fps Program / 5 fps Preview budget through one serialized request loop. It never permits overlapping monitor requests.
- The loop stops outside VJ Desk and while the document is hidden. Target changes invalidate stale responses. Backend contention returns an immediate `busy` packet; the UI retains the last good frame and counts busy drops.
- Each successful frame reports sequence, PTS, render time and JPEG encode time in a fixed 40-byte `SYLV` v1 header. Tauri returns the packet as a raw `ArrayBuffer`, avoiding the former JSON number-array and frontend PNG/data-URL path.
- The backend clamps requests to 640x360 / 230,400 pixels, JPEG quality 40-90 and Program decode budget 1-2. Renderer locks are released before JPEG encoding.

The Preview bus now has an ephemeral transport that is separate from project and Program state:

- A clip is staged only by an explicit `P` action. Selection and monitor polling never auto-stage a layer. Staging starts paused at zero or at the layer loop in-point rather than copying an ended Program playhead.
- Play/Pause, millisecond seek, forward/reverse speed from 0.25x through 4x, existing loop bounds and Clear update only the Preview transport. Still images remain fixed and cannot enter a running state.
- `Cut` sends the staged playhead and speed to Program with a zero-duration transition. `Take` does the same with the configured fade. The backend rejects transfer unless the requested layer is the currently staged Preview layer; ordinary direct clip launch remains a separate operation.
- Preview supports local `File` and `Still Image` layers with a valid path. Camera, Screen, NDI and Spout sources are deliberately rejected so auditioning cannot consume a shared latest-frame queue.
- Preview transport is runtime-only: it is excluded from `.sdc`, Recovery and Undo/Redo. New/load/Undo/standby transitions, layer removal, source identity replacement and Clear invalidate it.
- Preview owns a dedicated decoder/renderer, uses decode prefetch zero and renders a one-layer snapshot. Reset uses `try_lock`, so Clear and project transitions do not wait for a slow decode. Source identity and transport generation are checked after rendering and again after JPEG encoding; a raced frame is returned as `busy`, not published as current.

## Verification completed

- `cargo test -p syndocal live_video_monitor -- --nocapture`: 3 passed. The packet layout, request bounds and JPEG decodability are covered.
- `pnpm build`: TypeScript and production Vite build passed.
- `pnpm check:localization`: static UI coverage remains 100%, including the Preview transport and Program-transfer labels.
- `pnpm check:viewport`: all Control, Setup and Touch surfaces use 1920x1080 as the primary operational/visual gate, 2048x1152 as the extended ceiling, and 1366x768 plus 1280x720 as compact containment fallbacks. VJ acceptance requires exactly one Preview bus, one Program bus, one independent Preview transport and zero normal Refresh buttons at every size.
- The sign-off screenshot is the maximized 1920x1080 VJ Desk, where both 16:9 buses must remain visible beside Clips, Outputs and Layers without document scrolling. Compact screenshots are diagnostic-only and do not substitute for this density review.
- Focused backend tests cover explicit staging, Program-state independence, pause/seek, forward and reverse looping, speed sanitization, unsupported sources, source replacement/removal, in-point rewind and stale-frame generation checks.

## Claim boundary and remaining performance work

This closes the independent Preview playhead gap for local files and stills. It does **not** prove that either monitor samples the final native WGPU swapchain: Program is a CPU render from the same project state and mapping rules, Preview is a separate CPU render of the staged layer, and the Display window remains a separate 60 Hz native GPU path. Preview has independent transport time and speed, but it still reads the staged layer's current transform, color and effect definition; it is not a second persisted layer editor. Preview audio scrubbing and live Camera/Screen/NDI/Spout staging are not implemented.

The Program monitor still shares its renderer with existing diagnostic/recording-adjacent preview work and can report visible busy drops under contention. Preview has a dedicated renderer and non-blocking reset, but it also creates a separate decoder session/cache rather than sharing decoded frames with Program or output workers. Real-media sustained fps, freshness, aggregate CPU/GPU cost, cache duplication and native-output interference still require measurement.

The Libav backend now keeps an eight-layer LRU working set, with at most one sequential decode session and one RGBA frame per retained layer. Requests advancing by at most 250 ms continue the same demuxer/decoder; repeated requests reuse the current decoded frame, while reverse motion, loops, larger jumps, source/size/signature changes and errors explicitly reopen or evict the session. Non-file, pathless and HAP-routed replacements release stale sessions immediately. Decoded PTS and seeks share the stream start-time origin instead of assuming zero-based media. Generated MPEG-4 B-frame and offset-start streams are compared against an independent FFmpeg sequential RGBA oracle, including forward continuation, reverse/large-jump reopen, EOF drain and terminal-frame reuse. Session/open/reset/continue/reuse/eviction/error counters are exposed in runtime diagnostics. The CLI fallback is independently capped to a one-frame-per-layer, eight-layer LRU so no-libav and transient-error paths cannot accumulate full RGBA history.

This removes per-frame decoder construction from ordinary forward playback, but the scaler is still created per request and sessions are not shared between Preview, Program and output workers. No claim of lower VJ playback cost than SynapseRack is allowed until representative multi-layer H.264/H.265/ProRes/HAP measurements pass with both monitor buses enabled on the target venue GPU.
