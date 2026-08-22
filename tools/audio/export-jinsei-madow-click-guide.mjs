import { createHash } from "node:crypto";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SAMPLE_RATE = 48_000;
const CLICK_SECONDS = 0.05;
const CLICK_RAMP_SECONDS = 0.045;
const CLICK_START_GAIN = 0.1;
const CLICK_END_GAIN = 0.0001;
const STRONG_FREQUENCY = 1320;
const WEAK_FREQUENCY = 920;
const MIX_CLICK_GAIN = 1.0;
const MIX_GUIDE_GAIN = 0.85;
const LIFE_FIXED_BPM = 170;
const MADOW_BPM = 194;
const LIFE_AUTHORED_MEASURES = 156;
const LIFE_LOOP_MEASURE = 98;
const LIFE_LOOP_PASSES = 8;
const LIFE_RAMP_START_MEASURE = 149;
const LIFE_RAMP_BEATS = 32;
const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "../..");
const GUIDE_DIR = resolve(REPO_ROOT, "app/src-tauri/assets/timeline-guide/en");
const OUTPUT_DIR = resolve(process.argv[2] ?? "C:/TEMP/syndocal-show-audio");

const MADOW_METER = new Map([
  [18, 6],
  [117, 5],
  [118, 5],
  [119, 5],
  [120, 6],
  [121, 5],
  [122, 5],
  [123, 5],
  [124, 6],
]);

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

// Section starts are copied from the authoritative SongPart maps in
// C:/Users/kouty/Documents/Guiter/app/trainer.tsx. The English Guide labels
// below are the product-approved vocabulary for those chart parts.
const LIFE_SECTION_GUIDES = [
  { label: "Intro", measure: 1 },
  { label: "Verse", measure: 18 },
  { label: "Pre Chorus", measure: 26 },
  { label: "Chorus", measure: 34 },
  { label: "Interlude", measure: 50 },
  { label: "Verse", measure: 58 },
  { label: "Pre Chorus", measure: 66 },
  { label: "Chorus", measure: 82 },
  { label: "Bridge", measure: 98 },
  { label: "Breakdown", measure: 114 },
  { label: "Chorus", measure: 117 },
  { label: "Outro", measure: 142 },
];

const MADOW_SECTION_GUIDES = [
  { label: "Verse", measure: 19 },
  { label: "Pre Chorus", measure: 43 },
  { label: "Chorus", measure: 77 },
  { label: "Interlude", measure: 101 },
  { label: "Verse", measure: 125 },
  { label: "Pre Chorus", measure: 147 },
  { label: "Chorus", measure: 164 },
  { label: "Outro", measure: 192 },
];

function assert(condition, message) {
  if (!condition) throw new Error(`Self-test failed: ${message}`);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function round9(value) {
  return Number(value.toFixed(9));
}

function readPcm16MonoWav(bytes, expectedSampleRate) {
  assert(bytes.length >= 44, "guide WAV is at least 44 bytes");
  assert(bytes.toString("ascii", 0, 4) === "RIFF", "guide WAV has RIFF header");
  assert(bytes.toString("ascii", 8, 12) === "WAVE", "guide WAV has WAVE header");
  let format;
  let data;
  for (let offset = 12; offset + 8 <= bytes.length;) {
    const id = bytes.toString("ascii", offset, offset + 4);
    const size = bytes.readUInt32LE(offset + 4);
    const body = offset + 8;
    assert(body + size <= bytes.length, `guide WAV chunk ${id} is in bounds`);
    if (id === "fmt ") {
      assert(size >= 16, "guide WAV fmt chunk is complete");
      format = {
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
  assert(format, "guide WAV has fmt chunk");
  assert(data, "guide WAV has data chunk");
  assert(format.audioFormat === 1, "guide WAV is integer PCM");
  assert(format.channels === 1, "guide WAV is mono");
  assert(format.sampleRate === expectedSampleRate, `guide WAV is ${expectedSampleRate} Hz`);
  assert(format.bitsPerSample === 16, "guide WAV is 16-bit");
  assert(data.length % 2 === 0, "guide WAV has complete PCM16 frames");
  const samples = new Float64Array(data.length / 2);
  for (let i = 0; i < samples.length; i += 1) {
    samples[i] = data.readInt16LE(i * 2) / 32768;
  }
  return { samples, format };
}

function resampleLinearDeterministic(samples, sourceRate, destinationRate) {
  const outputLength = Math.round(samples.length * destinationRate / sourceRate);
  const output = new Float64Array(outputLength);
  for (let i = 0; i < outputLength; i += 1) {
    const sourcePosition = i * sourceRate / destinationRate;
    const left = Math.min(samples.length - 1, Math.floor(sourcePosition));
    const right = Math.min(samples.length - 1, left + 1);
    const fraction = sourcePosition - left;
    output[i] = samples[left] + (samples[right] - samples[left]) * fraction;
  }
  return output;
}

function encodePcm16MonoWav(samples) {
  const dataBytes = samples.length * 2;
  const wav = Buffer.alloc(44 + dataBytes);
  wav.write("RIFF", 0);
  wav.writeUInt32LE(36 + dataBytes, 4);
  wav.write("WAVE", 8);
  wav.write("fmt ", 12);
  wav.writeUInt32LE(16, 16);
  wav.writeUInt16LE(1, 20);
  wav.writeUInt16LE(1, 22);
  wav.writeUInt32LE(SAMPLE_RATE, 24);
  wav.writeUInt32LE(SAMPLE_RATE * 2, 28);
  wav.writeUInt16LE(2, 32);
  wav.writeUInt16LE(16, 34);
  wav.write("data", 36);
  wav.writeUInt32LE(dataBytes, 40);
  let peak = 0;
  for (let i = 0; i < samples.length; i += 1) {
    const sample = samples[i];
    assert(Number.isFinite(sample), `output sample ${i} is finite`);
    peak = Math.max(peak, Math.abs(sample));
    assert(Math.abs(sample) <= 1, `output sample ${i} does not clip (${sample})`);
    wav.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(sample * 32767))), 44 + i * 2);
  }
  return { wav, peak };
}

function validateGeneratedWav(bytes, expectedFrames) {
  assert(bytes.length === 44 + expectedFrames * 2, "generated WAV byte count matches frames");
  assert(bytes.toString("ascii", 0, 4) === "RIFF", "generated WAV has RIFF header");
  assert(bytes.readUInt32LE(4) === bytes.length - 8, "generated WAV RIFF size is valid");
  assert(bytes.toString("ascii", 8, 12) === "WAVE", "generated WAV has WAVE header");
  assert(bytes.toString("ascii", 12, 16) === "fmt ", "generated WAV has fmt chunk");
  assert(bytes.readUInt16LE(20) === 1, "generated WAV is PCM");
  assert(bytes.readUInt16LE(22) === 1, "generated WAV is mono");
  assert(bytes.readUInt32LE(24) === SAMPLE_RATE, "generated WAV is 48 kHz");
  assert(bytes.readUInt16LE(34) === 16, "generated WAV is 16-bit");
  assert(bytes.toString("ascii", 36, 40) === "data", "generated WAV has data chunk");
  assert(bytes.readUInt32LE(40) === expectedFrames * 2, "generated WAV data size is valid");
}

function rampSecondsAtBeat(beat) {
  const bpmDelta = MADOW_BPM - LIFE_FIXED_BPM;
  return 60 * LIFE_RAMP_BEATS / bpmDelta
    * Math.log((LIFE_FIXED_BPM + bpmDelta * beat / LIFE_RAMP_BEATS) / LIFE_FIXED_BPM);
}

function guideBeforeTarget(clicks, {
  label, measure, pass = 1, allowSongStart = false, leadPerformanceBeats = 1, kind = "section",
}) {
  const targetIndex = clicks.findIndex((event) => (
    event.measure === measure && event.pass === pass && event.beat === 1
  ));
  assert(targetIndex >= 0, `${label} target measure ${measure} pass ${pass} exists`);
  assert(targetIndex >= leadPerformanceBeats || allowSongStart, `${label} target has the requested preceding performance beats`);
  const target = clicks[targetIndex];
  const onset = targetIndex === 0 ? target : clicks[targetIndex - leadPerformanceBeats];
  return {
    song: onset.song,
    label,
    kind,
    semanticLabels: [label],
    measure: onset.measure,
    pass: onset.pass,
    beat: onset.beat,
    localFrame: onset.localFrame,
    announcesSong: target.song,
    announcesMeasure: target.measure,
    announcesPass: target.pass,
    announcesBeat: target.beat,
    leadPerformanceBeats: targetIndex === 0 ? 0 : leadPerformanceBeats,
    placement: targetIndex === 0
      ? "song-start-no-preroll"
      : leadPerformanceBeats === 1 ? "previous-performance-beat" : `${leadPerformanceBeats}-performance-beats-before`,
  };
}

function buildSchedules() {
  const lifeClicks = [];
  let seconds = 0;
  const appendLifeBeat = (measure, pass, beat, bpm, eventSeconds) => {
    lifeClicks.push({
      song: "jinsei-over",
      measure,
      pass,
      beat,
      bpm: round9(bpm),
      localFrame: Math.round(eventSeconds * SAMPLE_RATE),
      strong: beat === 1,
      frequencyHz: beat === 1 ? STRONG_FREQUENCY : WEAK_FREQUENCY,
    });
  };

  for (let measure = 1; measure <= 148; measure += 1) {
    const passes = measure === LIFE_LOOP_MEASURE ? LIFE_LOOP_PASSES : 1;
    for (let pass = 1; pass <= passes; pass += 1) {
      for (let beat = 1; beat <= 4; beat += 1) {
        appendLifeBeat(measure, pass, beat, LIFE_FIXED_BPM, seconds);
        seconds += 60 / LIFE_FIXED_BPM;
      }
    }
  }

  const rampStartSeconds = seconds;
  for (let rampBeat = 0; rampBeat < LIFE_RAMP_BEATS; rampBeat += 1) {
    const measure = LIFE_RAMP_START_MEASURE + Math.floor(rampBeat / 4);
    const beat = rampBeat % 4 + 1;
    const eventSeconds = rampStartSeconds + rampSecondsAtBeat(rampBeat);
    const bpm = LIFE_FIXED_BPM
      + (MADOW_BPM - LIFE_FIXED_BPM) * rampBeat / LIFE_RAMP_BEATS;
    appendLifeBeat(measure, 1, beat, bpm, eventSeconds);
  }
  const lifeDurationSeconds = rampStartSeconds + rampSecondsAtBeat(LIFE_RAMP_BEATS);
  const lifeFrames = Math.round(lifeDurationSeconds * SAMPLE_RATE);

  const madowClicks = [];
  let madowBeatOffset = 0;
  for (let measure = 1; measure <= 207; measure += 1) {
    const beats = MADOW_METER.get(measure) ?? 4;
    for (let beat = 1; beat <= beats; beat += 1) {
      madowClicks.push({
        song: "madow-hoshi",
        measure,
        pass: 1,
        beat,
        bpm: MADOW_BPM,
        localFrame: Math.round(madowBeatOffset * 60 / MADOW_BPM * SAMPLE_RATE),
        strong: beat === 1,
        frequencyHz: beat === 1 ? STRONG_FREQUENCY : WEAK_FREQUENCY,
      });
      madowBeatOffset += 1;
    }
  }
  const madowFrames = Math.round(madowBeatOffset * 60 / MADOW_BPM * SAMPLE_RATE);

  // Bridge at measure 98 is intentionally silent because its requested cue
  // would collide with the required Looping cue for pass 1.
  const lifeGuides = LIFE_SECTION_GUIDES.filter((guide) => guide.label !== "Bridge").map((guide) => guideBeforeTarget(lifeClicks, {
    ...guide,
    allowSongStart: guide.measure === 1,
  }));
  for (let pass = 1; pass <= LIFE_LOOP_PASSES; pass += 1) {
    lifeGuides.push(guideBeforeTarget(lifeClicks, {
      label: "Looping",
      measure: LIFE_LOOP_MEASURE,
      pass,
      kind: "operation",
    }));
  }
  lifeGuides.push(guideBeforeTarget(lifeClicks, { label: "Break", measure: 99, kind: "operation" }));
  for (let measure = LIFE_RAMP_START_MEASURE; measure <= LIFE_AUTHORED_MEASURES; measure += 2) {
    lifeGuides.push(guideBeforeTarget(lifeClicks, { label: "Trans", measure, kind: "transition" }));
  }

  // Madow Intro is intentionally silent: the cross-song cue is Complete only.
  const madowGuides = MADOW_SECTION_GUIDES.map((guide) => (
    guideBeforeTarget(madowClicks, guide)
  ));
  const lifeLastBeat = lifeClicks.at(-1);
  const madowFirstBeat = madowClicks[0];
  const transitionGuide = {
    song: lifeLastBeat.song,
    label: "Complete",
    kind: "transition",
    semanticLabels: ["Complete"],
    measure: lifeLastBeat.measure,
    pass: lifeLastBeat.pass,
    beat: lifeLastBeat.beat,
    localFrame: lifeLastBeat.localFrame,
    announcesSong: madowFirstBeat.song,
    announcesMeasure: madowFirstBeat.measure,
    announcesPass: madowFirstBeat.pass,
    announcesBeat: madowFirstBeat.beat,
    leadPerformanceBeats: 1,
    placement: "previous-connected-performance-beat",
  };

  return {
    life: { clicks: lifeClicks, guides: lifeGuides, frames: lifeFrames, durationSeconds: lifeDurationSeconds },
    madow: { clicks: madowClicks, guides: madowGuides, frames: madowFrames, durationSeconds: madowBeatOffset * 60 / MADOW_BPM },
    transitionGuide,
  };
}

function renderClick(events, frameCount) {
  const output = new Float64Array(frameCount);
  const clickFrames = Math.ceil(CLICK_SECONDS * SAMPLE_RATE);
  for (const event of events) {
    for (let frame = 0; frame < clickFrames && event.localFrame + frame < frameCount; frame += 1) {
      const time = frame / SAMPLE_RATE;
      const progress = Math.min(1, time / CLICK_RAMP_SECONDS);
      const gain = CLICK_START_GAIN * ((CLICK_END_GAIN / CLICK_START_GAIN) ** progress);
      const phase = 2 * Math.PI * event.frequencyHz * time;
      output[event.localFrame + frame] += (Math.sin(phase) >= 0 ? 1 : -1) * gain;
    }
  }
  return output;
}

function renderGuide(events, frameCount, assets) {
  const output = new Float64Array(frameCount);
  for (const event of events) {
    const asset = assets[event.label];
    assert(asset, `guide asset exists for ${event.label}`);
    assert(event.globalFrame + asset.samples.length <= frameCount, `${event.label} voice tail stays inside the connected timeline`);
    for (let frame = 0; frame < asset.samples.length; frame += 1) {
      output[event.globalFrame + frame] += asset.samples[frame];
    }
  }
  return output;
}

function renderMix(click, guide) {
  assert(click.length === guide.length, "click and Guide stems have equal duration");
  const output = new Float64Array(click.length);
  for (let i = 0; i < output.length; i += 1) {
    output[i] = click[i] * MIX_CLICK_GAIN + guide[i] * MIX_GUIDE_GAIN;
  }
  return output;
}

function countBy(values, key) {
  const counts = {};
  for (const value of values) counts[value[key]] = (counts[value[key]] ?? 0) + 1;
  return counts;
}

function clickScheduleFingerprint(events) {
  const projection = events.map((event) => ({
    song: event.song,
    measure: event.measure,
    pass: event.pass,
    beat: event.beat,
    bpm: event.bpm,
    localFrame: event.localFrame,
    strong: event.strong,
    frequencyHz: event.frequencyHz,
  }));
  return sha256(Buffer.from(JSON.stringify(projection)));
}

async function loadGuideAssets() {
  const assets = {};
  for (const [label, file] of Object.entries(GUIDE_ASSETS)) {
    const path = resolve(GUIDE_DIR, file);
    const bytes = await readFile(path);
    const decoded = readPcm16MonoWav(bytes, 22_050);
    assets[label] = {
      path,
      file,
      sourceFrames: decoded.samples.length,
      sourceSha256: sha256(bytes),
      samples: resampleLinearDeterministic(decoded.samples, 22_050, SAMPLE_RATE),
    };
  }
  return assets;
}

async function writeStem(fileName, samples, kind, song) {
  const { wav, peak } = encodePcm16MonoWav(samples);
  validateGeneratedWav(wav, samples.length);
  const outputPath = resolve(OUTPUT_DIR, fileName);
  await writeFile(outputPath, wav);
  const reread = await readFile(outputPath);
  validateGeneratedWav(reread, samples.length);
  return {
    file: fileName,
    absolutePath: outputPath,
    song,
    kind,
    frames: samples.length,
    durationSeconds: round9(samples.length / SAMPLE_RATE),
    peakFloat: round9(peak),
    sha256: sha256(reread),
  };
}

await mkdir(OUTPUT_DIR, { recursive: true });
const assets = await loadGuideAssets();
const schedule = buildSchedules();
const guideTargetClick = (event) => {
  const songSchedule = event.announcesSong === "jinsei-over" ? schedule.life : schedule.madow;
  const target = songSchedule.clicks.find((click) => (
    click.measure === event.announcesMeasure
    && click.pass === event.announcesPass
    && click.beat === event.announcesBeat
  ));
  assert(target, `${event.label} announced click exists`);
  return target;
};
const guideSongOffset = (song) => (song === "madow-hoshi" ? schedule.life.frames : 0);
const connectedGuideEvents = [
  ...schedule.life.guides,
  schedule.transitionGuide,
  ...schedule.madow.guides,
].map((event) => ({
  ...event,
  globalFrame: guideSongOffset(event.song) + event.localFrame,
  announcesGlobalFrame: guideSongOffset(event.announcesSong) + guideTargetClick(event).localFrame,
})).sort((left, right) => left.globalFrame - right.globalFrame);

assert(LIFE_AUTHORED_MEASURES * 4 === 624, "Life authored beat count is 624");
assert((LIFE_LOOP_PASSES - 1) * 4 === 28, "Life loop adds 28 beats");
assert(schedule.life.clicks.length === 652, "Life performance click count is 652");
assert(schedule.madow.clicks.length === 840, "Madow click count is 840");
assert(schedule.life.clicks.length + schedule.madow.clicks.length === 1492, "connected click count is 1492");
assert(schedule.life.frames === 11_010_639, "Life timeline frames stay unchanged");
assert(schedule.madow.frames === 12_470_103, "Madow timeline frames stay unchanged");
assert(clickScheduleFingerprint(schedule.life.clicks) === "6e27f826d53572fe914d0e551b38261b3ec7f474a2219846d60061bc5785e46b", "Life click sample schedule stays byte-for-byte unchanged");
assert(clickScheduleFingerprint(schedule.madow.clicks) === "809aa5faa51df68f1298ef9c239fae77a6ff56c2604150f89fa0efe4be3f573b", "Madow click sample schedule stays byte-for-byte unchanged");
const physicalGuideCounts = countBy(connectedGuideEvents, "label");
assert(physicalGuideCounts.Intro === 1, "Guide has one Life Intro cue and intentionally no Madow Intro cue");
assert(physicalGuideCounts.Verse === 4, "Guide has 4 Verse cues");
assert(physicalGuideCounts["Pre Chorus"] === 4, "Guide has 4 Pre Chorus cues");
assert(physicalGuideCounts.Chorus === 5, "Guide has 5 Chorus cues");
assert(physicalGuideCounts.Interlude === 2, "Guide has 2 Interlude cues");
assert(physicalGuideCounts.Breakdown === 1, "Guide has one chart-section Breakdown cue");
assert(physicalGuideCounts.Outro === 2, "Guide has 2 Outro cues");
assert(physicalGuideCounts.Bridge === undefined, "Guide intentionally suppresses the colliding measure-98 Bridge cue");
assert(physicalGuideCounts.Looping === 8, "Guide has 8 Looping cues");
assert(physicalGuideCounts.Break === 1, "Guide has 1 Break cue");
assert(physicalGuideCounts.Trans === 4, "Guide has 4 two-measure Trans cues");
assert(physicalGuideCounts.Complete === 1, "Guide has one Complete cue and no Madow Intro voice");
assert(connectedGuideEvents.length === 33, "Guide has 33 non-overlapping physical events");
const semanticGuideCounts = {};
for (const event of connectedGuideEvents) {
  for (const label of event.semanticLabels) {
    semanticGuideCounts[label] = (semanticGuideCounts[label] ?? 0) + 1;
  }
}
assert(semanticGuideCounts.Intro === 1, "Guide semantically announces Life Intro only");
assert(semanticGuideCounts.Verse === 4, "Guide semantically announces 4 Verses");
assert(semanticGuideCounts["Pre Chorus"] === 4, "Guide semantically announces 4 Pre Choruses");
assert(semanticGuideCounts.Chorus === 5, "Guide semantically announces 5 Choruses");
assert(semanticGuideCounts.Interlude === 2, "Guide semantically announces 2 Interludes");
assert(semanticGuideCounts.Bridge === undefined, "Guide has no Bridge semantic event because Looping owns the coincident entry");
assert(semanticGuideCounts.Breakdown === 1, "Guide semantically announces one Breakdown section");
assert(semanticGuideCounts.Outro === 2, "Guide semantically announces 2 Outros");
assert(semanticGuideCounts.Looping === 8, "Guide semantically announces 8 loop passes");
assert(semanticGuideCounts.Break === 1, "Guide semantically announces one Break");
assert(semanticGuideCounts.Trans === 4, "Guide semantically announces 4 two-measure transition blocks");
assert(semanticGuideCounts.Complete === 1, "Guide semantically announces Complete once");
assert(Object.values(semanticGuideCounts).reduce((sum, count) => sum + count, 0) === 33, "Guide has 33 semantic announcements");
const chartKeys = (song, kind) => connectedGuideEvents
  .filter((event) => event.announcesSong === song && event.kind === kind)
  .map((event) => `${event.label}@${event.announcesMeasure}:${event.announcesPass}`);
assert(JSON.stringify(chartKeys("jinsei-over", "section")) === JSON.stringify([
  "Intro@1:1", "Verse@18:1", "Pre Chorus@26:1", "Chorus@34:1",
  "Interlude@50:1", "Verse@58:1", "Pre Chorus@66:1", "Chorus@82:1",
  "Breakdown@114:1", "Chorus@117:1", "Outro@142:1",
]), "Life section Guide chart matches every approved song-map boundary exactly");
assert(JSON.stringify(chartKeys("madow-hoshi", "section")) === JSON.stringify([
  "Verse@19:1", "Pre Chorus@43:1", "Chorus@77:1", "Interlude@101:1",
  "Verse@125:1", "Pre Chorus@147:1", "Chorus@164:1", "Outro@192:1",
]), "Madow section Guide chart matches every approved boundary and omits Intro exactly");
assert(JSON.stringify(chartKeys("jinsei-over", "operation")) === JSON.stringify([
  "Looping@98:1", "Looping@98:2", "Looping@98:3", "Looping@98:4",
  "Looping@98:5", "Looping@98:6", "Looping@98:7", "Looping@98:8", "Break@99:1",
]), "Life operational Guide chart has every loop pass then Break exactly");
assert(JSON.stringify(chartKeys("jinsei-over", "transition")) === JSON.stringify([
  "Trans@149:1", "Trans@151:1", "Trans@153:1", "Trans@155:1",
]), "Life transition Guide chart announces every two-measure block exactly");
assert(JSON.stringify(chartKeys("madow-hoshi", "transition")) === JSON.stringify([
  "Complete@1:1",
]), "connected transition chart has Complete only at Madow entry");
for (const [measure, beats] of MADOW_METER) {
  assert(schedule.madow.clicks.filter((event) => event.measure === measure).length === beats, `Madow measure ${measure} has ${beats} clicks`);
}
for (let measure = 1; measure <= 207; measure += 1) {
  const expectedBeats = MADOW_METER.get(measure) ?? 4;
  assert(schedule.madow.clicks.filter((event) => event.measure === measure).length === expectedBeats, `Madow measure ${measure} uses its exact meter`);
}
assert(schedule.madow.clicks.filter((event) => event.strong).length === 207, "Madow has one strong downbeat per measure");
for (let index = 1; index < schedule.life.clicks.length; index += 1) {
  assert(schedule.life.clicks[index].localFrame > schedule.life.clicks[index - 1].localFrame, `Life click ${index} is sample-monotonic`);
}
const transitionClicks = schedule.life.clicks.filter((event) => event.measure >= LIFE_RAMP_START_MEASURE);
assert(transitionClicks.length === LIFE_RAMP_BEATS, "transition has 32 beats");
assert(transitionClicks.filter((event) => event.strong).length === 8, "transition has 8 measure downbeats");
assert(transitionClicks.at(-1).localFrame < schedule.life.frames, "transition last click precedes exact boundary");
assert(schedule.madow.clicks[0].localFrame === 0, "Madow first click is its own downbeat");
for (const event of connectedGuideEvents) {
  if (event.placement === "song-start-no-preroll") {
    assert(event.label === "Intro" && event.song === "jinsei-over" && event.globalFrame === 0, "only Life Intro uses the no-preroll frame-zero exception");
    assert(event.announcesGlobalFrame === 0 && event.leadPerformanceBeats === 0, "Life Intro truthfully records zero-beat lead");
    continue;
  }
  const connectedClicks = [
    ...schedule.life.clicks.map((click) => ({ ...click, globalFrame: click.localFrame })),
    ...schedule.madow.clicks.map((click) => ({ ...click, globalFrame: schedule.life.frames + click.localFrame })),
  ];
  const targetIndex = connectedClicks.findIndex((click) => click.globalFrame === event.announcesGlobalFrame);
  assert(targetIndex > 0, `${event.label} target has a previous connected click`);
  const previousClick = connectedClicks[targetIndex - 1];
  assert(event.globalFrame === previousClick.globalFrame, `${event.label} starts on the exact previous performance beat`);
  assert(event.measure === previousClick.measure && event.pass === previousClick.pass && event.beat === previousClick.beat, `${event.label} manifest onset measure/pass/beat matches the previous performance beat`);
  assert(event.leadPerformanceBeats === 1, `${event.label} records an exact one-beat lead`);
}
for (let index = 1; index < connectedGuideEvents.length; index += 1) {
  const previous = connectedGuideEvents[index - 1];
  const current = connectedGuideEvents[index];
  assert(
    previous.globalFrame + assets[previous.label].samples.length <= current.globalFrame,
    `${previous.label} Guide voice does not overlap ${current.label} across the connected show`,
  );
}

const lifeClick = renderClick(schedule.life.clicks, schedule.life.frames);
const madowClick = renderClick(schedule.madow.clicks, schedule.madow.frames);
const connectedFrames = schedule.life.frames + schedule.madow.frames;
const connectedClick = new Float64Array(connectedFrames);
connectedClick.set(lifeClick, 0);
connectedClick.set(madowClick, schedule.life.frames);
const connectedGuide = renderGuide(connectedGuideEvents, connectedFrames, assets);
for (const event of connectedGuideEvents) {
  const asset = assets[event.label];
  const firstAudibleFrame = asset.samples.findIndex((sample) => sample !== 0);
  assert(firstAudibleFrame >= 0, `${event.label} source contains audible PCM`);
  assert(
    connectedGuide[event.globalFrame + firstAudibleFrame] === asset.samples[firstAudibleFrame],
    `${event.label} first audible PCM sample is injected at its exact scheduled onset offset`,
  );
}
const lifeGuide = connectedGuide.slice(0, schedule.life.frames);
const madowGuide = connectedGuide.slice(schedule.life.frames);
const lifeMix = renderMix(lifeClick, lifeGuide);
const madowMix = renderMix(madowClick, madowGuide);
const connectedMix = renderMix(connectedClick, connectedGuide);
const firstMadowConnectedClick = schedule.life.frames + schedule.madow.clicks[0].localFrame;
assert(firstMadowConnectedClick === schedule.life.frames, "Madow connected downbeat lands exactly at the independently calculated Life end boundary");
assert(schedule.transitionGuide.localFrame === schedule.life.clicks.at(-1).localFrame, "Complete starts on Life's exact final performance beat");
assert(connectedGuideEvents.find((event) => event.label === "Complete").announcesGlobalFrame === firstMadowConnectedClick, "Complete announces Madow's exact first downbeat without a Madow Intro voice");
assert(schedule.transitionGuide.localFrame + assets.Complete.samples.length > schedule.life.frames, "Complete voice tail crosses the song boundary without being truncated from the connected Guide");

const outputs = [];
outputs.push(await writeStem("jinsei-over-click.wav", lifeClick, "click", "jinsei-over"));
outputs.push(await writeStem("jinsei-over-guide.wav", lifeGuide, "guide", "jinsei-over"));
outputs.push(await writeStem("jinsei-over-mix.wav", lifeMix, "mix", "jinsei-over"));
outputs.push(await writeStem("madow-hoshi-click.wav", madowClick, "click", "madow-hoshi"));
outputs.push(await writeStem("madow-hoshi-guide.wav", madowGuide, "guide", "madow-hoshi"));
outputs.push(await writeStem("madow-hoshi-mix.wav", madowMix, "mix", "madow-hoshi"));
outputs.push(await writeStem("connected-click.wav", connectedClick, "click", "connected"));
outputs.push(await writeStem("connected-guide.wav", connectedGuide, "guide", "connected"));
outputs.push(await writeStem("connected-mix.wav", connectedMix, "mix", "connected"));

const decorateClick = (event, songOffsetFrames) => ({
  ...event,
  globalFrame: songOffsetFrames + event.localFrame,
  localSeconds: round9(event.localFrame / SAMPLE_RATE),
  globalSeconds: round9((songOffsetFrames + event.localFrame) / SAMPLE_RATE),
});
const decorateGuide = (event) => ({
  ...event,
  localSeconds: round9(event.localFrame / SAMPLE_RATE),
  globalSeconds: round9(event.globalFrame / SAMPLE_RATE),
  announcesLocalFrame: guideTargetClick(event).localFrame,
  announcesLocalSeconds: round9(guideTargetClick(event).localFrame / SAMPLE_RATE),
  announcesGlobalSeconds: round9(event.announcesGlobalFrame / SAMPLE_RATE),
  sourceAsset: GUIDE_ASSETS[event.label],
  sourceFrames22050: assets[event.label].sourceFrames,
  resampledFrames48000: assets[event.label].samples.length,
});

const manifest = {
  schema: "syndocal-show-audio-export/v2",
  deterministicSource: "tools/audio/export-jinsei-madow-click-guide.mjs",
  audioFormat: { codec: "pcm_s16le", sampleRate: SAMPLE_RATE, channels: 1, bitsPerSample: 16 },
  clickSpec: {
    waveform: "square",
    strongFrequencyHz: STRONG_FREQUENCY,
    weakFrequencyHz: WEAK_FREQUENCY,
    durationMs: CLICK_SECONDS * 1000,
    startGain: CLICK_START_GAIN,
    exponentialRampEndGain: CLICK_END_GAIN,
    exponentialRampMs: CLICK_RAMP_SECONDS * 1000,
  },
  guideSpec: {
    voice: "Microsoft Zira Desktop",
    synthesisRate: 2,
    synthesisVolume: 100,
    pitchOrTimeShift: false,
    sourceFormat: "PCM16 mono 22050 Hz",
    resampler: "deterministic linear interpolation 22050->48000; outputLength=round(inputFrames*48000/22050)",
    placement: "Section and operational announcements start on the exact previous performance click. Life Intro is the sole frame-zero/no-preroll exception. Madow Intro is intentionally silent and Complete starts on Life's final click.",
    sectionEvidence: {
      source: "C:/Users/kouty/Documents/Guiter/app/trainer.tsx",
      life: "LIFE_OVER_SONG_MAP: Intro 1, Verse 18/58, Pre Chorus 26/66, Chorus 34/82/117, Interlude 50, Bridge 98, Breakdown 114, Outro 142, End/Trans 149",
      madow: "MADOW_SONG_MAP: Intro 1, Verse 19/125, Pre Chorus 43/147, Chorus 77/164, Interlude 101, Outro 192",
      productExceptions: [
        "Life Bridge 98 is silent because Looping pass 1 owns the same required pre-entry beat.",
        "Madow Intro 1 is silent; Complete is the sole transition-boundary voice.",
        "Life End 149-156 uses Trans every two measures at 149/151/153/155.",
      ],
    },
    assets: Object.fromEntries(Object.entries(assets).map(([label, asset]) => [label, {
      file: asset.file,
      absolutePath: asset.path,
      sourceFrames: asset.sourceFrames,
      resampledFrames: asset.samples.length,
      sha256: asset.sourceSha256,
    }])),
  },
  mixSpec: {
    clickGain: MIX_CLICK_GAIN,
    guideGain: MIX_GUIDE_GAIN,
    rationale: "Click peak is <=0.1 and Guide events never overlap; 1.0*click + 0.85*Guide has a <=0.95 mathematical bound before PCM16 quantization.",
    clippingPolicy: "fail export if any float sample exceeds unity; no limiter or normalization",
  },
  songs: {
    "jinsei-over": {
      authoredMeasures: LIFE_AUTHORED_MEASURES,
      authoredBeats: 624,
      loopMeasure: LIFE_LOOP_MEASURE,
      loopTotalPasses: LIFE_LOOP_PASSES,
      loopAddedBeats: 28,
      performanceClicks: schedule.life.clicks.length,
      timelineFrames: schedule.life.frames,
      timelineDurationSeconds: round9(schedule.life.frames / SAMPLE_RATE),
      fixedTempo: { bpm: LIFE_FIXED_BPM, measures: "1-148" },
      transitionTempo: {
        measures: "149-156",
        beats: LIFE_RAMP_BEATS,
        startBpm: LIFE_FIXED_BPM,
        endBoundaryBpm: MADOW_BPM,
        shape: "BPM(b)=170+24*b/32; time(b)=60*32/24*ln(BPM(b)/170)",
        startFrame: transitionClicks[0].localFrame,
        lastClickFrame: transitionClicks.at(-1).localFrame,
        endBoundaryFrame: schedule.life.frames,
      },
    },
    "madow-hoshi": {
      measures: 207,
      bpm: MADOW_BPM,
      performanceClicks: schedule.madow.clicks.length,
      timelineFrames: schedule.madow.frames,
      timelineDurationSeconds: round9(schedule.madow.frames / SAMPLE_RATE),
      connectedStartFrame: schedule.life.frames,
      connectedStartSeconds: round9(schedule.life.frames / SAMPLE_RATE),
      meterMap: Object.fromEntries([...MADOW_METER].map(([measure, beats]) => [String(measure), beats])),
      defaultBeatsPerMeasure: 4,
      startsWithCompleteTail: true,
      completePhysicalOnsetFrame: schedule.transitionGuide.localFrame,
      completeTargetFrame: schedule.life.frames,
    },
  },
  totals: {
    clickCount: schedule.life.clicks.length + schedule.madow.clicks.length,
    physicalGuideEventCount: connectedGuideEvents.length,
    semanticGuideEventCount: Object.values(semanticGuideCounts).reduce((sum, count) => sum + count, 0),
    physicalGuideCounts,
    semanticGuideCounts,
    connectedFrames,
    connectedDurationSeconds: round9(connectedFrames / SAMPLE_RATE),
  },
  clickEvents: [
    ...schedule.life.clicks.map((event) => decorateClick(event, 0)),
    ...schedule.madow.clicks.map((event) => decorateClick(event, schedule.life.frames)),
  ],
  guideEvents: connectedGuideEvents.map(decorateGuide),
  outputs,
};

const manifestPath = resolve(OUTPUT_DIR, "manifest.json");
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(JSON.stringify({
  outputDirectory: OUTPUT_DIR,
  manifestPath,
  lifeFrames: schedule.life.frames,
  madowStartFrame: schedule.life.frames,
  madowFrames: schedule.madow.frames,
  connectedFrames,
  clickCount: manifest.totals.clickCount,
  physicalGuideEventCount: manifest.totals.physicalGuideEventCount,
  semanticGuideEventCount: manifest.totals.semanticGuideEventCount,
  outputCount: outputs.length,
}, null, 2));
