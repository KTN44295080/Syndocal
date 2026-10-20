export interface VideoNdiSource {
  name: string;
  /** Opaque native identity; never replace with the display name. */
  endpoint_name: string;
}

export type VideoNdiDiscoveryState = "available" | "not_built" | "runtime_missing" | "error";

export interface VideoNdiDiscovery {
  state: VideoNdiDiscoveryState;
  sources: VideoNdiSource[];
  message: string | null;
}

const invalidDiscovery = () => new Error("NDI source discovery returned an invalid response.");

/** Reject malformed or ambiguous catalogs; no free-text or first-source fallback. */
export function parseVideoNdiDiscovery(value: unknown): VideoNdiDiscovery {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw invalidDiscovery();
  const catalog = value as Record<string, unknown>;
  if (typeof catalog.state !== "string" || !["available", "not_built", "runtime_missing", "error"].includes(catalog.state)
    || !Array.isArray(catalog.sources)
    || (catalog.message !== null && typeof catalog.message !== "string")) throw invalidDiscovery();
  const state = catalog.state as VideoNdiDiscoveryState;
  if (state !== "available" && catalog.sources.length !== 0) throw invalidDiscovery();
  const identities = new Set<string>();
  const sources = catalog.sources.map((entry): VideoNdiSource => {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) throw invalidDiscovery();
    const source = entry as Record<string, unknown>;
    if (typeof source.name !== "string" || !source.name.trim()
      || typeof source.endpoint_name !== "string" || !source.endpoint_name.trim()
      || source.endpoint_name.trim() !== source.endpoint_name
      || /[\u0000-\u001f\u007f-\u009f]/u.test(source.endpoint_name)
      || new TextEncoder().encode(source.endpoint_name).length > 4096
      || identities.has(source.endpoint_name)) throw invalidDiscovery();
    identities.add(source.endpoint_name);
    return { name: source.name, endpoint_name: source.endpoint_name };
  });
  return { state, sources, message: catalog.message as string | null };
}

export function ndiSourceAllowsAdd(
  endpoint: string,
  discovery: VideoNdiDiscovery | null,
  busy: boolean,
): boolean {
  return !busy && discovery?.state === "available"
    && discovery.sources.some((source) => source.endpoint_name === endpoint);
}

export function ndiDiscoveryStateMessage(discovery: VideoNdiDiscovery | null): string {
  switch (discovery?.state) {
    case "not_built": return "NDI input is not included in this build.";
    case "runtime_missing": return "NDI runtime not installed. Install the NDI runtime and restart Syndocal.";
    case "error": return "NDI source discovery failed. Refresh to try again.";
    case "available": return discovery.sources.length === 0
      ? "No NDI sources found. Check the sender and network, then refresh."
      : "Select the exact NDI source to add. Receiving starts after the layer is added.";
    default: return "Refresh to discover NDI sources on this network.";
  }
}
