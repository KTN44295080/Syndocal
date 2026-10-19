import type { createSafetyBlackoutRuntimeController } from "./safetyBlackoutRuntimeController";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import { BLACKOUT_STATUS_KEY, blackoutNotAppliedMessage, type OutputDriveStatus } from "./outputDriveStatus";
import {
  executeTargetBlackout,
  executeBlackoutRelease,
  type OutputControlTargetRole,
  type OutputControlReceipt,
} from "./outputControlController";

/** Operator blackout is authored target state. Emergency S0 is independent. */
export function createTargetBlackoutController(options: {
  invoke: FrontendTauriInvoke;
  safety: ReturnType<typeof createSafetyBlackoutRuntimeController>;
  safetyBlackoutEngaged?: () => boolean;
  refreshProjectAuthority?: (receipt: OutputControlReceipt) => Promise<unknown>;
  refreshSnapshot: () => Promise<unknown>;
  setMessage: (message: string, key?: string) => unknown;
  /** Drop the footer message owned by `key` (a no-op when another message replaced it). */
  clearMessage?: (key: string) => unknown;
  /**
   * Fresh read of whether this machine can change output state. A blocked
   * answer stops an ordinary blackout before any command is sent and names the
   * reason; an unknown answer never blocks.
   */
  checkOutputDrive?: () => Promise<OutputDriveStatus>;
}) {
  let pending = false;
  const safety = options.safety;
  const blockedDrive = async (): Promise<Extract<OutputDriveStatus, { kind: "blocked" }> | null> => {
    if (!options.checkOutputDrive) return null;
    try {
      const drive = await options.checkOutputDrive();
      return drive.kind === "blocked" ? drive : null;
    } catch { return null; }
  };
  const set = async (target: OutputControlTargetRole, enabled: boolean) => {
    if (pending) return;
    pending = true;
    let runtimeReleased = false;
    try {
      // The runtime S0 latch release stays reachable; only an ordinary target
      // blackout that cannot reach any output is refused here, before any command.
      const releasesRuntimeLatch = !enabled && target !== "video" && options.safetyBlackoutEngaged?.();
      if (!releasesRuntimeLatch && options.checkOutputDrive) {
        const blocked = await blockedDrive();
        if (blocked) {
          options.setMessage(blackoutNotAppliedMessage(blocked), BLACKOUT_STATUS_KEY);
          return;
        }
      }
      let expectedProject: { project_epoch: number; project_revision: number; checkpoint_hash: string } | undefined;
      if (!enabled && target !== "video" && options.safetyBlackoutEngaged?.()) {
        const release = await executeBlackoutRelease(options.invoke);
        runtimeReleased = true;
        expectedProject = { project_epoch: release.fence_after.project_epoch, project_revision: release.fence_after.project_revision, checkpoint_hash: release.fence_after.project_checkpoint_hash };
      }
      const receipt = await executeTargetBlackout(options.invoke, target, enabled, { expectedProject });
      await options.refreshProjectAuthority?.(receipt);
      await options.refreshSnapshot();
      options.clearMessage?.(BLACKOUT_STATUS_KEY);
    } catch (error) {
      // A refusal caused by the output role is explained as such, not as the
      // raw lease error; the keyed message is retired when the state changes.
      const blocked = runtimeReleased ? null : await blockedDrive();
      options.setMessage(
        blocked ? blackoutNotAppliedMessage(blocked)
          : runtimeReleased ? `Runtime blackout cleared; target blackout update failed: ${String(error)}` : String(error),
        BLACKOUT_STATUS_KEY,
      );
    } finally {
      pending = false;
    }
  };
  const releaseSafetyBlackout = async () => {
    if (pending) return;
    pending = true;
    try {
      await executeBlackoutRelease(options.invoke);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
    finally { pending = false; }
  };
  return {
    async engageSafetyBlackout() {
      // Emergency engage must never wait for an ordinary pending toggle.
      try { await safety.engage(); await options.refreshSnapshot(); }
      catch (error) { options.setMessage(String(error)); }
    },
    releaseSafetyBlackout,
    setLightingBlackout: (enabled: boolean) => set("lighting", enabled),
    setVideoBlackout: (enabled: boolean) => set("video", enabled),
    setAllBlackout: (enabled: boolean) => set("both", enabled),
  };
}
