import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { createMemo, createSignal, For, onMount, Show } from "solid-js";
import {
  fixtureCatalogSearchTextMatches,
  previewVerifiedProfiles,
  type GdtfFixtureCacheEntry,
  type VerifiedFixtureProfileSummary,
} from "../fixtureCatalog";
import type { FixtureProfileSummary, PatchedFixtureSummary } from "../types";
import { ProfileImportSources, type ProfileImportSourcesProps } from "./ProfileLoadPanel";

export interface PatchRecentProfileEntry {
  profile: FixtureProfileSummary;
  modeName: string;
}

export interface PatchProfileDragItem {
  source: "verified" | "cache" | "session" | "project";
  key: string;
  label: string;
  modeName: string;
  footprint: number;
  activate: () => boolean | Promise<boolean>;
}

interface PatchProfileBrowserPanelProps extends ProfileImportSourcesProps {
  backendAvailable: boolean;
  selectedProfile: FixtureProfileSummary | null;
  selectedMode: string;
  recentProfiles: PatchRecentProfileEntry[];
  projectFixtures: PatchedFixtureSummary[];
  onLoadVerified: (profileId: string, modeName: string) => boolean | Promise<boolean>;
  onLoadCached: (path: string, modeName: string | null) => boolean | Promise<boolean>;
  onLoadRecent: (entry: PatchRecentProfileEntry) => boolean | Promise<boolean>;
  onLoadProject: (fixture: PatchedFixtureSummary) => boolean | Promise<boolean>;
  onProfileDragStart: (item: PatchProfileDragItem) => void;
  onProfileDragEnd: () => void;
  onMessage: (message: string) => void;
}

interface RecentProjectProfileRow {
  fixture: PatchedFixtureSummary;
  key: string;
  manufacturer: string;
  name: string;
  modeName: string;
  footprint: number;
}

const normalized = (value: string) => value.trim().toLocaleLowerCase();

const profileModeFootprint = (profile: FixtureProfileSummary, modeName: string) => {
  const mode = profile.dmx_modes.find((candidate) => candidate.name === modeName) ?? profile.dmx_modes[0];
  return Math.max(0, ...(mode?.controls.flatMap((control) => control.offsets) ?? []));
};

const fixtureFootprint = (fixture: PatchedFixtureSummary) =>
  Math.max(0, ...fixture.controls.flatMap((control) => control.offsets));

const profileModeKey = (path: string, modeName: string) => `${normalized(path)}::${normalized(modeName)}`;

const textMatches = (values: string[], query: string) => {
  const needle = normalized(query);
  return !needle || values.some((value) => normalized(value).includes(needle));
};

export function PatchProfileBrowserPanel(props: PatchProfileBrowserPanelProps) {
  const [query, setQuery] = createSignal("");
  const [cacheEntries, setCacheEntries] = createSignal<GdtfFixtureCacheEntry[]>([]);
  const [verifiedProfiles, setVerifiedProfiles] = createSignal<VerifiedFixtureProfileSummary[]>(previewVerifiedProfiles);
  const [busy, setBusy] = createSignal(false);

  const refreshLocalProfiles = async () => {
    if (!props.backendAvailable) return;
    setBusy(true);
    try {
      const [cache, verified] = await Promise.all([
        tauriInvoke<GdtfFixtureCacheEntry[]>("list_gdtf_fixture_cache"),
        tauriInvoke<VerifiedFixtureProfileSummary[]>("list_verified_fixture_profiles"),
      ]);
      setCacheEntries(cache);
      setVerifiedProfiles(verified);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(false);
    }
  };

  onMount(() => void refreshLocalProfiles());

  const visibleVerified = createMemo(() => verifiedProfiles().filter((entry) => textMatches([
    entry.manufacturer,
    entry.name,
    entry.mode_name,
    entry.description,
  ], query())));

  const visibleCache = createMemo(() => cacheEntries().filter((entry) =>
    fixtureCatalogSearchTextMatches(entry, query())));

  const projectRows = createMemo(() => {
    const rows: RecentProjectProfileRow[] = [];
    const seen = new Set<string>();
    for (const fixture of props.projectFixtures) {
      const key = profileModeKey(fixture.profile_source_path, fixture.mode_name);
      if (seen.has(key)) continue;
      seen.add(key);
      rows.push({
        fixture,
        key,
        manufacturer: fixture.manufacturer,
        name: fixture.profile_name,
        modeName: fixture.mode_name,
        footprint: fixtureFootprint(fixture),
      });
    }
    return rows;
  });

  const recentRows = createMemo(() => {
    const sessionRows = props.recentProfiles.map((entry) => ({
      kind: "session" as const,
      key: profileModeKey(entry.profile.source_path, entry.modeName),
      entry,
      manufacturer: entry.profile.manufacturer,
      name: entry.profile.name,
      modeName: entry.modeName,
      footprint: profileModeFootprint(entry.profile, entry.modeName),
    }));
    const sessionKeys = new Set(sessionRows.map((entry) => entry.key));
    return [
      ...sessionRows,
      ...projectRows()
        .filter((entry) => !sessionKeys.has(entry.key))
        .map((entry) => ({ kind: "project" as const, ...entry })),
    ].filter((entry) => textMatches([
      entry.manufacturer,
      entry.name,
      entry.modeName,
      entry.kind === "session" ? "Session" : "Project",
    ], query()));
  });

  const selected = (path: string, manufacturer: string, name: string, modeName: string) => {
    const profile = props.selectedProfile;
    return Boolean(profile) && props.selectedMode === modeName && (
      normalized(profile!.source_path) === normalized(path) ||
      (profile!.manufacturer === manufacturer && profile!.name === name)
    );
  };

  const beginProfileDrag = (event: DragEvent, item: PatchProfileDragItem) => {
    const dataTransfer = event.dataTransfer;
    if (!dataTransfer) {
      event.preventDefault();
      return;
    }
    dataTransfer.effectAllowed = "copy";
    dataTransfer.setData("application/x-syndocal-fixture-profile", JSON.stringify({
      source: item.source,
      key: item.key,
      label: item.label,
      modeName: item.modeName,
      footprint: item.footprint,
    }));
    dataTransfer.setData("text/plain", `${item.label} / ${item.modeName}`);
    props.onProfileDragStart(item);
  };

  return (
    <section class="patchProfileBrowserPanel" data-patch-profile-browser>
      <header class="profileLoadHeader patchProfileBrowserHeader">
        <h2>Patch Source</h2>
        <button
          type="button"
          onClick={() => void refreshLocalProfiles()}
          disabled={!props.backendAvailable || busy()}
        >
          Refresh
        </button>
      </header>

      <label class="patchProfileSearch">
        <span>Search profiles</span>
        <input
          type="search"
          value={query()}
          placeholder="Fixture, manufacturer, or mode"
          onInput={(event) => setQuery(event.currentTarget.value)}
          data-patch-profile-search
        />
      </label>

      <div class="patchProfileBrowserScroll" data-patch-profile-browser-scroll>
        <section class="patchProfileBrowserSection" data-patch-profile-section="verified">
          <header><strong>Verified common-rig pack</strong><span>{visibleVerified().length}</span></header>
          <div class="patchProfileRows">
            <For each={visibleVerified()} fallback={<p class="empty patchProfileRowEmpty">No matching profiles.</p>}>
              {(entry) => (
                <button
                  type="button"
                  class="patchProfileRow"
                  data-patch-profile-row
                  data-profile-source="verified"
                  data-profile-footprint={entry.footprint}
                  draggable="true"
                  aria-pressed={selected(entry.id, entry.manufacturer, entry.name, entry.mode_name)}
                  disabled={!props.backendAvailable || busy()}
                  onClick={() => void props.onLoadVerified(entry.id, entry.mode_name)}
                  onDragStart={(event) => beginProfileDrag(event, {
                    source: "verified",
                    key: entry.id,
                    label: `${entry.manufacturer} ${entry.name}`,
                    modeName: entry.mode_name,
                    footprint: entry.footprint,
                    activate: () => props.onLoadVerified(entry.id, entry.mode_name),
                  })}
                  onDragEnd={props.onProfileDragEnd}
                >
                  <strong data-no-localize>{entry.name}</strong>
                  <span data-no-localize>{entry.mode_name} · {entry.footprint}ch</span>
                </button>
              )}
            </For>
          </div>
        </section>

        <section class="patchProfileBrowserSection" data-patch-profile-section="cache">
          <header><strong>Cached / offline GDTF</strong><span>{visibleCache().length}</span></header>
          <div class="patchProfileRows">
            <For each={visibleCache()} fallback={<p class="empty patchProfileRowEmpty">No cached GDTF profiles.</p>}>
              {(entry) => (
                <For each={entry.modes.length > 0 ? entry.modes : [{ name: "", dmx_footprint: null }]}>
                  {(mode) => (
                    <button
                      type="button"
                      class="patchProfileRow"
                      data-patch-profile-row
                      data-profile-source="cache"
                      data-profile-footprint={mode.dmx_footprint ?? 0}
                      draggable="true"
                      aria-pressed={selected(entry.path, entry.manufacturer, entry.fixture, mode.name)}
                      disabled={entry.health === "invalid" || busy()}
                      onClick={() => void props.onLoadCached(entry.path, mode.name || null)}
                      onDragStart={(event) => beginProfileDrag(event, {
                        source: "cache",
                        key: profileModeKey(entry.path, mode.name),
                        label: `${entry.manufacturer} ${entry.fixture}`,
                        modeName: mode.name,
                        footprint: mode.dmx_footprint ?? 0,
                        activate: () => props.onLoadCached(entry.path, mode.name || null),
                      })}
                      onDragEnd={props.onProfileDragEnd}
                    >
                      <strong data-no-localize>{entry.manufacturer} {entry.fixture}</strong>
                      <span data-no-localize>{mode.name || "Default"}{mode.dmx_footprint ? ` · ${mode.dmx_footprint}ch` : ""}</span>
                    </button>
                  )}
                </For>
              )}
            </For>
          </div>
        </section>

        <section class="patchProfileBrowserSection" data-patch-profile-section="recent">
          <header><strong>Recently used</strong><span>{recentRows().length}</span></header>
          <div class="patchProfileRows">
            <For each={recentRows()} fallback={<p class="empty patchProfileRowEmpty">No recently used profiles.</p>}>
              {(entry) => (
                <button
                  type="button"
                  class="patchProfileRow"
                  data-patch-profile-row
                  data-profile-source={entry.kind}
                  data-profile-footprint={entry.footprint}
                  draggable="true"
                  aria-pressed={entry.kind === "session"
                    ? selected(entry.entry.profile.source_path, entry.manufacturer, entry.name, entry.modeName)
                    : selected(entry.fixture.profile_source_path, entry.manufacturer, entry.name, entry.modeName)}
                  onClick={() => void (entry.kind === "session"
                    ? props.onLoadRecent(entry.entry)
                    : props.onLoadProject(entry.fixture))}
                  onDragStart={(event) => beginProfileDrag(event, {
                    source: entry.kind,
                    key: entry.key,
                    label: `${entry.manufacturer} ${entry.name}`,
                    modeName: entry.modeName,
                    footprint: entry.footprint,
                    activate: () => entry.kind === "session"
                      ? props.onLoadRecent(entry.entry)
                      : props.onLoadProject(entry.fixture),
                  })}
                  onDragEnd={props.onProfileDragEnd}
                >
                  <strong data-no-localize>{entry.manufacturer} {entry.name}</strong>
                  <span>
                    <i>{entry.kind === "session" ? "Session" : "Project"}</i>
                    <b data-no-localize>{entry.modeName} · {entry.footprint}ch</b>
                  </span>
                </button>
              )}
            </For>
          </div>
        </section>

        <ProfileImportSources {...props} folded />
      </div>
    </section>
  );
}
