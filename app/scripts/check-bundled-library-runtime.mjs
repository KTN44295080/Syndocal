import assert from "node:assert/strict";

const originalFetch = globalThis.fetch;
let fail = true;
let fetches = 0;
globalThis.fetch = async (url) => {
  fetches += 1;
  if (fail) throw new Error(`temporary asset failure: ${url}`);
  const source = String(url).includes("oflLibrary") ? "Open Fixture Library" : "QLC+ Fixture Library";
  return {
    ok: true,
    status: 200,
    json: async () => ({
      v: 1,
      source,
      sourceRevision: "0123456789abcdef",
      license: "test-license",
      copyright: "test",
      url: "https://example.invalid",
      priority: source.startsWith("Open") ? 2 : 1,
      fixtures: [{ m: source, n: "Fixture", c: "test", modes: [{ n: "Mode", a: ["Dimmer"] }] }],
    }),
  };
};

try {
  const library = await import("../src/bundledLibrary.ts");
  await assert.rejects(library.loadBundledLibraryFixtures(), /temporary asset failure/);
  const failedFetches = fetches;
  fail = false;
  const fixtures = await library.loadBundledLibraryFixtures();
  const attributions = await library.loadBundledLibraryAttributions();
  assert.equal(fixtures.length, 2);
  assert.equal(attributions.length, 2);
  assert.ok(fetches > failedFetches, "a rejected first load must perform a fresh asset fetch on retry");
  assert.ok(library.bundledLibraryProfileRequest(fixtures[0].modes[0].key));
} finally {
  globalThis.fetch = originalFetch;
}

console.log("bundled fixture library failure/retry checks passed");
