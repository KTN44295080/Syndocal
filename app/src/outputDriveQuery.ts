import { hasOnlyActiveOutputLease, queryOutputLeaseAuthority } from "./outputControlController";
import { classifyOutputDrive, type OutputDriveStatus } from "./outputDriveStatus";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { OutputOwnershipStatus } from "./types";

/**
 * Read-only: the two authoritative queries the blackout command itself relies
 * on. Any failure is "unknown", which never blocks an operator action.
 */
export async function queryOutputDriveStatus(invoke: FrontendTauriInvoke): Promise<OutputDriveStatus> {
  try {
    const [ownership, leases] = await Promise.all([
      invoke<OutputOwnershipStatus>("get_output_ownership_status"),
      queryOutputLeaseAuthority(invoke),
    ]);
    return classifyOutputDrive(ownership, hasOnlyActiveOutputLease(leases, ["lighting", "video"]));
  } catch {
    return { kind: "unknown" };
  }
}
