import { onCleanup, onMount } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke as transport } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { AgentBridgeEffects } from "./agentBridgeBlackout";
import { startDeferredAgentBridge } from "./agentBridgeBootstrap";

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
    const bridge = startDeferredAgentBridge(invoke, listen, report, transport, {
      refreshProjectAuthority, refreshSnapshot,
    });
    onCleanup(bridge.dispose);
  });
}
