import type { OutputOwnershipStatus } from "./types";

/**
 * Whether this machine can currently change output state (the Lighting/Video
 * blackout targets). Blackout needs exactly one active Lighting+Video output
 * lease; a Standby machine holds none. Surfacing that before the operator acts
 * keeps the control honest instead of failing after a confirmation.
 *
 * "unknown" never blocks: an unreadable or not-yet-polled state must not take
 * the emergency path away from an operator whose machine does drive outputs.
 */
export type OutputDriveBlockReason = "standby" | "fenced" | "no_lease" | "not_taken";

export type OutputDriveStatus =
  | { kind: "unknown" }
  | { kind: "driving" }
  | { kind: "blocked"; reason: OutputDriveBlockReason; message: string };

export const OUTPUT_DRIVE_STANDBY_MESSAGE = "Standby: this machine is not driving outputs";
export const OUTPUT_DRIVE_FENCED_MESSAGE = "Fenced: outputs are held while the machine output role changes";
export const OUTPUT_DRIVE_NO_LEASE_MESSAGE = "Output off: no active output lease on this machine";
/** App start or a project replacement left outputs denied and nothing has taken them yet. */
export const OUTPUT_DRIVE_NOT_TAKEN_MESSAGE = "Output off: this machine has not taken the outputs yet";

/** Footer status key owned by blackout refusals; cleared when the state changes. */
export const BLACKOUT_STATUS_KEY = "blackout-unavailable";

const messageFor = (reason: OutputDriveBlockReason): string => {
  switch (reason) {
    case "standby": return OUTPUT_DRIVE_STANDBY_MESSAGE;
    case "fenced": return OUTPUT_DRIVE_FENCED_MESSAGE;
    case "no_lease": return OUTPUT_DRIVE_NO_LEASE_MESSAGE;
    case "not_taken": return OUTPUT_DRIVE_NOT_TAKEN_MESSAGE;
  }
};

/**
 * Classify from the authoritative ownership status plus whether exactly one
 * active Lighting+Video lease exists (the precondition the blackout command
 * itself enforces). A held lease always means this machine drives outputs.
 */
export function classifyOutputDrive(
  ownership: Pick<OutputOwnershipStatus, "state" | "effective_role" | "desired_role">
    & Partial<Pick<OutputOwnershipStatus, "lighting_reason">>,
  bothLeaseActive: boolean,
): OutputDriveStatus {
  if (bothLeaseActive) return { kind: "driving" };
  // Startup and project replacement deny outputs until they are taken again;
  // that is neither a role change in progress nor a chosen Standby.
  const notTaken = ownership.lighting_reason === "StartupDenied"
    || ownership.lighting_reason === "ProjectSwapDisarmed";
  const reason: OutputDriveBlockReason = notTaken
    ? "not_taken"
    : ownership.state !== "Ready"
    ? "fenced"
    : ownership.effective_role === "Standby" || ownership.desired_role === "Standby"
      ? "standby"
      : "no_lease";
  return { kind: "blocked", reason, message: messageFor(reason) };
}

/** Compact label for tight layouts: the text before the colon ("Standby"). */
export const outputDriveShortLabel = (message: string): string => message.split(":")[0].trim();

export const outputDriveBlockedMessage = (status: OutputDriveStatus | null | undefined): string | null =>
  status?.kind === "blocked" ? status.message : null;

/** Identical polls must not re-render dependents or reset a footer message. */
export const sameOutputDriveStatus = (a: OutputDriveStatus, b: OutputDriveStatus): boolean =>
  a.kind === b.kind
  && (a.kind !== "blocked" || b.kind !== "blocked" || a.reason === b.reason);

/** Full operator-facing refusal for a blackout command that was not sent. */
export const blackoutNotAppliedMessage = (status: Extract<OutputDriveStatus, { kind: "blocked" }>): string =>
  `No blackout was applied. ${status.message}.`;
