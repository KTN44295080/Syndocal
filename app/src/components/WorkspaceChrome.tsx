import { createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import {
  projectRecoverySourceLabel,
  projectRecoveryTimeLabel,
  type ProjectRecoveryCheckpoint,
} from "../projectRecoveryStorage";
import { recentProjectFileName } from "../projectRecentStorage";
import type {
  ApplicationUpdateCheck,
  ApplicationUpdateConfiguration,
  ApplicationUpdateProgress,
  ProjectBackupSummary,
  ProjectHistoryStatus,
  OperatorLockMode,
} from "../types";
import { controlModes, setupAreaForSubTab, setupAreas, setupSubTabs, setupSubTabsForArea } from "../uiModes";
import type { ControlMode, SetupSubTab, WorkspaceTab } from "../uiModes";
import type { UiLocale } from "../uiLocalization";

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
  projectBackups: ProjectBackupSummary[];
  historyStatus: ProjectHistoryStatus;
  applicationUpdateConfiguration: ApplicationUpdateConfiguration | null;
  applicationUpdateCheck: ApplicationUpdateCheck | null;
  applicationUpdateProgress: ApplicationUpdateProgress | null;
  applicationUpdateBusy: boolean;
  applicationUpdateError: string | null;
  uiScale: 90 | 100 | 110;
  uiLocale: UiLocale;
  canGo: boolean;
  nextCueLabel: string;
  operatorLockMode: OperatorLockMode | null;
  operations: JSX.Element;
  onWorkspaceTab: (tab: WorkspaceTab) => void;
  onSetupSubTab: (tab: SetupSubTab) => void;
  onControlMode: (mode: ControlMode) => void;
  onGo: () => void;
  onNewProject: () => void;
  onSaveUserTemplate: () => void;
  onLoadUserTemplate: () => void;
  onSaveProject: () => void;
  onSaveProjectAs: () => void;
  onLoadProject: () => void;
  onImportDaslightProject: () => void;
  onLoadRecentProject: (path: string) => void;
  onClearRecentProjects: () => void;
  onLoadRecovery: () => void;
  onDiscardRecovery: () => void;
  onLoadBackup: (backup: ProjectBackupSummary) => void;
  onDeleteBackup: (backup: ProjectBackupSummary) => void;
  onExportDiagnostics: () => void;
  onCheckForUpdates: () => void;
  onInstallUpdate: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onUiLocale: (locale: UiLocale) => void;
  onUiScale: (scale: 90 | 100 | 110) => void;
  onResetWorkspaceLayout: () => void;
  onLoadSample: () => void;
  onRunSmoke: () => void;
};

export function WorkspaceChrome(props: WorkspaceChromeProps) {
  let projectMenuRoot: HTMLDivElement | undefined;
  const [projectMenuOpen, setProjectMenuOpen] = createSignal(false);
  const activeSetupArea = () => setupAreaForSubTab(props.setupSubTab);

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

  const updateProgressLabel = () => {
    const progress = props.applicationUpdateProgress;
    if (!progress) return "";
    if (progress.phase === "verified") return "Signature verified";
    if (progress.phase === "verifying") return "Verifying signature...";
    if (progress.total_bytes && progress.total_bytes > 0) {
      return `${Math.min(100, Math.round((progress.downloaded_bytes / progress.total_bytes) * 100))}% downloaded`;
    }
    return `${Math.round(progress.downloaded_bytes / 1024 / 1024)} MB downloaded`;
  };

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
            disabled={props.operatorLockMode !== null}
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
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onImportDaslightProject)}>
                <span>Import .dvc</span>
                <small>Daslight Project (.dvc)</small>
              </button>
              <div class="appProjectMenuLabel templateLabel" role="separator">
                <span>User Templates</span>
              </div>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onLoadUserTemplate)}>
                <span>New from Template</span>
                <small>Outputs open disabled</small>
              </button>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onSaveUserTemplate)}>
                <span>Save as Template</span>
                <small>Includes MIDI / OSC mappings</small>
              </button>
              <button
                role="menuitem"
                disabled={!props.historyStatus.can_undo}
                aria-keyshortcuts="Control+Z Meta+Z"
                title={props.historyStatus.undo_label ?? "Nothing to undo"}
                onClick={() => runProjectMenuAction(props.onUndo)}
              >
                <span>Undo{props.historyStatus.undo_label ? ` · ${props.historyStatus.undo_label}` : ""}</span>
                <small>{props.historyStatus.undo_depth} step{props.historyStatus.undo_depth === 1 ? "" : "s"}</small>
              </button>
              <button
                role="menuitem"
                disabled={!props.historyStatus.can_redo}
                aria-keyshortcuts="Control+Y Meta+Shift+Z"
                title={props.historyStatus.redo_label ?? "Nothing to redo"}
                onClick={() => runProjectMenuAction(props.onRedo)}
              >
                <span>Redo{props.historyStatus.redo_label ? ` · ${props.historyStatus.redo_label}` : ""}</span>
                <small>{props.historyStatus.redo_depth} step{props.historyStatus.redo_depth === 1 ? "" : "s"}</small>
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
              <Show when={props.projectBackups.length > 0}>
                <div class="appProjectMenuLabel backupLabel" role="separator">
                  <span>Backups</span>
                </div>
                <For each={props.projectBackups.slice(0, 5)}>
                  {(backup) => (
                    <div class="backupProjectMenuItem">
                      <button
                        role="menuitem"
                        title={backup.source_path ?? "Untitled project"}
                        onClick={() => runProjectMenuAction(() => props.onLoadBackup(backup))}
                      >
                        <span>{backup.source_path ? recentProjectFileName(backup.source_path) : "Untitled project"}</span>
                        <small>{new Date(backup.created_at_unix_ms).toLocaleString()} · {backup.reason}</small>
                      </button>
                      <button
                        class="deleteBackupMenuItem"
                        role="menuitem"
                        title="Delete this backup"
                        aria-label={`Delete backup from ${new Date(backup.created_at_unix_ms).toLocaleString()}`}
                        onClick={() => runProjectMenuAction(() => props.onDeleteBackup(backup))}
                      >
                        ×
                      </button>
                    </div>
                  )}
                </For>
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
              <div class="appProjectMenuLabel" role="separator">
                <span>Interface Scale</span>
              </div>
              <div class="uiScaleMenu" role="group" aria-label="Interface scale">
                <For each={[90, 100, 110] as const}>
                  {(scale) => (
                    <button
                      type="button"
                      aria-pressed={props.uiScale === scale}
                      class={props.uiScale === scale ? "active" : ""}
                      onClick={() => props.onUiScale(scale)}
                    >
                      {scale}%
                    </button>
                  )}
                </For>
              </div>
              <div class="appProjectMenuLabel" role="separator">
                <span>UI language</span>
              </div>
              <div class="uiScaleMenu uiLocaleMenu" role="group" aria-label="UI language">
                <button
                  type="button"
                  aria-pressed={props.uiLocale === "en"}
                  class={props.uiLocale === "en" ? "active" : ""}
                  onClick={() => props.onUiLocale("en")}
                >
                  English
                </button>
                <button
                  type="button"
                  aria-pressed={props.uiLocale === "ja"}
                  class={props.uiLocale === "ja" ? "active" : ""}
                  onClick={() => props.onUiLocale("ja")}
                >
                  Japanese
                </button>
              </div>
              <div class="appProjectMenuLabel" role="separator">
                <span>Workspace Layout</span>
              </div>
              <button
                role="menuitem"
                data-reset-workspace-layout
                onClick={() => runProjectMenuAction(props.onResetWorkspaceLayout)}
              >
                <span>Reset Layout</span>
                <small>Tabs, panes, and desk surfaces auto-save on this device</small>
              </button>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onLoadSample)}>
                <span>Sample</span>
              </button>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onRunSmoke)}>
                <span>Smoke</span>
              </button>
              <button role="menuitem" onClick={() => runProjectMenuAction(props.onExportDiagnostics)}>
                <span>Export Diagnostics</span>
              </button>
              <div class="appProjectMenuLabel" role="separator">
                <span>Application Updates</span>
                <small>
                  v{props.applicationUpdateConfiguration?.current_version ?? "..."} · {props.applicationUpdateConfiguration?.channel ?? "..."}
                </small>
              </div>
              <button
                role="menuitem"
                disabled={!props.applicationUpdateConfiguration?.enabled || props.applicationUpdateBusy}
                title={props.applicationUpdateConfiguration?.reason ?? props.applicationUpdateConfiguration?.endpoint_origin ?? "Signed update channel"}
                onClick={props.onCheckForUpdates}
              >
                <span>{props.applicationUpdateBusy ? "Checking..." : "Check for Updates"}</span>
                <small>
                  {props.applicationUpdateConfiguration?.enabled
                    ? props.applicationUpdateConfiguration.endpoint_origin
                    : props.applicationUpdateConfiguration?.reason ?? "Reading update configuration..."}
                </small>
              </button>
              <Show when={props.applicationUpdateCheck?.available && props.applicationUpdateCheck.version}>
                <button
                  class="updateAvailableMenuItem"
                  role="menuitem"
                  disabled={props.applicationUpdateBusy}
                  onClick={props.onInstallUpdate}
                >
                  <span>Install v{props.applicationUpdateCheck?.version}</span>
                  <small>{updateProgressLabel() || "Signed artifact · project backup first"}</small>
                </button>
              </Show>
              <Show when={props.applicationUpdateError}>
                <div class="inlineError" role="status">{props.applicationUpdateError}</div>
              </Show>
            </div>
          </Show>
          <nav class="workspaceTabs" aria-label="Workspace">
            <button
              class={props.workspaceTab === "setup" ? "active" : ""}
              title="Setup workspace"
              aria-keyshortcuts="F1"
              onClick={() => props.onWorkspaceTab("setup")}
              aria-pressed={props.workspaceTab === "setup"}
              disabled={props.operatorLockMode !== null}
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
          {props.operations}
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
          <button
            class="projectAction"
            title="Load project (Ctrl/Cmd+O)"
            disabled={props.operatorLockMode !== null}
            onClick={props.onLoadProject}
          >
            Load
          </button>
        </div>
      </header>

      <Show when={props.workspaceTab === "setup"}>
        <div class="setupNavigation">
          <nav class="setupAreaTabs" aria-label="Setup area">
            <For each={setupAreas}>
              {(area) => (
                <button
                  class={activeSetupArea() === area.id ? "active" : ""}
                  title={area.description}
                  onClick={() => props.onSetupSubTab(area.defaultTab)}
                  aria-pressed={activeSetupArea() === area.id}
                >
                  {area.label}
                </button>
              )}
            </For>
          </nav>
          <nav class="setupModeTabs" aria-label={`${activeSetupArea()} setup mode`}>
            <For each={setupSubTabsForArea(activeSetupArea())}>
              {(tab) => (
                <button
                  class={props.setupSubTab === tab.id ? "active" : ""}
                  title={tab.description}
                  aria-keyshortcuts={`Alt+${setupSubTabs.findIndex((candidate) => candidate.id === tab.id) + 1}`}
                  onClick={() => props.onSetupSubTab(tab.id)}
                  aria-pressed={props.setupSubTab === tab.id}
                >
                  {tab.label}
                </button>
              )}
            </For>
          </nav>
        </div>
      </Show>
      <Show when={props.workspaceTab === "control"}>
        <Show
          when={props.controlMode === "mixer"}
          fallback={
            <div class="controlWorkspaceHeader" aria-label="Control workspace header">
              <strong>Live Desk</strong>
              <span>{controlModes.find((mode) => mode.id === props.controlMode)?.label}</span>
            </div>
          }
        >
          <nav class="controlModeTabs" aria-label="Control mode">
            <For each={controlModes}>
              {(mode) => (
                <button
                  class={props.controlMode === mode.id ? "active" : ""}
                  title={mode.description}
                  aria-keyshortcuts={mode.label[0]}
                  onClick={() => props.onControlMode(mode.id)}
                  aria-pressed={props.controlMode === mode.id}
                  disabled={props.operatorLockMode === "Partial" && mode.id === "edit"}
                >
                  {mode.label}
                </button>
              )}
            </For>
          </nav>
        </Show>
      </Show>
    </div>
  );
}
