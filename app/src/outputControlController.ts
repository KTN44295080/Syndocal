import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID =
  "syndocal.query.output.control.authority.v1";
export const OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID =
  "syndocal.output.lease.authority.query.v1";
export const OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID =
  "syndocal.query.output.display.add.authority.v1";
export const OUTPUT_OWNERSHIP_ARM_OPERATION_ID = "syndocal.output.ownership.arm.v2";
export const OUTPUT_BLACKOUT_SET_OPERATION_ID = "syndocal.output.blackout.set.v2";
export const OUTPUT_BLACKOUT_RELEASE_OPERATION_ID = "syndocal.output.blackout.release.v2";
export const OUTPUT_STANDBY_TAKEOVER_OPERATION_ID = "syndocal.output.standby.takeover.v2";
export const OUTPUT_DISPLAY_ADD_OPERATION_ID = "syndocal.output.display.add.v2";
export const OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID =
  "syndocal.output.display.window.set_open.v2";
export const OUTPUT_VIDEO_COMPOSITION_ASSIGN_OPERATION_ID =
  "syndocal.output.video.composition.assign.v2";
export const OUTPUT_ENABLE_OPERATION_ID = "syndocal.output.enable.v2";
export const OUTPUT_SHOW_ARTNET_LOOPBACK_ROUTE_ENABLE_OPERATION_ID =
  "syndocal.output.show_artnet_loopback_route.enable.v1";
export const OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_ENABLE_OPERATION_ID =
  "syndocal.output.show_serial_dmx_s0_route.enable.v1";
export const OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_STOP_OPERATION_ID =
  "syndocal.output.show_serial_dmx_s0_route.stop.v1";
export const OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_OPERATION_ID =
  "syndocal.output.dsf2026_artnet_acceptance_probe.send.v1";
export const OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_RECONCILE_OPERATION_ID =
  "syndocal.output.dsf2026_artnet_acceptance_probe.reconcile.v1";
export const OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID =
  "syndocal.query.output.dsf2026_artnet_acceptance_probe.status.v1";
export const OUTPUT_SHOW_SPOUT_OUTPUTS_ENABLE_OPERATION_ID =
  "syndocal.output.show_spout_outputs.enable.v2";
export const OUTPUT_SHOW_SPOUT_OUTPUTS_RESET_OPERATION_ID =
  "syndocal.output.show_spout_outputs.reset.v1";
export const OUTPUT_LEASE_ACQUIRE_OPERATION_ID = "syndocal.output.lease.acquire.v2";
export const OUTPUT_LEASE_RENEW_OPERATION_ID = "syndocal.output.lease.renew.v2";
export const OUTPUT_LEASE_RECOVER_OPERATION_ID = "syndocal.output.lease.recover.v2";
export const OUTPUT_LEASE_RELINQUISH_OPERATION_ID = "syndocal.output.lease.relinquish.v2";
export const OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID =
  "syndocal.output.lease.force_transfer.v2";

const MAX_LEASE_QUERY_STATUSES = 64;
const MAX_SAFE_REQUEST_ID = Number.MAX_SAFE_INTEGER;

export interface OutputControlFence {
  process_incarnation: number;
  session_incarnation: number;
  project_epoch: number;
  project_revision: number;
  project_checkpoint_hash: string;
  project_publication_generation: number;
  output_epoch: number;
  output_generation: number;
  safety_blackout_epoch: number;
  safety_blackout_generation: number;
}

/** The persisted project identity expected by a caller before a mutation. */
export interface OutputControlExpectedProject {
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
}

export type OutputControlTargetRole = "lighting" | "video" | "both";
export type OutputLeaseResource = "lighting" | "video";
export type OutputLeaseResources = readonly OutputLeaseResource[];

export interface OutputLeaseAuthority {
  lease_id: string;
  generation: number;
}

export interface OutputLeaseAuthorityQueryUnavailable {
  status: "unavailable";
}

export interface OutputLeaseAuthorityQueryHeld {
  status: "held_active" | "held_orphaned";
  authority: OutputLeaseAuthority;
  resources: OutputLeaseResource[];
}

export type OutputLeaseAuthorityQueryStatus =
  | OutputLeaseAuthorityQueryUnavailable
  | OutputLeaseAuthorityQueryHeld;

export interface OutputLeaseAuthorityQuery {
  operation_id: typeof OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID;
  statuses: OutputLeaseAuthorityQueryStatus[];
}

export type Dsf2026ArtNetAcceptanceProbeStatus = "available" | "in_doubt" | "consumed";

export interface Dsf2026ArtNetAcceptanceProbeStatusQuery {
  operationId: typeof OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID;
  status: Dsf2026ArtNetAcceptanceProbeStatus;
}

export type DisplayAddLeaseAuthorityStatus =
  | "held_active"
  | "expired_recoverable"
  | "held_orphaned"
  | "unavailable";

export interface DisplayAddLeaseAuthorityQuery {
  operationId: typeof OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID;
  status: DisplayAddLeaseAuthorityStatus;
  authority: OutputLeaseAuthority | null;
  resources: OutputLeaseResource[];
}

export interface DisplayOutputSpec {
  label: string;
  monitor_identity: string;
  monitor_index: number;
  width: number;
  height: number;
  fullscreen: boolean;
}

export type OutputDisplayAction = {
  kind: "add_display";
  spec: DisplayOutputSpec;
  lease: OutputLeaseAuthority;
};

/** The sole routine mutation for the native live Display window. */
export type OutputDisplayWindowAction = {
  kind: "set_display_window_open";
  output_id: number;
  open: boolean;
  lease: OutputLeaseAuthority;
};

/** The sole persisted assignment of one output to one composition. */
export type OutputVideoCompositionAssignmentAction = {
  kind: "assign_video_output_composition";
  output_id: number;
  composition_id: number;
  lease: OutputLeaseAuthority;
};

export type OutputEnableAction = { kind: "enable_output" };
/** The sole staged show route activation. It has no mutable route fields. */
export type OutputShowArtNetLoopbackRouteEnableAction = {
  kind: "enable_show_art_net_loopback_route";
  lease: OutputLeaseAuthority;
};
/** Fixed Open DMX U0 worker start; device identity remains machine-local. */
export type OutputShowSerialDmxSafetyBlackoutRouteEnableAction = {
  kind: "enable_show_serial_dmx_safety_blackout_route";
  lease: OutputLeaseAuthority;
};
/** Fixed Open DMX U0 worker stop; no generic route is accepted. */
export type OutputShowSerialDmxSafetyBlackoutRouteStopAction = {
  kind: "stop_show_serial_dmx_safety_blackout_route";
  lease: OutputLeaseAuthority;
};
/** A fixed, one-shot local ArtDmx proof frame; it accepts no mutable payload. */
export type OutputDsf2026ArtNetAcceptanceProbeAction = {
  kind: "send_dsf2026_artnet_acceptance_probe";
  lease: OutputLeaseAuthority;
};
/** Explicit no-send acknowledgement after independent physical reconciliation. */
export type OutputDsf2026ArtNetAcceptanceProbeReconcileAction = {
  kind: "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt";
  lease: OutputLeaseAuthority;
};
/** The fixed two-sender same-PC show output activation. */
export type OutputShowSpoutOutputsEnableAction = {
  kind: "enable_show_spout_outputs";
  lease: OutputLeaseAuthority;
};
/** Local R4 cleanup is intentionally payloadless and lease-free. */
export type OutputShowSpoutOutputsResetAction = {
  kind: "reset_show_spout_outputs";
};

export type OutputControlAction =
  | OutputEnableAction
  | OutputShowArtNetLoopbackRouteEnableAction
  | OutputShowSerialDmxSafetyBlackoutRouteEnableAction
  | OutputShowSerialDmxSafetyBlackoutRouteStopAction
  | OutputDsf2026ArtNetAcceptanceProbeAction
  | OutputDsf2026ArtNetAcceptanceProbeReconcileAction
  | OutputShowSpoutOutputsEnableAction
  | OutputShowSpoutOutputsResetAction
  | { kind: "arm"; role: OutputControlTargetRole; lease: OutputLeaseAuthority }
  | { kind: "set_blackout"; target: OutputControlTargetRole; enabled: boolean; lease: OutputLeaseAuthority }
  | { kind: "release_blackout"; lease: OutputLeaseAuthority }
  | {
      kind: "take_over_standby";
      force: boolean;
      standby_session_id: string;
      standby_generation: number;
      lease: OutputLeaseAuthority;
    }
  | OutputDisplayAction
  | OutputDisplayWindowAction
  | OutputVideoCompositionAssignmentAction;

export type OutputLeaseLifecycleAction =
  | { kind: "acquire_lease"; role: OutputControlTargetRole }
  | { kind: "renew_lease"; lease: OutputLeaseAuthority }
  | { kind: "recover_lease"; lease: OutputLeaseAuthority }
  | { kind: "relinquish_output_lease"; lease: OutputLeaseAuthority }
  | { kind: "force_transfer_lease"; lease: OutputLeaseAuthority };

export type OutputControlOperationAction = OutputControlAction | OutputLeaseLifecycleAction;

export type OutputLeaseReceiptPhase = "unclaimed" | "held_active" | "held_orphaned";
export type OutputLeaseReceiptOutcome =
  | "acquired"
  | "renewed"
  | "expiry_observed"
  | "recovered"
  | "relinquished"
  | "transferred"
  | "owner_retired"
  | "project_orphaned"
  | "authorized";

export interface OutputLeaseReceiptChange {
  lease_id: string;
  before_generation: number | null;
  after_generation: number | null;
  before_resources: OutputLeaseResource[];
  after_resources: OutputLeaseResource[];
  before_phase: OutputLeaseReceiptPhase | null;
  after_phase: OutputLeaseReceiptPhase | null;
}

export interface OutputControlLeaseResult {
  authority: OutputLeaseAuthority;
  resources: OutputLeaseResource[];
  phase: OutputLeaseReceiptPhase;
  outcome: OutputLeaseReceiptOutcome;
  audit_sequence: number;
  changes: OutputLeaseReceiptChange[];
}

export interface OutputControlReceipt {
  operation_id: string;
  request_id: number;
  shape_sha256: string;
  argument_fingerprint: string;
  audit_sequence: number;
  fence_before: OutputControlFence;
  fence_after: OutputControlFence;
  outcome: "applied" | "no_op";
  lease_result: OutputControlLeaseResult | null;
}

export const VIDEO_OUTPUT_WINDOW_STATE_EVENT = "syndocal://video-output-window-state";
export const VIDEO_OUTPUT_WINDOW_STATE_EVENT_SCHEMA = 1 as const;

/** Normalized shape of the backend's typed live-window event. */
export interface VideoOutputWindowStateEvent {
  schema: typeof VIDEO_OUTPUT_WINDOW_STATE_EVENT_SCHEMA;
  output_id: number;
  mode: "live";
  window_incarnation: number;
  actual_open: boolean;
  retirement_outcome: string | null;
  retirement_reason: string | null;
}

export interface VideoOutputWindowPhysicalState extends VideoOutputWindowStateEvent {
  observed_from: "event" | "query";
}

export interface VideoOutputWindowStateApplyResult {
  state: Readonly<Record<string, VideoOutputWindowPhysicalState>>;
  accepted: boolean;
  event: VideoOutputWindowStateEvent | null;
  reason: "applied" | "stale" | "malformed";
}

const videoWindowEventKeys = new Set([
  "schema", "schema_version", "schemaVersion", "output_id", "outputId", "mode",
  "window_incarnation", "live_window_incarnation", "windowIncarnation", "liveWindowIncarnation",
  "actual_open", "actualOpen", "retirement_outcome", "retirementOutcome",
  "retirement_reason", "retirementReason", "reason",
]);

const readEventAlias = (
  value: Record<string, unknown>,
  aliases: readonly string[],
): unknown => {
  const present = aliases.filter((key) => Object.prototype.hasOwnProperty.call(value, key));
  return present.length === 1 ? value[present[0]] : undefined;
};

const validWindowEventText = (value: unknown): value is string | null =>
  value === null || typeof value === "string" && value.length <= 1024;

/** Strictly validate and normalize either coordinated output-id spelling. */
export const parseVideoOutputWindowStateEvent = (
  value: unknown,
): VideoOutputWindowStateEvent | null => {
  if (!isObject(value)) return null;
  const keys = Object.keys(value);
  if (keys.some((key) => !videoWindowEventKeys.has(key))) return null;
  const schema = readEventAlias(value, ["schema", "schema_version", "schemaVersion"]);
  const outputId = readEventAlias(value, ["output_id", "outputId"]);
  const incarnation = readEventAlias(value, [
    "window_incarnation", "live_window_incarnation", "windowIncarnation", "liveWindowIncarnation",
  ]);
  const actualOpen = readEventAlias(value, ["actual_open", "actualOpen"]);
  const retirementOutcome = readEventAlias(value, ["retirement_outcome", "retirementOutcome"]);
  const retirementReason = readEventAlias(value, ["retirement_reason", "retirementReason", "reason"]);
  if (schema !== VIDEO_OUTPUT_WINDOW_STATE_EVENT_SCHEMA
    || !isPositiveSafeInteger(outputId)
    || value.mode !== "live"
    || !isPositiveSafeInteger(incarnation)
    || typeof actualOpen !== "boolean"
    || !validWindowEventText(retirementOutcome)
    || !validWindowEventText(retirementReason)) return null;
  return {
    schema: VIDEO_OUTPUT_WINDOW_STATE_EVENT_SCHEMA,
    output_id: outputId,
    mode: "live",
    window_incarnation: incarnation,
    actual_open: actualOpen,
    retirement_outcome: (retirementOutcome ?? null) as string | null,
    retirement_reason: (retirementReason ?? null) as string | null,
  };
};

/**
 * Apply only a matching-or-newer positive incarnation. This is deliberately a
 * pure reducer so event A arriving after newer event B cannot regress UI truth.
 */
export const applyVideoOutputWindowStateEvent = (
  previous: Readonly<Record<string, VideoOutputWindowPhysicalState>>,
  value: unknown,
): VideoOutputWindowStateApplyResult => {
  const event = parseVideoOutputWindowStateEvent(value);
  if (!event) return { state: previous, accepted: false, event: null, reason: "malformed" };
  const current = previous[String(event.output_id)];
  if (current && current.window_incarnation > event.window_incarnation) {
    return { state: previous, accepted: false, event, reason: "stale" };
  }
  return {
    state: {
      ...previous,
      [event.output_id]: { ...event, observed_from: "event" },
    },
    accepted: true,
    event,
    reason: "applied",
  };
};

export interface VideoOutputWindowStatusTruth {
  output_id: number;
  live_open: boolean;
  live_window_incarnation?: number | null;
}

/**
 * Resolve the normal Open/Close control from backend query truth. Incarnation
 * zero is the explicit process-start state for a configured Display that has
 * never created a native window; it is closed, not unknown. Positive event
 * truth always wins over a query captured before that event.
 */
export const resolveVideoOutputWindowActualOpen = (
  physical: VideoOutputWindowPhysicalState | undefined,
  status: VideoOutputWindowStatusTruth | null,
): boolean | null => {
  if (physical) return physical.actual_open;
  if (!status || typeof status.live_open !== "boolean") return null;
  const incarnation = status.live_window_incarnation;
  if (typeof incarnation !== "number" || !Number.isSafeInteger(incarnation) || incarnation < 0) return null;
  return status.live_open;
};

/** Apply a positive-incarnation reconciliation query without allowing an
 * equal-incarnation query captured earlier to overwrite a native event. */
export const applyVideoOutputWindowStatusQuery = (
  previous: Readonly<Record<string, VideoOutputWindowPhysicalState>>,
  status: VideoOutputWindowStatusTruth,
): VideoOutputWindowStateApplyResult => {
  const incarnation = status.live_window_incarnation;
  if (!isPositiveSafeInteger(status.output_id)
    || typeof status.live_open !== "boolean"
    || !isPositiveSafeInteger(incarnation)) {
    return { state: previous, accepted: false, event: null, reason: "malformed" };
  }
  const current = previous[String(status.output_id)];
  if (current && (current.window_incarnation > incarnation
    || current.window_incarnation === incarnation && current.observed_from === "event")) {
    return { state: previous, accepted: false, event: null, reason: "stale" };
  }
  const result = applyVideoOutputWindowStateEvent(previous, {
    schema: VIDEO_OUTPUT_WINDOW_STATE_EVENT_SCHEMA,
    output_id: status.output_id,
    mode: "live",
    window_incarnation: incarnation,
    actual_open: status.live_open,
    retirement_outcome: null,
    retirement_reason: null,
  });
  if (!result.accepted || !result.event) return result;
  return {
    ...result,
    state: {
      ...result.state,
      [status.output_id]: { ...result.state[String(status.output_id)], observed_from: "query" },
    },
  };
};

let nextOutputControlRequestId = 1;

const allocateRequestId = (): number => {
  if (nextOutputControlRequestId <= 0 || nextOutputControlRequestId > MAX_SAFE_REQUEST_ID) {
    throw new Error(
      "OutputControl request identity is exhausted; refresh the renderer session before retrying.",
    );
  }
  const requestId = nextOutputControlRequestId;
  nextOutputControlRequestId = requestId === MAX_SAFE_REQUEST_ID
    ? MAX_SAFE_REQUEST_ID + 1
    : requestId + 1;
  return requestId;
};

const operationIdForAction = (action: OutputControlOperationAction): string => {
  switch (action.kind) {
    case "enable_output": return OUTPUT_ENABLE_OPERATION_ID;
    case "enable_show_art_net_loopback_route": return OUTPUT_SHOW_ARTNET_LOOPBACK_ROUTE_ENABLE_OPERATION_ID;
    case "enable_show_serial_dmx_safety_blackout_route": return OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_ENABLE_OPERATION_ID;
    case "stop_show_serial_dmx_safety_blackout_route": return OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_STOP_OPERATION_ID;
    case "send_dsf2026_artnet_acceptance_probe": return OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_OPERATION_ID;
    case "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt": return OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_RECONCILE_OPERATION_ID;
    case "enable_show_spout_outputs": return OUTPUT_SHOW_SPOUT_OUTPUTS_ENABLE_OPERATION_ID;
    case "reset_show_spout_outputs": return OUTPUT_SHOW_SPOUT_OUTPUTS_RESET_OPERATION_ID;
    case "arm": return OUTPUT_OWNERSHIP_ARM_OPERATION_ID;
    case "set_blackout": return OUTPUT_BLACKOUT_SET_OPERATION_ID;
    case "release_blackout": return OUTPUT_BLACKOUT_RELEASE_OPERATION_ID;
    case "take_over_standby": return OUTPUT_STANDBY_TAKEOVER_OPERATION_ID;
    case "add_display": return OUTPUT_DISPLAY_ADD_OPERATION_ID;
    case "set_display_window_open": return OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID;
    case "assign_video_output_composition": return OUTPUT_VIDEO_COMPOSITION_ASSIGN_OPERATION_ID;
    case "acquire_lease": return OUTPUT_LEASE_ACQUIRE_OPERATION_ID;
    case "renew_lease": return OUTPUT_LEASE_RENEW_OPERATION_ID;
    case "recover_lease": return OUTPUT_LEASE_RECOVER_OPERATION_ID;
    case "relinquish_output_lease": return OUTPUT_LEASE_RELINQUISH_OPERATION_ID;
    case "force_transfer_lease": return OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID;
  }
};

type OutputControlInvokeCommand =
  | "enable_output_control_v2"
  | "enable_show_art_net_loopback_route_v1"
  | "enable_show_serial_dmx_safety_blackout_route_v1"
  | "stop_show_serial_dmx_safety_blackout_route_v1"
  | "send_dsf2026_artnet_acceptance_probe_v1"
  | "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1"
  | "enable_show_spout_outputs_v2"
  | "reset_show_spout_outputs_v1"
  | "arm_output_control_v2"
  | "set_blackout_output_control_v2"
  | "release_blackout_output_control_v2"
  | "take_over_output_control_v2"
  | "add_display_output_v2"
  | "set_display_output_window_open_v2"
  | "assign_video_output_composition_v2"
  | "acquire_output_lease_v2"
  | "renew_output_lease_v2"
  | "recover_output_lease_v2"
  | "relinquish_output_lease_v2"
  | "force_transfer_output_lease_v2";

const commandForAction = (action: OutputControlOperationAction): OutputControlInvokeCommand => {
  switch (action.kind) {
    case "enable_output": return "enable_output_control_v2";
    case "enable_show_art_net_loopback_route": return "enable_show_art_net_loopback_route_v1";
    case "enable_show_serial_dmx_safety_blackout_route": return "enable_show_serial_dmx_safety_blackout_route_v1";
    case "stop_show_serial_dmx_safety_blackout_route": return "stop_show_serial_dmx_safety_blackout_route_v1";
    case "send_dsf2026_artnet_acceptance_probe": return "send_dsf2026_artnet_acceptance_probe_v1";
    case "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt": return "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1";
    case "enable_show_spout_outputs": return "enable_show_spout_outputs_v2";
    case "reset_show_spout_outputs": return "reset_show_spout_outputs_v1";
    case "arm": return "arm_output_control_v2";
    case "set_blackout": return "set_blackout_output_control_v2";
    case "release_blackout": return "release_blackout_output_control_v2";
    case "take_over_standby": return "take_over_output_control_v2";
    case "add_display": return "add_display_output_v2";
    case "set_display_window_open": return "set_display_output_window_open_v2";
    case "assign_video_output_composition": return "assign_video_output_composition_v2";
    case "acquire_lease": return "acquire_output_lease_v2";
    case "renew_lease": return "renew_output_lease_v2";
    case "recover_lease": return "recover_output_lease_v2";
    case "relinquish_output_lease": return "relinquish_output_lease_v2";
    case "force_transfer_lease": return "force_transfer_output_lease_v2";
  }
};

const isObject = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object";
const isPositiveSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;
const isNonnegativeSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const isLowerHexSha256 = (value: unknown): value is string =>
  typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
const isLeaseId = (value: unknown): value is string =>
  typeof value === "string" && /^lease-[0-9a-f]{16}$/.test(value)
    && !/^lease-0{16}$/.test(value);
const hasExactKeys = (value: Record<string, unknown>, expected: readonly string[]): boolean => {
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  return actual.length === required.length && actual.every((key, index) => key === required[index]);
};

const outputControlFenceKeys = [
  "process_incarnation", "session_incarnation", "project_epoch", "project_revision",
  "project_checkpoint_hash", "project_publication_generation", "output_epoch",
  "output_generation", "safety_blackout_epoch", "safety_blackout_generation",
] as const;

const assertFence = (value: unknown, errorMessage: string): OutputControlFence => {
  if (!isObject(value) || !hasExactKeys(value, outputControlFenceKeys)
    || !isPositiveSafeInteger(value.process_incarnation)
    || !isPositiveSafeInteger(value.session_incarnation)
    || !isNonnegativeSafeInteger(value.project_epoch)
    || !isNonnegativeSafeInteger(value.project_revision)
    || !isLowerHexSha256(value.project_checkpoint_hash)
    || !isNonnegativeSafeInteger(value.project_publication_generation)
    || !isPositiveSafeInteger(value.output_epoch)
    || !isPositiveSafeInteger(value.output_generation)
    || !isPositiveSafeInteger(value.safety_blackout_epoch)
    || !isPositiveSafeInteger(value.safety_blackout_generation)) throw new Error(errorMessage);
  return value as unknown as OutputControlFence;
};

const fencesEqual = (left: OutputControlFence, right: OutputControlFence): boolean =>
  left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation
  && left.output_epoch === right.output_epoch
  && left.output_generation === right.output_generation
  && left.safety_blackout_epoch === right.safety_blackout_epoch
  && left.safety_blackout_generation === right.safety_blackout_generation;

const assertLeaseAuthority = (value: unknown, errorMessage: string): OutputLeaseAuthority => {
  if (!isObject(value) || !hasExactKeys(value, ["lease_id", "generation"])
    || !isLeaseId(value.lease_id) || !isPositiveSafeInteger(value.generation)) throw new Error(errorMessage);
  return value as unknown as OutputLeaseAuthority;
};

const assertResources = (value: unknown, errorMessage: string): OutputLeaseResource[] => {
  if (!Array.isArray(value) || value.length === 0 || value.length > 2
    || value.some((resource) => resource !== "lighting" && resource !== "video")
    || value.some((resource, index) => value.indexOf(resource) !== index)
    || value.length === 2 && (value[0] !== "lighting" || value[1] !== "video")) throw new Error(errorMessage);
  return [...value] as OutputLeaseResource[];
};

const sameResources = (left: readonly string[], right: readonly string[]): boolean =>
  left.length === right.length && left.every((resource, index) => resource === right[index]);

const assertDisplayAddLeaseAuthorityQuery = (value: unknown): DisplayAddLeaseAuthorityQuery => {
  const errorMessage = "Display Add lease authority response was invalid; nothing was applied.";
  if (!isObject(value) || !hasExactKeys(value, ["operationId", "status", "authority", "resources"])
    || value.operationId !== OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID
    || value.status !== "held_active" && value.status !== "expired_recoverable"
      && value.status !== "held_orphaned" && value.status !== "unavailable"
    || !Array.isArray(value.resources)) {
    throw new Error(errorMessage);
  }
  if (value.status === "unavailable") {
    if (value.authority !== null || value.resources.length !== 0) throw new Error(errorMessage);
    return {
      operationId: OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
      status: "unavailable",
      authority: null,
      resources: [],
    };
  }
  if (value.authority === null || !sameResources(value.resources, ["lighting", "video"])) {
    throw new Error(errorMessage);
  }
  return {
    operationId: OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: value.status,
    authority: assertLeaseAuthority(value.authority, errorMessage),
    resources: ["lighting", "video"],
  };
};

const resourcesForRole = (role: OutputControlTargetRole): OutputLeaseResource[] => {
  switch (role) {
    case "lighting": return ["lighting"];
    case "video": return ["video"];
    case "both": return ["lighting", "video"];
  }
};

const assertOutputLeaseAuthorityQuery = (value: unknown): OutputLeaseAuthorityQuery => {
  const errorMessage = "Output lease authority query was invalid; refresh lease state before retrying.";
  if (!isObject(value) || !hasExactKeys(value, ["operation_id", "statuses"])
    || value.operation_id !== OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID
    || !Array.isArray(value.statuses) || value.statuses.length === 0
    || value.statuses.length > MAX_LEASE_QUERY_STATUSES) throw new Error(errorMessage);
  const statuses = value.statuses.map((candidate): OutputLeaseAuthorityQueryStatus => {
    if (!isObject(candidate)) throw new Error(errorMessage);
    if (candidate.status === "unavailable") {
      if (!hasExactKeys(candidate, ["status"])) throw new Error(errorMessage);
      return { status: "unavailable" };
    }
    if ((candidate.status !== "held_active" && candidate.status !== "held_orphaned")
      || !hasExactKeys(candidate, ["status", "authority", "resources"])) throw new Error(errorMessage);
    return {
      status: candidate.status,
      authority: assertLeaseAuthority(candidate.authority, errorMessage),
      resources: assertResources(candidate.resources, errorMessage),
    };
  });
  if (statuses.some((status) => status.status === "unavailable")) {
    if (statuses.length !== 1) throw new Error(errorMessage);
  } else {
    for (let index = 1; index < statuses.length; index += 1) {
      const previous = statuses[index - 1];
      const current = statuses[index];
      if (previous.status === "unavailable" || current.status === "unavailable"
        || previous.authority.lease_id >= current.authority.lease_id) throw new Error(errorMessage);
    }
  }
  return { operation_id: OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID, statuses };
};

export async function queryOutputLeaseAuthority(
  invoke: FrontendTauriInvoke,
): Promise<OutputLeaseAuthorityQuery> {
  return assertOutputLeaseAuthorityQuery(await invoke<unknown>("query_output_lease_authority_v1"));
}

const assertDsf2026ArtNetAcceptanceProbeStatusQuery = (
  value: unknown,
): Dsf2026ArtNetAcceptanceProbeStatusQuery => {
  if (!isObject(value)
    || !hasExactKeys(value, ["operationId", "status"])
    || value.operationId !== OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID
    || value.status !== "available" && value.status !== "in_doubt" && value.status !== "consumed") {
    throw new Error("DSF2026 Art-Net acceptance probe status was invalid; no probe was sent.");
  }
  return {
    operationId: OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
    status: value.status,
  };
};

/** Read-only durable status for the single fixed physical proof budget. */
export async function queryDsf2026ArtNetAcceptanceProbeStatus(
  invoke: FrontendTauriInvoke,
): Promise<Dsf2026ArtNetAcceptanceProbeStatusQuery> {
  return assertDsf2026ArtNetAcceptanceProbeStatusQuery(
    await invoke<unknown>("query_dsf2026_artnet_acceptance_probe_status_v1"),
  );
}

export async function queryDisplayAddLeaseAuthority(
  invoke: FrontendTauriInvoke,
): Promise<DisplayAddLeaseAuthorityQuery> {
  return assertDisplayAddLeaseAuthorityQuery(
    await invoke<unknown>("query_display_add_lease_authority_v1"),
  );
}

const activeStatuses = (query: OutputLeaseAuthorityQuery): OutputLeaseAuthorityQueryHeld[] =>
  query.statuses.filter((status): status is OutputLeaseAuthorityQueryHeld => status.status === "held_active");

export function selectOutputLeaseAuthority(
  query: OutputLeaseAuthorityQuery,
  leaseId: string | null,
  expectedResources?: OutputLeaseResources,
): OutputLeaseAuthority {
  if (query.statuses.length === 1 && query.statuses[0].status === "unavailable") {
    throw new Error("Output lease is unavailable; output was not applied. Refresh lease state.");
  }
  if (!leaseId || !isLeaseId(leaseId)) throw new Error("Select one exact active output lease before continuing; nothing was applied.");
  const matches = activeStatuses(query).filter((status) => status.authority.lease_id === leaseId);
  if (matches.length !== 1 || expectedResources && !sameResources(matches[0].resources, expectedResources)) {
    throw new Error("Selected output lease is stale, orphaned, or has the wrong resources; nothing was applied.");
  }
  return { ...matches[0].authority };
}

export function selectOnlyActiveOutputLease(
  query: OutputLeaseAuthorityQuery,
  expectedResources: OutputLeaseResources,
): OutputLeaseAuthority {
  const matches = activeStatuses(query).filter((status) => sameResources(status.resources, expectedResources));
  if (matches.length !== 1) throw new Error("Exactly one active output lease with the requested resources is required; nothing was applied.");
  return { ...matches[0].authority };
}

/**
 * Stable display-add adapter: the backend's durable Add path receives exactly
 * one Both authority that is either currently active or recoverable from the
 * same owner/project. The backend performs the orphan recovery and Add in one
 * durable candidate; the renderer never performs an Enable-then-Add sequence.
 */
export function selectExactBothLeaseForDisplayAdd(
  query: DisplayAddLeaseAuthorityQuery,
): OutputLeaseAuthority {
  const validated = assertDisplayAddLeaseAuthorityQuery(query);
  if (validated.status === "unavailable" || !validated.authority) {
    throw new Error("Exactly one active or recoverable Both output lease is required; nothing was applied.");
  }
  return { ...validated.authority };
}

export function hasOnlyActiveOutputLease(
  query: OutputLeaseAuthorityQuery,
  expectedResources: OutputLeaseResources,
): boolean {
  return activeStatuses(query).filter(
    (status) => sameResources(status.resources, expectedResources),
  ).length === 1;
}

const assertAction = (action: OutputControlOperationAction): void => {
  const record = action as unknown as Record<string, unknown>;
  if (action.kind === "enable_output") {
    if (!hasExactKeys(record, ["kind"])) throw new Error("Output enable action was invalid; nothing was applied.");
    return;
  }
  if (action.kind === "enable_show_art_net_loopback_route") {
    if (!hasExactKeys(record, ["kind", "lease"])) {
      throw new Error("Show Art-Net loopback route action was invalid; nothing was applied.");
    }
    assertLeaseAuthority(action.lease, "Show Art-Net loopback route lease was invalid; nothing was applied.");
    return;
  }
  if (action.kind === "enable_show_serial_dmx_safety_blackout_route"
    || action.kind === "stop_show_serial_dmx_safety_blackout_route") {
    if (!hasExactKeys(record, ["kind", "lease"])) {
      throw new Error("Show serial DMX safety worker action was invalid; nothing was applied.");
    }
    assertLeaseAuthority(action.lease, "Show serial DMX safety worker lease was invalid; nothing was applied.");
    return;
  }
  if (action.kind === "send_dsf2026_artnet_acceptance_probe") {
    if (!hasExactKeys(record, ["kind", "lease"])) {
      throw new Error("DSF2026 Art-Net acceptance probe action was invalid; nothing was sent.");
    }
    assertLeaseAuthority(action.lease, "DSF2026 Art-Net acceptance probe lease was invalid; nothing was sent.");
    return;
  }
  if (action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt") {
    if (!hasExactKeys(record, ["kind", "lease"])) {
      throw new Error("DSF2026 Art-Net acceptance probe reconciliation action was invalid; nothing was applied.");
    }
    assertLeaseAuthority(action.lease, "DSF2026 Art-Net acceptance probe reconciliation lease was invalid; nothing was applied.");
    return;
  }
  if (action.kind === "enable_show_spout_outputs") {
    if (!hasExactKeys(record, ["kind", "lease"])) {
      throw new Error("Show Spout output action was invalid; nothing was applied.");
    }
    assertLeaseAuthority(action.lease, "Show Spout output lease was invalid; nothing was applied.");
    return;
  }
  if (action.kind === "reset_show_spout_outputs") {
    if (!hasExactKeys(record, ["kind"])) {
      throw new Error("Show Spout reset action was invalid; nothing was applied.");
    }
    return;
  }
  if (action.kind === "acquire_lease") {
    if (!hasExactKeys(record, ["kind", "role"]) || !["lighting", "video", "both"].includes(action.role)) {
      throw new Error("Output lease acquire action was invalid; nothing was applied.");
    }
    return;
  }
  if (action.kind === "arm") {
    if (!hasExactKeys(record, ["kind", "role", "lease"]) || !["lighting", "video", "both"].includes(action.role)) {
      throw new Error("OutputControl arm action was invalid; nothing was applied.");
    }
  } else if (action.kind === "set_blackout") {
    if (!hasExactKeys(record, ["kind", "target", "enabled", "lease"])
      || !["lighting", "video", "both"].includes(action.target)
      || typeof action.enabled !== "boolean") throw new Error("Output blackout action was invalid; nothing was applied.");
  } else if (action.kind === "release_blackout") {
    if (!hasExactKeys(record, ["kind", "lease"])) throw new Error("OutputControl release action was invalid; nothing was applied.");
  } else if (action.kind === "take_over_standby") {
    if (!hasExactKeys(record, ["kind", "force", "standby_session_id", "standby_generation", "lease"])
      || typeof action.force !== "boolean" || !action.standby_session_id
      || !isPositiveSafeInteger(action.standby_generation)) throw new Error("OutputControl Take Over action was invalid; nothing was applied.");
  } else if (action.kind === "add_display") {
    const spec = action.spec as unknown as Record<string, unknown>;
    if (!hasExactKeys(record, ["kind", "spec", "lease"])
      || !isObject(spec)
      || !hasExactKeys(spec, ["label", "monitor_identity", "monitor_index", "width", "height", "fullscreen"])
      || typeof spec.label !== "string" || spec.label.trim().length === 0 || spec.label.length > 128
      || !/^[0-9a-f]{64}$/.test(String(spec.monitor_identity))
      || !isNonnegativeSafeInteger(spec.monitor_index) || spec.monitor_index > 255
      || !isPositiveSafeInteger(spec.width) || spec.width > 16_384
      || !isPositiveSafeInteger(spec.height) || spec.height > 16_384
      || typeof spec.fullscreen !== "boolean") {
      throw new Error("OutputControl display action was invalid; nothing was applied.");
    }
  } else if (action.kind === "set_display_window_open") {
    if (!hasExactKeys(record, ["kind", "output_id", "open", "lease"])
      || !isPositiveSafeInteger(action.output_id)
      || typeof action.open !== "boolean") {
      throw new Error("OutputControl Display window action was invalid; nothing was applied.");
    }
  } else if (action.kind === "assign_video_output_composition") {
    if (!hasExactKeys(record, ["kind", "output_id", "composition_id", "lease"])
      || !isPositiveSafeInteger(action.output_id)
      || !isPositiveSafeInteger(action.composition_id)) {
      throw new Error("OutputControl video composition assignment was invalid; nothing was applied.");
    }
  } else if (!hasExactKeys(record, ["kind", "lease"])) {
    throw new Error("Output lease lifecycle action was invalid; nothing was applied.");
  }
  assertLeaseAuthority(action.lease, "Output lease authority was invalid; nothing was applied.");
};

const assertAuthority = (value: unknown): { operation_id: string; fence: OutputControlFence } => {
  if (!isObject(value) || !hasExactKeys(value, ["operation_id", "fence"])
    || value.operation_id !== OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID) throw new Error("OutputControl authority response was invalid; nothing was applied.");
  return {
    operation_id: OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
    fence: assertFence(value.fence, "OutputControl authority response was invalid; nothing was applied."),
  };
};

const assertPhase = (value: unknown, errorMessage: string): OutputLeaseReceiptPhase => {
  if (value !== "unclaimed" && value !== "held_active" && value !== "held_orphaned") throw new Error(errorMessage);
  return value;
};
const assertOutcome = (value: unknown, errorMessage: string): OutputLeaseReceiptOutcome => {
  if (!["acquired", "renewed", "expiry_observed", "recovered", "relinquished", "transferred", "owner_retired", "project_orphaned", "authorized"].includes(value as string)) throw new Error(errorMessage);
  return value as OutputLeaseReceiptOutcome;
};

const assertLeaseResult = (value: unknown, action: OutputControlOperationAction): OutputControlLeaseResult => {
  if (action.kind === "reset_show_spout_outputs") {
    throw new Error("Show Spout reset must not carry output-lease state.");
  }
  const errorMessage = "OutputControl lease result was invalid; physical output state is unknown.";
  if (!isObject(value) || !hasExactKeys(value, ["authority", "resources", "phase", "outcome", "audit_sequence", "changes"])
    || !Array.isArray(value.changes) || value.changes.length !== 1 || !isPositiveSafeInteger(value.audit_sequence)) throw new Error(errorMessage);
  const authority = assertLeaseAuthority(value.authority, errorMessage);
  const resources = assertResources(value.resources, errorMessage);
  const phase = assertPhase(value.phase, errorMessage);
  const outcome = assertOutcome(value.outcome, errorMessage);
  const rawChange = value.changes[0];
  if (!isObject(rawChange) || !hasExactKeys(rawChange, [
    "lease_id", "before_generation", "after_generation", "before_resources", "after_resources", "before_phase", "after_phase",
  ]) || rawChange.lease_id !== authority.lease_id) throw new Error(errorMessage);
  const beforeGeneration = rawChange.before_generation === null ? null : isPositiveSafeInteger(rawChange.before_generation) ? rawChange.before_generation : undefined;
  const afterGeneration = rawChange.after_generation === null ? null : isPositiveSafeInteger(rawChange.after_generation) ? rawChange.after_generation : undefined;
  if (beforeGeneration === undefined || afterGeneration === undefined || beforeGeneration === null && afterGeneration === null) throw new Error(errorMessage);
  const beforePhase = rawChange.before_phase === null ? null : assertPhase(rawChange.before_phase, errorMessage);
  const afterPhase = rawChange.after_phase === null ? null : assertPhase(rawChange.after_phase, errorMessage);
  const beforeResources = beforeGeneration === null
    ? (Array.isArray(rawChange.before_resources) && rawChange.before_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
    : beforePhase === "unclaimed"
      ? (Array.isArray(rawChange.before_resources) && rawChange.before_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
      : assertResources(rawChange.before_resources, errorMessage);
  const afterResources = afterGeneration === null
    ? (Array.isArray(rawChange.after_resources) && rawChange.after_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
    : afterPhase === "unclaimed"
      ? (Array.isArray(rawChange.after_resources) && rawChange.after_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
      : assertResources(rawChange.after_resources, errorMessage);
  if (beforeGeneration !== null && beforePhase === null || beforeGeneration === null && beforePhase !== null
    || afterGeneration !== null && afterPhase === null || afterGeneration === null && afterPhase !== null) throw new Error(errorMessage);
  const terminalGeneration = afterGeneration ?? beforeGeneration;
  const terminalPhase = afterPhase ?? beforePhase;
  const terminalResources = outcome === "relinquished" ? beforeResources : afterResources;
  if (terminalGeneration !== authority.generation || terminalPhase !== phase || !sameResources(resources, terminalResources)) throw new Error(errorMessage);
  if (action.kind !== "acquire_lease" && action.kind !== "enable_output"
    && authority.lease_id !== action.lease.lease_id) throw new Error(errorMessage);
  switch (action.kind) {
    case "enable_output":
      if ((outcome !== "acquired" && outcome !== "recovered")
        || phase !== "held_active" || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      if (outcome === "acquired"
        && (beforeGeneration !== null || beforePhase !== null || beforeResources.length !== 0)) throw new Error(errorMessage);
      if (outcome === "recovered"
        && (beforeGeneration === null || afterGeneration === null || afterGeneration <= beforeGeneration
          || beforePhase !== "held_orphaned" || !sameResources(beforeResources, ["lighting", "video"]))) throw new Error(errorMessage);
      break;
    case "enable_show_art_net_loopback_route":
    case "enable_show_serial_dmx_safety_blackout_route":
    case "stop_show_serial_dmx_safety_blackout_route":
    case "send_dsf2026_artnet_acceptance_probe":
    case "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt":
    case "enable_show_spout_outputs":
      if (outcome !== "authorized" || phase !== "held_active"
        || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "arm":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, resourcesForRole(action.role))) throw new Error(errorMessage);
      break;
    case "set_blackout":
    case "release_blackout":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "take_over_standby":
      if (!(outcome === "authorized" && phase === "held_active" || outcome === "project_orphaned" && phase === "held_orphaned") || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "add_display":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "set_display_window_open":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "assign_video_output_composition":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "acquire_lease":
      if (outcome !== "acquired" || phase !== "held_active" || !sameResources(resources, resourcesForRole(action.role))) throw new Error(errorMessage);
      break;
    case "renew_lease":
      if (outcome !== "renewed" || phase !== "held_active") throw new Error(errorMessage);
      break;
    case "recover_lease":
      if (outcome !== "recovered" || phase !== "held_active") throw new Error(errorMessage);
      break;
    case "relinquish_output_lease":
      if (outcome !== "relinquished" || phase !== "unclaimed") throw new Error(errorMessage);
      break;
    case "force_transfer_lease":
      if (outcome !== "transferred" || phase !== "held_active") throw new Error(errorMessage);
      break;
  }
  return {
    authority,
    resources,
    phase,
    outcome,
    audit_sequence: value.audit_sequence,
    changes: [{
      lease_id: rawChange.lease_id,
      before_generation: beforeGeneration,
      after_generation: afterGeneration,
      before_resources: beforeResources,
      after_resources: afterResources,
      before_phase: beforePhase,
      after_phase: afterPhase,
    }],
  };
};

const knownPreActionRejections = new Set([
  "invalid_request", "forbidden", "stale_fence", "busy", "overloaded",
]);

const deepFreeze = <T>(value: T): T => {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child);
  }
  return value;
};

const assertResponse = (
  value: unknown,
  action: OutputControlOperationAction,
  operationId: string,
  requestId: number,
  expectedFence: OutputControlFence,
): OutputControlReceipt => {
  if (!isObject(value) || value.type !== "receipt" && value.type !== "rejected") throw new Error("OutputControl terminal response was invalid; physical output state is unknown.");
  if (!hasExactKeys(value, value.type === "receipt" ? ["type", "receipt"] : ["type", "rejection"])) throw new Error("OutputControl terminal response was invalid; physical output state is unknown.");
  const result = (value.type === "receipt" ? value.receipt : value.rejection) as Record<string, unknown> | undefined;
  if (!isObject(result) || result.operation_id !== operationId || result.request_id !== requestId) throw new Error("OutputControl terminal response identity did not match the request; physical output state is unknown.");
  if (value.type === "rejected") {
    if (!hasExactKeys(result, ["operation_id", "request_id", "error"]) || typeof result.error !== "string") throw new Error("OutputControl rejection was invalid; physical output state is unknown.");
    if (result.error === "forbidden") throw new Error(`OutputControl rejected (${result.error}); output was not applied; refresh lease state.`);
    if (knownPreActionRejections.has(result.error)) throw new Error(`OutputControl rejected (${result.error}); nothing was applied.`);
    throw new Error(`OutputControl rejected (${result.error}); physical output state is unknown.`);
  }
  if (!hasExactKeys(result, ["operation_id", "request_id", "shape_sha256", "argument_fingerprint", "audit_sequence", "fence_before", "fence_after", "outcome", "lease_result"])
    || !isLowerHexSha256(result.shape_sha256) || !isLowerHexSha256(result.argument_fingerprint)
    || !isPositiveSafeInteger(result.audit_sequence)
    || result.outcome !== "applied" && result.outcome !== "no_op") throw new Error("OutputControl receipt was invalid; physical output state is unknown.");
  const fenceBefore = assertFence(result.fence_before, "OutputControl receipt was invalid; physical output state is unknown.");
  const fenceAfter = assertFence(result.fence_after, "OutputControl receipt was invalid; physical output state is unknown.");
  const fenceUnchanged = fencesEqual(fenceBefore, fenceAfter);
  const spoutAppliedPersistedProjectFence = action.kind === "enable_show_spout_outputs"
    && result.outcome === "applied"
    && fenceAfter.process_incarnation === fenceBefore.process_incarnation
    && fenceAfter.session_incarnation === fenceBefore.session_incarnation
    && fenceAfter.project_epoch === fenceBefore.project_epoch
    && fenceAfter.project_revision === fenceBefore.project_revision + 1
    && fenceAfter.project_checkpoint_hash !== fenceBefore.project_checkpoint_hash
    && fenceAfter.project_publication_generation === fenceBefore.project_publication_generation + 1
    && fenceAfter.output_epoch === fenceBefore.output_epoch
    && fenceAfter.output_generation === fenceBefore.output_generation
    && fenceAfter.safety_blackout_epoch === fenceBefore.safety_blackout_epoch
    && fenceAfter.safety_blackout_generation === fenceBefore.safety_blackout_generation;
  const fenceUnchangedPhysicalAction = action.kind === "set_display_window_open"
    || action.kind === "enable_show_serial_dmx_safety_blackout_route"
    || action.kind === "stop_show_serial_dmx_safety_blackout_route"
    || action.kind === "send_dsf2026_artnet_acceptance_probe"
    || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt"
    || action.kind === "enable_show_spout_outputs";
  const persistedProjectFenceIsExact = (action.kind !== "enable_show_art_net_loopback_route" && action.kind !== "set_blackout")
    || result.outcome !== "applied"
    || fenceAfter.process_incarnation === fenceBefore.process_incarnation
      && fenceAfter.session_incarnation === fenceBefore.session_incarnation
      && fenceAfter.project_epoch === fenceBefore.project_epoch
      && fenceAfter.project_revision === fenceBefore.project_revision + 1
      && fenceAfter.project_checkpoint_hash !== fenceBefore.project_checkpoint_hash
      && fenceAfter.project_publication_generation === fenceBefore.project_publication_generation + 1
      && fenceAfter.output_epoch === fenceBefore.output_epoch
      && fenceAfter.output_generation === fenceBefore.output_generation
      && (action.kind !== "set_blackout" || fenceAfter.safety_blackout_epoch === fenceBefore.safety_blackout_epoch
        && fenceAfter.safety_blackout_generation === fenceBefore.safety_blackout_generation);
  const resetWithoutLease = action.kind === "reset_show_spout_outputs" && result.lease_result === null;
  if (!fencesEqual(fenceBefore, expectedFence)
    || result.outcome === "no_op" && !fenceUnchanged
    // Ordinary project/output-control mutations must advance their fence on
    // Applied. The physical Display shell and the runtime-only serial/Spout/
    // window forms are deliberately outside that persisted fence, so both
    // Applied and NoOp retain the same fence. Enabling the authored Art-Net
    // route or enabling the authored Spout pair changes persisted project
    // truth and therefore must advance its project fence.
    || result.outcome === "applied" && !fenceUnchangedPhysicalAction && fenceUnchanged
    || fenceUnchangedPhysicalAction && !fenceUnchanged && !spoutAppliedPersistedProjectFence
    || !persistedProjectFenceIsExact && !spoutAppliedPersistedProjectFence) throw new Error("OutputControl receipt was inconsistent; physical output state is unknown.");
  return {
    operation_id: operationId,
    request_id: requestId,
    shape_sha256: result.shape_sha256,
    argument_fingerprint: result.argument_fingerprint,
    audit_sequence: result.audit_sequence,
    fence_before: fenceBefore,
    fence_after: fenceAfter,
    outcome: result.outcome,
    lease_result: resetWithoutLease ? null : assertLeaseResult(result.lease_result, action),
  };
};

const assertSelectedLeaseIsUsable = (query: OutputLeaseAuthorityQuery, action: OutputControlOperationAction): void => {
  if (action.kind === "acquire_lease" || action.kind === "enable_output" || action.kind === "reset_show_spout_outputs") return;
  const selected = query.statuses.filter((status): status is OutputLeaseAuthorityQueryHeld =>
    status.status !== "unavailable" && status.authority.lease_id === action.lease.lease_id);
  if (selected.length !== 1 || selected[0].authority.generation !== action.lease.generation
    || action.kind === "arm" && selected[0].status !== "held_active"
    || action.kind === "set_blackout" && selected[0].status !== "held_active"
    || action.kind === "release_blackout" && selected[0].status !== "held_active"
    || action.kind === "enable_show_art_net_loopback_route" && selected[0].status !== "held_active"
    || action.kind === "enable_show_serial_dmx_safety_blackout_route" && selected[0].status !== "held_active"
    || action.kind === "stop_show_serial_dmx_safety_blackout_route" && selected[0].status !== "held_active"
    || action.kind === "send_dsf2026_artnet_acceptance_probe" && selected[0].status !== "held_active"
    || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt" && selected[0].status !== "held_active"
    || action.kind === "enable_show_spout_outputs" && selected[0].status !== "held_active"
    || action.kind === "take_over_standby" && selected[0].status !== "held_active"
    || action.kind === "add_display"
      && selected[0].status !== "held_active" && selected[0].status !== "held_orphaned"
    || action.kind === "set_display_window_open"
      && selected[0].status !== "held_active" && selected[0].status !== "held_orphaned"
    || action.kind === "assign_video_output_composition" && selected[0].status !== "held_active"
    || action.kind === "renew_lease" && selected[0].status !== "held_active"
    || action.kind === "recover_lease" && selected[0].status !== "held_orphaned") {
    throw new Error("Selected output lease is unavailable, orphaned, stale, or has the wrong resources; nothing was applied.");
  }
  if (action.kind === "arm" || action.kind === "set_blackout" || action.kind === "release_blackout" || action.kind === "take_over_standby") {
    const expectedResources = action.kind === "arm"
      ? resourcesForRole(action.role)
      : ["lighting", "video"] as const;
    if (!sameResources(selected[0].resources, expectedResources)) throw new Error("Selected output lease is unavailable, orphaned, stale, or has the wrong resources; nothing was applied.");
  } else if (action.kind === "add_display" || action.kind === "set_display_window_open"
    || action.kind === "enable_show_art_net_loopback_route"
    || action.kind === "enable_show_serial_dmx_safety_blackout_route"
    || action.kind === "stop_show_serial_dmx_safety_blackout_route"
    || action.kind === "send_dsf2026_artnet_acceptance_probe"
    || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt"
    || action.kind === "enable_show_spout_outputs"
    || action.kind === "assign_video_output_composition") {
    const expectedResources = ["lighting", "video"] as const;
    if (!sameResources(selected[0].resources, expectedResources)) throw new Error("Selected output lease is unavailable, orphaned, stale, or has the wrong resources; nothing was applied.");
  }
};

type OutputControlExecutionOptions = {
  /**
   * The only non-Display bypass is the same-call Enable -> blackout release
   * bridge below.  The Enable receipt is already a strictly validated active
   * exact-Both proof; the backend still revalidates the lease before release.
   */
  skipPublicLeaseQuery?: "display-authority" | "enabled-lease-proof";
  /**
   * Optional caller fence for an operation whose intent was prepared from a
   * canonical project bundle.  The live OutputControl authority is checked
   * immediately before any command dispatch; native CAS remains authoritative.
   */
  expectedProject?: OutputControlExpectedProject;
  /** Set immediately before the first native mutation dispatch. */
  onMutationDispatch?: () => void;
};

const executeOutputControlOperation = async (
  invoke: FrontendTauriInvoke,
  action: OutputControlOperationAction,
  options: OutputControlExecutionOptions = {},
): Promise<OutputControlReceipt> => {
  const requestId = allocateRequestId();
  const operationId = operationIdForAction(action);
  assertAction(action);
  const authority = assertAuthority(await invoke<unknown>("query_output_control_authority_v1"));
  if (options.expectedProject
    && (authority.fence.project_epoch !== options.expectedProject.project_epoch
      || authority.fence.project_revision !== options.expectedProject.project_revision
      || authority.fence.project_checkpoint_hash !== options.expectedProject.checkpoint_hash)) {
    throw new Error("OutputControl project authority changed before dispatch; nothing was applied.");
  }
  if (options.skipPublicLeaseQuery) {
    const displayBypass = options.skipPublicLeaseQuery === "display-authority"
      && (action.kind === "add_display" || action.kind === "set_display_window_open");
    const enabledLeaseProofBypass = options.skipPublicLeaseQuery === "enabled-lease-proof"
      && action.kind === "release_blackout";
    if (!displayBypass && !enabledLeaseProofBypass) {
      throw new Error("Only canonical Display actions may bypass the public lease query; nothing was applied.");
    }
  } else if (action.kind !== "reset_show_spout_outputs") {
    const leaseQuery = await queryOutputLeaseAuthority(invoke);
    assertSelectedLeaseIsUsable(leaseQuery, action);
  }
  const executeArgs = deepFreeze({
    request: { operation_id: operationId, request_id: requestId, expected_fence: authority.fence, action },
  });
  let terminal: unknown;
  const command = commandForAction(action);
  try {
    options.onMutationDispatch?.();
    terminal = await invoke<unknown>(command, executeArgs);
  } catch (firstError) {
    if (action.kind === "send_dsf2026_artnet_acceptance_probe"
      || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt") {
      throw new Error(
        `DSF2026 Art-Net probe execution reply was lost (${String(firstError)}); no IPC retry was attempted. The fixed physical outcome is unknown; query its durable status before any further action.`,
      );
    }
    try {
      options.onMutationDispatch?.();
      terminal = await invoke<unknown>(command, executeArgs);
    } catch {
      throw new Error(`OutputControl execution reply was lost (${String(firstError)}); physical output state is unknown.`);
    }
  }
  return assertResponse(terminal, action, operationId, requestId, authority.fence);
};

/** Execute an ordinary action with a caller-selected exact active lease. */
export async function executeOutputControl(
  invoke: FrontendTauriInvoke,
  action: OutputControlAction,
): Promise<OutputControlReceipt> {
  return executeOutputControlOperation(invoke, action);
}

/**
 * Execute AddDisplay using the dedicated read-only authority query. The
 * backend v2 command owns same-owner orphan recovery and Add atomically, so
 * this lane deliberately does not fall back to the public lifecycle query.
 */
export async function executeDisplayAddOutputControl(
  invoke: FrontendTauriInvoke,
  action: OutputDisplayAction,
  authorityQuery: DisplayAddLeaseAuthorityQuery,
): Promise<OutputControlReceipt> {
  const authority = selectExactBothLeaseForDisplayAdd(authorityQuery);
  if (authority.lease_id !== action.lease.lease_id || authority.generation !== action.lease.generation) {
    throw new Error("Display Add lease authority changed before execution; nothing was applied.");
  }
  return executeOutputControlOperation(invoke, action, { skipPublicLeaseQuery: "display-authority" });
}

/** Execute the live Display window action through the Add-equivalent
 * active-or-recoverable exact-Both authority lane. */
export async function executeDisplayWindowOutputControl(
  invoke: FrontendTauriInvoke,
  action: OutputDisplayWindowAction,
  authorityQuery: DisplayAddLeaseAuthorityQuery,
): Promise<OutputControlReceipt> {
  const authority = selectExactBothLeaseForDisplayAdd(authorityQuery);
  if (authority.lease_id !== action.lease.lease_id || authority.generation !== action.lease.generation) {
    throw new Error("Display window lease authority changed before execution; nothing was applied.");
  }
  return executeOutputControlOperation(invoke, action, { skipPublicLeaseQuery: "display-authority" });
}

/** Normal one-step output path; no lease selection or six-digit code is shown. */
export async function enableOutput(
  invoke: FrontendTauriInvoke,
): Promise<OutputControlReceipt> {
  return executeOutputControl(invoke, { kind: "enable_output" });
}

/**
 * Release the global DMX safety latch through the same owner-bound R4 route
 * used by the ordinary operator control.  S0 engage is intentionally
 * lease-free, so a restart/expiry can leave the latch engaged while no active
 * Both lease is visible.  In that one bounded case, recover the canonical
 * local Both lease first; never auto-recover when any active split/foreign or
 * ambiguous lease exists, because that would conceal an ownership conflict.
 */
export async function executeBlackoutRelease(
  invoke: FrontendTauriInvoke,
): Promise<OutputControlReceipt> {
  const expectedResources = ["lighting", "video"] as const;
  let leaseQuery = await queryOutputLeaseAuthority(invoke);
  let lease: OutputLeaseAuthority;
  if (hasOnlyActiveOutputLease(leaseQuery, expectedResources)) {
    lease = selectOnlyActiveOutputLease(leaseQuery, expectedResources);
  } else if (!leaseQuery.statuses.some((status) => status.status === "held_active")) {
    // Engage can be issued safely before Enable Output.  Recovering the
    // canonical Both lease keeps the existing native ReleaseBlackout
    // ownership/consent boundary intact while making the UI toggle usable
    // after a process restart or lease expiry.
    const enableReceipt = await enableOutput(invoke);
    const enabledLease = enableReceipt.lease_result;
    if (!enabledLease
      || enabledLease.phase !== "held_active"
      || enabledLease.outcome !== "acquired" && enabledLease.outcome !== "recovered"
      || !sameResources(enabledLease.resources, expectedResources)) {
      throw new Error(
        "Output enable returned without a current active Both lease; blackout remains engaged. Refresh output ownership in Setup > I/O.",
      );
    }
    // Do not immediately re-read the public query here.  That query is a
    // fail-fast observation and can still show the pre-Enable view while the
    // renderer's read lane settles.  The receipt is the exact successful
    // Enable proof; Release still performs its own backend owner/fence/lease
    // CAS before touching the safety latch.
    lease = { ...enabledLease.authority };
    return executeOutputControlOperation(
      invoke,
      { kind: "release_blackout", lease },
      { skipPublicLeaseQuery: "enabled-lease-proof" },
    );
  } else {
    throw new Error(
      "Blackout control requires one active local Both output lease; resolve output ownership in Setup > I/O first. Blackout remains engaged.",
    );
  }
  return executeOutputControl(invoke, { kind: "release_blackout", lease });
}

/** Targeted blackout never arms previously inactive physical outputs. */
export async function executeTargetBlackout(
  invoke: FrontendTauriInvoke,
  target: OutputControlTargetRole,
  enabled: boolean,
  options: Pick<OutputControlExecutionOptions, "expectedProject" | "onMutationDispatch"> = {},
): Promise<OutputControlReceipt> {
  const query = await queryOutputLeaseAuthority(invoke);
  const lease = selectOnlyActiveOutputLease(query, ["lighting", "video"]);
  return executeOutputControlOperation(invoke, { kind: "set_blackout", target, enabled, lease }, options);
}

/** Execute a lease lifecycle action through the same fenced receipt lane. */
export async function executeOutputLeaseLifecycle(
  invoke: FrontendTauriInvoke,
  action: OutputLeaseLifecycleAction,
): Promise<OutputControlReceipt> {
  return executeOutputControlOperation(invoke, action);
}
