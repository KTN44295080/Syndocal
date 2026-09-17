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

## Continuation — current-source accessibility recheck after high-DPI UI repair — 2026-09-15

At current source HEAD `bcf3194b`, after the high-DPI Video upper-desk repair,
the focused accessibility-adjacent source sequence was rerun with the exact
MSVC `14.44.35207` linker environment initialized and confirmed first by
`where.exe link.exe`:

```text
pnpm.cmd --dir app run check:localization
pnpm.cmd --dir app run check:terminology
pnpm.cmd --dir app run check:empty-states
node app/scripts/check-project-history-keyboard.mjs
node app/scripts/check-timeline-space-keyboard.mjs
pnpm.cmd --dir app run check:stage-labels
pnpm.cmd --dir app exec tsc --noEmit
pnpm.cmd --dir app run build
```

All eight commands exited `0`. Static Japanese UI coverage remained
`3844/3844 (100.0%)` with zero unprotected bare user-data labels; terminology,
empty-state guidance, project-history keyboard routing, Timeline Space routing,
and Stage label contracts passed. TypeScript passed and the Vite build
transformed `358` modules successfully. The existing large-chunk message is a
Vite advisory; no first-party compiler warning was observed.

This is current-source localization, semantic, keyboard-routing, and build
evidence only. No NVDA/JAWS/Narrator traversal, High Contrast rendering,
125/150/200% native scaling matrix, IME composition, reduced-motion native run,
or native keyboard-only dangerous-action workflow was performed.
`ACCESSIBILITY-NATIVE-001` remains **Open** pending the named native
accessibility environment and operator matrix.

## Current native NVDA/UI Automation and keyboard-only recheck — 2026-09-15

The current release executable was exercised while NVDA `2026.1.1.55980`
(`2026.1.1`) was installed and running through `nvda_noUIAccess.exe`. NVDA
usage-statistics transmission was explicitly declined; the local NVDA config
records `allowUsageStats = False` and `askedAllowUsageStats = True`.

The Syndocal native window was inspected through Windows UI Automation:

```text
AutomationElement descendants: 202
named keyboard-focusable controls: Setup, Edit, Control, Workspaces, GO,
DMX Blackout, Video Blackout, All Blackout, MIDI Learn, OSC Learn, DMX Learn,
I/O, USB-DMX, Prepare, RDM tools, and the stage/video controls
```

Using the real native window, keyboard-only `Tab` traversal reached the I/O
route and the named `Prepare` button. Focusing `Prepare` and pressing Enter
opened the native `出力制御の確認` warning; selecting `いいえ` cancelled before
the physical start path. The same keyboard-only focus path reached `All
Blackout`; the resulting UI showed `All BO` while the status bar retained
`OutputControl rejected (forbidden); output was not applied; refresh lease
state.` No physical blackout or fixture state is claimed from this UI result.

This establishes a current native semantic/focus and pre-action confirmation
slice, not NVDA speech announcement acceptance. No speech-viewer transcript,
High Contrast rendering, 125/150/200% native scaling matrix, IME composition,
reduced-motion run, or full keyboard-only dangerous-action/recovery workflow
was completed. `ACCESSIBILITY-NATIVE-001` remains **Open** pending those
operator-observed native gates.

## Continuation — current-source accessibility recheck and UI-label repair — 2026-09-18

At current source HEAD after the UI containment changes, the accessibility-
adjacent source sequence was rerun. The first run found a real localization
regression in the newly compact safety-blackout topbar control:
`3843/3845` static strings were covered, with `Release safety blackout` and
the intentional compact badge `SAFE` reported as untranslated. The fix adds
the Japanese ARIA label `安全ブラックアウトを解除` and explicitly retains
`SAFE` as a locale-invariant product badge so the compact visual label does
not expand and reintroduce the upper-bar squeeze.

The corrected run and all remaining source checks exited `0`:

```text
check:localization: PASS (3845/3845; 100.0%; 0 unprotected user-data labels)
check:terminology: PASS
check:empty-states: PASS
check-project-history-keyboard: PASS
check-timeline-space-keyboard: PASS
check:stage-labels: PASS
tsc --noEmit: PASS
vite build: PASS (358 modules)
```

This checkpoint fixes only the discovered source/localization regression and
records current-source semantic/build evidence. The existing native
UIA/NVDA/keyboard evidence remains limited to its recorded 2026-09-15 slice;
no new speech-viewer transcript, High Contrast rendering, 125/150/200% native
scaling matrix, IME composition, reduced-motion run, or full keyboard-only
dangerous-action/recovery workflow was performed. Therefore
`ACCESSIBILITY-NATIVE-001` remains **Open**.

## Current release UI Automation probe — 2026-09-18

The exact current release executable was running as one responsive
`target/release/syndocal.exe` process. A read-only Windows UI Automation probe
found the main window and its WebView2 child, but the WebView content controls
were not exposed as descendants of the native UIA tree: 16 descendants were
reported, with only the `Syndocal` and `Syndocal - Web コンテンツ` panes carrying
names. No Enter, dangerous action, or state mutation was sent.

This confirms the host can locate the native window, while it does not
substitute for NVDA speech-viewer traversal or prove the inner web controls'
native accessibility exposure. The existing source/ARIA checks remain valid,
but `ACCESSIBILITY-NATIVE-001` stays **Open** for NVDA/Narrator, High Contrast,
scaling, IME, reduced motion, and keyboard-only safety/recovery observation.
