import type { ExternalVideoTransportStatus, VideoLayerSummary } from "./types";

export type NdiInputHealthState = "connected" | "no_signal" | "lost" | "unavailable" | "disabled";
export interface NdiInputHealthRow {
  layerId: number;
  label: string;
  sourceName: string;
  state: NdiInputHealthState;
  detail: string;
  diagnostic: string | null;
}

export const ndiInputHealthTitle: Record<NdiInputHealthState, string> = {
  connected: "Connected",
  no_signal: "No signal",
  lost: "Signal lost",
  unavailable: "Unavailable",
  disabled: "Disabled",
};

const detailForState: Record<NdiInputHealthState, string> = {
  connected: "NDI video frames are being received.",
  no_signal: "Waiting for the first NDI video frame. Check the sender and network.",
  lost: "NDI signal was lost. Reconnecting to the same source automatically.",
  unavailable: "NDI input status is unavailable. Check the runtime and input settings.",
  disabled: "NDI input is disabled.",
};

/** Same UTF-8 byte-length identity as LiveVideoSourceIdentity::stable_key. */
export function ndiInputSourceIdentity(endpoint: string): string {
  const name = endpoint.trim();
  return `v1:3:ndi:${new TextEncoder().encode(name).length}:${name}`;
}

/** Never treat a started route as frame proof, or reuse another source's status. */
export function buildNdiInputHealthRows(
  layers: readonly VideoLayerSummary[],
  status: ExternalVideoTransportStatus | null,
): NdiInputHealthRow[] {
  return layers.filter((layer) => layer.source.kind === "Ndi").map((layer) => {
    const sourceName = layer.source.name?.trim() ?? "";
    const runtime = status?.live_sources.find((source) => source.route_id === layer.id
      && source.identity === ndiInputSourceIdentity(sourceName));
    const route = status?.active_routes.find((candidate) => candidate.direction === "Input"
      && candidate.backend_id === "ndi" && candidate.route_id === layer.id
      && candidate.endpoint_name === sourceName);
    // Faults carry a route id, so pair them only after the runtime's full
    // source identity has matched this layer's current endpoint.
    const fault = runtime ? [...(status?.live_source_faults ?? []), ...(status?.capture_faults ?? [])]
      .find((candidate) => candidate.route_id === layer.id && candidate.backend_id === "ndi") : undefined;
    const diagnostic = fault?.message ?? runtime?.issue ?? null;
    let state: NdiInputHealthState = "unavailable";
    if (!layer.state.enabled || runtime?.state === "Disabled") state = "disabled";
    // Physical output ownership is independent of receiving a live input.
    else if (runtime && sourceName) {
      if (diagnostic?.startsWith("NDI input signal lost for ")) state = "lost";
      else if (diagnostic?.startsWith("NDI input has no signal for ")) state = "no_signal";
      else if (runtime.state === "Live" && route && !diagnostic) state = "connected";
      else if ((runtime.state === "Ready" || runtime.state === "Starting") && !diagnostic) state = "no_signal";
    }
    return { layerId: layer.id, label: layer.label, sourceName, state, detail: detailForState[state], diagnostic };
  });
}
