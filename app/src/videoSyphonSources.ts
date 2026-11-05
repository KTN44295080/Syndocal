export interface VideoSyphonServer {
  name: string;
  app_name: string;
  uuid: string;
}

const invalidCatalog = () => new Error("Syphon server discovery returned an invalid response.");
const boundedString = (value: unknown, maximum: number, empty = false): value is string =>
  typeof value === "string" && (empty || value.trim().length > 0)
  && !/[\u0000-\u001f\u007f-\u009f]/u.test(value)
  && new TextEncoder().encode(value).length <= maximum;

/** Validate the complete bounded directory; duplicate UUIDs cannot be selected. */
export function parseVideoSyphonServers(value: unknown): VideoSyphonServer[] {
  if (!Array.isArray(value) || value.length > 64) throw invalidCatalog();
  const identities = new Set<string>();
  return value.map((entry) => {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)
      || !boundedString(entry.name, 1024, true) || !boundedString(entry.app_name, 1024)
      || !boundedString(entry.uuid, 1024) || entry.uuid.trim() !== entry.uuid
      || identities.has(entry.uuid)) throw invalidCatalog();
    identities.add(entry.uuid);
    const server = { uuid: entry.uuid, name: entry.name, app_name: entry.app_name };
    if (new TextEncoder().encode(syphonServerEndpoint(server)).length > 4096) throw invalidCatalog();
    return server;
  });
}

/** Existing source.name field carries the selected server and restart identity. */
export function syphonServerEndpoint(server: VideoSyphonServer): string {
  return JSON.stringify({ uuid: server.uuid, name: server.name, app_name: server.app_name });
}

export function syphonServerAllowsAdd(endpoint: string, servers: VideoSyphonServer[] | null, busy: boolean): boolean {
  return !busy && !!servers?.some(server => syphonServerEndpoint(server) === endpoint);
}

export function syphonServerLabel(server: VideoSyphonServer): string {
  return server.name ? `${server.app_name} / ${server.name}` : server.app_name;
}

export function syphonEndpointLabel(endpoint: string): string | null {
  try { return syphonServerLabel(parseVideoSyphonServers([JSON.parse(endpoint)])[0]); }
  catch { return null; }
}
