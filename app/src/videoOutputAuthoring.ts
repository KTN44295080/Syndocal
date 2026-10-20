import type { VideoOutputKind, VideoOutputSummary } from "./types";
import type { ExternalVideoOutputSpec, VideoOutputConfigurationSpec } from "./outputControlController";

type BackendAvailability = readonly { id: string; state: string }[] | null | undefined;
export const isVideoOutputKindAvailable = (kind: VideoOutputKind, backends: BackendAvailability): boolean => {
  if (kind === "Display") return true;
  const id = kind === "SpoutSender" ? "spout" : kind === "NdiSender" ? "ndi" : "syphon";
  return backends?.some((backend) => backend.id === id && backend.state === "Available") ?? false;
};
const asciiFold = (value: string): string => value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());
const bytes = (value: string): number => new TextEncoder().encode(value).length;
const messages = new Set([
  "This output type is unavailable on this computer or in this build.",
  "Enter an output label of up to 128 UTF-8 bytes without slashes or control characters.",
  "Use whole-number dimensions from 1 to 8192 for external outputs, or 1 to 16384 for Display.",
  "Enter an endpoint or server name of up to 255 UTF-8 bytes without control characters.",
  "Another output of this type already uses that endpoint or server name. Choose a different name.",
  "Choose an existing composition before adding the output.",
  "The selected display is unavailable. Refresh the display list and choose a connected screen.",
  "Another Display output already uses that screen. Choose a different screen.",
  "The selected output no longer exists. Refresh the output list.",
]);
export function validateVideoOutputAuthoringSpec(
  spec: ExternalVideoOutputSpec | VideoOutputConfigurationSpec,
  outputs: readonly VideoOutputSummary[],
  backends: BackendAvailability,
  excludeId?: number,
): void {
  if (!isVideoOutputKindAvailable(spec.kind, backends)) throw new Error("This output type is unavailable on this computer or in this build.");
  if (!spec.label || spec.label !== spec.label.trim() || bytes(spec.label) > 128 || /[\u0000-\u001f\u007f-\u009f/\\]/.test(spec.label)) {
    throw new Error("Enter an output label of up to 128 UTF-8 bytes without slashes or control characters.");
  }
  const maximum = spec.kind === "Display" ? 16384 : 8192;
  if (![spec.width, spec.height].every((value) => Number.isSafeInteger(value) && value >= 1 && value <= maximum)) {
    throw new Error("Use whole-number dimensions from 1 to 8192 for external outputs, or 1 to 16384 for Display.");
  }
  if (spec.kind !== "Display") {
    const endpoint = spec.endpoint_name;
    if (!endpoint || endpoint !== endpoint.trim() || bytes(endpoint) > 255 || /[\u0000-\u001f\u007f-\u009f]/.test(endpoint)) {
      throw new Error("Enter an endpoint or server name of up to 255 UTF-8 bytes without control characters.");
    }
    if (outputs.some((output) => output.id !== excludeId && output.kind === spec.kind
      && asciiFold(output.endpoint_name?.trim() ?? "") === asciiFold(endpoint))) {
      throw new Error("Another output of this type already uses that endpoint or server name. Choose a different name.");
    }
  } else if ("monitor_index" in spec && outputs.some((output) => output.id !== excludeId && output.kind === "Display"
    && (output.monitor_id === spec.monitor_index || spec.monitor_identity !== null
      && (output as VideoOutputSummary & { monitor_identity?: string | null }).monitor_identity === spec.monitor_identity))) {
    throw new Error("Another Display output already uses that screen. Choose a different screen.");
  }
}
export function videoOutputAuthoringErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  // A retained Pending publication cannot be repaired by refreshing leases
  // in this process. Classify it before any nested lease/authority detail.
  if (/\bpublication_failed\b/i.test(message)
    || message.startsWith("Video output authoring is in doubt; pending receipt retained:")) {
    return "The output change could not be verified. Restart Syndocal and use Recovery before editing outputs again.";
  }
  if (messages.has(message)) return message;
  if (/stale_fence|project authority changed/.test(message)) return "The project changed before this output edit. Refresh and try again.";
  if (/invalid_request|settings are invalid/.test(message)) return "The output edit was rejected. Check the name, resolution, target, backend and composition.";
  if (/forbidden|lease|authority|ownership/i.test(message)) return "Video output ownership is unavailable. Refresh output ownership and try again.";
  if (/busy|overloaded/.test(message)) return "Another output operation is in progress. Wait for it to finish and try again.";
  return "The output state could not be verified. Refresh the output list before trying again.";
}
