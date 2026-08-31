import type {
  SerialDmxMachineBindingStatus,
} from "./types";

type UnknownRecord = Record<string, unknown>;

const bindingStates = new Set<SerialDmxMachineBindingStatus["state"]>([
  "missing_selection",
  "selected_and_present",
  "stale_or_missing",
  "ambiguous",
  "blocked_persistence",
]);

const isRecord = (value: unknown): value is UnknownRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasExactKeys = (value: UnknownRecord, expected: readonly string[], label: string): string | null => {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    return `${label} has missing or unexpected fields`;
  }
  return null;
};

const nonemptyText = (value: unknown, label: string): string | null =>
  typeof value === "string" && value.trim().length > 0
    ? null
    : `${label} must be a nonempty string`;

const nonzeroU16 = (value: unknown, label: string): string | null =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0 && value <= 0xffff
    ? null
    : `${label} must be a nonzero u16`;

const routeRevision = (value: unknown, label: string): string | null =>
  typeof value === "string" && /^(?:[1-9]\d*)$/.test(value)
    ? null
    : `${label} must be a nonzero decimal revision`;

const validateIdentity = (value: unknown): string | null => {
  if (!isRecord(value)) return "binding selected identity must be an object";
  const keyError = hasExactKeys(value, [
    "port_name",
    "port_type",
    "usb_vid",
    "usb_pid",
    "serial_number",
    "manufacturer",
    "product",
    "windows_device_instance_id",
  ], "binding selected identity");
  if (keyError) return keyError;
  for (const [key, label] of [
    ["port_name", "binding COM path"],
    ["port_type", "binding serial port type"],
    ["serial_number", "binding hardware serial"],
    ["manufacturer", "binding manufacturer"],
    ["product", "binding product"],
    ["windows_device_instance_id", "binding Windows PnP instance"],
  ] as const) {
    const textError = nonemptyText(value[key], label);
    if (textError) return textError;
  }
  return nonzeroU16(value.usb_vid, "binding USB VID")
    ?? nonzeroU16(value.usb_pid, "binding USB PID");
};

/**
 * Tauri type parameters disappear at the IPC boundary. Validate every status
 * field before it can enable an output action; rejection returns a redacted
 * field-level reason suitable for a diagnostic log, never raw device data.
 */
export function validateSerialDmxStatusSnapshot(
  binding: unknown,
  route: unknown,
): string | null {
  if (!isRecord(binding)) return "binding status must be an object";
  let error = hasExactKeys(binding, ["state", "selected", "detail", "routeStatusRevision"], "binding status");
  if (error) return error;
  if (typeof binding.state !== "string" || !bindingStates.has(binding.state as SerialDmxMachineBindingStatus["state"])) {
    return "binding state is unsupported";
  }
  error = nonemptyText(binding.detail, "binding detail");
  if (error) return error;
  error = routeRevision(binding.routeStatusRevision, "binding route status");
  if (error) return error;
  const requiresSelectedIdentity = binding.state === "selected_and_present"
    || binding.state === "stale_or_missing"
    || binding.state === "ambiguous";
  if (requiresSelectedIdentity) {
    error = validateIdentity(binding.selected);
    if (error) return error;
  } else if (binding.selected !== null) {
    return "binding state requires a null selected identity";
  }

  if (!isRecord(route)) return "USB-DMX route status must be an object";
  error = hasExactKeys(route, [
    "routeStatusRevision",
    "active",
    "zeroFrameQueued",
    "zeroFramePhysicalWriteCompleted",
    "liveFrameQueued",
    "workerShutdownCompleted",
    "faulted",
    "artnetMirrorLive",
    "artnetMirrorDetail",
    "detail",
  ], "USB-DMX route status");
  if (error) return error;
  for (const key of [
    "active",
    "zeroFrameQueued",
    "zeroFramePhysicalWriteCompleted",
    "liveFrameQueued",
    "workerShutdownCompleted",
    "faulted",
    "artnetMirrorLive",
  ] as const) {
    if (typeof route[key] !== "boolean") return `USB-DMX route ${key} must be boolean`;
  }
  error = routeRevision(route.routeStatusRevision, "USB-DMX route status");
  if (error) return error;
  if (binding.routeStatusRevision !== route.routeStatusRevision) {
    return "USB-DMX binding and route status revisions do not form one coherent fence";
  }
  error = nonemptyText(route.detail, "USB-DMX route detail");
  if (error) return error;
  error = nonemptyText(route.artnetMirrorDetail, "Art-Net mirror detail");
  if (error) return error;
  if (route.faulted && (route.active || route.liveFrameQueued)) {
    return "faulted USB-DMX route cannot be active or live";
  }
  if (route.zeroFramePhysicalWriteCompleted && !route.zeroFrameQueued) {
    return "physical USB-DMX zero completion requires queued zero";
  }
  if (route.liveFrameQueued && !route.active) {
    return "live USB-DMX frame requires an active worker";
  }
  if (route.active && route.workerShutdownCompleted) {
    return "active USB-DMX worker cannot report completed shutdown";
  }
  if (!route.active && !route.faulted && !route.workerShutdownCompleted) {
    return "stopped nonfaulted USB-DMX route requires completed worker shutdown";
  }
  return null;
}

/**
 * Event payloads carry only the native route revision. Anything else is not
 * an authority update: the caller stays Unknown and waits for a fresh strict
 * binding+route query pair.
 */
export function parseSerialDmxRouteStatusEventRevision(payload: unknown): string | null {
  if (!isRecord(payload)) return null;
  if (hasExactKeys(payload, ["routeStatusRevision"], "USB-DMX status event")) return null;
  return routeRevision(payload.routeStatusRevision, "USB-DMX status event")
    ? null
    : payload.routeStatusRevision as string;
}
