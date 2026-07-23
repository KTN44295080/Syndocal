import type { OperatorLockMode, OperatorPolicy } from "./types";

export const operatorCredentialScheme = "PBKDF2-SHA256" as const;
export const operatorCredentialIterations = 600_000;
const operatorSaltBytes = 16;
const operatorVerifierBytes = 32;
const operatorPasswordMinCharacters = 8;
const operatorPasswordMaxBytes = 1_024;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const bytesToBase64 = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes));

const base64ToBytes = (value: string) => {
  const decoded = atob(value);
  return Uint8Array.from(decoded, (character) => character.charCodeAt(0));
};

const passwordBytes = (password: string) => {
  const bytes = new TextEncoder().encode(password);
  if (password.length < operatorPasswordMinCharacters || bytes.length > operatorPasswordMaxBytes) {
    throw new Error(
      `Operator password must contain at least ${operatorPasswordMinCharacters} characters and at most ${operatorPasswordMaxBytes} UTF-8 bytes.`,
    );
  }
  return bytes;
};

const deriveVerifier = async (password: string, salt: Uint8Array, iterations: number) => {
  const key = await crypto.subtle.importKey("raw", passwordBytes(password), "PBKDF2", false, ["deriveBits"]);
  const bits = await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      hash: "SHA-256",
      salt: new Uint8Array(salt).buffer,
      iterations,
    },
    key,
    operatorVerifierBytes * 8,
  );
  return new Uint8Array(bits);
};

export const createOperatorPolicy = async (
  password: string,
  lockMode: OperatorLockMode,
  lockOnLoad: boolean,
): Promise<OperatorPolicy> => {
  const salt = crypto.getRandomValues(new Uint8Array(operatorSaltBytes));
  const verifier = await deriveVerifier(password, salt, operatorCredentialIterations);
  return {
    lock_mode: lockMode,
    lock_on_load: lockOnLoad,
    credential: {
      scheme: operatorCredentialScheme,
      iterations: operatorCredentialIterations,
      salt_b64: bytesToBase64(salt),
      verifier_b64: bytesToBase64(verifier),
    },
  };
};

export const operatorPolicyFromUnknown = (value: unknown): OperatorPolicy | null => {
  if (!isRecord(value) || (value.lock_mode !== "Full" && value.lock_mode !== "Partial")) return null;
  if (typeof value.lock_on_load !== "boolean" || !isRecord(value.credential)) return null;
  const credential = value.credential;
  if (
    credential.scheme !== operatorCredentialScheme ||
    !Number.isInteger(credential.iterations) ||
    (credential.iterations as number) < 100_000 ||
    (credential.iterations as number) > 2_000_000 ||
    typeof credential.salt_b64 !== "string" ||
    typeof credential.verifier_b64 !== "string"
  ) {
    return null;
  }
  try {
    if (base64ToBytes(credential.salt_b64).length < 16 || base64ToBytes(credential.salt_b64).length > 64) {
      return null;
    }
    if (base64ToBytes(credential.verifier_b64).length !== operatorVerifierBytes) return null;
  } catch {
    return null;
  }
  return value as unknown as OperatorPolicy;
};

export const verifyOperatorPassword = async (policy: OperatorPolicy, password: string) => {
  const expected = base64ToBytes(policy.credential.verifier_b64);
  const actual = await deriveVerifier(
    password,
    base64ToBytes(policy.credential.salt_b64),
    policy.credential.iterations,
  );
  if (actual.length !== expected.length) return false;
  let difference = 0;
  for (let index = 0; index < actual.length; index += 1) {
    difference |= actual[index] ^ expected[index];
  }
  return difference === 0;
};

const fullLockEmergencyCommands = new Set([
  "get_operator_policy",
  "get_snapshot",
  "get_snapshot_delta",
  "set_blackout",
  "set_all_blackout",
  "set_video_blackout",
  "set_video_output_blackout",
]);

const partialLockBoundaryCommands = new Set([
  "new_project",
  "load_user_template",
  "load_project",
  "load_project_path",
  "load_project_checkpoint",
  "load_project_backup",
  "load_startup_project",
  "load_phase1_sample_project",
  "import_daslight_project",
  "undo_project_transaction",
  "redo_project_transaction",
  "clear_project_history",
  "set_operator_policy",
  "clear_operator_policy",
  "set_programmer_mode",
  "set_programmer_attribute",
  "set_programmer_group_attribute",
  "clear_programmer",
]);

export const operatorCommandAllowed = (
  mode: OperatorLockMode | null,
  command: string,
  isProjectMutation: boolean,
) => {
  if (mode === null) return true;
  if (mode === "Full") return fullLockEmergencyCommands.has(command);
  return !isProjectMutation && !partialLockBoundaryCommands.has(command);
};
