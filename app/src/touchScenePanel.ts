import type { CueListSummary, CueSummary, EngineSnapshot, TouchPageSummary } from "./types";
import type { FullBankAuthoritySnapshot } from "./bankAuthority";
import { touchSurfaceGridColumns } from "./touchSurface";

/** Control > Lighting reserves the last three grid columns for the Scene panel. */
export const touchScenePanelColumns = 3;
export const touchScenePanelStorageKey = "syndocal.touchScenePanel.v1";

export interface TouchSceneBank {
  bank: CueListSummary;
  cues: CueSummary[];
}

/**
 * Every Scene of the one Bank authority, grouped by Bank in authored order.
 * Empty Banks are omitted. An unavailable authority yields nothing: callers
 * show the authority issue instead of a valid-looking subset.
 */
export const touchSceneBanks = (authority: FullBankAuthoritySnapshot): TouchSceneBank[] => {
  if (authority.issue !== null) return [];
  const cuesByBank = new Map<number, CueSummary[]>();
  for (const cue of authority.cues) {
    const list = cuesByBank.get(cue.cue_list_id);
    if (list) list.push(cue);
    else cuesByBank.set(cue.cue_list_id, [cue]);
  }
  return authority.cueLists
    .map((bank) => ({ bank, cues: cuesByBank.get(bank.id) ?? [] }))
    .filter((entry) => entry.cues.length > 0);
};

/** Same activity rule as the Edit > Lighting Scene matrix. */
export const touchSceneActive = (
  cue: CueSummary,
  snapshot: Pick<EngineSnapshot, "active_cue_id" | "active_group_cue_ids">,
) => cue.group_id
  ? snapshot.active_group_cue_ids?.[cue.group_id] === cue.id
  : snapshot.active_cue_id === cue.id;

/** True when no control of the page occupies the columns the panel covers. */
export const touchScenePanelColumnsFree = (page: TouchPageSummary | undefined) =>
  (page?.controls ?? []).every(
    (control) => control.x + control.w <= touchSurfaceGridColumns - touchScenePanelColumns,
  );

export type TouchScenePanelPreference = "open" | "closed" | null;

/** An explicit operator choice wins; otherwise open only over free columns. */
export const touchScenePanelOpen = (
  preference: TouchScenePanelPreference,
  page: TouchPageSummary | undefined,
) => preference === null ? touchScenePanelColumnsFree(page) : preference === "open";

export const readTouchScenePanelPreference = (): TouchScenePanelPreference => {
  try {
    const value = globalThis.localStorage?.getItem(touchScenePanelStorageKey);
    return value === "open" || value === "closed" ? value : null;
  } catch {
    return null;
  }
};

export const writeTouchScenePanelPreference = (value: Exclude<TouchScenePanelPreference, null>) => {
  try {
    globalThis.localStorage?.setItem(touchScenePanelStorageKey, value);
  } catch {
    // Per-device convenience only; the panel still toggles for this session.
  }
};
