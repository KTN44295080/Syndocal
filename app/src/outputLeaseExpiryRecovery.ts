import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import {
  executeObservedExpiredVideoLeaseRelinquish,
  queryVideoAuthoringLeaseAuthority,
  type OutputControlFence,
  type OutputLeaseAuthority,
} from "./outputControlController";

export interface ExpiredVideoLease {
  authority: OutputLeaseAuthority;
  fence: OutputControlFence;
}

/** Discovery remains read-only; an expired claim is never advertised as active. */
export async function queryExpiredVideoOutputLease(invoke: FrontendTauriInvoke): Promise<ExpiredVideoLease | null> {
  const query = await queryVideoAuthoringLeaseAuthority(invoke);
  if (query.status !== "expired" || !query.projectMatches || query.managed
    || query.resources.length !== 1 || query.resources[0] !== "video" || !query.authority) return null;
  return { authority: { ...query.authority }, fence: { ...query.fence } };
}

const projectKeys = ["process_incarnation", "session_incarnation", "project_epoch", "project_revision",
  "project_checkpoint_hash", "project_publication_generation"] as const;
const matchesObservation = (current: ExpiredVideoLease | null, expected: ExpiredVideoLease): current is ExpiredVideoLease =>
  current !== null && current.authority.lease_id === expected.authority.lease_id
  && current.authority.generation === expected.authority.generation
  && projectKeys.every(key => current.fence[key] === expected.fence[key]);
const record = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === "object";
const counter = (value: unknown): value is number => typeof value === "number"
  && Number.isSafeInteger(value) && value >= 0 && value < Number.MAX_SAFE_INTEGER;

/** Explicit operator cleanup. Stop physical owners before retiring their claim;
 * remain Standby even on success and require a separate explicit Enable. */
export async function releaseExpiredVideoOutputLease(invoke: FrontendTauriInvoke, expected: ExpiredVideoLease) {
  const before = await queryExpiredVideoOutputLease(invoke);
  if (!matchesObservation(before, expected)) throw new Error("The expired lease changed; refresh before releasing it.");
  const standby = await invoke<unknown>("set_output_ownership_role", { role: "Standby" });
  if (!record(standby) || standby.role !== "Standby" || standby.effective_role !== "Standby"
    || standby.desired_role !== "Standby" || standby.persisted_role !== "Standby" || standby.state !== "Ready"
    || standby.lighting_allowed !== false || standby.video_allowed !== false || standby.error !== null
    || !counter(standby.epoch) || !counter(standby.generation)) {
    throw new Error("Standby could not be confirmed; the expired lease was not released.");
  }
  const after = await queryExpiredVideoOutputLease(invoke);
  if (!matchesObservation(after, before)
    // Canonical query counters project the engine's zero-based counters as +1.
    || after.fence.output_epoch !== standby.epoch + 1
    || after.fence.output_generation !== standby.generation + 1) {
    throw new Error("Output authority changed after Standby; refresh before releasing the expired lease.");
  }
  const receipt = await executeObservedExpiredVideoLeaseRelinquish(invoke, after.authority, after.fence);
  const result = receipt.lease_result;
  const change = result?.changes[0];
  if (!result || result.resources.length !== 1 || result.resources[0] !== "video"
    || result.phase !== "unclaimed" || result.outcome !== "relinquished"
    || result.authority.lease_id !== after.authority.lease_id
    || result.authority.generation !== after.authority.generation + 1
    || result.changes.length !== 1 || change?.lease_id !== after.authority.lease_id
    || change.before_generation !== after.authority.generation || change.after_generation !== after.authority.generation + 1
    || change.before_phase !== "held_active" || change.after_phase !== "unclaimed"
    || change.before_resources.length !== 1 || change.before_resources[0] !== "video" || change.after_resources.length !== 0) {
    throw new Error("Expired lease release could not be verified; outputs remain in Standby.");
  }
  return receipt;
}
