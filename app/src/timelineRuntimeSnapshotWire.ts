import type {
  EngineSnapshot,
  EngineSnapshotRuntimeWireResponse,
  TimelineFollowAdmissionReason,
  TimelineFollowOutcome,
  TimelineFollowRuntimeWire,
  TimelineFollowRuntimeStatus,
  TimelineFollowSettlementConsumerSummary,
  TimelineFollowSettlementDomain,
  TimelineFollowSettlementDomainSummary,
  TimelineFollowSettlementState,
  TimelineFollowSettlementSummary,
  TimelineLoopRuntimeSummary,
  TimelineRuntimeSnapshotWire,
} from "./types";

type UnknownRecord = Record<string, unknown>;

const isRecord = (value: unknown): value is UnknownRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasExactKeys = (value: UnknownRecord, required: readonly string[], optional: readonly string[] = []) => {
  const allowed = new Set([...required, ...optional]);
  return required.every((key) => Object.prototype.hasOwnProperty.call(value, key))
    && Object.keys(value).every((key) => allowed.has(key));
};

const isCounter = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

const isTransportCounter = (value: unknown): value is number => isCounter(value) && value > 0;

const isNullableString = (value: unknown): value is string | null =>
  value === null || typeof value === "string";

const isOptionalString = (value: unknown): value is string | undefined =>
  value === undefined || typeof value === "string";

const isTimelineId = (value: unknown): value is number => isCounter(value);

const followStatuses = [
  "idle", "pending", "armed", "transitioning", "settling", "held", "fault", "aborting",
 ] as const satisfies readonly TimelineFollowRuntimeStatus[];
const followAdmissionReasons = [
  "natural_playback_boundary", "preroll_before_natural_playback_boundary",
 ] as const satisfies readonly TimelineFollowAdmissionReason[];
const followAbortReasons = [
  "stop", "manual_seek", "project_replacement", "timeline_bank_replacement", "loop_wrap",
  "playback_fault", "explicit_abort", "clock_discontinuity",
 ] as const;
const followSettlementStates = [
  "pending", "applied", "not_applicable", "fault", "timed_out",
 ] as const satisfies readonly TimelineFollowSettlementState[];
const followSettlementDomains = ["audio", "video", "lighting"] as const satisfies readonly TimelineFollowSettlementDomain[];
const followFaultPolicies = ["hold", "cut", "fault"] as const;

const knownEnum = <T extends string>(value: unknown, allowed: readonly T[]): T | null =>
  typeof value === "string" && (allowed as readonly string[]).includes(value) ? value as T : null;

const followOutcomeFromUnknown = (value: unknown): TimelineFollowOutcome | null | undefined => {
  if (value === null) return null;
  if (!isRecord(value) || typeof value.kind !== "string") return undefined;
  if (["completed", "cut", "held", "fault"].includes(value.kind) && hasExactKeys(value, ["kind"])) {
    return { kind: value.kind as "completed" | "cut" | "held" | "fault" };
  }
  const reason = knownEnum(value.reason, followAbortReasons);
  return value.kind === "aborted" && hasExactKeys(value, ["kind", "reason"]) && reason !== null
    ? { kind: "aborted", reason }
    : undefined;
};

type FollowSettlementConsumerId = TimelineFollowSettlementConsumerSummary["consumer_id"];

const followSettlementConsumerIdFromUnknown = (value: unknown): FollowSettlementConsumerId | null => {
  if (!isRecord(value) || typeof value.kind !== "string") return null;
  if ((value.kind === "audio" || value.kind === "lighting") && hasExactKeys(value, ["kind"])) {
    return { kind: value.kind };
  }
  return value.kind === "video_output" && hasExactKeys(value, ["kind", "output_id"]) && isCounter(value.output_id)
    ? { kind: "video_output", output_id: value.output_id }
    : null;
};

const consumerDomain = (consumerId: FollowSettlementConsumerId): TimelineFollowSettlementDomain =>
  consumerId.kind === "audio" ? "audio" : consumerId.kind === "lighting" ? "lighting" : "video";

const faultStateIsValid = (state: TimelineFollowSettlementState, fault: string | undefined): boolean =>
  state === "fault" ? typeof fault === "string" && fault.trim().length > 0 : fault === undefined;

const followSettlementConsumerFromUnknown = (
  value: unknown,
): TimelineFollowSettlementConsumerSummary | null => {
  if (!isRecord(value) || !hasExactKeys(value, ["consumer_id", "state"], ["fault"])) return null;
  const consumerId = followSettlementConsumerIdFromUnknown(value.consumer_id);
  const state = knownEnum(value.state, followSettlementStates);
  if (consumerId === null || state === null || !isOptionalString(value.fault) || !faultStateIsValid(state, value.fault)) {
    return null;
  }
  return value.fault === undefined ? { consumer_id: consumerId, state } : {
    consumer_id: consumerId,
    state,
    fault: value.fault,
  };
};

const followSettlementDomainFromUnknown = (
  value: unknown,
): TimelineFollowSettlementDomainSummary | null => {
  if (!isRecord(value) || !hasExactKeys(value, ["domain", "state"], ["fault", "consumers"])) return null;
  const domain = knownEnum(value.domain, followSettlementDomains);
  const state = knownEnum(value.state, followSettlementStates);
  if (domain === null || state === null || !isOptionalString(value.fault) || !faultStateIsValid(state, value.fault)
    || !(value.consumers === undefined || Array.isArray(value.consumers))) return null;
  const parsedConsumers: TimelineFollowSettlementConsumerSummary[] = [];
  for (const rawConsumer of value.consumers ?? []) {
    const consumer = followSettlementConsumerFromUnknown(rawConsumer);
    if (consumer === null) return null;
    parsedConsumers.push(consumer);
  }
  if (parsedConsumers.some((consumer) => consumerDomain(consumer.consumer_id) !== domain)) return null;
  return value.fault === undefined ? { domain, state, consumers: parsedConsumers } : {
    domain,
    state,
    fault: value.fault,
    consumers: parsedConsumers,
  };
};

const deriveSettlementState = (
  states: readonly TimelineFollowSettlementState[],
  allNotApplicable: TimelineFollowSettlementState,
): TimelineFollowSettlementState | null => {
  if (states.includes("fault")) return "fault";
  if (states.includes("timed_out")) return "timed_out";
  if (states.includes("pending")) return "pending";
  if (states.every((state) => state === "not_applicable")) return allNotApplicable;
  if (states.every((state) => state === "applied" || state === "not_applicable")
    && states.includes("applied")) return "applied";
  return null;
};

const settlementDomainStateIsValid = (domain: TimelineFollowSettlementDomainSummary): boolean => {
  if (domain.consumers.length === 0) {
    return domain.state === "not_applicable"
      || (domain.domain === "lighting" && (domain.state === "fault" || domain.state === "timed_out"));
  }
  const derived = deriveSettlementState(domain.consumers.map((consumer) => consumer.state), "not_applicable");
  return derived !== null && (domain.state === derived
    || (domain.domain === "lighting"
      && (domain.state === "fault" || domain.state === "timed_out")
      && derived !== "fault"
      && (domain.state !== "timed_out" || derived !== "timed_out")));
};

const followSettlementFromUnknown = (value: unknown): TimelineFollowSettlementSummary | null => {
  if (!isRecord(value) || !hasExactKeys(value, [
    "started_at_ms", "deadline_ms", "state", "progress_millis", "fault_policy", "domains",
  ], ["fault"])
    || !isCounter(value.started_at_ms)
    || !isCounter(value.deadline_ms)
    || value.deadline_ms < value.started_at_ms
    || !isCounter(value.progress_millis)
    || value.progress_millis > 1000
    || !isOptionalString(value.fault)
    || !Array.isArray(value.domains)) return null;
  const state = knownEnum(value.state, followSettlementStates);
  const faultPolicy = knownEnum(value.fault_policy, followFaultPolicies);
  if (state === null || faultPolicy === null || !faultStateIsValid(state, value.fault)) return null;
  const parsedDomains: TimelineFollowSettlementDomainSummary[] = [];
  for (const rawDomain of value.domains) {
    const domain = followSettlementDomainFromUnknown(rawDomain);
    if (domain === null) return null;
    parsedDomains.push(domain);
  }
  const domainSet = new Set(parsedDomains.map((domain) => domain.domain));
  const consumerSet = new Set<string>();
  if (parsedDomains.length !== followSettlementDomains.length
    || domainSet.size !== followSettlementDomains.length
    || followSettlementDomains.some((domain) => !domainSet.has(domain))
    || parsedDomains.some((domain) => !settlementDomainStateIsValid(domain))) return null;
  for (const domain of parsedDomains) {
    for (const consumer of domain.consumers) {
      const key = consumer.consumer_id.kind === "video_output"
        ? `video_output:${consumer.consumer_id.output_id}`
        : consumer.consumer_id.kind;
      if (consumerSet.has(key)) return null;
      consumerSet.add(key);
    }
  }
  const aggregate = deriveSettlementState(parsedDomains.map((domain) => domain.state), "applied");
  if (aggregate === null || aggregate !== state) return null;
  return value.fault === undefined ? {
    started_at_ms: value.started_at_ms,
    deadline_ms: value.deadline_ms,
    state,
    progress_millis: value.progress_millis,
    fault_policy: faultPolicy,
    domains: parsedDomains,
  } : {
    started_at_ms: value.started_at_ms,
    deadline_ms: value.deadline_ms,
    state,
    progress_millis: value.progress_millis,
    fault_policy: faultPolicy,
    fault: value.fault,
    domains: parsedDomains,
  };
};

const loopRuntimeFromUnknown = (value: unknown): TimelineLoopRuntimeSummary | null => {
  if (!isRecord(value)
    || !hasExactKeys(value, ["generation", "status", "a_ms", "b_ms", "wrap_count"], ["musical_length_millibeats"])
    || !isCounter(value.generation)
    || !["disabled", "armed", "looping"].includes(String(value.status))
    || !(value.a_ms === null || isCounter(value.a_ms))
    || !(value.b_ms === null || isCounter(value.b_ms))
    || !(value.musical_length_millibeats === undefined || value.musical_length_millibeats === null
      || isCounter(value.musical_length_millibeats))
    || !isCounter(value.wrap_count)) return null;
  return structuredClone(value) as unknown as TimelineLoopRuntimeSummary;
};

const followRuntimeFromUnknown = (value: unknown): TimelineFollowRuntimeWire | null => {
  if (!isRecord(value)
    || !hasExactKeys(value, [
      "epoch", "generation", "status", "admission_reason", "outcome", "source_timeline_id",
      "target_timeline_id", "elapsed_ms", "duration_ms", "progress_millis", "fault",
      "transition_hold_active", "waiting_for_pedal_start",
    ], ["settlement"])
    || !isTransportCounter(value.epoch)
    || !isCounter(value.generation)
    || !(value.source_timeline_id === null || isTimelineId(value.source_timeline_id))
    || !(value.target_timeline_id === null || isTimelineId(value.target_timeline_id))
    || !isCounter(value.elapsed_ms)
    || !isCounter(value.duration_ms)
    || !isCounter(value.progress_millis)
    || value.progress_millis > 1000
    || !isNullableString(value.fault)
    || typeof value.transition_hold_active !== "boolean"
    || typeof value.waiting_for_pedal_start !== "boolean") return null;
  const status = knownEnum(value.status, followStatuses);
  const admissionReason = value.admission_reason === null
    ? null
    : knownEnum(value.admission_reason, followAdmissionReasons);
  const outcome = followOutcomeFromUnknown(value.outcome);
  const settlement = value.settlement === undefined ? undefined : followSettlementFromUnknown(value.settlement);
  if (status === null || admissionReason === null && value.admission_reason !== null
    || outcome === undefined || settlement === null) return null;
  const runtime: TimelineFollowRuntimeWire = {
    epoch: value.epoch,
    generation: value.generation,
    status,
    admission_reason: admissionReason,
    outcome,
    source_timeline_id: value.source_timeline_id,
    target_timeline_id: value.target_timeline_id,
    elapsed_ms: value.elapsed_ms,
    duration_ms: value.duration_ms,
    progress_millis: value.progress_millis,
    fault: value.fault,
    transition_hold_active: value.transition_hold_active,
    waiting_for_pedal_start: value.waiting_for_pedal_start,
  };
  if (settlement !== undefined) runtime.settlement = settlement;
  return runtime;
};

/** Parse the only supported runtime projection; missing, stale-shaped, or future wire data is rejected. */
export const timelineRuntimeSnapshotWireFromUnknown = (value: unknown): TimelineRuntimeSnapshotWire | null => {
  if (!isRecord(value)
    || !hasExactKeys(value, ["transport_epoch", "transport_generation", "loop_runtime", "follow_runtime"])
    || !isTransportCounter(value.transport_epoch)
    || !isTransportCounter(value.transport_generation)) return null;
  const loopRuntime = loopRuntimeFromUnknown(value.loop_runtime);
  const followRuntime = followRuntimeFromUnknown(value.follow_runtime);
  if (loopRuntime === null || followRuntime === null || followRuntime.epoch !== value.transport_epoch) return null;
  return {
    transport_epoch: value.transport_epoch,
    transport_generation: value.transport_generation,
    loop_runtime: loopRuntime,
    follow_runtime: followRuntime,
  };
};

/** A ProjectAuthorityBundle may only apply a runtime whose redundant outer fence agrees exactly. */
export const projectAuthorityBundleTimelineRuntimeFromUnknown = (
  value: unknown,
): TimelineRuntimeSnapshotWire | null => {
  if (!isRecord(value)
    || !isTransportCounter(value.timeline_transport_epoch)
    || !isTransportCounter(value.timeline_transport_generation)) return null;
  const runtime = timelineRuntimeSnapshotWireFromUnknown(value.timeline_runtime);
  return runtime !== null
    && runtime.transport_epoch === value.timeline_transport_epoch
    && runtime.transport_generation === value.timeline_transport_generation
    ? runtime
    : null;
};

/** Hydrate only a validated engine-owned projection; authored snapshot fields never supply runtime fallback. */
export const hydrateTimelineRuntimeSnapshot = (
  snapshot: EngineSnapshot,
  wire: unknown,
): EngineSnapshot | null => {
  const runtime = timelineRuntimeSnapshotWireFromUnknown(wire);
  if (runtime === null) return null;
  return {
    ...snapshot,
    timeline: {
      ...snapshot.timeline,
      transport_epoch: runtime.transport_epoch,
      transport_generation: runtime.transport_generation,
      loop_runtime: runtime.loop_runtime,
      follow_runtime: runtime.follow_runtime,
    },
  };
};

/** Direct full `get_snapshot` has no legacy unwrapped form in a Tauri runtime. */
export const engineSnapshotRuntimeWireResponseFromUnknown = (
  value: unknown,
): EngineSnapshotRuntimeWireResponse | null => {
  if (!isRecord(value) || !hasExactKeys(value, ["snapshot", "timeline_runtime"]) || !isRecord(value.snapshot)) {
    return null;
  }
  const snapshot = hydrateTimelineRuntimeSnapshot(value.snapshot as unknown as EngineSnapshot, value.timeline_runtime);
  return snapshot === null ? null : { snapshot, timeline_runtime: timelineRuntimeSnapshotWireFromUnknown(value.timeline_runtime)! };
};
