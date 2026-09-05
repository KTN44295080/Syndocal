import { createEffect, onCleanup, untrack, type Accessor } from "solid-js";
import type { ControlMode, EditDeskSurface, SetupSubTab, TimelineDeskSurface, WorkspaceTab } from "./uiModes";

export interface WorkspaceNavigationRoute {
  workspace: WorkspaceTab;
  setupSubTab?: SetupSubTab;
  activeIoConnection?: string;
  controlMode?: ControlMode;
  editDeskSurface?: EditDeskSurface;
  timelineDeskSurface?: TimelineDeskSurface;
  timelineChildCueId?: number | null;
  touchControlDomain?: "lighting" | "video";
}
interface HistoryPort {
  readonly state: unknown;
  pushState(data: unknown, unused: string): void;
  replaceState(data: unknown, unused: string): void;
  go(delta: number): void;
}
interface Options {
  route: Accessor<WorkspaceNavigationRoute>;
  projectEpoch: Accessor<number>;
  history: HistoryPort;
  events: Pick<Window, "addEventListener" | "removeEventListener">;
  restore: (route: WorkspaceNavigationRoute) => boolean;
  /** An unpredictable identity unique to this mounted WebView. */
  sessionId: string;
}
interface Entry { index: number; epoch: number; route: WorkspaceNavigationRoute | null }

// Browser state is only a pointer into this instance's private ledger. Never
// restore a caller-supplied route, a previous mount's state, or a prior project.
export function createWorkspaceNavigationController(options: Options) {
  const entries = new Map<number, Entry>();
  const entryTokens = new Map<number, number>();
  let nextToken = 0;
  let current: Entry | null = null;
  let initialized = false;
  let restoring = false;
  let repairing = false;
  let disposed = false;
  const stateFor = (index: number) => {
    const token = ++nextToken;
    entryTokens.set(index, token);
    return { syndocalNavigation: 1, session: options.sessionId, index, token };
  };
  const readIndex = (state: unknown): number | null => {
    if (!state || typeof state !== "object") return null;
    const value = state as Record<string, unknown>;
    return value.syndocalNavigation === 1 && value.session === options.sessionId
      && Number.isSafeInteger(value.index) && entries.has(value.index as number)
      && value.token === entryTokens.get(value.index as number)
      ? value.index as number : null;
  };
  const copy = (route: WorkspaceNavigationRoute) => ({ ...route });
  const same = (a: WorkspaceNavigationRoute, b: WorkspaceNavigationRoute) => {
    const keys = Object.keys(a) as (keyof WorkspaceNavigationRoute)[];
    return keys.length === Object.keys(b).length && keys.every((key) => a[key] === b[key]);
  };
  const push = (route: WorkspaceNavigationRoute, epoch: number) => {
    const index = (readIndex(options.history.state) ?? current?.index ?? -1) + 1;
    for (const key of entries.keys()) if (key >= index) entries.delete(key);
    current = { index, epoch, route: copy(route) };
    entries.set(index, current);
    options.history.pushState(stateFor(index), "");
  };
  const repair = (targetIndex: number | null) => {
    if (!current) return;
    if (targetIndex === null) {
      // Unknown same-document state has no trusted offset. Replace its marker
      // with a fresh lower boundary and current route; never consume its data.
      entries.clear();
      entries.set(0, { index: 0, epoch: current.epoch, route: null });
      options.history.replaceState(stateFor(0), "");
      const route = current.route!;
      const epoch = current.epoch;
      current = { index: 0, epoch, route: null };
      push(route, epoch);
      return;
    }
    const delta = current.index - targetIndex;
    if (delta !== 0) { repairing = true; options.history.go(delta); }
  };
  const onPopState = (event: PopStateEvent) => {
    if (disposed || !current) return;
    const index = readIndex(event.state);
    // A traversal may already have been superseded by a new route/branch.
    // Index alone is insufficient when a branch reuses that index.
    if (index !== readIndex(options.history.state)) return;
    if (repairing && index === current.index) { repairing = false; return; }
    const target = index === null ? null : entries.get(index)!;
    if (!target?.route || target.epoch !== untrack(options.projectEpoch)) { repair(index); return; }
    if (index === current.index) return;
    let accepted = false;
    restoring = true;
    try { accepted = untrack(() => options.restore(copy(target.route!))); }
    catch { accepted = false; }
    finally { restoring = false; }
    if (accepted) { current = target; repairing = false; }
    else repair(index);
  };
  options.events.addEventListener("popstate", onPopState as EventListener);
  createEffect(() => {
    const epoch = options.projectEpoch();
    const route = options.route();
    if (disposed || restoring) return;
    if (!initialized) {
      initialized = true;
      entries.set(0, { index: 0, epoch, route: null });
      options.history.replaceState(stateFor(0), "");
      current = { index: 0, epoch, route: null };
      push(route, epoch);
      return;
    }
    if (current && current.epoch === epoch && current.route && same(current.route, route)) return;
    // A project boundary becomes the new lower guard. Older ledger entries
    // remain recognizable solely so a queued traversal can be bounced back.
    if (current && current.epoch !== epoch) {
      current = { index: readIndex(options.history.state) ?? current.index, epoch, route: null };
      entries.set(current.index, current);
      options.history.replaceState(stateFor(current.index), "");
    }
    repairing = false;
    push(route, epoch);
  });
  onCleanup(() => {
    disposed = true;
    options.events.removeEventListener("popstate", onPopState as EventListener);
    entries.clear();
    entryTokens.clear();
  });
}
