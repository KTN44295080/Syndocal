# Video C2/C4 checkpoint — 2026-09-13

This checkpoint closes the current-source software and rendered-browser slice of
`VIDEO-C2-C4-001` from source base `69cf28d1c910bce676d168e68eed70e2bf74cb3e`
on branch `codex/showclock-review-20260912`. It does not claim native renderer,
GPU breadth, 4K or multi-display presentation, physical output, venue, signing,
publishing, or product-wide acceptance.

## Revalidated behavior

- Clip Slot/Layer Bus authority is wired through acknowledged transactions,
  including active/queued Clip Take state, transition kind/duration, rollback,
  and fail-closed invalid state.
- Timeline uses stable layer/clip ids and the authoritative mutation path for
  clip placement, layer mapping, event timing, fades, and child Timeline
  navigation.
- The layered Timeline browser fixture proves six typed lanes, Audio clip
  rendering, waveform/fade affordances, section-scoped add controls, internal
  scrolling, zero outer scroll, explicit lane expansion/collapse, mute state,
  and child-Timeline round trip.
- The layered viewport checker now targets unautomated Lighting lane 13 for the
  explicit expansion assertion. Lane 12 is intentionally auto-expanded by the
  fixture's implicit Lighting automation, so using it would test the wrong
  contract rather than the operator's expand/collapse action.

## Evidence

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS |
| `pnpm.cmd --dir app run check:timeline-follow-runtime` | PASS |
| `pnpm.cmd --dir app run check:timeline-follow-hold-ui` | PASS |
| `pnpm.cmd --dir app run check:video-runtime-polling` | PASS |
| `pnpm.cmd --dir app run check:video-output-routing-runtime` | PASS |
| `pnpm.cmd --dir app run check:timeline-advanced` | PASS |
| `CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe pnpm.cmd --dir app run check:timeline-authority` | PASS — 1280x720 browser IPC request contract |
| `CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe pnpm.cmd --dir app run check:timeline-layered` | PASS — 1920x1080, 1920x1032, 2048x1152, 1366x768, 1280x720; explicit expansion 30->54->30px |
| MSVC 14.44.35207 x64 `cargo test -p engine --release --locked -j 1 video_full_gate_engine_path -- --test-threads=1` | PASS — 1 test |
| MSVC 14.44.35207 x64 `cargo test -p engine --release --locked -j 1 video_sample_clip_take_queue -- --test-threads=1` | PASS — 1 test |
| MSVC 14.44.35207 x64 `cargo test -p engine --release --locked -j 1 video_sample_follow_admission -- --test-threads=1` | PASS — 1 test |
| MSVC 14.44.35207 x64 `cargo test -p engine --release --locked -j 1 video_transition_bus_c3_is_typed -- --test-threads=1` | PASS — 1 test |

The MSVC procedure reported the exact linker:
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.

## Remaining boundary

`VIDEO-C2-C4-001` is complete for the supported current-source/browser scope.
Native-window interaction, native renderer/GPU/display matrix, 4K and
multi-display presentation, physical devices, venue soak, external clients,
signing/publication, and product-wide acceptance remain separate gates. The
Q1 `COV-VIDEO-SLOT-001` row therefore remains `In progress` until its broader
acceptance boundary is satisfied. `TIMELINE-FOLLOW-001` and later Flow markers
remain independently tracked.
