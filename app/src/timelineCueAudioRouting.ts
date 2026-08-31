import type { TimelineCueAudioEndpointSummary } from "./types";

/**
 * Exact-name selection is intentionally fail-closed.  A duplicate display
 * name is not a stable device identity, even when native status accidentally
 * marks one of the records selectable.
 */
export const timelineCueAudioEndpointIsSelectable = (
  endpoints: readonly TimelineCueAudioEndpointSummary[],
  endpoint: TimelineCueAudioEndpointSummary,
): boolean => endpoints.filter((candidate) => candidate.name === endpoint.name).length === 1
  && endpoint.occurrences === 1
  && endpoint.selectable;

export const timelineCueAudioEndpointByExactName = (
  endpoints: readonly TimelineCueAudioEndpointSummary[],
  name: string | null,
): TimelineCueAudioEndpointSummary | null => {
  if (!name) return null;
  const matches = endpoints.filter((endpoint) => endpoint.name === name);
  return matches.length === 1 && timelineCueAudioEndpointIsSelectable(endpoints, matches[0]!)
    ? matches[0]!
    : null;
};

export const timelineCueAudioMissingSavedName = (
  endpoints: readonly TimelineCueAudioEndpointSummary[],
  name: string | null,
): boolean => Boolean(name) && !endpoints.some((endpoint) => endpoint.name === name);
