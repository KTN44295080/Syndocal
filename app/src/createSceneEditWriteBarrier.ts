/** Own scene-edit writes through their native ACK and queued scene capture.
 * Live controls cannot cross an outstanding Edit write when mode changes. */
export function createSceneEditWriteBarrier(editing: () => boolean) {
  const pending = new Set<Promise<unknown>>();
  const track = <T,>(write: () => Promise<T>): Promise<T> => {
    const owned = editing();
    const result = write();
    if (!owned) return result;
    let tracked: Promise<T>;
    tracked = result.finally(() => { pending.delete(tracked); });
    pending.add(tracked);
    return tracked;
  };
  const settle = async () => {
    while (pending.size > 0) await Promise.all([...pending]);
  };
  return { track, settle };
}
