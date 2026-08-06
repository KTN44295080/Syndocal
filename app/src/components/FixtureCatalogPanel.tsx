import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { createMemo, createSignal, For, onMount, Show } from "solid-js";
import {
  fixtureCatalogCacheMegabytes,
  fixtureCatalogFavoriteKey,
  fixtureCatalogHealthLabel,
  fixtureCatalogIdentityMatches,
  fixtureCatalogSearchTextMatches,
  fixtureFootprintBandBounds,
  loadFixtureCatalogFavorites,
  previewVerifiedProfiles,
  saveFixtureCatalogFavorites,
  toggledFixtureCatalogFavorites,
  verifiedFixtureFavoriteKey,
  verifiedFixtureProfileRequest,
  type FixtureFootprintBand,
  type FixtureProfileHealthSummary,
  type GdtfFixtureCacheEntry,
  type GdtfShareDownloadRequest,
  type GdtfShareFixtureSummary,
  type GdtfShareSearchRequest,
  type GdtfShareSearchResponse,
  type VerifiedFixtureProfileSummary,
} from "../fixtureCatalog";
import type { FixtureProfileSummary } from "../types";

interface FixtureCatalogPanelProps {
  backendAvailable: boolean;
  shareUser: string;
  sharePassword: string;
  selectedFixtureId: number | null;
  selectedProfile: FixtureProfileSummary | null;
  selectedMode: string;
  onShareUser: (value: string) => void;
  onSharePassword: (value: string) => void;
  onProfileLoaded: (profile: FixtureProfileSummary, message: string, openPatch: boolean) => void;
  onRepair: (fixtureId: number, profilePath: string, modeName: string | null) => Promise<void>;
  onMessage: (message: string) => void;
}

const emptySearchResponse = (): GdtfShareSearchResponse => ({
  fixtures: [],
  facets: { manufacturers: [], modes: [], versions: [] },
  filter_support: {
    release_status: null,
    tested_in_visualizer: null,
    tested_in_real_life: null,
  },
  total_matches: 0,
});

const modeSummary = (modes: { name: string; dmx_footprint: number | null }[]) => modes.length === 0
  ? "Modes unavailable"
  : modes.slice(0, 3).map((mode) => `${mode.name}${mode.dmx_footprint ? ` ${mode.dmx_footprint}ch` : ""}`).join(" · ");

export function FixtureCatalogPanel(props: FixtureCatalogPanelProps) {
  const [manufacturer, setManufacturer] = createSignal("");
  const [fixtureQuery, setFixtureQuery] = createSignal("");
  const [globalQuery, setGlobalQuery] = createSignal("");
  const [mode, setMode] = createSignal("");
  const [footprintBand, setFootprintBand] = createSignal<FixtureFootprintBand>("any");
  const [releaseOnly, setReleaseOnly] = createSignal(false);
  const [testedVisualizer, setTestedVisualizer] = createSignal(false);
  const [testedReal, setTestedReal] = createSignal(false);
  const [favoritesOnly, setFavoritesOnly] = createSignal(false);
  const [favorites, setFavorites] = createSignal(loadFixtureCatalogFavorites());
  const [searchResponse, setSearchResponse] = createSignal(emptySearchResponse());
  const [cacheEntries, setCacheEntries] = createSignal<GdtfFixtureCacheEntry[]>([]);
  const [projectHealth, setProjectHealth] = createSignal<FixtureProfileHealthSummary[]>([]);
  const [busy, setBusy] = createSignal<"search" | "local" | "cache" | "load" | "repair" | null>(null);

  const favoriteSet = createMemo(() => new Set(favorites()));
  const visibleOnline = createMemo(() => searchResponse().fixtures.filter((fixture) =>
    !favoritesOnly() || favoriteSet().has(fixtureCatalogFavoriteKey(fixture))));
  const visibleCache = createMemo(() => cacheEntries().filter((entry) =>
    fixtureCatalogSearchTextMatches(entry, globalQuery())
      && (!favoritesOnly() || favoriteSet().has(fixtureCatalogFavoriteKey(entry)))));
  const visibleVerified = createMemo(() => previewVerifiedProfiles.filter((entry) =>
    (!globalQuery().trim()
      || `${entry.manufacturer} ${entry.name} ${entry.description}`.toLocaleLowerCase()
        .includes(globalQuery().trim().toLocaleLowerCase()))
      && (!favoritesOnly() || favoriteSet().has(verifiedFixtureFavoriteKey(entry.id)))));
  const selectedFixtureHealth = createMemo(() => projectHealth().find(
    (entry) => entry.fixture_id === props.selectedFixtureId,
  ) ?? null);
  const healthyCacheCount = createMemo(() => cacheEntries().filter(
    (entry) => entry.health === "healthy" || entry.health === "warnings",
  ).length);
  const cacheMegabytes = createMemo(() => fixtureCatalogCacheMegabytes(cacheEntries()));
  const repairCount = createMemo(() => projectHealth().filter((entry) => entry.repairable).length);

  const toggleFavorite = (key: string) => {
    const next = saveFixtureCatalogFavorites(toggledFixtureCatalogFavorites(favorites(), key));
    setFavorites(next);
  };

  const refreshLocalCatalog = async () => {
    if (!props.backendAvailable) return;
    setBusy("local");
    try {
      const [cache, health] = await Promise.all([
        tauriInvoke<GdtfFixtureCacheEntry[]>("list_gdtf_fixture_cache"),
        tauriInvoke<FixtureProfileHealthSummary[]>("get_fixture_profile_health"),
      ]);
      setCacheEntries(cache);
      setProjectHealth(health);
      props.onMessage(`Fixture catalog ready: ${cache.length} cached, ${health.length} patched, ${previewVerifiedProfiles.length} verified profiles.`);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(null);
    }
  };

  onMount(() => void refreshLocalCatalog());

  const searchShare = async () => {
    if (!props.backendAvailable) {
      props.onMessage("GDTF Share search requires the Syndocal desktop backend.");
      return;
    }
    const bounds = fixtureFootprintBandBounds(footprintBand());
    const request: GdtfShareSearchRequest = {
      user: props.shareUser,
      password: props.sharePassword,
      manufacturer: manufacturer().trim() || null,
      fixture: fixtureQuery().trim() || null,
      query: globalQuery().trim() || null,
      mode: mode().trim() || null,
      min_footprint: bounds.min,
      max_footprint: bounds.max,
      release_only: releaseOnly(),
      tested_in_visualizer: testedVisualizer(),
      tested_in_real_life: testedReal(),
      limit: 80,
    };
    setBusy("search");
    try {
      const response = await tauriInvoke<GdtfShareSearchResponse>("search_gdtf_share", { request });
      setSearchResponse(response);
      props.onMessage(`GDTF Share: showing ${response.fixtures.length} of ${response.total_matches} matching revisions.`);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(null);
    }
  };

  const loadCachedProfile = async (entry: GdtfFixtureCacheEntry, openPatch = true) => {
    if (entry.health === "invalid") {
      props.onMessage(`Cannot load invalid cached profile: ${entry.detail}`);
      return;
    }
    setBusy("load");
    try {
      const profile = await tauriInvoke<FixtureProfileSummary>("import_gdtf", { path: entry.path });
      props.onProfileLoaded(profile, `Loaded cached ${entry.manufacturer} ${entry.fixture}`, openPatch);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(null);
    }
  };

  const cacheAndLoad = async (fixture: GdtfShareFixtureSummary) => {
    const cachedEntry = cacheEntries().find((candidate) =>
      candidate.health !== "invalid" && fixtureCatalogIdentityMatches(fixture, candidate));
    if (cachedEntry) {
      await loadCachedProfile(cachedEntry, true);
      return;
    }
    const request: GdtfShareDownloadRequest = {
      user: props.shareUser,
      password: props.sharePassword,
      rid: fixture.rid,
      uuid: fixture.uuid,
      manufacturer: fixture.manufacturer,
      fixture: fixture.fixture,
      revision: fixture.revision,
    };
    setBusy("cache");
    try {
      const entry = await tauriInvoke<GdtfFixtureCacheEntry>("cache_gdtf_from_share", { request });
      await refreshLocalCatalog();
      await loadCachedProfile(entry, true);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(null);
    }
  };

  const loadVerifiedProfile = async (entry: VerifiedFixtureProfileSummary) => {
    if (!props.backendAvailable) {
      props.onMessage("Verified profiles require the Syndocal desktop backend.");
      return;
    }
    setBusy("load");
    try {
      const request = verifiedFixtureProfileRequest(entry.id);
      if (!request) throw new Error(`Verified fixture profile '${entry.id}' was not found`);
      const profile = await tauriInvoke<FixtureProfileSummary>("create_custom_fixture_profile", { request });
      props.onProfileLoaded(profile, `Loaded verified ${entry.name}`, true);
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(null);
    }
  };

  const repairSelectedFixture = async () => {
    const health = selectedFixtureHealth();
    const profile = props.selectedProfile;
    if (!health?.repairable || !profile) return;
    setBusy("repair");
    try {
      await props.onRepair(health.fixture_id, profile.source_path, props.selectedMode || null);
      await refreshLocalCatalog();
    } catch (error) {
      props.onMessage(String(error));
    } finally {
      setBusy(null);
    }
  };

  return (
    <section class="fixtureCatalogPanel">
      <header class="fixtureCatalogHeader">
        <div>
          <h3>Fixture Catalog</h3>
          <span>GDTF Share + Offline</span>
        </div>
        <div class="fixtureCatalogHealthSummary">
          <span class="healthy">{healthyCacheCount()} cached</span>
          <span class={repairCount() > 0 ? "warning" : "healthy"}>{repairCount()} repair</span>
          <button onClick={() => void refreshLocalCatalog()} disabled={!props.backendAvailable || busy() !== null}>Refresh</button>
        </div>
      </header>

      <div class="fixtureCatalogCredentials">
        <label>
          Share User
          <input value={props.shareUser} autocomplete="off" onInput={(event) => props.onShareUser(event.currentTarget.value)} />
        </label>
        <label>
          Share Password
          <input type="password" value={props.sharePassword} autocomplete="off" onInput={(event) => props.onSharePassword(event.currentTarget.value)} />
        </label>
        <small>Credentials stay in memory for this session and are never written to the project or cache metadata.</small>
      </div>

      <div class="fixtureCatalogFacets">
        <label>
          Search
          <input value={globalQuery()} onInput={(event) => setGlobalQuery(event.currentTarget.value)} placeholder="fixture, revision, mode" />
        </label>
        <label>
          Manufacturer
          <input list="gdtf-manufacturers" value={manufacturer()} onInput={(event) => setManufacturer(event.currentTarget.value)} />
          <datalist id="gdtf-manufacturers">
            <For each={searchResponse().facets.manufacturers}>{(entry) => <option value={entry} />}</For>
          </datalist>
        </label>
        <label>
          Fixture
          <input value={fixtureQuery()} onInput={(event) => setFixtureQuery(event.currentTarget.value)} />
        </label>
        <label>
          DMX Mode
          <input list="gdtf-modes" value={mode()} onInput={(event) => setMode(event.currentTarget.value)} />
          <datalist id="gdtf-modes">
            <For each={searchResponse().facets.modes}>{(entry) => <option value={entry} />}</For>
          </datalist>
        </label>
        <label>
          Footprint
          <select value={footprintBand()} onChange={(event) => setFootprintBand(event.currentTarget.value as FixtureFootprintBand)}>
            <option value="any">Any</option>
            <option value="1-4">1-4 ch</option>
            <option value="5-16">5-16 ch</option>
            <option value="17-32">17-32 ch</option>
            <option value="33-512">33+ ch</option>
          </select>
        </label>
      </div>
      <div class="fixtureCatalogFilterRow">
        <label><input type="checkbox" checked={releaseOnly()} onChange={(event) => setReleaseOnly(event.currentTarget.checked)} /> Releases</label>
        <label><input type="checkbox" checked={testedVisualizer()} onChange={(event) => setTestedVisualizer(event.currentTarget.checked)} /> Visualizer tested</label>
        <label><input type="checkbox" checked={testedReal()} onChange={(event) => setTestedReal(event.currentTarget.checked)} /> Real-life tested</label>
        <label><input type="checkbox" checked={favoritesOnly()} onChange={(event) => setFavoritesOnly(event.currentTarget.checked)} /> Favorites only</label>
        <button class="primary" onClick={() => void searchShare()} disabled={!props.backendAvailable || busy() !== null}>Search Share</button>
      </div>
      <Show when={searchResponse().filter_support.release_status === false
        || searchResponse().filter_support.tested_in_visualizer === false
        || searchResponse().filter_support.tested_in_real_life === false}>
        <small class="fixtureCatalogCapabilityNote">
          The public Share list did not provide release/test metadata. Those filters fail closed; clear them to include revisions with unknown status.
        </small>
      </Show>

      <div class="fixtureCatalogColumns">
        <section class="fixtureCatalogSection online">
          <header><strong>Online revisions</strong><span>{visibleOnline().length}/{searchResponse().total_matches}</span></header>
          <div class="fixtureCatalogList">
            <For each={visibleOnline()} fallback={<p class="empty">Search GDTF Share or adjust filters.</p>}>
              {(entry) => {
                const key = fixtureCatalogFavoriteKey(entry);
                const cached = () => cacheEntries().some((candidate) =>
                  candidate.health !== "invalid" && fixtureCatalogIdentityMatches(entry, candidate));
                return (
                  <article class="fixtureCatalogCard">
                    <button class="fixtureCatalogFavorite" aria-label="Favorite fixture profile" aria-pressed={favoriteSet().has(key)} onClick={() => toggleFavorite(key)}>{favoriteSet().has(key) ? "★" : "☆"}</button>
                    <strong data-no-localize>{entry.manufacturer} {entry.fixture}</strong>
                    <span data-no-localize>{entry.revision} · GDTF {entry.version ?? "?"}</span>
                    <small data-no-localize>{modeSummary(entry.modes)}</small>
                    <div class="fixtureCatalogBadges">
                      <Show when={entry.release_status}><i>{entry.release_status}</i></Show>
                      <Show when={entry.tested_in_visualizer === true}><i>Visualizer</i></Show>
                      <Show when={entry.tested_in_real_life === true}><i>Real life</i></Show>
                      <Show when={cached()}><i class="healthy">Cached</i></Show>
                    </div>
                    <button onClick={() => void cacheAndLoad(entry)} disabled={busy() !== null}>{cached() ? "Load Cache" : "Cache + Load"}</button>
                  </article>
                );
              }}
            </For>
          </div>
        </section>

        <section class="fixtureCatalogSection offline">
          <header>
            <strong>Offline cache</strong>
            <span class="fixtureCatalogCacheSummary">
              <b data-no-localize>{cacheEntries().length}</b>{" "}<span>profiles</span>{" · "}
              <b data-no-localize>{cacheMegabytes()}</b>{" "}<span data-no-localize>MB</span>
            </span>
          </header>
          <div class="fixtureCatalogList">
            <For each={visibleCache()} fallback={<p class="empty">No cached GDTF profiles.</p>}>
              {(entry) => {
                const key = fixtureCatalogFavoriteKey(entry);
                return (
                  <article class={`fixtureCatalogCard health-${entry.health}`}>
                    <button class="fixtureCatalogFavorite" aria-label="Favorite cached profile" aria-pressed={favoriteSet().has(key)} onClick={() => toggleFavorite(key)}>{favoriteSet().has(key) ? "★" : "☆"}</button>
                    <strong data-no-localize>{entry.manufacturer} {entry.fixture}</strong>
                    <span data-no-localize>{entry.revision}</span>
                    <small>{fixtureCatalogHealthLabel(entry.health)} · {entry.detail}</small>
                    <small data-no-localize>{modeSummary(entry.modes)}</small>
                    <button onClick={() => void loadCachedProfile(entry, true)} disabled={entry.health === "invalid" || busy() !== null}>Use Profile</button>
                  </article>
                );
              }}
            </For>
          </div>
        </section>
      </div>

      <section class="fixtureCatalogSection verified">
        <header><strong>Verified fixture packs</strong><span>{visibleVerified().length}</span></header>
        <p>University fixture modes are manual-verified, and personal fixture modes are matched to the supplied Daslight files. Confirm the fixture manual before using generic common-rig layouts.</p>
        <div class="fixtureVerifiedGrid">
          <For each={visibleVerified()}>
            {(entry) => {
              const key = verifiedFixtureFavoriteKey(entry.id);
              return (
                <article class="fixtureCatalogCard">
                  <button class="fixtureCatalogFavorite" aria-label="Favorite verified profile" aria-pressed={favoriteSet().has(key)} onClick={() => toggleFavorite(key)}>{favoriteSet().has(key) ? "★" : "☆"}</button>
                  <strong>{entry.name}</strong>
                  <span>{entry.footprint} ch · {entry.mode_name}</span>
                  <small>{entry.description}</small>
                  <button onClick={() => void loadVerifiedProfile(entry)} disabled={!props.backendAvailable || busy() !== null}>Use Profile</button>
                </article>
              );
            }}
          </For>
        </div>
      </section>

      <section class="fixtureCatalogSection projectHealth">
        <header><strong>Project profile health</strong><span>{projectHealth().length}</span></header>
        <div class="fixtureProjectHealthList">
          <For each={projectHealth()} fallback={<p class="empty">No patched fixtures.</p>}>
            {(entry) => (
              <article class={entry.fixture_id === props.selectedFixtureId ? "selected" : ""}>
                <span class={`healthBadge ${entry.status}`}>{fixtureCatalogHealthLabel(entry.status)}</span>
                <strong data-no-localize>{entry.label}</strong>
                <small data-no-localize>{entry.manufacturer} {entry.profile_name} · {entry.mode_name}</small>
                <span>{entry.detail}</span>
              </article>
            )}
          </For>
        </div>
        <Show when={selectedFixtureHealth()?.repairable}>
          <div class="fixtureRepairBar">
            <span>Selected fixture uses a fallback or missing source. Load the exact replacement profile and matching mode first.</span>
            <button class="primary" disabled={!props.selectedProfile || busy() !== null} onClick={() => void repairSelectedFixture()}>Repair Selected Fixture</button>
          </div>
        </Show>
      </section>
    </section>
  );
}
