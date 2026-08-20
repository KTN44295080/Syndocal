import { getCurrentWindow } from "@tauri-apps/api/window";
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
import { editDomainModes, setupAreaForSubTab, setupAreas, setupSubTabs, setupSubTabsForArea } from "../uiModes";
import type { ControlMode, SetupSubTab, WorkspaceTab } from "../uiModes";
import type { UiLocale } from "../uiLocalization";
import { TopbarPulseMeter } from "./TopbarPulseMeter";
import {
  controlMappingTargetData,
  type ControlLearnMode,
} from "../controlMappingLearn";

type WorkspaceChromeProps = {
  workspaceTab: WorkspaceTab;
  setupSubTab: SetupSubTab;
  controlMode: ControlMode;
  blackout: boolean;
  videoBlackout: boolean;
  lightingMaster: number;
  videoMaster: number;
  bpm: number;
  liveAudioInputRunning: boolean;
  liveAudioInputStale: boolean;
  liveAudioInputSafetyClearPending: boolean;
  liveAudioInputTelemetryFresh: boolean;
  liveAudioInputRms: number;
  liveAudioInputPeak: number;
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
  daslightProjectImportBusy: boolean;
  historyStatus: ProjectHistoryStatus;
  applicationUpdateConfiguration: ApplicationUpdateConfiguration | null;
  applicationUpdateCheck: ApplicationUpdateCheck | null;
  applicationUpdateProgress: ApplicationUpdateProgress | null;
  applicationUpdateBusy: boolean;
  applicationUpdateError: string | null;
  uiScale: 90 | 100 | 110;
  uiLocale: UiLocale;
  canBack: boolean;
  canGo: boolean;
  canPauseFade: boolean;
  fadePaused: boolean;
  canToggleTimeline: boolean;
  timelinePlaying: boolean;
  anyFixtureFlags: boolean;
  nextCueLabel: string;
  operatorLockMode: OperatorLockMode | null;
  controlLearnMode: ControlLearnMode | null;
  controlLearnBusy: boolean;
  controlLearnTargetLabel: string | null;
  operations: JSX.Element;
  onWorkspaceTab: (tab: WorkspaceTab) => void;
  onSetupSubTab: (tab: SetupSubTab) => void;
  onControlMode: (mode: ControlMode) => void;
  onBack: () => void;
  onGo: () => void;
  onToggleFade: () => void;
  onToggleTimeline: () => void;
  onSetBlackout: (enabled: boolean) => void;
  onSetVideoBlackout: (enabled: boolean) => void;
  onSetAllBlackout: (enabled: boolean) => void;
  onClearFixtureFlags: () => void;
  onControlLearnMode: (mode: ControlLearnMode | null) => void;
  onLightingMaster: (level: number) => void | Promise<void>;
  onVideoMaster: (level: number) => void | Promise<void>;
  onTapBpm: () => void | Promise<void>;
  onOpenLiveAudioInputSettings: () => void;
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

const isTauriRuntime = () =>
  typeof window !== "undefined" &&
  Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

type WindowControlAction = "minimize" | "toggleMaximize" | "close";

function WindowControls() {
  const runWindowControl = async (action: WindowControlAction) => {
    if (!isTauriRuntime()) return;
    const appWindow = getCurrentWindow();
    if (action === "minimize") {
      await appWindow.minimize();
      return;
    }
    if (action === "toggleMaximize") {
      await appWindow.toggleMaximize();
      return;
    }
    // close(), unlike destroy(), raises the existing CloseRequested event so
    // App owns recovery/dirty-editor confirmation before the window exits.
    await appWindow.close();
  };

  return (
    <div class="windowControls" data-window-controls role="group" aria-label="ウィンドウ操作">
      <button
        type="button"
        data-window-control="minimize"
        title="最小化"
        aria-label="最小化"
        onClick={() => void runWindowControl("minimize")}
      >
        <span class="windowControlGlyph minimize" aria-hidden="true" />
      </button>
      <button
        type="button"
        data-window-control="maximize"
        title="最大化または元に戻す"
        aria-label="最大化または元に戻す"
        onClick={() => void runWindowControl("toggleMaximize")}
      >
        <span class="windowControlGlyph maximize" aria-hidden="true" />
        <span class="windowControlGlyph restore" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="windowCloseButton"
        data-window-control="close"
        title="閉じる"
        aria-label="閉じる"
        onClick={() => void runWindowControl("close")}
      >
        <span class="windowControlGlyph close" aria-hidden="true" />
      </button>
    </div>
  );
}

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
      <header class="topbar" data-tauri-drag-region>
        <div class="topbarLeft" data-tauri-drag-region ref={projectMenuRoot}>
          <button
            class={projectMenuOpen() ? "appMenuButton active" : "appMenuButton"}
            data-project-drop-surface
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
              <button
                role="menuitem"
                data-project-menu-action="save"
                aria-keyshortcuts="Control+S Meta+S"
                onClick={() => runProjectMenuAction(props.onSaveProject)}
              >
                <span>Save</span>
              </button>
              <button
                role="menuitem"
                aria-keyshortcuts="Control+Shift+S Meta+Shift+S"
                onClick={() => runProjectMenuAction(props.onSaveProjectAs)}
              >
                <span>Save As</span>
              </button>
              <button
                role="menuitem"
                data-project-menu-action="load"
                aria-keyshortcuts="Control+O Meta+O"
                onClick={() => runProjectMenuAction(props.onLoadProject)}
              >
                <span>Load</span>
              </button>
              <button
                role="menuitem"
                disabled={props.daslightProjectImportBusy}
                aria-busy={props.daslightProjectImportBusy}
                onClick={() => runProjectMenuAction(props.onImportDaslightProject)}
              >
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
                <small>Includes MIDI / OSC / DMX mappings</small>
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
              data-workspace-option="setup"
              data-no-localize
              title="Setup workspace"
              aria-label="Setup workspace"
              aria-keyshortcuts="F1"
              onClick={() => props.onWorkspaceTab("setup")}
              aria-pressed={props.workspaceTab === "setup"}
              disabled={props.operatorLockMode !== null}
            >
              Setup
            </button>
            <button
              class={props.workspaceTab === "control" ? "active" : ""}
              data-workspace-option="control"
              data-no-localize
              title="Edit workspace"
              aria-label="Edit workspace"
              aria-keyshortcuts="F2"
              onClick={() => props.onWorkspaceTab("control")}
              aria-pressed={props.workspaceTab === "control"}
            >
              Edit
            </button>
            <button
              class={props.workspaceTab === "touch" ? "active" : ""}
              data-workspace-option="touch"
              data-no-localize
              title="Control workspace"
              aria-label="Control workspace"
              aria-keyshortcuts="F3"
              onClick={() => props.onWorkspaceTab("touch")}
              aria-pressed={props.workspaceTab === "touch"}
            >
              Control
            </button>
          </nav>
        </div>

        <div
          class={props.projectDirty ? "topbarProject dirty" : "topbarProject"}
          title={props.currentProjectPath ?? "Unsaved project"}
          data-tauri-drag-region
        >
          <strong data-tauri-drag-region>Syndocal</strong>
          <span data-tauri-drag-region>{props.projectLabel}</span>
          {props.operations}
        </div>

        <div class="status" data-tauri-drag-region>
          <div class="topbarTransportCluster" role="group" aria-label="Global show controls">
            <button
              type="button"
              class="topbarIconButton"
              data-global-operator-action="back"
              title="Back"
              aria-label="Back"
              disabled={!props.canBack}
              onClick={props.onBack}
              {...controlMappingTargetData({ action: "TriggerPreviousCue", label: "Back" })}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M5 4v12M15 5l-7 5 7 5z" />
              </svg>
            </button>
            <button
              class="goButton"
              type="button"
              data-global-operator-action="go"
              title={`GO: ${props.nextCueLabel}`}
              disabled={!props.canGo}
              onClick={props.onGo}
              {...controlMappingTargetData({ action: "TriggerNextCue", label: "GO" })}
            >
              GO
            </button>
            <button
              type="button"
              class="topbarIconButton"
              data-global-operator-action="fade"
              title={props.fadePaused ? "Resume Fade" : "Pause Fade"}
              aria-label={props.fadePaused ? "Resume Fade" : "Pause Fade"}
              aria-pressed={props.fadePaused}
              disabled={!props.canPauseFade}
              onClick={props.onToggleFade}
              {...controlMappingTargetData({ action: "CueFadePause", label: "Cue fade pause" })}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                {props.fadePaused
                  ? <path d="M7 5.5v9M12.5 5.5v9" />
                  : <path d="M7 4.5 14 10l-7 5z" />}
              </svg>
            </button>
            <button
              type="button"
              class="topbarIconButton"
              data-global-operator-action="timeline"
              title={props.timelinePlaying ? "Pause Timeline" : "Play Timeline"}
              aria-label={props.timelinePlaying ? "Pause Timeline" : "Play Timeline"}
              aria-pressed={props.timelinePlaying}
              disabled={!props.canToggleTimeline}
              onClick={props.onToggleTimeline}
              {...controlMappingTargetData({ action: "TimelinePlay", label: "Timeline play" })}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M3.5 16h13M5 14v4M10 14v4M15 14v4" />
                {props.timelinePlaying
                  ? <path d="M7 4.5v9M12.5 4.5v9" />
                  : <path d="M7 4.5 14 9l-7 4.5z" />}
              </svg>
            </button>
            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.blackout ? " engaged" : ""}`}
              data-global-operator-action="dmx-blackout"
              title={props.blackout ? "Clear DMX Blackout" : "DMX Blackout"}
              aria-label={props.blackout ? "Clear DMX Blackout" : "DMX Blackout"}
              aria-pressed={props.blackout}
              onClick={() => props.onSetBlackout(!props.blackout)}
              {...controlMappingTargetData({ action: "Blackout", label: "DMX Blackout" })}
            >
              <span data-no-localize>DMX</span>
            </button>
            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.videoBlackout ? " engaged" : ""}`}
              data-global-operator-action="video-blackout"
              title={props.videoBlackout ? "Clear Video Blackout" : "Video Blackout"}
              aria-label={props.videoBlackout ? "Clear Video Blackout" : "Video Blackout"}
              aria-pressed={props.videoBlackout}
              onClick={() => props.onSetVideoBlackout(!props.videoBlackout)}
              {...controlMappingTargetData({ action: "VideoBlackout", label: "Video Blackout" })}
            >
              <span data-no-localize>VID</span>
            </button>
            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.blackout && props.videoBlackout ? " engaged" : ""}`}
              data-global-operator-action="all-blackout"
              title={props.blackout && props.videoBlackout ? "Clear All Blackout" : "All Blackout"}
              aria-label={props.blackout && props.videoBlackout ? "Clear All Blackout" : "All Blackout"}
              aria-pressed={props.blackout && props.videoBlackout}
              onClick={() => props.onSetAllBlackout(!(props.blackout && props.videoBlackout))}
              {...controlMappingTargetData({ action: "AllBlackout", label: "All Blackout" })}
            >
              <span data-no-localize>ALL</span>
            </button>
            <button
              type="button"
              class="topbarIconButton"
              data-global-operator-action="clear-flags"
              title="Clear Fixture Flags"
              aria-label="Clear Fixture Flags"
              disabled={!props.anyFixtureFlags}
              onClick={props.onClearFixtureFlags}
              {...controlMappingTargetData({ action: "ClearFixtureFlags", attribute: "all", label: "Clear fixture flags" })}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M5 17V3.5M5 4h8l-1.5 3L13 10H5M10 13l5 5M15 13l-5 5" />
              </svg>
            </button>
          </div>
          <div class="topbarLearnCluster" role="group" aria-label="Control mapping learn">
            <button
              type="button"
              class="topbarIconButton topbarLearnButton"
              classList={{ active: props.controlLearnMode === "midi", waiting: props.controlLearnMode === "midi" && props.controlLearnBusy }}
              data-control-learn-toggle="midi"
              title={props.controlLearnMode === "midi" ? "Exit MIDI Learn" : "MIDI Learn"}
              aria-label={props.controlLearnMode === "midi" ? "Exit MIDI Learn" : "MIDI Learn"}
              aria-pressed={props.controlLearnMode === "midi"}
              onClick={() => props.onControlLearnMode(props.controlLearnMode === "midi" ? null : "midi")}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M3 5h14v10H3zM6 5v6M10 5v6M14 5v6M5 11v4M9 11v4M13 11v4" />
              </svg>
            </button>
            <button
              type="button"
              class="topbarIconButton topbarLearnButton"
              classList={{ active: props.controlLearnMode === "osc", waiting: props.controlLearnMode === "osc" && props.controlLearnBusy }}
              data-control-learn-toggle="osc"
              title={props.controlLearnMode === "osc" ? "Exit OSC Learn" : "OSC Learn"}
              aria-label={props.controlLearnMode === "osc" ? "Exit OSC Learn" : "OSC Learn"}
              aria-pressed={props.controlLearnMode === "osc"}
              onClick={() => props.onControlLearnMode(props.controlLearnMode === "osc" ? null : "osc")}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <circle cx="5" cy="10" r="2" />
                <circle cx="15" cy="5" r="2" />
                <circle cx="15" cy="15" r="2" />
                <path d="M7 9l6-3M7 11l6 3" />
              </svg>
            </button>
            <button
              type="button"
              class="topbarIconButton topbarLearnButton"
              classList={{ active: props.controlLearnMode === "dmx", waiting: props.controlLearnMode === "dmx" && props.controlLearnBusy }}
              data-control-learn-toggle="dmx"
              title={props.controlLearnMode === "dmx" ? "Exit DMX Learn" : "DMX Learn"}
              aria-label={props.controlLearnMode === "dmx" ? "Exit DMX Learn" : "DMX Learn"}
              aria-pressed={props.controlLearnMode === "dmx"}
              onClick={() => props.onControlLearnMode(props.controlLearnMode === "dmx" ? null : "dmx")}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M3 4h14v12H3zM6 7h2v6H6zM9 9h2v4H9zM12 6h2v7h-2z" />
              </svg>
            </button>
          </div>
          <span class="bpmReadout" data-tauri-drag-region>
            <small data-tauri-drag-region>BPM</small>
            <strong data-tauri-drag-region>{props.bpm.toFixed(0)}</strong>
          </span>
          <button
            class="topbarTapButton"
            type="button"
            data-topbar-tap
            title="Tap"
            aria-label="Tap"
            onClick={() => void props.onTapBpm()}
            {...controlMappingTargetData({ action: "TapBpm", label: "Tap BPM" })}
          >
            Tap
          </button>
          <TopbarPulseMeter
            running={props.liveAudioInputRunning}
            engineStale={props.liveAudioInputStale}
            safetyClearPending={props.liveAudioInputSafetyClearPending}
            telemetryFresh={props.liveAudioInputTelemetryFresh}
            rms={props.liveAudioInputRms}
            peak={props.liveAudioInputPeak}
            onOpenSettings={props.onOpenLiveAudioInputSettings}
          />
          <span
            class={props.blackout || props.videoBlackout ? "pill danger" : "pill ok"}
            title={`${liveLabel()} · Engine ${props.tickMs} ms / jitter ${props.jitterUs} us / ${props.packetBytes} B · DMX ${props.dmxSuccessCount}/${props.dmxOutputCount}`}
            data-tauri-drag-region
          >
            {liveLabel()}
          </span>
        </div>
        <WindowControls />
      </header>
      <Show when={props.workspaceTab === "control"}>
        <nav
          class="controlModeTabs contextModeTabs editDomainNavigation"
          aria-label="Edit domain"
          data-edit-domain-navigation
        >
          <For each={editDomainModes}>
            {(mode) => {
              const active = () => mode.id === "edit"
                ? props.controlMode === "edit" || props.controlMode === "live"
                : props.controlMode === mode.id;
              const shortcut = mode.id === "edit" ? "E" : "M";
              return (
                <button
                  class={active() ? "active" : ""}
                  data-control-mode-option={mode.id}
                  data-no-localize
                  title={`${mode.label}: ${mode.description}`}
                  aria-label={mode.label}
                  aria-keyshortcuts={shortcut}
                  aria-pressed={active()}
                  disabled={props.operatorLockMode === "Partial" && mode.id === "edit"}
                  onClick={() => props.onControlMode(mode.id)}
                >
                  {mode.label}
                </button>
              );
            }}
          </For>
        </nav>
      </Show>
      <Show when={props.controlLearnMode}>
        {(mode) => (
          <div
            class="controlLearnPrompt"
            classList={{ waiting: props.controlLearnBusy }}
            role="status"
            aria-live="polite"
            data-control-learn-prompt={mode()}
          >
            <strong>{mode().toUpperCase()} Learn</strong>
            <Show when={props.controlLearnBusy} fallback={
              <span>Select a pink control, then move or press the hardware control</span>
            }>
              <span>Listening for input</span>
              <b data-no-localize>{props.controlLearnTargetLabel ?? "selected control"}</b>
            </Show>
            <kbd>Esc</kbd>
          </div>
        )}
      </Show>

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
          <Show when={activeSetupArea() !== "io"}>
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
          </Show>
        </div>
      </Show>
    </div>
  );
}
