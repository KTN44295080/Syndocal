import {
  audioMappingFreshnessMs, audioMappingIntervalMs, createAudioMappingEvaluator, isAudioMapping,
  type AudioMappingObservation, type AudioRuntimeBatch,
} from "./audioControlMapping";
import type { MidiControlMapping } from "./types";
import type { PerformRoutedInput } from "./controlMappingPerform";

type PublishedAudio = Omit<AudioMappingObservation, "observedAt">;

/** Only a new publication refreshes the lease. A repeated poll can never keep
 * a dead source alive, or revive a source cleared by Stop or a failed read. */
export function createAudioMappingSource(now: () => number = () => performance.now()) {
  let observation: AudioMappingObservation | null = null;
  let generation = 0;
  let sequence = 0;
  return {
    current: () => observation,
    clear() { observation = null; },
    accept(frame: PublishedAudio) {
      if (!frame.running || frame.stale || frame.safety_clear_pending
        || !Number.isSafeInteger(frame.generation) || frame.generation <= 0
        || !Number.isSafeInteger(frame.feature_sequence) || frame.feature_sequence <= 0) {
        observation = null; return;
      }
      if (frame.generation < generation
        || (frame.generation === generation && frame.feature_sequence <= sequence)) return;
      generation = frame.generation; sequence = frame.feature_sequence;
      observation = { ...frame, observedAt: now() };
    },
  };
}

export function createAudioControlRuntime(options: {
  mappings: () => readonly MidiControlMapping[];
  observation: () => AudioMappingObservation | null;
  projectEpoch: () => number;
  suspended: () => boolean;
  dispatch: (input: PerformRoutedInput) => Promise<unknown>;
  dispatchRuntime: (batch: AudioRuntimeBatch, sourceIsCurrent: () => boolean) => Promise<unknown>;
  now?: () => number;
}) {
  const now = options.now ?? (() => performance.now());
  const evaluator = createAudioMappingEvaluator();
  let epoch = options.projectEpoch();
  let lastTick = -Infinity;
  let hadValues = false;
  let triggersInFlight = false;
  let disposed = false;
  let leaseVersion = 0;
  let mappingSignature: string | null = null;

  const advance = () => {
    const at = now();
    if (disposed || !Number.isFinite(at) || at - lastTick < audioMappingIntervalMs) return;
    lastTick = at;
    const currentEpoch = options.projectEpoch();
    if (epoch !== currentEpoch) {
      epoch = currentEpoch; leaseVersion += 1; evaluator.reset();
    }
    const observation = options.observation();
    const mappings = options.mappings();
    const nextMappingSignature = JSON.stringify(mappings.filter(isAudioMapping));
    if (mappingSignature !== nextMappingSignature) {
      mappingSignature = nextMappingSignature; leaseVersion += 1;
    }
    const { batch, triggers } = evaluator.evaluate(mappings, observation, at, options.suspended());
    // A resumed source never revives a pulse queued before a modal, Stop or stale read.
    if (batch.audioGeneration === 0) leaseVersion += 1;
    const capturedEpoch = epoch;
    const version = leaseVersion;
    const signatures = new Map(mappings.filter(isAudioMapping).map(mapping => [mapping.audio.id, JSON.stringify(mapping)]));
    const sourceIsCurrent = () => {
      const current = options.observation();
      return !disposed && version === leaseVersion && options.projectEpoch() === capturedEpoch
        && !options.suspended() && current !== null && current.running && !current.stale
        && !current.safety_clear_pending && current.generation === batch.audioGeneration
        && current.feature_sequence >= batch.featureSequence && now() >= current.observedAt
        && now() - current.observedAt < audioMappingFreshnessMs && now() - at < audioMappingFreshnessMs;
    };
    const mappingIsCurrent = (id: string) => {
      const current = options.mappings().find(mapping => isAudioMapping(mapping) && mapping.audio.id === id);
      return current !== undefined && JSON.stringify(current) === signatures.get(id);
    };
    if (batch.values.length > 0 || hadValues) {
      const hasValues = batch.values.length > 0;
      void options.dispatchRuntime(batch, hasValues
        ? () => sourceIsCurrent() && batch.values.every(value => mappingIsCurrent(value.mappingId))
        : () => options.projectEpoch() === capturedEpoch).catch(() => undefined);
      hadValues = hasValues;
    }
    // A slow ticketed Queue/Take owns this pulse. No trigger backlog accumulates.
    if (!triggersInFlight && triggers.length > 0) {
      triggersInFlight = true;
      void Promise.all(triggers.map(input => options.dispatch({
        ...input, sourceIsCurrent: () => sourceIsCurrent() && mappingIsCurrent(input.audioMappingId ?? ""),
      }))).catch(() => undefined).finally(() => { triggersInFlight = false; });
    }
  };
  return {
    advance,
    dispose() {
      disposed = true; leaseVersion += 1; evaluator.reset();
      if (hadValues) {
        const capturedEpoch = epoch;
        void options.dispatchRuntime({ audioGeneration: 0, featureSequence: 0, values: [] },
          () => options.projectEpoch() === capturedEpoch).catch(() => undefined);
      }
      hadValues = false;
    },
  };
}
