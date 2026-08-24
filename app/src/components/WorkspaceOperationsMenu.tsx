import { createMemo, createSignal, For, Show } from "solid-js";
import type { OperatorLockMode, OperatorPolicy } from "../types";
import { paneWindowKinds, type NamedWorkspaceProfile, type PaneWindowKind } from "../workspaceProfiles";

export type PaneWindowOperationPhase = "opening" | "closing";

type WorkspaceOperationsMenuProps = {
  profiles: NamedWorkspaceProfile[];
  selectedProfileId: string | null;
  poppedPanes: PaneWindowKind[];
  paneTransitions: Partial<Record<PaneWindowKind, PaneWindowOperationPhase>>;
  workspaceBusy: boolean;
  operatorPolicy: OperatorPolicy | null;
  operatorLockMode: OperatorLockMode | null;
  onSelectProfile: (id: string | null) => void;
  onSaveProfile: (name: string) => Promise<void>;
  onApplyProfile: (profile: NamedWorkspaceProfile) => Promise<void>;
  onDeleteProfile: (profile: NamedWorkspaceProfile) => void;
  onTogglePane: (pane: PaneWindowKind) => void;
  onConfigurePolicy: (password: string, mode: OperatorLockMode, lockOnLoad: boolean) => Promise<boolean>;
  onClearPolicy: () => Promise<boolean>;
  onLock: (mode: OperatorLockMode) => void;
  onUnlock: (password: string) => Promise<boolean>;
};

const paneLabels: Record<PaneWindowKind, string> = {
  stage: "Stage",
  timeline: "Timeline",
  programmer: "Programmer",
  setup: "Setup",
  live: "Live",
  mixer: "Mixer",
  touch: "Touch",
};

export function WorkspaceOperationsMenu(props: WorkspaceOperationsMenuProps) {
  const [open, setOpen] = createSignal(false);
  const [workspaceName, setWorkspaceName] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [confirmation, setConfirmation] = createSignal("");
  const [unlockPassword, setUnlockPassword] = createSignal("");
  const [policyMode, setPolicyMode] = createSignal<OperatorLockMode>("Partial");
  const [lockOnLoad, setLockOnLoad] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const selectedProfile = createMemo(() =>
    props.profiles.find((profile) => profile.id === props.selectedProfileId) ?? null,
  );
  const workspaceTransitionBusy = createMemo(
    () => props.workspaceBusy || Object.keys(props.paneTransitions).length > 0,
  );
  const paneTransitionBusy = (pane: PaneWindowKind) => Boolean(props.paneTransitions[pane]);

  const configure = async () => {
    if (password() !== confirmation()) return;
    setBusy(true);
    try {
      if (await props.onConfigurePolicy(password(), policyMode(), lockOnLoad())) {
        setPassword("");
        setConfirmation("");
      }
    } finally {
      setBusy(false);
    }
  };

  const unlock = async () => {
    setBusy(true);
    try {
      if (await props.onUnlock(unlockPassword())) {
        setUnlockPassword("");
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div class="workspaceOperationsMenu" data-workspace-operations>
      <button
        type="button"
        class={props.operatorLockMode ? "workspaceOperationsButton locked" : "workspaceOperationsButton"}
        aria-expanded={open()}
        aria-haspopup="dialog"
        onClick={() => setOpen((value) => !value)}
      >
        {props.operatorLockMode ? `${props.operatorLockMode} Lock` : "Workspaces"}
      </button>
      <Show when={open()}>
        <div class="workspaceOperationsPopover" role="dialog" aria-label="Workspace and operator controls">
          <section>
            <header>
              <strong>Named Workspaces</strong>
              <small>Local to this device</small>
            </header>
            <div class="workspaceProfileRow">
              <input
                value={workspaceName()}
                maxlength={48}
                placeholder="Workspace name"
                aria-label="Workspace name"
                disabled={props.operatorLockMode !== null}
                onInput={(event) => setWorkspaceName(event.currentTarget.value)}
              />
              <button
                type="button"
                disabled={props.operatorLockMode !== null || workspaceTransitionBusy() || workspaceName().trim().length === 0}
                onClick={() => void props.onSaveProfile(workspaceName())}
              >
                Save Current
              </button>
            </div>
            <div class="workspaceProfileRow">
              <select
                aria-label="Saved workspace"
                value={props.selectedProfileId ?? ""}
                disabled={props.profiles.length === 0 || props.operatorLockMode !== null || workspaceTransitionBusy()}
                onChange={(event) => props.onSelectProfile(event.currentTarget.value || null)}
              >
                <option value="">Select workspace</option>
                <For each={props.profiles}>{(profile) => <option value={profile.id} data-no-localize>{profile.name}</option>}</For>
              </select>
              <button
                type="button"
                aria-busy={props.workspaceBusy ? "true" : undefined}
                disabled={!selectedProfile() || props.operatorLockMode !== null || workspaceTransitionBusy()}
                onClick={() => selectedProfile() && void props.onApplyProfile(selectedProfile()!)}
              >
                Apply
              </button>
              <button
                type="button"
                class="dangerSubtle"
                disabled={!selectedProfile() || props.operatorLockMode !== null || workspaceTransitionBusy()}
                onClick={() => selectedProfile() && props.onDeleteProfile(selectedProfile()!)}
              >
                Delete
              </button>
            </div>
          </section>

          <section>
            <header>
              <strong>Pane Windows</strong>
              <small>Position and size save with named workspaces</small>
            </header>
            <div class="workspacePaneButtons">
              <For each={paneWindowKinds}>
                {(pane) => (
                  <button
                    type="button"
                    data-workspace-pane-toggle={pane}
                    class={props.poppedPanes.includes(pane) ? "active" : ""}
                    aria-pressed={props.poppedPanes.includes(pane)}
                    aria-busy={props.operatorLockMode === null && (paneTransitionBusy(pane) || props.workspaceBusy) ? "true" : undefined}
                    disabled={props.operatorLockMode !== null || paneTransitionBusy(pane) || props.workspaceBusy}
                    onClick={() => props.onTogglePane(pane)}
                  >
                    {paneLabels[pane]}
                  </button>
                )}
              </For>
            </div>
          </section>

          <section class="operatorPolicySection">
            <header>
              <strong>Operator Lock</strong>
              <small>Show-operation guard, not operating-system security</small>
            </header>
            <Show
              when={props.operatorLockMode === null}
              fallback={
                <div class="operatorUnlockRow">
                  <span>{props.operatorLockMode} Lock is active.</span>
                  <input
                    type="password"
                    autocomplete="current-password"
                    value={unlockPassword()}
                    placeholder="Operator password"
                    aria-label="Operator password"
                    onInput={(event) => setUnlockPassword(event.currentTarget.value)}
                  />
                  <button type="button" disabled={busy() || unlockPassword().length === 0} onClick={() => void unlock()}>
                    Unlock
                  </button>
                </div>
              }
            >
              <Show when={props.operatorPolicy}>
                {(policy) => (
                  <div class="operatorPolicyStatus">
                    <span>{policy().lock_mode} Lock · {policy().lock_on_load ? "locks on project load" : "manual lock"}</span>
                    <button type="button" onClick={() => props.onLock(policy().lock_mode)}>Lock Now</button>
                    <button type="button" class="dangerSubtle" disabled={busy()} onClick={() => void props.onClearPolicy()}>
                      Remove Policy
                    </button>
                  </div>
                )}
              </Show>
              <div class="operatorPolicyForm">
                <select
                  aria-label="Operator lock mode"
                  value={policyMode()}
                  onChange={(event) => setPolicyMode(event.currentTarget.value as OperatorLockMode)}
                >
                  <option value="Partial">Partial · Live / Mixer / Touch</option>
                  <option value="Full">Full · Emergency blackout only</option>
                </select>
                <label>
                  <input type="checkbox" checked={lockOnLoad()} onChange={(event) => setLockOnLoad(event.currentTarget.checked)} />
                  Lock on project load
                </label>
                <input
                  type="password"
                  autocomplete="new-password"
                  value={password()}
                  placeholder="New password · 8+ characters"
                  aria-label="New operator password"
                  onInput={(event) => setPassword(event.currentTarget.value)}
                />
                <input
                  type="password"
                  autocomplete="new-password"
                  value={confirmation()}
                  placeholder="Confirm password"
                  aria-label="Confirm operator password"
                  onInput={(event) => setConfirmation(event.currentTarget.value)}
                />
                <button
                  type="button"
                  disabled={busy() || password().length < 8 || password() !== confirmation()}
                  onClick={() => void configure()}
                >
                  {props.operatorPolicy ? "Replace Policy" : "Set Policy"}
                </button>
              </div>
            </Show>
          </section>
        </div>
      </Show>
    </div>
  );
}
