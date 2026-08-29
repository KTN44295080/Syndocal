// Camera profile DTOs and fail-closed presentation helpers.  Camera endpoint
// names are opaque backend identities: the UI may display them as status, but
// it must never construct or substitute one from a device name or index.

export interface VideoCameraProfileDescriptor {
  device_identity: string;
  device_name: string;
  profile_identity: string;
  width: number;
  height: number;
  frame_rate_numerator: number;
  frame_rate_denominator: number;
  frame_rate_label: string;
  pixel_format: string | null;
  codec: string | null;
  endpoint_name: string;
}

export interface VideoCameraProbeResult {
  success: boolean;
  endpoint_name: string;
  actual_width: number;
  actual_height: number;
  frame_rate_numerator: number;
  frame_rate_denominator: number;
  frame_rate_label: string;
  pixel_format: string | null;
  codec: string | null;
}

export interface VideoCameraDeviceOption {
  device_identity: string;
  device_name: string;
  profile_count: number;
}

export type VideoCameraProbeStatus = "idle" | "loading" | "success" | "error" | "stale";

export interface VideoCameraProbeState {
  status: VideoCameraProbeStatus;
  endpoint_name: string | null;
  profile_identity: string | null;
  result: VideoCameraProbeResult | null;
  message: string | null;
}

const profileKeys = [
  "device_identity",
  "device_name",
  "profile_identity",
  "width",
  "height",
  "frame_rate_numerator",
  "frame_rate_denominator",
  "frame_rate_label",
  "pixel_format",
  "codec",
  "endpoint_name",
] as const;

const probeKeys = [
  "success",
  "endpoint_name",
  "actual_width",
  "actual_height",
  "frame_rate_numerator",
  "frame_rate_denominator",
  "frame_rate_label",
  "pixel_format",
  "codec",
] as const;

type RecordLike = Record<string, unknown>;

const isRecord = (value: unknown): value is RecordLike =>
  Boolean(value) && typeof value === "object" && !Array.isArray(value);

const hasExactKeys = (value: RecordLike, keys: readonly string[]) => {
  const actual = Object.keys(value);
  return actual.length === keys.length && keys.every((key) => Object.prototype.hasOwnProperty.call(value, key));
};

const nonEmptyString = (value: unknown): value is string =>
  typeof value === "string" && value.trim().length > 0 && !/[\r\n]/u.test(value);

const nullableString = (value: unknown): value is string | null =>
  value === null || nonEmptyString(value);

const positiveInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const isProfileDescriptor = (value: unknown): value is VideoCameraProfileDescriptor => {
  if (!isRecord(value) || !hasExactKeys(value, profileKeys)) return false;
  return nonEmptyString(value.device_identity)
    && nonEmptyString(value.device_name)
    && nonEmptyString(value.profile_identity)
    && positiveInteger(value.width)
    && positiveInteger(value.height)
    && positiveInteger(value.frame_rate_numerator)
    && positiveInteger(value.frame_rate_denominator)
    && nonEmptyString(value.frame_rate_label)
    && nullableString(value.pixel_format)
    && nullableString(value.codec)
    && nonEmptyString(value.endpoint_name);
};

const isProbeResult = (value: unknown): value is VideoCameraProbeResult => {
  if (!isRecord(value) || !hasExactKeys(value, probeKeys)) return false;
  return typeof value.success === "boolean"
    && nonEmptyString(value.endpoint_name)
    && positiveInteger(value.actual_width)
    && positiveInteger(value.actual_height)
    && positiveInteger(value.frame_rate_numerator)
    && positiveInteger(value.frame_rate_denominator)
    && nonEmptyString(value.frame_rate_label)
    && nullableString(value.pixel_format)
    && nullableString(value.codec);
};

/**
 * Validate the native catalog without fabricating a fallback profile.  A
 * malformed entry or duplicate opaque endpoint invalidates the whole catalog.
 */
export const parseVideoCameraProfiles = (value: unknown): VideoCameraProfileDescriptor[] => {
  if (!Array.isArray(value)) throw new Error("Camera profile catalog is not an array.");
  const profiles = value.map((entry, index) => {
    if (!isProfileDescriptor(entry)) {
      throw new Error(`Camera profile catalog entry ${index + 1} is invalid.`);
    }
    return entry;
  });
  const endpoints = new Set<string>();
  for (const profile of profiles) {
    if (endpoints.has(profile.endpoint_name)) {
      throw new Error(`Camera profile catalog contains duplicate endpoint '${profile.endpoint_name}'.`);
    }
    endpoints.add(profile.endpoint_name);
  }
  return profiles;
};

/**
 * Validate a probe response and bind it to the exact endpoint requested by
 * the current selection.  A successful response for another endpoint is
 * never promoted to a usable probe.
 */
export const parseVideoCameraProbe = (
  value: unknown,
  expectedEndpointName: string,
): VideoCameraProbeResult => {
  if (!isProbeResult(value)) throw new Error("Camera profile probe returned an invalid response.");
  if (value.endpoint_name !== expectedEndpointName) {
    throw new Error("Camera profile probe returned a different endpoint; test the current selection again.");
  }
  if (!value.success) throw new Error("Camera profile probe did not succeed for the selected profile.");
  return value;
};

export const cameraDeviceOptions = (
  profiles: readonly VideoCameraProfileDescriptor[],
): VideoCameraDeviceOption[] => {
  const devices = new Map<string, VideoCameraDeviceOption>();
  for (const profile of profiles) {
    const current = devices.get(profile.device_identity);
    if (current) {
      current.profile_count += 1;
    } else {
      devices.set(profile.device_identity, {
        device_identity: profile.device_identity,
        device_name: profile.device_name,
        profile_count: 1,
      });
    }
  }
  return [...devices.values()];
};

export const cameraProfileDisplayLabel = (profile: VideoCameraProfileDescriptor): string =>
  `${profile.width}x${profile.height} · ${profile.frame_rate_label}${profile.pixel_format ? ` · ${profile.pixel_format}` : ""}${profile.codec ? ` · ${profile.codec}` : ""}`;

export const cameraProbeDisplayLabel = (probe: VideoCameraProbeResult): string =>
  `${probe.actual_width}x${probe.actual_height} · ${probe.frame_rate_label}${probe.pixel_format ? ` · ${probe.pixel_format}` : ""}${probe.codec ? ` · ${probe.codec}` : ""}`;

export const idleCameraProbeState = (): VideoCameraProbeState => ({
  status: "idle",
  endpoint_name: null,
  profile_identity: null,
  result: null,
  message: null,
});

export const invalidatedCameraProbeState = (
  message: string,
  selection?: { endpoint_name?: string | null; profile_identity?: string | null },
): VideoCameraProbeState => ({
  status: "stale",
  endpoint_name: selection?.endpoint_name ?? null,
  profile_identity: selection?.profile_identity ?? null,
  result: null,
  message,
});

export const cameraProbeAllowsAdd = (
  sourceKind: string,
  currentPath: string,
  selectedProfile: VideoCameraProfileDescriptor | undefined,
  probe: VideoCameraProbeState,
): boolean => {
  if (sourceKind !== "Camera") return true;
  return Boolean(
    selectedProfile
      && currentPath === selectedProfile.endpoint_name
      && probe.status === "success"
      && probe.endpoint_name === selectedProfile.endpoint_name
      && probe.profile_identity === selectedProfile.profile_identity
      && probe.result?.success === true
      && probe.result.endpoint_name === selectedProfile.endpoint_name,
  );
};
