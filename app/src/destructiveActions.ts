export type DestructiveActionKind = "fixture" | "cue" | "cue list" | "palette" | "playback executor" | "video output" | "effect";

type ConfirmAction = (message: string) => boolean;

export function destructiveActionPrompt(kind: DestructiveActionKind, label: string): string {
  return `Remove ${kind} "${label}"?\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.`;
}

export function confirmDestructiveAction(
  kind: DestructiveActionKind,
  label: string,
  confirmAction: ConfirmAction = (message) => globalThis.confirm(message),
): boolean {
  return confirmAction(destructiveActionPrompt(kind, label));
}

const countLabel = (count: number, singular: string, plural = `${singular}s`) =>
  `${count} ${count === 1 ? singular : plural}`;

export function cueRemovalPrompt(
  label: string,
  timelinePlacementCount: number,
  incomingJumpCount: number,
): string {
  const placementImpact = timelinePlacementCount > 0
    ? `${countLabel(timelinePlacementCount, "timeline placement")} will also be removed.`
    : "No timeline placements are linked to this Cue.";
  const jumpImpact = incomingJumpCount > 0
    ? `${countLabel(incomingJumpCount, "incoming jump")} will be cleared.`
    : "No incoming jumps will be changed.";
  return `Remove cue "${label}"?\n\n${placementImpact}\n${jumpImpact}\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.`;
}

export function confirmCueRemoval(
  label: string,
  timelinePlacementCount: number,
  incomingJumpCount: number,
  confirmAction: ConfirmAction = (message) => globalThis.confirm(message),
): boolean {
  return confirmAction(cueRemovalPrompt(label, timelinePlacementCount, incomingJumpCount));
}

export function timelinePlacementRemovalPrompt(
  eventId: number,
  sourceLabel: string,
  spanMs: number,
): string {
  const kind = spanMs > 0 ? "Scene Block" : "timeline point";
  const span = spanMs > 0 ? ` · ${spanMs} ms span` : " · instant trigger";
  return `Remove ${kind} #${eventId}?\n\nSource: ${sourceLabel}${span}\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.`;
}

export function confirmTimelinePlacementRemoval(
  eventId: number,
  sourceLabel: string,
  spanMs: number,
  confirmAction: ConfirmAction = (message) => globalThis.confirm(message),
): boolean {
  return confirmAction(timelinePlacementRemovalPrompt(eventId, sourceLabel, spanMs));
}
