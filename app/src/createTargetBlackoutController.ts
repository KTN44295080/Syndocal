import type { createSafetyBlackoutRuntimeController } from "./safetyBlackoutRuntimeController";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import {
  executeTargetBlackout,
  executeBlackoutRelease,
  type OutputControlTargetRole,
} from "./outputControlController";

/** Operator blackout is authored target state. Emergency S0 is independent. */
export function createTargetBlackoutController(options: {
  invoke: FrontendTauriInvoke;
  safety: ReturnType<typeof createSafetyBlackoutRuntimeController>;
  refreshSnapshot: () => Promise<unknown>;
  setMessage: (message: string) => unknown;
}) {
  let pending = false;
  const safety = options.safety;
  const set = async (target: OutputControlTargetRole, enabled: boolean) => {
    if (pending) return;
    pending = true;
    try {
      await executeTargetBlackout(options.invoke, target, enabled);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
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
