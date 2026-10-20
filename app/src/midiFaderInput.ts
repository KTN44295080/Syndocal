import type { FaderValueMap } from "./createFaderValueController";
import { performRoutedInputFromPayload, type PerformRoutedInput } from "./controlMappingPerform";

export interface MidiFaderConnection {
  schema_version: 1;
  project_epoch: number;
  input_session: number;
}
export const routedInputStartupBufferLimit = 256;
const positive = (n: unknown): n is number => typeof n === "number" && Number.isSafeInteger(n) && n > 0;
export function midiFaderConnection(value: unknown, source: "MIDI" | "OSC" | "DMX" = "MIDI"): MidiFaderConnection {
  const v = value as Partial<MidiFaderConnection> | null;
  if (!v || v.schema_version !== 1 || (typeof v.project_epoch !== "number" || !Number.isSafeInteger(v.project_epoch) || v.project_epoch < 0) || !positive(v.input_session)) {
    throw new Error(`${source} input connection identity was invalid.`);
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
  applyRouted?: (input: PerformRoutedInput, sourceIsCurrent: () => boolean) => void;
  /** OSC/DMX may emit a first frame before the start reply arrives. */
  bufferRoutedUntilConnected?: boolean;
}) {
  let connection: MidiFaderConnection | null = null;
  let sequence = 0;
  let newestInputSession = 0;
  let pendingRouted: { projectEpoch: number; inputSession: number; inputs: Map<number, PerformRoutedInput> } | null = null;
  const setConnection = (next: MidiFaderConnection | null) => {
    if (next === null) {
      connection = null;
      sequence = 0;
      pendingRouted = null;
      return;
    }
    const epoch = options.projectEpoch();
    if (next.project_epoch !== epoch || next.input_session <= newestInputSession) return;
    newestInputSession = next.input_session;
    connection = next;
    sequence = 0;
    const waiting = pendingRouted;
    // An unconfirmed session cannot take over the acknowledged connection.
    if (waiting && (waiting.projectEpoch !== epoch || waiting.inputSession <= next.input_session)) pendingRouted = null;
    if (waiting?.projectEpoch === next.project_epoch && waiting.inputSession === next.input_session) {
      for (const [queuedSequence, input] of [...waiting.inputs].sort(([a], [b]) => a - b)) {
        acceptRouted({ ...next, sequence: queuedSequence, input });
      }
    }
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
    if (!options.applyRouted || (!connection && !options.bufferRoutedUntilConnected)
      || !payload || typeof payload !== "object") return false;
    const v = payload as Record<string, unknown>;
    const epoch = options.projectEpoch();
    if (v.schema_version !== 1 || v.project_epoch !== epoch
      || !positive(v.input_session) || !positive(v.sequence)) return false;
    const inputSession = v.input_session;
    const inputSequence = v.sequence;
    const matchesConnection = connection !== null && v.project_epoch === connection.project_epoch
      && inputSession === connection.input_session;
    if (matchesConnection ? inputSequence <= sequence
      : !options.bufferRoutedUntilConnected || inputSession <= newestInputSession) return false;
    const input = performRoutedInputFromPayload(v.input);
    if (!input) return false;
    if (matchesConnection) {
      sequence = inputSequence;
      const acceptedConnection = connection;
      options.applyRouted(input, () => connection === acceptedConnection
        && connection !== null && connection.project_epoch === options.projectEpoch());
      return true;
    }
    // Retain bounded validated data for the newest unconfirmed session.
    if (!pendingRouted || pendingRouted.projectEpoch !== epoch || inputSession > pendingRouted.inputSession) {
      pendingRouted = { projectEpoch: epoch, inputSession, inputs: new Map() };
    }
    if (inputSession !== pendingRouted.inputSession || pendingRouted.inputs.has(inputSequence)) return false;
    pendingRouted.inputs.set(inputSequence, input);
    if (pendingRouted.inputs.size > routedInputStartupBufferLimit) {
      pendingRouted.inputs.delete(Math.min(...pendingRouted.inputs.keys()));
    }
    // Queued input is accepted only when the exact native start reply drains it.
    return false;
  };
  return { setConnection, accept, acceptRouted };
}
