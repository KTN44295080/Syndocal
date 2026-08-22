import { createMemo, createSignal, For, Show, type JSX } from "solid-js";
import type {
  MediaAssetAvailability,
  MediaAssetSummary,
  TimelineLayerKind,
  TimelineLayerSummary,
} from "../types";
import type { TimelineExternalDragPayload } from "../timelineExternalDrag";
import {
  setTimelineExternalDragPayload,
  timelineExternalDragPayloadForMedia,
  timelineExternalDragPayloadForScene,
} from "../timelineExternalDrag";
import { loadUiLocale, translateUiText } from "../uiLocalization";
import type { TimelineSceneBlockCueOption } from "./TimelineSceneBlocksEditor";

export interface TimelineSourceShelfProps {
  cueOptions: TimelineSceneBlockCueOption[];
  mediaAssets: MediaAssetSummary[];
  mediaAssetAvailabilityById: Record<number, MediaAssetAvailability>;
  timelineLayers: TimelineLayerSummary[];
  positionMs: number;
  snapTimeMs: (timeMs: number) => number;
  onPlace: (
    source: TimelineExternalDragPayload,
    targetLayer: TimelineLayerSummary,
    timeMs: number,
  ) => void | Promise<void>;
  onStatus: (message: string) => void;
  onOpenInspector: () => void;
  contextMode?: "sources" | "inspector";
  onContextModeChange?: (mode: "sources" | "inspector") => void;
  inspectorContent?: JSX.Element;
}

const mediaAssetHasVideo = (asset: MediaAssetSummary) =>
  asset.source.kind === "StillImage" ||
  (asset.source.metadata?.width != null && asset.source.metadata?.height != null);

const mediaAssetHasAudio = (asset: MediaAssetSummary) => asset.source.metadata?.has_audio === true;

const mediaAssetAvailabilityLabel = (availability: MediaAssetAvailability | undefined) => {
  switch (availability?.kind) {
    case "available_verified": return "Verified";
    case "available_unverified": return "Available · not hash-verified";
    case "missing": return "Missing";
    case "hash_mismatch": return "Hash mismatch";
    case "unreadable": return "Unreadable";
    case "live_source": return "Live source";
    default: return "Not verified";
  }
};

const mediaAssetPlacementAllowed = (availability: MediaAssetAvailability | undefined) =>
  availability === undefined || availability.kind === "available_verified" || availability.kind === "available_unverified";

const mediaTimelineSourceLabel = (asset: MediaAssetSummary, laneKind: "Video" | "Audio") =>
  mediaAssetHasVideo(asset) && mediaAssetHasAudio(asset)
    ? `${laneKind} + linked ${laneKind === "Video" ? "Audio" : "Video"}`
    : laneKind;

const mediaTimelineSourceDescription = (asset: MediaAssetSummary, laneKind: "Video" | "Audio") => {
  const linked = laneKind === "Video" ? mediaAssetHasAudio(asset) : mediaAssetHasVideo(asset);
  return linked ? `; linked ${laneKind === "Video" ? "Audio" : "Video"} follows` : "";
};

// Source labels contain project-owned names, so translate the stable operator
// framing here rather than excluding dynamic statuses and ARIA labels from the
// locale pass.
const localizedSourceText = (source: string) => translateUiText(source, loadUiLocale());

export function TimelineSourceShelf(props: TimelineSourceShelfProps) {
  const [sourceShelfTab, setSourceShelfTab] = createSignal<"Scenes" | "Media Library">("Scenes");
  const [mediaShelfFilter, setMediaShelfFilter] = createSignal<"All" | "Video" | "Audio">("All");
  // A placement is authorized only by an explicitly selected, exact layer ID.
  // Do not silently route a linked companion to the first unlocked lane.
  const [sourceShelfTargetLayerIds, setSourceShelfTargetLayerIds] = createSignal<Partial<Record<TimelineLayerKind, number>>>({});
  const sourceContextMode = () => props.contextMode ?? "sources";
  const selectSourceContextMode = (mode: "sources" | "inspector") => {
    props.onContextModeChange?.(mode);
    if (mode === "inspector") props.onOpenInspector();
  };
  const focusSourceContextMode = (current: "sources" | "inspector", direction: -1 | 1 | "first" | "last") => {
    const modes = ["sources", "inspector"] as const;
    const index = modes.indexOf(current);
    const target = direction === "first" ? modes[0] : direction === "last" ? modes[1] : modes[(index + direction + modes.length) % modes.length];
    selectSourceContextMode(target);
    queueMicrotask(() => document.getElementById(`timeline-source-context-tab-${target}`)?.focus());
  };

  const mediaShelfAssets = createMemo(() => props.mediaAssets.filter((asset) => {
    const filter = mediaShelfFilter();
    return filter === "All" || (filter === "Video" ? mediaAssetHasVideo(asset) : mediaAssetHasAudio(asset));
  }));
  const sourceShelfLayers = (kind: TimelineLayerKind) =>
    props.timelineLayers.filter((layer) => layer.kind === kind && !layer.locked);
  const sourceShelfTargetLayer = (kind: TimelineLayerKind) => {
    const selectedId = sourceShelfTargetLayerIds()[kind];
    return sourceShelfLayers(kind).find((layer) => layer.id === selectedId) ?? null;
  };
  const setSourceShelfTargetLayer = (kind: TimelineLayerKind, value: string) => {
    const id = Number(value);
    setSourceShelfTargetLayerIds((current) => ({
      ...current,
      [kind]: Number.isSafeInteger(id) && id > 0 ? id : undefined,
    }));
  };
  const mediaPayload = (asset: MediaAssetSummary, laneKind: "Video" | "Audio") =>
    timelineExternalDragPayloadForMedia(
      asset.id,
      laneKind,
      mediaAssetHasVideo(asset) ? sourceShelfTargetLayer("Video")?.id ?? null : null,
      mediaAssetHasAudio(asset) ? sourceShelfTargetLayer("Audio")?.id ?? null : null,
    );
  const placeSourceShelfPayload = (payload: TimelineExternalDragPayload | null) => {
    if (!payload) return;
    if (payload.kind === "media_asset") {
      const asset = props.mediaAssets.find((candidate) => candidate.id === payload.media_asset_id);
      if (asset && !mediaAssetPlacementAllowed(props.mediaAssetAvailabilityById[asset.id])) {
        props.onStatus(localizedSourceText(`Media Asset ${asset.label} is unavailable for Timeline placement.`));
        return;
      }
    }
    const target = sourceShelfTargetLayer(payload.lane_kind);
    if (!target) {
      props.onStatus(localizedSourceText(`An unlocked ${payload.lane_kind} Timeline lane is required.`));
      return;
    }
    void props.onPlace(payload, target, props.snapTimeMs(props.positionMs));
  };
  const startSourceShelfDrag = (event: DragEvent, payload: TimelineExternalDragPayload | null) => {
    if (!payload) {
      event.preventDefault();
      return;
    }
    setTimelineExternalDragPayload(event, payload);
  };
  const renderTargetSelect = (kind: TimelineLayerKind, label: string) => (
    <label class="timelineExternalSourceTarget">
      <span>{label}</span>
      <select
        value={sourceShelfTargetLayer(kind)?.id ?? ""}
        onChange={(event) => setSourceShelfTargetLayer(kind, event.currentTarget.value)}
      >
        <option value="">Select a lane</option>
        <Show when={sourceShelfLayers(kind).length === 0}>
          <option value="">Unavailable</option>
        </Show>
        <For each={sourceShelfLayers(kind)}>
          {(layer) => <option value={layer.id} data-no-localize>{layer.label}</option>}
        </For>
      </select>
    </label>
  );

  return (
    <section class="timelineExternalSourceShelf" aria-label="Timeline source shelf" data-timeline-source-shelf>
      <header class="timelineExternalSourceShelfHeader">
        <div>
          <h3>Sources</h3>
          <p>Drag a Scene, Video, or Audio source onto its matching lane.</p>
        </div>
        <div class="timelineExternalSourceShelfModes" role="tablist" aria-label="Timeline source or inspector">
          <button type="button" id="timeline-source-context-tab-sources" role="tab" data-timeline-source-shelf-mode="sources" aria-selected={sourceContextMode() === "sources"} aria-controls="timeline-source-context-panel-sources" tabindex={sourceContextMode() === "sources" ? 0 : -1} classList={{ active: sourceContextMode() === "sources" }} onClick={() => selectSourceContextMode("sources")} onKeyDown={(event) => {
            if (event.key === "ArrowRight" || event.key === "ArrowDown") { event.preventDefault(); focusSourceContextMode("sources", 1); }
            else if (event.key === "ArrowLeft" || event.key === "ArrowUp") { event.preventDefault(); focusSourceContextMode("sources", -1); }
            else if (event.key === "Home") { event.preventDefault(); focusSourceContextMode("sources", "first"); }
            else if (event.key === "End") { event.preventDefault(); focusSourceContextMode("sources", "last"); }
          }}>Sources</button>
          <button type="button" id="timeline-source-context-tab-inspector" role="tab" data-timeline-source-shelf-mode="inspector" aria-selected={sourceContextMode() === "inspector"} aria-controls="timeline-source-context-panel-inspector" tabindex={sourceContextMode() === "inspector" ? 0 : -1} classList={{ active: sourceContextMode() === "inspector" }} onClick={() => selectSourceContextMode("inspector")} onKeyDown={(event) => {
            if (event.key === "ArrowLeft" || event.key === "ArrowUp") { event.preventDefault(); focusSourceContextMode("inspector", -1); }
            else if (event.key === "ArrowRight" || event.key === "ArrowDown") { event.preventDefault(); focusSourceContextMode("inspector", 1); }
            else if (event.key === "Home") { event.preventDefault(); focusSourceContextMode("inspector", "first"); }
            else if (event.key === "End") { event.preventDefault(); focusSourceContextMode("inspector", "last"); }
          }}>Inspector</button>
        </div>
        <div class="timelineExternalSourceShelfTabs" role="group" aria-label="Timeline source categories">
          <button
            type="button"
            data-timeline-source-shelf-category="scenes"
            aria-pressed={sourceShelfTab() === "Scenes"}
            classList={{ active: sourceShelfTab() === "Scenes" }}
            onClick={() => setSourceShelfTab("Scenes")}
          >Scenes</button>
          <button
            type="button"
            data-timeline-source-shelf-category="media"
            aria-pressed={sourceShelfTab() === "Media Library"}
            classList={{ active: sourceShelfTab() === "Media Library" }}
            onClick={() => setSourceShelfTab("Media Library")}
          >Media Library</button>
        </div>
      </header>
      <Show when={sourceContextMode() === "sources" && sourceShelfTab() === "Scenes"}>
        <div id="timeline-source-context-panel-sources" role="tabpanel" aria-labelledby="timeline-source-context-tab-sources" class="timelineExternalSourceShelfBody" aria-label="Scenes">
          {renderTargetSelect("Lighting", "Lighting lane")}
          <Show when={props.cueOptions.length > 0} fallback={<p class="emptyState">No Scenes are available yet.</p>}>
            <div class="timelineExternalSourceShelfItems">
              <For each={props.cueOptions}>
                {(cue) => (
                  <button
                    type="button"
                    class="timelineExternalSourceCard lighting"
                    draggable={true}
                    data-timeline-external-source="scene"
                    data-timeline-external-source-kind="Lighting"
                    data-timeline-source-cue-id={cue.id}
                    aria-label={localizedSourceText(`Drag Scene ${cue.label} to a Lighting Timeline lane`)}
                    title={localizedSourceText("Drag to a Lighting Timeline lane; click to place at the playhead")}
                    onDragStart={(event) => startSourceShelfDrag(event, timelineExternalDragPayloadForScene(cue.id))}
                    onClick={() => placeSourceShelfPayload(timelineExternalDragPayloadForScene(cue.id))}
                  >
                    <strong data-no-localize>{cue.label}</strong>
                    <span>Lighting</span>
                  </button>
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>
      <Show when={sourceContextMode() === "sources" && sourceShelfTab() === "Media Library"}>
        <div id="timeline-source-context-panel-sources" role="tabpanel" aria-labelledby="timeline-source-context-tab-sources" class="timelineExternalSourceShelfBody" aria-label="Media Library">
          <div class="timelineExternalSourceShelfFilters" role="group" aria-label="Media source filter">
            <For each={["All", "Video", "Audio"] as const}>
              {(filter) => (
                <button
                  type="button"
                  data-timeline-source-shelf-filter={filter.toLowerCase()}
                  classList={{ active: mediaShelfFilter() === filter }}
                  aria-pressed={mediaShelfFilter() === filter}
                  onClick={() => setMediaShelfFilter(filter)}
                >{filter}</button>
              )}
            </For>
          </div>
          <div class="timelineExternalSourceTargets" role="group" aria-label="Media Timeline target lanes">
            {renderTargetSelect("Video", "Video lane")}
            {renderTargetSelect("Audio", "Audio lane")}
          </div>
          <Show when={mediaShelfAssets().length > 0} fallback={<p class="emptyState">No matching Media Library sources.</p>}>
            <div class="timelineExternalSourceShelfItems">
              <For each={mediaShelfAssets()}>
                {(asset) => (
                  <article class="timelineExternalSourceCard media" data-timeline-source-media-id={asset.id}>
                    <div class="timelineExternalSourceCardTitle">
                      <strong data-no-localize>{asset.label}</strong>
                      <small>{mediaAssetAvailabilityLabel(props.mediaAssetAvailabilityById[asset.id])}</small>
                    </div>
                    <div class="timelineExternalSourceCardActions">
                      <Show when={mediaAssetHasVideo(asset)}>
                        <button
                          type="button"
                          draggable={true}
                          disabled={!mediaAssetPlacementAllowed(props.mediaAssetAvailabilityById[asset.id])}
                          data-timeline-external-source="media_asset"
                          data-timeline-external-source-kind="Video"
                          data-timeline-source-media-kind="Video"
                          aria-label={localizedSourceText(`Drag media ${asset.label} to a Video lane${mediaTimelineSourceDescription(asset, "Video")}`)}
                          title={localizedSourceText(`Drag to a Video Timeline lane${mediaTimelineSourceDescription(asset, "Video")}; click to place at the playhead`)}
                          onDragStart={(event) => startSourceShelfDrag(event, mediaPayload(asset, "Video"))}
                          onClick={() => placeSourceShelfPayload(mediaPayload(asset, "Video"))}
                        >{mediaTimelineSourceLabel(asset, "Video")}</button>
                      </Show>
                      <Show when={mediaAssetHasAudio(asset)}>
                        <button
                          type="button"
                          draggable={true}
                          disabled={!mediaAssetPlacementAllowed(props.mediaAssetAvailabilityById[asset.id])}
                          data-timeline-external-source="media_asset"
                          data-timeline-external-source-kind="Audio"
                          data-timeline-source-media-kind="Audio"
                          aria-label={localizedSourceText(`Drag media ${asset.label} to an Audio lane${mediaTimelineSourceDescription(asset, "Audio")}`)}
                          title={localizedSourceText(`Drag to an Audio Timeline lane${mediaTimelineSourceDescription(asset, "Audio")}; click to place at the playhead`)}
                          onDragStart={(event) => startSourceShelfDrag(event, mediaPayload(asset, "Audio"))}
                          onClick={() => placeSourceShelfPayload(mediaPayload(asset, "Audio"))}
                        >{mediaTimelineSourceLabel(asset, "Audio")}</button>
                      </Show>
                      <Show when={!mediaAssetHasVideo(asset) && !mediaAssetHasAudio(asset)}>
                        <span class="timelineExternalDragUnavailable">Timeline source unavailable</span>
                      </Show>
                    </div>
                  </article>
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>
      <Show when={sourceContextMode() === "inspector"}>
        <section id="timeline-source-context-panel-inspector" role="tabpanel" aria-labelledby="timeline-source-context-tab-inspector">
          {props.inspectorContent ?? <div class="timelineExternalSourceShelfInspector" data-timeline-source-shelf-inspector><p>Select a Timeline item to inspect it.</p></div>}
        </section>
      </Show>
    </section>
  );
}
