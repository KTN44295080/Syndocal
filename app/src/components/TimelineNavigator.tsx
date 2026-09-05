import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import type { CueSummary, TimelineSnapshot } from "../types";
import "./TimelineNavigator.css";

interface Props {
  timelines: readonly Pick<TimelineSnapshot, "id" | "label">[];
  childCues: readonly Pick<CueSummary, "id" | "label">[];
  activeTimelineId: number;
  selectedChildCueId: number | null;
  onSelectRoot: (timelineId: number) => void | Promise<void>;
  onSelectChild: (cueId: number) => void | Promise<void>;
  onClose: () => void;
}

export function TimelineNavigator(props: Props) {
  const roots = createMemo(() => new Map(props.timelines.map((timeline) => [timeline.id ?? 0, timeline.label])));
  const children = createMemo(() => new Map(props.childCues.map((cue) => [cue.id, cue.label])));
  const sameIds = (a: number[], b: number[]) => a.length === b.length && a.every((id, index) => id === b[index]);
  const rootIds = createMemo(() => [...roots().keys()], [], { equals: sameIds });
  const childIds = createMemo(() => [...children().keys()], [], { equals: sameIds });
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let mounted = true;
  onCleanup(() => { mounted = false; });
  const select = async (action: () => void | Promise<void>) => {
    if (busy()) return;
    setBusy(true); setError(null);
    try { await action(); }
    catch { if (mounted) setError("タイムラインを切り替えられませんでした。"); }
    finally { if (mounted) setBusy(false); }
  };
  return <aside id="timeline-navigator" class="timelineNavigator" aria-label="タイムライン一覧" data-timeline-navigator>
    <header><strong>タイムライン一覧</strong><button type="button" aria-label="タイムライン一覧を閉じる" onClick={props.onClose}>閉じる</button></header>
    <div class="timelineNavigatorScroll">
      <Show when={error()}><p role="alert">{error()}</p></Show>
      <section aria-label="統合タイムライン">
        <h3>統合タイムライン</h3>
        <Show when={props.timelines.length > 0} fallback={<p>タイムラインがありません</p>}>
          <For each={rootIds()}>{(id, index) => <button type="button"
            data-timeline-navigator-root={id} disabled={busy()}
            aria-current={props.selectedChildCueId === null && props.activeTimelineId === id ? "page" : undefined}
            title={roots().get(id)} onClick={() => void select(() => props.onSelectRoot(id))}>
            <span data-no-localize>{roots().get(id)?.trim() || `タイムライン ${index() + 1}`}</span>
          </button>}</For>
        </Show>
      </section>
      <section aria-label="シーン内タイムライン">
        <h3>シーン内タイムライン</h3>
        <Show when={props.childCues.length > 0} fallback={<p>シーン内タイムラインがありません</p>}>
          <For each={childIds()}>{(id) => <button type="button"
            data-timeline-navigator-child={id} disabled={busy()}
            aria-current={props.selectedChildCueId === id ? "page" : undefined}
            title={children().get(id)} onClick={() => void select(() => props.onSelectChild(id))}>
            <span data-no-localize>{children().get(id)}</span>
          </button>}</For>
        </Show>
      </section>
    </div>
  </aside>;
}
