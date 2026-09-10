import { onCleanup, onMount } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke as transport } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { FrontendTauriInvoke, FrontendTauriInvokeCommand } from "./tauriInvokeCommands";
import type { AgentBridgeEffects } from "./agentBridgeBlackout";
import { startDeferredAgentBridge } from "./agentBridgeBootstrap";

// Agent diagnostics must not wait behind the project-transaction owner barrier.
// Keep this allowlist read-only and narrow; mutations and canonical execution
// continue through the owner/lock-aware facade supplied by App.tsx.
const agentBridgeDirectReadCommands = new Set<FrontendTauriInvokeCommand>([
  "get_project_authority_bundle",
  "get_output_ownership_status",
  "query_output_control_authority_v1",
  "get_control_plane_canonical_registry",
  "video_output_recording_status",
]);

/** Only the native main window owns the agent request receiver. */
export function mountAgentBridge(
  eligible: boolean | (() => boolean),
  invoke: FrontendTauriInvoke,
  report: (message: string) => void,
  refreshProjectAuthority: AgentBridgeEffects["refreshProjectAuthority"],
  refreshSnapshot: AgentBridgeEffects["refreshSnapshot"],
) {
  onMount(() => {
    const isEligible = typeof eligible === "function" ? eligible() : eligible;
    if (!isEligible || getCurrentWindow().label !== "main") return;
    const bridgeInvoke: FrontendTauriInvoke = <T>(command: FrontendTauriInvokeCommand, args?: Record<string, unknown>) =>
      agentBridgeDirectReadCommands.has(command)
        ? transport<T>(command, args)
        : invoke<T>(command, args);
    const bridge = startDeferredAgentBridge(bridgeInvoke, listen, report, transport, {
      refreshProjectAuthority, refreshSnapshot,
    });
    onCleanup(bridge.dispose);
  });
}
