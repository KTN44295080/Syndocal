import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import {
  executeOutputControl,
  executeOutputLeaseLifecycle,
  queryOutputLeaseAuthority,
  selectOutputLeaseAuthority,
} from "../outputControlController";
import type {
  OutputControlChallengeNotice,
  OutputControlTargetRole,
  OutputLeaseAuthority,
  OutputLeaseAuthorityQuery,
  OutputLeaseAuthorityQueryHeld,
} from "../outputControlController";
import type { MachineOutputRole, OutputOwnershipStatus } from "../types";

type StandbySyncRole = "primary" | "standby";

interface StandbySyncStatus {
  running: boolean;
  role: StandbySyncRole | null;
  directory: string | null;
  session_id: string | null;
  generation: number | null;
  written_at_unix_ms: number | null;
  heartbeat_age_ms: number | null;
  heartbeat_stale: boolean;
  takeover_ready: boolean;
  split_brain: boolean;
  active_primary_sessions: string[];
  project_bytes: number;
  last_applied_generation: number | null;
  last_error: string | null;
}

const emptyStatus: StandbySyncStatus = {
  running: false,
  role: null,
  directory: null,
  session_id: null,
  generation: null,
  written_at_unix_ms: null,
  heartbeat_age_ms: null,
  heartbeat_stale: true,
  takeover_ready: false,
  split_brain: false,
  active_primary_sessions: [],
  project_bytes: 0,
  last_applied_generation: null,
  last_error: null,
};

const standbyDirectoryStorageKey = "syndocal.standbySyncDirectory";
const standbyRoleStorageKey = "syndocal.standbySyncRole";

const emptyOwnershipStatus: OutputOwnershipStatus = {
  role: "Standby",
  effective_role: "Standby",
  desired_role: "Standby",
  persisted_role: null,
  state: "Failed",
  generation: 0,
  epoch: 0,
  lighting_allowed: false,
  video_allowed: false,
  lighting_reason: "StartupDenied",
  video_reason: "StartupDenied",
  error: "Machine output ownership has not been initialized",
};

const emptyLeaseQuery: OutputLeaseAuthorityQuery = {
  operation_id: "syndocal.output.lease.authority.query.v1",
  statuses: [{ status: "unavailable" }],
};

function storedValue(key: string): string {
  try {
    return window.localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function formatHeartbeat(ageMs: number | null): string {
  if (ageMs === null) return "Waiting";
  if (ageMs < 1000) return `${ageMs} ms`;
  return `${(ageMs / 1000).toFixed(1)} s`;
}

function ownershipReasonLabel(reason: OutputOwnershipStatus["lighting_reason"]): string {
  switch (reason) {
    case "OwnedByMachineRole":
      return "Owned by this machine role";
    case "BlockedByMachineRole":
      return "Blocked by this machine role";
    case "Transitioning":
      return "Transition in progress";
    case "ProjectSwapDisarmed":
      return "Project changed — outputs disarmed";
    case "StartupDenied":
      return "Startup denied output ownership";
    case "TransitionFailed":
      return "Transition failed; outputs remain fenced";
  }
}

function ownershipStateLabel(state: OutputOwnershipStatus["state"]): string {
  switch (state) {
    case "Ready":
      return "Ready";
    case "Transitioning":
      return "Transitioning — outputs fenced";
    case "Activating":
      return "Activating — outputs fenced";
    case "Failed":
      return "Failed — outputs fenced";
  }
}

interface StandbySyncPanelProps {
  backendAvailable: boolean;
  invokeCommand: FrontendTauriInvoke;
}

export function StandbySyncPanel(props: StandbySyncPanelProps) {
  const initialRole = storedValue(standbyRoleStorageKey);
  const [directory, setDirectory] = createSignal(storedValue(standbyDirectoryStorageKey));
  const [role, setRole] = createSignal<StandbySyncRole>(initialRole === "standby" ? "standby" : "primary");
  const [status, setStatus] = createSignal<StandbySyncStatus>(emptyStatus);
  const [machineRole, setMachineRole] = createSignal<MachineOutputRole>("Standby");
  const [ownershipStatus, setOwnershipStatus] = createSignal<OutputOwnershipStatus>(emptyOwnershipStatus);
  const [leaseQuery, setLeaseQuery] = createSignal<OutputLeaseAuthorityQuery>(emptyLeaseQuery);
  const [selectedLeaseId, setSelectedLeaseId] = createSignal<string>("");
  const [busy, setBusy] = createSignal(false);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [forceConfirmed, setForceConfirmed] = createSignal(false);
  const [challengeNotice, setChallengeNotice] = createSignal<OutputControlChallengeNotice | null>(null);
  const [challengeClockUnixMs, setChallengeClockUnixMs] = createSignal(Date.now());
  let takeoverDialog!: HTMLDialogElement;

  const effectiveError = createMemo(() => actionError() ?? status().last_error);
  const activeRole = createMemo(() => status().role ?? role());
  const forceRequired = createMemo(() => !status().takeover_ready || status().split_brain);
  const heldLeases = createMemo(() => leaseQuery().statuses.filter(
    (candidate): candidate is OutputLeaseAuthorityQueryHeld => candidate.status !== "unavailable",
  ));
  const selectedLease = createMemo<OutputLeaseAuthority | null>(() => {
    const selected = heldLeases().find((candidate) => candidate.authority.lease_id === selectedLeaseId());
    return selected ? selected.authority : null;
  });
  const selectedLeaseStatus = createMemo<OutputLeaseAuthorityQueryHeld | null>(() =>
    heldLeases().find((candidate) => candidate.authority.lease_id === selectedLeaseId()) ?? null,
  );
  const challengeRemainingSeconds = createMemo(() => {
    const challenge = challengeNotice();
    if (!challenge) return 0;
    return Math.max(0, Math.ceil((challenge.expiresAtUnixMs - challengeClockUnixMs()) / 1000));
  });
  const statusLabel = createMemo(() => {
    const current = status();
    if (!props.backendAvailable) return "Desktop required";
    if (!current.running) return "Stopped";
    if (current.split_brain) return "Fenced";
    if (current.role === "primary") return current.last_error ? "Write error" : "Publishing";
    if (current.heartbeat_stale) return "Primary lost";
    if (current.last_applied_generation !== null) return "Warm standby";
    return "Waiting";
  });

  const pollStatus = async () => {
    try {
      const [syncStatus, outputStatus, outputLeases] = await Promise.all([
        props.invokeCommand<StandbySyncStatus>("standby_sync_status"),
        props.invokeCommand<OutputOwnershipStatus>("get_output_ownership_status"),
        queryOutputLeaseAuthority(props.invokeCommand),
      ]);
      setStatus(syncStatus);
      setOwnershipStatus(outputStatus);
      setMachineRole(outputStatus.desired_role);
      setLeaseQuery(outputLeases);
      if (!outputLeases.statuses.some((candidate) => candidate.status !== "unavailable" && candidate.authority.lease_id === selectedLeaseId())) {
        setSelectedLeaseId("");
      }
    } catch (error) {
      setActionError(String(error));
    }
  };

  onMount(() => {
    if (!props.backendAvailable) return;
    void pollStatus();
    const statusTimer = window.setInterval(() => void pollStatus(), 1000);
    const challengeTimer = window.setInterval(() => setChallengeClockUnixMs(Date.now()), 250);
    onCleanup(() => {
      window.clearInterval(statusTimer);
      window.clearInterval(challengeTimer);
    });
  });

  const chooseDirectory = async () => {
    setActionError(null);
    try {
      const selected = await props.invokeCommand<string | null>("select_standby_sync_directory");
      if (!selected) return;
      setDirectory(selected);
      window.localStorage.setItem(standbyDirectoryStorageKey, selected);
    } catch (error) {
      setActionError(String(error));
    }
  };

  const start = async () => {
    if (!directory().trim()) {
      setActionError("Select a shared synchronization folder before starting.");
      return;
    }
    setBusy(true);
    setActionError(null);
    try {
      window.localStorage.setItem(standbyDirectoryStorageKey, directory());
      window.localStorage.setItem(standbyRoleStorageKey, role());
      setStatus(await props.invokeCommand<StandbySyncStatus>("start_standby_sync", {
        directory: directory(),
        role: role(),
      }));
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await props.invokeCommand<StandbySyncStatus>("stop_standby_sync"));
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const changeMachineRole = async (nextRole: MachineOutputRole) => {
    setBusy(true);
    setActionError(null);
    setChallengeNotice(null);
    try {
      if (nextRole === "Standby") {
        // Standby is the safer-direction disarm path and remains available
        // without an output grant or physical release confirmation.
        const outputStatus = await props.invokeCommand<OutputOwnershipStatus>(
          "set_output_ownership_role",
          { role: nextRole },
        );
        setOwnershipStatus(outputStatus);
        setMachineRole(outputStatus.effective_role);
        return;
      }
      const role = nextRole === "Lighting" ? "lighting" : nextRole === "Video" ? "video" : "both";
      const lease = selectOutputLeaseAuthority(leaseQuery(), selectedLeaseId(), role === "lighting" ? ["lighting"] : role === "video" ? ["video"] : ["lighting", "video"]);
      await executeOutputControl(
        props.invokeCommand,
        { kind: "arm", role, lease },
        (notice) => setChallengeNotice(notice),
      );
      setActionError(null);
      // The terminal receipt is authoritative, but status polling remains the
      // source of the actual effective role after asynchronous route start.
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
      await pollStatus();
    } finally {
      setChallengeNotice(null);
      setBusy(false);
    }
  };

  const armMachineRole = async () => {
    setBusy(true);
    setActionError(null);
    setChallengeNotice(null);
    try {
      const requestedRole = machineRole();
      if (requestedRole === "Standby") {
        const outputStatus = await props.invokeCommand<OutputOwnershipStatus>(
          "arm_output_ownership_role",
        );
        setOwnershipStatus(outputStatus);
        setMachineRole(outputStatus.desired_role);
        return;
      }
      const role: OutputControlTargetRole = requestedRole === "Lighting" ? "lighting" : requestedRole === "Video" ? "video" : "both";
      const lease = selectOutputLeaseAuthority(leaseQuery(), selectedLeaseId(), role === "lighting" ? ["lighting"] : role === "video" ? ["video"] : ["lighting", "video"]);
      await executeOutputControl(
        props.invokeCommand,
        { kind: "arm", role, lease },
        (notice) => setChallengeNotice(notice),
      );
      setActionError(null);
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
      await pollStatus();
    } finally {
      setChallengeNotice(null);
      setBusy(false);
    }
  };

  const requestTakeOver = () => {
    setForceConfirmed(false);
    takeoverDialog.showModal();
  };

  const takeOver = async () => {
    setBusy(true);
    setActionError(null);
    setChallengeNotice(null);
    try {
      const current = status();
      if (!current.session_id || current.generation === null) {
        throw new Error("Standby Take Over checkpoint is unavailable; nothing was applied.");
      }
      const lease = selectOutputLeaseAuthority(leaseQuery(), selectedLeaseId(), ["lighting", "video"]);
      await executeOutputControl(
        props.invokeCommand,
        {
          kind: "take_over_standby",
          force: forceRequired(),
          standby_session_id: current.session_id,
          standby_generation: current.generation,
          lease,
        },
        (notice) => setChallengeNotice(notice),
      );
      setActionError(null);
      takeoverDialog.close();
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
    } finally {
      setChallengeNotice(null);
      setBusy(false);
    }
  };

  const selectedLifecycleLease = (): OutputLeaseAuthority => {
    const selected = selectedLease();
    if (!selected) throw new Error("Select one exact output lease before continuing; nothing was applied.");
    return { ...selected };
  };

  const runLeaseLifecycle = async (action: Parameters<typeof executeOutputLeaseLifecycle>[1]) => {
    setBusy(true);
    setActionError(null);
    setChallengeNotice(null);
    try {
      await executeOutputLeaseLifecycle(props.invokeCommand, action, (notice) => setChallengeNotice(notice));
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
      await pollStatus();
    } finally {
      setChallengeNotice(null);
      setBusy(false);
    }
  };

  const acquireLease = () => {
    const role = machineRole() === "Lighting" ? "lighting" : machineRole() === "Video" ? "video" : "both";
    return void runLeaseLifecycle({ kind: "acquire_lease", role });
  };
  const runSelectedLeaseLifecycle = (kind: "renew_lease" | "recover_lease" | "relinquish_output_lease") => {
    try {
      void runLeaseLifecycle({ kind, lease: selectedLifecycleLease() });
    } catch (error) {
      setActionError(String(error));
    }
  };
  const renewLease = () => runSelectedLeaseLifecycle("renew_lease");
  const recoverLease = () => runSelectedLeaseLifecycle("recover_lease");
  const relinquishLease = () => runSelectedLeaseLifecycle("relinquish_output_lease");

  return (
    <section class="standbySyncDesk">
      <header class="ioDeskHeader">
        <h2 class="textBalance">Active / Standby</h2>
        <span class="tabularNums">{statusLabel()}</span>
      </header>

      <p class="textPretty standbyIntro">
        Replicate complete show checkpoints through a dedicated shared folder. Output ownership stays local to this machine; Standby is fenced before it follows.
      </p>

      <div class="split">
        <label>
          Machine output role
          <select
            data-io-control="machine-output-role"
            aria-label="Machine output role"
            value={machineRole()}
            disabled={!props.backendAvailable || busy() || ownershipStatus().state === "Transitioning" || ownershipStatus().state === "Activating" || (status().running && status().role === "standby")}
            onChange={(event) => void changeMachineRole(event.currentTarget.value as MachineOutputRole)}
          >
            <option value="Lighting">Lighting</option>
            <option value="Video">Video</option>
            <option value="Both">Both</option>
            <option value="Standby">Standby</option>
          </select>
        </label>
        <div class="standbyModeSummary" aria-live="polite">
          <small>Output ownership</small>
          <strong>{machineRole()}</strong>
        </div>
      </div>

      <div class="buttonRow">
        <button
          data-io-control="arm-machine-output-role"
          aria-label="Arm selected machine output role"
          onClick={() => void armMachineRole()}
          disabled={!props.backendAvailable || busy() || ownershipStatus().state === "Transitioning" || ownershipStatus().state === "Activating" || (status().running && status().role === "standby")}
        >
          Arm selected role
        </button>
      </div>

      <dl class="standbyStatusGrid tabularNums" aria-label="Output ownership">
        <div>
          <dt>Effective role</dt>
          <dd>{ownershipStatus().effective_role} — {ownershipStateLabel(ownershipStatus().state)}</dd>
        </div>
        <div>
          <dt>Desired role</dt>
          <dd>{ownershipStatus().desired_role}</dd>
        </div>
        <div>
          <dt>Persisted role</dt>
          <dd>{ownershipStatus().persisted_role ?? "Not persisted"}</dd>
        </div>
        <div>
          <dt>Lighting output</dt>
          <dd>{ownershipStatus().lighting_allowed ? "Allowed" : "Blocked"} — {ownershipReasonLabel(ownershipStatus().lighting_reason)}</dd>
        </div>
        <div>
          <dt>Video output</dt>
          <dd>{ownershipStatus().video_allowed ? "Allowed" : "Blocked"} — {ownershipReasonLabel(ownershipStatus().video_reason)}</dd>
        </div>
        <div>
          <dt>Generation / epoch</dt>
          <dd>{ownershipStatus().generation} / {ownershipStatus().epoch}</dd>
        </div>
      </dl>
      <Show when={ownershipStatus().error}>
        <p class="fieldError textPretty" role="alert" aria-live="polite">{ownershipStatus().error}</p>
      </Show>

      <section class="standbyLeasePanel" aria-label="Output lease authority">
        <header class="ioDeskHeader">
          <h3 class="textBalance">Output lease</h3>
          <span class="tabularNums">
            {leaseQuery().statuses.length === 1 && leaseQuery().statuses[0].status === "unavailable"
              ? "Unavailable"
              : `${heldLeases().length} held`}
          </span>
        </header>
        <p class="textPretty">
          Lease IDs are backend-issued capabilities. Select one exact lease; lighting and video leases are never merged.
        </p>
        <Show when={heldLeases().length > 0} fallback={<p class="inlineWarning">Output lease state is Unavailable.</p>}>
          <div class="standbyStatusGrid tabularNums">
            <For each={heldLeases()}>{(lease) => (
              <div>
                <dt>{lease.status === "held_active" ? "Held active" : "Held orphaned"}</dt>
                <dd>{lease.authority.lease_id} · generation {lease.authority.generation} · {lease.resources.join(" + ")}</dd>
              </div>
            )}</For>
          </div>
          <label>
            Selected exact lease
            <select
              aria-label="Selected exact output lease"
              value={selectedLeaseId()}
              onChange={(event) => setSelectedLeaseId(event.currentTarget.value)}
              disabled={!props.backendAvailable || busy()}
            >
              <option value="">Select one lease</option>
              <For each={heldLeases()}>{(lease) => (
                <option value={lease.authority.lease_id}>
                  {lease.authority.lease_id} · {lease.status === "held_active" ? "active" : "orphaned"} · g{lease.authority.generation}
                </option>
              )}</For>
            </select>
          </label>
        </Show>
        <div class="buttonRow">
          <button onClick={acquireLease} disabled={!props.backendAvailable || busy()}>Acquire selected-role lease</button>
          <button onClick={renewLease} disabled={!props.backendAvailable || busy() || selectedLeaseStatus()?.status !== "held_active"}>Renew</button>
          <button onClick={recoverLease} disabled={!props.backendAvailable || busy() || selectedLeaseStatus()?.status !== "held_orphaned"}>Recover</button>
          <button onClick={relinquishLease} disabled={!props.backendAvailable || busy() || !selectedLeaseStatus()}>Relinquish</button>
        </div>
      </section>

      <Show when={challengeNotice()}>
        <aside class="standbyLeaseChallenge" aria-live="polite" aria-label="Backend physical confirmation">
          <strong>Backend physical confirmation required</strong>
          <span>Type challenge code <code>{challengeNotice()?.displayCode}</code> on the physical keyboard.</span>
          <small>{challengeRemainingSeconds()} s remaining. Only backend Raw Input counts; digits may also appear in the focused text field.</small>
        </aside>
      </Show>

      <label>
        Shared Folder
        <div class="standbyDirectoryRow">
          <input
            value={directory()}
            disabled={status().running}
            placeholder="Choose a local, UNC, or mounted shared folder"
            onInput={(event) => setDirectory(event.currentTarget.value)}
          />
          <button onClick={() => void chooseDirectory()} disabled={!props.backendAvailable || status().running || busy()}>Browse</button>
        </div>
      </label>

      <div class="split">
        <label>
          Role
          <select
            data-io-control="remote-standby-role"
            value={role()}
            disabled={status().running || busy()}
            onChange={(event) => setRole(event.currentTarget.value as StandbySyncRole)}
          >
            <option value="primary">Primary</option>
            <option value="standby">Standby</option>
          </select>
        </label>
        <div class="standbyModeSummary">
          <small>Safety mode</small>
          <strong>{activeRole() === "standby" ? "Outputs disarmed" : "Outputs unchanged"}</strong>
        </div>
      </div>

      <div class="buttonRow">
        <button class="primary" onClick={() => void start()} disabled={!props.backendAvailable || status().running || busy()}>
          Start Sync
        </button>
        <button onClick={() => void stop()} disabled={!props.backendAvailable || !status().running || busy()}>
          Stop Sync
        </button>
        <button
          class="danger"
          onClick={requestTakeOver}
          disabled={!props.backendAvailable || !status().running || status().role !== "standby" || ownershipStatus().effective_role !== "Standby" || ownershipStatus().state !== "Ready" || status().generation === null || busy()}
        >
          Take Over
        </button>
      </div>

      <Show when={status().running || status().generation !== null}>
        <dl class="standbyStatusGrid tabularNums">
          <div><dt>Generation</dt><dd>{status().generation ?? "—"}</dd></div>
          <div><dt>Heartbeat</dt><dd>{formatHeartbeat(status().heartbeat_age_ms)}</dd></div>
          <div><dt>Checkpoint</dt><dd>{formatBytes(status().project_bytes)}</dd></div>
          <div><dt>Applied</dt><dd>{status().last_applied_generation ?? "—"}</dd></div>
        </dl>
      </Show>

      <Show when={status().split_brain}>
        <p class="inlineWarning textPretty">
          Multiple Primary sessions are active ({status().active_primary_sessions.length}). Standby loading is fenced until only one remains.
        </p>
      </Show>
      <Show when={status().role === "standby" && status().heartbeat_stale && status().generation !== null && !status().split_brain}>
        <p class="inlineWarning textPretty">Primary heartbeat is stale. Verify network and Primary power before Take Over.</p>
      </Show>
      <Show when={effectiveError()}>
        <p class="fieldError textPretty" role="alert" aria-live="polite">{effectiveError()}</p>
      </Show>

      <dialog ref={takeoverDialog} class="standbyTakeoverDialog" aria-labelledby="standby-takeover-title">
        <form method="dialog">
          <h2 id="standby-takeover-title" class="textBalance">Keep outputs fenced during Take Over?</h2>
          <p class="textPretty">
            Take Over loads the latest replicated project while this machine remains Standby. Stop synchronization and choose an explicit machine role later before arming outputs.
          </p>
          <Show when={forceRequired()}>
            <label class="remoteLanToggle standbyFenceConfirmation">
              <input
                type="checkbox"
                checked={forceConfirmed()}
                onChange={(event) => setForceConfirmed(event.currentTarget.checked)}
              />
              I have fenced or disconnected every old Primary.
            </label>
          </Show>
          <p class="textPretty">
            The checkbox above is operator isolation confirmation. The backend physical confirmation challenge is shown separately and must also be completed.
          </p>
          <Show when={challengeNotice()}>
            <aside class="standbyLeaseChallenge" aria-live="polite" aria-label="Backend physical confirmation">
              <strong>Backend physical confirmation required</strong>
              <span>Type challenge code <code>{challengeNotice()?.displayCode}</code> on the physical keyboard.</span>
              <small>{challengeRemainingSeconds()} s remaining. Only backend Raw Input counts; digits may also appear in the focused text field.</small>
            </aside>
          </Show>
          <Show when={actionError()}>
            <p class="fieldError textPretty" role="alert">{actionError()}</p>
          </Show>
          <div class="buttonRow">
            <button value="cancel" disabled={busy()}>Cancel</button>
            <button
              type="button"
              class="danger"
              disabled={busy() || (forceRequired() && !forceConfirmed())}
              onClick={() => void takeOver()}
            >
              Confirm Take Over
            </button>
          </div>
        </form>
      </dialog>
    </section>
  );
}
