# Syndocal UI product vision

Status: implementation target, not a claim that the current UI already matches it.

Visual references:

- `qa/ui-product-vision/SYNDOCAL_UI_SETUP_2026-08-12.png`
- `qa/ui-current-2026-08-12/control-edit-1920x1080.png` (current-source visual reference for the user-approved current-executable Edit > Lighting layout; native recapture remains required after the worktree stabilizes)
- `qa/ui-product-vision/SYNDOCAL_UI_EDIT_VIDEO_2026-08-12.png` (Edit > Video companion concept)
- `qa/ui-product-vision/SYNDOCAL_UI_CONTROL_2026-08-12.png`
- `qa/SYNDOCAL_UI_PRODUCT_VISION_2026-08-12.png` (earlier integrated Control study)

## Product principle

Syndocal presents lighting and video as two views of one show, not as two applications placed side by side. The operator sees one clock, one transport truth, one timeline, one output-health model, and one consistent selection/inspector model.

The product has three primary workspaces: Setup, Edit, and Control. Setup builds and verifies the physical system, Edit creates the show, and Control operates it live. This division keeps setup and authoring complexity off the show surface without splitting lighting and video into separate applications.

## Persistent shell

- The top show bar contains the project/save state, authoritative Show Clock, transport state, and compact health for DMX, audio, HDMI 1, and HDMI 2.
- Blackout is visually separated from GO, Back, Release, Take, and routine navigation. Red is reserved for danger, blackout, failure, and genuine live state.
- The primary navigation contains exactly three operator concepts: Setup, Edit, and Control.
- Setup contains Lighting, Video, Stage, and I/O configuration contexts.
- Edit contains Lighting and Video authoring domains. Timeline is a shared editing surface inside both domains; Effects are contextual tools for the selected lighting or video object.
- Control contains Lighting / Video / Both emphasis modes. They do not fork the project, clock, timeline, or output state.

## Setup workspace

- Fixture library, DMX patch, stage map, and the selected fixture inspector form one direct left-to-right workflow.
- Video-output and I/O setup use the same selection and inspector model; diagnostics remain contextual rather than becoming a permanent dashboard wall.
- Setup may expose technical detail, but the main patch and output actions remain visible without opening modal chains.

## Edit workspace

- The first choice inside Edit is Lighting or Video. It replaces the current mixed-axis `Live Edit / Timeline / VJ Desk` navigation.
- Edit > Lighting keeps the current executable's user-approved layout and interaction model. It is the authoritative product baseline, not a redesign target; the repository reference image is browser-fixture evidence until native recapture.
- Edit > Video mirrors that same major geometry and operator rhythm: upper media/clip authoring plus current/next truth, lower-left composition/output canvas, and lower-right layer/transform/composite/effect inspector. It also keeps the shared Timeline expandable rather than making it a peer workspace.
- The authoritative Video object model is defined in `qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md`: reusable media assets feed clip slots on multiple layers; clip, layer, transition, composition, and output effects remain explicit scopes; clip-take and layer-to-layer transitions are separate first-class systems.
- `VJ Desk` is not a separate authoring island: its authoring features belong to Edit > Video, while its performance features belong to Control > Video.
- Scene, video, timeline, and effect editing use the same selection model and undo history.
- Advanced fixture, effect, transform, and automation parameters use progressive disclosure and do not duplicate live controls.

## Control workspace

### Lighting region

- The first view is a scene/cue matrix grouped into explicit banks.
- GO is the dominant action. Back and Release remain adjacent but visually secondary.
- The selected cue exposes only the most relevant fixture, intensity, and color controls.
- FX construction, advanced beam parameters, patching, and diagnostics live behind contextual Tools or More disclosures.

### Video region

- Import Media remains visible after the first clip is created.
- A clip pad exposes Launch, Preview, and Details. Stop, audio monitor, deck assignment, trim, effects, and transform belong to the selected-clip inspector instead of every pad.
- Preview and Program are visually distinct and connected by one Take action.
- The operator can always tell whether a clip is selected, staged, live, stopped, or unavailable without opening a dialog.

### Output and inspector region

- HDMI 1 and HDMI 2 are first-class output cards.
- Each output represents three independent truths: Enabled/Disabled, Clear/Blackout, and Window Open/Closed/Unchecked/Not applicable.
- Close Window remains reachable whenever a Display output is selected; it is not hidden merely because a status poll is stale.
- One contextual inspector follows the current lighting cue, video clip, layer, or output selection. Advanced groups are closed by default and preserve operator context when opened.

## Unified timeline

- Lighting blocks, video clips, audio waveform, beat markers, and the shared playhead use one time axis.
- A compact read-only show-progress strip remains visible in Control without dominating the show surface.
- Detailed timeline authoring belongs to Edit; Control provides one direct Open Timeline navigation action.

## Interaction contract

- One primary action per region.
- No new icon-only action without an accessible name and a nearby state explanation where ambiguity is possible.
- Existing typography, icons, controls, spacing, and hit targets are not reduced to make features fit.
- Desktop interactive controls target at least 40 px; coarse-pointer targets target at least 44 px.
- Space pressure is solved with reflow, internal scrolling, pagination, or closed disclosure. The application shell must not gain an outer scroll bar.
- Destructive or irreversible operations use the existing accessible confirmation primitive. Routine live operations do not gain unnecessary confirmation dialogs.
- Errors appear beside the operation or output that failed and never replace authored state with a misleading generic `Off` label.

## Responsive contract

- 1920x1080 and 1920x1032 show the full Both workspace.
- 1366x768 and 1280x720 preserve all primary actions without clipping or overlap. The contextual inspector becomes a closed drawer or internal-scroll region before any control is reduced.
- Lighting and Video emphasis modes may allocate more width to their primary region, but navigation, Show Clock, health, blackout, and unified timeline remain authoritative.

## Acceptance evidence

- No unreachable Import Media route after clips exist.
- A normal clip pad has no more than three exposed actions.
- HDMI output cards show the three independent state axes truthfully.
- GO, Back, Release, Take, Blackout, Import Media, and Close Window remain reachable at every supported viewport.
- Focus order and keyboard operation follow visible order; icon-only controls have names; reduced-motion preferences are respected.
- Browser viewport gates are necessary but do not replace the required maximized native Syndocal verification.
