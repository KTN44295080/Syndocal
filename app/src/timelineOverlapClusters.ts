export type TimelineOverlapEventId = number | string;

export const TIMELINE_OVERLAP_BADGE_WIDTH_PX = 24;
export const TIMELINE_MAX_VISIBLE_OVERLAP_RAILS = 8;

export interface TimelineOverlapEvent<
  TId extends TimelineOverlapEventId = TimelineOverlapEventId,
  TTrack extends string = string,
> {
  id: TId;
  track: TTrack;
  time_ms: number;
  total_duration_ms?: number;
  duration_ms?: number;
  loop_count?: number;
}

export interface TimelineOverlapInterval<
  TId extends TimelineOverlapEventId = TimelineOverlapEventId,
  TTrack extends string = string,
> {
  id: TId;
  track: TTrack;
  start_ms: number;
  end_ms: number;
}

export interface TimelineOverlapCluster<
  TId extends TimelineOverlapEventId = TimelineOverlapEventId,
  TTrack extends string = string,
> {
  id: string;
  label: string;
  track: TTrack;
  start_ms: number;
  end_ms: number;
  count: number;
  member_ids: TId[];
}

export interface TimelineOverlapBadgeCandidate<
  TId extends TimelineOverlapEventId = TimelineOverlapEventId,
  TTrack extends string = string,
> extends TimelineOverlapCluster<TId, TTrack> {
  x: number;
  width: number;
}

export interface TimelinePackedOverlapBadge<
  TId extends TimelineOverlapEventId = TimelineOverlapEventId,
  TTrack extends string = string,
> extends TimelineOverlapBadgeCandidate<TId, TTrack> {
  source_cluster_ids: string[];
  /** Number of unique source overlap clusters represented by this badge. */
  group_count: number;
  aggregated: boolean;
}

export interface TimelineOverlapRailEvent extends TimelineOverlapEvent {
  /** The authored lane identity; omitted when the caller already scopes track. */
  layer_id?: number | string;
}

export interface TimelineOverlapRailCluster {
  track: string;
  layer_id?: number | string;
  member_ids: readonly TimelineOverlapEventId[];
}

export interface TimelineOverlapRailPlacement {
  id: TimelineOverlapEventId;
  track: string;
  layer_id?: number | string;
  /** Zero-based internal rail, ordered from the top of the expanded lane. */
  rail_index: number | null;
  /** Number of visible rails reserved for this lane, capped at the exported maximum. */
  rail_count: number;
  /** No marker is rendered; the overlap inspector remains the discoverability path. */
  overflowed: boolean;
}

const FNV64_OFFSET = 0xcbf29ce484222325n;
const FNV64_PRIME = 0x100000001b3n;
const FNV64_MASK = 0xffffffffffffffffn;

const compareText = (left: string, right: string) => left < right ? -1 : left > right ? 1 : 0;

const eventIdKey = (id: TimelineOverlapEventId) =>
  typeof id === "number" ? `n:${Object.is(id, -0) ? 0 : id}` : `s:${id}`;

const layerIdKey = (layerId: number | string | undefined) =>
  layerId === undefined
    ? "u:"
    : `${typeof layerId === "number" ? "n" : "s"}:${String(layerId)}`;

const railMembershipKey = (
  track: string,
  layerId: number | string | undefined,
  memberId: TimelineOverlapEventId,
) => `${track}\u001f${layerIdKey(layerId)}\u001f${eventIdKey(memberId)}`;

const compareEventIds = (left: TimelineOverlapEventId, right: TimelineOverlapEventId) => {
  if (typeof left === "number" && typeof right === "number") return left - right;
  return compareText(eventIdKey(left), eventIdKey(right));
};

const stableHash = (value: string) => {
  let hash = FNV64_OFFSET;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= BigInt(value.charCodeAt(index));
    hash = (hash * FNV64_PRIME) & FNV64_MASK;
  }
  return hash.toString(36).padStart(13, "0");
};

const finiteNonNegative = (value: number | undefined) =>
  value !== undefined && Number.isFinite(value) ? Math.max(0, value) : null;

const timelineEventDurationMs = (event: TimelineOverlapEvent) => {
  const explicitTotalMs = finiteNonNegative(event.total_duration_ms);
  if (explicitTotalMs !== null) return explicitTotalMs;
  const durationMs = finiteNonNegative(event.duration_ms) ?? 0;
  const loopCount = Number.isFinite(event.loop_count)
    ? Math.max(1, Math.floor(event.loop_count ?? 1))
    : 1;
  return durationMs * loopCount;
};

export const timelineOverlapIntervalForEvent = <TEvent extends TimelineOverlapEvent>(
  event: TEvent,
): TimelineOverlapInterval<TEvent["id"], TEvent["track"]> | null => {
  if (!Number.isFinite(event.time_ms)) return null;
  const durationMs = timelineEventDurationMs(event);
  const endMs = event.time_ms + durationMs;
  if (!(durationMs > 0) || !Number.isFinite(endMs)) return null;
  return {
    id: event.id,
    track: event.track,
    start_ms: event.time_ms,
    end_ms: endMs,
  };
};

const compareIntervals = <
  TId extends TimelineOverlapEventId,
  TTrack extends string,
>(
  left: TimelineOverlapInterval<TId, TTrack>,
  right: TimelineOverlapInterval<TId, TTrack>,
) => left.start_ms - right.start_ms ||
  left.end_ms - right.end_ms ||
  compareEventIds(left.id, right.id);

const clusterId = <TId extends TimelineOverlapEventId>(track: string, memberIds: readonly TId[]) => {
  const membershipKey = memberIds
    .map(eventIdKey)
    .sort(compareText)
    .join("\u001f");
  return `timeline-overlap:${encodeURIComponent(track)}:${stableHash(membershipKey)}`;
};

const createCluster = <
  TId extends TimelineOverlapEventId,
  TTrack extends string,
>(
  track: TTrack,
  intervals: readonly TimelineOverlapInterval<TId, TTrack>[],
  startMs: number,
  endMs: number,
): TimelineOverlapCluster<TId, TTrack> => {
  const memberIds = intervals.map((interval) => interval.id);
  return {
    id: clusterId(track, memberIds),
    label: `${track} overlap (${memberIds.length})`,
    track,
    start_ms: startMs,
    end_ms: endMs,
    count: memberIds.length,
    member_ids: memberIds,
  };
};

/**
 * Builds connected overlap components from absolute half-open event intervals.
 * Adjacent events whose end/start timestamps only touch are not overlaps.
 */
export const buildTimelineOverlapClusters = <TEvent extends TimelineOverlapEvent>(
  events: readonly TEvent[],
): TimelineOverlapCluster<TEvent["id"], TEvent["track"]>[] => {
  type Interval = TimelineOverlapInterval<TEvent["id"], TEvent["track"]>;
  const intervalsByTrack = new Map<TEvent["track"], Interval[]>();

  for (const event of events) {
    const interval = timelineOverlapIntervalForEvent(event);
    if (!interval) continue;
    const trackIntervals = intervalsByTrack.get(interval.track);
    if (trackIntervals) trackIntervals.push(interval);
    else intervalsByTrack.set(interval.track, [interval]);
  }

  const tracks = [...intervalsByTrack.keys()].sort(compareText);
  const clusters: TimelineOverlapCluster<TEvent["id"], TEvent["track"]>[] = [];

  for (const track of tracks) {
    const intervals = intervalsByTrack.get(track) ?? [];
    intervals.sort(compareIntervals);
    let component: Interval[] = [];
    let componentStartMs = 0;
    let componentEndMs = 0;

    const flush = () => {
      if (component.length > 1) {
        clusters.push(createCluster(track, component, componentStartMs, componentEndMs));
      }
      component = [];
    };

    for (const interval of intervals) {
      if (component.length === 0) {
        component = [interval];
        componentStartMs = interval.start_ms;
        componentEndMs = interval.end_ms;
        continue;
      }
      if (interval.start_ms < componentEndMs) {
        component.push(interval);
        componentEndMs = Math.max(componentEndMs, interval.end_ms);
        continue;
      }
      flush();
      component = [interval];
      componentStartMs = interval.start_ms;
      componentEndMs = interval.end_ms;
    }
    flush();
  }

  return clusters;
};

/**
 * Assigns each visible overlap member to a deterministic interval rail.
 *
 * Rails are colored greedily after stable start/end/id sorting, reusing a rail
 * only when its previous interval has ended. Search is deliberately bounded to
 * eight rails; an event that arrives while all eight are occupied is returned
 * with `overflowed: true` and no render rail, leaving the overlap inspector as
 * its discoverability path. Explicit zero-duration events are excluded so
 * legacy point markers cannot acquire a fake span.
 */
export const buildTimelineOverlapRailLayout = (
  events: readonly TimelineOverlapRailEvent[],
  clusters: readonly TimelineOverlapRailCluster[],
): TimelineOverlapRailPlacement[] => {
  const memberKeys = new Set<string>();
  for (const cluster of clusters) {
    for (const memberId of cluster.member_ids) {
      memberKeys.add(railMembershipKey(cluster.track, cluster.layer_id, memberId));
    }
  }

  const eventsByLayer = new Map<string, {
    event: TimelineOverlapRailEvent;
    interval: TimelineOverlapInterval;
  }[]>();
  for (const event of events) {
    if (!memberKeys.has(railMembershipKey(event.track, event.layer_id, event.id))) continue;
    if (event.duration_ms !== undefined && !(Number.isFinite(event.duration_ms) && event.duration_ms > 0)) {
      continue;
    }
    const interval = timelineOverlapIntervalForEvent(event);
    if (!interval) continue;
    const layerKey = `${event.track}\u001f${layerIdKey(event.layer_id)}`;
    const layerEvents = eventsByLayer.get(layerKey);
    if (layerEvents) layerEvents.push({ event, interval });
    else eventsByLayer.set(layerKey, [{ event, interval }]);
  }

  const placements: TimelineOverlapRailPlacement[] = [];
  for (const layerKey of [...eventsByLayer.keys()].sort(compareText)) {
    const layerEvents = eventsByLayer.get(layerKey) ?? [];
    layerEvents.sort((left, right) => compareIntervals(left.interval, right.interval));
    const railEndMs: number[] = [];
    const layerPlacements: TimelineOverlapRailPlacement[] = [];
    for (const { event, interval } of layerEvents) {
      const railIndex = railEndMs.findIndex((endMs) => interval.start_ms >= endMs);
      const canOpenRail = railIndex < 0 && railEndMs.length < TIMELINE_MAX_VISIBLE_OVERLAP_RAILS;
      const assignedRail = railIndex >= 0 ? railIndex : canOpenRail ? railEndMs.length : null;
      if (assignedRail !== null) railEndMs[assignedRail] = interval.end_ms;
      layerPlacements.push({
        id: event.id,
        track: event.track,
        layer_id: event.layer_id,
        rail_index: assignedRail,
        rail_count: 0,
        overflowed: assignedRail === null,
      });
    }
    const railCount = railEndMs.length;
    placements.push(...layerPlacements.map((placement) => ({ ...placement, rail_count: railCount })));
  }
  return placements;
};

export const packTimelineOverlapClusterBadges = <
  TId extends TimelineOverlapEventId,
  TTrack extends string,
>(
  clusters: readonly TimelineOverlapBadgeCandidate<TId, TTrack>[],
  viewBoxWidth: number,
  minimumX = 27,
  badgeWidth = TIMELINE_OVERLAP_BADGE_WIDTH_PX,
  badgeGap = 2,
  rightPadding = 2,
  maximumBadgesPerTrack = Number.POSITIVE_INFINITY,
): TimelinePackedOverlapBadge<TId, TTrack>[] => {
  const safeMinimumX = Math.max(0, Number.isFinite(minimumX) ? minimumX : 27);
  const safeBadgeWidth = Math.max(
    1,
    Number.isFinite(badgeWidth) ? badgeWidth : TIMELINE_OVERLAP_BADGE_WIDTH_PX,
  );
  const safeGap = Math.max(0, Number.isFinite(badgeGap) ? badgeGap : 2);
  const slotWidth = safeBadgeWidth + safeGap;
  const maximumX = Math.max(
    safeMinimumX,
    (Number.isFinite(viewBoxWidth) ? viewBoxWidth : 100) - safeBadgeWidth - Math.max(0, rightPadding),
  );
  const geometricCapacity = Math.max(1, Math.floor((maximumX - safeMinimumX) / slotWidth) + 1);
  const requestedCapacity = Number.isFinite(maximumBadgesPerTrack)
    ? Math.max(1, Math.floor(maximumBadgesPerTrack))
    : geometricCapacity;
  const capacity = Math.min(geometricCapacity, requestedCapacity);
  const tracks = [...new Set(clusters.map((cluster) => cluster.track))].sort(compareText);
  const packed: TimelinePackedOverlapBadge<TId, TTrack>[] = [];

  for (const track of tracks) {
    const sorted = clusters
      .filter((cluster) => cluster.track === track)
      .slice()
      .sort((left, right) => left.x - right.x || compareText(left.id, right.id));
    const clamped: TimelinePackedOverlapBadge<TId, TTrack>[] = sorted.map((cluster) => ({
      ...cluster,
      x: Math.min(maximumX, Math.max(safeMinimumX, cluster.x)),
      source_cluster_ids: [cluster.id],
      group_count: 1,
      aggregated: false,
    }));
    const aggregate = (
      group: TimelinePackedOverlapBadge<TId, TTrack>[],
    ): TimelinePackedOverlapBadge<TId, TTrack> => {
      if (group.length === 1) return group[0];
      const sourceClusterIds = [...new Set(group.flatMap((cluster) => cluster.source_cluster_ids))];
      const membersByKey = new Map<string, TId>();
      for (const memberId of group.flatMap((cluster) => cluster.member_ids)) {
        membersByKey.set(eventIdKey(memberId), memberId);
      }
      const memberIds = [...membersByKey.values()].sort(compareEventIds);
      const first = group[0];
      const groupCount = sourceClusterIds.length;
      return {
        ...first,
        id: `timeline-overlap-groups:${encodeURIComponent(String(track))}:${stableHash(sourceClusterIds.join("\u001f"))}`,
        label: `${track} overlap groups (${groupCount})`,
        start_ms: Math.min(...group.map((cluster) => cluster.start_ms)),
        end_ms: Math.max(...group.map((cluster) => cluster.end_ms)),
        count: memberIds.length,
        member_ids: memberIds,
        x: first.x,
        width: Math.max(...group.map((cluster) => cluster.width)),
        source_cluster_ids: sourceClusterIds,
        group_count: groupCount,
        aggregated: true,
      };
    };
    let candidates: TimelinePackedOverlapBadge<TId, TTrack>[] = [];
    let collisionGroup: TimelinePackedOverlapBadge<TId, TTrack>[] = [];
    for (const candidate of clamped) {
      if (
        collisionGroup.length === 0 ||
        candidate.x < collisionGroup[0].x + slotWidth
      ) {
        collisionGroup.push(candidate);
        continue;
      }
      candidates.push(aggregate(collisionGroup));
      collisionGroup = [candidate];
    }
    if (collisionGroup.length > 0) candidates.push(aggregate(collisionGroup));
    if (candidates.length > capacity) {
      const individualCount = Math.max(0, capacity - 1);
      const overflow = candidates.slice(individualCount);
      candidates = [
        ...candidates.slice(0, individualCount),
        aggregate(overflow),
      ];
    }
    packed.push(...candidates);
  }
  return packed;
};
