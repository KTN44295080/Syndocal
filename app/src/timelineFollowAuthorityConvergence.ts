import type { ProjectAuthorityToken } from "./projectAuthority";

export const timelineFollowRuntimeProjectChangedMessage =
  "Project changed since this Timeline Follow runtime read was issued; retry";

type TimelineFollowAuthorityConvergenceOptions = {
  isCurrent: (captured: ProjectAuthorityToken) => boolean;
  inFlightAuthorityPoll: () => Promise<unknown> | null;
  refreshAuthority: () => Promise<unknown>;
};

type TimelineFollowAuthorityConvergenceAttempt = {
  authority: ProjectAuthorityToken;
  promise: Promise<boolean>;
};

const sameAuthority = (left: ProjectAuthorityToken, right: ProjectAuthorityToken): boolean =>
  left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash;

/**
 * Converges one stale Follow read to the canonical project authority without
 * replaying the Follow read itself. A single attempt is shared by concurrent
 * stale reads from the same authority; a failed/current attempt remains
 * recorded until the authority changes or the owner resets this lane.
 */
export const createTimelineFollowAuthorityConvergence = (
  options: TimelineFollowAuthorityConvergenceOptions,
) => {
  let attempt: TimelineFollowAuthorityConvergenceAttempt | null = null;

  const reset = () => {
    attempt = null;
  };

  const converge = (captured: ProjectAuthorityToken): Promise<boolean> => {
    if (!options.isCurrent(captured)) return Promise.resolve(true);
    if (attempt && sameAuthority(attempt.authority, captured)) return attempt.promise;

    const promise = (async () => {
      try {
        // A poll may have started before the mutation which made this read
        // stale. Let that observation settle, then force a fresh poll if its
        // result did not already advance the renderer's authority.
        const inFlight = options.inFlightAuthorityPoll();
        if (inFlight) await inFlight;
        if (!options.isCurrent(captured)) return true;
        await options.refreshAuthority();
      } catch {
        return false;
      }
      return !options.isCurrent(captured);
    })();
    attempt = { authority: { ...captured }, promise };
    return promise;
  };

  const suppressStaleReadError = (detail: string, captured: ProjectAuthorityToken): Promise<boolean> =>
    detail === timelineFollowRuntimeProjectChangedMessage
      ? converge(captured)
      : Promise.resolve(false);

  return { converge, suppressStaleReadError, reset };
};
