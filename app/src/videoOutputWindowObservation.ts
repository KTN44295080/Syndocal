// Exact, app-owned observation contract for native Display output windows.
//
// This parser intentionally accepts only the current schema and receives data
// directly from Tauri `get_video_output_window_observation_v1`.  It must never
// be fed operator-authored files, environment variables, or arbitrary JSON.

export const VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_COMMAND =
  "get_video_output_window_observation_v1" as const;
export const VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SCHEMA_VERSION = 1 as const;
export const VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SOURCE =
  "app-owned-read-only" as const;

export interface VideoOutputWindowObservationEntryV1 {
  output_id: string;
  label: string;
  live_open: boolean;
  live_window_label: string;
  native_window_handle_decimal: string | null;
}

export interface VideoOutputWindowObservationV1 {
  schema_version: typeof VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SCHEMA_VERSION;
  source: typeof VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SOURCE;
  outputs: VideoOutputWindowObservationEntryV1[];
}

export type VideoOutputWindowObservationInvoke = <T>(
  command: typeof VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_COMMAND,
) => Promise<T>;

const exactKeys = (value: Record<string, unknown>, expected: readonly string[]) => {
  const keys = Object.keys(value).sort();
  const sortedExpected = [...expected].sort();
  return keys.length === sortedExpected.length
    && keys.every((key, index) => key === sortedExpected[index]);
};

const asRecord = (value: unknown, context: string): Record<string, unknown> => {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`Fail closed: ${context} must be an object.`);
  }
  return value as Record<string, unknown>;
};

const U64_MAX = 18_446_744_073_709_551_615n;

const exactCanonicalPositiveDecimal = (value: unknown, context: string): string => {
  if (typeof value !== "string" || !/^[1-9][0-9]*$/.test(value)) {
    throw new Error(`Fail closed: ${context} must be one canonical positive decimal string.`);
  }
  if (BigInt(value) > U64_MAX) {
    throw new Error(`Fail closed: ${context} exceeds the supported unsigned 64-bit range.`);
  }
  return value;
};

const exactString = (value: unknown, context: string): string => {
  if (typeof value !== "string" || value.length === 0 || /[\r\n]/.test(value)) {
    throw new Error(`Fail closed: ${context} must be one non-empty single-line string.`);
  }
  return value;
};

/**
 * Parse the direct result of the app-owned command. This is deliberately a
 * strict clean-break parser: missing, future, unknown, proxy, or test-pattern
 * fields are rejected rather than normalised into a usable observation.
 */
const parseVideoOutputWindowObservationV1 = (
  value: unknown,
): VideoOutputWindowObservationV1 => {
  const root = asRecord(value, "video-output window observation");
  if (!exactKeys(root, ["schema_version", "source", "outputs"])) {
    throw new Error("Fail closed: video-output window observation has an unsupported field set.");
  }
  if (root.schema_version !== VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SCHEMA_VERSION) {
    throw new Error("Fail closed: video-output window observation schema_version is unsupported.");
  }
  if (root.source !== VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SOURCE) {
    throw new Error("Fail closed: video-output window observation is not app-owned-read-only.");
  }
  if (!Array.isArray(root.outputs)) {
    throw new Error("Fail closed: video-output window observation outputs must be an array.");
  }

  const seenOutputIds = new Set<string>();
  const outputs = root.outputs.map((candidate, index) => {
    const item = asRecord(candidate, `video-output observation output ${index}`);
    if (!exactKeys(item, [
      "output_id",
      "label",
      "live_open",
      "live_window_label",
      "native_window_handle_decimal",
    ])) {
      throw new Error(`Fail closed: video-output observation output ${index} has an unsupported field set.`);
    }
    const outputId = exactCanonicalPositiveDecimal(
      item.output_id,
      `video-output observation output ${index} ID`,
    );
    if (seenOutputIds.has(outputId)) {
      throw new Error(`Fail closed: video-output observation duplicates output ID ${outputId}.`);
    }
    seenOutputIds.add(outputId);
    const label = exactString(item.label, `video-output observation output ${index} label`);
    const liveWindowLabel = exactString(
      item.live_window_label,
      `video-output observation output ${index} live window label`,
    );
    if (liveWindowLabel !== `video-output-${outputId}`) {
      throw new Error(
        `Fail closed: video-output observation output ${outputId} live window label is not its exact live label.`,
      );
    }
    if (typeof item.live_open !== "boolean") {
      throw new Error(`Fail closed: video-output observation output ${outputId} live_open must be boolean.`);
    }
    const hwnd = item.native_window_handle_decimal;
    if (item.live_open) {
      return {
        output_id: outputId,
        label,
        live_open: true,
        live_window_label: liveWindowLabel,
        native_window_handle_decimal: exactCanonicalPositiveDecimal(
          hwnd,
          `video-output observation output ${outputId} HWND`,
        ),
      };
    }
    if (hwnd !== null) {
      throw new Error(`Fail closed: closed video-output observation output ${outputId} must have null HWND.`);
    }
    return {
      output_id: outputId,
      label,
      live_open: false,
      live_window_label: liveWindowLabel,
      native_window_handle_decimal: null,
    };
  });

  return {
    schema_version: VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SCHEMA_VERSION,
    source: VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_SOURCE,
    outputs,
  };
};

/** Invoke only the registered app command and immediately enforce its schema. */
export const requestVideoOutputWindowObservationV1 = async (
  invoke: VideoOutputWindowObservationInvoke,
): Promise<VideoOutputWindowObservationV1> => (
  parseVideoOutputWindowObservationV1(
    await invoke<unknown>(VIDEO_OUTPUT_WINDOW_OBSERVATION_V1_COMMAND),
  )
);
