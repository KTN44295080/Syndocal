import assert from "node:assert/strict";
import {
  DEFAULT_DJ_TITLE_CONTAINS,
  buildDjTrackSelector,
  djTrackSelectorMode,
  normalizeDjTrackTriggerMappings,
} from "../src/djTrackMappingPolicy.ts";

assert.equal(DEFAULT_DJ_TITLE_CONTAINS, "人生オーバー");
const productionSelector = buildDjTrackSelector({
  mode: "title_contains",
  contentId: "ignored-by-mode",
  title: "ignored-by-mode",
  artist: "ignored-by-mode",
  titleContains: " 人生オーバー ",
  fallbackDeck1: true,
});
assert.deepEqual(productionSelector, {
  ok: true,
  value: {
    contentId: null,
    title: null,
    artist: null,
    titleContains: "人生オーバー",
    fallbackDeck: 1,
  },
});
assert.equal(productionSelector.ok && djTrackSelectorMode(productionSelector.value), "title_contains");
assert.deepEqual(buildDjTrackSelector({
  mode: "title_contains",
  contentId: "",
  title: "",
  artist: "",
  titleContains: "人生オーバー",
  fallbackDeck1: false,
}), {
  ok: true,
  value: {
    contentId: null,
    title: null,
    artist: null,
    titleContains: "人生オーバー",
    fallbackDeck: null,
  },
});

const normalizedProductionMappings = normalizeDjTrackTriggerMappings([{
  id: " production-life ",
  selector: {
    titleContains: " 人生オーバー ",
    fallbackDeck: 1,
  },
  timelineId: 7,
  retrigger: "once_per_play_session",
}]);
assert.equal(normalizedProductionMappings.ok, true);
assert.deepEqual(normalizedProductionMappings.ok && normalizedProductionMappings.value[0], {
  id: "production-life",
  selector: {
    contentId: null,
    title: null,
    artist: null,
    titleContains: "人生オーバー",
    fallbackDeck: 1,
  },
  timelineId: 7,
  retrigger: "once_per_play_session",
});

const invalidMappings = [
  [{ id: "mixed", selector: { contentId: "x", titleContains: "人生オーバー" }, timelineId: 1, retrigger: "once_per_play_session" }],
  [{ id: "bad-fallback", selector: { titleContains: "人生オーバー", fallbackDeck: 2 }, timelineId: 1, retrigger: "once_per_play_session" }],
  [{ id: "orphan-fallback", selector: { contentId: "x", fallbackDeck: 1 }, timelineId: 1, retrigger: "once_per_play_session" }],
  [
    { id: "nfc", selector: { titleContains: "人生オーバー" }, timelineId: 1, retrigger: "once_per_play_session" },
    { id: "nfd", selector: { titleContains: "人生オーバー" }, timelineId: 2, retrigger: "once_per_play_session" },
  ],
  [{ id: "control", selector: { contentId: "bad\u0000id" }, timelineId: 1, retrigger: "once_per_play_session" }],
  [
    { id: " duplicate ", selector: { contentId: "a" }, timelineId: 1, retrigger: "once_per_play_session" },
    { id: "duplicate", selector: { contentId: "b" }, timelineId: 2, retrigger: "once_per_play_session" },
  ],
  Array.from({ length: 129 }, (_, index) => ({
    id: `mapping-${index}`,
    selector: { contentId: `content-${index}` },
    timelineId: 1,
    retrigger: "once_per_play_session",
  })),
];
for (const mappings of invalidMappings) {
  assert.equal(normalizeDjTrackTriggerMappings(mappings).ok, false);
}

console.log("DJ track mapping policy checks passed");
