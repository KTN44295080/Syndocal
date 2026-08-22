import { createHash } from "node:crypto";
import { access, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const [baselineArg, defaultCurrentArg, run1Arg, run2Arg] = process.argv.slice(2);
const BASELINE_DIR = resolve(baselineArg ?? "C:/TEMP/syndocal-show-audio");
const DEFAULT_CURRENT_DIR = resolve(defaultCurrentArg ?? "C:/TEMP/syndocal-show-audio-default-review");
const RUN1_DIR = resolve(run1Arg ?? "C:/TEMP/syndocal-show-audio-perceptual-review-1");
const RUN2_DIR = resolve(run2Arg ?? "C:/TEMP/syndocal-show-audio-perceptual-review-2");
const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "../..");
const EXPORTER = resolve(SCRIPT_DIR, "export-jinsei-madow-click-guide.mjs");
const GUIDE_DIR = resolve(REPO_ROOT, "app/src-tauri/assets/timeline-guide/en");
const SAMPLE_RATE = 48_000;
const SOURCE_RATE = 22_050;
const ACTIVITY_WINDOW_FRAMES = 480;
const ACTIVITY_ABSOLUTE_RMS_FLOOR = 0.004;
const ACTIVITY_RELATIVE_RMS = 0.02;
const END_MARGIN_FRAMES = 2_400;

const GUIDE_ASSETS = {
  Intro: "intro.wav",
  Verse: "verse.wav",
  "Pre Chorus": "pre_chorus.wav",
  Chorus: "chorus.wav",
  Interlude: "interlude.wav",
  Bridge: "bridge.wav",
  Breakdown: "breakdown.wav",
  Outro: "outro.wav",
  Looping: "looping.wav",
  Break: "break_word.wav",
  Trans: "trans.wav",
  Complete: "complete.wav",
};

function assert(condition, message) {
  if (!condition) throw new Error(`Perceptual checker failed: ${message}`);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function readManifest(directory) {
  return JSON.parse(await readFile(resolve(directory, "manifest.json"), "utf8"));
}

function parsePcm16MonoWav(bytes, expectedSampleRate) {
  assert(bytes.length >= 44, "WAV is at least 44 bytes");
  assert(bytes.toString("ascii", 0, 4) === "RIFF", "WAV has RIFF header");
  assert(bytes.toString("ascii", 8, 12) === "WAVE", "WAV has WAVE header");
  let fmt;
  let data;
  for (let offset = 12; offset + 8 <= bytes.length;) {
    const id = bytes.toString("ascii", offset, offset + 4);
    const size = bytes.readUInt32LE(offset + 4);
    const body = offset + 8;
    assert(body + size <= bytes.length, `WAV ${id} chunk is in bounds`);
    if (id === "fmt ") {
      assert(size >= 16, "WAV fmt chunk is complete");
      fmt = {
        audioFormat: bytes.readUInt16LE(body),
        channels: bytes.readUInt16LE(body + 2),
        sampleRate: bytes.readUInt32LE(body + 4),
        bitsPerSample: bytes.readUInt16LE(body + 14),
      };
    } else if (id === "data") {
      data = bytes.subarray(body, body + size);
    }
    offset = body + size + (size & 1);
  }
  assert(fmt && data, "WAV has fmt and data chunks");
  assert(fmt.audioFormat === 1, "WAV is integer PCM");
  assert(fmt.channels === 1, "WAV is mono");
  assert(fmt.sampleRate === expectedSampleRate, `WAV is ${expectedSampleRate} Hz`);
  assert(fmt.bitsPerSample === 16, "WAV is PCM16");
  assert(data.length % 2 === 0, "WAV has complete PCM16 frames");
  return { data, frames: data.length / 2, bytes };
}

function decodePcmFrames(wav, startFrame = 0, endFrame = wav.frames) {
  assert(startFrame >= 0 && endFrame >= startFrame && endFrame <= wav.frames, "PCM decode range is in bounds");
  const samples = new Float64Array(endFrame - startFrame);
  for (let index = 0; index < samples.length; index += 1) {
    samples[index] = wav.data.readInt16LE((startFrame + index) * 2) / 32768;
  }
  return samples;
}

function peakCode(wav) {
  let peak = 0;
  for (let frame = 0; frame < wav.frames; frame += 1) {
    peak = Math.max(peak, Math.abs(wav.data.readInt16LE(frame * 2)));
  }
  return peak;
}

function quantizePcm16Sample(sample) {
  return Math.max(-32768, Math.min(32767, Math.round(sample * 32767)));
}

function resampleLinearDeterministic(samples, sourceRate, destinationRate) {
  const outputLength = Math.round(samples.length * destinationRate / sourceRate);
  const output = new Float64Array(outputLength);
  for (let index = 0; index < outputLength; index += 1) {
    const sourcePosition = index * sourceRate / destinationRate;
    const left = Math.min(samples.length - 1, Math.floor(sourcePosition));
    const right = Math.min(samples.length - 1, left + 1);
    const fraction = sourcePosition - left;
    output[index] = samples[left] + (samples[right] - samples[left]) * fraction;
  }
  return output;
}

function measurePcmActivity(samples) {
  assert(samples.length >= ACTIVITY_WINDOW_FRAMES, "PCM is longer than one activity window");
  let rollingEnergy = 0;
  for (let frame = 0; frame < ACTIVITY_WINDOW_FRAMES; frame += 1) {
    rollingEnergy += samples[frame] * samples[frame];
  }
  const windowRms = new Float64Array(samples.length - ACTIVITY_WINDOW_FRAMES + 1);
  let peakRms = 0;
  for (let start = 0; start < windowRms.length; start += 1) {
    if (start > 0) {
      rollingEnergy += samples[start + ACTIVITY_WINDOW_FRAMES - 1] ** 2;
      rollingEnergy -= samples[start - 1] ** 2;
    }
    const rms = Math.sqrt(Math.max(0, rollingEnergy / ACTIVITY_WINDOW_FRAMES));
    windowRms[start] = rms;
    peakRms = Math.max(peakRms, rms);
  }
  const thresholdRms = Math.max(ACTIVITY_ABSOLUTE_RMS_FLOOR, peakRms * ACTIVITY_RELATIVE_RMS);
  const firstActiveWindow = windowRms.findIndex((rms) => rms >= thresholdRms);
  let lastActiveWindow = windowRms.length - 1;
  while (lastActiveWindow >= 0 && windowRms[lastActiveWindow] < thresholdRms) lastActiveWindow -= 1;
  assert(firstActiveWindow >= 0 && lastActiveWindow >= firstActiveWindow, "PCM activity interval is ordered");
  return {
    firstActiveFrame: firstActiveWindow,
    lastActiveFrame: Math.min(samples.length - 1, lastActiveWindow + ACTIVITY_WINDOW_FRAMES - 1),
    peakRms,
    thresholdRms,
  };
}

function measureRenderedPhysicalBounds(samples) {
  const firstNonZeroFrame = samples.findIndex((sample) => sample !== 0);
  let lastNonZeroFrame = samples.length - 1;
  while (lastNonZeroFrame >= 0 && samples[lastNonZeroFrame] === 0) lastNonZeroFrame -= 1;
  assert(firstNonZeroFrame >= 0 && lastNonZeroFrame >= firstNonZeroFrame, "rendered PCM physical bounds are ordered");
  return { firstNonZeroFrame, lastNonZeroFrame };
}

async function loadSourceAssets() {
  const assets = {};
  for (const [label, file] of Object.entries(GUIDE_ASSETS)) {
    const bytes = await readFile(resolve(GUIDE_DIR, file));
    const sourceWav = parsePcm16MonoWav(bytes, SOURCE_RATE);
    const sourceSamples = decodePcmFrames(sourceWav);
    const samples = resampleLinearDeterministic(sourceSamples, SOURCE_RATE, SAMPLE_RATE);
    assets[label] = {
      file,
      sourceSha256: sha256(bytes),
      samples,
      activity: measurePcmActivity(samples),
    };
  }
  return assets;
}

async function inspectOutputs(directory, manifest) {
  const inspected = {};
  for (const output of manifest.outputs) {
    const bytes = await readFile(resolve(directory, output.file));
    const wav = parsePcm16MonoWav(bytes, SAMPLE_RATE);
    const peak = peakCode(wav);
    const hash = sha256(bytes);
    assert(wav.frames === output.frames, `${output.file} manifest frame count matches WAV`);
    assert(hash === output.sha256, `${output.file} manifest SHA matches WAV`);
    assert(peak < 32768, `${output.file} contains no full-scale/clipped PCM sample`);
    inspected[output.file] = { frames: wav.frames, peakCode: peak, sha256: hash };
  }
  return inspected;
}

function outputMap(manifest) {
  return new Map(manifest.outputs.map((output) => [output.file, output]));
}

function clickProjection(events) {
  return events.map((event) => ({
    song: event.song,
    measure: event.measure,
    pass: event.pass,
    beat: event.beat,
    bpm: event.bpm,
    localFrame: event.localFrame,
    strong: event.strong,
    frequencyHz: event.frequencyHz,
  }));
}

function canonicalManifest(manifest) {
  const copy = JSON.parse(JSON.stringify(manifest));
  for (const output of copy.outputs ?? []) delete output.absolutePath;
  return copy;
}

function verifySemanticSchedule(manifest, expectedMode) {
  assert(manifest.mode === expectedMode, `manifest mode is ${expectedMode}`);
  assert(manifest.guideSpec.mode === expectedMode, `guideSpec mode is ${expectedMode}`);
  assert(manifest.guideSpec.activityRule.windowFrames === ACTIVITY_WINDOW_FRAMES, "manifest describes 10 ms activity windows");
  assert(manifest.guideSpec.activityRule.description.includes("forward-looking"), "activity-window direction is truthful");
  assert(!manifest.guideSpec.activityRule.description.includes("trailing"), "activity rule does not claim a trailing window");
  assert(manifest.guideSpec.activityRule.perceptualEndMarginFrames === END_MARGIN_FRAMES, "manifest describes a 50 ms perceptual margin");
  assert(manifest.totals.clickCount === 1492, "click count is 1492");
  assert(manifest.totals.physicalGuideEventCount === 33, "physical guide count is 33");
  const counts = {};
  for (const event of manifest.guideEvents) {
    counts[event.label] = (counts[event.label] ?? 0) + 1;
    assert(event.audibleActivityRule.includes("forward-looking"), `${event.label} event activity rule direction is truthful`);
    assert(!event.audibleActivityRule.includes("trailing"), `${event.label} event activity rule does not claim trailing windows`);
  }
  for (const asset of Object.values(manifest.guideSpec.assets)) {
    assert(asset.activity.rule.includes("forward-looking"), `${asset.file} asset activity rule direction is truthful`);
    assert(!asset.activity.rule.includes("trailing"), `${asset.file} asset activity rule does not claim trailing windows`);
  }
  assert(counts.Intro === 1 && counts.Verse === 4 && counts["Pre Chorus"] === 4, "Intro/Verse/Pre Chorus counts are exact");
  assert(counts.Chorus === 5 && counts.Interlude === 2 && counts.Breakdown === 1 && counts.Outro === 2, "section counts are exact");
  assert(counts.Looping === 8 && counts.Break === 1 && counts.Trans === 4 && counts.Complete === 1, "operation/transition counts are exact");
  assert(counts.Bridge === undefined, "Bridge remains suppressed by Looping priority");
  const transMeasures = manifest.guideEvents.filter((event) => event.label === "Trans").map((event) => event.announcesMeasure);
  assert(JSON.stringify(transMeasures) === JSON.stringify([149, 151, 153, 155]), "Trans remains every two measures");
  const loopingPasses = manifest.guideEvents.filter((event) => event.label === "Looping").map((event) => event.announcesPass);
  assert(JSON.stringify(loopingPasses) === JSON.stringify([1, 2, 3, 4, 5, 6, 7, 8]), "Looping remains eight passes");
  const complete = manifest.guideEvents.find((event) => event.label === "Complete");
  assert(complete.announcesSong === "madow-hoshi" && complete.announcesMeasure === 1 && complete.announcesBeat === 1, "Complete remains the Madow boundary cue");
  assert(!manifest.guideEvents.some((event) => event.announcesSong === "madow-hoshi" && event.announcesMeasure === 1 && event.label !== "Complete"), "Madow Intro has no competing voice");
  const intro = manifest.guideEvents.find((event) => event.label === "Intro");
  assert(intro.globalFrame === 0 && intro.announcesGlobalFrame === 0, "Life Intro remains frame zero");
  return counts;
}

function assertNoGuideOverlap(manifest, sourceAssets) {
  const sorted = [...manifest.guideEvents].sort((left, right) => left.globalFrame - right.globalFrame);
  for (let index = 1; index < sorted.length; index += 1) {
    const previous = sorted[index - 1];
    const current = sorted[index];
    assert(
      previous.globalFrame + sourceAssets[previous.label].samples.length <= current.globalFrame,
      `${previous.label} and ${current.label} full PCM voices do not overlap`,
    );
  }
}

function independentlyResolveTargetFrame(manifest, event) {
  const target = manifest.clickEvents.find((click) => (
    click.song === event.announcesSong
    && click.measure === event.announcesMeasure
    && click.pass === event.announcesPass
    && click.beat === event.announcesBeat
  ));
  assert(target, `${event.label} semantic target resolves to a click event`);
  assert(event.announcesGlobalFrame === target.globalFrame, `${event.label} announced global frame matches independently resolved target click`);
  return target.globalFrame;
}

async function verifyRenderedGuides(directory, manifest, sourceAssets, expectedMode) {
  assertNoGuideOverlap(manifest, sourceAssets);
  const bytes = await readFile(resolve(directory, "connected-guide.wav"));
  const connectedGuide = parsePcm16MonoWav(bytes, SAMPLE_RATE);
  const activityEvidence = [];
  let completeEvidence;
  for (const event of manifest.guideEvents) {
    const asset = sourceAssets[event.label];
    assert(event.globalFrame >= 0 && event.globalFrame + asset.samples.length <= connectedGuide.frames, `${event.label} rendered interval is in bounds`);
    for (let offset = 0; offset < asset.samples.length; offset += 1) {
      const actual = connectedGuide.data.readInt16LE((event.globalFrame + offset) * 2);
      const expected = quantizePcm16Sample(asset.samples[offset]);
      assert(actual === expected, `${event.label} rendered PCM matches independently resampled source at offset ${offset}`);
    }
    const renderedSamples = decodePcmFrames(connectedGuide, event.globalFrame, event.globalFrame + asset.samples.length);
    const renderedActivity = measurePcmActivity(renderedSamples);
    const renderedPhysical = measureRenderedPhysicalBounds(renderedSamples);
    assert(Math.abs(renderedActivity.firstActiveFrame - asset.activity.firstActiveFrame) <= 1, `${event.label} rendered audible onset matches source activity within one frame`);
    assert(Math.abs(renderedActivity.lastActiveFrame - asset.activity.lastActiveFrame) <= 1, `${event.label} rendered audible end matches source activity within one frame`);
    const actualAudibleEndGlobalFrame = event.globalFrame + renderedActivity.lastActiveFrame;
    const independentlyResolvedTargetFrame = independentlyResolveTargetFrame(manifest, event);
    const actualEndMarginFrames = independentlyResolvedTargetFrame - actualAudibleEndGlobalFrame;
    if (expectedMode === "perceptual-preview" && event.label !== "Intro") {
      assert(actualEndMarginFrames >= END_MARGIN_FRAMES - 1, `${event.label} rendered activity ends at least 50 ms before target`);
      assert(actualEndMarginFrames <= END_MARGIN_FRAMES + 1, `${event.label} rendered activity aligns within one frame of 50 ms`);
    }
    assert(Math.abs(event.audibleEndGlobalFrame - actualAudibleEndGlobalFrame) <= 1, `${event.label} manifest audible end agrees with independently measured render`);
    assert(Math.abs(event.audibleEndMarginFrames - actualEndMarginFrames) <= 1, `${event.label} manifest audible margin agrees with independently measured render`);
    activityEvidence.push({
      label: event.label,
      targetMeasure: event.announcesMeasure,
      targetPass: event.announcesPass,
      scheduledLeadFrames: independentlyResolvedTargetFrame - event.globalFrame,
      renderedAudibleEndMarginFrames: actualEndMarginFrames,
    });
    if (event.label === "Complete") {
      completeEvidence = {
        event,
        physicalFirstGlobalFrame: event.globalFrame + renderedPhysical.firstNonZeroFrame,
        physicalLastGlobalFrame: event.globalFrame + renderedPhysical.lastNonZeroFrame,
      };
    }
  }
  assert(completeEvidence, "rendered guide contains Complete evidence");
  const madow = manifest.songs["madow-hoshi"];
  const expectedStartsWithTail = completeEvidence.physicalLastGlobalFrame >= madow.connectedStartFrame;
  assert(madow.completePhysicalOnsetFrame === completeEvidence.event.globalFrame, "Complete physical onset metadata is event-derived");
  assert(madow.completePhysicalFirstNonZeroFrame === completeEvidence.physicalFirstGlobalFrame, "Complete first non-zero metadata matches rendered PCM");
  assert(madow.completePhysicalLastNonZeroFrame === completeEvidence.physicalLastGlobalFrame, "Complete last non-zero metadata matches rendered PCM");
  assert(madow.startsWithCompleteTail === expectedStartsWithTail, "startsWithCompleteTail is derived from rendered non-zero PCM");
  assert(madow.completeTargetFrame === madow.connectedStartFrame, "Complete target is the Madow boundary");
  let firstMadowSecondHasPcm = false;
  const firstSecondEnd = Math.min(connectedGuide.frames, madow.connectedStartFrame + SAMPLE_RATE);
  for (let frame = madow.connectedStartFrame; frame < firstSecondEnd; frame += 1) {
    if (connectedGuide.data.readInt16LE(frame * 2) !== 0) {
      firstMadowSecondHasPcm = true;
      break;
    }
  }
  if (expectedMode === "default") {
    assert(expectedStartsWithTail && firstMadowSecondHasPcm, "default Madow guide begins with Complete's rendered tail");
  } else {
    assert(!expectedStartsWithTail && !firstMadowSecondHasPcm, "perceptual Madow guide truthfully begins with one second of silence");
  }
  return { activityEvidence, completeEvidence: { ...completeEvidence, event: undefined, startsWithTail: expectedStartsWithTail } };
}

function assertPcmSliceEqual(sourceWav, sourceStartFrame, excerptWav, message) {
  assert(sourceStartFrame >= 0 && sourceStartFrame + excerptWav.frames <= sourceWav.frames, `${message} range is in bounds`);
  for (let frame = 0; frame < excerptWav.frames; frame += 1) {
    assert(
      excerptWav.data.readInt16LE(frame * 2) === sourceWav.data.readInt16LE((sourceStartFrame + frame) * 2),
      `${message} PCM matches at frame ${frame}`,
    );
  }
}

async function verifyComparisonSnippets(baseline, perceptual) {
  assert(perceptual.comparisonSnippets.length === 2, "there are two A/B target pairs");
  const targetMeasures = perceptual.comparisonSnippets.map((snippet) => snippet.targetMeasure);
  assert(JSON.stringify(targetMeasures) === JSON.stringify([26, 34]), "A/B pairs target Life m26 and m34");
  const baselineMix = parsePcm16MonoWav(await readFile(resolve(BASELINE_DIR, "jinsei-over-mix.wav")), SAMPLE_RATE);
  const perceptualMix = parsePcm16MonoWav(await readFile(resolve(RUN1_DIR, "jinsei-over-mix.wav")), SAMPLE_RATE);
  for (const snippet of perceptual.comparisonSnippets) {
    const expectedTarget = perceptual.clickEvents.find((event) => (
      event.song === "jinsei-over" && event.measure === snippet.targetMeasure && event.pass === 1 && event.beat === 1
    ));
    assert(expectedTarget && expectedTarget.localFrame === snippet.targetFrame, `m${snippet.targetMeasure} A/B target is the exact downbeat`);
    assert(snippet.windowBeforeBeats === 4 && snippet.windowAfterBeats === 2, `m${snippet.targetMeasure} A/B window is four beats before through two after`);
    const expectedFrames = snippet.endFrame - snippet.startFrame;
    const currentWav = parsePcm16MonoWav(await readFile(resolve(RUN1_DIR, snippet.current)), SAMPLE_RATE);
    const perceptualWav = parsePcm16MonoWav(await readFile(resolve(RUN1_DIR, snippet.perceptual)), SAMPLE_RATE);
    assert(currentWav.frames === expectedFrames && perceptualWav.frames === expectedFrames, `m${snippet.targetMeasure} A/B frame counts match manifest range`);
    assertPcmSliceEqual(baselineMix, snippet.startFrame, currentWav, `m${snippet.targetMeasure} current A/B`);
    assertPcmSliceEqual(perceptualMix, snippet.startFrame, perceptualWav, `m${snippet.targetMeasure} perceptual A/B`);
    assert(sha256(currentWav.bytes) !== sha256(perceptualWav.bytes), `m${snippet.targetMeasure} current and perceptual excerpts differ`);
  }
  assert(baseline.comparisonSnippets === undefined || baseline.comparisonSnippets.length === 0, "baseline has no opt-in A/B outputs");
}

async function verifyInvalidCliRejected() {
  const cases = [
    {
      name: "unknown flag",
      output: resolve(RUN1_DIR, "_invalid-unknown-flag-output"),
      args: (output) => [output, "--definitely-unknown"],
    },
    {
      name: "unknown short flag",
      output: resolve(RUN1_DIR, "_invalid-unknown-short-flag-output"),
      args: (output) => [output, "-x"],
    },
    {
      name: "extra positional output",
      output: resolve(RUN1_DIR, "_invalid-extra-arg-output"),
      args: (output) => [output, `${output}-extra`],
    },
    {
      name: "duplicate mode",
      output: resolve(RUN1_DIR, "_invalid-duplicate-mode-output"),
      args: (output) => [output, "--perceptual-preview", "--mode=perceptual-preview"],
    },
  ];
  for (const testCase of cases) {
    assert(!(await exists(testCase.output)), `${testCase.name} test output is initially absent`);
    const result = spawnSync(process.execPath, [EXPORTER, ...testCase.args(testCase.output)], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      windowsHide: true,
    });
    assert(result.status !== 0, `${testCase.name} exits non-zero`);
    assert(!(await exists(testCase.output)), `${testCase.name} is rejected before output creation`);
  }
}

async function main() {
  const baseline = await readManifest(BASELINE_DIR);
  const defaultCurrent = await readManifest(DEFAULT_CURRENT_DIR);
  const run1 = await readManifest(RUN1_DIR);
  const run2 = await readManifest(RUN2_DIR);
  const sourceAssets = await loadSourceAssets();
  const baselineOutputs = await inspectOutputs(BASELINE_DIR, baseline);
  const defaultOutputs = await inspectOutputs(DEFAULT_CURRENT_DIR, defaultCurrent);
  const run1Outputs = await inspectOutputs(RUN1_DIR, run1);
  const run2Outputs = await inspectOutputs(RUN2_DIR, run2);

  verifySemanticSchedule(defaultCurrent, "default");
  const physicalCounts = verifySemanticSchedule(run1, "perceptual-preview");
  const defaultRendered = await verifyRenderedGuides(DEFAULT_CURRENT_DIR, defaultCurrent, sourceAssets, "default");
  const perceptualRendered = await verifyRenderedGuides(RUN1_DIR, run1, sourceAssets, "perceptual-preview");
  await verifyComparisonSnippets(baseline, run1);
  await verifyInvalidCliRejected();

  assert(JSON.stringify(canonicalManifest(run1)) === JSON.stringify(canonicalManifest(run2)), "canonical manifests are deterministic across perceptual runs");
  const run1Map = outputMap(run1);
  const run2Map = outputMap(run2);
  for (const [file, output] of run1Map) {
    assert(run2Map.has(file), `${file} exists in both perceptual runs`);
    assert(output.sha256 === run2Map.get(file).sha256, `${file} manifest SHA is stable across runs`);
    assert(run1Outputs[file].sha256 === run2Outputs[file].sha256, `${file} bytes are stable across runs`);
  }

  const baselineMap = outputMap(baseline);
  const defaultMap = outputMap(defaultCurrent);
  for (const [file, output] of baselineMap) {
    assert(defaultMap.has(file), `fresh default contains baseline output ${file}`);
    assert(output.sha256 === defaultMap.get(file).sha256, `${file} fresh default WAV is byte-identical to baseline`);
    assert(baselineOutputs[file].sha256 === defaultOutputs[file].sha256, `${file} actual fresh default bytes match baseline`);
  }
  const clickFiles = ["jinsei-over-click.wav", "madow-hoshi-click.wav", "connected-click.wav"];
  for (const file of clickFiles) {
    assert(baselineMap.get(file).sha256 === run1Map.get(file).sha256, `${file} perceptual WAV is byte-identical to baseline`);
  }
  const baselineClickHash = sha256(Buffer.from(JSON.stringify(clickProjection(baseline.clickEvents))));
  assert(baselineClickHash === sha256(Buffer.from(JSON.stringify(clickProjection(defaultCurrent.clickEvents)))), "fresh default click schedule matches baseline");
  assert(baselineClickHash === sha256(Buffer.from(JSON.stringify(clickProjection(run1.clickEvents)))), "perceptual click schedule matches baseline");

  const canonicalManifestHash = sha256(Buffer.from(JSON.stringify(canonicalManifest(run1))));
  console.log(JSON.stringify({
    baselineDirectory: BASELINE_DIR,
    freshDefaultDirectory: DEFAULT_CURRENT_DIR,
    perceptualRun1: RUN1_DIR,
    perceptualRun2: RUN2_DIR,
    gates: {
      strictCliBeforeOutput: "PASS",
      defaultAllNineWavsByteIdenticalToBaseline: "PASS",
      deterministicPerceptualWavAndManifest: "PASS",
      wavFormatAndNoClipping: "PASS",
      sourceDecodeAndIndependentResample: "PASS",
      renderedPcmMatchesIndependentSource: "PASS",
      clickStemsAndScheduleByteIdenticalToBaseline: "PASS",
      guideNonOverlap: "PASS",
      renderedAudibleEndAtTargetMinus50ms: "PASS",
      completeMetadataAndTailBothModes: "PASS",
      abExcerptSlices: "PASS",
      semanticsLoopTransComplete: "PASS",
    },
    counts: { guideEventCount: run1.guideEvents.length, physicalCounts },
    complete: {
      default: defaultRendered.completeEvidence,
      perceptual: perceptualRendered.completeEvidence,
    },
    perceptualLeadRangeFrames: {
      minimum: Math.min(...perceptualRendered.activityEvidence.filter((event) => event.label !== "Intro").map((event) => event.scheduledLeadFrames)),
      maximum: Math.max(...perceptualRendered.activityEvidence.filter((event) => event.label !== "Intro").map((event) => event.scheduledLeadFrames)),
    },
    hashes: {
      canonicalManifest: canonicalManifestHash,
      clickSchedule: baselineClickHash,
      clickStems: Object.fromEntries(clickFiles.map((file) => [file, run1Map.get(file).sha256])),
      allPerceptualOutputs: Object.fromEntries([...run1Map].map(([file, output]) => [file, output.sha256])),
    },
    peakPcm16CodeByOutput: Object.fromEntries(Object.entries(run1Outputs).map(([file, wav]) => [file, wav.peakCode])),
    comparisonSnippets: run1.comparisonSnippets,
  }, null, 2));
}

await main();
