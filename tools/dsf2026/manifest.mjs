import {
  assertEqual,
  assertNumber,
  assertSafeInteger,
  equalJson,
  exactKeys,
  fail,
  isObject,
  parseJsonValue,
  round9,
} from "./common.mjs";

export const CANONICAL_MANIFEST_PATH = "C:/TEMP/syndocal-show-audio-indefinite-loop/manifest.json";
export const CANONICAL_MANIFEST_SHA256 = "F5F940F55532C75905C8E738698E4D92DA7B65FC98519DA619B1262E8DE800EC";
export const SAMPLE_RATE = 48_000;
export const SOURCE_FRAMES = 10_536_286;
export const DESTINATION_FRAMES = 12_470_103;
export const SOURCE_DURATION_MS = Math.round((SOURCE_FRAMES * 1_000) / SAMPLE_RATE);
export const DESTINATION_DURATION_MS = Math.round((DESTINATION_FRAMES * 1_000) / SAMPLE_RATE);
export const TEMPO_METER_MAP_VERSION = 1;

const EXPECTED_TOP_LEVEL_KEYS = [
  "schema", "deterministicSource", "mode", "audioFormat", "clickSpec", "guideSpec",
  "mixSpec", "songs", "totals", "clickEvents", "guideEvents", "comparisonSnippets", "outputs",
];
const EXPECTED_CLICK_KEYS = ["song", "measure", "pass", "beat", "bpm", "localFrame", "strong", "frequencyHz", "globalFrame", "localSeconds", "globalSeconds"];
const EXPECTED_GUIDE_KEYS = [
  "song", "label", "kind", "semanticLabels", "measure", "pass", "beat", "localFrame", "announcesSong",
  "announcesMeasure", "announcesPass", "announcesBeat", "leadPerformanceBeats", "placement", "globalFrame",
  "announcesGlobalFrame", "mode", "localSeconds", "globalSeconds", "announcesLocalFrame", "announcesLocalSeconds",
  "announcesGlobalSeconds", "scheduledLeadFrames", "scheduledLeadSeconds", "scheduledLeadBeatsAtTargetBpm",
  "audibleOnsetFrame", "audibleOnsetGlobalFrame", "audibleOnsetLeadFrames", "audibleOnsetLeadSeconds",
  "audibleEndFrame", "audibleEndGlobalFrame", "audibleEndMarginFrames", "audibleEndMarginSeconds",
  "audibleActivityRule", "audibleActivityThresholdRms", "sourceAsset", "sourceFrames22050", "resampledFrames48000",
];
const ACTIVITY_RULE = "10ms forward-looking RMS window; active if RMS >= max(0.004, peakRms*0.02); interval owns every frame in active windows";
const GUIDE_ASSETS = Object.freeze({
  Intro: ["intro.wav", 23_250, 50_612, "d0e2f402e231da5ddad2427e306dcaf9b2a243a6ebcc2e33ea1a7347dfd4fa15", 0.213526265, 0.004270525, 3_456, 22_810, 2_897, 24_223],
  Verse: ["verse.wav", 22_808, 49_650, "d19cd381bb82c5086990e43b68537976001956112504c497af1c7089163e8f84", 0.159727915, 0.004, 3_709, 21_162, 2_897, 23_260],
  "Pre Chorus": ["pre_chorus.wav", 28_871, 62_848, "97e0a27f0467eef4cc809c93de7ad9da28f8d7fe6ef2a75f066a82273e8908ef", 0.217109496, 0.00434219, 4_167, 34_244, 2_697, 34_770],
  Chorus: ["chorus.wav", 26_448, 57_574, "3cf0f90ca4f06a7bb54acda1d153cd21cc929631ae033e5b19b883bf820eaa9c", 0.189070258, 0.004, 4_342, 29_381, 2_673, 31_186],
  Interlude: ["interlude.wav", 25_785, 56_131, "695e2addf83d1f3c236e14b57bf20124269b587a098293293a4ee9cab99eae21", 0.199367354, 0.004, 3_694, 27_267, 3_125, 28_263],
  Bridge: ["bridge.wav", 24_573, 53_492, "3f30a92ae768e7054c1ce88c6a1b870c8373958f7c10473d609df10ca3b902c2", 0.192294772, 0.004, 5_451, 24_757, 2_673, 27_094],
  Breakdown: ["breakdown.wav", 28_099, 61_168, "f8618ed41272e0802083dbf70308c4fc4de21f8c8e3155d0c6627391930a5981", 0.177822585, 0.004, 5_451, 33_318, 2_673, 34_787],
  Outro: ["outro.wav", 23_691, 51_572, "443fb54c20e2bd9e8f283ca522bc3584192000bb9bb5105a1af9131d48f31a2b", 0.160940351, 0.004, 3_423, 23_683, 2_897, 25_183],
  Looping: ["looping.wav", 25_013, 54_450, "c1a9cdf97339ad658b299329ac3a326fdb78d3edbe76281441036800c9455e39", 0.193180394, 0.004, 3_513, 26_138, 2_897, 28_063],
  Break: ["break_word.wav", 23_360, 50_852, "064051a6e3c1e077ac2d97073a7f927d8d004cf5a45195d13ef699a14bff49b0", 0.168857808, 0.004, 5_452, 21_615, 2_673, 24_473],
  Trans: ["trans.wav", 27_219, 59_252, "afa787e27bc34625c914e6b7e4fed9d027334db83980a4d6f5a8a4dcee444b69", 0.165872817, 0.004, 3_639, 30_783, 2_673, 32_863],
  Complete: ["complete.wav", 26_556, 57_809, "3298c2ca19fb8baa2c25c731d67702249c1e71ee00d599d6ffb3d27f55e1cc5c", 0.182798751, 0.004, 4_143, 29_351, 2_673, 31_363],
});
const SOURCE_GUIDE_CHART = [
  ["Intro", "section", 1, 1, "jinsei-over", 1, 0, "song-start-no-preroll"],
  ["Verse", "section", 17, 4, "jinsei-over", 18, 1, "previous-performance-beat"],
  ["Pre Chorus", "section", 25, 4, "jinsei-over", 26, 1, "previous-performance-beat"],
  ["Chorus", "section", 33, 4, "jinsei-over", 34, 1, "previous-performance-beat"],
  ["Interlude", "section", 49, 4, "jinsei-over", 50, 1, "previous-performance-beat"],
  ["Verse", "section", 57, 4, "jinsei-over", 58, 1, "previous-performance-beat"],
  ["Pre Chorus", "section", 65, 4, "jinsei-over", 66, 1, "previous-performance-beat"],
  ["Chorus", "section", 81, 4, "jinsei-over", 82, 1, "previous-performance-beat"],
  ["Looping", "operation", 97, 4, "jinsei-over", 98, 1, "previous-performance-beat"],
  ["Break", "operation", 98, 4, "jinsei-over", 99, 1, "previous-performance-beat"],
  ["Breakdown", "section", 113, 4, "jinsei-over", 114, 1, "previous-performance-beat"],
  ["Chorus", "section", 116, 4, "jinsei-over", 117, 1, "previous-performance-beat"],
  ["Outro", "section", 141, 4, "jinsei-over", 142, 1, "previous-performance-beat"],
  ["Trans", "transition", 148, 4, "jinsei-over", 149, 1, "previous-performance-beat"],
  ["Trans", "transition", 150, 4, "jinsei-over", 151, 1, "previous-performance-beat"],
  ["Trans", "transition", 152, 4, "jinsei-over", 153, 1, "previous-performance-beat"],
  ["Trans", "transition", 154, 4, "jinsei-over", 155, 1, "previous-performance-beat"],
  ["Complete", "transition", 156, 4, "madow-hoshi", 1, 1, "previous-connected-performance-beat"],
];
const DESTINATION_GUIDE_CHART = [
  ["Verse", "section", 18, 6, "madow-hoshi", 19, 1, "previous-performance-beat"],
  ["Pre Chorus", "section", 42, 4, "madow-hoshi", 43, 1, "previous-performance-beat"],
  ["Chorus", "section", 76, 4, "madow-hoshi", 77, 1, "previous-performance-beat"],
  ["Interlude", "section", 100, 4, "madow-hoshi", 101, 1, "previous-performance-beat"],
  ["Verse", "section", 124, 6, "madow-hoshi", 125, 1, "previous-performance-beat"],
  ["Pre Chorus", "section", 146, 4, "madow-hoshi", 147, 1, "previous-performance-beat"],
  ["Chorus", "section", 163, 4, "madow-hoshi", 164, 1, "previous-performance-beat"],
  ["Outro", "section", 191, 4, "madow-hoshi", 192, 1, "previous-performance-beat"],
];

function meterBeatCount(song, measure) {
  if (song === "jinsei-over") return 4;
  if ([18, 120, 124].includes(measure)) return 6;
  if ([117, 118, 119, 121, 122, 123].includes(measure)) return 5;
  return 4;
}

function expectedClickLocalFrame(song, measure, beat) {
  if (song === "jinsei-over") {
    const beatIndex = (measure - 1) * 4 + beat - 1;
    if (beatIndex <= 592) return Math.round((beatIndex * 60 * SAMPLE_RATE) / 170);
    const transitionBeat = beatIndex - 592;
    const seconds = (592 * 60) / 170 + ((60 * 32) / 24) * Math.log((170 + (24 * transitionBeat) / 32) / 170);
    return Math.round(seconds * SAMPLE_RATE);
  }
  let quarterBeats = beat - 1;
  for (let previous = 1; previous < measure; previous += 1) quarterBeats += meterBeatCount(song, previous);
  return Math.round((quarterBeats * 60 * SAMPLE_RATE) / 194);
}

function expectedClickBpm(song, measure, beat) {
  if (song === "madow-hoshi") return 194;
  const beatIndex = (measure - 1) * 4 + beat - 1;
  return beatIndex <= 592 ? 170 : 170 + (24 * (beatIndex - 592)) / 32;
}

function clickKey(song, measure, beat) {
  return `${song}:${measure}:${beat}`;
}

function validateClickEvents(manifest) {
  if (!Array.isArray(manifest.clickEvents) || manifest.clickEvents.length !== 1_464) fail("clickEvents must contain the canonical 624 + 840 event chart");
  const clicks = new Map();
  let index = 0;
  for (const song of ["jinsei-over", "madow-hoshi"]) {
    const measures = song === "jinsei-over" ? 156 : 207;
    for (let measure = 1; measure <= measures; measure += 1) {
      for (let beat = 1; beat <= meterBeatCount(song, measure); beat += 1) {
        const actual = manifest.clickEvents[index];
        exactKeys(actual, EXPECTED_CLICK_KEYS, `clickEvents[${index}]`);
        const localFrame = expectedClickLocalFrame(song, measure, beat);
        const globalFrame = song === "jinsei-over" ? localFrame : SOURCE_FRAMES + localFrame;
        const bpm = expectedClickBpm(song, measure, beat);
        const expected = { song, measure, pass: 1, beat, bpm, localFrame, strong: beat === 1, frequencyHz: beat === 1 ? 1320 : 920, globalFrame, localSeconds: round9(localFrame / SAMPLE_RATE), globalSeconds: round9(globalFrame / SAMPLE_RATE) };
        for (const key of EXPECTED_CLICK_KEYS) if (!Object.is(actual[key], expected[key])) fail(`clickEvents[${index}].${key} differs from the canonical chart`);
        const key = clickKey(song, measure, beat);
        if (clicks.has(key)) fail(`clickEvents contains duplicate chart point ${key}`);
        clicks.set(key, actual);
        index += 1;
      }
    }
  }
  if (index !== manifest.clickEvents.length) fail("clickEvents contains an unexpected trailing chart");
  return clicks;
}

function validateGuideAssets(manifest) {
  exactKeys(manifest.guideSpec.assets, Object.keys(GUIDE_ASSETS), "guideSpec.assets");
  for (const [label, expected] of Object.entries(GUIDE_ASSETS)) {
    const actual = manifest.guideSpec.assets[label];
    exactKeys(actual, ["file", "absolutePath", "sourceFrames", "resampledFrames", "sha256", "activity", "physical"], `guideSpec.assets.${label}`);
    if (actual.file !== expected[0] || actual.absolutePath !== `C:\\Users\\kouty\\Documents\\KDMX\\app\\src-tauri\\assets\\timeline-guide\\en\\${expected[0]}` || actual.sourceFrames !== expected[1] || actual.resampledFrames !== expected[2] || actual.sha256 !== expected[3]) fail(`guideSpec.assets.${label} identity differs from the canonical manifest`);
    exactKeys(actual.activity, ["rule", "windowFrames", "windowMs", "absoluteRmsFloor", "relativePeakRms", "peakRms", "thresholdRms", "firstActiveWindow", "lastActiveWindow", "firstActiveFrame", "lastActiveFrame"], `guideSpec.assets.${label}.activity`);
    if (!equalAssetActivity(actual.activity, expected)) fail(`guideSpec.assets.${label}.activity differs from the canonical manifest`);
    assertEqual(actual.physical, { firstNonZeroFrame: expected[8], lastNonZeroFrame: expected[9] }, `guideSpec.assets.${label}.physical`);
  }
}

function equalAssetActivity(actual, expected) {
  return actual.rule === ACTIVITY_RULE
    && actual.windowFrames === 480
    && actual.windowMs === 10
    && actual.absoluteRmsFloor === 0.004
    && actual.relativePeakRms === 0.02
    && actual.peakRms === expected[4]
    && actual.thresholdRms === expected[5]
    && actual.firstActiveWindow === expected[6]
    && actual.firstActiveFrame === expected[6]
    && actual.lastActiveWindow === expected[7] - 479
    && actual.lastActiveFrame === expected[7];
}

function validateGuideEvents(manifest, clicks) {
  if (!Array.isArray(manifest.guideEvents) || manifest.guideEvents.length !== 26) fail("guideEvents must contain the canonical 26-event chart");
  const expected = [...SOURCE_GUIDE_CHART, ...DESTINATION_GUIDE_CHART];
  for (const [index, actual] of manifest.guideEvents.entries()) {
    exactKeys(actual, EXPECTED_GUIDE_KEYS, `guideEvents[${index}]`);
    const [label, kind, measure, beat, announcesSong, announcesMeasure, leadPerformanceBeats, placement] = expected[index];
    const song = index < SOURCE_GUIDE_CHART.length ? "jinsei-over" : "madow-hoshi";
    const source = clicks.get(clickKey(song, measure, beat));
    const target = clicks.get(clickKey(announcesSong, announcesMeasure, 1));
    const asset = manifest.guideSpec.assets[label];
    if (!source || !target || !asset) fail(`guideEvents[${index}] references missing canonical chart data`);
    const globalFrame = source.globalFrame;
    const announcesGlobalFrame = target.globalFrame;
    const scheduledLeadFrames = announcesGlobalFrame - globalFrame;
    const expectedValues = {
      song, label, kind, semanticLabels: [label], measure, pass: 1, beat, localFrame: source.localFrame,
      announcesSong, announcesMeasure, announcesPass: 1, announcesBeat: 1, leadPerformanceBeats, placement,
      globalFrame, announcesGlobalFrame, mode: "default", localSeconds: round9(source.localFrame / SAMPLE_RATE),
      globalSeconds: round9(globalFrame / SAMPLE_RATE), announcesLocalFrame: target.localFrame,
      announcesLocalSeconds: round9(target.localFrame / SAMPLE_RATE), announcesGlobalSeconds: round9(announcesGlobalFrame / SAMPLE_RATE),
      scheduledLeadFrames, scheduledLeadSeconds: round9(scheduledLeadFrames / SAMPLE_RATE),
      scheduledLeadBeatsAtTargetBpm: round9((scheduledLeadFrames * target.bpm) / (SAMPLE_RATE * 60)),
      audibleOnsetFrame: asset.activity.firstActiveFrame, audibleOnsetGlobalFrame: globalFrame + asset.activity.firstActiveFrame,
      audibleOnsetLeadFrames: announcesGlobalFrame - globalFrame - asset.activity.firstActiveFrame,
      audibleOnsetLeadSeconds: round9((announcesGlobalFrame - globalFrame - asset.activity.firstActiveFrame) / SAMPLE_RATE),
      audibleEndFrame: asset.activity.lastActiveFrame, audibleEndGlobalFrame: globalFrame + asset.activity.lastActiveFrame,
      audibleEndMarginFrames: announcesGlobalFrame - globalFrame - asset.activity.lastActiveFrame,
      audibleEndMarginSeconds: round9((announcesGlobalFrame - globalFrame - asset.activity.lastActiveFrame) / SAMPLE_RATE),
      audibleActivityRule: ACTIVITY_RULE, audibleActivityThresholdRms: asset.activity.thresholdRms, sourceAsset: asset.file,
      sourceFrames22050: asset.sourceFrames, resampledFrames48000: asset.resampledFrames,
    };
    for (const key of EXPECTED_GUIDE_KEYS) if (!equalJson(actual[key], expectedValues[key])) fail(`guideEvents[${index}].${key} differs from the canonical chart`);
  }
}

function validateGuideSpec(manifest) {
  exactKeys(manifest.guideSpec, ["mode", "voice", "synthesisRate", "synthesisVolume", "pitchOrTimeShift", "sourceFormat", "resampler", "placement", "activityRule", "sectionEvidence", "assets"], "guideSpec");
  assertEqual(manifest.guideSpec.activityRule, { description: ACTIVITY_RULE, windowFrames: 480, windowMs: 10, absoluteRmsFloor: 0.004, relativePeakRms: 0.02, perceptualEndMarginFrames: 7200, perceptualEndMarginMs: 150 }, "guideSpec.activityRule");
  assertEqual(manifest.guideSpec.sectionEvidence, {
    source: "C:/Users/kouty/Documents/Guiter/app/trainer.tsx",
    life: "LIFE_OVER_SONG_MAP: Intro 1, Verse 18/58, Pre Chorus 26/66, Chorus 34/82/117, Interlude 50, Bridge 98, Breakdown 114, Outro 142, End/Trans 149",
    madow: "MADOW_SONG_MAP: Intro 1, Verse 19/125, Pre Chorus 43/147, Chorus 77/164, Interlude 101, Outro 192",
    productExceptions: ["Life Bridge 98 is silent because the runtime-repeated Looping entry owns the same required pre-entry beat.", "Madow Intro 1 is silent; Complete is the sole transition-boundary voice.", "Life End 149-156 uses Trans every two measures at 149/151/153/155."],
  }, "guideSpec.sectionEvidence");
  if (manifest.guideSpec.mode !== "default" || manifest.guideSpec.voice !== "Microsoft Zira Desktop" || manifest.guideSpec.synthesisRate !== 2 || manifest.guideSpec.synthesisVolume !== 100 || manifest.guideSpec.pitchOrTimeShift !== false || manifest.guideSpec.sourceFormat !== "PCM16 mono 22050 Hz" || manifest.guideSpec.resampler !== "deterministic linear interpolation 22050->48000; outputLength=round(inputFrames*48000/22050)" || manifest.guideSpec.placement !== "Section and operational announcements start on the exact previous performance click. Life Intro is the sole frame-zero/no-preroll exception. Madow Intro is intentionally silent and Complete starts on Life's final click.") fail("guideSpec synthesis/placement contract differs from the canonical manifest");
  validateGuideAssets(manifest);
}

export function validateCanonicalManifest(manifestInput) {
  const manifest = parseJsonValue(manifestInput, "manifest");
  exactKeys(manifest, EXPECTED_TOP_LEVEL_KEYS, "manifest");
  if (manifest.schema !== "syndocal-show-audio-export/v3" || manifest.deterministicSource !== "tools/audio/export-jinsei-madow-click-guide.mjs" || manifest.mode !== "default") fail("manifest schema/source/mode is not the canonical DSF2026 export");
  assertEqual(manifest.audioFormat, { codec: "pcm_s16le", sampleRate: SAMPLE_RATE, channels: 1, bitsPerSample: 16 }, "audioFormat");
  assertEqual(manifest.clickSpec, { waveform: "square", strongFrequencyHz: 1320, weakFrequencyHz: 920, durationMs: 50, startGain: 0.1, exponentialRampEndGain: 0.0001, exponentialRampMs: 45 }, "clickSpec");
  assertEqual(manifest.mixSpec, { clickGain: 1, guideGain: 0.85, rationale: "Click peak is <=0.1 and Guide events never overlap; 1.0*click + 0.85*Guide has a <=0.95 mathematical bound before PCM16 quantization.", clippingPolicy: "fail export if any float sample exceeds unity; no limiter or normalization" }, "mixSpec");
  validateGuideSpec(manifest);
  assertEqual(manifest.songs, {
    "jinsei-over": { authoredMeasures: 156, authoredBeats: 624, runtimeLoop: { startMeasure: 98, endMeasureExclusive: 99, repeatMode: "indefinite", releaseTrigger: "F13", automaticRelease: false }, performanceClicks: 624, timelineFrames: SOURCE_FRAMES, timelineDurationSeconds: 219.505958333, fixedTempo: { bpm: 170, measures: "1-148" }, transitionTempo: { measures: "149-156", beats: 32, startBpm: 170, endBoundaryBpm: 194, shape: "BPM(b)=170+24*b/32; time(b)=60*32/24*ln(BPM(b)/170)", startFrame: 10029176, lastClickFrame: 10521412, endBoundaryFrame: SOURCE_FRAMES } },
    "madow-hoshi": { measures: 207, bpm: 194, performanceClicks: 840, timelineFrames: DESTINATION_FRAMES, timelineDurationSeconds: 259.7938125, connectedStartFrame: SOURCE_FRAMES, connectedStartSeconds: 219.505958333, meterMap: { "18": 6, "117": 5, "118": 5, "119": 5, "120": 6, "121": 5, "122": 5, "123": 5, "124": 6 }, defaultBeatsPerMeasure: 4, startsWithCompleteTail: true, completePhysicalOnsetFrame: 10521412, completePhysicalFirstNonZeroFrame: 10524085, completePhysicalLastNonZeroFrame: 10552775, completeTargetFrame: SOURCE_FRAMES },
  }, "songs");
  assertEqual(manifest.totals, { clickCount: 1464, physicalGuideEventCount: 26, semanticGuideEventCount: 26, physicalGuideCounts: { Intro: 1, Verse: 4, "Pre Chorus": 4, Chorus: 5, Interlude: 2, Looping: 1, Break: 1, Breakdown: 1, Outro: 2, Trans: 4, Complete: 1 }, semanticGuideCounts: { Intro: 1, Verse: 4, "Pre Chorus": 4, Chorus: 5, Interlude: 2, Looping: 1, Break: 1, Breakdown: 1, Outro: 2, Trans: 4, Complete: 1 }, connectedFrames: 23006389, connectedDurationSeconds: 479.299770833 }, "totals");
  if (!Array.isArray(manifest.comparisonSnippets) || manifest.comparisonSnippets.length !== 0) fail("comparisonSnippets must be the canonical empty array");
  const clicks = validateClickEvents(manifest);
  validateGuideEvents(manifest, clicks);
  const outputRows = [
    ["jinsei-over-click.wav", "jinsei-over", "click", SOURCE_FRAMES, 219.505958333, 0.1, "3d8b1d590cbc3cab4d5bbf6bac6f8493e6739e2ca10436e458322c707fc250d8"],
    ["jinsei-over-guide.wav", "jinsei-over", "guide", SOURCE_FRAMES, 219.505958333, 0.596565437, "8889c56fa552857dcd761c42a844ea1376415a4cfa4f41571d2e90162c1ffc2e"],
    ["jinsei-over-mix.wav", "jinsei-over", "mix", SOURCE_FRAMES, 219.505958333, 0.507679156, "fb02cc9f19efe818b63eca0d245840eb9d9194bf441c61c00d066cba53888f59"],
    ["madow-hoshi-click.wav", "madow-hoshi", "click", DESTINATION_FRAMES, 259.7938125, 0.1, "d84515808ab461dcf531957893292c7f48ec5e9513d994f78dceab040f72fd1b"],
    ["madow-hoshi-guide.wav", "madow-hoshi", "guide", DESTINATION_FRAMES, 259.7938125, 0.596565437, "db19e76c4136973d416cb4c1f14426a257e64d418c01f217f1fa8e507ffb5486"],
    ["madow-hoshi-mix.wav", "madow-hoshi", "mix", DESTINATION_FRAMES, 259.7938125, 0.530348507, "c4b2d159c565b7f6097062ee5150d58475a35896cf4c79fd51a8590406e6ce15"],
    ["connected-click.wav", "connected", "click", 23006389, 479.299770833, 0.1, "42d0667cc36436caa41595fbe74fcffc8c4ae1db70bbaae4fb6a503012bc5617"],
    ["connected-guide.wav", "connected", "guide", 23006389, 479.299770833, 0.596565437, "a228195c85e50f5c0ff2a2eea0a35bd0912c5de103209fae1b78fa56f6b8cef0"],
    ["connected-mix.wav", "connected", "mix", 23006389, 479.299770833, 0.530348507, "01cea13ccdbaeba9ed23d55fcc90a70df9550a39e9fd45225a84dee7338bc33e"],
  ];
  if (!Array.isArray(manifest.outputs) || manifest.outputs.length !== outputRows.length) fail("outputs must contain the canonical nine PCM outputs");
  for (const [index, output] of manifest.outputs.entries()) {
    exactKeys(output, ["file", "absolutePath", "song", "kind", "frames", "durationSeconds", "peakFloat", "sha256"], `outputs[${index}]`);
    const [file, song, kind, frames, durationSeconds, peakFloat, sha256] = outputRows[index];
    if (output.file !== file || output.absolutePath !== `C:\\TEMP\\syndocal-show-audio-indefinite-loop\\${file}` || output.song !== song || output.kind !== kind || output.frames !== frames || output.durationSeconds !== durationSeconds || output.peakFloat !== peakFloat || output.sha256 !== sha256) fail(`outputs[${index}] differs from the canonical manifest`);
    assertSafeInteger(output.frames, `outputs[${index}].frames`);
    assertNumber(output.durationSeconds, `outputs[${index}].durationSeconds`);
    assertNumber(output.peakFloat, `outputs[${index}].peakFloat`);
  }
  return manifest;
}

export const validateManifest = validateCanonicalManifest;
