import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import {
  executeTargetBlackout,
  type OutputControlExpectedProject,
  type OutputControlReceipt,
} from "./outputControlController";
import type { ProjectAuthorityBundle } from "./types";

export interface AgentBridgeEffects {
  /** Apply the receipt's canonical project fence before the snapshot read. */
  refreshProjectAuthority: (receipt: OutputControlReceipt) => Promise<unknown>;
  /** Refresh the UI snapshot after canonical authority has converged. */
  refreshSnapshot: () => Promise<unknown>;
}

type ProjectToken = Pick<
  ProjectAuthorityBundle,
  "project_epoch" | "project_revision" | "checkpoint_hash"
>;

const isObject = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object" && !Array.isArray(value);

const hasExactKeys = (value: Record<string, unknown>, expected: readonly string[]): boolean => {
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  return actual.length === required.length && actual.every((key, index) => key === required[index]);
};

const isNonnegativeSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

const isProjectToken = (value: unknown): value is ProjectToken =>
  isObject(value)
  && hasExactKeys(value, ["project_epoch", "project_revision", "checkpoint_hash"])
  && isNonnegativeSafeInteger(value.project_epoch)
  && isNonnegativeSafeInteger(value.project_revision)
  && typeof value.checkpoint_hash === "string"
  && /^[0-9a-f]{64}$/.test(value.checkpoint_hash);

const projectToken = (bundle: ProjectAuthorityBundle): ProjectToken => ({
  project_epoch: bundle.project_epoch,
  project_revision: bundle.project_revision,
  checkpoint_hash: bundle.checkpoint_hash,
});

const sameProjectToken = (left: ProjectToken, right: ProjectToken): boolean =>
  left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash;

interface ParsedVideoBlackoutRequest {
  enabled: boolean;
  expectedProject: OutputControlExpectedProject;
}

const parseVideoBlackoutRequest = (params: Record<string, unknown>): ParsedVideoBlackoutRequest => {
  if (!hasExactKeys(params, ["enabled", "expectedProject"]) || typeof params.enabled !== "boolean"
    || !isProjectToken(params.expectedProject)) {
    throw new Error(
      "Video blackout request was invalid; enabled and an exact expectedProject token are required.",
    );
  }
  return {
    enabled: params.enabled,
    expectedProject: params.expectedProject,
  };
};

const assertEffects = (effects: AgentBridgeEffects | undefined): AgentBridgeEffects => {
  if (!effects || typeof effects.refreshProjectAuthority !== "function"
    || typeof effects.refreshSnapshot !== "function") {
    throw new Error(
      "MCP video blackout requires canonical refresh hooks; no output was changed.",
    );
  }
  return effects;
};

/**
 * Execute the broker's video blackout intent through the same lease-bound
 * OutputControl action as the GUI.  The canonical project bundle is read
 * before selecting the lease, the live OutputControl fence is checked again
 * by `executeTargetBlackout`, and the receipt is confirmed by an exact
 * post-refresh authored-video readback.
 */
export async function executeAgentBridgeVideoBlackout(
  invoke: FrontendTauriInvoke,
  params: Record<string, unknown>,
  effects: AgentBridgeEffects | undefined,
  onMutationDispatch: () => void,
) {
  const parsed = parseVideoBlackoutRequest(params);
  const refresh = assertEffects(effects);
  const before = await invoke<ProjectAuthorityBundle>("get_project_authority_bundle", {
    expectedEpoch: parsed.expectedProject.project_epoch,
    expectedRevision: parsed.expectedProject.project_revision,
    expectedCheckpointHash: parsed.expectedProject.checkpoint_hash,
  });
  if (!sameProjectToken(projectToken(before), parsed.expectedProject)) {
    throw new Error("Expected project authority is no longer current; nothing was applied.");
  }

  const receipt = await executeTargetBlackout(invoke, "video", parsed.enabled, {
    expectedProject: parsed.expectedProject,
    onMutationDispatch,
  });
  await refresh.refreshProjectAuthority(receipt);
  await refresh.refreshSnapshot();

  const after = await invoke<ProjectAuthorityBundle>("get_project_authority_bundle", {});
  const receiptProject: ProjectToken = {
    project_epoch: receipt.fence_after.project_epoch,
    project_revision: receipt.fence_after.project_revision,
    checkpoint_hash: receipt.fence_after.project_checkpoint_hash,
  };
  // The public bundle deliberately strips the backend-only authored_video
  // image.  `snapshot.video.blackout` is the persisted video target bit; the
  // separate top-level/safety blackout flags must never be used as its
  // fallback because S0 is an independent authority.
  if (!sameProjectToken(projectToken(after), receiptProject)
    || typeof after.snapshot.video?.blackout !== "boolean"
    || after.snapshot.video.blackout !== parsed.enabled) {
    throw new Error(
      "Video blackout receipt was not confirmed by the exact canonical authored video state.",
    );
  }

  return {
    ok: true as const,
    project: projectToken(after),
    video: {
      blackout: after.snapshot.video.blackout,
      authored_blackout: after.snapshot.video.blackout,
      safety_blackout_engaged: after.snapshot.safety_blackout_engaged,
    },
    receipt,
    verification: "committed_project_state" as const,
  };
}
