import { createSignal, For, onCleanup, Show } from "solid-js";
import {
  projectRecoverySourceLabel,
  projectRecoveryTimeLabel,
  type ProjectRecoveryCheckpoint,
} from "../projectRecoveryStorage";
import { recentProjectFileName } from "../projectRecentStorage";
import { controlModes, setupSubTabs } from "../uiModes";
import type { ControlMode, SetupSubTab, WorkspaceTab } from "../uiModes";

type WorkspaceChromeProps = {
  workspaceTab: WorkspaceTab;
  setupSubTab: SetupSubTab;
  controlMode: ControlMode;
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
  recentProjectPaths: string[];
  recoveryCheckpoint: ProjectRecoveryCheckpoint | null;
  canGo: boolean;
  nextCueLabel: string;
  onWorkspaceTab: (tab: WorkspaceTab) => void;
  onSetupSubTab: (tab: SetupSubTab) => void;
  onControlMode: (mode: ControlMode) => void;
  onGo: () => void;
  onNewProject: () => void;
  onSaveProject: () => void;
  onSaveProjectAs: () => void;
  onLoadProject: () => void;
  onLoadRecentProject: (path: string) => void;
  onClearRecentProjects: () => void;
  onLoadRecovery: () => void;
  onDiscardRecovery: () => void;
  onLoadSample: () => void;
  onRunSmoke: () => void;
};

export function WorkspaceChrome(props: WorkspaceChromeProps) {
  let projectMenuRoot: HTMLDivElement | undefined;
  const [projectMenuOpen, setProjectMenuOpen] = createSignal(false);

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

  const closeProjectMenu = () => setProjectMenuOpen(false);

  const runProjectMenuAction = (action: () => void) => {
    closeProjectMenu();
    action();
  };

  const handleWindowPointerDown = (event: PointerEvent) => {
    if (!projectMenuOpen()) {
      return;
    }
    const target = event.target;
    if (target instanceof Node && projectMenuRoot?.contains(target)) {
      return;
    }
    closeProjectMenu();
  };

  const handleWindowKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      closeProjectMenu();
    }
  };

  window.addEventListener("pointerdown", handleWindowPointerDown);
  window.addEventListener("keydown", handleWindowKeyDown);
  onCleanup(() => {
    window.removeEventListener("pointerdown", handleWindowPointerDown);
    window.removeEventListener("keydown", handleWindowKeyDown);
  });

  return (
    <div class="workspaceChrome">
      <header class="topbar">
        <div class="topbarLeft" ref={projectMenuRoot}>
          <button
            class={projectMenuOpen() ? "appMenuButton active" : "appMenuButton"}
            title="Project menu"
            aria-label="Project menu"
            aria-haspopup="menu"
            aria-expanded={projectMenuOpen()}
            onClick={() => setProjectMenuOpen((open) => !open)}
          >
            ...
          </button>
          <Show when={projectMenuOpen()}>
            <div class="appProjectMenu" role="menu">
              <button role="menuitem" aria-keyshortcuts="Control+N Meta+N" onClick={() => runProjectMenuAction(props.onNewProject)}>
                <span>New</span>
              </button>
              <button role="menuitem" aria-keyshortcuts="Control+S Meta+S" onClick={() => runProjectMenuAction(props.onSaveProject)}>
                <span>Save</span>
              </button>
              <button
                role="menuitem"
                aria-keyshortcuts="Control+Shift+S Meta+Shift+S"
                onClick={() => runProjectMenuAction(props.onSaveProjectAs)}
              >
                <span>Save As</span>
              </button>
              <button role="menuitem" aria-keyshortcuts="Control+O Meta+O" onClick={() => runProjectMenuAction(props.onLoadProject)}>
                <span>Load</span>
              </button>
              <Show when={props.recoveryCheckpoint}>
                {(checkpoint) => (
                  <>
                    <div class="appProjectMenuLabel recoveryLabel" role="separator">
                      <span>Recovery</span>
                    </div>
                    <button class="recoveryProjectMenuItem" role="menuitem" onClick={() => runProjectMenuAction(props.onLoadRecovery)}>
                      <span>{projectRecoverySourceLabel(checkpoint())}</span>
                      <small>{projectRecoveryTimeLabel(checkpoint())}</small>
                    </button>
                    <button class="mutedMenuItem" role="menuitem" onClick={() => runProjectMenuAction(props.onDiscardRecovery)}>
                      <span>Discard Recovery</span>
                    </button>
                  </>
                )}
              </Show>
              <Show when={props.recentProjectPaths.length > 0}>
                <div class="appProjectMenuLabel" role="separator">
                  <span>Recent</span>
                </div>
                <For each={props.recentProjectPaths.slice(0, 5)}>
                  {(path) => (
                    <button
                      class="recentProjectMenuItem"
                      role="menuitem"
                      title={path}
                      onClick={() => runProjectMenuAction(() => props.onLoadRecentProject(path))}
                    >
                      <span>{recentProjectFileName(path)}</span>
                      <small>{path}</small>
                    </button>
                  )}
                </For>
                <button class="mutedMenuItem" role="menuitem" onClick={() => runProjectMenuAction(props.onClearRecentProjects)}>
                  <span>Clear Recent</span>
                </button>
              </Show>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onLoadSample)}>
                <span>Sample</span>
              </button>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onRunSmoke)}>
                <span>Smoke</span>
              </button>
            </div>
          </Show>
          <nav class="workspaceTabs" aria-label="Workspace">
            <button
              class={props.workspaceTab === "setup" ? "active" : ""}
              title="Setup workspace"
              aria-keyshortcuts="F1"
              onClick={() => props.onWorkspaceTab("setup")}
              aria-pressed={props.workspaceTab === "setup"}
            >
              Setup
            </button>
            <button
              class={props.workspaceTab === "control" ? "active" : ""}
              title="Control workspace"
              aria-keyshortcuts="F2"
              onClick={() => props.onWorkspaceTab("control")}
              aria-pressed={props.workspaceTab === "control"}
            >
              Control
            </button>
            <button
              class={props.workspaceTab === "touch" ? "active" : ""}
              title="Touch workspace"
              aria-keyshortcuts="F3"
              onClick={() => props.onWorkspaceTab("touch")}
              aria-pressed={props.workspaceTab === "touch"}
            >
              Touch
            </button>
          </nav>
        </div>

        <div
          class={props.projectDirty ? "topbarProject dirty" : "topbarProject"}
          title={props.currentProjectPath ?? "Unsaved project"}
        >
          <strong>Syndocal</strong>
          <span>{props.projectLabel}</span>
        </div>

        <div class="status">
          <button
            class="goButton"
            type="button"
            title={`GO: ${props.nextCueLabel}`}
            disabled={!props.canGo}
            onClick={props.onGo}
          >
            GO
          </button>
          <span class="bpmReadout">
            <small>BPM</small>
            <strong>{props.bpm.toFixed(0)}</strong>
          </span>
          <span class={props.blackout || props.videoBlackout ? "pill danger" : "pill ok"}>{liveLabel()}</span>
          <span
            class="metric tickMetric"
            title={`Engine ${props.tickMs} ms / jitter ${props.jitterUs} us / ${props.packetBytes} B`}
          >
            {props.tickMs} ms
          </span>
          <span class="metric outputMetric" title="Healthy DMX outputs / configured DMX outputs">
            <small>DMX</small>
            <strong>{props.dmxSuccessCount}/{props.dmxOutputCount}</strong>
          </span>
          <button class="projectAction" title="Save project (Ctrl/Cmd+S)" onClick={props.onSaveProject}>
            Save
          </button>
          <button class="projectAction" title="Load project (Ctrl/Cmd+O)" onClick={props.onLoadProject}>
            Load
          </button>
        </div>
      </header>

      <Show when={props.workspaceTab === "setup"}>
        <nav class="setupModeTabs" aria-label="Setup mode">
          <For each={setupSubTabs}>
            {(tab, index) => (
              <button
                class={props.setupSubTab === tab.id ? "active" : ""}
                title={tab.description}
                aria-keyshortcuts={`${index() + 1}`}
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
                aria-keyshortcuts={mode.label[0]}
                onClick={() => props.onControlMode(mode.id)}
                aria-pressed={props.controlMode === mode.id}
              >
                {mode.label}
              </button>
            )}
          </For>
        </nav>
      </Show>
    </div>
  );
}
