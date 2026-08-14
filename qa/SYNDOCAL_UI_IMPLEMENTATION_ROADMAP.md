# Syndocal UI implementation roadmap

Status: active implementation contract. This document does not claim that the current UI already matches the visual targets.

Visual targets:

- `qa/ui-product-vision/SYNDOCAL_UI_SETUP_2026-08-12.png`
- `qa/ui-current-2026-08-12/control-edit-1920x1080.png` (current-source reference for the user-approved current-executable Lighting Edit baseline; native recapture pending)
- `qa/ui-product-vision/SYNDOCAL_UI_EDIT_VIDEO_2026-08-12.png` (Video layout matched to that baseline)
- `qa/ui-product-vision/SYNDOCAL_UI_CONTROL_2026-08-12.png`

## Information architecture

The primary workspace navigation is limited to three concepts.

| Workspace | Operator question | Included contexts | Excluded from the default surface |
| --- | --- | --- | --- |
| Setup | What is connected, patched, placed, and healthy? | Lighting, Video, Stage, I/O | Scene programming, clip performance, live transports |
| Edit | What should the show do? | Lighting, Video; shared Timeline and contextual Effects | Patch diagnostics, permanent live-output controls |
| Control | What is live now, what is next, and how do I recover? | Lighting, Video, Both | Detailed effect construction and routing forms |

Touch remains a specialized operator surface reachable from Control. It is not a fourth peer workspace. Edit has exactly two primary domains, Lighting and Video. Timeline and Effects are contextual editing tools, not peer workspaces or separate applications.

The persistent shell owns only project/save truth, Show Clock, output health, and guarded emergency actions. A function belongs in the shell only when the operator must see or use it across all three workspaces.

## Interaction invariants

- One dominant action per region. GO, Take, Patch, Save, and Import Media must not compete with several equally styled actions.
- Related action and state stay together. An HDMI window action appears on its HDMI card; an import error appears beside Import Media.
- No feature is made to fit by reducing established typography, icon, control, spacing, or hit-target size.
- Desktop primary actions are at least 40 px; coarse-pointer primary actions are at least 44 px.
- Advanced settings use closed disclosure, contextual inspectors, internal scrolling, or pagination. The application shell does not gain outer scrolling.
- Enabled, Blackout/Clear, Window Open/Closed, and physical/effective ownership remain separate truths. They are never collapsed into a generic `Off` state.
- Empty states contain one clear next action. Populated states retain the action needed to add the next item.
- Icon-only controls have accessible names. Focus order follows visual order. Keyboard activation, visible focus, reduced motion, and paste remain supported.
- Red is reserved for blackout, failure, or genuinely destructive operations. Routine live selection is not presented as danger.

## Implementation tranches

### UI-0 — Reachability blockers

1. Keep one `Import Media` entry reachable in empty, populated, compact/mixer, and full clip-bank states.
2. Keep Close/Sync/Open actions reachable and truthful for each Display output without relying on a fresh status poll to reveal them.
3. Keep scene/group creation and selection on one shared Group Picker contract so a mistyped group cannot disappear from the matrix.
4. Prove each route with branch-specific tests; a global string-presence assertion is insufficient.

Exit evidence:

- zero unreachable primary actions at 1920x1080, 1920x1032, 1366x768, and 1280x720;
- keyboard path reaches the same action exactly once;
- populated-state tests preserve authored items while opening the add/create flow.

### UI-1 — Shared shell and workspace model

1. Present Setup, Edit, and Control as the only peer workspaces.
2. Move authoring-only controls from the persistent chrome into Edit context.
3. Keep Show Clock, save truth, concise output health, and isolated Blackout persistent.
4. Preserve deep-link/session restoration for the existing contexts while mapping them to the new workspace model.

Exit evidence:

- a first-time operator can identify where to patch, create a scene, edit a timeline, and run the show without opening unrelated panels;
- no duplicated command with conflicting state in shell and workspace;
- back/forward/session restore returns to the same selected object and context.

### UI-2 — Setup workspace

1. Make Fixture Library → Patch → Stage placement → selected Fixture Inspector one visible workflow.
2. Reuse the same selection/inspector model for Video outputs and I/O.
3. Keep diagnostics contextual and collapse advanced protocol fields by default.
4. Preserve authored configuration while machine-local output ownership blocks physical emission.

Exit evidence:

- patch/add/address/identify/diagnose tasks meet or beat the pinned Daslight task path;
- no modal chain is required for the primary patch path;
- errors identify the fixture/output and recovery action beside the failed operation.

### UI-3 — Edit workspace

1. Replace the mixed-axis `Live Edit / Timeline / VJ Desk` navigation with `Lighting / Video`.
2. Edit > Lighting preserves the current executable layout without structural redesign: Scene Matrix, groups/fixtures, editable Stage, Attributes, Faders, Effects, DMX, Edit/Blind/Live truth, and a shared expandable Timeline.
3. Edit > Video uses the same upper/lower and left/right geometry as current Edit > Lighting. Its matching regions are media/clip authoring plus current/next truth, composition/output canvas, and layer/transform/composite/effect inspection. Live Take/GO-style performance remains in Control > Video.
4. Implement `qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md` before treating the Video layout as complete. A thumbnail grid over the legacy one-source-per-layer model is not sufficient.
4. Timeline opens as the same shared editing surface from either domain and keeps lighting, video, audio, automation, beats, and one playhead aligned.
5. Use one selection model and one undo history across scenes, clips, timeline blocks, and effects.
6. Replace free-text/known-list group divergence with a shared Group Picker and an explicit `Create group` action.
7. Keep advanced fixture/effect/transform/automation groups closed until requested.

Exit evidence:

- scene, cue, effect, clip, and timeline authoring tasks meet or beat the pinned comparison path;
- undo/redo and save feedback identify the affected object;
- selected object and inspector never disagree after navigation, delete, undo, or project load.

### UI-4 — Control workspace

1. Provide Lighting, Video, and Both emphasis without forking the Show Clock or project state.
2. Lighting exposes Current, Next, cue matrix, Back, dominant GO, Release, and master intensity; editing stays in Edit.
3. Video exposes persistent Import Media, clip grid, Preview, Program, one Take action, and per-output HDMI truth.
4. Both keeps these operator paths visible while using a compact read-only Show Progress strip. Full timeline editing opens Edit.
5. Audio Reactive and Auto Operation show one explicit mode, confidence/activity, and a clear manual override.

Exit evidence:

- GO/Back/Release/Blackout and Preview/Program/Take remain reachable at every supported viewport;
- the operator can state Current, Next, Program, HDMI 1/2 window state, and output ownership after one glance;
- no normal clip pad exposes more than three primary actions.

### UI-5 — Native and comparative acceptance

1. Run focused component/static gates, then supported viewport gates.
2. Build the native release target, launch the exact checkout executable, verify one responsive Syndocal window, and maximize it before UI operation.
3. Run keyboard, coarse-pointer/touch, screen-reader naming, localization, reduced-motion, and focus-return checks.
4. Measure the pinned Daslight and SynapseRack comparison tasks with the same start state and operator instructions.

Completion requires recorded task time, clicks/keystrokes, wrong turns, errors, recovery, and named evidence. Static screenshots or a browser-only preview do not prove native completion.

## Current first defects

- The current clip-grid add route is rendered only by the empty-state branch; once layers exist, the grid replaces that branch and no persistent import entry is rendered. UI-0 fixes this first.
- Scene Settings accepts a free-text group while the cue editor selects known groups and the matrix moves unknown groups into Show. UI-3 replaces this split contract.
- The visual targets deliberately remove editing density from Control; they are product targets, not instructions to shrink or restyle every current component at once.
