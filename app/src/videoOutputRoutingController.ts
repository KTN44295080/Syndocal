import type { EngineSnapshot } from "./types";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import {
  executeOutputControl,
  queryOutputLeaseAuthority,
  selectOnlyActiveOutputLease,
} from "./outputControlController";

type VideoOutputRoutingControllerOptions = {
  invoke: FrontendTauriInvoke;
  snapshot: () => EngineSnapshot;
  refreshSnapshotAndVideoOutputRenderPlans: () => Promise<void>;
  setMessage: (message: string) => void;
  setBusy: (outputId: number, busy: boolean) => void;
};

const isPositiveSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const messageForError = (error: unknown): string =>
  error instanceof Error && error.message
    ? error.message
    : "Video output assignment failed before a verified receipt; route state is unknown. Refresh before retrying.";

/**
 * Owns only the renderer-side presentation of the canonical route operation.
 * It deliberately never writes a local project snapshot: a terminal R4
 * receipt is followed by a fresh backend snapshot/render-plan observation and
 * an exact output-to-composition verification.
 */
export function createVideoOutputRoutingController(options: VideoOutputRoutingControllerOptions) {
  const inFlight = new Set<number>();

  const assign = async (outputId: number, compositionId: number): Promise<boolean> => {
    if (!isPositiveSafeInteger(outputId) || !isPositiveSafeInteger(compositionId)) {
      options.setMessage("Video output route is invalid; no state changed.");
      return false;
    }
    if (inFlight.has(outputId)) {
      options.setMessage("Video output route assignment is already in progress; wait for its verified receipt.");
      return false;
    }
    const before = options.snapshot();
    const output = before.video.outputs.find((candidate) => candidate.id === outputId);
    const composition = before.video.compositions.find((candidate) => candidate.id === compositionId);
    if (!output || !composition) {
      options.setMessage("Video output or composition is stale; refresh before assigning a route.");
      return false;
    }

    inFlight.add(outputId);
    options.setBusy(outputId, true);
    try {
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      const receipt = await executeOutputControl(options.invoke, {
        kind: "assign_video_output_composition",
        output_id: outputId,
        composition_id: compositionId,
        lease,
      });
      await options.refreshSnapshotAndVideoOutputRenderPlans();
      const refreshed = options.snapshot().video.outputs.find((candidate) => candidate.id === outputId);
      if (!refreshed || refreshed.composition_id !== compositionId) {
        throw new Error(
          "Video output assignment receipt was terminal but the refreshed authoritative project did not contain the exact route; route state is unknown. Refresh before retrying.",
        );
      }
      options.setMessage(
        receipt.outcome === "no_op"
          ? `Video output ${outputId} already routes to ${composition.label}; authoritative route verified.`
          : `Video output ${outputId} now routes to ${composition.label}; authoritative route verified.`,
      );
      return true;
    } catch (error) {
      options.setMessage(messageForError(error));
      return false;
    } finally {
      inFlight.delete(outputId);
      options.setBusy(outputId, false);
    }
  };

  return { assign };
}
