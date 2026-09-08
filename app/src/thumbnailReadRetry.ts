const transientThumbnailErrors = new Set([
  "Thumbnail work is already queued or running; retry after it completes",
  "Thumbnail renderer wait timed out; retry after preview work completes",
]);

/** One serial retry of an explicitly transient native failure, owned by its batch. */
export async function readThumbnailWithRetry(
  load: () => Promise<string>,
  isCurrent: () => boolean,
  wait: () => Promise<void> = () => new Promise(resolve => setTimeout(resolve, 100)),
): Promise<string> {
  if (!isCurrent()) throw new Error("Thumbnail batch was retired before read");
  try {
    return await load();
  } catch (error) {
    const message = error instanceof Error ? error.message : error;
    if (!isCurrent() || typeof message !== "string" || !transientThumbnailErrors.has(message)) throw error;
    await wait();
    if (!isCurrent()) throw new Error("Thumbnail batch was retired before retry");
    // A second failure is terminal: no recursive retry, queue growth or stale fallback.
    return load();
  }
}
