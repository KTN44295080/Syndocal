import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import {
  fixtureCatalogFavoriteKey,
  fixtureCatalogIdentityMatches,
  previewVerifiedProfiles,
  type GdtfFixtureCacheEntry,
  type GdtfShareDownloadRequest,
  type GdtfShareFixtureSummary,
  type GdtfShareSearchRequest,
  type GdtfShareSearchResponse,
  type VerifiedFixtureProfileSummary,
} from "../fixtureCatalog";
import type { FixtureProfileSummary, PatchedFixtureSummary } from "../types";
import {
  filterGdtfProfileTreeFixtures,
  GdtfProfileTree,
  type GdtfProfileTreeFixture,
  type GdtfProfileTreeMode,
} from "./GdtfProfileTree";
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

const profileTreeFixture = (
  entry: Pick<GdtfFixtureCacheEntry, "manufacturer" | "fixture" | "revision" | "modes">,
  key: string,
): GdtfProfileTreeFixture => ({
  key,
  manufacturer: entry.manufacturer,
  fixture: entry.fixture,
  revision: entry.revision,
  modeCount: Math.max(1, entry.modes.length),
  modes: (entry.modes.length > 0 ? entry.modes : [{ name: "", dmx_footprint: null }]).map((mode) => ({
    key: profileModeKey(key, mode.name || "Default"),
    name: mode.name || "Default",
    modeName: mode.name || null,
    footprint: mode.dmx_footprint ?? 0,
  })),
});

const verifiedProfileTreeFixtures = (
  profiles: VerifiedFixtureProfileSummary[],
): GdtfProfileTreeFixture[] => {
  const fixtures = new Map<string, GdtfProfileTreeFixture>();
  for (const profile of profiles) {
    const key = `${profile.category}::${normalized(profile.fixture_family)}`;
    const fixture = fixtures.get(key) ?? {
      key,
      manufacturer: profile.category_name,
      manufacturerDescription: profile.category_description,
      fixture: profile.fixture_family,
      revision: "",
      modeCount: 0,
      modes: [],
    };
    fixture.modes.push({
      key: profile.id,
      name: profile.mode_name,
      modeName: profile.mode_name,
      footprint: profile.footprint,
      description: profile.description,
    });
    fixture.modeCount = fixture.modes.length;
    fixtures.set(key, fixture);
  }
  return [...fixtures.values()];
};

const textMatches = (values: string[], query: string) => {
  const needle = normalized(query);
  return !needle || values.some((value) => normalized(value).includes(needle));
};

export function PatchProfileBrowserPanel(props: PatchProfileBrowserPanelProps) {
  const [query, setQuery] = createSignal("");
  const [cacheEntries, setCacheEntries] = createSignal<GdtfFixtureCacheEntry[]>([]);
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
      const cache = await tauriInvoke<GdtfFixtureCacheEntry[]>("list_gdtf_fixture_cache");
      setCacheEntries(cache);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(false);
    }
  };

  onMount(() => void refreshLocalProfiles());

  const shareCredentialsReady = () => Boolean(props.shareUser.trim() && props.sharePassword);

  const cachedShareEntry = (fixture: GdtfShareFixtureSummary) => cacheEntries().find((candidate) =>
    candidate.health !== "invalid" && fixtureCatalogIdentityMatches(fixture, candidate));

  const verifiedTreeFixtures = verifiedProfileTreeFixtures(previewVerifiedProfiles);
  const visibleVerifiedFixtures = createMemo(() =>
    filterGdtfProfileTreeFixtures(verifiedTreeFixtures, query()));
  const visibleVerifiedProfileCount = createMemo(() => visibleVerifiedFixtures()
    .reduce((total, fixture) => total + fixture.modes.length, 0));
  const verifiedEntryForMode = (mode: GdtfProfileTreeMode) =>
    previewVerifiedProfiles.find((entry) => entry.id === mode.key);
  const cacheTreeFixtures = createMemo(() => cacheEntries().map((entry) =>
    profileTreeFixture(entry, entry.path)));
  const visibleCacheFixtures = createMemo(() =>
    filterGdtfProfileTreeFixtures(cacheTreeFixtures(), query()));
  const shareTreeFixtures = createMemo(() => shareResponse().fixtures.map((entry) =>
    profileTreeFixture(entry, fixtureCatalogFavoriteKey(entry))));
  const visibleShareFixtures = createMemo(() =>
    filterGdtfProfileTreeFixtures(shareTreeFixtures(), query()));
  const cacheEntryForTreeFixture = (fixture: GdtfProfileTreeFixture) =>
    cacheEntries().find((entry) => entry.path === fixture.key);
  const shareEntryForTreeFixture = (fixture: GdtfProfileTreeFixture) =>
    shareResponse().fixtures.find((entry) => fixtureCatalogFavoriteKey(entry) === fixture.key);

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

  const downloadAndUseShareProfile = async (
    fixture: GdtfShareFixtureSummary,
    requestedModeName: string | null,
  ) => {
    const alreadyCached = cachedShareEntry(fixture);
    const preferredMode = alreadyCached?.modes.find((mode) => mode.name === requestedModeName)?.name ||
      requestedModeName || alreadyCached?.modes[0]?.name || fixture.modes[0]?.name || null;
    if (alreadyCached) return props.onLoadCached(alreadyCached.path, preferredMode);
    if (!props.backendAvailable || !shareCredentialsReady()) return false;

    const key = profileModeKey(fixtureCatalogFavoriteKey(fixture), preferredMode || "Default");
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
      const loaded = await props.onLoadCached(entry.path, preferredMode || entry.modes[0]?.name || null);
      if (loaded) await refreshLocalProfiles();
      return loaded;
    } catch (error) {
      props.onMessage(String(error));
      return false;
    } finally {
      setDownloadingShareKey(null);
    }
  };

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

  const beginShareProfileDrag = (
    event: DragEvent,
    fixture: GdtfShareFixtureSummary,
    requestedMode: GdtfProfileTreeMode,
  ) => {
    const cached = cachedShareEntry(fixture);
    if (!cached) {
      event.preventDefault();
      return;
    }
    const mode = cached.modes.find((candidate) => candidate.name === requestedMode.modeName) ??
      cached.modes[0] ?? fixture.modes[0] ?? { name: "", dmx_footprint: null };
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
        <span>Search profiles <small>Find by ch count</small></span>
        <input
          type="search"
          value={query()}
          placeholder="Fixture, manufacturer, mode, or channel count"
          onInput={(event) => setQuery(event.currentTarget.value)}
          data-patch-profile-search
          data-patch-footprint-filter={/^\d{1,3}$/.test(query().trim()) ? query().trim() : undefined}
        />
      </label>

      <div class="patchProfileBrowserScroll" data-patch-profile-browser-scroll>
        <section class="patchProfileBrowserSection" data-patch-profile-section="verified">
          <header><strong>Verified common-rig pack</strong><span>{visibleVerifiedProfileCount()}</span></header>
          <GdtfProfileTree
            ariaLabel="Verified generic profile tree"
            source="verified"
            fixtures={visibleVerifiedFixtures()}
            searchActive={Boolean(query().trim())}
            disabled={() => !props.backendAvailable || busy()}
            draggable={() => true}
            selected={(_fixture, mode) => {
              const entry = verifiedEntryForMode(mode);
              return Boolean(entry) && selected(
                entry!.id,
                entry!.manufacturer,
                entry!.name,
                entry!.mode_name,
              );
            }}
            onActivate={(_fixture, mode) => {
              const entry = verifiedEntryForMode(mode);
              if (entry) void props.onLoadVerified(entry.id, entry.mode_name);
            }}
            onDragStart={(event, _fixture, mode) => {
              const entry = verifiedEntryForMode(mode);
              if (!entry) {
                event.preventDefault();
                return;
              }
              beginProfileDrag(event, {
                source: "verified",
                key: entry.id,
                label: `${entry.manufacturer} ${entry.name}`,
                modeName: entry.mode_name,
                footprint: entry.footprint,
                activate: () => props.onLoadVerified(entry.id, entry.mode_name),
              });
            }}
            onDragEnd={props.onProfileDragEnd}
          />
          <Show when={visibleVerifiedProfileCount() === 0}>
            <p class="empty patchProfileRowEmpty">No matching profiles.</p>
          </Show>
        </section>

        <section class="patchProfileBrowserSection" data-patch-profile-section="cache">
          <header><strong>Cached / offline GDTF</strong><span>{visibleCacheFixtures().length}</span></header>
          <GdtfProfileTree
            ariaLabel="Cached GDTF profile tree"
            source="cache"
            fixtures={visibleCacheFixtures()}
            searchActive={Boolean(query().trim())}
            disabled={(fixture) => cacheEntryForTreeFixture(fixture)?.health === "invalid" || busy()}
            draggable={() => true}
            selected={(fixture, mode) => {
              const entry = cacheEntryForTreeFixture(fixture);
              return Boolean(entry) && selected(
                entry!.path,
                entry!.manufacturer,
                entry!.fixture,
                mode.modeName ?? "",
              );
            }}
            onActivate={(fixture, mode) => {
              const entry = cacheEntryForTreeFixture(fixture);
              if (entry) void props.onLoadCached(entry.path, mode.modeName);
            }}
            onDragStart={(event, fixture, mode) => {
              const entry = cacheEntryForTreeFixture(fixture);
              if (!entry) {
                event.preventDefault();
                return;
              }
              beginProfileDrag(event, {
                source: "cache",
                key: profileModeKey(entry.path, mode.modeName ?? ""),
                label: `${entry.manufacturer} ${entry.fixture}`,
                modeName: mode.modeName ?? "",
                footprint: mode.footprint,
                activate: () => props.onLoadCached(entry.path, mode.modeName),
              });
            }}
            onDragEnd={props.onProfileDragEnd}
          />
          <Show when={visibleCacheFixtures().length === 0}>
            <p class="empty patchProfileRowEmpty">No cached GDTF profiles.</p>
          </Show>
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
              <GdtfProfileTree
                ariaLabel="GDTF Share profile tree"
                source="share"
                fixtures={visibleShareFixtures()}
                searchActive={Boolean(query().trim())}
                disabled={(_fixture, mode) => downloadingShareKey() === mode.key}
                draggable={(fixture) => {
                  const entry = shareEntryForTreeFixture(fixture);
                  return Boolean(entry && cachedShareEntry(entry));
                }}
                selected={(fixture, mode) => {
                  const entry = shareEntryForTreeFixture(fixture);
                  const cached = entry ? cachedShareEntry(entry) : undefined;
                  return Boolean(cached) && selected(
                    cached!.path,
                    fixture.manufacturer,
                    fixture.fixture,
                    mode.modeName ?? "",
                  );
                }}
                downloading={(_fixture, mode) => downloadingShareKey() === mode.key}
                cached={(fixture) => {
                  const entry = shareEntryForTreeFixture(fixture);
                  return Boolean(entry && cachedShareEntry(entry));
                }}
                onActivate={(fixture, mode) => {
                  const entry = shareEntryForTreeFixture(fixture);
                  if (entry) void downloadAndUseShareProfile(entry, mode.modeName);
                }}
                onDragStart={(event, fixture, mode) => {
                  const entry = shareEntryForTreeFixture(fixture);
                  if (entry) beginShareProfileDrag(event, entry, mode);
                  else event.preventDefault();
                }}
                onDragEnd={props.onProfileDragEnd}
              />
              <Show when={visibleShareFixtures().length === 0}>
                <p class="empty patchProfileRowEmpty">No GDTF Share profiles match.</p>
              </Show>
            </div>
          </Show>
        </section>

        <ProfileImportSources {...props} folded />
      </div>
    </section>
  );
}
