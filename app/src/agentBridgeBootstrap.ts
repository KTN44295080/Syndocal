import type { startAgentBridgeRuntime } from "./agentBridgeRuntime";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

type BridgeModule = { startAgentBridgeRuntime: typeof startAgentBridgeRuntime };

/** Load the optional native agent interface without retaining an unmounted UI. */
export function startDeferredAgentBridge(
  invoke: FrontendTauriInvoke,
  listen: Parameters<typeof startAgentBridgeRuntime>[1],
  report: Parameters<typeof startAgentBridgeRuntime>[2],
  transport: FrontendTauriInvoke,
  effects: Parameters<typeof startAgentBridgeRuntime>[4],
  load: () => Promise<BridgeModule> = () => import("./agentBridgeRuntime"),
) {
  let disposed = false;
  let bridge: ReturnType<typeof startAgentBridgeRuntime> | undefined;
  const ready = load().then(module => {
    if (disposed) return;
    bridge = module.startAgentBridgeRuntime(invoke, listen, report, transport, effects);
    return bridge.ready;
  }).catch(error => {
    if (!disposed) report(`Agent bridge unavailable: ${String(error)}`);
  });
  return {
    ready,
    dispose() {
      if (disposed) return;
      disposed = true;
      bridge?.dispose();
    },
  };
}
