#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const DEFAULT_MAX_DIFFERENCES = 128;

function parseArgs(argv) {
  const options = {
    referencePath: "",
    candidatePath: "",
    evidencePath: "",
    universe: 0,
    maxDifferences: DEFAULT_MAX_DIFFERENCES,
    selfTest: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const nextValue = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`Missing value for ${argument}`);
      return argv[index];
    };
    if (argument === "--reference") options.referencePath = nextValue();
    else if (argument === "--candidate") options.candidatePath = nextValue();
    else if (argument === "--universe") options.universe = Number(nextValue());
    else if (argument === "--evidence") options.evidencePath = nextValue();
    else if (argument === "--max-differences") options.maxDifferences = Number(nextValue());
    else if (argument === "--self-test") options.selfTest = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }

  if (!Number.isInteger(options.universe) || options.universe < 0 || options.universe > 32767) {
    throw new Error("Universe must be an integer from 0 to 32767");
  }
  if (!Number.isInteger(options.maxDifferences) || options.maxDifferences < 0) {
    throw new Error("Maximum differences must be zero or a positive integer");
  }
  if (!options.selfTest && (!options.referencePath || !options.candidatePath)) {
    throw new Error("Both --reference and --candidate evidence files are required");
  }
  return options;
}

function validateChannelData(data, label) {
  if (!Array.isArray(data) || data.length < 2 || data.length > 512) {
    throw new Error(`${label} must contain 2 to 512 DMX channel values`);
  }
  if (data.some((value) => !Number.isInteger(value) || value < 0 || value > 255)) {
    throw new Error(`${label} contains a value outside the DMX byte range`);
  }
}

function frameForUniverse(evidence, universe, label) {
  const keyedFrame = evidence?.lastFrames?.[String(universe)];
  const fallbackFrame = evidence?.lastFrame?.universe === universe ? evidence.lastFrame : null;
  const frame = keyedFrame ?? fallbackFrame;
  if (!frame) throw new Error(`${label} has no captured last frame for universe ${universe}`);
  validateChannelData(frame.data, `${label} universe ${universe}`);
  return frame;
}

function summarizeFrame(frame) {
  let nonZeroChannels = 0;
  let maxValue = 0;
  for (const value of frame.data) {
    if (value > 0) nonZeroChannels += 1;
    maxValue = Math.max(maxValue, value);
  }
  return {
    at: frame.at ?? null,
    source: frame.source ?? null,
    sequence: frame.sequence ?? null,
    digest: frame.digest ?? null,
    dataLength: frame.data.length,
    nonZeroChannels,
    maxValue,
  };
}

function transitionSummary(evidence, universe) {
  const transitions = Array.isArray(evidence?.transitions)
    ? evidence.transitions.filter((transition) => transition?.universe === universe)
    : [];
  return {
    captured: transitions.length,
    includesChannelData: transitions.some((transition) => Array.isArray(transition.data)),
    digests: transitions.map((transition) => transition.digest ?? null),
  };
}

export function compareArtNetEvidence(
  reference,
  candidate,
  { universe = 0, maxDifferences = DEFAULT_MAX_DIFFERENCES } = {},
) {
  const referenceFrame = frameForUniverse(reference, universe, "Reference evidence");
  const candidateFrame = frameForUniverse(candidate, universe, "Candidate evidence");
  const channelCount = Math.max(referenceFrame.data.length, candidateFrame.data.length);
  const differences = [];
  let differingChannels = 0;

  for (let index = 0; index < channelCount; index += 1) {
    const referenceValue = referenceFrame.data[index] ?? null;
    const candidateValue = candidateFrame.data[index] ?? null;
    if (referenceValue === candidateValue) continue;
    differingChannels += 1;
    if (differences.length < maxDifferences) {
      differences.push({
        channel: index + 1,
        reference: referenceValue,
        candidate: candidateValue,
        delta:
          referenceValue === null || candidateValue === null
            ? null
            : candidateValue - referenceValue,
      });
    }
  }

  const referenceTransitions = transitionSummary(reference, universe);
  const candidateTransitions = transitionSummary(candidate, universe);
  const comparableTransitionCount = Math.min(
    referenceTransitions.digests.length,
    candidateTransitions.digests.length,
  );
  let matchingTransitionPrefix = 0;
  while (
    matchingTransitionPrefix < comparableTransitionCount &&
    referenceTransitions.digests[matchingTransitionPrefix] ===
      candidateTransitions.digests[matchingTransitionPrefix]
  ) {
    matchingTransitionPrefix += 1;
  }

  return {
    schema: 1,
    product: "Syndocal Art-Net evidence comparator",
    comparedAt: new Date().toISOString(),
    universe,
    equal: differingChannels === 0,
    channelCount,
    differingChannels,
    reportedDifferences: differences.length,
    differencesTruncated: differingChannels > differences.length,
    differences,
    reference: summarizeFrame(referenceFrame),
    candidate: summarizeFrame(candidateFrame),
    transitions: {
      reference: {
        captured: referenceTransitions.captured,
        includesChannelData: referenceTransitions.includesChannelData,
      },
      candidate: {
        captured: candidateTransitions.captured,
        includesChannelData: candidateTransitions.includesChannelData,
      },
      matchingDigestPrefix: matchingTransitionPrefix,
      exactDigestSequence:
        referenceTransitions.digests.length === candidateTransitions.digests.length &&
        matchingTransitionPrefix === referenceTransitions.digests.length,
      note:
        "Transition timing and phase are diagnostic only; the equal verdict compares the final DMX frame byte-for-byte.",
    },
  };
}

function readEvidence(filePath, label) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    throw new Error(`${label} could not be read as JSON: ${error.message}`);
  }
}

function writeEvidence(filePath, report) {
  if (!filePath) return;
  fs.mkdirSync(path.dirname(path.resolve(filePath)), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
}

function runSelfTest() {
  const frame = {
    universe: 0,
    sequence: 7,
    digest: "same",
    data: [0, 127, 255, 0],
  };
  const reference = {
    lastFrames: { 0: frame },
    transitions: [{ universe: 0, digest: "same", data: frame.data }],
  };
  const equalReport = compareArtNetEvidence(reference, reference);
  if (!equalReport.equal || equalReport.differingChannels !== 0) {
    throw new Error("Equal-frame comparison self-test failed");
  }

  const candidate = {
    lastFrames: { 0: { ...frame, data: [0, 128, 255, 0] } },
    transitions: [{ universe: 0, digest: "different", data: [0, 128, 255, 0] }],
  };
  const differenceReport = compareArtNetEvidence(reference, candidate);
  if (
    differenceReport.equal ||
    differenceReport.differingChannels !== 1 ||
    differenceReport.differences[0]?.channel !== 2 ||
    differenceReport.differences[0]?.delta !== 1
  ) {
    throw new Error("Channel-difference comparison self-test failed");
  }

  let rejectedMalformed = false;
  try {
    compareArtNetEvidence(reference, { lastFrames: { 0: { ...frame, data: [999, 0] } } });
  } catch {
    rejectedMalformed = true;
  }
  if (!rejectedMalformed) throw new Error("Malformed evidence self-test failed");
  console.log("Art-Net evidence comparator self-test: 3 assertions passed");
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runSelfTest();
    return;
  }

  const reference = readEvidence(options.referencePath, "Reference evidence");
  const candidate = readEvidence(options.candidatePath, "Candidate evidence");
  const report = compareArtNetEvidence(reference, candidate, options);
  report.inputs = {
    reference: path.resolve(options.referencePath),
    candidate: path.resolve(options.candidatePath),
  };
  writeEvidence(options.evidencePath, report);
  console.log(JSON.stringify(report, null, 2));
  if (!report.equal) process.exitCode = 1;
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (invokedDirectly) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 2;
  }
}
