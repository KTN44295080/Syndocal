import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import {
  fixtureCatalogFavoriteKey,
  fixtureCatalogIdentityMatches,
  fixtureCatalogSearchTextMatches,
  previewVerifiedProfiles,
  type GdtfFixtureCacheEntry,
  type GdtfShareDownloadRequest,
  type GdtfShareFixtureSummary,
  type GdtfShareSearchRequest,
  type GdtfShareSearchResponse,
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
  shareUser: string;
  sharePassword: string;
  selectedProfile: FixtureProfileSummary | null;
  selectedMode: string;
  recentProfiles: PatchRecentProfileEntry[];
  projectFixtures: PatchedFixtureSummary[];
  onLoadVerified: (profileId: string, modeName: string) => boolean | Promise<boolean>;
  onLoadCached: (path: string, modeName: string | null) => boolean | Promise<boolean>;
  onLoadRecent: (entry: PatchRecentProfileEntry) => boolean | Promise<boolean>;
  onLoadProject: (fixture: PatchedFixtureSummary) => boolean | Promise<boolean>;
  onShareUser: (value: string) => void;
  onSharePassword: (value: string) => void;
  onOpenLibrary: () => void;
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

type PatchShareState =
  | { kind: "idle" | "signed-out" | "searching" | "results" | "empty" | "offline" }
  | {
    kind: "error";
    category: "auth" | "generic";
    message: string;
    searchQuery: string;
  };

interface PatchShareManufacturerGroup {
  manufacturer: string;
  fixtures: GdtfShareFixtureSummary[];
}

const emptyShareSearchResponse = (): GdtfShareSearchResponse => ({
  fixtures: [],
  facets: { manufacturers: [], modes: [], versions: [] },
  filter_support: {
    release_status: null,
    tested_in_visualizer: null,
    tested_in_real_life: null,
  },
  total_matches: 0,
});

const normalized = (value: string) => value.trim().toLocaleLowerCase();

const gdtfShareConnectivityErrorPatterns = [
  "failed to start curl",
  "curl failed",
  "could not resolve",
  "failed to connect",
  "could not connect",
  "timed out",
  "timeout",
  "connection refused",
  "connection reset",
  "network is unreachable",
];

const gdtfShareAuthErrorPatterns = [
  "user or password",
  "unauthorized",
  "login",
  "invalid credential",
  "authentication failed",
];

const backendErrorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);

const isGdtfShareConnectivityError = (message: string) => {
  const value = normalized(message);
  return gdtfShareConnectivityErrorPatterns.some((pattern) => value.includes(pattern));
};

const isGdtfShareAuthError = (message: string) => {
  const value = normalized(message);
  return gdtfShareAuthErrorPatterns.some((pattern) => value.includes(pattern));
};

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
  const [shareResponse, setShareResponse] = createSignal(emptyShareSearchResponse());
  const [shareState, setShareState] = createSignal<PatchShareState>(
    { kind: props.backendAvailable ? "signed-out" : "offline" },
  );
  const [downloadingShareKey, setDownloadingShareKey] = createSignal<string | null>(null);
  let shareSearchGeneration = 0;
  let pausedAuthErrorQuery: string | null = null;

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

  const shareCredentialsReady = () => Boolean(props.shareUser.trim() && props.sharePassword);

  const shareGroups = createMemo<PatchShareManufacturerGroup[]>(() => {
    const grouped = new Map<string, GdtfShareFixtureSummary[]>();
    for (const fixture of shareResponse().fixtures) {
      const entries = grouped.get(fixture.manufacturer) ?? [];
      entries.push(fixture);
      grouped.set(fixture.manufacturer, entries);
    }
    return [...grouped.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([manufacturer, fixtures]) => ({
        manufacturer,
        fixtures: [...fixtures].sort((left, right) =>
          left.fixture.localeCompare(right.fixture) || left.revision.localeCompare(right.revision)),
      }));
  });

  const cachedShareEntry = (fixture: GdtfShareFixtureSummary) => cacheEntries().find((candidate) =>
    candidate.health !== "invalid" && fixtureCatalogIdentityMatches(fixture, candidate));

  const searchShare = async (searchQuery: string, generation: number) => {
    const request: GdtfShareSearchRequest = {
      user: props.shareUser,
      password: props.sharePassword,
      manufacturer: null,
      fixture: null,
      query: searchQuery,
      mode: null,
      min_footprint: null,
      max_footprint: null,
      release_only: false,
      tested_in_visualizer: false,
      tested_in_real_life: false,
      limit: 80,
    };
    try {
      const response = await tauriInvoke<GdtfShareSearchResponse>("search_gdtf_share", { request });
      if (generation !== shareSearchGeneration) return;
      pausedAuthErrorQuery = null;
      setShareResponse(response);
      setShareState({ kind: response.fixtures.length > 0 ? "results" : "empty" });
      props.onMessage(`GDTF Share: showing ${response.fixtures.length} of ${response.total_matches} matching revisions.`);
    } catch (error) {
      if (generation !== shareSearchGeneration) return;
      const message = backendErrorMessage(error);
      setShareResponse(emptyShareSearchResponse());
      if (isGdtfShareConnectivityError(message)) {
        setShareState({ kind: "offline" });
      } else {
        const category = isGdtfShareAuthError(message) ? "auth" : "generic";
        if (category === "auth") pausedAuthErrorQuery = searchQuery;
        setShareState({ kind: "error", category, message, searchQuery });
      }
      props.onMessage(String(error));
    }
  };

  const retryShareSearch = (searchQuery: string) => {
    if (!props.backendAvailable || !shareCredentialsReady() || searchQuery.length < 2) return;
    pausedAuthErrorQuery = null;
    const generation = ++shareSearchGeneration;
    setShareResponse(emptyShareSearchResponse());
    setShareState({ kind: "searching" });
    void searchShare(searchQuery, generation);
  };

  createEffect(() => {
    const backendAvailable = props.backendAvailable;
    const credentialsReady = shareCredentialsReady();
    const searchQuery = query().trim();
    const generation = ++shareSearchGeneration;
    setShareResponse(emptyShareSearchResponse());
    if (!backendAvailable) {
      setShareState({ kind: "offline" });
      return;
    }
    if (pausedAuthErrorQuery === searchQuery) return;
    if (!credentialsReady) {
      setShareState({ kind: "signed-out" });
      return;
    }
    if (searchQuery.length < 2) {
      setShareState({ kind: "idle" });
      return;
    }
    setShareState({ kind: "searching" });
    const timer = window.setTimeout(() => void searchShare(searchQuery, generation), 450);
    onCleanup(() => window.clearTimeout(timer));
  });

  const shareStateKind = () => shareState().kind;
  const shareError = () => {
    const state = shareState();
    return state.kind === "error" ? state : null;
  };

  const downloadAndUseShareProfile = async (fixture: GdtfShareFixtureSummary) => {
    const alreadyCached = cachedShareEntry(fixture);
    const preferredMode = alreadyCached?.modes[0]?.name || fixture.modes[0]?.name || null;
    if (alreadyCached) return props.onLoadCached(alreadyCached.path, preferredMode);
    if (!props.backendAvailable || !shareCredentialsReady()) return false;

    const key = fixtureCatalogFavoriteKey(fixture);
    const request: GdtfShareDownloadRequest = {
      user: props.shareUser,
      password: props.sharePassword,
      rid: fixture.rid,
      uuid: fixture.uuid,
      manufacturer: fixture.manufacturer,
      fixture: fixture.fixture,
      revision: fixture.revision,
    };
    setDownloadingShareKey(key);
    try {
      const entry = await tauriInvoke<GdtfFixtureCacheEntry>("cache_gdtf_from_share", { request });
      setCacheEntries((current) => [
        entry,
        ...current.filter((candidate) => candidate.key !== entry.key && candidate.path !== entry.path),
      ]);
      const loaded = await props.onLoadCached(entry.path, entry.modes[0]?.name || preferredMode);
      if (loaded) await refreshLocalProfiles();
      return loaded;
    } catch (error) {
      props.onMessage(String(error));
      return false;
    } finally {
      setDownloadingShareKey(null);
    }
  };

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

  const beginShareProfileDrag = (event: DragEvent, fixture: GdtfShareFixtureSummary) => {
    const cached = cachedShareEntry(fixture);
    if (!cached) {
      event.preventDefault();
      return;
    }
    const mode = cached.modes[0] ?? fixture.modes[0] ?? { name: "", dmx_footprint: null };
    beginProfileDrag(event, {
      source: "cache",
      key: profileModeKey(cached.path, mode.name),
      label: `${fixture.manufacturer} ${fixture.fixture}`,
      modeName: mode.name,
      footprint: mode.dmx_footprint ?? 0,
      activate: () => props.onLoadCached(cached.path, mode.name || null),
    });
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

        <section class="patchProfileBrowserSection patchShareSection" data-patch-profile-section="share">
          <header>
            <strong>GDTF Share</strong>
            <span>{shareStateKind() === "results" ? `${shareResponse().fixtures.length}/${shareResponse().total_matches}` : "—"}</span>
          </header>

          <Show when={shareStateKind() === "signed-out"}>
            <div class="patchShareGuidance" data-patch-share-state="signed-out">
              <span>Sign in to GDTF Share here, or open Library.</span>
              <div class="patchShareCredentialControls">
                <input
                  value={props.shareUser}
                  autocomplete="off"
                  aria-label="Share User"
                  placeholder="Share User"
                  onInput={(event) => props.onShareUser(event.currentTarget.value)}
                />
                <input
                  type="password"
                  value={props.sharePassword}
                  autocomplete="off"
                  aria-label="Share Password"
                  placeholder="Share Password"
                  onInput={(event) => props.onSharePassword(event.currentTarget.value)}
                />
                <button type="button" data-patch-share-open-library onClick={props.onOpenLibrary}>Open Library</button>
              </div>
              <small>Credentials stay in session memory only.</small>
            </div>
          </Show>

          <Show when={shareStateKind() === "idle"}>
            <p class="empty patchProfileRowEmpty" data-patch-share-state="idle">
              Type 2 or more characters to search GDTF Share.
            </p>
          </Show>

          <Show when={shareStateKind() === "searching"}>
            <p class="empty patchProfileRowEmpty" data-patch-share-state="searching" aria-live="polite">
              Searching GDTF Share…
            </p>
          </Show>

          <Show when={shareStateKind() === "empty"}>
            <p class="empty patchProfileRowEmpty" data-patch-share-state="empty">
              No GDTF Share profiles match.
            </p>
          </Show>

          <Show when={shareStateKind() === "offline"}>
            <p class="empty patchProfileRowEmpty patchShareUnavailable" data-patch-share-state="offline">
              GDTF Share is offline or unavailable.
            </p>
          </Show>

          <Show when={shareError()}>
            {(error) => (
              <div
                class="patchShareErrorPanel"
                data-patch-share-state="error"
                data-patch-share-error-kind={error().category}
              >
                <div class="patchShareErrorRow" role="alert">
                  <span class="patchShareErrorText">
                    <Show when={error().category === "auth"} fallback={<span>GDTF Share error:</span>}>
                      <span>Login failed:</span>
                    </Show>{" "}
                    <span data-no-localize>{error().message}</span>
                  </span>
                  <Show when={error().category === "generic"}>
                    <button
                      type="button"
                      data-patch-share-retry
                      aria-label="Retry GDTF Share search"
                      onClick={() => retryShareSearch(error().searchQuery)}
                    >
                      Retry
                    </button>
                  </Show>
                </div>
                <Show when={error().category === "auth"}>
                  <div class="patchShareCredentialControls">
                    <input
                      value={props.shareUser}
                      autocomplete="off"
                      aria-label="Share User"
                      placeholder="Share User"
                      onInput={(event) => props.onShareUser(event.currentTarget.value)}
                    />
                    <input
                      type="password"
                      value={props.sharePassword}
                      autocomplete="off"
                      aria-label="Share Password"
                      placeholder="Share Password"
                      onInput={(event) => props.onSharePassword(event.currentTarget.value)}
                    />
                    <button
                      type="button"
                      data-patch-share-retry
                      disabled={!props.backendAvailable || !shareCredentialsReady()}
                      onClick={() => retryShareSearch(error().searchQuery)}
                    >
                      Retry
                    </button>
                  </div>
                </Show>
              </div>
            )}
          </Show>

          <Show when={shareStateKind() === "results"}>
            <div class="patchShareManufacturerGroups" data-patch-share-state="results">
              <For each={shareGroups()}>
                {(group) => (
                  <section class="patchShareManufacturerGroup" data-share-manufacturer={group.manufacturer}>
                    <header><strong data-no-localize>{group.manufacturer}</strong><span>{group.fixtures.length}</span></header>
                    <div class="patchProfileRows">
                      <For each={group.fixtures}>
                        {(entry) => {
                          const key = fixtureCatalogFavoriteKey(entry);
                          const cached = () => cachedShareEntry(entry);
                          const mode = () => cached()?.modes[0] ?? entry.modes[0] ?? { name: "", dmx_footprint: null };
                          const downloading = () => downloadingShareKey() === key;
                          return (
                            <button
                              type="button"
                              class="patchProfileRow patchShareProfileRow"
                              data-patch-profile-row
                              data-profile-source="share"
                              data-profile-cached={cached() ? "true" : "false"}
                              data-profile-footprint={mode().dmx_footprint ?? 0}
                              data-share-profile-key={key}
                              draggable={Boolean(cached())}
                              aria-pressed={Boolean(cached()) && selected(
                                cached()!.path,
                                entry.manufacturer,
                                entry.fixture,
                                mode().name,
                              )}
                              disabled={downloading()}
                              onClick={() => void downloadAndUseShareProfile(entry)}
                              onDragStart={(event) => beginShareProfileDrag(event, entry)}
                              onDragEnd={props.onProfileDragEnd}
                            >
                              <strong data-no-localize>{entry.manufacturer} · {entry.fixture}</strong>
                              <span>
                                <Show when={downloading()} fallback={<b data-no-localize>{entry.revision}</b>}>
                                  <i>Downloading…</i>
                                </Show>
                              </span>
                            </button>
                          );
                        }}
                      </For>
                    </div>
                  </section>
                )}
              </For>
            </div>
          </Show>
        </section>

        <ProfileImportSources {...props} folded />
      </div>
    </section>
  );
}
