import type {
  FrontendTauriInvoke,
  FrontendTauriInvokeCommand,
} from "./tauriInvokeCommands";
import {
  executeAgentBridgeRequest,
  type AgentBridgeEffects,
  type AgentBridgeRequest,
} from "./agentBridgeTools";

type Listen = (event: string, handler: (event: { payload: AgentBridgeRequest }) => void) => Promise<() => void>;

/** Events only wake the receiver. Native claim supplies the canonical, single-use intent. */
export function startAgentBridgeRuntime(
  invoke: FrontendTauriInvoke,
  listen: Listen,
  report: (error: string) => void,
  transport: FrontendTauriInvoke = <T>(
    command: FrontendTauriInvokeCommand,
    args?: Record<string, unknown>,
  ) => invoke<T>(command, args),
  effects?: AgentBridgeEffects,
) {
  let disposed = false;
  let unsubscribe: (() => void) | undefined;
  let generation: Promise<number>;
  const ready = (async () => {
    unsubscribe = await listen("syndocal://agent-request-v1", event => {
      void (async () => {
        const current = await generation;
        if (disposed || event.payload.rendererGeneration !== current) return;
        let claimed: AgentBridgeRequest;
        try {
          claimed = await transport<AgentBridgeRequest>("agent_bridge_claim_v1", {
            rendererGeneration: current, requestId: event.payload.requestId,
          });
        } catch { return; } // Forged, duplicate and obsolete wake hints never execute.
        if (disposed) return;
        const result = await executeAgentBridgeRequest(invoke, claimed, effects);
        await transport("agent_bridge_complete_v1", {
          rendererGeneration: current, requestId: claimed.requestId, result,
        });
      })().catch(error => report(`Agent bridge: ${String(error)}`));
    });
    if (disposed) { unsubscribe(); return; }
    generation = transport<number>("agent_bridge_register_v1", {});
    await generation;
  })().catch(error => report(`Agent bridge unavailable: ${String(error)}`));
  return { ready, dispose: () => { disposed = true; unsubscribe?.(); } };
}
