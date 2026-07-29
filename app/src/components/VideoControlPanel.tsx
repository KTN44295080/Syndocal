import { Show, createSignal, type ComponentProps, type JSX } from "solid-js";
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
  modeTabs?: JSX.Element;
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
}

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
  return (
    <section class={`panel videoControlPanel controlPanel ${props.mixer ? "videoControlPanelMixer" : ""}`}>
      <div class="panelHeader">
        <h2>Video Control</h2>
        <span>{props.layerCount} layer(s)</span>
        {props.modeTabs}
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
          <span>LIVE TAKE</span>
        </header>
        <VideoMasterControlsPanel {...props.masterControls} />
        <Show when={props.mixer}>
          <MixerDrawerBar id="audio-in" title="Audio In" status={audioInStatus()} open={audioInOpen()} onToggle={() => toggleDrawer("audio-in")} />
          <LiveAudioInputRail {...props.clipGrid} compact />
          <MixerDrawerBar id="auto-vj" title="Auto VJ" status={autoVjStatus()} open={autoVjOpen()} onToggle={() => toggleDrawer("auto-vj")} />
          <AutoVjStrip {...props.autoVj} />
          <MixerDrawerBar id="reactive" title="Reactive" status={reactiveStatus()} open={reactiveOpen()} onToggle={() => toggleDrawer("reactive")} />
          <AudioReactiveVjStrip {...props.audioReactive} />
        </Show>
        <VideoClipGridPanel {...props.clipGrid} compact={props.mixer} />
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
      <div class="videoMixerSetupTools">
        <VideoSourceCreatePanel {...props.sourceCreate} />
      </div>
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
