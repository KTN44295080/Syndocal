// T17 scene live-modifier helpers shared by the Scene Matrix and Touch pads.
// Clamp ranges mirror the engine's sanitizers exactly (crates/engine
// sanitize_live_modifier_*): speed 0.05..20 (non-finite/<=0 -> 1), size 0..2
// (non-finite/<0 -> 1), phase wrapped into 0..1.
import type { CueLiveModifierSettings, CueLiveModifierState, CueSummary } from "./types";

export const CUE_LIVE_MODIFIER_SPEED_MIN = 0.05;
export const CUE_LIVE_MODIFIER_SPEED_MAX = 20;
export const CUE_LIVE_MODIFIER_SIZE_MAX = 2;

export const neutralCueLiveModifier = (): CueLiveModifierSettings => ({
  speed: 1,
  size: 1,
  phase: 0,
  flash: false,
});

export const sanitizeLiveModifierSpeed = (speed: number): number => {
  if (!Number.isFinite(speed) || speed <= 0) return 1;
  return Math.min(Math.max(speed, CUE_LIVE_MODIFIER_SPEED_MIN), CUE_LIVE_MODIFIER_SPEED_MAX);
};

export const sanitizeLiveModifierSize = (size: number): number => {
  if (!Number.isFinite(size) || size < 0) return 1;
  return Math.min(size, CUE_LIVE_MODIFIER_SIZE_MAX);
};

export const sanitizeLiveModifierPhase = (phase: number): number => {
  if (!Number.isFinite(phase)) return 0;
  const wrapped = phase % 1;
  return wrapped < 0 ? wrapped + 1 : wrapped;
};

/** Authored dial position of a cue; absent settings mean neutral. */
export const authoredCueLiveModifier = (cue: CueSummary): CueLiveModifierSettings => ({
  speed: sanitizeLiveModifierSpeed(cue.live_modifiers?.speed ?? 1),
  size: sanitizeLiveModifierSize(cue.live_modifiers?.size ?? 1),
  phase: sanitizeLiveModifierPhase(cue.live_modifiers?.phase ?? 0),
  flash: cue.live_modifiers?.flash ?? false,
});

/** Effective dial position: the latched live override wins over authored. */
export const effectiveCueLiveModifier = (
  cue: CueSummary,
  liveStates: CueLiveModifierState[] | undefined,
): CueLiveModifierSettings => {
  const authored = authoredCueLiveModifier(cue);
  const live = liveStates?.find((state) => state.cue_id === cue.id);
  if (!live) return authored;
  return {
    speed: sanitizeLiveModifierSpeed(live.speed),
    size: sanitizeLiveModifierSize(live.size),
    phase: sanitizeLiveModifierPhase(live.phase),
    flash: authored.flash,
  };
};

/** True while a latched live override diverges from the authored dials. */
export const cueLiveModifierIsOverridden = (
  cue: CueSummary,
  liveStates: CueLiveModifierState[] | undefined,
): boolean => {
  const live = liveStates?.find((state) => state.cue_id === cue.id);
  if (!live) return false;
  const authored = authoredCueLiveModifier(cue);
  return (
    sanitizeLiveModifierSpeed(live.speed) !== authored.speed ||
    sanitizeLiveModifierSize(live.size) !== authored.size ||
    sanitizeLiveModifierPhase(live.phase) !== authored.phase
  );
};

export const formatLiveModifierSpeed = (speed: number): string =>
  `x${sanitizeLiveModifierSpeed(speed).toFixed(2).replace(/\.?0+$/, "")}`;

export const formatLiveModifierSize = (size: number): string =>
  `${Math.round(sanitizeLiveModifierSize(size) * 100)}%`;

export const formatLiveModifierPhase = (phase: number): string =>
  `${Math.round(sanitizeLiveModifierPhase(phase) * 100)}%`;
