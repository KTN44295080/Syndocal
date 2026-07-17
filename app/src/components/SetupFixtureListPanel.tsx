import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import type { PatchedFixtureSummary } from "../types";
import { virtualListRange } from "../virtualList";
import { groupIdentityCss } from "../identityColor";

export interface SetupFixtureGroupRow {
  groupId: string;
  count: number;
}

const fixtureVirtualizationThreshold = 80;
const fixtureVirtualRowHeight = 104;
const fixtureVirtualOverscan = 5;

interface SetupFixtureListPanelProps {
  fixtures: PatchedFixtureSummary[];
  totalFixtureCount: number;
  groupColors?: Record<string, string>;
  selectedGroupId: string | null;
  groupRows: SetupFixtureGroupRow[];
  selectedFixtureId: number | null;
  children?: JSX.Element;
  onSelectGroup: (groupId: string | null) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
}

export function SetupFixtureListPanel(props: SetupFixtureListPanelProps) {
  let fixtureListElement: HTMLDivElement | undefined;
  const [scrollTop, setScrollTop] = createSignal(0);
  const [viewportHeight, setViewportHeight] = createSignal(480);
  const virtualized = createMemo(() => props.fixtures.length > fixtureVirtualizationThreshold);
  const virtualRange = createMemo(() => virtualListRange({
    itemCount: props.fixtures.length,
    itemHeight: fixtureVirtualRowHeight,
    scrollTop: scrollTop(),
    viewportHeight: viewportHeight(),
    overscan: fixtureVirtualOverscan,
  }));
  const visibleFixtures = createMemo(() => {
    const range = virtualRange();
    return props.fixtures.slice(range.startIndex, range.endIndex);
  });

  onMount(() => {
    const measure = () => setViewportHeight(fixtureListElement?.clientHeight || 480);
    measure();
    if (typeof ResizeObserver === "undefined" || !fixtureListElement) return;
    const observer = new ResizeObserver(measure);
    observer.observe(fixtureListElement);
    onCleanup(() => observer.disconnect());
  });

  createEffect(() => {
    const selectedIndex = props.fixtures.findIndex((fixture) => fixture.id === props.selectedFixtureId);
    const list = fixtureListElement;
    if (!virtualized() || selectedIndex < 0 || !list) return;
    const rowTop = selectedIndex * fixtureVirtualRowHeight;
    const rowBottom = rowTop + fixtureVirtualRowHeight;
    if (rowTop < list.scrollTop) list.scrollTop = rowTop;
    else if (rowBottom > list.scrollTop + list.clientHeight) list.scrollTop = rowBottom - list.clientHeight;
  });

  const fixtureButton = (fixture: PatchedFixtureSummary) => (
    <button
      class={fixture.id === props.selectedFixtureId ? "fixture selected" : "fixture"}
      aria-current={fixture.id === props.selectedFixtureId ? "true" : undefined}
      onClick={() => props.onSelectFixture(fixture)}
    >
      <strong data-no-localize>{fixture.label}</strong>
      <span data-no-localize>{fixture.manufacturer} {fixture.profile_name}</span>
      <small data-no-localize>U{fixture.universe} A{fixture.address} / {fixture.mode_name}</small>
      <Show when={fixture.highlighted || fixture.soloed || fixture.parked}>
        <small>
          {[fixture.highlighted ? "Highlight" : "", fixture.soloed ? "Solo" : "", fixture.parked ? "Park" : ""]
            .filter(Boolean)
            .join(" / ")}
        </small>
      </Show>
      <Show when={fixture.group_ids.length > 0}>
        <small data-no-localize>{fixture.group_ids.join(", ")}</small>
      </Show>
    </button>
  );

  return (
    <>
      <div class="panelHeader">
        <h2>Fixtures</h2>
        <span>{props.fixtures.length} / {props.totalFixtureCount}</span>
      </div>
      <div class="groupOverview">
        <button
          class={!props.selectedGroupId ? "groupChip active" : "groupChip"}
          onClick={() => props.onSelectGroup(null)}
        >
          All
              <span data-no-localize>{props.totalFixtureCount}</span>
        </button>
        <For each={props.groupRows}>
          {(group) => (
            <button
              class={props.selectedGroupId === group.groupId ? "groupChip active" : "groupChip"}
              style={{ "--identity": groupIdentityCss(group.groupId, props.groupColors, "fill") }}
              onClick={() => props.onSelectGroup(group.groupId)}
            >
              <span data-no-localize>{group.groupId}</span>
              <span>{group.count}</span>
            </button>
          )}
        </For>
      </div>
      {props.children}
      <div
        ref={fixtureListElement}
        class={`fixtureList ${virtualized() ? "virtualizedFixtureList" : ""}`}
        role="list"
        aria-label="Patched fixtures"
        aria-rowcount={props.fixtures.length}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        <Show
          when={virtualized()}
          fallback={<For each={props.fixtures}>{(fixture) => <div role="listitem">{fixtureButton(fixture)}</div>}</For>}
        >
          <div class="virtualFixtureSpacer" style={{ height: `${virtualRange().totalHeightPx}px` }}>
            <div class="virtualFixtureWindow" style={{ transform: `translateY(${virtualRange().offsetPx}px)` }}>
              <For each={visibleFixtures()}>{(fixture) => <div class="virtualFixtureRow" role="listitem">{fixtureButton(fixture)}</div>}</For>
            </div>
          </div>
        </Show>
        <Show when={props.totalFixtureCount === 0}>
          <p class="empty">No fixtures patched. Load a profile in Library, then use Patch Fixture.</p>
        </Show>
        <Show when={props.totalFixtureCount > 0 && props.fixtures.length === 0}>
          <p class="empty">No fixtures in group.</p>
        </Show>
      </div>
    </>
  );
}
