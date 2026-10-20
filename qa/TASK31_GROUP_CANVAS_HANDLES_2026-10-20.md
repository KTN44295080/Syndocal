# Task 31: group authoring canvas

- Branch: `task31-group-canvas-handles`; base: `e52afa3f`.
- Scope: selected video-layer groups in Edit > Video's group inspector and the
  Perform desk's Group FX inspector. Enable the group's Transform checkbox to
  reveal the canvas. The existing Edit media thumbnail and Perform single-layer
  Preview represent different sources, so the group inspector owns a separate
  composition preview. Program monitors, outputs and recordings receive no overlay.

## Behavior

- Drag inside the group bounds to move, drag a corner to scale about the group
  center, and drag the round handle to rotate about that same pivot. Shift keeps
  the initial scale ratio or snaps rotation to 15 degrees. Arrow keys move 0.01
  composition units; Shift moves 0.1. Geometry follows the compositor's normalized
  rotate-then-translate convention, including non-square output resolutions.
- Pointer capture owns each gesture. Escape, pointer cancellation, capture loss,
  resize, group/catalog change or project-authority change discards its draft.
  Inspector numbers follow the draft and display at most four decimal places.
- All pointer moves coalesce locally. A changed pointer-up submits exactly one
  catalog mutation through the existing ticketed inspector/controller path.
  No-op clicks and cancelled gestures submit none. An unresolved publication
  blocks additional gestures; a rejected publication discards the local draft.
- Preview is a bounded, one-shot composition still on selection/publication or
  explicit Refresh; it is not a live playback monitor. The existing
  `get_debug_video_preview` route accepts an optional composition ID, preserves
  omitted-ID callers and rejects a missing explicit composition. No routes or
  MIDI/OSC/DMX actions were added; invoke inventory remains 507.
- Preview requests have one in flight and at most one latest-state refresh
  queued. Each frame is at most 512 pixels per dimension. Generation checks
  suppress superseded/unmounted replies. There is no new polling loop. Remaining
  blocking boundary: the existing native CPU preview renderer and its lock; no
  new hardware/render-performance acceptance is claimed.
- Theme tokens, 13px body/14px controls, visible keyboard focus, JA/EN labels and
  44px corner/rotation targets are preserved. Pixel-based padding contains every
  full handle at large transforms on a 390px viewport.

## Evidence

- `pnpm install --frozen-lockfile --prefer-offline` in `app`: passed.
- `pnpm --dir app run check:video-group-canvas`: passed geometry and Chromium/CDP
  interaction checks. Eight move events produced zero publications until release,
  then one valid ticket and one history entry. Undo restored the starting transform.
  Also passed Shift scale/rotation/nudge, Escape, pointer cancellation, authority
  interleaving, rejection, no-op and pending-publication duplicate guards.
- Browser screenshots inspected at 1400x1000 EN and 390x844 JA; zero console
  errors, no framework overlay, correct aspect and no horizontal page overflow.
  Extreme scale=3, rotation=45, x=1/y=-1: all full 44px handle rectangles contained.
- Evidence: `target/qa/task31-group-canvas/{results.json,canvas-1400-en.png,canvas-390-ja.png}`.
  Browser plugin unavailable; used the repository's owned Chromium/CDP approach
  with Vite 5470 and CDP 9470. IPC/history is a strict in-memory backend fixture,
  exercising the actual Solid inspector and runtime controller.
- `check:video-layer-groups`, `check-project-undo-grouping.mjs`,
  `check:frontend-invokes` (507), `check:localization` (4151/4151), and
  `pnpm --dir app exec tsc --noEmit`: passed.
- `check:perform-layout`: passed all six viewport/locale and five runtime-focus
  cases; evidence in `target/qa/vj-perform-ui/browser/results.json`.
- `pcargo-here.cmd check -p syndocal`: passed debug profile, exact MSVC
  14.44.35207 linker verified, CARGO_TARGET_DIR unset. Current compiler warnings:
  zero. Pre-change compiler baseline was not measured; no compiler delta claim.
- Independent read-only review: both aspect and extreme-handle containment
  findings fixed and covered by browser assertions; final review approved.
- `git diff --check`: passed.

## Boundaries and next action

The task forbids native launch/release builds, push and merge. No native window,
real decoder/effect rendering, device or physical-output acceptance was performed.
After the runner integrates the owned commit, the main-checkout owner should run
the required native gate and inspect the group canvas in both authoring contexts.
No product version/schema bump or pinned route-count change is needed.
