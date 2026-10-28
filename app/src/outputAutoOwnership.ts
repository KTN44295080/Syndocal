import type { OutputLeaseAuthorityQuery } from "./outputControlController";
import type { OutputOwnershipStatus } from "./types";

/**
 * Automatic output ownership for a single primary machine.
 *
 * At app start and after a project replacement the backend leaves outputs
 * denied (`StartupDenied` / `ProjectSwapDisarmed`). On a primary machine this
 * policy runs the canonical local Enable (the same R4 OutputControl request as
 * "Enable Output" and stage 1 of DMX "Prepare"), so blackout, masters and live
 * output controls work without a manual step.
 *
 * It never takes ownership while another session holds a lease, while AI
 * access is shut off ("Revoke all"), on a backup machine, after Standby was
 * chosen, or while the backend reports a failed or busy transition. Every
 * hard boundary stays in the backend: the lease registry, the output-control
 * fence, the role-ordering guard and main-window close still decide; this
 * module only chooses whether to ask. It is dependency-free so the checker can
 * execute it directly.
 */

/** Machine-local switch (Setup > I/O). Absent or unreadable means on. */
export const AUTO_OUTPUT_OWNERSHIP_STORAGE_KEY = "syndocal.output.autoTake.v1";
/** Written by the Active / Standby panel when Standby sync starts. */
export const STANDBY_SYNC_ROLE_STORAGE_KEY = "syndocal.standbySyncRole";

export type AutoOutputSkipReason =
  | "setting_off"
  | "backup_computer"
  | "standby_sync_backup"
  | "split_brain"
  | "kill_switch"
  | "operator_standby"
  | "other_lease"
  | "busy"
  | "startup_failed"
  | "partial_role"
  | "transition_failed"
  | "status_unavailable"
  | "needs_operator";

export interface AutoOutputStandbySyncView {
  running: boolean;
  role: "primary" | "standby" | null;
  split_brain: boolean;
}

export type AutoOutputOwnershipView = Pick<
  OutputOwnershipStatus,
  "state" | "effective_role" | "desired_role" | "persisted_role" | "lighting_reason" | "video_reason" | "epoch" | "generation"
>;

export interface AutoOutputInputs {
  settingOn: boolean;
  rememberedStandbySyncRole: string | null;
  /** null: the status could not be read (fail closed). */
  standbySync: AutoOutputStandbySyncView | null;
  /** null: the status could not be read (fail closed). */
  killSwitchActive: boolean | null;
  /** Standby was chosen in this session and outputs were not explicitly taken since. */
  operatorStandby: boolean;
  ownership: AutoOutputOwnershipView;
  leases: OutputLeaseAuthorityQuery;
}

export type AutoOutputDecision =
  | { action: "take"; trigger: "start" | "project" }
  | { action: "owned" }
  | { action: "skip"; reason: AutoOutputSkipReason };

const isBothResources = (resources: readonly string[]): boolean =>
  resources.length === 2 && resources[0] === "lighting" && resources[1] === "video";

const ownsExactBothLease = (leases: OutputLeaseAuthorityQuery): boolean =>
  leases.statuses.filter((status) => status.status === "held_active" && isBothResources(status.resources)).length === 1;

/** No lease, or this machine's own orphaned Both lease (Enable recovers it, as Prepare does). */
const leaseFreeForEnable = (leases: OutputLeaseAuthorityQuery): boolean => {
  if (leases.statuses.length !== 1) return false;
  const only = leases.statuses[0];
  return only.status === "unavailable" || only.status === "held_orphaned" && isBothResources(only.resources);
};

const takeableRole = (role: OutputOwnershipStatus["desired_role"]): boolean =>
  role === "Both" || role === "Standby";

/**
 * Pure decision. Hard refusals are checked before anything that could ask the
 * backend to arm, so the reported reason is always the strongest one.
 */
export function decideAutoOutputOwnership(inputs: AutoOutputInputs): AutoOutputDecision {
  const { ownership, leases } = inputs;
  if (ownsExactBothLease(leases)) return { action: "owned" };
  if (!inputs.settingOn) return { action: "skip", reason: "setting_off" };
  if (inputs.rememberedStandbySyncRole === "standby") return { action: "skip", reason: "backup_computer" };
  if (!inputs.standbySync) return { action: "skip", reason: "status_unavailable" };
  if (inputs.standbySync.running && inputs.standbySync.role === "standby") {
    return { action: "skip", reason: "standby_sync_backup" };
  }
  if (inputs.standbySync.split_brain) return { action: "skip", reason: "split_brain" };
  if (inputs.killSwitchActive === null) return { action: "skip", reason: "status_unavailable" };
  if (inputs.killSwitchActive) return { action: "skip", reason: "kill_switch" };
  if (inputs.operatorStandby) return { action: "skip", reason: "operator_standby" };
  if (!leaseFreeForEnable(leases)) return { action: "skip", reason: "other_lease" };
  if (ownership.state === "Transitioning" || ownership.state === "Activating") {
    return { action: "skip", reason: "busy" };
  }
  const reason = ownership.lighting_reason;
  if (reason !== ownership.video_reason) return { action: "skip", reason: "needs_operator" };
  if (reason === "StartupDenied") {
    // A normal start publishes the stored preference as both desired and
    // persisted role. A startup failure leaves no stored role (or a different
    // one) and must stay off until the operator acts.
    if (ownership.state !== "Failed" || ownership.persisted_role === null
      || ownership.persisted_role !== ownership.desired_role) {
      return { action: "skip", reason: "startup_failed" };
    }
    if (!takeableRole(ownership.desired_role)) return { action: "skip", reason: "partial_role" };
    return { action: "take", trigger: "start" };
  }
  if (reason === "ProjectSwapDisarmed") {
    if (ownership.state !== "Ready" || ownership.effective_role !== "Standby") {
      return { action: "skip", reason: "needs_operator" };
    }
    if (!takeableRole(ownership.desired_role)) return { action: "skip", reason: "partial_role" };
    return { action: "take", trigger: "project" };
  }
  if (reason === "TransitionFailed") return { action: "skip", reason: "transition_failed" };
  if (reason === "BlockedByMachineRole" && ownership.effective_role === "Standby") {
    return { action: "skip", reason: "operator_standby" };
  }
  return { action: "skip", reason: "needs_operator" };
}

/** A machine that is in Standby by choice (any caller) suppresses later automatic takes. */
export const observedChosenStandby = (ownership: AutoOutputOwnershipView): boolean =>
  ownership.state === "Ready"
  && ownership.effective_role === "Standby"
  && ownership.desired_role === "Standby"
  && ownership.lighting_reason === "BlockedByMachineRole";

export type AutoOutputStatus =
  | { kind: "unknown" }
  | { kind: "owned" }
  | { kind: "taking" }
  | { kind: "skipped"; reason: AutoOutputSkipReason }
  | { kind: "failed"; detail: string };

export const AUTO_OUTPUT_SETTING_LABEL = "Take outputs automatically when Syndocal starts or opens a project";
export const AUTO_OUTPUT_SETTING_HINT = "Turn this off on a backup computer so it never sends output by itself.";
export const AUTO_OUTPUT_FAILED_PREFIX = "Outputs were not taken automatically:";

const skipMessages: Record<AutoOutputSkipReason, string> = {
  setting_off: "Automatic output is off. Use Enable Output to send outputs.",
  backup_computer: "Not taken automatically: this computer is set as the backup in Active / Standby.",
  standby_sync_backup: "Not taken automatically: this computer is following the main computer as the backup.",
  split_brain: "Not taken automatically: another computer is also acting as the main computer.",
  kill_switch: "Not taken automatically: AI access is shut off with Revoke all.",
  operator_standby: "Not taken automatically: Standby was chosen. Use Enable Output to send outputs again.",
  other_lease: "Not taken automatically: another session already controls the outputs.",
  busy: "Waiting for the current output change to finish.",
  startup_failed: "Not taken automatically: the output state could not be loaded at start. Use Enable Output.",
  partial_role: "Not taken automatically: this computer is set to send only lighting or only video.",
  transition_failed: "Not taken automatically: the last output change failed. Use Enable Output to try again.",
  status_unavailable: "Not taken automatically: the output state could not be read.",
  needs_operator: "Not taken automatically: use Enable Output to send outputs.",
};

export const AUTO_OUTPUT_OWNED_MESSAGE = "This computer is sending the outputs.";
export const AUTO_OUTPUT_TAKING_MESSAGE = "Taking the outputs…";

export function autoOutputStatusMessage(status: AutoOutputStatus): string | null {
  switch (status.kind) {
    case "unknown": return null;
    case "owned": return AUTO_OUTPUT_OWNED_MESSAGE;
    case "taking": return AUTO_OUTPUT_TAKING_MESSAGE;
    case "skipped": return skipMessages[status.reason];
    case "failed": return `${AUTO_OUTPUT_FAILED_PREFIX} ${status.detail}`;
  }
}

export const sameAutoOutputStatus = (a: AutoOutputStatus, b: AutoOutputStatus): boolean =>
  a.kind === b.kind
  && (a.kind !== "skipped" || b.kind !== "skipped" || a.reason === b.reason)
  && (a.kind !== "failed" || b.kind !== "failed" || a.detail === b.detail);

export function readAutoOutputSetting(storage: () => Pick<Storage, "getItem">): boolean {
  try {
    return storage().getItem(AUTO_OUTPUT_OWNERSHIP_STORAGE_KEY) !== "off";
  } catch {
    return true;
  }
}

export function writeAutoOutputSetting(storage: () => Pick<Storage, "setItem">, on: boolean): boolean {
  try {
    storage().setItem(AUTO_OUTPUT_OWNERSHIP_STORAGE_KEY, on ? "on" : "off");
    return true;
  } catch {
    return false;
  }
}

export function readRememberedStandbySyncRole(storage: () => Pick<Storage, "getItem">): string | null {
  try {
    return storage().getItem(STANDBY_SYNC_ROLE_STORAGE_KEY);
  } catch {
    // Unknown is treated as "not marked as backup"; the setting stays the
    // operator's explicit off switch.
    return null;
  }
}

/** Process-wide operator intent. Standby always wins over a later automatic take. */
const operatorIntent: {
  standbyGeneration: number;
  standbyChosen: boolean;
  /** Ownership counters at which the chosen Standby was observed applied. */
  standbyObservedAt: { epoch: number; generation: number } | null;
} = { standbyGeneration: 0, standbyChosen: false, standbyObservedAt: null };

const laterThan = (
  ownership: Pick<AutoOutputOwnershipView, "epoch" | "generation">,
  mark: { epoch: number; generation: number },
): boolean => ownership.epoch > mark.epoch || ownership.epoch === mark.epoch && ownership.generation > mark.generation;

/**
 * Track chosen Standby from the backend view as well, so Standby from any
 * caller suppresses later automatic takes. The suppression ends only when the
 * outputs are seen owned again at a later ownership generation than the
 * observed Standby (an explicit Enable, Arm, Prepare or Take Over); a stale
 * read taken before the Standby landed can never end it.
 */
function observeOperatorIntent(view: Pick<AutoOutputInputs, "ownership" | "leases">): void {
  if (observedChosenStandby(view.ownership)) {
    operatorIntent.standbyChosen = true;
    if (!operatorIntent.standbyObservedAt || laterThan(view.ownership, operatorIntent.standbyObservedAt)) {
      operatorIntent.standbyObservedAt = { epoch: view.ownership.epoch, generation: view.ownership.generation };
    }
  } else if (operatorIntent.standbyChosen && operatorIntent.standbyObservedAt
    && ownsExactBothLease(view.leases) && laterThan(view.ownership, operatorIntent.standbyObservedAt)) {
    operatorIntent.standbyChosen = false;
    operatorIntent.standbyObservedAt = null;
  }
}

/** Call synchronously when the operator (or a Standby flow) asks for Standby. */
export function noteOperatorStandbyRequested(): void {
  operatorIntent.standbyGeneration += 1;
  operatorIntent.standbyChosen = true;
  operatorIntent.standbyObservedAt = null;
}

/** Call after the operator explicitly took outputs again (Enable, Arm, Take Over, Prepare). */
export function noteOperatorTookOutputs(): void {
  operatorIntent.standbyChosen = false;
  operatorIntent.standbyObservedAt = null;
}

/** Test seam: a fresh process starts with no recorded Standby choice. */
export function resetOperatorIntentForTests(): void {
  operatorIntent.standbyGeneration = 0;
  operatorIntent.standbyChosen = false;
  operatorIntent.standbyObservedAt = null;
}

export const operatorStandbyChosen = (): boolean => operatorIntent.standbyChosen;
export const operatorStandbyGeneration = (): number => operatorIntent.standbyGeneration;

/** Transient refusals may be retried once more for the same backend state. */
const retriableRejection = (message: string): boolean =>
  /OutputControl rejected \((stale_fence|busy|overloaded)\)/.test(message)
  || /fence changed before|superseded|publication is active|retry/i.test(message);

export const AUTO_OUTPUT_ABORTED_BY_STANDBY = "Automatic output was cancelled because Standby was chosen; nothing was applied.";
export const AUTO_OUTPUT_ABORTED_BY_SETTING = "Automatic output was turned off; nothing was applied.";

export interface AutoOutputController {
  /** Read, decide and (when eligible) take. Joins an in-flight evaluation. */
  evaluate(): Promise<AutoOutputStatus>;
  /** Resolves when no automatic take is in flight. Never rejects. */
  settled(): Promise<void>;
  status(): AutoOutputStatus;
}

export function createOutputAutoOwnershipController(options: {
  /** Ownership and lease authority (the same reads the blackout pre-check uses). */
  readOwnership: () => Promise<Pick<AutoOutputInputs, "ownership" | "leases">>;
  /** Read only when outputs are not already owned. */
  readGuards: () => Promise<Pick<AutoOutputInputs, "standbySync" | "killSwitchActive" | "rememberedStandbySyncRole">>;
  readSetting: () => boolean;
  /**
   * Run the canonical Enable. `abortReason` is called after the output fence
   * was captured and immediately before dispatch; a non-null answer must abort
   * without sending anything.
   */
  enable: (abortReason: () => string | null) => Promise<unknown>;
  onStatus?: (status: AutoOutputStatus) => void;
  maxAttemptsPerState?: number;
}): AutoOutputController {
  const maxAttempts = Math.max(1, options.maxAttemptsPerState ?? 2);
  const attempts = new Map<string, number>();
  let current: AutoOutputStatus = { kind: "unknown" };
  let flight: Promise<AutoOutputStatus> | null = null;
  const publish = (next: AutoOutputStatus): AutoOutputStatus => {
    if (!sameAutoOutputStatus(current, next)) {
      current = next;
      options.onStatus?.(next);
    }
    return current;
  };
  const run = async (): Promise<AutoOutputStatus> => {
    let inputs: AutoOutputInputs;
    try {
      const view = await options.readOwnership();
      observeOperatorIntent(view);
      if (ownsExactBothLease(view.leases)) return publish({ kind: "owned" });
      const guards = await options.readGuards();
      inputs = { ...view, ...guards, settingOn: options.readSetting(), operatorStandby: operatorIntent.standbyChosen };
    } catch {
      return publish({ kind: "skipped", reason: "status_unavailable" });
    }
    const decision = decideAutoOutputOwnership(inputs);
    if (decision.action === "owned") return publish({ kind: "owned" });
    if (decision.action === "skip") {
      // Keep the concrete failure visible instead of the generic follow-up state.
      if (decision.reason === "transition_failed" && current.kind === "failed") return current;
      return publish({ kind: "skipped", reason: decision.reason });
    }
    const key = `${inputs.ownership.epoch}:${inputs.ownership.generation}:${inputs.ownership.lighting_reason}`;
    const used = attempts.get(key) ?? 0;
    if (used >= maxAttempts) return current;
    attempts.set(key, used + 1);
    const standbyGeneration = operatorIntent.standbyGeneration;
    publish({ kind: "taking" });
    try {
      await options.enable(() => {
        if (operatorIntent.standbyGeneration !== standbyGeneration || operatorIntent.standbyChosen) {
          return AUTO_OUTPUT_ABORTED_BY_STANDBY;
        }
        if (!options.readSetting()) return AUTO_OUTPUT_ABORTED_BY_SETTING;
        return null;
      });
      return publish({ kind: "owned" });
    } catch (error) {
      const detail = String(error instanceof Error ? error.message : error).replace(/^Error:\s*/i, "");
      if (detail === AUTO_OUTPUT_ABORTED_BY_STANDBY) return publish({ kind: "skipped", reason: "operator_standby" });
      if (detail === AUTO_OUTPUT_ABORTED_BY_SETTING) return publish({ kind: "skipped", reason: "setting_off" });
      if (!retriableRejection(detail)) attempts.set(key, maxAttempts);
      return publish({ kind: "failed", detail });
    }
  };
  return {
    evaluate() {
      if (flight) return flight;
      const next = run().finally(() => { if (flight === next) flight = null; });
      flight = next;
      return next;
    },
    settled() {
      return flight ? flight.then(() => undefined, () => undefined) : Promise.resolve();
    },
    status: () => current,
  };
}
