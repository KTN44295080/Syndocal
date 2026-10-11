import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export interface FixtureFaderWrite {
  targets: { fixture_id: number; attribute: string }[];
  value: number;
}
export type FixtureFaderStream = (send: (write: FixtureFaderWrite) => Promise<void>) => Promise<void>;

/** Latest values flow within one bounded history transaction, not one per input. */
export function createContinuousFixtureFader(invoke: FrontendTauriInvoke) {
  const pending = new Map<string, { write: FixtureFaderWrite; serial: number }>();
  let serial = 0;
  let active: Promise<void> | null = null;
  let generation = 0;
  let wake: (() => void) | null = null;
  const waiters: { generation: number; serial: number; resolve: () => void; reject: (error: unknown) => void }[] = [];

  const run = async (capturedGeneration: number) => {
    if (capturedGeneration !== generation) throw new Error("Fader gesture cancelled by project change.");
    while (pending.size && capturedGeneration === generation) {
      let completedSerial = 0;
      const stream: FixtureFaderStream = async (send) => {
        const started = performance.now();
        let sent = 0;
        while (performance.now() - started < 2_000 && sent < 512) {
          if (capturedGeneration !== generation) throw new Error("Fader gesture cancelled by project change.");
          if (!pending.size) {
            let timer: ReturnType<typeof setTimeout>;
            await new Promise<void>((resolve) => {
              wake = resolve;
              timer = setTimeout(resolve, 120);
            });
            clearTimeout(timer!);
            wake = null;
            if (capturedGeneration !== generation) throw new Error("Fader gesture cancelled by project change.");
            if (!pending.size) return;
          }
          const [key, { write, serial }] = pending.entries().next().value!;
          pending.delete(key);
          await send(write);
          sent++;
          completedSerial = Math.max(completedSerial, serial);
        }
      };
      await invoke("stream_fixture_attribute_v1", { __fixtureFaderStream: stream });
      for (let i = waiters.length - 1; i >= 0; i--) {
        if (waiters[i].generation === capturedGeneration && waiters[i].serial <= completedSerial) {
          waiters.splice(i, 1)[0].resolve();
        }
      }
    }
  };
  const start = () => {
    const runningGeneration = generation;
    // Publish the active promise before any injected transport can re-enter enqueue.
    active = Promise.resolve().then(() => run(runningGeneration)).catch((error) => {
      if (generation === runningGeneration) pending.clear();
      for (let i = waiters.length - 1; i >= 0; i--) {
        if (waiters[i].generation === runningGeneration) waiters.splice(i, 1)[0].reject(error);
      }
    }).finally(() => {
      active = null;
      if (pending.size) start();
    });
  };
  const enqueue = (key: string, write: FixtureFaderWrite): Promise<void> => {
    if (waiters.length >= 1024 || serial >= Number.MAX_SAFE_INTEGER) {
      return Promise.reject(new Error("Fader input capacity exceeded; wait for the active gesture."));
    }
    const writeSerial = ++serial;
    // Updating an overlapping group must follow earlier individual writes.
    pending.delete(key);
    pending.set(key, { write, serial: writeSerial });
    wake?.();
    const result = new Promise<void>((resolve, reject) => waiters.push({ generation, serial: writeSerial, resolve, reject }));
    if (!active) start();
    return result;
  };
  const cancel = () => { generation++; pending.clear(); wake?.(); };
  return { enqueue, cancel };
}
