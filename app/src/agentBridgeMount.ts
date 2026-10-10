import { onCleanup, onMount } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke as transport } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import { startDeferredAgentBridge } from "./agentBridgeBootstrap";

/** Only the native main window owns the agent request receiver. */
export function mountAgentBridge(
  eligible: boolean,
  invoke: FrontendTauriInvoke,
  report: (message: string) => void,
) {
  onMount(() => {
    if (!eligible || getCurrentWindow().label !== "main") return;
    const bridge = startDeferredAgentBridge(invoke, listen, report, transport);
    onCleanup(bridge.dispose);
  });
}
