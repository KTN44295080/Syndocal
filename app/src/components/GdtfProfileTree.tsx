import { createEffect, createMemo, createSignal, For, Show } from "solid-js";

export interface GdtfProfileTreeMode {
  key: string;
  name: string;
  modeName: string | null;
  footprint: number;
  description?: string;
}

export interface GdtfProfileTreeFixture {
  key: string;
  manufacturer: string;
  fixture: string;
  revision: string;
  provenance?: string;
  modeCount: number;
  modes: GdtfProfileTreeMode[];
  manufacturerDescription?: string;
}

interface GdtfProfileTreeManufacturerGroup {
  key: string;
  manufacturer: string;
  fixtures: GdtfProfileTreeFixture[];
  profileCount: number;
  description?: string;
}

export interface GdtfProfileTreeManufacturerBatchState {
  manufacturer: string;
  phase: "enumerating" | "downloading" | "stopping" | "complete" | "cancelled" | "error";
  processed: number;
  total: number;
  cached: number;
  failures: number;
}

interface GdtfProfileTreeProps {
  ariaLabel: string;
  source: "verified" | "bundled" | "cache" | "share";
  fixtures: GdtfProfileTreeFixture[];
  searchActive: boolean;
  disabled: (fixture: GdtfProfileTreeFixture, mode: GdtfProfileTreeMode) => boolean;
  draggable: (fixture: GdtfProfileTreeFixture, mode: GdtfProfileTreeMode) => boolean;
  selected: (fixture: GdtfProfileTreeFixture, mode: GdtfProfileTreeMode) => boolean;
  downloading?: (fixture: GdtfProfileTreeFixture, mode: GdtfProfileTreeMode) => boolean;
  cached?: (fixture: GdtfProfileTreeFixture, mode: GdtfProfileTreeMode) => boolean;
  manufacturerBatch?: (manufacturer: string) => GdtfProfileTreeManufacturerBatchState | null;
  manufacturerBatchDisabled?: (manufacturer: string) => boolean;
  onManufacturerBatch?: (manufacturer: string) => void;
  onCancelManufacturerBatch?: (manufacturer: string) => void;
  onActivate: (fixture: GdtfProfileTreeFixture, mode: GdtfProfileTreeMode) => void;
  onDragStart: (
    event: DragEvent,
    fixture: GdtfProfileTreeFixture,
    mode: GdtfProfileTreeMode,
  ) => void;
  onDragEnd: () => void;
}

const normalized = (value: string) => value.trim().toLocaleLowerCase();

const setMembership = (
  setter: (update: (current: Set<string>) => Set<string>) => void,
  key: string,
  expanded: boolean,
) => setter((current) => {
  const next = new Set(current);
  if (expanded) next.add(key);
  else next.delete(key);
  return next;
});

const handleBranchKeyDown = (
  event: KeyboardEvent,
  expanded: boolean,
  setExpanded: (expanded: boolean) => void,
) => {
  if (event.key === "Enter") {
    event.preventDefault();
    setExpanded(!expanded);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    setExpanded(true);
  } else if (event.key === "ArrowLeft") {
    event.preventDefault();
    setExpanded(false);
  }
};

export const filterGdtfProfileTreeFixtures = (
  fixtures: GdtfProfileTreeFixture[],
  query: string,
) => {
  const needle = normalized(query);
  if (!needle) return fixtures;
  const footprint = /^\d{1,3}$/.test(needle) ? Number(needle) : null;
  return fixtures.flatMap((fixture) => {
    const manufacturerMatches = normalized(fixture.manufacturer).includes(needle);
    const fixtureMatches = [fixture.fixture, fixture.revision]
      .some((value) => normalized(value).includes(needle));
    const modes = manufacturerMatches || fixtureMatches
      ? fixture.modes
      : fixture.modes.filter((mode) =>
        normalized(mode.name).includes(needle)
        || normalized(mode.description ?? "").includes(needle)
        || (footprint !== null && mode.footprint === footprint));
    return manufacturerMatches || fixtureMatches || modes.length > 0
      ? [{ ...fixture, modes }]
      : [];
  });
};

export function GdtfProfileTree(props: GdtfProfileTreeProps) {
  const [expandedManufacturers, setExpandedManufacturers] = createSignal(new Set<string>());
  const [expandedFixtures, setExpandedFixtures] = createSignal(new Set<string>());
  let wasSearching = props.searchActive;

  createEffect(() => {
    const searching = props.searchActive;
    if (wasSearching && !searching) {
      setExpandedManufacturers(new Set<string>());
      setExpandedFixtures(new Set<string>());
    }
    wasSearching = searching;
  });

  const groups = createMemo<GdtfProfileTreeManufacturerGroup[]>(() => {
    const grouped = new Map<string, GdtfProfileTreeManufacturerGroup>();
    for (const fixture of props.fixtures) {
      const key = normalized(fixture.manufacturer);
      const current = grouped.get(key) ?? {
        key,
        manufacturer: fixture.manufacturer,
        fixtures: [],
        profileCount: 0,
        description: fixture.manufacturerDescription,
      };
      current.fixtures.push(fixture);
      current.profileCount += fixture.modes.length;
      grouped.set(key, current);
    }
    const ordered = [...grouped.values()].map((group) => ({
        ...group,
        fixtures: props.source === "verified"
          ? group.fixtures
          : [...group.fixtures].sort((left, right) =>
            left.fixture.localeCompare(right.fixture) || left.revision.localeCompare(right.revision)),
      }));
    return props.source === "verified"
      ? ordered
      : ordered.sort((left, right) => left.manufacturer.localeCompare(right.manufacturer));
  });

  const manufacturerExpanded = (key: string) =>
    props.searchActive || expandedManufacturers().has(key);
  const fixtureExpanded = (key: string) => props.searchActive || expandedFixtures().has(key);

  const setManufacturerExpanded = (key: string, expanded: boolean) =>
    setMembership(setExpandedManufacturers, key, expanded);
  const setFixtureExpanded = (key: string, expanded: boolean) =>
    setMembership(setExpandedFixtures, key, expanded);

  return (
    <div
      class="patchProfileTree"
      role="tree"
      aria-label={props.ariaLabel}
      data-patch-profile-tree={props.source}
    >
      <For each={groups()}>
        {(group) => {
          const expanded = () => manufacturerExpanded(group.key);
          const batch = () => props.manufacturerBatch?.(group.manufacturer) ?? null;
          const batchActive = () => {
            const phase = batch()?.phase;
            return phase === "enumerating" || phase === "downloading" || phase === "stopping";
          };
          return (
            <div
              class="patchProfileTreeManufacturer"
              data-profile-tree-manufacturer={group.manufacturer}
              data-profile-tree-category={props.source === "verified" ? group.manufacturer : undefined}
            >
              <div
                class={`patchProfileTreeManufacturerHeader${expanded() ? " is-expanded" : ""}`}
                data-share-manufacturer-batch-visible={batch() ? "true" : undefined}
              >
                <button
                  type="button"
                  class="patchProfileTreeBranchRow patchProfileTreeManufacturerRow"
                  role="treeitem"
                  aria-level="1"
                  aria-expanded={expanded()}
                  title={props.source === "verified" ? group.description : undefined}
                  data-profile-tree-item="manufacturer"
                  data-profile-tree-profile-count={props.source === "verified" ? group.profileCount : undefined}
                  onClick={() => setManufacturerExpanded(group.key, !expanded())}
                  onKeyDown={(event) => handleBranchKeyDown(
                    event,
                    expanded(),
                    (next) => setManufacturerExpanded(group.key, next),
                  )}
                >
                  <span class="patchProfileTreeChevron" aria-hidden="true">{expanded() ? "▾" : "▸"}</span>
                  <strong data-no-localize={props.source === "verified" ? undefined : ""}>{group.manufacturer}</strong>
                  <span class="patchProfileTreeCount">
                    <b data-no-localize>{props.source === "verified" ? group.profileCount : group.fixtures.length}</b>{" "}
                    {props.source === "verified"
                      ? (group.profileCount === 1 ? "Profile" : "profiles")
                      : (group.fixtures.length === 1 ? "Fixture" : "fixtures")}
                  </span>
                </button>

                <Show when={props.source === "share" && props.onManufacturerBatch && props.onCancelManufacturerBatch}>
                  <div
                    class="patchShareManufacturerBatch"
                    data-share-manufacturer-batch-state={batch()?.phase ?? "idle"}
                    data-share-manufacturer-batch-manufacturer={group.manufacturer}
                    aria-live="polite"
                  >
                    <Show when={batch()}>
                      {(state) => (
                        <span class="patchShareManufacturerBatchStatus" data-share-manufacturer-batch-status>
                          <Show when={state().phase === "enumerating"}>Listing catalog…</Show>
                          <Show when={state().phase === "downloading" || state().phase === "stopping"}>
                            <b data-no-localize>{state().processed}/{state().total}</b><span aria-hidden="true">…</span>
                            <Show when={state().failures > 0}>
                              {" ("}<b data-no-localize>{state().failures}</b>{" "}<span>failed</span>{")"}
                            </Show>
                            <Show when={state().phase === "stopping"}>{" · "}<span>Stopping…</span></Show>
                          </Show>
                          <Show when={state().phase === "complete" || state().phase === "cancelled"}>
                            <b data-no-localize>{state().cached}/{state().total}</b>{" "}<span>cached</span>
                            <Show when={state().failures > 0}>
                              {" ("}<b data-no-localize>{state().failures}</b>{" "}<span>failed</span>{")"}
                            </Show>
                            <Show when={state().phase === "cancelled"}>{" · "}<span>Canceled</span></Show>
                          </Show>
                          <Show when={state().phase === "error"}>Catalog failed</Show>
                        </span>
                      )}
                    </Show>
                    <button
                      type="button"
                      class="patchShareManufacturerBatchAction"
                      title={batchActive() ? "Stop manufacturer cache" : "Cache all manufacturer revisions"}
                      aria-label={batchActive() ? "Stop manufacturer cache" : "Cache all manufacturer revisions"}
                      data-share-manufacturer-batch-action
                      disabled={batch()?.phase === "stopping" || (!batchActive() && props.manufacturerBatchDisabled?.(group.manufacturer))}
                      onClick={() => batchActive()
                        ? props.onCancelManufacturerBatch?.(group.manufacturer)
                        : props.onManufacturerBatch?.(group.manufacturer)}
                    >
                      <span aria-hidden="true" data-no-localize>{batchActive() ? "■" : "↓"}</span>
                    </button>
                  </div>
                </Show>
              </div>

              <Show when={expanded()}>
                <div class="patchProfileTreeGroup" role="group">
                  <For each={group.fixtures}>
                    {(fixture) => {
                      const expandedFixture = () => fixtureExpanded(fixture.key);
                      return (
                        <div class="patchProfileTreeFixture" data-profile-tree-fixture={fixture.key}>
                            <button
                              type="button"
                              class="patchProfileTreeBranchRow patchProfileTreeFixtureRow"
                              role="treeitem"
                              aria-level="2"
                              aria-expanded={expandedFixture()}
                              data-profile-tree-item="fixture"
                              data-profile-fixture-key={fixture.key}
                              onClick={() => setFixtureExpanded(fixture.key, !expandedFixture())}
                              onKeyDown={(event) => handleBranchKeyDown(
                                event,
                                expandedFixture(),
                                (next) => setFixtureExpanded(fixture.key, next),
                              )}
                            >
                              <span class="patchProfileTreeChevron" aria-hidden="true">{expandedFixture() ? "▾" : "▸"}</span>
                              <strong data-no-localize>{fixture.fixture}</strong>
                              <span class="patchProfileTreeCount">
                                <b data-no-localize>{fixture.modeCount}</b>{" "}
                                {fixture.modeCount === 1 ? "Mode" : "Modes"}
                                <Show when={fixture.revision}>
                                  <i><span>Rev.</span>{" "}<span data-no-localize>{fixture.revision}</span></i>
                                </Show>
                                <Show when={fixture.provenance}>
                                  <i data-profile-provenance data-no-localize>{fixture.provenance}</i>
                                </Show>
                              </span>
                            </button>

                            <Show when={expandedFixture()}>
                              <div class="patchProfileTreeGroup patchProfileTreeModeGroup" role="group">
                                <For each={fixture.modes}>
                                  {(mode) => (
                                    <button
                                      type="button"
                                      class={`patchProfileRow patchProfileTreeRow patchProfileTreeModeRow${props.source === "share" ? " patchShareProfileRow" : ""}`}
                                      role="treeitem"
                                      aria-level="3"
                                      aria-selected={props.selected(fixture, mode)}
                                      aria-pressed={props.selected(fixture, mode)}
                                      title={mode.description}
                                      data-patch-profile-row
                                      data-profile-source={props.source}
                                      data-profile-tree-item="mode"
                                      data-profile-fixture-key={fixture.key}
                                      data-profile-mode-key={mode.key}
                                      data-profile-mode-name={mode.modeName ?? ""}
                                      data-profile-footprint={mode.footprint}
                                      data-profile-cached={props.cached?.(fixture, mode) ? "true" : props.source === "share" ? "false" : undefined}
                                      data-share-profile-key={props.source === "share" ? mode.key : undefined}
                                      draggable={props.draggable(fixture, mode)}
                                      disabled={props.disabled(fixture, mode)}
                                      onClick={() => props.onActivate(fixture, mode)}
                                      onDragStart={(event) => props.onDragStart(event, fixture, mode)}
                                      onDragEnd={props.onDragEnd}
                                    >
                                      <strong data-no-localize>{mode.name}</strong>
                                      <span>
                                        <Show when={props.downloading?.(fixture, mode)} fallback={(
                                          <>
                                            <Show when={props.source === "verified"}>
                                              <i class="patchVerifiedChip">Verified</i>
                                            </Show>
                                            <b data-no-localize>{mode.footprint > 0 ? `${mode.footprint}ch` : "—"}</b>
                                          </>
                                        )}>
                                          <i>Downloading…</i>
                                        </Show>
                                      </span>
                                    </button>
                                  )}
                                </For>
                              </div>
                            </Show>
                        </div>
                      );
                    }}
                  </For>
                </div>
              </Show>
            </div>
          );
        }}
      </For>
    </div>
  );
}
