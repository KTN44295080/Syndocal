import type { VideoClipRuntimeSnapshot, VideoLayerTransitionRuntimeSnapshot } from "./types";

/** Rust omits these empty vectors. Only omission means empty, never invalid data. */
function normalizeRuntimeCollection<T>(wire: unknown, field: "layers" | "buses"): T {
  if (wire === null || typeof wire !== "object" || Array.isArray(wire)) {
    throw new Error(`Invalid video runtime publication: expected an object containing ${field}.`);
  }
  const prototype = Object.getPrototypeOf(wire);
  if (prototype !== Object.prototype && prototype !== null) {
    throw new Error("Invalid video runtime publication prototype.");
  }
  if (!Object.hasOwn(wire, field)) return { ...wire, [field]: [] } as T;
  if (!Array.isArray((wire as Record<string, unknown>)[field])) {
    throw new Error(`Invalid video runtime publication: ${field} must be an array when present.`);
  }
  // Do not clone populated runtime arrays on every poll. Entry semantics remain
  // owned by the native runtime contract; this adapter only handles its collection shape.
  return wire as T;
}

export const normalizeVideoClipRuntimeSnapshot = (wire: unknown): VideoClipRuntimeSnapshot =>
  normalizeRuntimeCollection<VideoClipRuntimeSnapshot>(wire, "layers");

export const normalizeVideoLayerTransitionRuntimeSnapshot = (wire: unknown): VideoLayerTransitionRuntimeSnapshot =>
  normalizeRuntimeCollection<VideoLayerTransitionRuntimeSnapshot>(wire, "buses");
