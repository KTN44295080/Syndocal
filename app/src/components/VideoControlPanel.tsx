import type { ComponentProps } from "solid-js";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import { VideoMasterControlsPanel, VideoOutputControlListPanel } from "./VideoControlOutputsPanel";
import { VideoPreviewDiagnosticsPanel } from "./VideoPreviewDiagnosticsPanel";
import {
  ExternalVideoIoStatusPanel,
  VideoBackendStatusPanel,
  VideoOutputRenderPlanStatusPanel,
} from "./VideoRuntimeStatusPanels";
import { VideoPreviewImagePanel, VideoSourceCreatePanel } from "./VideoSourceCreatePanel";
import { VideoTimelineAutomationPanel } from "./VideoTimelineAutomationPanel";
import { VideoClipGridPanel } from "./VideoClipGridPanel";

interface VideoControlPanelProps {
  mixer: boolean;
  layerCount: number;
  previewDiagnostics: ComponentProps<typeof VideoPreviewDiagnosticsPanel>;
  renderPlanStatus: ComponentProps<typeof VideoOutputRenderPlanStatusPanel>;
  backendStatus: ComponentProps<typeof VideoBackendStatusPanel>;
  externalIoStatus: ComponentProps<typeof ExternalVideoIoStatusPanel>;
  masterControls: ComponentProps<typeof VideoMasterControlsPanel>;
  outputControls: ComponentProps<typeof VideoOutputControlListPanel>;
  previewImage: ComponentProps<typeof VideoPreviewImagePanel>;
  sourceCreate: ComponentProps<typeof VideoSourceCreatePanel>;
  clipGrid: ComponentProps<typeof VideoClipGridPanel>;
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  timelineAutomation: ComponentProps<typeof VideoTimelineAutomationPanel>;
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
        <VideoClipGridPanel {...props.clipGrid} compact={props.mixer} />
      </section>
      <section class="videoMixerProgramPane" aria-label="Program monitor and outputs">
        <header class="videoMixerPaneHeader">
          <div>
            <strong>Program Monitor</strong>
            <span>Reference preview</span>
          </div>
          <div class="videoMixerPaneActions">
            <span>PROGRAM</span>
            <button
              disabled={props.layerCount === 0}
              onClick={() => void props.previewDiagnostics.onRenderPreview()}
            >
              Refresh
            </button>
          </div>
        </header>
        <VideoPreviewImagePanel {...props.previewImage} />
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
