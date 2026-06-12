import { For, Show } from "solid-js";
import {
  controlModes,
  setupSubTabs,
} from "../uiModes";
import type { ControlMode, SetupSubTab, WorkspaceTab } from "../uiModes";

type HealthBadge = {
  className: string;
  label: string;
  detail: string;
  value: string;
};

type WorkspaceChromeProps = {
  workspaceTab: WorkspaceTab;
  setupSubTab: SetupSubTab;
  controlMode: ControlMode;
  blackout: boolean;
  videoBlackout: boolean;
  clockSource: string;
  beatTitle: string;
  bpm: number;
  beatPhasePercent: number;
  realtimeHealth: HealthBadge;
  dmxHealth: HealthBadge;
  packetBytes: number;
  currentProjectPath: string | null;
  onWorkspaceTab: (tab: WorkspaceTab) => void;
  onSetupSubTab: (tab: SetupSubTab) => void;
  onControlMode: (mode: ControlMode) => void;
  onNewProject: () => void;
  onSaveProject: () => void;
  onSaveProjectAs: () => void;
  onLoadProject: () => void;
};

export function WorkspaceChrome(props: WorkspaceChromeProps) {
  const liveLabel = () => {
    if (props.blackout && props.videoBlackout) {
      return "All Blackout";
    }
    if (props.blackout) {
      return "DMX Blackout";
    }
    if (props.videoBlackout) {
      return "Video Blackout";
    }
    return "Live";
  };
  const projectLabel = () => {
    const path = props.currentProjectPath;
    if (!path) {
      return "Untitled.ry";
    }
    const normalizedPath = path.replaceAll("\\", "/");
    return normalizedPath.split("/").pop() || path;
  };

  return (
    <>
      <header class="topbar">
        <div>
          <h1>Rayard</h1>
          <p>Unified lighting and video control: GDTF patch, cues, timeline, effects, DMX output. Seraf() KTN.</p>
        </div>
        <div class="status">
          <span class={props.blackout || props.videoBlackout ? "pill danger" : "pill ok"}>{liveLabel()}</span>
          <div class="beatClockPill" title={props.beatTitle}>
            <span>{props.clockSource}</span>
            <strong>{props.bpm.toFixed(1)} BPM</strong>
            <b aria-hidden="true">
              <i style={{ width: `${props.beatPhasePercent}%` }} />
            </b>
          </div>
          <span class={props.realtimeHealth.className} title={props.realtimeHealth.detail}>
            <b>{props.realtimeHealth.label}</b>
            <small>{props.realtimeHealth.value}</small>
          </span>
          <span class={props.dmxHealth.className} title={props.dmxHealth.detail}>
            <b>{props.dmxHealth.label}</b>
            <small>{props.dmxHealth.value}</small>
          </span>
          <span class="metric">{props.packetBytes} bytes</span>
          <span class="metric projectFile" title={props.currentProjectPath ?? "Unsaved project"}>{projectLabel()}</span>
          <button onClick={props.onNewProject}>New</button>
          <button onClick={props.onSaveProject}>Save</button>
          <button onClick={props.onSaveProjectAs}>Save As</button>
          <button onClick={props.onLoadProject}>Load</button>
        </div>
      </header>

      <nav class="workspaceTabs" aria-label="Workspace">
        <button
          class={props.workspaceTab === "setup" ? "active" : ""}
          onClick={() => props.onWorkspaceTab("setup")}
          aria-pressed={props.workspaceTab === "setup"}
        >
          Setup
        </button>
        <button
          class={props.workspaceTab === "control" ? "active" : ""}
          onClick={() => props.onWorkspaceTab("control")}
          aria-pressed={props.workspaceTab === "control"}
        >
          Control
        </button>
        <button
          class={props.workspaceTab === "touch" ? "active" : ""}
          onClick={() => props.onWorkspaceTab("touch")}
          aria-pressed={props.workspaceTab === "touch"}
        >
          Touch
        </button>
      </nav>

      <Show when={props.workspaceTab === "setup"}>
        <nav class="setupModeTabs" aria-label="Setup mode">
          <For each={setupSubTabs}>
            {(tab) => (
              <button
                class={props.setupSubTab === tab.id ? "active" : ""}
                title={tab.description}
                onClick={() => props.onSetupSubTab(tab.id)}
                aria-pressed={props.setupSubTab === tab.id}
              >
                {tab.label}
              </button>
            )}
          </For>
        </nav>
      </Show>

      <Show when={props.workspaceTab === "control"}>
        <nav class="controlModeTabs" aria-label="Control mode">
          <For each={controlModes}>
            {(mode) => (
              <button
                class={props.controlMode === mode.id ? "active" : ""}
                title={mode.description}
                onClick={() => props.onControlMode(mode.id)}
                aria-pressed={props.controlMode === mode.id}
              >
                {mode.label}
              </button>
            )}
          </For>
        </nav>
      </Show>
    </>
  );
}
