import type { ProjectAuthorityMappingFlushResult } from './projectAuthority';

/** All explicit consumers join one mapping flush, including its bounded ACK replay. */
export function createProjectControlMappingsFlusher<Result extends ProjectAuthorityMappingFlushResult>(
  flush: () => Promise<Result>,
): () => Promise<Result> {
  let pending: Promise<Result> | null = null;
  return () => {
    if (pending !== null) return pending;
    let owned: Promise<Result>;
    owned = Promise.resolve().then(flush).finally(() => {
      if (pending === owned) pending = null;
    });
    pending = owned;
    return owned;
  };
}
