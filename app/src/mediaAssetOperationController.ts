import type { MediaAssetActiveOperation, MediaAssetOperationPhase } from "./types";

type MediaAssetOperationPublisher = (
  update: MediaAssetActiveOperation[] | ((current: MediaAssetActiveOperation[]) => MediaAssetActiveOperation[]),
) => void;

export interface MediaAssetOperationControllerOptions {
  publish: MediaAssetOperationPublisher;
  registerCleanup?: (cleanup: () => void) => void;
}

export interface MediaAssetOperationHandle {
  signal: AbortSignal;
  setPhase: (phase: MediaAssetOperationPhase) => void;
  release: () => void;
}

/**
 * Owns the lifecycle of local-media operations without owning view state.
 * The App supplies the state publisher so this module cannot accidentally
 * couple the operation's cancellation map to a particular panel or signal.
 */
export function createMediaAssetOperationController({
  publish,
  registerCleanup,
}: MediaAssetOperationControllerOptions) {
  let nextOperationId = 0;
  const activeControllers = new Map<number, AbortController>();

  const begin = (label: string, initialPhase: MediaAssetOperationPhase): MediaAssetOperationHandle => {
    const controller = new AbortController();
    const id = ++nextOperationId;
    activeControllers.set(id, controller);
    publish((current) => [...current, { id, label, phase: initialPhase }]);
    let released = false;

    return {
      signal: controller.signal,
      setPhase: (phase) => {
        if (released || controller.signal.aborted) return;
        publish((current) => current.map((operation) =>
          operation.id === id ? { ...operation, phase } : operation));
      },
      release: () => {
        if (released) return;
        released = true;
        activeControllers.delete(id);
        publish((current) => current.filter((operation) => operation.id !== id));
      },
    };
  };

  const cancel = (id: number) => {
    const controller = activeControllers.get(id);
    if (!controller || controller.signal.aborted) return;
    controller.abort();
    publish((current) => current.map((operation) =>
      operation.id === id ? { ...operation, phase: "cancelling" } : operation));
  };

  const abortAll = () => {
    for (const controller of activeControllers.values()) controller.abort();
    publish((current) => current.map((operation) => ({
      ...operation,
      phase: "cancelling" as const,
    })));
  };

  registerCleanup?.(abortAll);

  return { begin, cancel, abortAll } as const;
}
