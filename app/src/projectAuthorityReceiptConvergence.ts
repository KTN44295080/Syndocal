import type { OutputControlReceipt } from "./outputControlController";
import {
  projectAuthorityTokenIsCurrent,
  type ProjectAuthorityToken,
} from "./projectAuthority";

export type ProjectAuthorityReceiptConvergenceOptions = {
  currentAuthority: () => ProjectAuthorityToken;
  inFlightAuthorityPoll: () => Promise<unknown> | null;
  pollProjectAuthorityBundle: () => Promise<unknown>;
  /** Human-readable operation name used when convergence fails. */
  operationLabel?: string;
};

const authorityIsStrictlyAfter = (
  expected: ProjectAuthorityToken,
  current: ProjectAuthorityToken,
) => current.project_epoch > expected.project_epoch
  || (current.project_epoch === expected.project_epoch
    && current.project_revision > expected.project_revision);

/**
 * Wait for a pre-receipt authority poll, then force one canonical read when
 * the output-control receipt's fence is not visible yet. A later authority
 * is also valid: another accepted publication may have advanced the project
 * before this convergence read completed.
 */
export const createProjectAuthorityReceiptConvergence = (
  options: ProjectAuthorityReceiptConvergenceOptions,
) => async (
  receipt: OutputControlReceipt,
  operationLabel = options.operationLabel ?? "Target blackout",
): Promise<void> => {
  const expected = {
    project_epoch: receipt.fence_after.project_epoch,
    project_revision: receipt.fence_after.project_revision,
    checkpoint_hash: receipt.fence_after.project_checkpoint_hash,
  };
  // A periodic poll can have captured the pre-commit token. Let that read
  // settle before requesting a fresh canonical bundle so its null result
  // cannot be mistaken for post-commit convergence.
  const inFlight = options.inFlightAuthorityPoll();
  if (inFlight) await inFlight;
  const current = options.currentAuthority();
  if (projectAuthorityTokenIsCurrent(expected, current)) return;
  await options.pollProjectAuthorityBundle();
  const converged = options.currentAuthority();
  if (!projectAuthorityTokenIsCurrent(expected, converged)
    && !authorityIsStrictlyAfter(expected, converged)) {
    throw new Error(
      `${operationLabel} applied, but canonical project authority did not converge; snapshot was not refreshed.`,
    );
  }
};
