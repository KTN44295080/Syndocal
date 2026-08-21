import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import {
  enableOutput,
  executeOutputControl,
  executeOutputLeaseLifecycle,
  hasOnlyActiveOutputLease,
  queryOutputLeaseAuthority,
  selectOutputLeaseAuthority,
} from "../outputControlController";
import type {
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

const STATUS_POLL_INVOKE_TIMEOUT_MS = 2_000;
const STATUS_POLL_TIMEOUT_MS = 3_000;
const STATUS_POLL_INTERVAL_MS = 1_000;
const OUTPUT_ENABLE_TIMEOUT_MS = 3_000;

function boundedPromise<T>(
  factory: () => Promise<T>,
  label: string,
  timeoutMs: number,
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    let settled = false;
    const timer = window.setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error(`${label} timed out`));
    }, timeoutMs);
    Promise.resolve()
      .then(factory)
      .then(
        (value) => {
          if (settled) return;
          settled = true;
          window.clearTimeout(timer);
          resolve(value);
        },
        (error) => {
          if (settled) return;
          settled = true;
          window.clearTimeout(timer);
          reject(error);
        },
      );
  });
}

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
  let takeoverDialog!: HTMLDialogElement;
  let disposed = false;
  let pollGeneration = 0;
  let pollFlight: Promise<void> | null = null;
  let pollWaiter: Promise<void> | null = null;
  let enableGeneration = 0;
  let enableMutationFlight: ReturnType<typeof enableOutput> | null = null;
  let enableOutcomeUnknown = false;

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
  const outputEnabled = createMemo(() => {
    const current = ownershipStatus();
    return props.backendAvailable
      && current.state === "Ready"
      && current.effective_role === "Both"
      && current.desired_role === "Both"
      && current.lighting_allowed
      && current.video_allowed
      && hasOnlyActiveOutputLease(leaseQuery(), ["lighting", "video"]);
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

  const clearOutputAuthority = () => {
    setOwnershipStatus(emptyOwnershipStatus);
    setMachineRole("Standby");
    setLeaseQuery(emptyLeaseQuery);
    setSelectedLeaseId("");
  };

  const pollStatus = (): Promise<void> => {
    if (pollFlight) return pollWaiter ?? pollFlight;
    const generation = ++pollGeneration;
    const isCurrent = () => !disposed && generation === pollGeneration;
    const rawEndpointFlights = [
      Promise.resolve().then(() => props.invokeCommand<StandbySyncStatus>("standby_sync_status")),
      Promise.resolve().then(() => props.invokeCommand<OutputOwnershipStatus>("get_output_ownership_status")),
      Promise.resolve().then(() => queryOutputLeaseAuthority(props.invokeCommand)),
    ] as const;
    const rawFlight = (async () => {
      const results = await Promise.allSettled(rawEndpointFlights);
      const failure = results.find((result) => result.status === "rejected");
      if (failure?.status === "rejected") throw failure.reason;
      const [syncResult, ownershipResult, leaseResult] = results as [
        PromiseFulfilledResult<StandbySyncStatus>,
        PromiseFulfilledResult<OutputOwnershipStatus>,
        PromiseFulfilledResult<OutputLeaseAuthorityQuery>,
      ];
      if (!isCurrent()) return;
      setStatus(syncResult.value);
      setOwnershipStatus(ownershipResult.value);
      setMachineRole(ownershipResult.value.desired_role);
      setLeaseQuery(leaseResult.value);
      const authoritativeBothReady = props.backendAvailable
        && ownershipResult.value.state === "Ready"
        && ownershipResult.value.effective_role === "Both"
        && ownershipResult.value.desired_role === "Both"
        && ownershipResult.value.lighting_allowed
        && ownershipResult.value.video_allowed
        && hasOnlyActiveOutputLease(leaseResult.value, ["lighting", "video"]);
      if (enableOutcomeUnknown && authoritativeBothReady) {
        enableOutcomeUnknown = false;
        setActionError(null);
      }
      if (!leaseResult.value.statuses.some((candidate) => candidate.status !== "unavailable" && candidate.authority.lease_id === selectedLeaseId())) {
        setSelectedLeaseId("");
      }
    })();
    const boundedEndpointFlights = [
      boundedPromise(() => rawEndpointFlights[0], "Standby status query", STATUS_POLL_INVOKE_TIMEOUT_MS),
      boundedPromise(() => rawEndpointFlights[1], "Output ownership query", STATUS_POLL_INVOKE_TIMEOUT_MS),
      boundedPromise(() => rawEndpointFlights[2], "Output lease query", STATUS_POLL_INVOKE_TIMEOUT_MS),
    ] as const;
    let completedFlight: Promise<void>;
    completedFlight = rawFlight.then(
      () => undefined,
      (error) => {
        if (!isCurrent()) return;
        // A failed authoritative poll must not leave old output authority visible as enabled.
        clearOutputAuthority();
        if (!enableOutcomeUnknown) setActionError(String(error));
      },
    );
    let retainedFlight: Promise<void>;
    retainedFlight = completedFlight.finally(() => {
      if (pollFlight === retainedFlight) {
        pollFlight = null;
        pollWaiter = null;
      }
    });
    pollFlight = retainedFlight;
    const waiter = boundedPromise(
      async () => {
        const results = await Promise.allSettled(boundedEndpointFlights);
        const failure = results.find((result) => result.status === "rejected");
        if (failure?.status === "rejected") throw failure.reason;
      },
      "Output status refresh",
      STATUS_POLL_TIMEOUT_MS,
    ).catch((error) => {
      if (!isCurrent()) return;
      // The UI waiter may time out before rawFlight settles. Invalidate its
      // generation so a late backend result cannot restore stale authority.
      pollGeneration += 1;
      clearOutputAuthority();
      setActionError(String(error));
    });
    pollWaiter = waiter;
    return waiter;
  };

  onMount(() => {
    if (!props.backendAvailable) return;
    void pollStatus();
    const statusTimer = window.setInterval(() => {
      // A slow backend query must never create an overlapping poll train.
      if (!pollFlight && !enableMutationFlight) void pollStatus();
    }, STATUS_POLL_INTERVAL_MS);
    onCleanup(() => {
      disposed = true;
      pollGeneration += 1;
      window.clearInterval(statusTimer);
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
    try {
      if (nextRole === "Standby") {
        // Standby is the safer-direction disarm path and remains available
        // without an output grant or dangerous transfer confirmation.
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
      );
      setActionError(null);
      // The terminal receipt is authoritative, but status polling remains the
      // source of the actual effective role after asynchronous route start.
      await pollStatus();
    } catch (error) {
      clearOutputAuthority();
      setActionError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const armMachineRole = async () => {
    setBusy(true);
    setActionError(null);
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
      );
      setActionError(null);
      await pollStatus();
    } catch (error) {
      clearOutputAuthority();
      setActionError(String(error));
    } finally {
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
      );
      setActionError(null);
      takeoverDialog.close();
      await pollStatus();
    } catch (error) {
      setActionError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const beginEnableMutation = (): { generation: number; flight: ReturnType<typeof enableOutput> } => {
    if (enableMutationFlight) {
      throw new Error("Output enable is still in progress; final outcome is unknown. Wait before retrying.");
    }
    const generation = ++enableGeneration;
    const rawFlight = enableOutput(props.invokeCommand);
    let trackedFlight: ReturnType<typeof enableOutput>;
    trackedFlight = rawFlight.then(
      (receipt) => {
        if (enableMutationFlight === trackedFlight) enableMutationFlight = null;
        return receipt;
      },
      (error) => {
        if (enableMutationFlight === trackedFlight) enableMutationFlight = null;
        throw error;
      },
    );
    // The UI waiter below owns the user-visible error. Keep the raw mutation
    // rejection handled while its identity remains in-flight for dedupe.
    void trackedFlight.catch(() => undefined);
    enableMutationFlight = trackedFlight;
    return { generation, flight: trackedFlight };
  };

  const enableBothOutput = async () => {
    setBusy(true);
    if (!enableMutationFlight) enableOutcomeUnknown = false;
    setActionError(null);
    let generation: number | null = null;
    try {
      const mutation = beginEnableMutation();
      generation = mutation.generation;
      await boundedPromise(
        () => mutation.flight,
        "Output enable",
        OUTPUT_ENABLE_TIMEOUT_MS,
      );
      if (disposed || generation !== enableGeneration) return;
      enableOutcomeUnknown = false;
      // The terminal receipt is authoritative. Release the primary action
      // lock before the best-effort refresh so a slow query cannot freeze the
      // normal one-click path.
      setBusy(false);
      void pollStatus();
    } catch (error) {
      if (disposed || generation !== null && generation !== enableGeneration) return;
      // A timeout means the mutation is still retained underneath the UI
      // waiter; keep authority unavailable and block duplicate mutation until
      // that raw request settles.
      clearOutputAuthority();
      const timedOut = generation !== null && String(error).includes("timed out");
      if (timedOut) enableOutcomeUnknown = true;
      setActionError(timedOut
        ? "Output enable timed out; final outcome is unknown. Wait before retrying."
        : String(error));
    } finally {
      if (!disposed) setBusy(false);
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
    try {
      await executeOutputLeaseLifecycle(props.invokeCommand, action);
      await pollStatus();
    } catch (error) {
      clearOutputAuthority();
      setActionError(String(error));
    } finally {
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
      // A locally invalid lease selection is also fail-closed.
      setOwnershipStatus(emptyOwnershipStatus);
      setLeaseQuery(emptyLeaseQuery);
      setSelectedLeaseId("");
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

      <div class="buttonRow outputEnablePrimary">
        <button
          class="primary"
          data-io-control="enable-output"
          aria-label="Enable lighting and video output"
          onClick={() => void enableBothOutput()}
          disabled={!props.backendAvailable || busy() || ownershipStatus().state === "Transitioning" || ownershipStatus().state === "Activating" || outputEnabled()}
        >
          Enable Output
        </button>
        <small>Enable both lighting and video output in one step.</small>
      </div>
      <Show when={outputEnabled()}>
        <p class="outputEnableSuccess" role="status" aria-live="polite">Output enabled</p>
      </Show>

      <details class="advancedOutputControls">
        <summary>Advanced output and lease controls</summary>
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
          Advanced lease controls expose backend-issued authorities; normal Enable Output creates one exact lighting + video lease atomically.
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
      </details>

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
            The checkbox above is the explicit operator isolation confirmation required before this dangerous transfer.
          </p>
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
