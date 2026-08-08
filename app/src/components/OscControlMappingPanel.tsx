import { For, Show } from "solid-js";
import {
  fixtureFlagClearKinds,
  isGroupFlagMappingAction,
  isVideoLayerMappingAction,
  isVideoOutputMappingAction,
  videoOutputMappingFieldOptions,
  type FixtureFlagClearKind,
} from "../controlMappingActions";
import type { EngineSnapshot, OscControlAction, OscControlMapping, VideoParam } from "../types";

interface OscControlMappingPanelProps {
  snapshot: EngineSnapshot;
  bindIp: string;
  port: number;
  running: boolean;
  mappings: OscControlMapping[];
  mapAddress: string;
  mapAction: OscControlAction;
  mapAttribute: string;
  clearFixtureFlagKind: FixtureFlagClearKind;
  selectedCueId: number | null;
  selectedEffectId: number | null;
  selectedNodeGraphId: number | null;
  mapGroupId: string;
  selectedLayerId: number | null;
  selectedOutputId: number | null;
  selectedMappingField: string;
  selectedMappingPresetLabel: string;
  mapVideoParam: VideoParam;
  mapCuePointIndex: number;
  mapDurationMs: number;
  mapLow: number;
  mapHigh: number;
  mappingTargetLabel: (mapping: OscControlMapping) => string;
  onBindIp: (value: string) => void;
  onPort: (value: number) => void;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onMapAddress: (value: string) => void;
  onMapAction: (action: OscControlAction) => void;
  onMapAttribute: (value: string) => void;
  onMapCueId: (id: number) => void;
  onMapEffectId: (id: number) => void;
  onMapNodeGraphId: (id: number) => void;
  onMapGroupId: (value: string) => void;
  onMapLayerId: (id: number) => void;
  onMapOutputId: (id: number) => void;
  onMappingField: (field: string) => void;
  onMappingPresetLabel: (label: string) => void;
  onMapVideoParam: (param: VideoParam) => void;
  onMapCuePointIndex: (value: number) => void;
  onMapDurationMs: (value: number) => void;
  onMapLow: (value: number) => void;
  onMapHigh: (value: number) => void;
  onLearn: () => void | Promise<void>;
  onAddMapping: () => void;
  onLoadMappings: () => void | Promise<void>;
  onSaveMappings: () => void | Promise<void>;
  onRemoveMapping: (index: number) => void;
}

export function OscControlMappingPanel(props: OscControlMappingPanelProps) {
  return (
    <div class="oscInput ioOperatorSurface">
      <section class="ioConnectionDesk" data-io-default-surface="osc">
        <header class="ioDeskHeader">
          <div>
            <h2>OSC Connection</h2>
            <span>UDP listener</span>
          </div>
          <span class={`ioConnectionState ${props.running ? "ok" : "idle"}`}><i aria-hidden="true" />{props.running ? "Listening" : "Stopped"}</span>
        </header>
        <div class="ioConnectionControls">
          <label>
            Bind IP
            <input data-io-control="osc-bind-ip" value={props.bindIp} disabled={props.running} onInput={(event) => props.onBindIp(event.currentTarget.value)} />
          </label>
          <label>
            Port
            <input data-io-control="osc-port" type="number" min="1" value={props.port} disabled={props.running} onInput={(event) => props.onPort(Number(event.currentTarget.value))} />
          </label>
          <Show when={props.running} fallback={
            <button data-io-control="osc-start" class="primary" onClick={() => void props.onStart()}>Start OSC</button>
          }>
            <button data-io-control="osc-stop" onClick={() => void props.onStop()}>Stop OSC</button>
          </Show>
        </div>
      </section>

      <div class="ioDisclosureStack">
        <details class="ioDisclosure" data-io-disclosure="osc-mapping">
          <summary>Control mapping and learn</summary>
          <div class="ioDisclosureBody ioMappingWorkbench" data-io-disclosure-body>
          <div class="mappingEditorDesk">
      <h3>OSC mapping</h3>
      <label>
        Address
        <input
          data-io-control="osc-map-address"
          value={props.mapAddress}
          placeholder="/touchosc/page/*/fader/1"
          onInput={(event) => props.onMapAddress(event.currentTarget.value)}
        />
      </label>
      <div class="split">
        <label>
          Action
          <select value={props.mapAction} onInput={(event) => props.onMapAction(event.currentTarget.value as OscControlAction)}>
            <option value="FixtureAttribute">Fixture Attribute</option>
            <option value="FixtureHighlight">Fixture Highlight</option>
            <option value="FixtureSolo">Fixture Solo</option>
            <option value="FixturePark">Fixture Park</option>
            <option value="GroupHighlight">Group Highlight</option>
            <option value="GroupSolo">Group Solo</option>
            <option value="GroupPark">Group Park</option>
            <option value="TriggerCue">Trigger Cue</option>
            <option value="TriggerNextCue">Cue Next</option>
            <option value="TriggerPreviousCue">Cue Previous</option>
            <option value="EffectEnabled">Effect Enable</option>
            <option value="NodeGraphEnabled">Node Graph Enable</option>
            <option value="VideoParam">Video Param</option>
            <option value="VideoCuePointAdd">Video Cue Point Add</option>
            <option value="VideoCuePointRemove">Video Cue Point Remove</option>
            <option value="VideoCuePointJump">Video Cue Point Jump</option>
            <option value="VideoCuePointPrevious">Video Cue Point Previous</option>
            <option value="VideoCuePointNext">Video Cue Point Next</option>
            <option value="VideoLayerEnabled">Video Layer Enable</option>
            <option value="VideoLayerSolo">Video Layer Solo</option>
            <option value="VideoPlay">Video Play</option>
            <option value="VideoLoop">Video A-B Loop</option>
            <option value="VideoLayerFade">Video Layer Fade</option>
            <option value="VideoOutputEnabled">Video Output Enable</option>
            <option value="VideoOutputOpacity">Video Output Opacity</option>
            <option value="VideoOutputFade">Video Output Fade</option>
            <option value="VideoOutputMappingField">Video Output Mapping Field</option>
            <option value="VideoOutputMappingPreset">Video Output Mapping Preset</option>
            <option value="VideoOutputBlackout">Video Output Blackout</option>
            <option value="TimelinePlay">Timeline Play</option>
            <option value="TimelineSeek">Timeline Seek</option>
            <option value="TimelineBeatPrevious">Beat Previous</option>
            <option value="TimelineBeatNext">Beat Next</option>
            <option value="LightingMaster">Lighting Master</option>
            <option value="VideoMaster">Video Master</option>
            <option value="GroupSubmaster">Group Submaster</option>
            <option value="SetBpm">Set BPM</option>
            <option value="TapBpm">Tap BPM</option>
            <option value="CueFadePause">Cue Fade Pause</option>
            <option value="Blackout">DMX Blackout</option>
            <option value="AllBlackout">All Blackout</option>
            <option value="VideoBlackout">Video Blackout</option>
            <option value="ClearFixtureFlags">Clear Fixture Flags</option>
          </select>
        </label>
        <label>
          Attribute
          <input
            value={props.mapAttribute}
            onInput={(event) => props.onMapAttribute(event.currentTarget.value)}
            disabled={props.mapAction !== "FixtureAttribute"}
          />
        </label>
      </div>

      <Show when={props.mapAction === "ClearFixtureFlags"}>
        <label>
          Clear flags
          <select value={props.clearFixtureFlagKind} onInput={(event) => props.onMapAttribute(event.currentTarget.value)}>
            <For each={fixtureFlagClearKinds}>{(kind) => <option value={kind}>{kind}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "TriggerCue"}>
        <label>
          Cue
          <select value={props.selectedCueId ?? ""} onInput={(event) => props.onMapCueId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.cues}>{(cue) => <option data-no-localize value={cue.id}>{cue.id}: {cue.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "EffectEnabled"}>
        <label>
          Effect
          <select value={props.selectedEffectId ?? ""} onInput={(event) => props.onMapEffectId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.effects}>{(effect) => <option value={effect.id}>{effect.id}: {effect.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "NodeGraphEnabled"}>
        <label>
          Node Graph
          <select value={props.selectedNodeGraphId ?? ""} onInput={(event) => props.onMapNodeGraphId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.node_graphs}>{(graph) => <option value={graph.id}>{graph.id}: {graph.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "GroupSubmaster" || isGroupFlagMappingAction(props.mapAction)}>
        <label>
          Group ID
          <input value={props.mapGroupId} onInput={(event) => props.onMapGroupId(event.currentTarget.value)} />
        </label>
      </Show>
      <Show when={isVideoLayerMappingAction(props.mapAction)}>
        <label>
          Layer
          <select value={props.selectedLayerId ?? ""} onInput={(event) => props.onMapLayerId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.video.layers}>{(layer) => <option data-no-localize value={layer.id}>{layer.id}: {layer.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={isVideoOutputMappingAction(props.mapAction)}>
        <label>
          Output
          <select value={props.selectedOutputId ?? ""} onInput={(event) => props.onMapOutputId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.video.outputs}>{(output) => <option data-no-localize value={output.id}>{output.id}: {output.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoOutputMappingField"}>
        <label>
          Mapping Field
          <select value={props.selectedMappingField} onInput={(event) => props.onMappingField(event.currentTarget.value)}>
            <For each={videoOutputMappingFieldOptions}>{(field) => <option value={field.value}>{field.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoOutputMappingPreset"}>
        <label>
          Mapping Preset
          <select
            value={props.selectedMappingPresetLabel}
            disabled={props.snapshot.video.mapping_presets.length === 0}
            onInput={(event) => props.onMappingPresetLabel(event.currentTarget.value)}
          >
            <Show when={props.snapshot.video.mapping_presets.length === 0}>
              <option value="">No presets</option>
            </Show>
            <For each={props.snapshot.video.mapping_presets}>{(preset) => <option data-no-localize value={preset.label}>{preset.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoParam"}>
        <label>
          Video Param
          <select value={props.mapVideoParam} onInput={(event) => props.onMapVideoParam(event.currentTarget.value as VideoParam)}>
            <option value="Opacity">Opacity</option>
            <option value="Speed">Speed</option>
            <option value="PositionMs">Position</option>
            <option value="BpmSyncEnabled">BPM Sync</option>
            <option value="BpmSyncRatio">BPM Sync Ratio</option>
            <option value="BpmSyncLoopBars">BPM Loop Bars</option>
            <option value="TransformX">X</option>
            <option value="TransformY">Y</option>
            <option value="TransformScaleX">Scale X</option>
            <option value="TransformScaleY">Scale Y</option>
            <option value="TransformRotationDeg">Rotation</option>
            <option value="ColorBrightness">Brightness</option>
            <option value="ColorContrast">Contrast</option>
            <option value="ColorHueDeg">Hue</option>
            <option value="ColorSaturation">Saturation</option>
            <option value="ColorGamma">Gamma</option>
            <option value="FxPixelate">Pixelate</option>
            <option value="FxBlur">Blur</option>
            <option value="FxGlow">Glow</option>
            <option value="FxEdge">Edge</option>
            <option value="FxKeyRed">Key R</option>
            <option value="FxKeyGreen">Key G</option>
            <option value="FxKeyBlue">Key B</option>
            <option value="FxKeyThreshold">Key Threshold</option>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoCuePointJump"}>
        <label>
          Cue point index
          <input type="number" min="0" value={props.mapCuePointIndex} onInput={(event) => props.onMapCuePointIndex(Number(event.currentTarget.value))} />
        </label>
      </Show>
      <Show
        when={
          props.mapAction === "VideoOutputFade" ||
          props.mapAction === "VideoLayerFade" ||
          props.mapAction === "VideoCuePointAdd" ||
          props.mapAction === "VideoCuePointRemove"
        }
      >
        <label>
          {props.mapAction === "VideoOutputFade" || props.mapAction === "VideoLayerFade" ? "Fade ms" : "Cue point ms"}
          <input type="number" min="0" step="10" value={props.mapDurationMs} onInput={(event) => props.onMapDurationMs(Number(event.currentTarget.value))} />
        </label>
      </Show>
      <div class="split">
        <label>
          {props.mapAction === "VideoLoop"
            ? "Loop In ms"
            : props.mapAction === "SetBpm"
              ? "BPM Low"
              : props.mapAction === "VideoOutputMappingField"
                ? "Field Low"
                : "Low"}
          <input type="number" value={props.mapLow} onInput={(event) => props.onMapLow(Number(event.currentTarget.value))} />
        </label>
        <label>
          {props.mapAction === "VideoLoop"
            ? "Loop Out ms"
            : props.mapAction === "SetBpm"
              ? "BPM High"
              : props.mapAction === "VideoOutputMappingField"
                ? "Field High"
                : "High"}
          <input type="number" value={props.mapHigh} onInput={(event) => props.onMapHigh(Number(event.currentTarget.value))} />
        </label>
      </div>
      <div class="buttonRow">
        <button onClick={() => void props.onLearn()} disabled={props.running}>
          Learn
        </button>
        <button onClick={props.onAddMapping}>Add OSC Mapping</button>
        <button onClick={() => void props.onLoadMappings()}>Load Mapping</button>
        <button onClick={() => void props.onSaveMappings()} disabled={props.mappings.length === 0}>
          Save Mapping
        </button>
      </div>
      </div>
      <section class="mappingListDesk">
        <header class="ioDeskHeader">
          <h2>Mappings</h2>
          <span>{props.mappings.length}</span>
        </header>
      <div class="timelineList">
        <For each={props.mappings}>
          {(mapping, index) => (
            <div class="timelineItem">
              <strong>{mapping.address}</strong>
              <span>{props.mappingTargetLabel(mapping)}</span>
              <button onClick={() => props.onRemoveMapping(index())}>Remove</button>
            </div>
          )}
        </For>
      </div>
      </section>
          </div>
        </details>
      </div>
    </div>
  );
}
