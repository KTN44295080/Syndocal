export type AppStatusTone = "info" | "success" | "warning" | "error";

export interface AppStatus {
  text: string;
  tone: AppStatusTone;
  key?: string;
}

const errorPattern = /\b(error|failed|failure|cannot|could not|invalid|unavailable|not found|exceeds|conflict|timed out|must be)\b/i;
const warningPattern = /\b(canceled|cancelled|blocked|unsupported|skipped|no|nothing|select|add at least|requires?|needs?)\b/i;
const successPattern = /\b(saved|loaded|created|added|updated|applied|patched|removed|exported|imported|sent|connected|disconnected|started|stopped|opened|closed|copied|rendered|synced|analyzed|cleared|reset|duplicated|moved|aligned|distributed|rotated|mirrored|enabled|disabled|set)\b/i;

export const appStatusTone = (text: string): AppStatusTone => {
  const normalized = text.trim();
  if (errorPattern.test(normalized)) return "error";
  if (warningPattern.test(normalized)) return "warning";
  if (successPattern.test(normalized)) return "success";
  return "info";
};

export const appStatusFromMessage = (text: string, key?: string): AppStatus => ({
  text: text.trim() || "Ready",
  tone: appStatusTone(text),
  ...(key ? { key } : {}),
});
