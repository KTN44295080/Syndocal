# VJ Live Monitor Acceptance

Updated: 2026-07-13

## Implemented contract

Control > VJ Desk now exposes two continuously refreshed buses instead of a manually refreshed reference image:

- `PREVIEW` renders the layer staged with its `P` button.
- `PROGRAM` renders the currently selected video output, including its composition routing, opacity, blackout and projection mapping.
- The UI requests 320x180 JPEG frames at a 10 fps Program / 5 fps Preview budget through one serialized request loop. It never permits overlapping monitor requests.
- The loop stops outside VJ Desk and while the document is hidden. Target changes invalidate stale responses. Backend contention returns an immediate `busy` packet; the UI retains the last good frame and counts busy drops.
- Each successful frame reports sequence, PTS, render time and JPEG encode time in a fixed 40-byte `SYLV` v1 header. Tauri returns the packet as a raw `ArrayBuffer`, avoiding the former JSON number-array and frontend PNG/data-URL path.
- The backend clamps requests to 640x360 / 230,400 pixels, JPEG quality 40-90 and Program decode budget 1-2. It releases the shared preview-renderer mutex before JPEG encoding.

## Verification completed

- `cargo test -p syndocal live_video_monitor -- --nocapture`: 3 passed. The packet layout, request bounds and JPEG decodability are covered.
- `pnpm build`: TypeScript and production Vite build passed.
- `pnpm check:localization`: 1959/1959 static strings covered, with no unprotected user labels.
- `pnpm check:viewport`: all Control, Setup and Touch surfaces passed at 1280x720, 1366x768 and 2048x1129. VJ acceptance now requires exactly one Preview bus, one Program bus and zero normal Refresh buttons.
- A 1366x768 screenshot review confirmed that both 16:9 buses remain visible beside Clips, Outputs and Layers without document scrolling.

## Claim boundary and remaining performance work

This closes the manual-reference-preview UI gap. It does **not** prove that the monitor samples the final native WGPU swapchain: Program is a CPU render from the same project state and mapping rules, while the Display window remains a separate 60 Hz native GPU path. Preview follows the selected layer's current position; it is not yet an independent preview transport with its own playhead.

The shared preview renderer uses non-blocking `try_lock`, so recording or a manual diagnostic render can produce visible busy drops. Conversely, a synchronous monitor render can briefly occupy that renderer before recording acquires it. Real-media sustained fps, freshness, CPU/GPU cost and native-output interference still require measurement.

The Libav backend now keeps an eight-layer LRU working set, with at most one sequential decode session and one RGBA frame per retained layer. Requests advancing by at most 250 ms continue the same demuxer/decoder; repeated requests reuse the current decoded frame, while reverse motion, loops, larger jumps, source/size/signature changes and errors explicitly reopen or evict the session. Non-file, pathless and HAP-routed replacements release stale sessions immediately. Decoded PTS and seeks share the stream start-time origin instead of assuming zero-based media. Generated MPEG-4 B-frame and offset-start streams are compared against an independent FFmpeg sequential RGBA oracle, including forward continuation, reverse/large-jump reopen, EOF drain and terminal-frame reuse. Session/open/reset/continue/reuse/eviction/error counters are exposed in runtime diagnostics. The CLI fallback is independently capped to a one-frame-per-layer, eight-layer LRU so no-libav and transient-error paths cannot accumulate full RGBA history.

This removes per-frame decoder construction from ordinary forward playback, but the scaler is still created per request and sessions are not shared between output workers. No claim of lower VJ playback cost than SynapseRack is allowed until representative multi-layer H.264/H.265/ProRes/HAP measurements pass with both monitor buses enabled.
