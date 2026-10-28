import type {
  DmxOutputConfig,
  DmxOutputRouteTelemetry,
} from "./types";

export type DmxUniverseTone = "ready" | "idle" | "error";

/** The State cell of one row in the DMX universes table. */
export interface DmxUniverseState {
  label: string;
  tone: DmxUniverseTone;
}

/** An applied Art-Net / sACN route and its last send result. */
export function networkDmxRouteState(
  route: DmxOutputConfig,
  telemetry: DmxOutputRouteTelemetry | undefined,
): DmxUniverseState {
  if (!route.enabled) return { label: "Disabled", tone: "idle" };
  if (telemetry?.reconnecting || (telemetry && telemetry.attempted && !telemetry.success)) {
    return { label: telemetry.error ? `Send failed · ${telemetry.error}` : "Send failed", tone: "error" };
  }
  if (telemetry?.success) return { label: "Sending", tone: "ready" };
  return { label: "Enabled", tone: "idle" };
}

/** Whether a draft network row carries exactly the applied route's settings. */
export function sameNetworkDmxRoute(left: DmxOutputConfig | undefined, right: DmxOutputConfig | undefined): boolean {
  return Boolean(left && right)
    && left!.enabled === right!.enabled
    && left!.protocol === right!.protocol
    && left!.target_ip === right!.target_ip
    && left!.port === right!.port
    && left!.universe === right!.universe;
}
