# Tap interval feedback and external-clock diagnosis

Branch `codex/showclock-review-20260912`, base `c78981bc7346bed45e04ed99b401f0000e74ce80`.

The first Tap, including the first sample after a pause over two seconds, does
not yet provide an interval. App previously reported `Tapped BPM` with the old
tempo anyway. It now says `Tap again to measure BPM.` A measured Tap still
reports its actual tempo. If the returned clock source is external, the message
names that source and does not claim that Tap owns the observed tempo. This adds
no header content, changes no sizing, and preserves the existing authority and
publication ordering from [the publication repair](TAP_BPM_PUBLICATION_2026-10-03.md).

Two deterministic tests reproduce distinct existing external-source behaviors:

- Ableton Link or DJ Link ingress between 750ms taps clears the history each
  time, leaving one sample and the external 124 BPM. When ingress stops, the
  next Tap measures 80 BPM.
- External ingress after a successfully measured 80 BPM Tap replaces that
  tempo with 124 BPM and clears the samples.

These tests establish reproducible conditions, not the operator's active source.
This checkpoint does not change external-clock priority or disconnect inputs.
The operator's physical clicks and actual external ingress remain unobserved.

## Evidence

| Check | Result |
| --- | --- |
| Clock unit tests | 7 passed, including the two new external-ingress reproductions |
| Tap controller | 9 async cases plus pending/applied/source message assertions passed |
| Frontend invoke inventory | 481 commands, unchanged |
| Optimized Windows QA build | Passed with exact MSVC 14.44 linker |
| Isolated native lifecycle | 8 groups passed; App callback, actual header and footer observed |
| Ordinary Windows build/window | Passed; one responsive visible maximized `Syndocal` window |

The native QA executable SHA-256 is
`40c3705136f58522cab1db67bedcbddb508f1a91d1153b98b1a382d2384dbe8d`.
Five scheduled 750ms taps changed 120 to `80.79931` BPM; the header showed `81`
and the footer `Tapped BPM 80.8`. The first and post-pause taps instead showed
the pending message. Header height remained 42 CSS pixels and the enabled,
unobstructed Tap target remained 40 by 40. Backend screenshots show readable
header/footer and no visible error overlay. The private QA app starts in Setup;
this validates the shared header/App callback, not a physical Control click.

[Clock tests](artifacts/tap-feedback-clock-tests-2026-10-05.txt),
[controller tests](artifacts/tap-feedback-controller-tests-2026-10-05.txt),
[frontend inventory](artifacts/tap-feedback-frontend-invokes-2026-10-05.txt),
[QA build](artifacts/tap-feedback-qa-build-2026-10-05.txt),
[native evidence](artifacts/tap-feedback-native-2026-10-05.json),
[native log](artifacts/tap-feedback-native-2026-10-05.txt),
[visual evidence](artifacts/tap-feedback-visuals-2026-10-05.json), and
[source freeze](artifacts/tap-feedback-source-freeze-2026-10-05.json).
Screenshots are retained outside Git at the paths and hashes in the visual
evidence. Browser plugin unavailable; process-verified native backend queries,
the opt-in production App callback and passive screenshots were used. No
Computer Use, DOM clicks, subagents, or physical-output activation was used.

The ordinary optimized no-bundle build passed. Its executable SHA-256 is
`092059a8723ec3154e134df8bc6fbfc5a5dace5e278efc1b0c083bd7557674f4`;
PID `70372` owns exactly one responsive visible maximized `Syndocal` window.
The previous exact-path PID `57652` was closed gracefully after its hash was
verified; no force termination was used. The ordinary bundle contains no Tap
QA receiver. [Build log](artifacts/tap-feedback-normal-build-2026-10-05.txt) and
[passive window proof](artifacts/tap-feedback-normal-window-2026-10-05.json)
verify the refreshed artifact and unauthenticated-read rejection. This does
not establish authenticated primary-profile Tap or the operator's clock source.

Optimized Rust/TypeScript warning baseline/current/delta is `0/0/0` against the
preceding backup-inspection builds. Vite's existing chunk advisory is `1/1/0`.
The clock-unit configuration emitted zero current warnings; no separate warning
baseline is claimed for that configuration.

Only the two Tap import/message hunks in the already-dirty `App.tsx` are owned.
Reversing exactly those edits reproduces its protected original SHA-256. The
other four protected files retain their original hashes; unrelated App hunks
remain unstaged. No version, protocol schema, registry, grants or ledger status
changes are made. Stable self-review is not independent review.

The broader goal remains active. Canonical backup restore/list/delete and the
unobserved native/hardware/venue acceptance boundaries remain open.
