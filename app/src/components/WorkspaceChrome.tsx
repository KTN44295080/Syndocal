import { For, Show } from "solid-js";
import { setupSubTabs } from "../uiModes";
import type { SetupSubTab, WorkspaceTab } from "../uiModes";

type WorkspaceChromeProps = {
  workspaceTab: WorkspaceTab;
  setupSubTab: SetupSubTab;
  blackout: boolean;
  videoBlackout: boolean;
  bpm: number;
  tickMs: number;
  jitterUs: number;
  packetBytes: number;
  dmxSuccessCount: number;
  dmxOutputCount: number;
  projectLabel: string;
  projectDirty: boolean;
  currentProjectPath: string | null;
  onWorkspaceTab: (tab: WorkspaceTab) => void;
  onSetupSubTab: (tab: SetupSubTab) => void;
  onNewProject: () => void;
  onSaveProject: () => void;
  onSaveProjectAs: () => void;
  onLoadProject: () => void;
  onLoadSample: () => void;
  onRunSmoke: () => void;
};

export function WorkspaceChrome(props: WorkspaceChromeProps) {
  const liveLabel = () => {
    if (props.blackout && props.videoBlackout) {
      return "All BO";
    }
    if (props.blackout) {
      return "DMX BO";
    }
    if (props.videoBlackout) {
      return "Video BO";
    }
    return "Live";
  };

  return (
    <div class="workspaceChrome">
      <header class="topbar">
        <div>
          <h1>Rayard</h1>
          <p>Unified lighting and video control: GDTF patch, cues, timeline, effects, DMX output.</p>
        </div>
        <div class="status">
          <span class={props.blackout || props.videoBlackout ? "pill danger" : "pill ok"}>{liveLabel()}</span>
          <span class="metric">{props.bpm.toFixed(1)} BPM</span>
          <span class="metric">{props.tickMs} ms tick</span>
          <span class="metric">{props.jitterUs} us jitter</span>
          <span class="metric">{props.packetBytes} bytes</span>
          <span class="metric">
            {props.dmxSuccessCount}/{props.dmxOutputCount} outputs
          </span>
          <span
            class={props.projectDirty ? "metric projectFile dirty" : "metric projectFile"}
            title={props.currentProjectPath ?? "Unsaved project"}
          >
            {props.projectLabel}
          </span>
          <button onClick={props.onNewProject}>New</button>
          <button onClick={props.onSaveProject}>Save</button>
          <button onClick={props.onSaveProjectAs}>Save As</button>
          <button onClick={props.onLoadProject}>Load</button>
          <button onClick={props.onLoadSample}>Load Sample</button>
          <button onClick={props.onRunSmoke}>Run Smoke</button>
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
    </div>
  );
}
