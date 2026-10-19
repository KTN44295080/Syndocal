import assert from "node:assert/strict";
import { typescriptFixtureModuleUrl } from "./typescript-fixture-module.mjs";

const health = await import(await typescriptFixtureModuleUrl(new URL("../src/mediaHealthModel.ts", import.meta.url)));
const plan = await import(await typescriptFixtureModuleUrl(new URL("../src/mediaRelinkPlan.ts", import.meta.url)));

const hash = (hex) => ({ algorithm: "Sha256", hex });
const asset = (id, path, byteSize, kind = "File") => ({
  id,
  label: `Asset ${id}`,
  source: { kind, path },
  content_hash: hash(String(id).repeat(64).slice(0, 64)),
  byte_size: byteSize,
});

// --- State classification -------------------------------------------------
assert.equal(health.mediaHealthStateOf(undefined), "unverified", "an unchecked asset is unverified, not available");
assert.equal(health.mediaHealthStateOf({ kind: "available_unverified", asset_id: 1 }), "unverified");
assert.equal(health.mediaHealthStateOf({ kind: "available_verified", asset_id: 1 }), "available");
assert.equal(health.mediaHealthStateOf({ kind: "missing", asset_id: 1 }), "missing");
assert.equal(health.mediaHealthStateOf({ kind: "unreadable", asset_id: 1, error: "denied" }), "unreadable");
assert.equal(
  health.mediaHealthStateOf({ kind: "hash_mismatch", asset_id: 1, expected: hash("a"), actual: hash("b") }),
  "hash_mismatch",
);
assert.equal(health.mediaHealthStateOf({ kind: "live_source", asset_id: 1 }), "live");

const assets = [
  asset(1, "C:\\media\\a.mp4", 1024),
  asset(2, "/media/b.mov", 2048),
  asset(3, "/media/c.png", 10, "StillImage"),
  asset(4, "/media/d.mp4", 4_194_304),
  asset(5, "/media/e.mp4", 5),
  asset(6, "/media/f.mp4", null),
  { id: 7, label: "Camera", source: { kind: "Camera", path: null } },
  { id: 8, label: "Generator", source: { kind: { Generator: { Solid: { color: { r: 0, g: 0, b: 0, a: 1 } } } } } },
];
const availability = {
  1: { kind: "missing", asset_id: 1 },
  2: { kind: "unreadable", asset_id: 2, error: "Access is denied" },
  3: { kind: "hash_mismatch", asset_id: 3, expected: hash("e".repeat(64)), actual: hash("f".repeat(64)) },
  4: { kind: "available_verified", asset_id: 4 },
  5: { kind: "available_unverified", asset_id: 5 },
  7: { kind: "live_source", asset_id: 7 },
};
const rows = health.buildMediaHealthRows(assets, availability, { 4: 1_700_000_000_000 });
assert.deepEqual(rows.map((row) => row.assetId), [1, 2, 3, 4, 5, 6], "live and generated sources are not listed");
assert.equal(rows[5].state, "unverified", "an asset with no availability row is unverified");
assert.equal(rows[0].path, "C:\\media\\a.mp4");
assert.equal(rows[0].byteSize, 1024);
assert.equal(rows[0].checkedAtMs, null, "an unchecked row has no last-check time");
assert.equal(rows[3].checkedAtMs, 1_700_000_000_000);
assert.equal(rows[1].detail, "Access is denied", "the engine reason is kept for an unreadable file");
assert.match(rows[2].detail, /Expected e{64}; found f{64}/, "both hashes are kept for a mismatch");
assert.equal(rows[5].byteSize, null, "an unrecorded size stays unknown");

const groups = health.groupMediaHealthRows(rows);
assert.deepEqual(
  groups.map((group) => group.state),
  ["missing", "unreadable", "hash_mismatch", "unverified", "available"],
  "groups follow operator priority and empty groups are dropped",
);
assert.deepEqual(groups.find((group) => group.state === "unverified").rows.map((row) => row.assetId), [5, 6]);

const summary = health.summarizeMediaHealth(assets, availability);
assert.deepEqual(summary.counts, { missing: 1, unreadable: 1, hash_mismatch: 1, unverified: 2, available: 1 });
assert.equal(summary.live, 2, "camera and generator assets are counted as live, not listed");
assert.equal(summary.attention, 3);
assert.equal(summary.total, 8);
assert.deepEqual(health.groupMediaHealthRows([]), []);

// --- Formatting -----------------------------------------------------------
assert.equal(health.formatMediaByteSize(null), "");
assert.equal(health.formatMediaByteSize(undefined), "");
assert.equal(health.formatMediaByteSize(-1), "");
assert.equal(health.formatMediaByteSize(0), "0 B");
assert.equal(health.formatMediaByteSize(1023), "1023 B");
assert.equal(health.formatMediaByteSize(1024), "1.0 KiB");
assert.equal(health.formatMediaByteSize(4_194_304), "4.0 MiB");
assert.equal(health.formatMediaByteSize(1_610_612_736), "1.5 GiB");
assert.equal(health.formatMediaByteSize(250 * 1024 * 1024), "250 MiB");
assert.equal(health.mediaPathBaseName("C:\\media\\sub\\Clip.MP4"), "Clip.MP4");
assert.equal(health.mediaPathBaseName("/media/sub/clip.mp4"), "clip.mp4");
assert.equal(health.mediaPathBaseName("clip.mp4"), "clip.mp4");
assert.equal(health.mediaPathBaseName("/media/sub/"), "sub");
assert.equal(health.mediaPathBaseName(""), "");

// --- Folder relink matching ----------------------------------------------
const file = (path, size) => ({ path, file_name: health.mediaPathBaseName(path), byte_size: size });
const scan = (files, truncated = false) => ({ directory: "/new", files, truncated });
const missing = [
  asset(10, "C:\\old\\Intro.MP4", 100),
  asset(11, "/old/loop.mov", 200),
  asset(12, "/old/dup.mp4", 300),
  asset(13, "/old/gone.mp4", 400),
  asset(14, "/old/nosize.mp4", null),
  asset(15, "", 500),
  asset(16, "/old/twins.mp4", 600),
];
const result = plan.planFolderRelink(missing, scan([
  file("/new/sub/intro.mp4", 100),
  file("/new/loop.mov", 999),
  file("/new/a/dup.mp4", 300),
  file("/new/b/dup.mp4", 777),
  file("/new/nosize.mp4", 5),
  file("/new/a/twins.mp4", 600),
  file("/new/b/twins.mp4", 600),
  file("/new/other.mp4", 1),
], true));
assert.equal(result.directory, "/new");
assert.equal(result.truncated, true, "a truncated scan is surfaced to the operator");
const byId = Object.fromEntries(result.proposals.map((proposal) => [proposal.assetId, proposal]));
assert.deepEqual(Object.keys(byId).map(Number).sort((a, b) => a - b), [10, 11, 12, 14]);
assert.equal(byId[10].replacementPath, "/new/sub/intro.mp4", "names match case-insensitively across path styles");
assert.equal(byId[10].confidence, "exact", "same name and same size is exact");
assert.equal(byId[11].confidence, "name_only", "a size difference is flagged, not hidden");
assert.equal(byId[11].replacementByteSize, 999);
assert.equal(byId[11].recordedByteSize, 200);
assert.equal(byId[12].replacementPath, "/new/a/dup.mp4", "size disambiguates two files with one name");
assert.equal(byId[12].confidence, "exact");
assert.equal(byId[14].confidence, "name_only", "an asset with no recorded size can only match by name");
assert.deepEqual(result.ambiguous.map((entry) => entry.assetId), [16], "two same-size candidates are never guessed");
assert.deepEqual(result.ambiguous[0].candidatePaths, ["/new/a/twins.mp4", "/new/b/twins.mp4"]);
assert.deepEqual(result.unmatched.map((entry) => entry.assetId), [13, 15], "no file and no recorded path stay unmatched");
assert.equal(
  result.proposals.length + result.ambiguous.length + result.unmatched.length,
  missing.length,
  "every missing asset lands in exactly one bucket",
);

const unsizedTwins = plan.planFolderRelink([asset(20, "/old/x.mp4", null)], scan([file("/n/a/x.mp4", 1), file("/n/b/x.mp4", 2)]));
assert.equal(unsizedTwins.proposals.length, 0, "an unsized asset with two name matches is ambiguous");
assert.equal(unsizedTwins.ambiguous.length, 1);
const noneSized = plan.planFolderRelink([asset(21, "/old/y.mp4", 50)], scan([file("/n/a/y.mp4", 1), file("/n/b/y.mp4", 2)]));
assert.equal(noneSized.proposals.length, 0, "several differently sized candidates are ambiguous, not picked");
assert.equal(noneSized.ambiguous[0].candidatePaths.length, 2);
const empty = plan.planFolderRelink([], scan([]));
assert.deepEqual([empty.proposals, empty.ambiguous, empty.unmatched], [[], [], []]);

// --- Apply summary --------------------------------------------------------
assert.equal(
  plan.mediaRelinkApplySummary({ relinked: [1, 2], differentContent: [{ assetId: 3, label: "c" }], failed: [], skipped: [] }),
  "Folder relink: 2 relinked, 1 have different content.",
);
assert.equal(
  plan.mediaRelinkApplySummary({ relinked: [], differentContent: [], failed: [{ assetId: 1, label: "a", message: "x" }], skipped: [4, 5] }),
  "Folder relink: 1 failed, 2 not attempted.",
);
assert.equal(
  plan.mediaRelinkApplySummary({ relinked: [], differentContent: [], failed: [], skipped: [] }),
  "Folder relink: nothing was applied.",
);

console.log("media health model contract ok: state classification, grouping, size formatting, and folder relink matching (exact, name-only, ambiguous, unmatched)");
