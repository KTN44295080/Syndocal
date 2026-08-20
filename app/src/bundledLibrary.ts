import type { CustomFixtureProfileRequest } from "./types";
import type { GdtfProfileTreeFixture, GdtfProfileTreeMode } from "./components/GdtfProfileTree";

/* Offline manufacturer profiles are split into independently lazy-loaded
   source bundles. OFL has the higher conversion priority; the QLC+ build step
   removes exact normalized manufacturer/model conflicts before shipping. */

interface BundledLibraryMode {
  n: string;
  a: string[];
}

interface BundledLibraryFixture {
  m: string;
  n: string;
  c: string;
  modes: BundledLibraryMode[];
}

interface BundledLibraryPayload {
  v: number;
  source: string;
  sourceRevision: string;
  license: string;
  copyright: string;
  url: string;
  priority: number;
  fixtures: BundledLibraryFixture[];
}

interface BundledLibrarySource {
  id: "ofl" | "qlc";
  shortName: "OFL" | "QLC+";
  load: () => Promise<BundledLibraryPayload>;
}

const loadBundledLibraryAsset = async (url: URL): Promise<BundledLibraryPayload> => {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Bundled fixture library could not be loaded (${response.status})`);
  }
  return response.json() as Promise<BundledLibraryPayload>;
};

export interface BundledLibraryAttribution {
  source: string;
  shortName: string;
  sourceRevision: string;
  license: string;
  copyright: string;
  url: string;
  fixtureCount: number;
  modeCount: number;
}

const sources: BundledLibrarySource[] = [
  {
    id: "ofl",
    shortName: "OFL",
    load: () => loadBundledLibraryAsset(new URL("./generated/oflLibrary.json", import.meta.url)),
  },
  {
    id: "qlc",
    shortName: "QLC+",
    load: () => loadBundledLibraryAsset(new URL("./generated/qlcLibrary.json", import.meta.url)),
  },
];

const footprintOf = (attributes: readonly string[]) =>
  attributes.reduce((total, attribute) => total + (attribute.endsWith(":16") ? 2 : 1), 0);

export const bundledLibraryProfileKey = (
  manufacturer: string,
  fixture: string,
  modeName: string,
  source = "ofl",
) => `bundled:${source}:${manufacturer}\u0000${fixture}\u0000${modeName}`;

let payloadPromise: Promise<Array<{ source: BundledLibrarySource; payload: BundledLibraryPayload }>> | null = null;
let cachedFixtures: GdtfProfileTreeFixture[] | null = null;
const requestsByKey = new Map<string, CustomFixtureProfileRequest>();

const loadPayloads = () => {
  payloadPromise ??= Promise.all(sources.map(async (source) => ({
    source,
    payload: await source.load(),
  })))
    .then((entries) => entries.sort((left, right) => right.payload.priority - left.payload.priority))
    .catch((error) => {
      // A transient asset-protocol/read failure must not poison every later
      // attempt for the lifetime of the application.
      payloadPromise = null;
      throw error;
    });
  return payloadPromise;
};

/** Loads once and returns all offline sources through the existing tree. */
export const loadBundledLibraryFixtures = async (): Promise<GdtfProfileTreeFixture[]> => {
  if (cachedFixtures) return cachedFixtures;
  const payloads = await loadPayloads();
  const fixtures = payloads.flatMap(({ source, payload }) => payload.fixtures.map((entry) => {
    const modes: GdtfProfileTreeMode[] = entry.modes.map((mode) => {
      const key = bundledLibraryProfileKey(entry.m, entry.n, mode.n, source.id);
      requestsByKey.set(key, {
        manufacturer: entry.m,
        name: entry.n,
        mode_name: mode.n,
        attributes: mode.a,
      });
      return {
        key,
        name: mode.n,
        modeName: mode.n,
        footprint: footprintOf(mode.a),
        description: `${source.shortName} · ${entry.c}`,
      };
    });
    return {
      key: `bundled:${source.id}:${entry.m}\u0000${entry.n}`,
      manufacturer: entry.m,
      fixture: entry.n,
      revision: "",
      provenance: `${source.shortName} · ${entry.c}`,
      modeCount: modes.length,
      modes,
    } satisfies GdtfProfileTreeFixture;
  }));
  cachedFixtures = fixtures;
  return fixtures;
};

/** The profile request for a bundled mode key, once the library is loaded. */
export const bundledLibraryProfileRequest = (key: string): CustomFixtureProfileRequest | null =>
  requestsByKey.get(key) ?? null;

export const loadBundledLibraryAttributions = async (): Promise<BundledLibraryAttribution[]> => {
  const payloads = await loadPayloads();
  return payloads.map(({ source, payload }) => ({
    source: payload.source,
    shortName: source.shortName,
    sourceRevision: payload.sourceRevision,
    license: payload.license,
    copyright: payload.copyright,
    url: payload.url,
    fixtureCount: payload.fixtures.length,
    modeCount: payload.fixtures.reduce((total, fixture) => total + fixture.modes.length, 0),
  }));
};
