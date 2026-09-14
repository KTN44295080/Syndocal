# Accessibility Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ACCESSIBILITY-NATIVE-001` (section 6, Open)
- Q1 row: `COV-ACCESSIBILITY-001`
- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `0a6672502eb40ff40a7fe2e147292e39d967e7fc`
- Scope: current-source Japanese localization, terminology, empty-state,
  keyboard routing, stage-label contracts, and accessible H5 Control Both
  blackout-state semantics after the H5 Control addition.

## Implementation change

The current source exposed 113 previously unregistered English static UI texts
through the H5/AI operator surfaces. They were added to the existing bounded
`translateUiText` Japanese dictionary in `app/src/uiLocalization.ts`; no raw
user-data labels were made translatable. This is a UI copy/localization change,
not a native accessibility acceptance claim.

The Control Both overview now adds `aria-pressed` and stable action names to its
DMX, all-output, and Video blackout toggles. Visible operator labels are
unchanged, while assistive technology receives the current on/off state. This
is current-source semantic evidence only; it does not claim NVDA or native
accessibility acceptance.

## Verification

The exact Q1 automated proof was rerun with command chaining that stopped on the
first non-zero result:

```text
pnpm.cmd --dir app run check:localization
pnpm.cmd --dir app run check:terminology
pnpm.cmd --dir app run check:empty-states
node app/scripts/check-project-history-keyboard.mjs
node app/scripts/check-timeline-space-keyboard.mjs
pnpm.cmd --dir app run check:stage-labels
```

Result: exit code 0.

- Japanese static UI coverage: `3844/3844 (100.0%)`.
- Unprotected bare user-data labels: `0`.
- UI terminology: PASS.
- Empty-state guidance: PASS.
- Project history keyboard routing: PASS; native text-editor Undo remains
  protected.
- Timeline Space routing: PASS across play/pause/resume, root-child mismatch,
  empty, repeat, modifier, and outside-surface cases.
- Stage label footprint/priority/overlap contract: PASS.
- `pnpm.cmd --dir app exec tsc --noEmit`: PASS.
- `pnpm.cmd --dir app run build`: PASS; 358 modules transformed. The existing
  Vite large-chunk advisory remains an advisory, not a first-party compiler
  warning.
- First-party warning count observed in the focused run: 0.

## Takeover rerun — 2026-09-14

The focused source sequence was rerun after takeover against current source:
localization remained `3844/3844` with zero unprotected user-data labels;
terminology, empty-state guidance, project-history keyboard routing,
Timeline Space routing, and stage labels all passed. TypeScript passed and the
Vite production build transformed 358 modules successfully. The existing
large-chunk message remained a Vite advisory; no first-party compiler warning
was observed. No native accessibility API or screen-reader environment was
used.

The dedicated Control browser rerun could not start: the available Chromium
binary was rejected by Windows before CDP startup because its Side-by-Side
configuration was invalid. No new rendered or console evidence is claimed from
that failed start.

The initial pre-fix localization run was intentionally not counted as evidence:
it reported `3731/3844 (97.1%)` and exited non-zero. The dictionary update was
then applied and the full Q1 sequence above passed.

## Native and external boundary

The native accessibility proof remains not-run. This environment reported native
computer APIs disabled, so no unsupported native UI observation is claimed.
The separate native maximized/F11 preflight also failed closed before any UI
interaction because this run had only a `1280x800` display (`1280x752` work
area), below the required `1920x1080` acceptance display. This does not change
the accessibility status and is not counted as native accessibility evidence.

The native gate was retried after the host GPU query changed to `2560x1600`.
The app-owned maximized client remained `1280x752`, and the gate failed closed
before F11 or UI interaction. The current user display registry reports
`AppliedDPI=192` (200%), so the logical client/work-area measurement still does
not satisfy the required `1920x1000` / `1920x1080` native boundary. No native
accessibility evidence is claimed from this retry.

The later current-source native window/pane acceptance did pass the corrected
Per-Monitor V2 geometry gate and both Stage/Timeline lifecycle orders on a
temporarily selected physical `1920x1080` display. That result is recorded as
native window/lifecycle evidence only; it did not run a screen reader or any
accessibility workflow and therefore does not change this marker.
The following require the operator PC and named accessibility environments:

- NVDA screen-reader traversal and announcements;
- Windows High Contrast and color-independent state visibility;
- 125%, 150%, and 200% scaling with no clipped/hidden safety controls;
- keyboard-only dangerous-action and recovery workflows;
- IME composition and focus retention;
- dialog/popout focus return and Escape behavior;
- reduced-motion behavior.

Next action is a native release executable run on the operator PC with the
accessibility matrix recorded per workflow. Until that evidence exists,
`ACCESSIBILITY-NATIVE-001` stays Open and the current-source PASS must not be
reported as native accessibility acceptance.

## Takeover continuation — current-source accessibility recheck — 2026-09-14

The focused source sequence was rerun against HEAD `ecc1799e`: localization
reported `3844/3844` with zero unprotected user-data labels; terminology,
empty-state guidance, project-history keyboard routing, Timeline Space routing,
and stage-label contracts all passed. TypeScript passed and the Vite build
transformed `358` modules successfully. The existing large-chunk message was
an advisory; no first-party compiler warning was observed.

No native screen-reader, High Contrast, scaling, IME, reduced-motion, or
native keyboard-only environment was used. `ACCESSIBILITY-NATIVE-001` remains
`Open` for that native matrix and the related physical/external acceptance.

## Takeover continuation — current-source accessibility recheck — 2026-09-14

At current source HEAD `af0f5cde`, the focused source contracts were rerun:

```text
pnpm.cmd --dir app run check:localization
static Japanese UI coverage: 3844/3844 (100.0%); unprotected bare user-data labels: 0

pnpm.cmd --dir app run check:terminology
ui terminology ok

pnpm.cmd --dir app run check:empty-states
empty state guidance ok

node app/scripts/check-project-history-keyboard.mjs
PASS actual keyboard controller routing and text-editor guards

node app/scripts/check-timeline-space-keyboard.mjs
PASS selected Timeline Space routing cases

pnpm.cmd --dir app run check:stage-labels
T24-A stage label footprint/priority/overlap contracts ok

pnpm.cmd --dir app exec tsc --noEmit
exit code 0
```

No NVDA/JAWS/Narrator traversal, High Contrast rendering, scaling matrix,
IME composition, reduced-motion native run, or native keyboard-only dangerous
action workflow was performed. A read-only process inventory found no active
screen-reader process. These source checks therefore do not change
`ACCESSIBILITY-NATIVE-001`, which remains `Open` pending the named native
accessibility environment and operator matrix.

## Takeover continuation — current-source accessibility recheck after Video repair — 2026-09-14

At current source HEAD `9da6c1ff`, the focused source contracts were rerun after
the Video upper-desk repair. All checks exited `0`:

```text
pnpm.cmd run check:localization
static Japanese UI coverage: 3844/3844 (100.0%); unprotected bare user-data labels: 0

pnpm.cmd run check:terminology
ui terminology ok

pnpm.cmd run check:empty-states
empty state guidance ok

node scripts/check-project-history-keyboard.mjs
PASS actual keyboard controller routing and text-editor guards

node scripts/check-timeline-space-keyboard.mjs
PASS selected Timeline Space routing cases

pnpm.cmd run check:stage-labels
T24-A stage label footprint/priority/overlap contracts ok

pnpm.cmd exec tsc --noEmit
exit code 0

pnpm.cmd run build
358 modules transformed; built in 10.64s
```

The build retained the existing Vite large-chunk advisory; no first-party
compiler warning was observed. These results confirm current-source
localization, terminology, empty-state, keyboard-routing, stage-label,
TypeScript, and production-build contracts only. No NVDA/JAWS/Narrator
traversal, High Contrast rendering, 125/150/200% scaling matrix, IME
composition, reduced-motion native run, or native keyboard-only dangerous-action
workflow was performed. No native accessibility API was used. Therefore
`ACCESSIBILITY-NATIVE-001` remains `Open` pending the named native
accessibility environment and operator matrix.
