import type { DmxOutputConfig } from "./types";

export interface NetworkDmxRoute {
  enabled: boolean;
  protocol: "ArtNet" | "Sacn";
  target_ip: string;
  port: number;
  universe: number;
}

export function validateNetworkDmxRoutes(value: unknown): asserts value is NetworkDmxRoute[] {
  if (!Array.isArray(value) || value.length > 256) throw new Error("At most 256 network routes are supported.");
  const keys = new Set<string>();
  for (const row of value) {
    if (!row || typeof row !== "object" || Object.keys(row).sort().join(",") !== "enabled,port,protocol,target_ip,universe"
      || typeof row.enabled !== "boolean" || (row.protocol !== "ArtNet" && row.protocol !== "Sacn")
      || typeof row.target_ip !== "string" || !Number.isInteger(row.port) || row.port < 1 || row.port > 65535
      || !Number.isInteger(row.universe) || row.universe < (row.protocol === "Sacn" ? 1 : 0)
      || row.universe > (row.protocol === "Sacn" ? 63999 : 32767)) throw new Error("Network DMX route fields are invalid.");
    const parts = row.target_ip.split(".");
    const multicast = row.protocol === "Sacn" && row.target_ip === "multicast";
    if (!multicast && (parts.length !== 4 || parts.some((v: string) => !/^(0|[1-9][0-9]{0,2})$/.test(v) || Number(v) > 255)
      || row.target_ip === "0.0.0.0")) throw new Error("Enter an IPv4 destination or sACN multicast.");
    const target = multicast ? "239.255." + (row.universe >> 8) + "." + (row.universe & 255) : row.target_ip;
    const key = [row.protocol, target, row.port, row.universe].join("|");
    if (row.enabled && keys.has(key)) throw new Error("Duplicate enabled DMX destination and universe.");
    if (row.enabled) keys.add(key);
  }
}

export function networkDmxRoutePayload(routes: DmxOutputConfig[]): NetworkDmxRoute[] {
  if (routes.some(row => row.serial_port !== "")) throw new Error("Network routes cannot carry a serial device alias.");
  const payload = routes.map(({enabled, protocol, target_ip, port, universe}) => ({enabled, protocol, target_ip, port, universe}));
  validateNetworkDmxRoutes(payload);
  return payload;
}
