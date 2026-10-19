import type { FaderValueMap } from "./createFaderValueController";
import { performRoutedInputFromPayload, type PerformRoutedInput } from "./controlMappingPerform";

export interface MidiFaderConnection {
  schema_version: 1;
  project_epoch: number;
  input_session: number;
}
const positive = (n: unknown): n is number => typeof n === "number" && Number.isSafeInteger(n) && n > 0;
export function midiFaderConnection(value: unknown): MidiFaderConnection {
  const v = value as Partial<MidiFaderConnection> | null;
  if (!v || v.schema_version !== 1 || (typeof v.project_epoch !== "number" || !Number.isSafeInteger(v.project_epoch) || v.project_epoch < 0) || !positive(v.input_session)) {
    throw new Error("MIDI input connection identity was invalid.");
  }
  return v as MidiFaderConnection;
}

/** Native events project accepted input, not proof of physical output.
 * Exact connection and sequence fences reject late/reordered input. Fader
 * values and renderer-routed Perform inputs share one connection and one
 * sequence, as the native worker numbers them in a single series. */
export function createMidiFaderInput(options: {
  projectEpoch: () => number;
  apply: (values: FaderValueMap) => void;
  /** Runs an accepted Perform input through the renderer's own commands. */
  applyRouted?: (input: PerformRoutedInput) => void;
}) {
  let connection: MidiFaderConnection | null = null;
  let sequence = 0;
  const setConnection = (next: MidiFaderConnection | null) => {
    connection = next;
    sequence = 0;
  };
  const accept = (payload: unknown): boolean => {
    if (!connection || !payload || typeof payload !== "object") return false;
    const v = payload as Record<string, unknown>;
    if (v.schema_version !== 1 || v.project_epoch !== connection.project_epoch
      || v.project_epoch !== options.projectEpoch() || v.input_session !== connection.input_session
      || !positive(v.sequence) || v.sequence <= sequence
      || !Array.isArray(v.values) || v.values.length === 0 || v.values.length > 4096) return false;
    const incoming: FaderValueMap = {};
    for (const item of v.values) {
      if (!item || typeof item !== "object" || !positive(item.fixture_id)
        || typeof item.attribute !== "string" || !item.attribute || item.attribute.length > 128
        || typeof item.value !== "number" || !Number.isInteger(item.value) || item.value < 0 || item.value > 65535) return false;
      incoming[`${item.fixture_id}:${item.attribute}`] = item.value;
    }
    sequence = v.sequence;
    options.apply(incoming);
    return true;
  };
  const acceptRouted = (payload: unknown): boolean => {
    if (!connection || !options.applyRouted || !payload || typeof payload !== "object") return false;
    const v = payload as Record<string, unknown>;
    if (v.schema_version !== 1 || v.project_epoch !== connection.project_epoch
      || v.project_epoch !== options.projectEpoch() || v.input_session !== connection.input_session
      || !positive(v.sequence) || v.sequence <= sequence) return false;
    const input = performRoutedInputFromPayload(v.input);
    if (!input) return false;
    sequence = v.sequence;
    options.applyRouted(input);
    return true;
  };
  return { setConnection, accept, acceptRouted };
}
