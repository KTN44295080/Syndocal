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
