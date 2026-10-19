import type { ConfirmDialogOptions } from "./inAppDialog";

export type DestructiveActionKind = "fixture" | "cue" | "Bank" | "palette" | "playback executor" | "video output" | "effect";

/** The in-app `confirmDialog` (or a test double). A synchronous answer is accepted too. */
export type ConfirmDialogFunction = (options: ConfirmDialogOptions) => boolean | Promise<boolean>;

const undoMessage = "You can undo this from the Project menu or with Ctrl/Cmd+Z.";

export function destructiveActionDialog(kind: DestructiveActionKind, label: string): ConfirmDialogOptions {
  return {
    title: `Remove ${kind} "${label}"?`,
    message: undoMessage,
    confirmLabel: "Remove",
    tone: "danger",
  };
}

export async function confirmDestructiveAction(
  kind: DestructiveActionKind,
  label: string,
  ask: ConfirmDialogFunction,
): Promise<boolean> {
  return await ask(destructiveActionDialog(kind, label)) === true;
}

const countLabel = (count: number, singular: string, plural = `${singular}s`) =>
  `${count} ${count === 1 ? singular : plural}`;

export function cueRemovalDialog(
  label: string,
  timelinePlacementCount: number,
  incomingJumpCount: number,
): ConfirmDialogOptions {
  const placementImpact = timelinePlacementCount > 0
    ? `${countLabel(timelinePlacementCount, "timeline placement")} will also be removed.`
    : "No timeline placements are linked to this Cue.";
  const jumpImpact = incomingJumpCount > 0
    ? `${countLabel(incomingJumpCount, "incoming jump")} will be cleared.`
    : "No incoming jumps will be changed.";
  return {
    title: `Remove cue "${label}"?`,
    message: [placementImpact, jumpImpact, undoMessage],
    confirmLabel: "Remove",
    tone: "danger",
  };
}

export async function confirmCueRemoval(
  label: string,
  timelinePlacementCount: number,
  incomingJumpCount: number,
  ask: ConfirmDialogFunction,
): Promise<boolean> {
  return await ask(cueRemovalDialog(label, timelinePlacementCount, incomingJumpCount)) === true;
}

export function timelinePlacementRemovalDialog(
  eventId: number,
  sourceLabel: string,
  spanMs: number,
): ConfirmDialogOptions {
  const kind = spanMs > 0 ? "Scene Block" : "timeline point";
  const span = spanMs > 0 ? `${spanMs} ms span` : "instant trigger";
  return {
    title: `Remove ${kind} #${eventId}?`,
    message: [`Source: ${sourceLabel} · ${span}`, undoMessage],
    confirmLabel: "Remove",
    tone: "danger",
  };
}

export async function confirmTimelinePlacementRemoval(
  eventId: number,
  sourceLabel: string,
  spanMs: number,
  ask: ConfirmDialogFunction,
): Promise<boolean> {
  return await ask(timelinePlacementRemovalDialog(eventId, sourceLabel, spanMs)) === true;
}
