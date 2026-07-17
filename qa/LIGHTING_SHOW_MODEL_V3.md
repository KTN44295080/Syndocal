# Lighting Show Model v3 — F-series Design (Daslight functional parity)

作成: 2026-07-16 / 設計: Fable Workflow（Inventory×2→Design→敵対検証14主張全CONFIRMED）
/ 実装: Codex gpt-5.6-sol（tranche-cycleスキル準拠）
状態: **F1-F3意味論確定・承認済み（2026-07-16）**。確定事項:
- レイヤー競合 = **上のレイヤーが勝つ**（top-layer-wins、決定論的優先順位）
- BPM変更時 = **即時再適合**（再生中もブロックが伸縮、ライブ運用優先）
- マルチステップStaticシーン = **採用**（F8として追加、Daslight完全パリティ）
- 映像レイヤー = **同一タイムラインに混在**（layer kindとして共存、F6入れ子はv3照明専用）
残るOpen Questions（グループ列排他/FX削除時所有権/入れ子ループ音声/音声スコープ）はF5/F6着手時に確認

根拠: Daslight 5実機操作観察（target/qa/ui-comparison/PRIMARY_OBSERVATIONS.md）+
ユーザー確認済み意味論（Static/FXシーン、レイヤー式Timeline、入れ子、BPM管理、
RATE/WINDOWストレッチ、下半分フェードドラッグ）。優先方針: 照明完成がVJより先。

## F1: Timeline Layers v3 — N user layers with functional mute/solo/lock (protocol + engine)
risk: medium / est: L / depends: none
**✅完了 2026-07-16 コミット63a9ce1** — kind条項込みで設計どおり実装。16新テスト+2000ブロック
予算テストの8レイヤー化、protocol 22 / engine timeline_layer 11 / timeline 43 / syndocal
project_ 60 / tsc 全green（監督独立再実行済み）。TimelineEventPlacementUpdate.layer_idも
追加済みでF2の原子的レイヤー移動に使える。

**Problem**: Syndocal has exactly two hard-coded visual-only lanes (TimelineTrackKind::Lighting|Video, protocol lib.rs:1385-1388). The eye toggle in TimelineOverview only dims pixels; the engine ignores `track` entirely when firing ('placement always recalls the complete source Cue', engine lib.rs:11167-11168). Daslight's model — and the user's Shin show — needs 1..N user-created ordered layers whose mute actually gates cue dispatch, whose lock rejects edits, and whose ordering gives a deterministic result when blocks overlap.

**Design**: Additive layer model, ms-domain unchanged. Protocol: new `TimelineLayerSummary { id: u32, label: String, order: u32, muted: bool, locked: bool, solo: bool }`; `TimelineSnapshot.layers: Vec<TimelineLayerSummary>` with #[serde(default)] (empty = legacy mode, snapshot() synthesizes implicit 'Lighting'(0)/'Video'(1) layers for display without persisting them); `TimelineCueEventSummary.layer_id: Option<u32>` with #[serde(default)] (None = derive from the untouched legacy `track` field). Keep writing `track` on save (nearest kind per layer) so the field stays populated for any old tooling; v1 readers ignore the new fields per the .sdc contract. Engine: RuntimeTimelineEvent gains a pre-resolved {layer_order, layer_muted_effective} pair recomputed once per layer/placement command on the command-drain path (ack'd), never looked up per tick — the 44 Hz occurrence collector just tests one bool. Solo semantics: any solo active => non-solo layers treated muted. Dispatch ordering becomes (time, layer_order desc, event_id) so simultaneous triggers resolve top-layer-wins deterministically (interim policy; full HTP/LTP merge is an open question, see openQuestions). New EngineCommands AddTimelineLayer / UpdateTimelineLayer / RemoveTimelineLayer / ReorderTimelineLayers use the established mpsc::SyncSender<Result<(),String>> ack + rollback pattern (engine lib.rs:334ff). Locked layers: placement/snap/remove commands targeting a locked layer's events return an ack error and mutate nothing. RemoveTimelineLayer requires the layer be empty or takes an explicit reassign-target. Validation in main.rs: layer_id must reference an existing layer; orders normalized; legacy files (no layers, events with track only) load byte-compatibly. Tests: muted-layer blocks do not fire during Play; solo isolates; locked-layer edit rejected via ack; the existing 2000-block scheduling budget test extended with 8 layers stays under budget; legacy .sdc load test (embedded mini-show) green.

**Protocol changes (.sdc-compat)**:
- TimelineSnapshot.layers: Vec<TimelineLayerSummary> — new, #[serde(default)] empty; v1 readers ignore unknown fields, old files load with empty vec (implicit legacy lanes).
- TimelineCueEventSummary.layer_id: Option<u32> — new, #[serde(default)] None; None resolves from the existing `track` field, which remains serialized unchanged.
- New struct TimelineLayerSummary { id, label, order, muted, locked, solo } — muted/locked/solo #[serde(default)] false so hand-edited minimal entries load.
- New EngineCommand variants AddTimelineLayer/UpdateTimelineLayer/RemoveTimelineLayer/ReorderTimelineLayers with ack: mpsc::SyncSender<Result<(),String>> — runtime-only, not serialized into .sdc.

**Files**: C:/Users/kouty/Documents/KDMX/crates/protocol/src/lib.rs, C:/Users/kouty/Documents/KDMX/crates/engine/src/lib.rs, C:/Users/kouty/Documents/KDMX/app/src-tauri/src/main.rs, C:/Users/kouty/Documents/KDMX/app/src/types.ts

**Acceptance**:
- cargo test -p engine: muted layer suppresses cue dispatch during timeline Play; solo on layer A suppresses layer B; unmute mid-play resumes future occurrences only.
- cargo test -p engine: placement update / snap / remove against a locked layer returns Err via ack and leaves timeline_events unchanged.
- cargo test -p engine: extended 2000-block budget test across 8 layers completes within the existing time bound; no per-tick allocation added to collect_timeline_cue_occurrences_between.
- cargo test -p syndocal project_: legacy .sdc (no layers field) loads; events resolve to implicit Lighting/Video layers; save/load round-trip preserves layer_id and still writes `track`.
- cargo test -p syndocal: validation rejects layer_id referencing a missing layer with an actionable message.

## F2: Layered timeline desk UI — lane gutter, drag-drop scene placement, block edge-resize
risk: medium / est: L / depends: F1, T10
**✅完了 2026-07-17 コミットf4a62b2** — 型付きセクション（Audio→Lighting→Video、見出し+kindティント）、
ガター（実効mute/lock/solo）、cueグリップ→Lightingレーンのドラッグ配置（F3 authored_beatsから既定尺、
kind別ドロップ検証）、ブロック縦移動+端リサイズ（選択ブロックのみハンドル描画=ノード予算3057/3500）。
Codex 3パス（本体+2px range marginあふれ修正+overlapClusterアイドルチャーン修正+ノード予算修正）。
フルマトリクス212全緑、DnDはCDP実機操作で確認（配置成功+Video拒否メッセージ）。
mainチャンクはlocalization分割で508→407kB。ハーネスにlayered-deskチェック13断言（マトリクス合否直結）。

**Problem**: Placement today is form-based (TimelineSceneBlocksEditor composer + 'At Playhead'); existing blocks drag only horizontally; duration/loop edits require the numeric row editor; no drag from a scene pool onto a lane, no vertical lane-change drag, no block edge-resize (resize handles exist only for automation ranges). Daslight's core workflow is drag-scene-onto-layer.

**Design**: Lands inside the T10 unified shell (Control context pane hosts the Timeline). TimelineOverview renders N lanes from TimelineSnapshot.layers (F1), each with a gutter: name, eye = functional mute (issues UpdateTimelineLayer, no more CSS-only dimming), lock, solo, plus add/remove/reorder lane controls in the panel header. Drag sources: cue rows in the existing cue pool (CueManagementPanel list; later also F5 matrix cards) become draggable; dropping on a lane creates a linked Scene Block via the existing add-event path extended with layer_id, start snapped through the existing snapTimeMs Beat/Bar/Grid logic, duration defaulted from the cue's authored length when present (F3) else the current composer default. Existing blocks: vertical drag moves layer_id (rejected with status message if target lane locked); left/right edge handles resize duration_ms with snap, reusing the automation-range resize interaction pattern already in TimelineOverview. Block anatomy (identity band, name, duration stamp, x-loops title) is preserved. Lanes area scrolls internally inside the timeline pane when N is large — app shell stays scroll-free; check-viewport-containment.mjs gains assertions for lane gutter presence and zero document/app overflow with 6 lanes.

**Protocol changes (.sdc-compat)**:
- None new — consumes F1's layer_id on the existing TimelineEventPlacementUpdate/add-event surfaces (layer_id added there in F1 as Option<u32>, serde-default None so old payloads remain valid).

**Files**: C:/Users/kouty/Documents/KDMX/app/src/components/TimelineOverview.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineSceneBlocksEditor.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineCueEventsPanel.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/CueManagementPanel.tsx, C:/Users/kouty/Documents/KDMX/app/src/App.tsx, C:/Users/kouty/Documents/KDMX/app/src/styles.css, C:/Users/kouty/Documents/KDMX/app/scripts/check-viewport-containment.mjs

**Acceptance**:
- Dragging a cue from the pool onto lane 3 creates a linked block at the snapped drop time on that lane; block fires that cue under Play (manual verify per repo verify culture).
- Vertical drag moves a block between lanes; drop on a locked lane is rejected with a status-line message and no mutation.
- Edge-resize changes duration_ms with Beat/Bar/Grid snap; loop span readout (duration x loops) updates live.
- Eye toggle on a lane audibly/visibly stops that lane's cues from firing (engine-verified mute, not CSS dimming).
- npm run check:viewport --silent green at 1280x720/1366x768/2048x1129 with 6 lanes; npm --prefix app run build green.

## F3: Conform-to-tempo — authored scene length, beat-domain placement, re-conform pass, [N.NNx] badge
risk: medium / est: M / depends: F1
**✅完了 2026-07-16 コミット09fc18c** — 即時re-conform（再生中含む、ロック層にも適用=トランスポート
操作扱い）、loop_fill上限256、rateはロード時再計算、Chaserのbounce走査(2n-2)シード対応。
engine conform 9 / timeline 44 / project_ 64 / protocol 23 / フルviewportマトリクス212全緑
（監督独立再実行済み）。付随してcueEditRowのflex-wrap化とT10冗長scrollbar-gutter除去、
ハーネス基盤強化（ポートfail-fast/ツリーkill/フェーズ間ブラウザリサイクル）を同梱。

**Problem**: Scene Blocks are fixed-milliseconds: Beat/Bar snap is frontend-only ms rounding discarded at commit (App.tsx snapTimeMs), so BPM changes leave stale placements; cues have no authored period to conform against; no conform flag, no rate multiplier, no badge. The exact rate math already exists for video (effective_speed = loop_length_ms / (240000/bpm * loop_bars / ratio), video lib.rs:4828) but nothing on the block->cue path uses it.

**Design**: ms stays the playback truth (external MTC/LTC/SPP drive the timeline in ms, and BpmClock's anchor rebases on tap/sync make the live beat counter an unstable axis — engine lib.rs:16769-16790); beats fields are stored authoring intent. Protocol: CueSummary.authored_beats: Option<f32> #[serde(default)] — the scene's intrinsic musical length (capture UI seeds it from a referenced effect's clock_sync.beats or chaser step count when unambiguous, else user-entered in the cue editor). TimelineCueEventSummary gains time_beats: Option<f64>, duration_beats: Option<f64>, conform_to_tempo: bool, loop_fill: bool — all #[serde(default)]. Semantics: when conform_to_tempo, the block's iteration period = authored_beats * 60000 / bpm; with loop_fill the block keeps its duration and the engine derives the iteration count to fill it (capped by MAX_TIMELINE_SCENE_BLOCK_LOOPS=256), matching Daslight's 'Loop ON inside a 02m00s20 block'; without loop_fill, duration_ms itself is recomputed from duration_beats. Re-conform: a single engine-side pass (new command ReconformTimelineToBpm, also invoked from the SetBpm/Tap handlers when any conformed block exists) rewrites time_ms/duration_ms from the beats fields at the new BPM during command drain — never in the 44 Hz tick. Rate: rate = free_run_period_ms_of_scene / conformed_iteration_ms computed at edit/trigger time, stored pre-resolved on RuntimeTimelineEvent and surfaced additively on the snapshot event (derived, recomputed on load); in F3 the rate only drives the retrigger cadence and the badge — content-speed application to FX lands in F4. UI: block title/tooltip and block-properties row gain the [N.NNx] badge and a Conform/Loop-fill toggle pair (attaches to the F2 overview; if F2 is not yet landed the badge attaches to the current TimelineOverview block title, which already renders duration stamps and 'duration x loops' tooltips).

**Protocol changes (.sdc-compat)**:
- CueSummary.authored_beats: Option<f32> — new, #[serde(default)] None; legacy cues load unchanged; validation clamps to 0.25..=1024 beats when present.
- TimelineCueEventSummary.time_beats: Option<f64> — new, #[serde(default)] None (authoring intent; ms field remains authoritative for playback).
- TimelineCueEventSummary.duration_beats: Option<f64> — new, #[serde(default)] None.
- TimelineCueEventSummary.conform_to_tempo: bool — new, #[serde(default)] false.
- TimelineCueEventSummary.loop_fill: bool — new, #[serde(default)] false; when true loop_count is derived, capped at MAX_TIMELINE_SCENE_BLOCK_LOOPS.
- TimelineCueEventSummary.rate: Option<f32> (derived display value) — new, #[serde(default)] None, recomputed on load so stale saved values are harmless.
- New EngineCommand ReconformTimelineToBpm with ack — runtime-only.

**Files**: C:/Users/kouty/Documents/KDMX/crates/protocol/src/lib.rs, C:/Users/kouty/Documents/KDMX/crates/engine/src/lib.rs, C:/Users/kouty/Documents/KDMX/app/src-tauri/src/main.rs, C:/Users/kouty/Documents/KDMX/app/src/types.ts, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineOverview.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineSceneBlocksEditor.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/CueManagementPanel.tsx

**Acceptance**:
- cargo test -p engine: a conformed loop_fill block at BPM 120 with authored_beats 4 retriggers every 2000ms; after SetBpm(60) + reconform it retriggers every 4000ms and block boundaries in ms move accordingly.
- cargo test -p engine: reconform runs on command drain only; tick-path functions show no new allocation/lookup (budget test unchanged).
- cargo test -p syndocal project_: legacy .sdc loads with conform fields defaulted false/None; round-trip preserves beats fields.
- UI: conformed block shows [N.NNx] badge matching authored period / conformed period to 2 decimals; toggling BPM updates the badge after reconform.
- Snap commit now writes time_beats alongside time_ms when snap mode is Beat/Bar, so later BPM changes re-derive positions.

## F4: Cue-owned FX parameters + activation-scoped effect instances with rate
risk: high / est: L / depends: F3
**✅完了 2026-07-17 コミットb8de22d** — EffectParamsSnapshot（6種request形状）、copy-on-capture、
activation-scopedインスタンス（rateは全6評価器へ乗算1回、ループ毎位相リセット、ブロック終端/
手動releaseで消滅、jump/seek/逆行/rollbackの整理まで網羅）、release_cueコマンド追加。
44Hzティックへの追加ゼロ（ドレイン時プリビルド）。18新テスト+回帰全緑+フルマトリクス212緑
（監督独立再実行済み）。**削除ポリシー（要ユーザー確認）**: params保持シーンはグローバル
スタックのエフェクト削除後も自分のlookを維持する既定で実装済み。

**Problem**: FX scenes are only enable-flag flips on a shared global effect stack: CueEffectTarget is { effect_id, enabled } (protocol lib.rs:342-345) and apply_cue_effect_targets mutates the shared instance's enabled bit (engine lib.rs:10734-10744). Two FX scenes cannot carry different parameters of the same effect; recalling scene B silently changes what scene A meant; two blocks placing the same FX scene at different conform rates would fight over one instance and one created_at; nothing stops a cue's effects at block end.

**Design**: Protocol: CueEffectTarget gains #[serde(default)] params: Option<EffectParamsSnapshot>, where EffectParamsSnapshot is an enum over the six existing request payloads (Lfo/PositionWave/Color/Chaser/Move/Value — reusing the request structs the .effect preset path already serializes). Capture flow: the existing 'effects' capture scope copies current parameters of referenced enabled effects into the cue (copy-on-capture); legacy targets without params keep exact current enable-flip behavior. Engine: when a Scene Block fires (inside advance_pending_cue, the established budget-tested hook), targets WITH params instantiate activation-scoped RuntimeEffect copies carrying their own created_at, the block's rate (F3, default 1.0), and an activation key (event_id, iteration); instances are pre-built during command/pending drain into a pre-sized activation Vec — zero hot-path allocation — and evaluated by the same per-tick effect walk as global effects. Lifecycle: activation instances end at the block's end boundary (the occurrence scheduler already knows time_ms + duration*loops; emit an end marker alongside the trigger occurrence) or on manual cue release; loop iterations reuse the instance with created_at rebased per iteration so phase restarts like Daslight scene loops. Rate threading: each evaluator's normalized-cycle reduction gains one multiply — synced: cycle = beat_position/beats * rate; free-run: cycle = elapsed * rate / period_ms — via the instance field, never a per-tick map lookup. Global stack recall (pads/executors, no block window) with params creates one activation instance keyed to the cue-list active cue and ends when that cue list moves on/releases. Deletion: sanitize_cue_effect_targets keeps pruning stale effect_ids only for param-less legacy targets; param-carrying targets survive stack deletion (the scene owns its look) — flagged in openQuestions for confirmation.

**Protocol changes (.sdc-compat)**:
- CueEffectTarget.params: Option<EffectParamsSnapshot> — new, #[serde(default)] None; None = legacy enable-flip behavior, so every existing .sdc and every old cue keeps byte-identical semantics.
- New enum EffectParamsSnapshot { Lfo(LfoEffectRequest-shaped), PositionWave(...), Color(...), Chaser(...), Move(...), Value(...) } — reuses existing request payload shapes already proven in .effect preset serialization; unknown-to-old-readers per v1 ignore policy.
- No new persisted runtime state: activation instances are engine-runtime only and never serialized.

**Files**: C:/Users/kouty/Documents/KDMX/crates/protocol/src/lib.rs, C:/Users/kouty/Documents/KDMX/crates/engine/src/lib.rs, C:/Users/kouty/Documents/KDMX/app/src-tauri/src/main.rs, C:/Users/kouty/Documents/KDMX/app/src/types.ts, C:/Users/kouty/Documents/KDMX/app/src/components/CueManagementPanel.tsx

**Acceptance**:
- cargo test -p engine: two blocks placing the same FX scene at rates 1.0 and 2.78 run concurrently with independent phase and independent periods; DMX preview proves both waveforms present.
- cargo test -p engine: editing the global stack effect after capture does NOT change a param-carrying scene's recall output; legacy param-less target still follows the stack instance.
- cargo test -p engine: activation instances stop contributing at block end (DMX returns to base) and restart phase at each loop iteration.
- cargo test -p engine: existing effect/telemetry budget tests unchanged; no allocation in evaluate_* paths (instance Vec pre-sized at drain).
- cargo test -p syndocal project_ + sample_effect: legacy .sdc and .effect presets load; capture-with-params round-trips.

## F5: Per-group scene matrix — group-affiliated cues, hue-coded columns, optional column exclusivity
risk: medium / est: L / depends: F2, T10

**Problem**: Daslight organizes scenes in a group-column matrix with hue-coded headers and per-column exclusivity; Syndocal's Control cue surface is a flat paged 10-pad bank (App.tsx cuePadSize=10) and CueSummary has no group affiliation at all, so a 'Par' column with its scenes cannot even be derived from the data model. The hue identity device (cueIdentityHue/groupIdentityHue in identityColor.ts) already exists and is explicitly Daslight-modeled.

**Design**: Protocol: CueSummary.group_id: Option<String> #[serde(default)] — a fixture group path matching the existing group_ids vocabulary; capture UI defaults it from the selectedGroup capture scope, editable in cue metadata. UI (inside the T10 shell's Control surface): a Scene Matrix pane — one column per group in group order, header tinted with groupIdentityHue, scene cards per column tinted with cueIdentityHue, active-cue highlight from engine snapshot; ungrouped cues collect in a trailing 'Show' column; cards double as F2 drag sources for timeline placement. Exclusivity: engine grows a per-group active-cue map; when a cue with group_id G triggers with the exclusivity policy on, the previously active cue of G is released (its targets fade back to base/tracked values using the released cue's fade_ms) before/while the new cue applies — implemented on the existing cue-apply path, resolved at trigger time, no per-tick work. Whether exclusivity is matrix-only, global (pads/MIDI/OSC/timeline too), or off-by-default is an openQuestion; the tranche ships the mechanism behind a project-level policy flag defaulting to the user's answer. The flat pad bank remains for hotkey/MIDI muscle memory.

**Amendment (2026-07-17 ユーザー回答反映)**: 列排他はプロジェクト全体フラグではなく
**cue毎の再生モード**（Daslight Bank相当）。CueSummary.recall_mode: #[serde(default)] Coexist |
ReplaceGroup。ReplaceGroupのcueが発火すると、同じgroup_idで現在アクティブなcueをそのcueの
fade_msでリリースしてから適用（per-group active-cue mapはトリガー時解決、per-tickゼロ）。
Coexist（既定）は現状どおり重ね掛け。発火経路（マトリクス/パッド/MIDI/OSC/タイムライン）に
よらずcue自身の設定に従う。cueエディタにモードセレクタ、マトリクスカードにReplaceバッジ表示。

**Protocol changes (.sdc-compat)**:
- CueSummary.group_id: Option<String> — new, #[serde(default)] None; legacy cues load ungrouped; validation warns (not errors) when group_id names a group no fixture carries.
- CueSummary.recall_mode: RecallMode (Coexist | ReplaceGroup) — new, #[serde(default)] Coexist; legacy cues keep coexist semantics byte-identically.
- Project-level setting group_column_exclusivity: bool (location: project settings summary) — new, #[serde(default)] false until the user decides the default.
- Cue trigger command surface gains an optional exclusivity override flag (runtime-only, not serialized).

**Files**: C:/Users/kouty/Documents/KDMX/crates/protocol/src/lib.rs, C:/Users/kouty/Documents/KDMX/crates/engine/src/lib.rs, C:/Users/kouty/Documents/KDMX/app/src-tauri/src/main.rs, C:/Users/kouty/Documents/KDMX/app/src/types.ts, C:/Users/kouty/Documents/KDMX/app/src/identityColor.ts, C:/Users/kouty/Documents/KDMX/app/src/App.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/CueManagementPanel.tsx, C:/Users/kouty/Documents/KDMX/app/src/styles.css

**Acceptance**:
- Matrix renders one hue-coded column per group with that group's cues; active cue highlighted from snapshot; ungrouped cues in trailing column.
- cargo test -p engine: with exclusivity on, triggering cue B (group G) releases cue A (group G) — A's exclusive targets fade to base over A's fade_ms — while cues of other groups stay active.
- cargo test -p syndocal project_: legacy .sdc loads with group_id None; round-trip preserves group_id.
- Matrix card drag creates a timeline block on a lane (integration with F2).
- npm run check:viewport --silent green — matrix scrolls internally, app shell overflow 0.

## F6: Super Scenes — nested child timeline inside a cue with per-block child transport
risk: high / est: XL / depends: F1, F3, F4

**Problem**: The engine owns exactly one flat TimelineSnapshot; a cue cannot contain a timeline and there is no per-block child transport, so the user's Super Scene 'Shin' (~10 layers: audio, full-length intensity lane, scene-block layers) is unrepresentable. jump_to_event_id and follow_ms chains express only linear sequences, not a parallel multi-layer bundle.

**Design**: Protocol: CueSummary.child_timeline: Option<ChildTimelineSummary> #[serde(default)], where ChildTimelineSummary reuses the existing content structs { layers, events, automations, video_automations, audio: Option<AudioAnalysisSummary>, duration_ms } minus transport fields (playing/position live only in the parent). Depth-1 for v3: validation rejects a child event whose cue itself has a child_timeline, and rejects self/cyclic references. Engine: when a block whose cue carries a child timeline fires, advance_pending_cue spawns a pre-allocated ChildTransport { activation key, window start/end from the parent block, child position, rate from F3 conform }; advance_timeline advances active child transports immediately after the parent playhead each tick (child_pos = (parent_pos - block_start) * rate, modulo child duration under loop_fill), collects child occurrences with the same layer-gated occurrence collector (reused, parameterized over an event slice), and dispatches through the normal cue path — child FX scenes get F4 activation instances, so nesting composes. Seek/scrub: a parent seek recomputes each in-window child position and re-fires state-establishing occurrences the same way the flat timeline already handles seek-past events; MTC/LTC stay authoritative on the parent only. Block end kills the child transport and its activation instances. Editing: the child timeline is edited by opening the super scene in the same F2 timeline surface (breadcrumb: Show > Shin), reusing every lane/drag/resize interaction; placed super-scene blocks stay source-linked, so child edits update all placements. Bounded memory: child transports capped by simultaneously active super blocks; a focused budget test covers 16 concurrent super blocks each with 200 child events.

**Protocol changes (.sdc-compat)**:
- CueSummary.child_timeline: Option<ChildTimelineSummary> — new, #[serde(default)] None; legacy cues unchanged; v1 readers ignore the field.
- New struct ChildTimelineSummary { layers, events, automations, video_automations, audio, duration_ms } — composed entirely of existing serialized types (F1 layers included), all collections #[serde(default)].
- Validation additions: recursion depth > 1 and cycles are validation errors, not load failures — file still loads for inspection with the offending recall inert.
- New EngineCommand SetCueChildTimeline with ack — runtime-only.

**Files**: C:/Users/kouty/Documents/KDMX/crates/protocol/src/lib.rs, C:/Users/kouty/Documents/KDMX/crates/engine/src/lib.rs, C:/Users/kouty/Documents/KDMX/app/src-tauri/src/main.rs, C:/Users/kouty/Documents/KDMX/app/src/types.ts, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineOverview.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineSceneBlocksEditor.tsx, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineCueEventsPanel.tsx, C:/Users/kouty/Documents/KDMX/app/src/App.tsx

**Acceptance**:
- cargo test -p engine: a super-scene block at t=10s whose child has blocks at 0s/2s fires the child cues at absolute 10s/12s; with conform rate 2.0 they fire at 10s/11s.
- cargo test -p engine: parent seek into the middle of a super block establishes correct child state (child automations recomputed at mapped position); seek past the block end leaves no child residue.
- cargo test -p engine: recursion (super inside super) rejected by validation; cyclic reference rejected; flat legacy timeline behavior byte-identical when no child timelines exist.
- cargo test -p engine: 16 concurrent super blocks x 200 child events budget test passes; no per-tick allocation (transports pre-allocated at trigger).
- cargo test -p syndocal project_: .sdc round-trip of a cue with child timeline; legacy files load with None.
- UI: opening a super scene shows its child timeline in the same lane surface with breadcrumb; edits propagate to placed instances.

## F7: Audible timeline audio — play the analyzed track during timeline Play, with offset and mute
risk: low / est: M / depends: none

**Problem**: The analyzed timeline audio track (waveform/beats/BPM) is never audibly played during timeline Play — the only rodio playback path is MediaAudioPlayback whose sinks are keyed by VideoLayerId (main.rs:662, video-layer media audio) — so a music-driven light show must route music as a video layer or play it externally. A lighting desk that shows a waveform it cannot play is incomplete (Daslight plays the track).

**Design**: No engine hot-path change: the engine snapshot already carries timeline playing/position_ms every poll. Protocol: TimelineSnapshot.audio_offset_ms: i64 and audio_muted: bool, both #[serde(default)], edited via existing timeline-audio commands (set alongside analyze/clear). Tauri side: main.rs grows a dedicated timeline master sink inside MediaAudioPlayback (new key domain, not a VideoLayerId) that follows the engine snapshot — Play starts/decodes audio.path seeked to position_ms - offset, Pause stops, seek re-cues, and periodic drift resync reuses the tolerance/re-seek approach the video-layer audio path already implements. Mute toggles the sink without touching analysis. UI: mute + offset controls join the existing audio row in TimelineCueEventsPanel next to Apply BPM; the status line reports decode failures. Super-scene child audio (Shin's embedded audio layer) is explicitly F6 follow-up scope, not this tranche: one master track first, per the user's lighting-first directive. Sequenced independently of F1-F6; can land any time.

**Protocol changes (.sdc-compat)**:
- TimelineSnapshot.audio_offset_ms: i64 — new, #[serde(default)] 0; legacy files load with zero offset.
- TimelineSnapshot.audio_muted: bool — new, #[serde(default)] false.
- New/extended Tauri commands set_timeline_audio_offset / set_timeline_audio_muted — runtime surface; engine command uses the standard ack pattern.

**Files**: C:/Users/kouty/Documents/KDMX/crates/protocol/src/lib.rs, C:/Users/kouty/Documents/KDMX/crates/engine/src/lib.rs, C:/Users/kouty/Documents/KDMX/app/src-tauri/src/main.rs, C:/Users/kouty/Documents/KDMX/app/src/types.ts, C:/Users/kouty/Documents/KDMX/app/src/components/TimelineCueEventsPanel.tsx

**Acceptance**:
- Manual verify (repo verify culture): timeline Play audibly plays the analyzed file in sync with the playhead; Pause silences; seek re-cues within drift tolerance; mute toggle works live.
- cargo test -p syndocal: offset/mute round-trip through .sdc; legacy files load with 0/false; missing audio file at Play degrades to silent playback with a status-line warning, never a crash.
- cargo test -p engine: no change to tick-path telemetry (audio is entirely on the Tauri side); existing timeline tests green.
- Drift resync unit coverage on the new sink follows the existing video-layer audio resync test pattern.

## User decisions (2026-07-17)

- **F4削除ポリシー確定（2026-07-17再確認、F4b不要）**: ライブエフェクトスタック（Effectsパネル）は
  シーンを組む「作業台」であり、params所有済みシーンは作業台のエフェクトを削除しても**無傷で残る**
  （例: DimmerStrobeシーンをキャプチャ後、元のsine LFOを作業台から消してもシーンは再生可能）。
  現F4実装（b8de22d）の既定どおり。先の「警告+連動削除」回答はスタック=プリセット置き場という
  誤解に基づくもので撤回。プリセット（.effect/サンプル）は削除操作の対象外の別概念。
- **F5列排他はシーン毎設定（Daslight Bank相当）**: プロジェクト全体フラグではなく、cue単位の
  再生モード設定 — 「共存」or「前のシーンをクリア」。CueSummaryへ #[serde(default)] の
  recall_mode（Coexist既定 / ReplaceGroup）を追加する設計へ変更。F5のper-group active-cue map
  はこの per-cue モードを参照する。

## Open Questions（ユーザー回答待ち）
- Layer conflict policy: when overlapping blocks on different layers drive the same fixture attributes, which wins — top layer by order (Daslight-like priority), latest trigger regardless of layer (LTP), or HTP for intensity only? F1 ships deterministic top-layer-wins on simultaneous triggers and last-trigger-wins otherwise as the interim; confirm or redirect before F2 UI bakes expectations.
- Group-column exclusivity scope: should activating a scene in a group column always auto-release the previous scene of that group (Daslight behavior) everywhere (pads, MIDI/OSC, timeline blocks too), only from the matrix surface, or stay off by default? And should the release snap or fade using the released cue's fade_ms?
- Static scene steps: does v3 need Daslight-style multi-step static scenes as first-class cue content, or is the existing Chaser effect + follow_ms-chained cues acceptable? (Affects whether authored_beats can be derived from step counts in F3.)
- Live re-conform behavior: when BPM changes during playback, should conformed blocks re-stretch immediately (blocks move/resize under the running playhead), or only re-conform when transport is stopped / on an explicit Reconform action?
- Video's place in the layered timeline: do video layers stay as layer kinds on the same single timeline alongside lighting layers, and should F6 child timelines be lighting-only for v3 given your lighting-completeness-first directive?
- FX ownership after F4: if a global stack effect referenced by an FX scene is later deleted, should the scene keep working from its captured parameter snapshot (scene owns its look — proposed default), or go inert as today?
- Super scene loop semantics: when a super-scene block loops (loop_fill or loop_count), should the child audio layer restart each iteration with the child timeline, and is depth-1 nesting (a super scene cannot contain another super scene) acceptable for v3?
- Timeline audio scope: is one master audio track with offset/mute (F7) enough for this series — with per-super-scene audio arriving via F6 — or do you need multiple audio clips placeable as blocks on an audio lane now?

## Verified claims
14 claims adversarially verified: CONFIRMED
## Supervisor amendments (Fable, 2026-07-16)

1. **Manual RATE stretch** (user-confirmed Daslight behavior, arrived after design launch):
   the rate multiplier must have TWO sources, both feeding F4's rate threading:
   (a) manual rate-mode stretch: rate = authored_length / block_duration (BPM-independent;
   shortening plays faster, lengthening slower); (b) conform-to-tempo: iteration period
   locked to authored_beats at current BPM. F2/T4's edge-drag is MODAL via a timeline
   toolbar toggle: RATE mode edits rate (keeps content whole), WINDOW mode edits
   duration_ms with loop_fill/truncation (keeps rate, crops or fills). The [N.NNx] badge
   shows the effective rate from either source.
2. **Lower-half fade drag** (user-confirmed): block vertical zones — upper band = move,
   edges = stretch (per mode), lower-band edges = Fade In/Out drag handles. Lands with F2.
3. Sequencing vs UI track: F1 (engine boundary) may start immediately after T10 lands
   (no tree conflict rule: one tranche at a time). F2 requires T10+F1. T8/T3/T4 from the
   UI plan interleave naturally: T3's Block Properties inspector should include the
   Conform/Loop-fill toggles and rate badge from F3.

## F8: マルチステップStaticシーン（ユーザー承認 2026-07-16、F3/F4後）
Daslightパリティ: Cueが内部ステップ列（各ステップ = 属性値スナップショット + fade + hold）を持てる。
- Protocol: CueSummary.steps: Vec<CueStepSummary> #[serde(default)] 空（CueStepSummary =
  { values: Vec<CueFixtureTarget相当>, fade_ms, hold_ms }）。空 = 現行の単一状態Cue（完全互換）。
- Engine: ブロックローカル時間×rate（F3/F4）でステップを進行。activation-scoped（F4と同じ
  ライフサイクル）。authored_beatsをステップ合計から導出可能に（F3のcapture UIを拡張）。
- UI: Cueエディタにステップ列（追加/複製/並べ替え/fade/hold）、T3インスペクタにステップ数表示。
- 受入: ステップ進行のengine test、rate=2で2倍速進行、legacy Cue（steps空）不変、.sdc round-trip。

## Approved semantics (2026-07-16)
- Layer conflict: top-layer-wins (deterministic; HTP-for-intensity may be revisited later).
- Conform re-stretch: immediate on BPM change, including during Play (blocks may move under
  the running playhead; engine command-drain path, never the 44Hz tick).
- Multi-step static scenes: approved as F8.
- Video: stays on the single shared timeline as a layer kind; F6 nesting is lighting-only in v3.

## Amendment: Premiere-style typed sections + placeable audio (user, 2026-07-16)

ユーザー指示: 「同一タイムラインにあるが、分かれていることが明確化されている」（Premiere Pro型）。
音源もDaslight同様にタイムラインへ置けること。メディア3種 = 音 / 映像 / 照明。

1. **F1修正 — レイヤーkindとセクション**: `TimelineLayerSummary.kind: Lighting | Video | Audio`
   を追加（serde default = Lighting; legacy track からの導出は既存設計どおり）。レイヤーは
   kind別セクションにグループ化され、セクション順は固定: **Audio（最上段、波形=整列基準、
   Daslight同配置）→ Lighting → Video**。順序変更は将来のオプション。
2. **F2修正 — セクションの視覚分離**: セクション見出し行（kind名+レイヤー数+折りたたみ）、
   セクション間の太い区切り線、kind別の背景ティント（照明=graphite基調、映像=青系微差、
   音声=波形色）。ドラッグでのレイヤー移動はセクション内のみ（kind跨ぎ不可）。
   Cueドロップは照明セクションのみ受け付け、映像レイヤーは映像ソース、音声レイヤーは
   音声ファイルを受け付ける（kind別ドロップ検証）。
3. **F7昇格 — 音声クリップをブロックとして配置**: 「マスター1トラック」ではなく、
   音声レイヤー上に複数の音声クリップブロック（file参照、start/offset/duration、
   ブロック内波形描画=既存audio analysis+T2ピクセルキャンバス、per-block gain、
   下半分ドラッグでfade in/out=F2ゾーン設計と同一）。再生はタイムライン位置に追従。
   これは旧Open Question「音声スコープ」への回答確定を意味する（複数クリップ配置=採用）。
   Protocol: TimelineAudioClipSummary { id, layer_id, path, start_ms, offset_ms, duration_ms,
   gain, fade_in_ms, fade_out_ms } を Vec で TimelineSnapshot へ #[serde(default)] 追加。
   既存の単一 audio analysis はクリップ単位の解析へ拡張（互換: 旧projectの解析は
   クリップ0扱いで読み込み）。
