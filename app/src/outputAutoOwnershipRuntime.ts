import { createSignal } from "solid-js";
import { queryOutputLeaseAuthority } from "./outputControlController";
import {
  readAutoOutputSetting,
  readRememberedStandbySyncRole,
  writeAutoOutputSetting,
  type AutoOutputInputs,
  type AutoOutputStandbySyncView,
  type AutoOutputStatus,
} from "./outputAutoOwnership";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { OutputOwnershipStatus } from "./types";

// UI adapter for automatic output ownership: the machine-local Setup switch,
// the last automatic outcome, and the read-only queries the policy needs.

const storage = () => window.localStorage;

const [autoOutputSetting, setAutoOutputSettingSignal] = createSignal(readAutoOutputSetting(storage));
const [autoOutputStatus, setAutoOutputStatus] = createSignal<AutoOutputStatus>({ kind: "unknown" });

export { autoOutputSetting, autoOutputStatus, setAutoOutputStatus };

/** Turning it off only stops future automatic takes; current outputs stay as they are. */
export function setAutoOutputSetting(on: boolean): void {
  writeAutoOutputSetting(storage, on);
  setAutoOutputSettingSignal(on);
}

export async function readAutoOutputOwnershipView(
  invoke: FrontendTauriInvoke,
): Promise<Pick<AutoOutputInputs, "ownership" | "leases">> {
  const [ownership, leases] = await Promise.all([
    invoke<OutputOwnershipStatus>("get_output_ownership_status"),
    queryOutputLeaseAuthority(invoke),
  ]);
  return { ownership, leases };
}

const standbySyncView = (value: unknown): AutoOutputStandbySyncView | null => {
  if (!value || typeof value !== "object") return null;
  const status = value as { running?: unknown; role?: unknown; split_brain?: unknown };
  if (typeof status.running !== "boolean" || typeof status.split_brain !== "boolean") return null;
  const role = status.role === "primary" || status.role === "standby" ? status.role : null;
  if (status.role !== null && role === null) return null;
  return { running: status.running, role, split_brain: status.split_brain };
};

/** Unreadable guards are reported as null and refuse the automatic take. */
export async function readAutoOutputGuards(
  invoke: FrontendTauriInvoke,
): Promise<Pick<AutoOutputInputs, "standbySync" | "killSwitchActive" | "rememberedStandbySyncRole">> {
  const [standbySync, killSwitchActive] = await Promise.all([
    invoke<unknown>("standby_sync_status").then(standbySyncView, () => null),
    invoke<{ killSwitchActive?: unknown }>("agent_authority_status_v1").then(
      (status) => typeof status?.killSwitchActive === "boolean" ? status.killSwitchActive : null,
      () => null,
    ),
  ]);
  return { standbySync, killSwitchActive, rememberedStandbySyncRole: readRememberedStandbySyncRole(storage) };
}
