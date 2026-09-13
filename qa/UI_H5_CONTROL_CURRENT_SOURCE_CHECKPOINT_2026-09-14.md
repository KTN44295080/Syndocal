# UI-H5-CONTROL-001 current-source checkpoint — 2026-09-14

- Marker: `UI-H5-CONTROL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `8480c84627b0fa2f1168fed433e60a52289cd06a`
- Product code change: Control Both blackout toggles now expose `aria-pressed`
  and stable accessible action names; visible labels and authority callbacks
  are unchanged.

## Existing H5 implementation evidence

The current H5 implementation includes the combined `Both` overview and the
lease-bound Lighting/Video master, cue, blackout, clip, Take, and launch paths.
The previously recorded `EV-UI-H5-BOTH-SW-2026-09-14` evidence covers its
TypeScript/Vite, routing, rendered 1920x1080 browser, and pinned Windows native
build/process-smoke checks. The exact process-smoke artifact recorded there is
the unsigned current-source executable with SHA-256
`16A853E5F1AB38C97918854B436A2CC8A9B15CF8AA72732BA16DDB2C8A7456AC`.

The previously recorded evidence remains the implementation baseline for
layout and routing. The latest source change adds semantic toggle state to the
three Both blackout controls. The detailed implementation record is
`qa/CONTROL_BOTH_CHECKPOINT_2026-09-14.md`.

## Acceptance boundary

`UI-H5-CONTROL-001` remains `Open`. The existing evidence does not establish
native button-by-button interaction, full live Lighting/Video/Audio workflow
completion, native accessibility, physical output, failure/recovery rehearsal,
external clients, venue/soak, signing, publication, or product completion.

The new browser rerun did not reach CDP: the available Chromium executable was
rejected by Windows with a Side-by-Side configuration error. The prior passing
rendered evidence remains the pre-change layout baseline; no fresh rendered
pass is claimed for the new semantic assertions.

## Resume procedure

Run the H5 matrix on the exact current artifact: live Cue/Clip/Take/Transition,
Blackout/Arm/Take Over, recording and diagnostics, failure/recovery, and native
interaction at the required operator viewports. Preserve the first failure and
separate native, physical, and venue evidence before changing the marker.
