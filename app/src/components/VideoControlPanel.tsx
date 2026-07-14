import { Show, type ComponentProps } from "solid-js";
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
}

export function VideoControlPanel(props: VideoControlPanelProps) {
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
          <span>LIVE TAKE</span>
        </header>
        <VideoMasterControlsPanel {...props.masterControls} />
        <Show when={props.mixer}>
          <LiveAudioInputRail {...props.clipGrid} compact />
          <AutoVjStrip {...props.autoVj} />
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
