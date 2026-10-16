# Syndocal competitive review - 2026-10-16

- Task 9 Part B / Task 8. Shared `main`, base `8959408d79d1b2ca4fcc512f7678f8cf2a829f75`.
- Docs-only review, committed separately under Task 10 Part A. No product edits, rebuild, version bump,
  installation, sign-in, payment, competitor benchmark or CI wait.
- Generators and ten blend modes are already Syndocal features. The older
  [SynapseRack comparison](SYNAPSERACK_FEATURE_COMPARISON_2026-10-15.md) describes their
  earlier absence; those two gap conclusions are superseded here.

## Answer to the owner

Syndocal cannot yet be called complete alongside commercial VJ software. Performance
has improved substantially, but there is no measured cross-product winner or parity
claim. Its feature strengths are the integrated lighting/video show model, explicit
output ownership and standby policy; clip banks, transitions, layered effects,
generators and common blends now cover the basic categories. Independent group-image
controls, wider shader compatibility and ordinary NDI/Syphon delivery remain gaps
against the documented workflows below. Operability is the first priority: native
first-show staging required a manual retry, media health messages disagree in prominence,
and essential VJ controls disappear behind scrolling or workspace changes on a laptop.
The fixed desk and explicit safety state are useful, but the observed UI does not yet
justify an overall usability lead. Competitor documentation establishes available
workflows, not superior measured usability, reliability or physical output.

## Evidence and method

- **V = verified (hands-on this review)**; **D = documented** in current vendor
  material, inspected repository code, or explicitly dated prior QA; **U = unknown**.
  These markers apply to product cells throughout. A verified panel is not a verified
  end-to-end task. Partial routes explicitly say what stopped.
- Click/step counts are **logical route steps**, not elapsed-time benchmarks or a
  recording of every exploratory click. One drag, selection or field entry is one
  step; opening a menu and choosing its item are two. Scrolling, typing characters,
  startup, tool attachment and automation recovery are excluded. `>=` means a partial
  route; `U` means no defensible complete count. Documented counts are reconstructions.
- Discoverability and live suitability are reviewer judgments about the cited route,
  not novice-user research or show acceptance. `K` = keyboard; `M` = MIDI.
  Keyboard focusability does not prove a dedicated shortcut. No physical MIDI event
  was sent. Device-dependent parts remain U even when Learn mode was visible.
- Resolume Arena: **7.28.1 rev 26535, 6 October 2026**, listed on the official
  download page at review time [R0]. Public material only; not installed.
- VDMX: current **VDMX6** manual; Plus-only integrations are qualified [V0].
  Public material only; no Mac session. Some rendering-manual examples still mention
  OpenGL/QC; the VDMX6 migration page explicitly identifies the Metal migration [V10].
- SynapseRack: installed **3.3.0 Demo**, existing PID **18060**, responsive.
  Current manual is D and may exceed the installed build. Fresh screenshots are
  `target/qa/competitive-review/synapse-*.png`; earlier build-122 provenance is
  in [Tier C](SYNAPSERACK_TIER_C_2026-10-15.md#software-and-hashes).
- Syndocal: no process was running initially. Launched only this checkout's release
  executable, PID **44000**, one responsive maximized main window. SHA-256
  `C03967D4742D8B38D01072B0066D31A6FBD2AA1235B6617FA6C27EE1BD5E32E6`,
  matching the accepted [generator build](VIDEO_GENERATORS_2026-10-16.md).
  No newer build was needed. Native Computer Use JS was unavailable; existing
  process-bound WebView2 CDP plus Win32 observation/input were used. No mocked IPC.
- All new evidence, scripts, shader fixture and review-only show files are under
  `target/qa/competitive-review/` (ignored). JSON snapshots record actual published
  state, UI text and output permissions. File-dialog focus/handle retries are harness
  setup attempts, excluded from workflow counts and from product failure claims.
- Syndocal stayed disarmed; no Display, lighting, external sender, audio input or
  controller was enabled for this review. SynapseRack's Master and Mapping destinations
  were both None; mapper Display Off. Its documented always-on local Spout publication
  is a separate vendor behavior [S8], not an observed peer test.

## Task walkthroughs

### Syndocal native

- Start state: empty disarmed project, Edit > Video. Import uses the existing silent
  H.264 1080p30 fixture. A Solid generator supplies the second A/B source.
- Evidence keys below name JSON files inside `target/qa/competitive-review/`.

| Task | Route, count and observed result | Discoverability / live suitability | Keyboard / MIDI |
| --- | --- | --- | --- |
| Load and trigger | V: Choose Media & Set Up -> filename -> Open -> P retry -> Play: 5 steps. Library, layer, bank and disabled Program output created. `native-imported`, `native-preview`, `native-play`. | V: labelled entry; first automatic Preview failed. P retry succeeded; old error remained. Staging is suitable; import should be prepared before a show. | V: buttons/fields exposed. D: normal focus navigation. U: a complete mapped import/trigger hardware path. |
| Crossfade / transition | V: assign existing clip to A, Solid to B, move A/B Mix: 3 steps after sources exist. `native-decks`, `native-ab-mixed`. D: bank Take supports Cut/Crossfade/Dip/Wipe/Luma/Displacement/Blur/Glitch/Custom. | V: A/B is below the clip grid and needs scrolling in this layout. U: active-output transition pixels; no output enabled. | D: MIDI action mappings exist. U: specific crossfader/button hardware mapping and dedicated shortcut. |
| Opacity / blend | V: move layer opacity to 0.5; select Overlay: 2 steps. `native-opacity`, `native-blend`. | V: explicit fader and ten options; layer panel is narrow. Suitable for a prepared desk; controls must remain visible. | D: native inputs are focusable. U: hardware response and complete target coverage. |
| Layer FX, then whole output FX | V: built-in selection -> Advanced -> select parameterized stage -> edit -> bypass: 5 steps. Invert and Posterize exercised; levels=5. Output: scoped disclosure -> Output disclosure -> Add ISF -> filename -> Open -> Parameters -> gain -> bypass: 8 steps. `native-layer-tweaked`, `native-fx-bypassed`, `native-output-fx-tweaked`, `native-output-fx-bypassed`. | V: scoped output chain works, but uses file import where layer offers a built-in selector. Scope/parameter disclosures and scrolling add live navigation. U: output pixels and visual audition. | D: button/input navigation. U: same K/M mapping coverage for every layer/output parameter. |
| BPM sync | V: three Tap presses; enable layer BPM sync: 4 steps. BPM changed from 120 to 119; published `bpm_sync.enabled=true`. `native-tap`, `native-bpm-sync`. | V: Tap is persistent; layer sync is deeper in controls. D: beat-related controls exist. U: observed beat-synced visual parameter motion. | V: Tap/button and checkbox exposed. U: physical clock, mapped Tap or synced parameter hardware. |
| Audio-reactive parameter | V: Reactive drawer, 1 step: no mappings; tells operator to load a project containing an audio Node Graph mapping. `native-reactive`. D: Audio -> transform -> video target graph exists. Complete creation count U. | V: existing graph status is easy to inspect; no direct creation action in this empty drawer. Prepare graph before performance. U: live signal/result. | D: graph controls and source configuration. U: live audio/device and per-parameter K/M reachability. |
| MIDI Learn fader and button | V: enter Learn in 1 step; pink-target instruction and Esc shown. `native-midi-learn`. Complete fader/button learn count U without a controller. | V: explicit mode banner; must leave Learn before performing. D: target selection precedes incoming event. | V: Escape exits. U: actual CC/note binding, feedback and coverage of the desired two controls. |
| Second output / mapping | V: first-show wizard authored one disabled Display; blocked status visible. D: Setup > I/O > Video, output geometry/mapping/edge blend controls. Complete second-projector count U. | V: disabled output is explicit. D: configure while disarmed. U: second-display assignment and keystone/edge-blend result. | U: complete K/M mapping workflow for output calibration. |
| Emergency blackout / recovery | V: VID clicked twice, 2 attempted steps; both rejected without the required active lease and applied nothing. `native-blackout`, `native-blackout-recovered`. D: separate Video/DMX/All blackout controls. | V: always-visible target and specific refusal. U: emergency blackout/recovery while Active; this is correct disarmed rejection, not a failed blackout on an armed show. | D: global/action routes exist. U: physical mapped blackout and recovery. |
| Save / recall / backup | V: menu -> Save As -> filename -> Save: 4; menu -> Load -> filename -> Open: 4. Own `.sdc` saved; modified-path copy loaded with authored FX/state. Recovery and backup entries inspected. `native-saved`, `native-save-updated-state`, `native-missing-media`. | V: explicit unsaved-change dialogs. D: autosave/backups and ShowClock standby. U: crash restoration and second-machine takeover. | D: project shortcut inventory supports save/open. U: MIDI project-load/backup policy and physical redundancy. |
| Missing media / crash | V: own saved show copied with an intentionally nonexistent path, loaded, warning observed; library disclosure -> Relink -> filename -> Open: 4 recovery steps. Original path restored in published state. `native-missing-media`, `native-relink-controls`, `native-relinked`. | V: relink exists and works. Load warning says missing while library says Not checked and clip card says READY; health meanings need clearer presentation. U: live decoder crash/last-frame behavior. | D: dialog/button navigation. U: dedicated K/M recovery mapping; process crash recovery. |

### SynapseRack native

- Existing four-layer template, MediaBrowser with bundled clips, and running sequencer.
  Fresh observations do not establish a clean-show default or beginner click-time ranking.
- Fresh save `synapse-review.synapse` (30,459 bytes) was recalled through Project >
  LoadProject. No existing show file was overwritten. The 14:58 restoration capture
  records cleared review content, Add blend, MIDI assignment off, Audio category and
  outputs None/Off. Later folder-import attempts (captures 32-35) and a restart
  (capture 36, PID 44392) postdate that capture; final project/panel restoration
  after those attempts is unverified. Fresh-folder import remains U.

| Task | Route, count and observed result | Discoverability / live suitability | Keyboard / MIDI |
| --- | --- | --- | --- |
| Load and trigger | V: layer preview -> library thumbnail: 2 clicks; bundled H.264 plays on Layer_0. `synapse-02-layer`, `03-media`. D: file/folder import via OS drag or MediaBrowserCache > ImportFromDirectory [S1]. Fresh-file import count U. | V: quick after destination is understood; selecting versus assigning shares the thumbnail action. D: assignment resets selection [S1]. Prepare library before show. | D: keyboard/MIDI assignment [S7, S10]. U: triggering from hardware. |
| Crossfade / transition | V: inspected layer transport/FX catalog; no complete A/B graph built. D: connect two textures to CrossFade and vary Mix [S3]. Complete count U; graph setup is not one clip click. | D: patch before show; then one Mix gesture. U: live transition and recovery. | D: Mix is a graph parameter with mapping mechanisms [S3, S7]. U: tested controller. |
| Opacity / blend | V: one fader gesture plus dropdown -> AlphaBlend: 3 steps; source continued playing. `synapse-05-opacity-drag`, `06-blendmenu`, `07-blend-changed`. | V: controls adjacent to preview; existing sequencer changes opacity, so exact manual 50% result U. D: numeric opacity readout is scaled incorrectly [S2]. | V: fader selectable in Learn overlay. D: K/M assignment [S7]. U: physical value/feedback. |
| Layer FX, then whole output FX | V: Fx category inspected; Fx Apply and Bloom tiles present (`synapse-26-fx-category`). D: image/effect graph and FxIn/FxOut custom chain [S3]. Complete layer/master application/tweak/bypass count U. | V: graph canvas already overlapped by panels. D: configure graph before show. U: reliable complete graph editing/render result in this run. | D: node parameters can be assigned [S7]. U: tested layer/master bypass buttons. |
| BPM sync | V: Window -> GlobalTempo -> TAP: 3 steps to first tap, existing BPM 207. `synapse-14-globaltempo`, `15-tap`. D: layer Beat Sync uses this tempo [S4]. Parameter sync completion U. | V: tempo can be summoned centrally; it covers the library. D: prepare sync wiring first. | D: six tempo buttons support MIDI [S4]. U: hardware sync/phase. |
| Audio-reactive parameter | V: AudioReactive catalog inspected (`synapse-01-initial`, `31-restore-audio-category`). D: create node -> choose device -> connect Bass/Hit to target [S5]. Completed count U; no audio device opened. | D: patch creation before show; monitor/threshold adjustment live. U: signal/result. | D: assignment mechanisms [S7]. U: specific learned target and device response. |
| MIDI Learn fader and button | V: Setting -> MidiMappin -> opacity target: 3 steps to selected red fader. `synapse-09-midi-learn`, `10-midi-fader`. D: move controller; repeat for button [S7]. Complete binding U. | V: large blue target overlays and MIDI Inspector; layer window can cover Inspector. Leave mode before performing. | D: single-key bindings, MIDI CC/note [S7, S10]. U: physical fader/button event. |
| Second output / mapping | V: mapper open, no quads; output menu enumerates one monitor, Master None / Mapping None. `synapse-27-output-menu`. D: Window -> SynapseMapper -> +Quad -> source/corners -> Output Mapping target, >=5 steps [S6]. | V: available destination is explicit. D: direct selector allows one active output; plan calibration off-air [S6]. U: second projector and edge blending. | D: detached window shortcuts [S10]. U: complete keyboard/MIDI calibration. |
| Emergency blackout / recovery | V: Output menu inspected, 1 step; both destinations already None. D: chosen output can be toggled off [S6]. Full stop/restore count U; none was activated. | V: no dedicated global blackout was demonstrated. D: output toggle is different from a retained black frame. U: blackout/recovery under load. | D: output-window OS shortcuts [S10]. U: dedicated emergency K/M mapping. |
| Save / recall / backup | V: Project -> SaveProject -> filename -> Save, 4 steps; Project -> LoadProject -> filename -> Open, 4. `synapse-18-save-dialog-front`, `25-after-recall`; file retained. D: earlier abnormal-exit recovery prompt recorded in prior QA. | V: native dialogs interrupt performance. U: crash restore, autosave guarantees and paired backup machine. | D: official shortcuts explicitly say Ctrl/Cmd+S is not assigned [S10]. U: MIDI save/recall. |
| Missing media / crash | V: no original sample was moved or deleted. D: missing thumbnail/badge and unavailable-codec behavior [S1]; sender loss holds last image without warning [S8]. Complete recovery count U. | U: missing-file relink and process-crash recovery under performance. Do not infer these from a source menu. | U: recovery K/M actions. |

### Resolume Arena - reconstructed from official documentation

| Task | Documented route and count | Discoverability / live suitability judgment | Keyboard / MIDI |
| --- | --- | --- | --- |
| Load and trigger | D: drag file into deck -> thumbnail, 2 steps after browsing; Files/clip grid [R1]. | D: direct grid affordance; browse/import before show. U: novice success. | D: keyboard/MIDI clip shortcuts [R5]. |
| Crossfade / transition | D: View -> Show Layer Transition Controls -> time -> next clip, 4; Layers panel can set type [R2]. | D: one trigger after preparation; discovering hidden transition controls needs View knowledge. | D: previous/next and clip triggers are assignable [R2, R5]. |
| Opacity / blend | D: V slider -> blend menu -> mode, 3; layer controls [R2]. | D: adjacent to clip grid, useful live; actual ergonomics U. | D: parameter shortcuts [R5]. |
| Layer FX / whole output FX | D: Effects panel -> drag to Layer/Composition -> parameter -> Bypass, 4 per scope [R3]. | D: direct scope drop; Alt-drop at zero opacity supports gradual introduction. | D: effect/bypass parameters assignable [R3, R5]. |
| BPM sync | D: Tap -> parameter cog -> BPM Sync -> beat duration, 4 [R4]. | D: per-parameter menu; tune timing before performance. | D: BPM/parameter shortcuts [R4, R5]. |
| Audio-reactive parameter | D: cog -> FFT source -> band -> gain/fall, >=4; external device/channel adds setup [R4]. | D: contextual parameter workflow; external routing is split across preferences. | D: parameter shortcuts [R5]; actual audio/hardware U. |
| MIDI Learn fader/button | D: Shift+Ctrl+M -> target -> move CC -> target -> press note -> Esc, 6 after device setup [R5]. | D: prepare mappings before show; shortcut editor changes interaction mode. | D: keyboard and MIDI editors [R5]. |
| Second output / mapping | D: Advanced Output -> add screen -> destination -> slice -> output transform -> points; >=6 plus edge-blend adjustment [R6]. | D: separate calibration workspace; prepare before show. U: physical result. | D: Ctrl+Shift+A opens output; arrow/shift-arrow point nudging [R5, R6]. U: MIDI mapping geometry. |
| Blackout / recovery | D: composition master opacity to zero -> restore, 2 gestures; output disable is a separate action [R2, R5]. | D: prepare a master shortcut and known recovery value. U: guarantee that all routed slices honor master bypass settings. | D: mapped master; Ctrl+Shift+D disables output [R5]. |
| Save / recall / backup | D: Ctrl+S save, Ctrl+O + file selection recall, 1 / >=2; collect media separately [R5, R7]. | D: prepare composition plus mapping and media package. U: paired fenced takeover. | D: project shortcuts [R5]. U: safe MIDI project-reload policy. |
| Missing media / crash | D: red clip Relocate -> file -> fix siblings, >=3; Media Manager [R7]. | D: visible missing state and batch relink help. U: decoder/process-crash continuity. | D: dialog navigation. U: MIDI recovery. |

### VDMX6 - reconstructed from official documentation

| Task | Documented route and count | Discoverability / live suitability judgment | Keyboard / MIDI |
| --- | --- | --- | --- |
| Load and trigger | D: use Simple Mixer template -> import page media -> bin trigger, >=3 [V1]. | D: template supplies a starting layout; target-layer setup matters. | D: bin triggers can receive control sources [V3, V4]. |
| Crossfade / transition | D: choose two layers in mixer -> crossfader, 3 after plugin exists [V4]. | D: one gesture once prepared. | D: generic receivers [V3]; hardware result U. |
| Opacity / blend | D: inspect layer -> opacity -> composition-mode choice, >=3 [V2]. | D: detachable components can stay visible. U: default laptop ergonomics. | D: UI receivers for sliders/popups [V3]. |
| Layer FX / whole output FX | D: inspect layer or Canvas -> Load Asset -> parameter -> On/Off, 4 per scope [V2]. | D: shared chain model; setup can be exposed for performance. | D: generic UI receivers [V3]. |
| BPM sync | D: Clock Tap -> inspect receiver -> choose clock-derived source, >=3 after plugin setup [V3, V4]. | D: requires prepared plugin/receiver layout; actual sync U. | D: internal/MIDI Follow clock, Link documented [V4]. |
| Audio-reactive parameter | D: Audio Analysis source/filter -> target receiver, >=3 after plugin setup [V3, V4]. | D: modular setup; convenient once bound. U: input/result. | D: same receiver system [V3]. |
| MIDI Learn fader/button | D: Cmd+L -> fader -> move CC -> button -> press note -> Cmd+L, 6 after device setup [V3]. Inspector Receiving is an alternative, not a required extra route. | D: global blue-target Learn mode; leave it before performing. U: measured ergonomics. | D: MIDI/OSC/DMX and boolean/decimal receiver types [V3]. |
| Second output / mapping | D: Cmd+F -> Advanced -> add window -> source -> region -> screen/geometry, >=6 [V5]. | D: explicit output regions. U: native multi-projector edge blending; don't equate layer quad warp with edge blend. | D: Cmd+F / Cmd+Shift+F; U: complete MIDI calibration [V5]. |
| Blackout / recovery | D: Workspace Inspector master zero -> restore, 2 gestures [V2]. | D: map/preposition before performance; active-output behavior U. | D: slider receiver [V3]. U: dedicated emergency shortcut. |
| Save / recall / backup | D: create/update workspace preset -> Cmd+S; load project -> preset thumbnail, >=2 / >=2 [V6]. | D: distinction between updating preset and only saving project must be understood. | D: Cmd+S and preset boolean receiver [V6]. U: fenced backup/takeover. |
| Missing media / crash | D: Workspace > Relink Missing Media -> Find -> result/Browse, >=3; optional Spotlight fast relink [V1]. | D: batch tools documented; ambiguity and actual crash continuity U. | D: normal dialog access. U: MIDI recovery. |

## UI quality and concrete Syndocal findings

- These are observations and design judgments, not an accessibility certification.
- Required Syndocal screenshots (native release WebView, CDP viewport emulation,
  **CSS pixels**, device scale factor 1; physical editor remained maximized):
  - Edit > Video: `target/qa/competitive-review/syndocal-1920x1080.png`
    and `syndocal-1366x768.png`.
  - Control > Video: `target/qa/competitive-review/syndocal-control-video-1920x1080.png`
    and `syndocal-control-video-1366x768.png`.
  - Also captured Control > Lighting at both sizes and missing-media Edit at 1920x1080.
  - Matching JSON records `innerWidth/innerHeight` and document scroll dimensions.
    These are actual native UI captures, not a browser fixture; they do not establish
    Windows DPI/window-resize or projector behavior at those physical resolutions.
- **1920x1080 Edit:** monitor band occupies roughly y=126..518; three operational
  columns y=520..750; lower context y=754..1052. Layer controls receive only about
  x=1496..1905 (409 px), with blend/FX below the first visible transport row.
- **1366x768 Edit:** operational row is about y=374..517 (143 px); the layer
  column x=1087..1353 (266 px) shows its heading and Add generator, with live
  layer fader/blend below the viewport of that pane. Lower context still occupies
  y=520..741. Reflow/disclosure should reclaim this space without smaller controls.
- **Control > Video:** at 1366x768, the upper task surface ends near y=424.
  The bank header/settings consume y=185..307 and only one complete slot row is
  visible. The persistent lower lighting Stage/faders occupy y=430..741 despite
  zero fixtures. Output previews/opacity/FX are not simultaneously visible at the
  top. At 1920x1080 there is more bank space, but it still needs scrolling.
- **Errors:** first-show retry succeeded without clearing its previous error.
  The missing-media load warning was accurate; library availability still read
  Not checked and the clip tile READY. Preserve separate technical meanings, but
  show one clear operator health summary and an explicit recovery completion.
- **FX:** layer built-ins are directly selectable; output scope initially offers
  only Add ISF. Successful output gain edit/bypass proves that the capability
  exists. The inconsistency is in access and scope visibility.
- **Reactive:** the drawer exposes existing graph health, but its empty state
  sends the performer to another project rather than starting a mapping.

| Dimension | Syndocal | Resolume Arena | VDMX6 | SynapseRack Demo |
| --- | --- | --- | --- | --- |
| Density | V: large monitors plus narrow operational panes; compact layout hides frequent controls. | D: clip grid/layer controls with configurable panels [R2, R8]. U: hands-on density. | D: user-built plugin/window layout [V9]. U: hands-on density. | V: four layer previews, library, mapper and node canvas compete; overlaps visible. |
| Visible while performing | V: top BPM/blackout persistent; Edit and Control expose different portions. | D: monitors can duplicate for layer/group/composition; dashboard [R8]. | D: components can detach; custom Control Surface [V2, V4]. | V: transport/opacity/blend adjacent; mapper/Inspector can cover other panels. |
| Mode switching | V: Setup/Edit/Control plus domain and control-mode choices; extra navigation for setup versus performance. | D: panel selection and shortcut-edit modes [R5, R8]. | D: contextual inspectors and prepared templates [V1, V9]. | V: mapping overlays replace ordinary interaction; movable internal panels. |
| Consistency | V: clear labels, but layer/output FX access differs. | D: common parameter animation/shortcut pattern [R3, R4, R5]. U: complete consistency. | D: shared slider/button/popup inspectors [V3]. | V: similar layer strips. D: group strip includes inactive controls [S2]. |
| Undo | V: project menu labels action and count. D: durable transaction history. U: every live action. | D: mouse edits undo; triggers/external input excluded; output history separate [R8]. | D: workspace presets/revert [V6]. U: comprehensive edit undo. | D: pointer-scoped graph/widget/layer histories; live faders excluded [S10]. Layer deletion wording conflicts with older Layer guide [S2]; installed enforcement U. |
| Errors | V: specific refusal/missing warning; stale message and mixed readiness labels reduce clarity. | D: red missing clip + Relocate [R7]. U: crash visibility. | D: missing icon and relink tools [V1]. U: crash visibility. | D: missing/bad-codec badges [S1]; sender freezes without warning [S8]. V: scaled opacity readout observed, exact value U. |
| Localization | D: EN/JA contract and prior rendered checks [G]. V: English UI has some Japanese window/audio labels in captured controls. | U: complete app localization; English manual/UI examples do not establish it [R1]. | U: full localized UI; English documentation alone is insufficient [V0]. | V: English menus; Japanese OS dialogs. D: MediaBrowser labels stay English [S1]. |
| Accessibility | V: many named native DOM controls and Learn mode instructions; no screen-reader audit. | D: shortcut editing and contextual Help [R5, R1]. U: screen reader/contrast compliance. | D: inspector/receiver controls [V3]. U: screen reader/contrast compliance. | V: custom visual controls; semantic desktop targeting not established. D: single-key mappings [S10]. U: assistive-tech compliance. |
| Laptop / small screen | V: no outer overflow in captured metrics; internal scrolling and unused lower lighting surface remain. | D: hide/move/undock panels and presets [R8]. U: 1366x768 render. | D: templates and window arrangement [V9]. U: 1366x768 render. | V: overlapping panels even on this desktop. D: detach/center/resize controls [S10]. U: 1366x768 render. |

## Four-product features

- Same areas as the earlier SynapseRack table. D for Syndocal source/QA means
  implemented/documented, not newly demonstrated device or transport acceptance.
- Similar feature names do not establish pixel equivalence. Unknown means unknown,
  not absence. General image graphs and parameter-modulation graphs are distinct.

### Clip/deck/layer model

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Clip/bank/layer | V: assets, 32-slot bank, layers, Preview and A/B. | D: decks, columns, layer clips [R1, R2]. | D: media pages/bins and layers [V1, V2]. | V: layers and library assignment; D: folder import [S1]. |
| Group image | D: contiguous groups; scoped group-image FX. Independent group opacity/blend/transform absent in current model [L]. | D: sub-composition controls/FX/transforms [R10]. | D: hierarchical group composition/FX [V2]. | D: composite group then blend/opacity/transform [S-group]. |

### Composition and blend

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Blend operations | V: ten labels; Overlay publication. D: Normal/Add/Multiply/Screen/Overlay/Subtract/Difference/Lighten/Darken/SoftLight [B]. | D: blend and transition families [R2]. U: pixel equality. | D: composition modes [V2]. U: exact catalog/pixels. | V: eight names including AlphaBlend/Substract/Cutout [S2]. U: pixel equality. |
| Image graph | D: ordered video layers; graph modulates parameters, not arbitrary image ports [L]. | D: Video Router [R-source]. U: included graph-authoring equivalence. | D: layer/group sources and feedback routing [V2]. | D: typed image-node graph [S3]. |

### Effects and generators, including ISF/shaders

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Effect scopes | V: layer + output edit/bypass. D: 14 built-ins; clip/group/composition scopes, up to 32 stages [L]. | D: clip/layer/group/composition, over 100 built-ins, FFGL [R3]. | D: layer/group/canvas chains [V2]. | V: FX catalog; D: node/custom chains [S3]. |
| Shader import | D: single-image ISF; explicit rejection of multipass/persistent/imported/audio-texture cases [L]. | D: FFGL; ISF in Wire is a distinct integration [R3, R-isf]. U: universal ISF import into Arena. | D: ISF generators/filters and multipass ecosystem [V7]. | D: ISF/Shadertoy conversion to .srfx; service export; D3D11/Metal [S-fx]. |
| Pure generators | V: Solid created; D: gradient + deterministic 30 Hz noise landed. Text omitted [G]. | D: sources generate without a media clip [R-source]. | D: ISF/other built-in sources [V7, V2]. | D: Gradient node; other generator parity U [S-gradient]. |

### Transitions

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Switching / crossfade | V: A/B opacity mix. D: nine Take styles and musical durations [L]. | D: automatic layer transitions [R2]. | D: two-channel mixer [V4]. | D: CrossFade image node and SDK mixer [S-crossfade, S11]. |
| Loop seam | U: equivalent automatic seam overlap not established [L]. | U: equivalent seam behavior in reviewed pages [R2]. | U: equivalent seam behavior in reviewed pages [V2]. | D: crossfade-loop option [S12]. |

### Output, mapping and projection

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Multi-output / mapping | D: independent composition/output IDs, monitor identity, native routes [L]. V: disabled route status. | D: screens/slices/transforms and multi-projector edge blending [R6]. | D: advanced per-window source/region/output [V5]; U: edge-blend equivalence. | V: mapper and destinations; D: quad/perspective/grid warp; one direct output at a time [S6]. |
| Spout / NDI / Syphon | D: default Windows Spout; NDI gated/licensed overlay; Syphon NotBuilt [L]. | D: local Spout/Syphon and network NDI [R11]. | D: Syphon/NDI plugins [V5]. U: Spout on Mac. | D: Windows Spout, Mac Syphon, both NDI [S8]. |

### Audio-reactive and BPM/tempo

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Audio parameter drive | V: empty Reactive status. D: band/onset/features to lighting/video graph [L]. | D: internal/external FFT parameter animation [R4]. | D: Audio Analysis sources [V4]. | D: AudioReactive loudness/bands/hits [S5]. |
| Tempo / detection | V: Tap and layer sync state. D: live BPM estimate/confidence, MIDI clock input [L]. U: network Link interoperability. | D: Tap/BPM synchronization [R4]; Link [R-link]. | D: clock, MIDI follow, detection, Link [V4]. | V: tempo/TAP UI; D: newer BPMDetect catalog [S-bpm]. U: installed detector; old guide conflict. |

### MIDI/OSC/DMX/timecode control

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| MIDI / OSC | V: Learn mode; D: mapping/action/OSC learn paths [L]. | D: keyboard/MIDI/OSC shortcut systems [R5]. | D: receiver/sender system [V3]. | V: Learn overlay/fader selection; D: OSC mapping [S7]. |
| Lighting / DMX | D: fixtures, cues, venue output ownership, Art-Net/sACN input [L]. | D: pixel/fixture DMX output [R-dmx]. U: equal theatre-lighting cue workflow. | D: Video-to-DMX/data I/O [V4, V3]. U: equal fixture/cue engine. | D: OFL Fixture/Art-Net nodes; not video-only [S13]. |
| Clock / MTC / LTC | D: MIDI-clock receive, MTC I/O, LTC capture/decode [L]. | D: Arena SMPTE input [R12]. U: complete MTC I/O equivalence. | D: cue-list MIDI/MTC/LTC synchronization [V4]. | D: MIDI-clock output; older guide denies receive [S4]. U: MTC/LTC parity. |

### Recording

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| File capture | D: selected output, H.264/optional AAC, atomic artifact publication [L]. | D: composition/advanced-output recording [R-record]. | D: Movie Recorder [V4, V5]. | D: graph FileRecorder MP4 [S9]. |
| Retrospective / resampling | U: equivalent retrospective/granular buffer [L]. | D: recorded clip can be reimported [R-record]; U: equivalent retrospective buffer. | D: recorder exists [V4]; U: equivalent retrospective buffer. | D: DumpSource/DumpTrigger [S9] and Buffer Granular [S-buffer]. |

### Media formats and codecs

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Files / images | V: H.264 import/play/relink. D: HAP, HEVC/ProRes regressions, PNG/JPEG [L]. | D: recommends DXV GPU decode [R9]. U: all-codec parity. | D: AVFoundation + HAP, H.264, ProRes [V8]. | V: sample H.264. D: HAP/H.264 guidance; Windows HEVC/VP8/VP9 rejected [S1]. |
| Live / capture inputs | D: camera/screen/transport availability is explicit; enum is not backend proof [L]. | D: live capture and network/local senders [R10, R11]. | D: capture and shared-video sources [V2, V5]. | D: camera/window/external-source catalog [S1, S8]. U: device recovery parity. |

### Automation and timeline

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Sequencing | D: unified lighting/video/audio timeline, cues and automation [L]. | D: parameter animation/autopilot [R2, R4]; U: unified show-timeline parity. | D: Cue List/data looper [V4]. | V: sequencer template; D: StepSequencer [S4]; U: unified show timeline. |
| Autonomous variation | D: AutoVJ with manual Take precedence/blackout retention [L]. | D: autopilot and parameter animation [R2, R4]. | D: receiver/automation patching [V3, V4]. | D: node/SDK automation [S3, S11]. U: identical AutoVJ policy. |

### Show safety and redundancy

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Blackout / source loss | V: disarmed blackout rejection. D: exact-black/freshness/ownership gates [L]. | D: master/output controls [R2, R5]. U: equivalent source-loss policy. | D: master fade [V2]. U: equivalent source-loss policy. | V: output selectors; D: sender freezes/reconnects by name [S8]. |
| Standby / recovery | V: persisted Standby, final empty/disarmed. D: authenticated ShowClock, stale latch/fencing [L]. U: two-PC physical takeover. | D: collect/relink [R7]; U: fenced paired takeover. | D: presets/relink [V6, V1]; U: fenced paired takeover. | V: save/recall; D: earlier recovery prompt [L]. U: fenced paired takeover. |

### UI and workflow

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Performance surface | V: fixed workspaces and context split, two-size captures. | D: custom layouts/detachment [R8]. | D: templates/custom surfaces [V1, V4, V9]. | V: movable panels; D: WidgetPanel and F10 detach [S10]. |
| Reuse / extensions | D: FX presets, bounded modulation graph, native MCP [L]. U: embedded app SDK equivalence. | D: FFGL plugins [R3]; Wire ISF workflow [R-isf]. | D: ISF; TouchDesigner/Vuo require Plus [V0, V7]. | D: HTML/JS Apps SDK and graph groups [S11, S3]. |

### Licensing limits of the Demo / packages

| Feature | Syndocal | Arena | VDMX6 | SynapseRack |
| --- | --- | --- | --- | --- |
| Licensing / capacity | D: MIT source; resources remain bounded; ASIO/NDI licensing separate [L]. | D: commercial product download [R0]. U: capacity ceiling from reviewed pages. | D: VDMX6/Plus split [V0]. U: capacity ceiling. | V: Demo 3.3.0. D: four layers/two groups, no other Demo feature restrictions [S0]. |
| Shipping boundaries | D: NDI overlay approval and Mac Syphon backend incomplete; ASIO separately licensed [L]. | D: Windows/macOS downloads [R0]. U: device compatibility here. | D: macOS product [V0]. U: native acceptance here. | D: Windows/macOS with platform transport differences [S0, S8]. U: macOS acceptance. |

## Performance - published claims beside existing Syndocal measurements

| Product | Codec / pipeline documentation | Stated layers / resolution limits | Evidence class |
| --- | --- | --- | --- |
| Arena | Recommends DXV because decompression runs on GPU; performance advice emphasizes prepared media [R9]. | U: no guaranteed layer count/resolution/fps combination established. Composition fps cap is a pacing setting, not capacity [R9]. | D: vendor guidance; no timings here. |
| VDMX6 | AVFoundation + HAP; HAP recommended for >=3 HD layers, H.264/ProRes reasonable for 1-2. Metal migration [V8, V10]. | D: 4K-and-larger HAP guidance; U: guaranteed maximum layers/resolution. H.264 guideline is not an app canvas limit [V8]. | D: vendor guidance; no timings here. |
| SynapseRack | HAP/H.264 recommended; Windows HEVC/VP8/VP9 rejected. Custom .srfx path documents D3D11/Metal [S1, S-fx]. | D: Demo four layers/two groups; Gradient FullHD/HD/SD choices are node presets, not a global maximum [S0, S-gradient]. U: full-product capacity/fps guarantee. | D: vendor claims; no benchmark. |
| Syndocal | libav decode-ahead; eligible >=2-layer native Display GPU composition. FX, active transitions, unsupported formats and other restrictions retain CPU path; recording/NDI/Spout/preview are different paths [P]. | D: bounded by device/resources; no universal layer/4K/60 guarantee. | D: prior measured results below, not fresh timing in this review. |

- [Tier C, 2026-10-15][T]: native **1080p** H.264 p95 for 1/2/4/8 layers:
  **17.912 / 18.406 / 19.559 / 23.357 ms**. HAP:
  **17.900 / 18.325 / 30.088 / 18.312 ms**. The historical four-layer HAP row
  used CPU composition; do not present it as the current threshold policy.
- Later [threshold-change rerun][P]: HAP x4 **p50/p95/p99
  16.960/17.844/18.269 ms**, N=3535, >16.7 ms=2420 (68.46%), >33.4 ms=0.
  H.264 x4 **16.865/19.847/20.461 ms**, N=3461, >16.7 ms=1908 (55.13%),
  >33.4 ms=6 (0.17%). Both showed four GPU layer slots.
- Native presentation intervals include vsync; percentages over 16.7 ms are not
  dropped-media-frame percentages or physical latency. HAP x4 improved from about
  36 to about 60 presents/s; H.264 p95 increased slightly. No significance claim.
- [Synthetic GPU completion benchmark][P], no decode/surface: 4K four RGBA layers
  p50/p95 **9.5/11.2 ms**; eight **18.6/21.5 ms**. These cannot establish a
  real 4K window plus effects/transitions/recording workload.
- [Generator provider][G]: noise mean/p95 **3.224/3.806 ms at 1080p**,
  **7.936/8.349 ms at 4K**. Provider-only, not total frame cost.
- No new competitor performance tests. All six cancelled SynapseRack measurements
  remain cancelled. Vendor advice and different benchmark tiers are not normalized
  into a performance score. No product-wide stability or release acceptance follows.

## One prioritized improvement queue

- Owner decision, 2026-10-16: adopted this order. Complete operability items 1-6
  first, then items 7-10: group-image controls, ISF, NDI and Syphon.
- Estimates are engineering scope judgments, not commitments: **S** 1-3 days,
  **M** roughly 1-2 weeks, **L** several weeks including acceptance; physical and
  cross-platform scheduling can add time. Validate observed issues before changing code.
- Queue change: put operator layout, first-run/error recovery and parameter/FX access
  before new integrations. Move independent group-image controls ahead of broad ISF
  expansion; keep NDI ahead of Syphon for the current Windows workflow. The four
  existing queue entries are retained as items 7-10. Generator/blend work is done.

| Rank | Priority / size | Item and user impact | Evidence / acceptance direction |
| --- | --- | --- | --- |
| 1 | P1 / M-L | Make a compact VJ performance layout: keep clip launch, Preview/Program, layer opacity/blend and blackout reachable together. Reduce navigation during a show. | Two-size Edit/Control captures above; [layout comparison][R8]. Reflow/reclaim irrelevant lower context; preserve typography/hit targets. |
| 2 | P1 / M | Fix first-show Preview settlement and stale error lifecycle. A successful retry must clear the obsolete failure. | `native-imported.json` -> `native-preview.json`; rerun first-show path on current native build. |
| 3 | P1 / M-L | Add a direct path from a parameter to BPM/audio/MIDI assignment, with source/owner feedback. Reduce graph/project detours and prove fader + button learn. | `native-reactive.json`, `native-midi-learn.json`; contextual workflows [R4, V3, S7]. Reuse existing graph/action authority. |
| 4 | P1 / M | Unify layer/group/composition/output FX access and safe audition/preset/bypass. Make the target scope conspicuous. | Native output Add ISF versus layer built-ins; [scope workflow][R3]. Retain working scoped chain implementation. |
| 5 | P1 / M | Unify media health and recovery completion; keep Verify/Relink/backup access near the failed clip. Prevent READY/Not checked/missing ambiguity. | `native-missing-media.json`, `native-relink-controls.json`, `native-relinked.json`; [batch relink][R7]. Relink already works; do not rebuild it unnecessarily. |
| 6 | P1 / M-L | Publish and validate a realistic performance envelope including FX/transitions and 4K native output. Give operators supported limits and visible budget/fallback status. | [Current GPU eligibility and measured boundaries][P]. This is Syndocal acceptance work; no cancelled competitor benchmark restart. |
| 7 | P2 / L | Independent group-result opacity/blend/transform. Move a prepared multi-layer look as one image. | [Current group model][L] versus [Arena groups][R10], [VDMX groups][V2], [Synapse group][S-group]. Preserve scoped group FX; define nesting separately. |
| 8 | P2 / L | Expand ISF compatibility in bounded stages: explicit multipass/feedback/import/audio cases and resource limits. Enable reusable creative content. | [Current rejection boundaries][L]; [VDMX ISF][V7], [Synapse conversion][S-fx]. Keep unsupported/future input rejection. |
| 9 | P2 / M-L | Complete approved NDI distribution/licensing and real sender/receiver acceptance. Make the ordinary package useful in networked VJ rigs. | [Package boundary][L]; [Arena NDI][R11], [Synapse exchange][S8]. Distribution and interoperability are separate gates. |
| 10 | P3 / L | Implement/accept macOS Syphon for Mac-based shows. Retain explicit NotBuilt until it works. | [Current backend boundary][L]; [VDMX output][V5], [Synapse exchange][S8]. Requires Mac/device evidence; lower current Windows impact. |

- P3 investigation backlog after those ten: text generation, loop-seam overlap and
  retrospective/granular capture. Confirm demand and current support first. These
  are not substitutes for operator workflow closure or requests to implement now.

## Limits, cleanup and validation

- This is an evidence-based review with partial hands-on routes, not eleven fully
  passed tasks in both applications. SynapseRack complete crossfade graph, scoped FX,
  audio graph, missing-file recovery and crash recovery were not demonstrated.
  Existing custom UI and overlapping canvas did not yield a reliably verified
  complete graph-edit route; those cells remain D/U, not feature absences.
- No physical MIDI/OSC/DMX, audio device, external video peer, second monitor,
  projector/LED calibration, edge blend, armed blackout/recovery, Mac session,
  two-PC takeover or process-crash drill. No accessibility compliance claim.
- Native UI screenshots use emulated CSS viewport sizes. Desktop Win32 captures
  additionally prove the observed maximized application, not physical 1080p/768p modes.
  UI preview fps labels were not used as performance measurements.
- Normal Syndocal New restored zero layers/outputs, persisted/effective Standby,
  both output permissions false (`native-final.json`). Close showed an unsaved
  empty-project confirmation, then Discard and Close exited; no Syndocal process
  remained (`native-exit.json`). OS exit code was unavailable, so exit 0 is not claimed.
- SynapseRack's earlier restoration capture predates later folder-import attempts
  and restart evidence. Task 10 read-only process inspection found responsive PID
  44392; original PID 18060 was absent. Final project/panel state after restart is
  unverified; the original-process-preservation claim is withdrawn. No existing
  project overwrite or enabled output was demonstrated. Review artifacts are retained.
- Report validation: table structure/status markers, source references, local evidence
  paths, PNG dimensions/viewport metrics, published action state and Git scope checked;
  `git diff --check`. Task 10 Part A adds the owner's ordering decision and commits
  only this report; `.agents/` untouched.
- No compiler/tests/build run for this docs-only review. Compiler-warning baseline,
  current and delta are **unmeasured here**; previous generator build/test results
  remain the separate Part A evidence [G].

## Source register - URLs and cited sections

- Vendor references accessed 2026-10-16. Public official pages only. Links below
  identify the page and relevant sections. D never means hands-on verification.
  Successful HTTP text snapshots and failed-URL inventory are retained in ignored
  `target/qa/competitive-review/sources/`; failed guessed slugs are not evidence.
- R0: [Download][R0] - current version/date/platform listings.
- R1: [Clips][R1] - loading/triggering; [Quickstart](https://resolume.com/support/en/quickstart-tutorial) - Trigger Clips, Mixing, Effects, Help.
- R2: [Layers][R2] - blending, Layer Transition, Previous/Next, Auto Pilot.
- R3: [Effects][R3] - Adding Effects, Video Effects, presets, bypass, More Effects.
- R4: [Parameter Animation][R4] - BPM Sync, Audio Analysis, Using FFT;
  [BPM](https://resolume.com/support/en/bpm) and [Ableton Link](https://resolume.com/support/en/link).
- R5: [MIDI Shortcuts][R5] - creating shortcuts; [Keyboard shortcuts](https://resolume.com/support/en/shortcuts) - Output, Composition;
  [OSC](https://resolume.com/support/en/osc) - external control.
- R6: [Advanced Output][R6] - screens/slices; [Output Transformation](https://resolume.com/support/en/output-transformation) - warping;
  [Edge Blending](https://resolume.com/support/en/edge-blending) - projector overlap.
- R7: [Media Manager][R7] - Clip Reconnect, Relocate All, Collect Media.
- R8: [Layouts][R8] - hiding/moving/undocking, multiple monitors;
  [Undo](https://resolume.com/support/en/undo) - exclusions, scope, stack.
- R9: [Preparing Media][R9] - Use DXV; [Composition](https://resolume.com/support/en/composition) - Frame Rate;
  [Tech Specs](https://resolume.com/support/en/tech-specs) - requirements (not capacity guarantees).
- R10: [Groups][R10] - group sub-compositions/blending;
  [Sources](https://resolume.com/support/en/sources) - routing/generation;
  [Wire ISF](https://resolume.com/support/en/isf) - separate shader workflow.
- R11: [NDI Inputs and Outputs][R11] - Input, Output, Advanced output;
  [Syphon/Spout](https://resolume.com/support/en/syphonspout) - Input/Output;
  [Recording](https://resolume.com/support/en/recording) - capture/reimport.
- R12: [SMPTE][R12] - Arena timecode input;
  [DMX Output](https://resolume.com/support/en/dmx) - fixture/pixel routing.
- V0: [VDMX6 manual][V0] - VDMX6 versus Plus/platform introduction.
- V1: [Media Management][V1] - pages/import, missing-media relinking;
  [Templates](https://docs.vidvox.net/vdmx/vdmx_templates) - Simple Mixer/4 Layer starter.
- V2: [Rendering][V2] - layers/groups/canvas, FX chain, composition/master controls.
- V3: [UI Controls][V3] - sliders/buttons/popups, Receiving, UI Items and Hardware Learn Mode;
  [Data I/O](https://docs.vidvox.net/vdmx/vdmx_data_io) - receivers/senders.
- V4: [Plugins][V4] - Two Channel Mixer, Audio Analysis, Clock, Cue List,
  Data Looper, Movie Recorder, Control Surface, Video to DMX.
- V5: [Video Display][V5] - fullscreen/Advanced output, Syphon/NDI, recording.
- V6: [Saving][V6] - File menu, workspace presets, trigger/revert.
- V7: [ISF][V7] - generators, filters, multipass discussion.
- V8: [Video Codecs][V8] - support, recommended playback formats, storage guidance.
- V9: [Inspectors][V9] - windows/components; [Preferences](https://docs.vidvox.net/vdmx/vdmx_preferences) - UI/render settings.
- V10: [Updating from VDMX5][V10] - Metal migration and older format boundaries.
- S0: [FAQ][S0] - Demo limits, platform differences, project versus machine settings.
- S1: [Loading video/images][S1] - import, assignment, missing/codec badges, MediaBrowser localization.
- S2: [Layer][S2] - top row, opacity, transport, Beat Sync;
  [Group Layer](https://synapserack.com/en/docs/window/group-layer/) - independent result controls.
- S3: [Node Editor][S3] - image ports/history;
  [Fx Editor](https://synapserack.com/en/docs/fxeditor/) - ISF conversion/export, limitations;
  [CrossFade](https://synapserack.com/en/docs/nodes/composite/CrossFade/) - ports/Mix;
  [Gradient](https://synapserack.com/en/docs/nodes/generator/gen.gradient/) - generator/resolution presets.
- S4: [Global Tempo][S4] - opening, TAP, MIDI;
  [Beat Sync](https://synapserack.com/en/docs/beat-sync/) - clock limitations;
  [BPMDetect](https://synapserack.com/ja/docs/nodes/audio/BPMDetect/) - newer detector catalog;
  [StepSequencer](https://synapserack.com/en/docs/nodes/tempo/StepSequencer/) - sequencing.
- S5: [AudioReactive][S5] - device and example parameter wiring.
- S6: [Projector][S6] - output toggle, one-output limit;
  [SynapseMapper](https://synapserack.com/en/docs/window/synapse-mapper/) - quad, corners, warp grid.
- S7: [MIDI Mapping][S7] - assign mode, target, input/device identity;
  [OSC](https://synapserack.com/en/docs/osc-input-output/) - assignment.
- S8: [Video exchange][S8] - receiving, sender loss/reconnect, always-on master send.
- S9: [FileRecorder][S9] - MP4 and retrospective dump;
  [BufferGranular](https://synapserack.com/en/docs/nodes/record/BufferGranular/) - buffer playback.
- S10: [Shortcuts][S10] - pointer-scoped undo, keyboard mapping, save not assigned;
  [WidgetPanel](https://synapserack.com/en/docs/widget-panel/) - custom performance surface.
- S11: [Apps SDK][S11] - HTML/JS extensions, layer/mixer API.
- S12: [Crossfade loop][S12] - loop overlap behavior.
- S13: [OFL Fixture][S13] - lighting node scope/limits.
- L: [prior source-linked feature inventory][L], refreshed for generator/blend changes;
  `crates/protocol/src/lib.rs`, `crates/video/src/isf_runtime.rs`, `app/src-tauri/Cargo.toml`,
  `app/src/components/VideoControlPanel.tsx`, `VideoLayerListPanel.tsx`,
  `VideoEffectScopePanel.tsx`, `VideoClipSlotBankPanel.tsx`, `AudioReactiveVjStrip.tsx`.
- G/B: [generator acceptance][G], [blend acceptance][B]. P/T: [render benchmark][P], [native Tier C][T].

[R0]: https://resolume.com/download/ "Download: current version/date"
[R1]: https://resolume.com/support/en/clips "Clips: loading and triggering"
[R2]: https://resolume.com/support/en/layers "Layers: blends, transitions and autopilot"
[R3]: https://resolume.com/support/en/effects "Effects: adding, scopes, bypass, plugins"
[R4]: https://resolume.com/support/en/parameter-animation "Parameter Animation: BPM Sync and Audio Analysis"
[R5]: https://resolume.com/support/en/midi-shortcuts "MIDI Shortcuts: creating mappings"
[R6]: https://resolume.com/support/en/advanced-output "Advanced Output: screens and slices"
[R7]: https://resolume.com/support/en/media-manager "Media Manager: reconnect, relocate, collect"
[R8]: https://resolume.com/support/en/layouts "Layouts: hide, move, undock, monitors"
[R9]: https://resolume.com/support/en/preparing-media "Preparing Media: DXV GPU decode"
[R10]: https://resolume.com/support/en/groups "Groups: sub-composition and group blending"
[R11]: https://resolume.com/support/en/NDI_inputs_and_outputs "NDI: input and output"
[R12]: https://resolume.com/support/en/smpte "SMPTE: Arena timecode input"
[R-source]: https://resolume.com/support/en/sources "Sources: generation and routing"
[R-isf]: https://resolume.com/support/en/isf "Wire ISF: shaders and inputs"
[R-link]: https://resolume.com/support/en/link "Ableton Link: synchronization"
[R-dmx]: https://resolume.com/support/en/dmx "DMX Output: Lumiverses and fixtures"
[R-record]: https://resolume.com/support/en/recording "Recording: output capture and reimport"
[V0]: https://docs.vidvox.net/vdmx/ "VDMX6 manual: versions and platform"
[V1]: https://docs.vidvox.net/vdmx/vdmx_media_management "Media: import and relinking"
[V2]: https://docs.vidvox.net/vdmx/vdmx_rendering "Rendering: layers, groups, FX, composition"
[V3]: https://docs.vidvox.net/vdmx/vdmx_ui_controls "UI controls: receiving and sending"
[V4]: https://docs.vidvox.net/vdmx/vdmx_plugins "Plugins: named subsections in source register"
[V5]: https://docs.vidvox.net/vdmx/vdmx_video_display "Display: fullscreen and Advanced output"
[V6]: https://docs.vidvox.net/vdmx/vdmx_saving "Saving: project and workspace presets"
[V7]: https://docs.vidvox.net/vdmx/vdmx_isf "ISF: generators and filters"
[V8]: https://docs.vidvox.net/vdmx/vdmx_video_codecs "Codecs: support and playback advice"
[V9]: https://docs.vidvox.net/vdmx/vdmx_inspectors "Inspectors and windows"
[V10]: https://docs.vidvox.net/vdmx/vdmx_updating_faq "VDMX6 migration: Metal"
[S0]: https://synapserack.com/en/docs/faq/ "FAQ: Demo and platform limits"
[S1]: https://synapserack.com/en/docs/load-video/ "Media: import, assignment and missing files"
[S2]: https://synapserack.com/en/docs/window/layer/ "Layer: top/middle rows, opacity and transport"
[S3]: https://synapserack.com/en/docs/window/node-editor/ "Node Editor: canvas, ports and history"
[S4]: https://synapserack.com/en/docs/window/global-tempo/ "Global Tempo: TAP and mappings"
[S5]: https://synapserack.com/en/docs/audio-reactive/ "AudioReactive: devices and parameter wiring"
[S6]: https://synapserack.com/en/docs/projector/ "Projector: output toggling and limit"
[S7]: https://synapserack.com/en/docs/midi-mapping/ "MIDI: assign mode and device identity"
[S8]: https://synapserack.com/en/docs/load-spout/ "Video exchange: source loss and sending"
[S9]: https://synapserack.com/en/docs/nodes/record/FileRecorder/ "FileRecorder: MP4 and dump"
[S10]: https://synapserack.com/en/docs/shortcuts/ "Shortcuts: histories, keys and save limitation"
[S11]: https://synapserack.com/en/docs/apps-sdk/ "Apps SDK: extensions and mixers"
[S12]: https://synapserack.com/en/docs/crossfade-loop/ "Crossfade loop"
[S13]: https://synapserack.com/en/docs/nodes/io/ofl_fixture/ "OFL Fixture: lighting controls"
[S-group]: https://synapserack.com/en/docs/window/group-layer/ "Group Layer: available result controls"
[S-fx]: https://synapserack.com/en/docs/fxeditor/ "Fx Editor: conversion, export and limitations"
[S-gradient]: https://synapserack.com/en/docs/nodes/generator/gen.gradient/ "Gradient: parameters and resolution presets"
[S-crossfade]: https://synapserack.com/en/docs/nodes/composite/CrossFade/ "CrossFade: inputs and Mix"
[S-bpm]: https://synapserack.com/ja/docs/nodes/audio/BPMDetect/ "BPMDetect: output ports"
[S-buffer]: https://synapserack.com/en/docs/nodes/record/BufferGranular/ "Buffer Granular: playback"
[L]: SYNAPSERACK_FEATURE_COMPARISON_2026-10-15.md
[G]: VIDEO_GENERATORS_2026-10-16.md
[B]: VIDEO_BLEND_MODES_2026-10-16.md
[P]: VJ_BENCHMARK_2026-10-14.md
[T]: SYNAPSERACK_TIER_C_2026-10-15.md
