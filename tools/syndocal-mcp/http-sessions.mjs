// Transport negotiation only; native credentials, grants and receipts live elsewhere.
// Expire lazily using a monotonic clock, without a background timer.
export function createHttpSessions({ limit = 64, idleMs = 300_000, now = () => performance.now() } = {}) {
  const entries = new Map();
  const prune = () => {
    const time = now();
    for (const [id, state] of entries) {
      if (!state.active && state.inFlight === 0 && time - state.touched >= idleMs) entries.delete(id);
    }
    return time;
  };
  return {
    acquire(id) {
      const time = prune();
      let state = entries.get(id);
      if (!state) {
        if (entries.size >= limit) return undefined;
        state = { initialized: false, negotiated: false, active: false, inFlight: 0, touched: time };
        entries.set(id, state);
      }
      state.touched = time;
      return state;
    },
    touch(state) { state.touched = now(); },
    remove(id) {
      prune();
      const state = entries.get(id);
      if (!state) return 404;
      if (state.active || state.inFlight > 0) return 409;
      entries.delete(id);
      return 204;
    },
  };
}
