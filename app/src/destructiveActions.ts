export type DestructiveActionKind = "fixture" | "cue" | "video output" | "effect";

type ConfirmAction = (message: string) => boolean;

export function destructiveActionPrompt(kind: DestructiveActionKind, label: string): string {
  return `Remove ${kind} "${label}"?\n\nThis action cannot be undone.`;
}

export function confirmDestructiveAction(
  kind: DestructiveActionKind,
  label: string,
  confirmAction: ConfirmAction = (message) => globalThis.confirm(message),
): boolean {
  return confirmAction(destructiveActionPrompt(kind, label));
}
