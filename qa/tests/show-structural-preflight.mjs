import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import {
  preflightShowContract,
  preflightShowFile,
  SHOW_TITLE_CONTAINS,
} from "../../tools/show-structural-preflight.mjs";

const fixturePath = new URL("../specimens/show-structural-preflight.sdc", import.meta.url);
const fixtureText = await readFile(fixturePath, "utf8");
const fixture = JSON.parse(fixtureText);
const legacyFixture = structuredClone(fixture);
for (const timeline of [legacyFixture.snapshot.timeline, legacyFixture.snapshot.timeline_bank[0]]) {
  delete timeline.follow.destination_start_mode;
  timeline.follow.hold_first_destination_measure = true;
}
const legacyText = await readFile(new URL("../../samples/phase1-mini-show.sdc", import.meta.url), "utf8");
const cliPath = fileURLToPath(new URL("../../tools/show-structural-preflight.mjs", import.meta.url));

const check = (report, id) => report.checks.find((entry) => entry.id === id);
const assertBlocked = (report, id, pattern) => {
  const entry = check(report, id);
  assert.equal(entry?.status, "BLOCKED", `${id} must be BLOCKED`);
  if (pattern) assert.match(entry.detail, pattern, `${id} detail must explain the block`);
};
const runCli = (...args) => spawnSync(process.execPath, [cliPath, ...args], {
  encoding: "utf8",
});

const valid = preflightShowContract(fixture);
assert.equal(valid.status, "PASS", "the deterministic current-format fixture passes authored checks");
assert.equal(check(valid, "dj_mapping")?.status, "PASS");
assert.equal(check(valid, "explicit_ids")?.status, "PASS");
assert.equal(check(valid, "active_bank_projection")?.status, "PASS");
assert.equal(check(valid, "adjacent_bank_entries")?.status, "PASS");
assert.equal(check(valid, "follow")?.status, "PASS");
assert.equal(check(valid, "source_measure_transition")?.status, "PASS");
assert.equal(check(valid, "destination_first_measure")?.status, "PASS");
assert.equal(check(valid, "destination_pedal_wait")?.status, "PASS");
assert.equal(check(valid, "source_loop")?.status, "PASS");
assert.match(check(valid, "destination_pedal_wait")?.detail ?? "", /wait_for_pedal persists .*intent/);
assert.doesNotMatch(check(valid, "destination_pedal_wait")?.detail ?? "", /until explicit F13 release/);
assert.match(valid.limitations.join("\n"), /no media availability.*hardware.*network.*runtime-state claim/i);
assert.match(valid.limitations.join("\n"), /never falls back to snapshot\.clock or assumes 4\/4/i);

const fromFixture = await preflightShowFile(fileURLToPath(fixturePath));
assert.equal(fromFixture.status, "PASS", "the checked-in canonical fixture must pass the current Follow contract");
assert.equal(check(fromFixture, "follow")?.status, "PASS");
const legacyFixtureReport = preflightShowContract(legacyFixture);
assert.equal(legacyFixtureReport.status, "BLOCKED", "the independently constructed legacy fixture must not pass the current Follow contract");
assertBlocked(legacyFixtureReport, "follow", /destination_start_mode|hold_first_destination_measure/);

const legacy = preflightShowContract(legacyText);
assert.equal(legacy.status, "BLOCKED", "a real current-format legacy .sdc blocks missing show evidence");
assertBlocked(legacy, "dj_mapping", /dj_track_triggers|mapping/);
assertBlocked(legacy, "adjacent_bank_entries", /timeline_bank/);

{
  const missingTempo = structuredClone(fixture);
  delete missingTempo.snapshot.timeline_bank[0].tempo_meter_map;
  assertBlocked(preflightShowContract(missingTempo), "source_measure_transition", /measure\/tempo evidence is BLOCKED/);
}

{
  const missingDestinationTempo = structuredClone(fixture);
  delete missingDestinationTempo.snapshot.timeline_bank[1].tempo_meter_map;
  assertBlocked(preflightShowContract(missingDestinationTempo), "destination_first_measure", /measure\/tempo evidence is BLOCKED/);
  assertBlocked(preflightShowContract(missingDestinationTempo), "destination_pedal_wait", /measure\/tempo evidence is BLOCKED/);
}

{
  const wrongDestination = structuredClone(fixture);
  wrongDestination.snapshot.timeline_bank[0].follow.next_timeline_id = 99;
  assertBlocked(preflightShowContract(wrongDestination), "explicit_ids", /destination Timeline ID|next_timeline_id/);
  assertBlocked(preflightShowContract(wrongDestination), "adjacent_bank_entries", /not adjacent/);
  assertBlocked(preflightShowContract(wrongDestination), "follow", /destination Timeline/);
}

{
  const nonAdjacent = structuredClone(fixture);
  nonAdjacent.snapshot.timeline_bank.splice(1, 0, {
    id: 7,
    label: "Unrelated",
    events: [],
    automations: [],
    video_automations: [],
    tempo_meter_map_version: 1,
    tempo_meter_map: [{
      position_sixteenth_steps: 0,
      bpm: 120,
      numerator: 4,
      denominator: 4,
      interpolation: "Step",
    }],
    playing: false,
    position_ms: 0,
    duration_ms: 2000,
  });
  assertBlocked(preflightShowContract(nonAdjacent), "adjacent_bank_entries", /not adjacent/);
}

{
  const cut = structuredClone(fixture);
  cut.snapshot.timeline_bank[0].follow.video_kind = "Cut";
  assertBlocked(preflightShowContract(cut), "follow", /non-Cut/);
  assertBlocked(preflightShowContract(cut), "source_measure_transition", /non-Cut/);

  const rustCaseVariant = structuredClone(fixture);
  rustCaseVariant.snapshot.timeline_bank[0].follow.lighting_policy = "HoldThenCut";
  assertBlocked(preflightShowContract(rustCaseVariant), "follow", /Rust serde value/);
  assertBlocked(preflightShowContract(rustCaseVariant), "source_measure_transition", /Rust serde value/);

  const unknownLightingPolicy = structuredClone(fixture);
  unknownLightingPolicy.snapshot.timeline_bank[0].follow.lighting_policy = "crossfade_everything";
  assertBlocked(preflightShowContract(unknownLightingPolicy), "follow", /lighting_policy/);

  const unknownVideoKind = structuredClone(fixture);
  unknownVideoKind.snapshot.timeline_bank[0].follow.video_kind = "CrossfadeEverything";
  assertBlocked(preflightShowContract(unknownVideoKind), "follow", /exact Rust serde value/);
  assertBlocked(preflightShowContract(unknownVideoKind), "source_measure_transition", /exact Rust serde value/);

  const unknownCurve = structuredClone(fixture);
  unknownCurve.snapshot.timeline_bank[0].follow.curve = "NotARealCurve";
  assertBlocked(preflightShowContract(unknownCurve), "follow", /curve.*exact Rust serde value/);

  const synchronizedUnknownCurve = structuredClone(fixture);
  synchronizedUnknownCurve.snapshot.timeline.follow.curve = "linear";
  synchronizedUnknownCurve.snapshot.timeline_bank[0].follow.curve = "linear";
  const synchronizedUnknownCurveReport = preflightShowContract(synchronizedUnknownCurve);
  assert.equal(synchronizedUnknownCurveReport.status, "BLOCKED", "an invalid curve must block even when active and bank agree");
  assertBlocked(synchronizedUnknownCurveReport, "sdc_shape", /curve.*exact Rust serde value/);

  const unknownDestinationVideoKind = structuredClone(fixture);
  unknownDestinationVideoKind.snapshot.timeline_bank[1].follow = {
    enabled: false,
    next_timeline_id: 3,
    duration: { unit: "Bars", value_milliunits: 1000 },
    video_kind: "CrossfadeEverything",
    hold_first_destination_measure: false,
  };
  assertBlocked(preflightShowContract(unknownDestinationVideoKind), "follow", /video_kind.*exact Rust serde value/);

  const rustCaseFaultVariant = structuredClone(fixture);
  rustCaseFaultVariant.snapshot.timeline_bank[0].follow.fault_policy = "Hold";
  assertBlocked(preflightShowContract(rustCaseFaultVariant), "follow", /Rust serde value/);
  assertBlocked(preflightShowContract(rustCaseFaultVariant), "source_measure_transition", /Rust serde value/);

  const unknownFaultPolicy = structuredClone(fixture);
  unknownFaultPolicy.snapshot.timeline_bank[0].follow.fault_policy = "retry";
  assertBlocked(preflightShowContract(unknownFaultPolicy), "follow", /fault_policy/);

  const unknownDestinationStartMode = structuredClone(fixture);
  unknownDestinationStartMode.snapshot.timeline_bank[0].follow.destination_start_mode = "auto_start";
  assertBlocked(preflightShowContract(unknownDestinationStartMode), "follow", /destination_start_mode.*exact Rust serde value/);

  const invalidActiveTimelineEnum = structuredClone(fixture);
  invalidActiveTimelineEnum.snapshot.timeline.follow.lighting_policy = "HoldThenCut";
  assertBlocked(preflightShowContract(invalidActiveTimelineEnum), "sdc_shape", /Rust serde value/);
}

{
  const unknownSelectorField = structuredClone(fixture);
  unknownSelectorField.dj_track_triggers[0].selector.unexpected = true;
  assertBlocked(preflightShowContract(unknownSelectorField), "dj_mapping", /unknown field.*unexpected/);

  const unknownMappingField = structuredClone(fixture);
  unknownMappingField.dj_track_triggers[0].unexpected = true;
  assertBlocked(preflightShowContract(unknownMappingField), "dj_mapping", /unknown field.*unexpected/);

  const snakeSelectorAlias = structuredClone(fixture);
  snakeSelectorAlias.dj_track_triggers[0].selector.title_contains = SHOW_TITLE_CONTAINS;
  assertBlocked(preflightShowContract(snakeSelectorAlias), "dj_mapping", /unknown field.*title_contains/);

  const snakeMappingAlias = structuredClone(fixture);
  snakeMappingAlias.dj_track_triggers[0].timeline_id = 1;
  assertBlocked(preflightShowContract(snakeMappingAlias), "dj_mapping", /unknown field.*timeline_id/);
}

{
  const missingSourceDuration = structuredClone(fixture);
  delete missingSourceDuration.snapshot.timeline_bank[0].duration_ms;
  assertBlocked(preflightShowContract(missingSourceDuration), "source_measure_transition", /duration_ms.*finite and positive/);
  assertBlocked(preflightShowContract(missingSourceDuration), "source_loop", /duration_ms.*finite and positive/);

  const zeroSourceDuration = structuredClone(fixture);
  zeroSourceDuration.snapshot.timeline_bank[0].duration_ms = 0;
  assertBlocked(preflightShowContract(zeroSourceDuration), "source_measure_transition", /duration_ms.*finite and positive/);
}

{
  const activeBankConflict = structuredClone(fixture);
  activeBankConflict.snapshot.timeline_bank[0].label = "Conflicting authored label";
  assertBlocked(preflightShowContract(activeBankConflict), "active_bank_projection", /conflicts/);

  const runtimeOnlyDifference = structuredClone(fixture);
  runtimeOnlyDifference.snapshot.timeline.playing = true;
  runtimeOnlyDifference.snapshot.timeline.position_ms = 777;
  const runtimeOnlyReport = preflightShowContract(runtimeOnlyDifference);
  assert.equal(runtimeOnlyReport.status, "PASS", "runtime-only active transport fields do not conflict with authored bank projection");
  assert.equal(check(runtimeOnlyReport, "active_bank_projection")?.status, "PASS");
}

{
  const disabled = structuredClone(fixture);
  disabled.snapshot.timeline_bank[0].follow.enabled = false;
  assertBlocked(preflightShowContract(disabled), "follow", /enabled/);
}

{
  const extraFollow = structuredClone(fixture);
  extraFollow.snapshot.timeline_bank.push({
    id: 3,
    label: "Third",
    events: [],
    automations: [],
    video_automations: [],
    follow: {
      enabled: true,
      next_timeline_id: 2,
      duration: { unit: "Bars", value_milliunits: 1000 },
      video_kind: "Crossfade",
      hold_first_destination_measure: true,
    },
    tempo_meter_map_version: 1,
    tempo_meter_map: [{
      position_sixteenth_steps: 0,
      bpm: 120,
      numerator: 4,
      denominator: 4,
      interpolation: "Step",
    }],
    playing: false,
    position_ms: 0,
    duration_ms: 2000,
  });
  assertBlocked(preflightShowContract(extraFollow), "source_measure_transition", /exactly one/);
}

{
  const wrongBarCount = structuredClone(fixture);
  wrongBarCount.snapshot.timeline_bank[0].follow.duration.value_milliunits = 2000;
  assertBlocked(preflightShowContract(wrongBarCount), "source_measure_transition", /exactly one meter-aware bar/);
}

{
  const legacyHold = structuredClone(fixture);
  legacyHold.snapshot.timeline_bank[0].follow.hold_first_destination_measure = true;
  assertBlocked(preflightShowContract(legacyHold), "follow", /hold_first_destination_measure/);
  assertBlocked(preflightShowContract(legacyHold), "destination_pedal_wait", /hold_first_destination_measure/);

  const autoStart = structuredClone(fixture);
  autoStart.snapshot.timeline_bank[0].follow.destination_start_mode = "play";
  assertBlocked(preflightShowContract(autoStart), "follow", /destination_start_mode.*auto-start/);
  assertBlocked(preflightShowContract(autoStart), "destination_pedal_wait", /destination_start_mode.*auto-start/);

  const omittedDestinationStartMode = structuredClone(fixture);
  delete omittedDestinationStartMode.snapshot.timeline.follow.destination_start_mode;
  delete omittedDestinationStartMode.snapshot.timeline_bank[0].follow.destination_start_mode;
  assertBlocked(preflightShowContract(omittedDestinationStartMode), "follow", /destination_start_mode.*wait_for_pedal/);

  const omittedDestinationHold = structuredClone(fixture);
  delete omittedDestinationHold.snapshot.timeline.follow.hold_first_destination_measure;
  delete omittedDestinationHold.snapshot.timeline_bank[0].follow.hold_first_destination_measure;
  assertBlocked(preflightShowContract(omittedDestinationHold), "follow", /hold_first_destination_measure.*false/);
}

{
  const shortDestination = structuredClone(fixture);
  shortDestination.snapshot.timeline_bank[1].duration_ms = 1000;
  assertBlocked(preflightShowContract(shortDestination), "destination_first_measure", /finite and cover|complete destination/);
  assertBlocked(preflightShowContract(shortDestination), "destination_pedal_wait", /finite valid destination measure/);
}

{
  const emptyLoop = structuredClone(fixture);
  emptyLoop.snapshot.timeline_bank[0].loop_region.b_ms = emptyLoop.snapshot.timeline_bank[0].loop_region.a_ms;
  assertBlocked(preflightShowContract(emptyLoop), "source_loop", /non-empty/);
}

{
  const finiteLoop = structuredClone(fixture);
  finiteLoop.snapshot.timeline_bank[0].loop_region.repeatMode = "finite";
  assertBlocked(preflightShowContract(finiteLoop), "source_loop", /repeatMode.*indefinite/);
}

{
  const repeatCount = structuredClone(fixture);
  repeatCount.snapshot.timeline_bank[0].loop_region.loopTotalPasses = 8;
  assertBlocked(preflightShowContract(repeatCount), "source_loop", /finite repeat-count/);
}

{
  const duplicateProduction = structuredClone(fixture);
  duplicateProduction.dj_track_triggers.push(structuredClone(duplicateProduction.dj_track_triggers[0]));
  duplicateProduction.dj_track_triggers[1].id = "duplicate-production";
  assertBlocked(preflightShowContract(duplicateProduction), "dj_mapping", /expected exactly one|duplicate/);

  const duplicateFallback = structuredClone(fixture);
  duplicateFallback.dj_track_triggers.push({
    id: "second-fallback",
    selector: { titleContains: "別のフォールバック曲", fallbackDeck: 1 },
    timelineId: 2,
    retrigger: "once_per_play_session",
  });
  assertBlocked(preflightShowContract(duplicateFallback), "dj_mapping", /fallbackDeck=1.*exactly one|reserved/);

  const additionalTitleContains = structuredClone(fixture);
  additionalTitleContains.dj_track_triggers.push({
    id: "secondary-title-contains",
    selector: { titleContains: "別のタイトル" },
    timelineId: 2,
    retrigger: "once_per_play_session",
  });
  assertBlocked(preflightShowContract(additionalTitleContains), "dj_mapping", /only titleContains=.*additional selector|ambiguous/);

  const overlappingExactTitle = structuredClone(fixture);
  overlappingExactTitle.dj_track_triggers.push({
    id: "overlapping-exact-title",
    selector: { title: "人生オーバー Remix", artist: "別のアーティスト" },
    timelineId: 2,
    retrigger: "once_per_play_session",
  });
  assertBlocked(preflightShowContract(overlappingExactTitle), "dj_mapping", /exact title\+artist.*overlaps|ambiguous/);

  const nonFallbackAdditionalMapping = structuredClone(fixture);
  nonFallbackAdditionalMapping.dj_track_triggers.push({
    id: "secondary-title-match",
    selector: { title: "別のタイトル", artist: "別のアーティスト" },
    timelineId: 2,
    retrigger: "once_per_play_session",
  });
  const nonFallbackReport = preflightShowContract(nonFallbackAdditionalMapping);
  assert.equal(nonFallbackReport.status, "PASS", "a distinct non-fallback mapping may coexist with the production fallback");
  assert.equal(check(nonFallbackReport, "dj_mapping")?.status, "PASS");
}

{
  const wrongFallback = structuredClone(fixture);
  wrongFallback.dj_track_triggers[0].selector.fallbackDeck = 2;
  assertBlocked(preflightShowContract(wrongFallback), "dj_mapping", /fallbackDeck.*exactly 1|expected exactly one/);
}

{
  const malformed = preflightShowContract('{"snapshot":{"timeline":1},"snapshot":{}}');
  assert.equal(malformed.status, "BLOCKED");
  assert.match(check(malformed, "sdc_shape")?.detail ?? "", /strict JSON|duplicate object key/);
}

{
  const missingProduction = structuredClone(fixture);
  missingProduction.dj_track_triggers[0].selector.titleContains = "別の曲";
  assertBlocked(preflightShowContract(missingProduction), "dj_mapping", new RegExp(`exactly one titleContains=${SHOW_TITLE_CONTAINS}`));
}

{
  const missingTimelineTarget = structuredClone(fixture);
  missingTimelineTarget.dj_track_triggers[0].timelineId = 99;
  assertBlocked(preflightShowContract(missingTimelineTarget), "dj_mapping", /existing authored Timeline ID/);
}

{
  const tooManyMappings = structuredClone(fixture);
  tooManyMappings.dj_track_triggers = Array.from({ length: 129 }, (_, index) => ({
    id: `mapping-${index}`,
    selector: { titleContains: `track-${index}`, fallbackDeck: 1 },
    timelineId: 1,
    retrigger: "once_per_play_session",
  }));
  assertBlocked(preflightShowContract(tooManyMappings), "dj_mapping", /128/);
}

{
  const invalidCadence = structuredClone(fixture);
  invalidCadence.snapshot.timeline_bank[0].follow.trans_cadence_bars = 0;
  assertBlocked(preflightShowContract(invalidCadence), "follow", /cadence/);

  const overCadence = structuredClone(fixture);
  overCadence.snapshot.timeline_bank[0].follow.trans_cadence_bars = 257;
  assertBlocked(preflightShowContract(overCadence), "source_measure_transition", /cadence/);

  const missingCadence = structuredClone(fixture);
  delete missingCadence.snapshot.timeline_bank[0].follow.trans_cadence_bars;
  assertBlocked(preflightShowContract(missingCadence), "follow", /trans_cadence_bars.*authored/);

  const missingPreroll = structuredClone(fixture);
  delete missingPreroll.snapshot.timeline_bank[0].follow.preroll_ms;
  assertBlocked(preflightShowContract(missingPreroll), "follow", /preroll_ms.*authored/);

  const missingDestinationBpm = structuredClone(fixture);
  delete missingDestinationBpm.snapshot.timeline_bank[0].follow.destination_bpm;
  assertBlocked(preflightShowContract(missingDestinationBpm), "follow", /destination_bpm.*authored/);

  const invalidDestinationBpm = structuredClone(fixture);
  invalidDestinationBpm.snapshot.timeline_bank[0].follow.destination_bpm = 301;
  assertBlocked(preflightShowContract(invalidDestinationBpm), "source_measure_transition", /BPM/);

  const excessivePreroll = structuredClone(fixture);
  excessivePreroll.snapshot.timeline_bank[0].follow.preroll_ms = 600001;
  assertBlocked(preflightShowContract(excessivePreroll), "follow", /timing|preroll/i);
}

{
  const terminalLinear = structuredClone(fixture);
  terminalLinear.snapshot.timeline_bank[0].tempo_meter_map[0].interpolation = "Linear";
  assertBlocked(preflightShowContract(terminalLinear), "source_measure_transition", /final.*linear.*end point/i);

  const tooManyTempoPoints = structuredClone(fixture);
  tooManyTempoPoints.snapshot.timeline_bank[0].tempo_meter_map = Array.from({ length: 4097 }, (_, index) => ({
    position_sixteenth_steps: index,
    bpm: 170,
    numerator: 4,
    denominator: 4,
    interpolation: "Step",
  }));
  assertBlocked(preflightShowContract(tooManyTempoPoints), "source_measure_transition", /4096/);

  const wrongMeasureNumber = structuredClone(fixture);
  wrongMeasureNumber.snapshot.timeline_bank[0].tempo_meter_map.push({
    position_sixteenth_steps: 16,
    bpm: 170,
    numerator: 4,
    denominator: 4,
    interpolation: "Step",
    measure_number: 3,
  });
  assertBlocked(preflightShowContract(wrongMeasureNumber), "source_measure_transition", /measure number.*expected boundary/);
}

{
  const usage = runCli();
  assert.equal(usage.status, 2, "CLI usage without a project must exit 2");
  assert.match(usage.stderr, /Usage:/);

  const unknownFlag = runCli("--unknown", fileURLToPath(fixturePath));
  assert.equal(unknownFlag.status, 2, "CLI unknown flags must exit 2");
  assert.match(unknownFlag.stderr, /Usage:/);

  const missingFile = runCli(fileURLToPath(new URL("../specimens/missing-show.sdc", import.meta.url)));
  assert.equal(missingFile.status, 2, "CLI read errors must exit 2");
  assert.match(missingFile.stdout, /show structural preflight: BLOCKED/);
}

console.log("show structural preflight: authored two-stage, meter, loop, and Deck 1 mapping checks passed");
