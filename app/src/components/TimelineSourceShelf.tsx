import { createMemo, createSignal, For, Show, type JSX } from "solid-js";
import { bankAuthorityIssueMessage, type FullBankAuthoritySnapshot } from "../bankAuthority";
import { groupIdentityCss, groupIdentityHue } from "../identityColor";
import { displayNumber } from "../numberDisplay";
import type {
  MediaAssetAvailability,
  MediaAssetId,
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
import {
  groupTimelineSourceShelfBanks,
  timelineSourceShelfBankIdentity,
  timelineSourceShelfBankViewsEqual,
  timelineSourceShelfCueOptionsMatchAuthority,
  type TimelineSourceShelfBankView,
} from "../timelineSceneBlocks";
import {
  mediaAssetAvailabilityAllowsTimelinePlacement,
  resolveTimelineExternalLayer,
} from "../timelineExternalDropRuntime";
import { translateUiText, type UiLocale } from "../uiLocalization";
import type { TimelineSceneBlockCueOption } from "./TimelineSceneBlocksEditor";

export interface TimelineSourceShelfProps {
  cueOptions: TimelineSceneBlockCueOption[];
  /** The exact complete App verdict. The Timeline never computes a subset authority. */
  bankAuthority: FullBankAuthoritySnapshot;
  /** Non-null only while editing a child Timeline; determines the exact allowed Scene set. */
  timelineChildCueId: number | null;
  uiLocale: UiLocale;
  mediaAssets: MediaAssetSummary[];
  mediaAssetAvailabilityById: Record<number, MediaAssetAvailability>;
  /** The App-owned machine-local inspection authority; never a Shelf-local probe. */
  onVerify: (assetIds: MediaAssetId[]) => void | Promise<void>;
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
    case "live_source": return "Live source · inspected";
    default: return "Verify before placement";
  }
};

const mediaTimelineSourceLabel = (asset: MediaAssetSummary, laneKind: "Video" | "Audio") =>
  mediaAssetHasVideo(asset) && mediaAssetHasAudio(asset)
    ? `${laneKind} + linked ${laneKind === "Video" ? "Audio" : "Video"}`
    : laneKind;

const mediaTimelineSourceDescription = (asset: MediaAssetSummary, laneKind: "Video" | "Audio") => {
  const linked = laneKind === "Video" ? mediaAssetHasAudio(asset) : mediaAssetHasVideo(asset);
  return linked ? `; linked ${laneKind === "Video" ? "Audio" : "Video"} follows` : "";
};

export function TimelineSourceShelf(props: TimelineSourceShelfProps) {
  // Source labels contain project-owned names, so translate only the stable
  // operator framing from the same reactive locale authority as App.
  const localizedSourceText = (source: string) => translateUiText(source, props.uiLocale);
  const [sourceShelfTab, setSourceShelfTab] = createSignal<"Scenes" | "Media Library">("Scenes");
  const [mediaShelfFilter, setMediaShelfFilter] = createSignal<"All" | "Video" | "Audio">("All");
  // A click has no canvas target, so retain an explicit choice only for the
  // ambiguous case. A single unlocked lane is the safe deterministic target;
  // DnD uses the lane actually hit in TimelineOverview as its primary target.
  const [sourceShelfTargetLayerIds, setSourceShelfTargetLayerIds] = createSignal<Partial<Record<TimelineLayerKind, number>>>({});
  const sourceContextMode = () => props.contextMode ?? "sources";
  const selectSourceContextMode = (mode: "sources" | "inspector") => {
    props.onContextModeChange?.(mode);
    if (mode === "inspector") props.onOpenInspector();
  };
  const selectSourceShelfTab = (tab: "Scenes" | "Media Library") => {
    setSourceShelfTab(tab);
    // The category buttons remain in the compact header while the Inspector
    // is open. Make their intent explicit: choosing a source category always
    // returns to the source browser instead of silently changing hidden state.
    if (sourceContextMode() !== "sources") selectSourceContextMode("sources");
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
  const mediaShelfHasSourceForKind = (kind: "Video" | "Audio") =>
    mediaShelfAssets().some((asset) => kind === "Video" ? mediaAssetHasVideo(asset) : mediaAssetHasAudio(asset));
  const mediaShelfRelevantKinds = () =>
    (["Video", "Audio"] as const).filter((kind) => mediaShelfHasSourceForKind(kind));
  const bankAuthority = () => props.bankAuthority;
  const sceneAuthorityUnavailable = createMemo(() => {
    const issue = bankAuthority().issue;
    if (issue) return bankAuthorityIssueMessage(issue);
    return timelineSourceShelfCueOptionsMatchAuthority(
      props.cueOptions,
      bankAuthority(),
      props.timelineChildCueId,
    )
      ? null
      : "Timeline Scene sources are unavailable because the Scene identity set does not match the current project.";
  });
  // Read-only Bank/Scene representation for Timeline placement. cueOptions and
  // the App verdict stay the authoritative inputs; this surface never edits a Bank.
  const shelfBanks = createMemo(
    () => bankAuthority().issue === null
      && sceneAuthorityUnavailable() === null
      ? groupTimelineSourceShelfBanks(props.cueOptions, bankAuthority(), props.timelineChildCueId)
      : [],
    [] as TimelineSourceShelfBankView<TimelineSceneBlockCueOption>[],
    { equals: timelineSourceShelfBankViewsEqual },
  );
  const renderSceneSourceCard = (cue: TimelineSceneBlockCueOption) => {
    const exactCue = bankAuthority().cueById.get(cue.id);
    if (bankAuthority().issue || !exactCue || exactCue.cue_list_id !== cue.cue_list_id) return null;
    const placementPayload = timelineExternalDragPayloadForScene(cue.id);
    return <button
      type="button"
      class="timelineExternalSourceCard lighting"
      draggable={true}
      data-timeline-external-source="scene"
      data-timeline-external-source-kind="Lighting"
      data-timeline-source-cue-id={cue.id}
      data-timeline-source-shelf-scene-placeable="true"
      aria-label={localizedSourceText(`Drag Scene ${cue.label} to a Lighting Timeline lane`)}
      title={localizedSourceText("Drag to a Lighting Timeline lane; click to place at the playhead")}
      onDragStart={(event) => startSourceShelfDrag(event, placementPayload)}
      onClick={() => placeSourceShelfPayload(placementPayload)}
    >
      <span class="timelineExternalSourceCardPrimaryRow" data-timeline-source-cue-primary-row>
        <span class="timelineExternalSourceCueNumber" data-no-localize>{cue.cue_number}</span>
        <strong data-no-localize>{cue.label}</strong>
      </span>
      <span class="timelineExternalSourceCardMetaRow" data-timeline-source-cue-meta-row>
        <span
          class={`sceneMatrixKindBadge uiMicroLabel ${cue.kind === "TIMELINE" ? "super" : cue.kind.toLowerCase()}`}
          data-timeline-source-cue-kind={cue.kind}
          data-no-localize
        >
          {cue.kind}
        </span>
        <Show when={cue.flash}>
          <span class="sceneMatrixFlashBadge">FLASH</span>
        </Show>
        <Show when={cue.replace_group}>
          <span class="sceneMatrixReplaceBadge">Replace group</span>
        </Show>
        <Show when={cue.super_scene}>
          <span class="sceneMatrixKindBadge superScene timelineExternalSuperSceneMark" data-timeline-source-super-scene={cue.id} data-no-localize>
            TL
          </span>
        </Show>
        <small>{displayNumber(cue.fade_ms, 0)}ms</small>
      </span>
    </button>;
  };
  const sourceShelfLayers = (kind: TimelineLayerKind) =>
    props.timelineLayers.filter((layer) => layer.kind === kind && !layer.locked);
  const sourceShelfLayerResolution = (kind: TimelineLayerKind) =>
    resolveTimelineExternalLayer(sourceShelfLayers(kind), kind, sourceShelfTargetLayerIds()[kind] ?? null);
  const sourceShelfTargetLayer = (kind: TimelineLayerKind) => {
    return sourceShelfLayerResolution(kind).selected;
  };
  const revealTimelineLayer = (layerId: number | undefined) => {
    if (typeof layerId !== "number" || !Number.isSafeInteger(layerId) || layerId <= 0) return;
    const resolvedLayerId = layerId;
    queueMicrotask(() => {
      const scrollport = document.querySelector<HTMLElement>("[data-timeline-layer-scrollport]");
      const layer = scrollport?.querySelector<HTMLElement>(
        `[data-timeline-layer-gutter][data-timeline-layer-id="${resolvedLayerId}"]`,
      );
      layer?.scrollIntoView({ block: "nearest", inline: "nearest" });
    });
  };
  const setSourceShelfTargetLayer = (kind: TimelineLayerKind, value: string) => {
    const id = Number(value);
    setSourceShelfTargetLayerIds((current) => ({
      ...current,
      [kind]: Number.isSafeInteger(id) && id > 0 ? id : undefined,
    }));
    // The compact upper arranger can contain more fixed-height lanes than its
    // viewport. Reveal the exact lane the operator selected without scrolling
    // the page or stealing focus from this selector.
    revealTimelineLayer(id);
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
    if (payload.kind === "scene") {
      const authority = bankAuthority();
      if (authority.issue) {
        props.onStatus(localizedSourceText(bankAuthorityIssueMessage(authority.issue)));
        return;
      }
      if (!authority.cueById.has(payload.cue_id)) {
        props.onStatus(localizedSourceText(`Scene ${payload.cue_id} is no longer available.`));
        return;
      }
    }
    if (payload.kind === "media_asset") {
      const asset = props.mediaAssets.find((candidate) => candidate.id === payload.media_asset_id);
      if (asset && !mediaAssetAvailabilityAllowsTimelinePlacement(props.mediaAssetAvailabilityById[asset.id])) {
        props.onStatus(localizedSourceText(`Media Asset ${asset.label} is unavailable for Timeline placement.`));
        return;
      }
    }
    const resolution = sourceShelfLayerResolution(payload.lane_kind);
    const target = resolution.selected;
    if (!target) {
      props.onStatus(localizedSourceText(
        resolution.mode === "stale"
          ? `Selected ${payload.lane_kind} Timeline lane is no longer available.`
          : resolution.mode === "ambiguous"
          ? payload.kind === "media_asset"
            ? `Select an unlocked ${payload.lane_kind} Timeline lane before placing media.`
            : `An unlocked ${payload.lane_kind} Timeline lane is required.`
          : `An unlocked ${payload.lane_kind} Timeline lane is required.`,
      ));
      return;
    }
    void props.onPlace(payload, target, props.snapTimeMs(props.positionMs));
  };
  const startSourceShelfDrag = (event: DragEvent, payload: TimelineExternalDragPayload | null) => {
    if (!payload || (payload.kind === "scene" && bankAuthority().issue !== null)) {
      event.preventDefault();
      return;
    }
    if (payload.kind === "media_asset") {
      const asset = props.mediaAssets.find((candidate) => candidate.id === payload.media_asset_id);
      if (asset && !mediaAssetAvailabilityAllowsTimelinePlacement(props.mediaAssetAvailabilityById[asset.id])) {
        event.preventDefault();
        props.onStatus(localizedSourceText(`Media Asset ${asset.label} is unavailable for Timeline placement.`));
        return;
      }
    }
    setTimelineExternalDragPayload(event, payload);
  };
  const renderTargetSelect = (kind: TimelineLayerKind, label: string) => (
    <Show when={
      sourceShelfLayerResolution(kind).candidates.length > 1
      || sourceShelfLayerResolution(kind).mode === "stale"
    }>
      <label class="timelineExternalSourceTarget">
        <span>{label}</span>
        <select
          data-timeline-source-target-kind={kind}
          value={sourceShelfTargetLayer(kind)?.id ?? ""}
          onChange={(event) => setSourceShelfTargetLayer(kind, event.currentTarget.value)}
        >
          <option value="">{localizedSourceText("Select a lane")}</option>
          <For each={sourceShelfLayerResolution(kind).candidates}>
            {(layer) => <option value={layer.id} data-no-localize>{layer.label}</option>}
          </For>
        </select>
      </label>
    </Show>
  );
  const renderNoLaneStatus = (kind: TimelineLayerKind) => (
    <Show when={sourceShelfLayerResolution(kind).mode === "none"}>
      <p
        class="emptyState timelineExternalSourceLaneUnavailable"
        role="alert"
        data-timeline-source-no-target-kind={kind}
      >
        {localizedSourceText(`An unlocked ${kind} Timeline lane is required.`)}
      </p>
    </Show>
  );
  const renderMediaNoLaneStatus = (kind: "Video" | "Audio") => (
    <Show when={mediaShelfHasSourceForKind(kind) && sourceShelfLayerResolution(kind).mode === "none"}>
      {renderNoLaneStatus(kind)}
    </Show>
  );
  const renderLaneChoiceDisclosure = (...kinds: TimelineLayerKind[]) => (
    <Show when={kinds.some((kind) => {
      const resolution = sourceShelfLayerResolution(kind);
      return resolution.candidates.length > 1 || resolution.mode === "stale";
    })}>
      <details
        class="timelineExternalSourcePlacementDisclosure"
        data-timeline-source-click-placement={kinds.join(",")}
        aria-label={localizedSourceText("Timeline click controls")}
      >
        <summary>{localizedSourceText("Timeline click controls")}</summary>
        <div class="timelineExternalSourceTargets" role="group" aria-label={localizedSourceText("Timeline click controls")}>
          <For each={kinds}>{(kind) => renderTargetSelect(kind, localizedSourceText(`${kind} lane`))}</For>
        </div>
      </details>
    </Show>
  );

  return (
    <section class="timelineExternalSourceShelf" aria-label="Timeline source shelf" data-timeline-source-shelf>
      <header class="timelineExternalSourceShelfHeader">
        <div>
          <h3>Sources</h3>
          <p>Drag a Scene, Video, or Audio source onto its matching lane.</p>
        </div>
        <div class="timelineExternalSourceShelfModes" role="tablist" aria-label="Timeline source or inspector">
          <button type="button" id="timeline-source-context-tab-sources" role="tab" data-timeline-source-shelf-mode="sources" aria-selected={sourceContextMode() === "sources"} aria-controls={sourceShelfTab() === "Scenes" ? "timeline-source-context-panel-scenes" : "timeline-source-context-panel-media"} tabindex={sourceContextMode() === "sources" ? 0 : -1} classList={{ active: sourceContextMode() === "sources" }} onClick={() => selectSourceContextMode("sources")} onKeyDown={(event) => {
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
            onClick={() => selectSourceShelfTab("Scenes")}
          >Scenes</button>
          <button
            type="button"
            data-timeline-source-shelf-category="media"
            aria-pressed={sourceShelfTab() === "Media Library"}
            classList={{ active: sourceShelfTab() === "Media Library" }}
            onClick={() => selectSourceShelfTab("Media Library")}
          >Media Library</button>
        </div>
      </header>
      <Show when={sourceContextMode() === "sources" && sourceShelfTab() === "Scenes"}>
        <div id="timeline-source-context-panel-scenes" role="tabpanel" aria-labelledby="timeline-source-context-tab-sources" class="timelineExternalSourceShelfBody" aria-label="Scenes">
          <Show
            when={sceneAuthorityUnavailable() === null}
            fallback={
              <p
                class="emptyState"
                role="alert"
                data-bank-authority-unavailable={bankAuthority().issue?.kind ?? "cue_option_mismatch"}
              >
                {localizedSourceText(sceneAuthorityUnavailable() ?? "")}
              </p>
            }
          >
          {renderNoLaneStatus("Lighting")}
          {renderLaneChoiceDisclosure("Lighting")}
          <Show when={shelfBanks().length > 0} fallback={<p class="emptyState">No Scenes are available yet.</p>}>
            <div class="timelineExternalSourceShelfBanks" data-timeline-source-shelf-banks>
              <For each={shelfBanks()}>
                {(bank) => (
                  <section
                    class="timelineExternalSourceShelfBank"
                    style={{
                      "--bank-identity": groupIdentityCss(timelineSourceShelfBankIdentity(bank.cue_list_id), undefined, "fill"),
                      "--bank-identity-text": groupIdentityCss(timelineSourceShelfBankIdentity(bank.cue_list_id), undefined, "text"),
                    }}
                    data-timeline-source-shelf-bank={bank.key}
                    data-timeline-source-shelf-bank-id={bank.cue_list_id}
                    data-timeline-source-shelf-bank-hue={groupIdentityHue(timelineSourceShelfBankIdentity(bank.cue_list_id))}
                    aria-label={localizedSourceText(`Timeline Bank ${bank.label}`)}
                  >
                    <header class="timelineExternalSourceShelfBankHeader">
                      <i class="timelineExternalSourceShelfBankStrip" aria-hidden="true" />
                      <div class="timelineExternalSourceShelfBankTitle">
                        <strong data-no-localize>
                          {bank.label}
                        </strong>
                        <span>{bank.scenes.length}</span>
                      </div>
                    </header>
                    <Show
                      when={bank.scenes.length > 0}
                      fallback={<p class="emptyState timelineExternalSourceBankEmpty">{localizedSourceText("No Scenes in this Bank yet.")}</p>}
                    >
                      <div class="timelineExternalSourceShelfBankScenes">
                        <For each={bank.scenes}>
                          {(cue) => renderSceneSourceCard(cue)}
                        </For>
                      </div>
                    </Show>
                  </section>
                )}
              </For>
            </div>
          </Show>
          </Show>
        </div>
      </Show>
      <Show when={sourceContextMode() === "sources" && sourceShelfTab() === "Media Library"}>
        <div id="timeline-source-context-panel-media" role="tabpanel" aria-labelledby="timeline-source-context-tab-sources" class="timelineExternalSourceShelfBody" aria-label="Media Library">
          <div class="timelineExternalSourceShelfFilters" role="group" aria-label={localizedSourceText("Media source filters and verification")}>
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
            <button
              type="button"
              disabled={props.mediaAssets.length === 0}
              data-timeline-source-media-verify-all
              title={localizedSourceText("Verify machine-local Media Library availability before Timeline placement")}
              onClick={() => void props.onVerify(props.mediaAssets.map((asset) => asset.id))}
            >{localizedSourceText("Verify All")}</button>
          </div>
          {renderMediaNoLaneStatus("Video")}
          {renderMediaNoLaneStatus("Audio")}
          {renderLaneChoiceDisclosure(...mediaShelfRelevantKinds())}
          <Show when={mediaShelfAssets().length > 0} fallback={<p class="emptyState">No matching Media Library sources.</p>}>
            <div class="timelineExternalSourceShelfItems">
              <For each={mediaShelfAssets()}>
                {(asset) => {
                  const availability = () => props.mediaAssetAvailabilityById[asset.id];
                  const placementAllowed = () => mediaAssetAvailabilityAllowsTimelinePlacement(availability());
                  const availabilityIsError = () => {
                    const kind = availability()?.kind;
                    return kind === "missing" || kind === "hash_mismatch" || kind === "unreadable";
                  };
                  const placementTitle = (laneKind: "Video" | "Audio") => placementAllowed()
                    ? `Drag to a ${laneKind} Timeline lane${mediaTimelineSourceDescription(asset, laneKind)}; click to place at the playhead`
                    : `Verify ${asset.label} before placing it on the Timeline`;
                  return <article class="timelineExternalSourceCard media" data-timeline-source-media-id={asset.id}>
                    <div class="timelineExternalSourceCardTitle">
                      <strong data-no-localize>{asset.label}</strong>
                      <small
                        data-timeline-source-media-availability={availability()?.kind ?? "unverified"}
                        role={availabilityIsError() ? "alert" : "status"}
                      >{localizedSourceText(mediaAssetAvailabilityLabel(availability()))}</small>
                    </div>
                    <div class="timelineExternalSourceCardActions">
                      <button
                        type="button"
                        data-timeline-source-media-verify={asset.id}
                        title={localizedSourceText(`Verify ${asset.label} on this machine`)}
                        onClick={() => void props.onVerify([asset.id])}
                      >{localizedSourceText("Verify")}</button>
                      <Show when={mediaAssetHasVideo(asset)}>
                        <button
                          type="button"
                          draggable={placementAllowed()}
                          disabled={!placementAllowed()}
                          data-timeline-external-source="media_asset"
                          data-timeline-external-source-kind="Video"
                          data-timeline-source-media-kind="Video"
                          aria-label={localizedSourceText(`Drag media ${asset.label} to a Video lane${mediaTimelineSourceDescription(asset, "Video")}`)}
                          title={localizedSourceText(placementTitle("Video"))}
                          onDragStart={(event) => startSourceShelfDrag(event, mediaPayload(asset, "Video"))}
                          onClick={() => placeSourceShelfPayload(mediaPayload(asset, "Video"))}
                        >{mediaTimelineSourceLabel(asset, "Video")}</button>
                      </Show>
                      <Show when={mediaAssetHasAudio(asset)}>
                        <button
                          type="button"
                          draggable={placementAllowed()}
                          disabled={!placementAllowed()}
                          data-timeline-external-source="media_asset"
                          data-timeline-external-source-kind="Audio"
                          data-timeline-source-media-kind="Audio"
                          aria-label={localizedSourceText(`Drag media ${asset.label} to an Audio lane${mediaTimelineSourceDescription(asset, "Audio")}`)}
                          title={localizedSourceText(placementTitle("Audio"))}
                          onDragStart={(event) => startSourceShelfDrag(event, mediaPayload(asset, "Audio"))}
                          onClick={() => placeSourceShelfPayload(mediaPayload(asset, "Audio"))}
                        >{mediaTimelineSourceLabel(asset, "Audio")}</button>
                      </Show>
                      <Show when={!mediaAssetHasVideo(asset) && !mediaAssetHasAudio(asset)}>
                        <span class="timelineExternalDragUnavailable">Timeline source unavailable</span>
                      </Show>
                    </div>
                  </article>
                }}
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
