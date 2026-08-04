export interface GdtfShareModeSummary {
  name: string;
  dmx_footprint: number | null;
}

export interface GdtfShareFixtureSummary {
  rid: number | null;
  uuid: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
  uploader: string | null;
  rating: string | null;
  version: string | null;
  creator: string | null;
  filesize: number | null;
  release_status: string | null;
  tested_in_visualizer: boolean | null;
  tested_in_real_life: boolean | null;
  modes: GdtfShareModeSummary[];
}

export interface GdtfShareSearchResponse {
  fixtures: GdtfShareFixtureSummary[];
  facets: {
    manufacturers: string[];
    modes: string[];
    versions: string[];
  };
  filter_support: {
    release_status: boolean | null;
    tested_in_visualizer: boolean | null;
    tested_in_real_life: boolean | null;
  };
  total_matches: number;
}

export interface GdtfFixtureCacheEntry {
  key: string;
  rid: number | null;
  uuid: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
  path: string;
  filesize: number;
  health: "healthy" | "warnings" | "invalid";
  detail: string;
  warnings: string[];
  modes: GdtfShareModeSummary[];
}

export interface FixtureProfileHealthSummary {
  fixture_id: number;
  label: string;
  manufacturer: string;
  profile_name: string;
  mode_name: string;
  source_path: string;
  status: "healthy" | "warnings" | "embedded" | "fallback" | "missing";
  detail: string;
  repairable: boolean;
}

export interface VerifiedFixtureProfileSummary {
  id: string;
  manufacturer: string;
  name: string;
  mode_name: string;
  footprint: number;
  description: string;
}

export const previewVerifiedProfiles: VerifiedFixtureProfileSummary[] = [
  { id: "dimmer-1ch", manufacturer: "Syndocal Verified", name: "Generic Dimmer 1ch", mode_name: "Standard", footprint: 1, description: "Single-channel intensity fixture for dimmer packs and practicals." },
  { id: "rgb-par-4ch", manufacturer: "Syndocal Verified", name: "Generic RGB PAR 4ch", mode_name: "Standard", footprint: 4, description: "Dimmer plus additive red, green and blue channels." },
  { id: "rgbw-par-5ch", manufacturer: "Syndocal Verified", name: "Generic RGBW PAR 5ch", mode_name: "Standard", footprint: 5, description: "Dimmer plus additive red, green, blue and white channels." },
  { id: "moving-head-rgbw-10ch", manufacturer: "Syndocal Verified", name: "Generic Moving Head RGBW 10ch", mode_name: "Standard", footprint: 10, description: "16-bit pan/tilt, dimmer, calibrated 1-25 Hz strobe and RGBW channels." },
];

export interface GdtfShareSearchRequest {
  user: string;
  password: string;
  manufacturer: string | null;
  fixture: string | null;
  query: string | null;
  mode: string | null;
  min_footprint: number | null;
  max_footprint: number | null;
  release_only: boolean;
  tested_in_visualizer: boolean;
  tested_in_real_life: boolean;
  limit: number;
}

export interface GdtfShareDownloadRequest {
  user: string;
  password: string;
  rid: number | null;
  uuid: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
}

export const fixtureCatalogFavoritesStorageKey = "syndocal.fixtureCatalogFavorites.v1";

const normalizedIdentityPart = (value: string) => value.trim().toLocaleLowerCase();

export const fixtureCatalogFavoriteKey = (fixture: {
  rid?: number | null;
  uuid?: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
}) => fixture.rid != null
  ? `share:rid:${fixture.rid}`
  : fixture.uuid?.trim()
    ? `share:uuid:${normalizedIdentityPart(fixture.uuid)}`
    : `share:name:${normalizedIdentityPart(fixture.manufacturer)}:${normalizedIdentityPart(fixture.fixture)}:${normalizedIdentityPart(fixture.revision)}`;

export const verifiedFixtureFavoriteKey = (profileId: string) => `verified:${normalizedIdentityPart(profileId)}`;

export const fixtureCatalogFavoritesFromUnknown = (value: unknown) => {
  if (!Array.isArray(value)) return [] as string[];
  return [...new Set(value
    .filter((entry): entry is string => typeof entry === "string")
    .map((entry) => entry.trim())
    .filter(Boolean))]
    .slice(0, 256);
};

export const loadFixtureCatalogFavorites = () => {
  try {
    return fixtureCatalogFavoritesFromUnknown(JSON.parse(
      window.localStorage.getItem(fixtureCatalogFavoritesStorageKey) ?? "[]",
    ));
  } catch {
    return [];
  }
};

export const saveFixtureCatalogFavorites = (favorites: string[]) => {
  const normalized = fixtureCatalogFavoritesFromUnknown(favorites);
  try {
    window.localStorage.setItem(fixtureCatalogFavoritesStorageKey, JSON.stringify(normalized));
  } catch {
    // Hardened WebViews can block localStorage; session state still works.
  }
  return normalized;
};

export const toggledFixtureCatalogFavorites = (favorites: string[], key: string) => {
  const normalized = fixtureCatalogFavoritesFromUnknown(favorites);
  return normalized.includes(key)
    ? normalized.filter((candidate) => candidate !== key)
    : fixtureCatalogFavoritesFromUnknown([...normalized, key]);
};

export type FixtureFootprintBand = "any" | "1-4" | "5-16" | "17-32" | "33-512";

export const fixtureFootprintBandBounds = (band: FixtureFootprintBand) => {
  switch (band) {
    case "1-4": return { min: 1, max: 4 };
    case "5-16": return { min: 5, max: 16 };
    case "17-32": return { min: 17, max: 32 };
    case "33-512": return { min: 33, max: 512 };
    default: return { min: null, max: null };
  }
};

export const fixtureCatalogIdentityMatches = (
  left: Pick<GdtfShareFixtureSummary, "rid" | "uuid" | "manufacturer" | "fixture" | "revision">,
  right: Pick<GdtfFixtureCacheEntry, "rid" | "uuid" | "manufacturer" | "fixture" | "revision">,
) => fixtureCatalogFavoriteKey(left) === fixtureCatalogFavoriteKey(right);

export const fixtureCatalogHealthLabel = (status: string) => {
  switch (status) {
    case "healthy": return "Ready";
    case "warnings": return "Warnings";
    case "embedded": return "Embedded";
    case "fallback": return "Fallback";
    case "missing": return "Missing";
    case "invalid": return "Invalid";
    default: return "Unknown";
  }
};

export const fixtureCatalogSearchTextMatches = (
  candidate: { manufacturer: string; fixture: string; revision: string; modes: GdtfShareModeSummary[] },
  query: string,
) => {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return true;
  return [candidate.manufacturer, candidate.fixture, candidate.revision, ...candidate.modes.map((mode) => mode.name)]
    .some((value) => value.toLocaleLowerCase().includes(needle));
};
