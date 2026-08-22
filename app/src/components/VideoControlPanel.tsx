import { For, Show, createEffect, createMemo, createSignal, onCleanup, type ComponentProps } from "solid-js";
import type {
  MediaAssetActiveOperation,
  MediaAssetAvailability,
  MediaAssetId,
  MediaAssetImportReport,
  MediaAssetPreviewSessionTicket,
  MediaAssetSummary,
  CompositionSummary,
  VideoEffectCatalog,
  VideoEffectPresetId,
  VideoEffectScope,
  VideoLayerTransitionRuntimeSnapshot,
  VideoOutputSummary,
} from "../types";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import { VideoMasterControlsPanel, VideoOutputControlListPanel } from "./VideoControlOutputsPanel";
import { VideoPreviewDiagnosticsPanel } from "./VideoPreviewDiagnosticsPanel";
import {
  ExternalVideoIoStatusPanel,
  VideoBackendStatusPanel,
  VideoOutputRenderPlanStatusPanel,
} from "./VideoRuntimeStatusPanels";
import { VideoSourceCreatePanel } from "./VideoSourceCreatePanel";
import { VideoTimelineAutomationPanel } from "./VideoTimelineAutomationPanel";
import { VideoClipGridPanel } from "./VideoClipGridPanel";
import { VideoClipSlotBankPanel } from "./VideoClipSlotBankPanel";
import { LiveVideoMonitorPanel } from "./LiveVideoMonitorPanel";
import { LiveAudioInputRail } from "./LiveAudioInputRail";
import { AutoVjStrip } from "./AutoVjStrip";
import { AudioReactiveVjStrip } from "./AudioReactiveVjStrip";
import { MixerDrawerBar, loadMixerDrawerOpen, saveMixerDrawerOpen, type MixerDrawerId } from "./MixerDrawerBar";
import { VideoEffectScopePanel } from "./VideoEffectScopePanel";
import { VideoEffectCatalogSetupPanel } from "./VideoEffectCatalogSetupPanel";
import { VideoTransitionBusPanel } from "./VideoTransitionBusPanel";

interface VideoControlPanelProps {
  mixer: boolean;
  libraryOnly?: boolean;
  selectedMediaAssetId?: MediaAssetId | null;
  onSelectMediaAsset?: (assetId: MediaAssetId) => void;
  layerCount: number;
  previewDiagnostics: ComponentProps<typeof VideoPreviewDiagnosticsPanel>;
  renderPlanStatus: ComponentProps<typeof VideoOutputRenderPlanStatusPanel>;
  backendStatus: ComponentProps<typeof VideoBackendStatusPanel>;
  externalIoStatus: ComponentProps<typeof ExternalVideoIoStatusPanel>;
  masterControls: ComponentProps<typeof VideoMasterControlsPanel>;
  outputControls: ComponentProps<typeof VideoOutputControlListPanel>;
  liveMonitors: ComponentProps<typeof LiveVideoMonitorPanel>;
  sourceCreate: ComponentProps<typeof VideoSourceCreatePanel>;
  clipGrid: ComponentProps<typeof VideoClipGridPanel>;
  clipSlotBank: ComponentProps<typeof VideoClipSlotBankPanel>;
  clipSlotTake: { enabled: boolean; label: string; onTake: () => void | Promise<void> };
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  effectScopes: {
    catalog: VideoEffectCatalog;
    compositions: CompositionSummary[];
    outputs: VideoOutputSummary[];
    transitionRuntime: VideoLayerTransitionRuntimeSnapshot;
    busy?: boolean;
    onApplyCatalog: (catalog: VideoEffectCatalog) => void | Promise<unknown>;
    onImportIsf: (scope: VideoEffectScope) => void | Promise<unknown>;
    onApplyPreset: (scope: VideoEffectScope, presetId: VideoEffectPresetId) => void | Promise<unknown>;
    onRemoveChain: (scope: VideoEffectScope) => void | Promise<unknown>;
    onLaunchTransition: ComponentProps<typeof VideoTransitionBusPanel>["onLaunch"];
    onReleaseTransition: ComponentProps<typeof VideoTransitionBusPanel>["onRelease"];
  };
  timelineAutomation: ComponentProps<typeof VideoTimelineAutomationPanel>;
  autoVj: ComponentProps<typeof AutoVjStrip>;
  audioReactive: ComponentProps<typeof AudioReactiveVjStrip>;
  mediaLibrary: {
    assets: MediaAssetSummary[];
    thumbnails: Record<number, string>;
    availabilityById: Record<number, MediaAssetAvailability>;
    activeOperations: MediaAssetActiveOperation[];
    lastImportReport: MediaAssetImportReport | null;
    backendAvailable: boolean;
    onVerify: (assetIds: MediaAssetId[]) => void | Promise<void>;
    onRelink: (assetId: MediaAssetId) => void | Promise<void>;
    onPreviewStart: (assetId: MediaAssetId) => Promise<MediaAssetPreviewSessionTicket>;
    onPreviewFrame: (ticket: MediaAssetPreviewSessionTicket, positionMs: number) => Promise<string>;
    onPreviewEnd: (ticket: MediaAssetPreviewSessionTicket) => Promise<void>;
    onCancelOperation: (operationId: number) => void;
  };
}

const mediaOperationPhaseLabel = (operation: MediaAssetActiveOperation) => {
  switch (operation.phase) {
    case "picker": return "Choosing file";
    case "preparing": return "Preparing";
    case "hashing": return "Hashing / probing";
    case "finalizing": return "Finalizing";
    case "committing": return "Committing";
    case "cancelling": return "Cancelling";
  }
};

const mediaAvailabilityLabel = (availability: MediaAssetAvailability | undefined) => {
  switch (availability?.kind) {
    case "available_verified": return "Verified";
    case "available_unverified": return "Available · not hash-verified";
    case "missing": return "Missing";
    case "hash_mismatch": return "Hash mismatch";
    case "unreadable": return "Unreadable";
    case "live_source": return "Live source";
    default: return "Not checked";
  }
};

const mediaAvailabilityDetail = (availability: MediaAssetAvailability | undefined) => {
  if (availability?.kind === "hash_mismatch") {
    return `Expected ${availability.expected.hex}; found ${availability.actual.hex}`;
  }
  if (availability?.kind === "unreadable") return availability.error;
  return mediaAvailabilityLabel(availability);
};

const mediaSourceLabel = (asset: MediaAssetSummary) =>
  asset.source.path ?? asset.source.name ?? asset.source.kind;

const mediaPreviewIdentity = (asset: MediaAssetSummary) =>
  `${asset.id}\u0000${asset.source.kind}\u0000${asset.source.path ?? ""}\u0000${asset.source.name ?? ""}\u0000${asset.content_hash?.algorithm ?? ""}\u0000${asset.content_hash?.hex ?? ""}\u0000${asset.byte_size ?? ""}`;

export function VideoControlPanel(props: VideoControlPanelProps) {
  // T6: Audio/AutoVJ/Reactive settings live behind collapsed drawers so the clip
  // bank dominates the pane. Open state persists per drawer in localStorage.
  const [audioInOpen, setAudioInOpen] = createSignal(loadMixerDrawerOpen("audio-in"));
  const [autoVjOpen, setAutoVjOpen] = createSignal(loadMixerDrawerOpen("auto-vj"));
  const [reactiveOpen, setReactiveOpen] = createSignal(loadMixerDrawerOpen("reactive"));
  const toggleDrawer = (id: MixerDrawerId) => {
    const [open, setOpen] =
      id === "audio-in" ? [audioInOpen, setAudioInOpen]
      : id === "auto-vj" ? [autoVjOpen, setAutoVjOpen]
      : [reactiveOpen, setReactiveOpen];
    const next = !open();
    setOpen(next);
    saveMixerDrawerOpen(id, next);
  };
  const audioInStatus = () => (props.clipGrid.liveAudioInputStatus.running ? "LIVE" : "OFF");
  const autoVjStatus = () => props.autoVj.snapshot.status.mode.toUpperCase();
  const reactiveStatus = () => {
    const audioGraphs = props.audioReactive.graphs.filter((graph) =>
      graph.nodes.some((node) => node.kind === "Audio"));
    const enabled = audioGraphs.filter((graph) => graph.enabled);
    if (audioGraphs.length === 0) return "NONE";
    if (enabled.length === 0) return `0/${audioGraphs.length} OFF`;
    return `${enabled.length}/${audioGraphs.length} ON`;
  };
  const firstRunGuarded = () =>
    props.clipGrid.layers.length === 0 &&
    (props.clipGrid.firstRunBusy || props.clipGrid.firstRunAvailable || Boolean(props.clipGrid.firstRunError));
  const sourceCreateVisible = () =>
    !props.mixer && !firstRunGuarded() && (props.clipGrid.layers.length > 0 || !props.clipGrid.firstRunAvailable);
  const importIssues = createMemo(() => props.mediaLibrary.lastImportReport?.entries.filter(
    (entry) => entry.status === "failed" || entry.status === "skipped",
  ) ?? []);
  const [previewAssetId, setPreviewAssetId] = createSignal<MediaAssetId | null>(null);
  const [previewAssetIdentity, setPreviewAssetIdentity] = createSignal<string | null>(null);
  const [previewFrameUrl, setPreviewFrameUrl] = createSignal("");
  let previewGeneration = 0;
  let previewTimer: number | undefined;
  let previewPositionMs = 0;
  let previewSession: MediaAssetPreviewSessionTicket | null = null;
  let previewSerial: Promise<void> = Promise.resolve();
  const previewMotionAllowed = () => !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const enqueuePreview = <T,>(action: () => Promise<T>) => {
    const result = previewSerial.then(action);
    previewSerial = result.then(() => undefined, () => undefined);
    return result;
  };
  const stopMediaLibraryPreviewNow = () => {
    previewGeneration += 1;
    if (previewTimer !== undefined) window.clearTimeout(previewTimer);
    previewTimer = undefined;
    previewPositionMs = 0;
    const session = previewSession;
    previewSession = null;
    if (session) void enqueuePreview(() => props.mediaLibrary.onPreviewEnd(session));
    setPreviewAssetId(null);
    setPreviewAssetIdentity(null);
    setPreviewFrameUrl("");
  };
  const startMediaLibraryPreview = (asset: MediaAssetSummary, hasStillThumbnail: boolean) => {
    if (!props.clipGrid.thumbnailsAuthorized || !hasStillThumbnail || asset.source.kind !== "File" || !previewMotionAllowed()) return;
    const identity = mediaPreviewIdentity(asset);
    if (previewAssetId() === asset.id && previewAssetIdentity() === identity) return;
    stopMediaLibraryPreviewNow();
    const generation = ++previewGeneration;
    // One exact-identity backend frame stream means hover/focus cannot create
    // multiple readers or audio paths. The source file is never opened by DOM.
    setPreviewAssetIdentity(mediaPreviewIdentity(asset));
    setPreviewAssetId(asset.id);
    void enqueuePreview(() => props.mediaLibrary.onPreviewStart(asset.id)).then((ticket) => {
      if (generation !== previewGeneration) {
        void enqueuePreview(() => props.mediaLibrary.onPreviewEnd(ticket));
        return;
      }
      previewSession = ticket;
      const tick = () => {
        if (generation !== previewGeneration || previewSession !== ticket) return;
        const positionMs = previewPositionMs;
        const durationMs = Math.max(1, asset.source.metadata?.duration_ms ?? 5_000);
        previewPositionMs = (previewPositionMs + 250) % durationMs;
        void enqueuePreview(() => props.mediaLibrary.onPreviewFrame(ticket, positionMs)).then((frameUrl) => {
          if (generation === previewGeneration && previewSession === ticket && previewAssetId() === asset.id && previewAssetIdentity() === identity) {
            setPreviewFrameUrl(frameUrl);
            previewTimer = window.setTimeout(tick, 250);
          }
        }).catch(() => {
          if (generation === previewGeneration) stopMediaLibraryPreviewNow();
        });
      };
      tick();
    }).catch(() => {
      if (generation === previewGeneration) stopMediaLibraryPreviewNow();
    });
  };
  const stopMediaLibraryPreview = (assetId: MediaAssetId, card: HTMLElement) => {
    window.requestAnimationFrame(() => {
      if (!card.matches(":hover") && !card.contains(document.activeElement) && previewAssetId() === assetId) {
        stopMediaLibraryPreviewNow();
      }
    });
  };
  createEffect(() => {
    const activeId = previewAssetId();
    if (activeId === null) return;
    const activeAsset = props.mediaLibrary.assets.find((asset) => asset.id === activeId);
    if (!props.clipGrid.thumbnailsAuthorized || !activeAsset || mediaPreviewIdentity(activeAsset) !== previewAssetIdentity()) {
      stopMediaLibraryPreviewNow();
    }
  });
  onCleanup(stopMediaLibraryPreviewNow);
  return (
    <section
      id={props.libraryOnly ? "edit-domain-panel-mixer" : undefined}
      role={props.libraryOnly ? "tabpanel" : undefined}
      aria-labelledby={props.libraryOnly ? "edit-domain-tab-mixer" : undefined}
      class={`panel videoControlPanel controlPanel ${props.mixer ? "videoControlPanelMixer" : ""}${props.libraryOnly ? " videoControlPanelLibrary" : ""}`}
      data-workspace-pane={props.libraryOnly ? "upper" : undefined}
      data-video-media-library={props.libraryOnly ? "true" : undefined}
    >
      <div class="panelHeader">
        <h2>{props.libraryOnly ? "Media Library" : "Video Control"}</h2>
        <span>{props.libraryOnly ? `${props.mediaLibrary.assets.length} asset(s)` : `${props.layerCount} layer(s)`}</span>
        <Show when={props.libraryOnly}>
          <details class="videoMixerSourceDisclosure" data-edit-video-import-disclosure>
            <summary aria-label="Import Media">Import Media</summary>
            <div class="videoMixerSourceCreateSurface" data-edit-video-import-surface>
              <VideoSourceCreatePanel {...props.sourceCreate} />
            </div>
          </details>
        </Show>
      </div>
      <div class="videoMixerDiagnostics">
        <VideoPreviewDiagnosticsPanel {...props.previewDiagnostics} />
        <VideoOutputRenderPlanStatusPanel {...props.renderPlanStatus} />
        <VideoBackendStatusPanel {...props.backendStatus} />
        <ExternalVideoIoStatusPanel {...props.externalIoStatus} />
      </div>
      <section class="videoMixerTopPane" aria-label="Preview and Program">
        <header class="videoMixerPaneHeader">
          <div>
            <strong>Preview / Program</strong>
            <span>Staged clip and routed output</span>
          </div>
          <span>LIVE MONITORS</span>
        </header>
        <div class="videoMixerTopContent">
          <LiveVideoMonitorPanel {...props.liveMonitors} />
          <VideoMasterControlsPanel {...props.masterControls} />
        </div>
      </section>
      <div class="videoMixerGridDivider" aria-hidden="true" />
      <section class="videoMixerClipPane" aria-label="Clip and transition desk">
        <header class="videoMixerPaneHeader">
          <div>
            <strong>Clips</strong>
            <span>{props.layerCount} available</span>
          </div>
          <div class="videoMixerPaneActions">
            <Show when={props.mixer && props.clipGrid.layers.length > 0}>
              <details class="videoMixerSourceDisclosure" data-vj-media-import-disclosure>
                <summary aria-label="Import Media">Import Media</summary>
                <div class="videoMixerSourceCreateSurface" data-vj-media-source-surface="mixer">
                  <VideoSourceCreatePanel {...props.sourceCreate} />
                </div>
              </details>
            </Show>
            <button
              type="button"
              class="primary videoClipSlotTake"
              data-video-clip-slot-take
              disabled={!props.clipSlotTake.enabled}
              onClick={() => void props.clipSlotTake.onTake()}
            >{props.clipSlotTake.label}</button>
          </div>
        </header>
        <Show when={props.mediaLibrary.activeOperations.length > 0}>
          <div class="videoMediaOperationRail" role="group" aria-label="Active Media operations">
            <For each={props.mediaLibrary.activeOperations}>
              {(operation) => (
                <div data-media-operation-phase={operation.phase}>
                  <span role="status" aria-live="polite">
                    <strong>{operation.label}</strong> · {mediaOperationPhaseLabel(operation)}
                  </span>
                  <button
                    class="danger"
                    data-media-operation-cancel={operation.id}
                    aria-label={`Cancel ${operation.label}`}
                    disabled={operation.phase === "cancelling"}
                    onClick={() => props.mediaLibrary.onCancelOperation(operation.id)}
                  >
                    {operation.phase === "cancelling" ? "Cancelling…" : "Cancel"}
                  </button>
                </div>
              )}
            </For>
          </div>
        </Show>
        <details class="videoMediaLibraryRail" data-media-library-rail open={props.libraryOnly}>
          <summary>
            <strong>Media Library</strong>
            <span>{props.mediaLibrary.assets.length} asset(s)</span>
          </summary>
          <div class="videoMediaLibrarySurface">
            <div class="videoMediaLibraryActions">
              <span>Availability is machine-local and never saved in the project.</span>
              <Show when={props.mediaLibrary.assets.length > 0 && !props.clipGrid.thumbnailsAuthorized}>
                <button
                  data-media-library-thumbnail-request
                  title="Read local media only after this explicit request"
                  onClick={props.clipGrid.onRequestThumbnails}
                >
                  Load Thumbnails
                </button>
              </Show>
              <button
                disabled={!props.mediaLibrary.backendAvailable || props.mediaLibrary.assets.length === 0}
                onClick={() => void props.mediaLibrary.onVerify(props.mediaLibrary.assets.map((asset) => asset.id))}
              >
                Verify All
              </button>
            </div>
            <Show
              when={props.mediaLibrary.assets.length > 0}
              fallback={<div class="emptyState">No Media Library assets. Import files to build the catalog.</div>}
            >
              <div class="videoMediaLibraryList">
                <For each={props.mediaLibrary.assets}>
                  {(asset) => {
                    const availability = () => props.mediaLibrary.availabilityById[asset.id];
                    const referenceCount = () => props.clipGrid.layers.reduce(
                      (count, layer) => count
                        + (layer.media_asset_id === asset.id ? 1 : 0)
                        + (layer.clip_slots?.filter((slot) => slot.media_asset_id === asset.id).length ?? 0),
                      0,
                    );
                    const thumbnail = () => props.mediaLibrary.thumbnails[asset.id];
                    const relinkable = () => asset.source.kind === "File" || asset.source.kind === "StillImage";
                    return (
                      <article
                        class={`videoMediaLibraryItem availability-${availability()?.kind ?? "unknown"}${props.selectedMediaAssetId === asset.id ? " selected" : ""}`}
                        tabindex="0"
                        aria-label={`${asset.label}. ${mediaAvailabilityLabel(availability())}. ${referenceCount()} layer and Clip Slot reference(s).`}
                        aria-describedby={`media-library-source-${asset.id}`}
                        onMouseEnter={() => startMediaLibraryPreview(asset, Boolean(thumbnail()))}
                        onMouseLeave={(event) => stopMediaLibraryPreview(asset.id, event.currentTarget)}
                        onFocusIn={() => startMediaLibraryPreview(asset, Boolean(thumbnail()))}
                        onFocusOut={(event) => stopMediaLibraryPreview(asset.id, event.currentTarget)}
                        onClick={() => props.onSelectMediaAsset?.(asset.id)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault();
                            props.onSelectMediaAsset?.(asset.id);
                          }
                        }}
                      >
                        <div class="videoMediaLibraryThumbnail" aria-hidden="true">
                          <Show
                            when={props.clipGrid.thumbnailsAuthorized && thumbnail()}
                            fallback={
                              <span
                                class="videoMediaLibraryThumbnailPlaceholder"
                                data-media-library-thumbnail={props.clipGrid.thumbnailsAuthorized ? "unavailable" : "placeholder"}
                              />
                            }
                          >
                            <img
                              data-media-library-thumbnail="loaded"
                              src={thumbnail()}
                              alt=""
                            />
                          </Show>
                          <Show when={previewAssetId() === asset.id && previewFrameUrl()}>
                            <img
                              class="videoMediaLibraryPreview"
                              data-media-library-preview="playing"
                              src={previewFrameUrl()}
                              alt=""
                            />
                          </Show>
                          <div class="videoMediaLibraryCardDetails">
                            <strong data-no-localize>{asset.label}</strong>
                            <span data-no-localize>{mediaSourceLabel(asset)}</span>
                          </div>
                        </div>
                        <div class="videoMediaLibraryCardHeading">
                          <span id={`media-library-source-${asset.id}`} class="srOnly" data-no-localize>{mediaSourceLabel(asset)}</span>
                          <small>{referenceCount()} layer and Clip Slot reference(s) · {asset.source.kind}</small>
                        </div>
                        <div class="videoMediaLibraryCardActions">
                          <span
                            class="videoMediaAvailability"
                            role={availability()?.kind === "missing" || availability()?.kind === "hash_mismatch" || availability()?.kind === "unreadable" ? "alert" : "status"}
                            title={mediaAvailabilityDetail(availability())}
                          >
                            {mediaAvailabilityLabel(availability())}
                          </span>
                          <button
                            disabled={!props.mediaLibrary.backendAvailable}
                            onClick={() => void props.mediaLibrary.onVerify([asset.id])}
                          >
                            Verify
                          </button>
                          <button
                            disabled={!props.mediaLibrary.backendAvailable || !relinkable()}
                            title={relinkable() ? "Choose a replacement file" : "Live sources cannot be relinked to a file"}
                            onClick={() => void props.mediaLibrary.onRelink(asset.id)}
                          >
                            Relink…
                          </button>
                        </div>
                      </article>
                    );
                  }}
                </For>
              </div>
            </Show>
            <Show when={importIssues().length > 0}>
              <details class="videoMediaImportIssues" open>
                <summary>{importIssues().length} import issue(s)</summary>
                <ul aria-label="Media import failure details">
                  <For each={importIssues()}>
                    {(entry) => (
                      <li>
                        <strong>{entry.status}</strong>
                        <span data-no-localize>{entry.path}</span>
                        <Show when={entry.message}><small data-no-localize>{entry.message}</small></Show>
                      </li>
                    )}
                  </For>
                </ul>
              </details>
            </Show>
          </div>
        </details>
        <Show when={props.mixer}>
          <MixerDrawerBar id="audio-in" title="Audio In" status={audioInStatus()} open={audioInOpen()} onToggle={() => toggleDrawer("audio-in")} />
          <LiveAudioInputRail {...props.clipGrid} compact />
          <MixerDrawerBar id="auto-vj" title="Auto VJ" status={autoVjStatus()} open={autoVjOpen()} onToggle={() => toggleDrawer("auto-vj")} />
          <AutoVjStrip {...props.autoVj} />
          <MixerDrawerBar id="reactive" title="Reactive" status={reactiveStatus()} open={reactiveOpen()} onToggle={() => toggleDrawer("reactive")} />
          <AudioReactiveVjStrip {...props.audioReactive} />
        </Show>
        <VideoClipSlotBankPanel {...props.clipSlotBank} />
        <details class="videoClipLegacyTransport" open={!props.mixer}>
          <summary>Layer transport &amp; capture</summary>
          <VideoClipGridPanel
            {...props.clipGrid}
            compact={props.mixer}
            sourceCreateVisible={sourceCreateVisible()}
          />
        </details>
      </section>
      <section class="videoMixerContextPane" aria-label="Layers and outputs">
        <section class="videoMixerProgramPane" aria-label="Video outputs">
          <header class="videoMixerPaneHeader">
            <div>
              <strong>Outputs</strong>
              <span>Program routing and windows</span>
            </div>
            <span>OUTPUTS</span>
          </header>
          <VideoOutputControlListPanel {...props.outputControls} compact={props.mixer} />
        </section>
        <section class="videoMixerLayerPane" aria-label="Live video layers">
          <header class="videoMixerPaneHeader">
            <div>
              <strong>Layers</strong>
              <span>{props.layerCount} in composition</span>
            </div>
            <span>COMPOSITE</span>
          </header>
          <VideoLayerListPanel {...props.layerList} compact={props.mixer} />
          <details class="videoEffectScopeCatalog">
            <summary>
              <strong>Scoped Video FX</strong>
              <span>Composition, Group, and Output chains</span>
            </summary>
            <div class="videoEffectScopeCatalogBody">
              <VideoEffectCatalogSetupPanel
                catalog={props.effectScopes.catalog}
                compositions={props.effectScopes.compositions}
                busy={props.effectScopes.busy}
                onApply={props.effectScopes.onApplyCatalog}
              />
              <VideoTransitionBusPanel
                catalog={props.effectScopes.catalog}
                compositions={props.effectScopes.compositions}
                runtime={props.effectScopes.transitionRuntime}
                busy={props.effectScopes.busy}
                onApplyCatalog={props.effectScopes.onApplyCatalog}
                onLaunch={props.effectScopes.onLaunchTransition}
                onRelease={props.effectScopes.onReleaseTransition}
              />
              <For each={props.effectScopes.catalog.transition_buses}>
                {(bus) => (
                  <VideoEffectScopePanel
                    scopeLabel="Transition Bus FX"
                    title={bus.label}
                    scope={{ scope: "transition", owner: { kind: "layer_bus", bus_id: bus.id } }}
                    catalog={props.effectScopes.catalog}
                    busy={props.effectScopes.busy}
                    onApplyCatalog={props.effectScopes.onApplyCatalog}
                    onImportIsf={props.effectScopes.onImportIsf}
                    onApplyPreset={props.effectScopes.onApplyPreset}
                    onRemoveChain={props.effectScopes.onRemoveChain}
                  />
                )}
              </For>
              <For each={props.effectScopes.compositions}>
                {(composition) => (
                  <VideoEffectScopePanel
                    scopeLabel="Composition FX"
                    title={composition.label}
                    scope={{ scope: "composition", composition_id: composition.id }}
                    catalog={props.effectScopes.catalog}
                    busy={props.effectScopes.busy}
                    onApplyCatalog={props.effectScopes.onApplyCatalog}
                    onImportIsf={props.effectScopes.onImportIsf}
                    onApplyPreset={props.effectScopes.onApplyPreset}
                    onRemoveChain={props.effectScopes.onRemoveChain}
                  />
                )}
              </For>
              <For each={props.effectScopes.catalog.layer_groups}>
                {(group) => (
                  <VideoEffectScopePanel
                    scopeLabel="Group FX"
                    title={group.label}
                    scope={{ scope: "group", group_id: group.id }}
                    catalog={props.effectScopes.catalog}
                    busy={props.effectScopes.busy}
                    onApplyCatalog={props.effectScopes.onApplyCatalog}
                    onImportIsf={props.effectScopes.onImportIsf}
                    onApplyPreset={props.effectScopes.onApplyPreset}
                    onRemoveChain={props.effectScopes.onRemoveChain}
                  />
                )}
              </For>
              <For each={props.effectScopes.outputs}>
                {(output) => (
                  <VideoEffectScopePanel
                    scopeLabel="Output FX"
                    title={output.label}
                    scope={{ scope: "output", output_id: output.id }}
                    catalog={props.effectScopes.catalog}
                    busy={props.effectScopes.busy}
                    onApplyCatalog={props.effectScopes.onApplyCatalog}
                    onImportIsf={props.effectScopes.onImportIsf}
                    onApplyPreset={props.effectScopes.onApplyPreset}
                    onRemoveChain={props.effectScopes.onRemoveChain}
                  />
                )}
              </For>
            </div>
          </details>
        </section>
      </section>
      <Show when={!props.mixer && sourceCreateVisible()}>
        <div class="videoMixerSetupTools" data-vj-media-source-surface="normal">
          <VideoSourceCreatePanel {...props.sourceCreate} />
        </div>
      </Show>
      <div class="videoMixerAutomationTools">
        <VideoTimelineAutomationPanel {...props.timelineAutomation} />
      </div>
    </section>
  );
}
