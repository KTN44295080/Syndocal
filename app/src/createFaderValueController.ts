import type { Setter } from "solid-js";

export type FaderValueMap = Record<string, number>;

interface FaderValueControllerOptions {
  setValues: Setter<FaderValueMap>;
}

interface PendingWrite {
  value: number;
  write: () => Promise<void>;
}

interface WriteWaiter {
  resolve: () => void;
  reject: (error: unknown) => void;
}

interface WriteSlot {
  running: boolean;
  next: PendingWrite | null;
  waiters: WriteWaiter[];
}

/**
 * Keep rapid fader input ordered at the IPC boundary and keep an optimistic
 * value visible while a snapshot poll catches up with the last accepted write.
 * A range input emits many values during one drag; independent invokes would
 * otherwise let an older completion or snapshot repaint the fader backwards.
 */
export function createFaderValueController(options: FaderValueControllerOptions) {
  const optimisticValues = new Map<string, number>();
  const slots = new Map<string, WriteSlot>();

  const setOptimistic = (key: string, value: number) => {
    optimisticValues.set(key, value);
    options.setValues((current) => ({ ...current, [key]: value }));
  };

  const mergeSnapshotValues = (current: FaderValueMap, incoming: FaderValueMap): FaderValueMap => {
    const next = { ...current };
    for (const [key, value] of Object.entries(incoming)) {
      const optimistic = optimisticValues.get(key);
      if (optimistic !== undefined) {
        if (optimistic === value) {
          optimisticValues.delete(key);
        } else {
          next[key] = optimistic;
          continue;
        }
      }
      next[key] = value;
    }
    return next;
  };

  const clearOptimistic = (key: string, value?: number) => {
    if (value === undefined || optimisticValues.get(key) === value) {
      optimisticValues.delete(key);
    }
  };

  const drain = async (key: string, slot: WriteSlot) => {
    slot.running = true;
    while (slot.next) {
      const current = slot.next;
      slot.next = null;
      try {
        await current.write();
      } catch (error) {
        clearOptimistic(key, current.value);
        slot.next = null;
        const waiters = slot.waiters.splice(0);
        for (const waiter of waiters) waiter.reject(error);
        break;
      }
      if (!slot.next) {
        const waiters = slot.waiters.splice(0);
        for (const waiter of waiters) waiter.resolve();
      }
    }
    slot.running = false;
    if (!slot.next && slot.waiters.length === 0) slots.delete(key);
  };

  const enqueue = (key: string, value: number, write: () => Promise<void>) => {
    setOptimistic(key, value);
    const slot = slots.get(key) ?? { running: false, next: null, waiters: [] };
    slots.set(key, slot);
    const promise = new Promise<void>((resolve, reject) => {
      slot.waiters.push({ resolve, reject });
    });
    slot.next = { value, write };
    if (!slot.running) void drain(key, slot);
    return promise;
  };

  const clearAll = () => {
    optimisticValues.clear();
  };

  return { enqueue, setOptimistic, mergeSnapshotValues, clearAll };
}
