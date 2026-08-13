import { For, Show, createMemo, createSignal, type ComponentProps } from "solid-js";
import type {
  MediaAssetActiveOperation,
  MediaAssetAvailability,
  MediaAssetId,
  MediaAssetImportReport,
  MediaAssetSummary,
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
import { LiveVideoMonitorPanel } from "./LiveVideoMonitorPanel";
import { LiveAudioInputRail } from "./LiveAudioInputRail";
import { AutoVjStrip } from "./AutoVjStrip";
import { AudioReactiveVjStrip } from "./AudioReactiveVjStrip";
import { MixerDrawerBar, loadMixerDrawerOpen, saveMixerDrawerOpen, type MixerDrawerId } from "./MixerDrawerBar";

interface VideoControlPanelProps {
  mixer: boolean;
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
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  timelineAutomation: ComponentProps<typeof VideoTimelineAutomationPanel>;
  autoVj: ComponentProps<typeof AutoVjStrip>;
  audioReactive: ComponentProps<typeof AudioReactiveVjStrip>;
  mediaLibrary: {
    assets: MediaAssetSummary[];
    availabilityById: Record<number, MediaAssetAvailability>;
    activeOperations: MediaAssetActiveOperation[];
    lastImportReport: MediaAssetImportReport | null;
    backendAvailable: boolean;
    onVerify: (assetIds: MediaAssetId[]) => void | Promise<void>;
    onRelink: (assetId: MediaAssetId) => void | Promise<void>;
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
  return (
    <section class={`panel videoControlPanel controlPanel ${props.mixer ? "videoControlPanelMixer" : ""}`}>
      <div class="panelHeader">
        <h2>Video Control</h2>
        <span>{props.layerCount} layer(s)</span>
      </div>
      <div class="videoMixerDiagnostics">
        <VideoPreviewDiagnosticsPanel {...props.previewDiagnostics} />
        <VideoOutputRenderPlanStatusPanel {...props.renderPlanStatus} />
        <VideoBackendStatusPanel {...props.backendStatus} />
        <ExternalVideoIoStatusPanel {...props.externalIoStatus} />
      </div>
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
            <span>LIVE TAKE</span>
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
        <details class="videoMediaLibraryRail" data-media-library-rail>
          <summary>
            <strong>Media Library</strong>
            <span>{props.mediaLibrary.assets.length} asset(s)</span>
          </summary>
          <div class="videoMediaLibrarySurface">
            <div class="videoMediaLibraryActions">
              <span>Availability is machine-local and never saved in the project.</span>
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
                    const referenceCount = () => props.clipGrid.layers.filter((layer) => layer.media_asset_id === asset.id).length;
                    const relinkable = () => asset.source.kind === "File" || asset.source.kind === "StillImage";
                    return (
                      <article class={`videoMediaLibraryItem availability-${availability()?.kind ?? "unknown"}`}>
                        <div>
                          <strong data-no-localize>{asset.label}</strong>
                          <span data-no-localize title={asset.source.path ?? asset.source.name ?? asset.source.kind}>
                            {asset.source.path ?? asset.source.name ?? asset.source.kind}
                          </span>
                          <small>{referenceCount()} layer reference(s) · {asset.source.kind}</small>
                        </div>
                        <div>
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
        <VideoMasterControlsPanel {...props.masterControls} />
        <Show when={props.mixer}>
          <MixerDrawerBar id="audio-in" title="Audio In" status={audioInStatus()} open={audioInOpen()} onToggle={() => toggleDrawer("audio-in")} />
          <LiveAudioInputRail {...props.clipGrid} compact />
          <MixerDrawerBar id="auto-vj" title="Auto VJ" status={autoVjStatus()} open={autoVjOpen()} onToggle={() => toggleDrawer("auto-vj")} />
          <AutoVjStrip {...props.autoVj} />
          <MixerDrawerBar id="reactive" title="Reactive" status={reactiveStatus()} open={reactiveOpen()} onToggle={() => toggleDrawer("reactive")} />
          <AudioReactiveVjStrip {...props.audioReactive} />
        </Show>
        <VideoClipGridPanel
          {...props.clipGrid}
          compact={props.mixer}
          sourceCreateVisible={sourceCreateVisible()}
        />
      </section>
      <section class="videoMixerProgramPane" aria-label="Program monitor and outputs">
        <header class="videoMixerPaneHeader">
          <div>
            <strong>Live Monitors</strong>
            <span>Staged clip and routed output</span>
          </div>
          <span>PREVIEW / PROGRAM</span>
        </header>
        <LiveVideoMonitorPanel {...props.liveMonitors} />
        <VideoOutputControlListPanel {...props.outputControls} compact={props.mixer} />
      </section>
      <Show when={!props.mixer && sourceCreateVisible()}>
        <div class="videoMixerSetupTools" data-vj-media-source-surface="normal">
          <VideoSourceCreatePanel {...props.sourceCreate} />
        </div>
      </Show>
      <section class="videoMixerLayerPane" aria-label="Live video layers">
        <header class="videoMixerPaneHeader">
          <div>
            <strong>Layers</strong>
            <span>{props.layerCount} in composition</span>
          </div>
          <span>COMPOSITE</span>
        </header>
        <VideoLayerListPanel {...props.layerList} compact={props.mixer} />
      </section>
      <div class="videoMixerAutomationTools">
        <VideoTimelineAutomationPanel {...props.timelineAutomation} />
      </div>
    </section>
  );
}
