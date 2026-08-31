import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  blocked,
  isObject,
  isSafePositiveInteger,
  passed,
  readOwn,
  REQUIRED_CHECK_IDS,
} from "./show-structural-preflight/primitives.mjs";
import { mappingSummary } from "./show-structural-preflight/mapping.mjs";
import {
  firstMeasureEvidence,
  followIntent,
  validateLoopIntent,
} from "./show-structural-preflight/timeline.mjs";
import {
  activeBankProjectionCheck,
  projectFromInput,
  shapeCheck,
  timelineBank,
} from "./show-structural-preflight/project.mjs";

/**
 * Read-only, token-free preflight for the authored two-stage show contract.
 *
 * This module deliberately reads the persisted JSON shape produced by the
 * current Syndocal project writer.  It does not import the runtime, invoke
 * Tauri, inspect files referenced by a project, open a device, or send a
 * network packet.  A successful result is therefore only structural/authored
 * evidence; it is not a media, hardware, network, or runtime acceptance.
 */

export {
  SHOW_TITLE_CONTAINS,
  SHOW_RELEASE_TRIGGER,
  SHOW_FOLLOW_BAR_MILLIUNITS,
  TIMELINE_TEMPO_METER_MAP_VERSION,
} from "./show-structural-preflight/primitives.mjs";

/**
 * Run the structural preflight over a parsed current-format `.sdc` object or
 * JSON text.  The returned report is deterministic and contains no machine
 * or runtime observations.
 */
export function preflightShowContract(input, options = {}) {
  const { project, parseError } = projectFromInput(input);
  const expected = isObject(options) ? options : {};
  if (parseError) {
    return {
      status: "BLOCKED",
      checks: [blocked("sdc_shape", `Syndocal .sdc is not readable as strict JSON: ${parseError}`)],
      limitations: limitations(),
    };
  }

  const checks = [shapeCheck(project)];
  if (checks[0].status === "BLOCKED") {
    return { status: "BLOCKED", checks, limitations: limitations() };
  }

  const bankResult = timelineBank(project);
  const authoredTimelineIds = new Set(
    bankResult.bank?.map((timeline) => timeline.id) ?? [],
  );
  if (isSafePositiveInteger(project.snapshot.timeline.id)) {
    authoredTimelineIds.add(project.snapshot.timeline.id);
  }
  const mapping = mappingSummary(project, authoredTimelineIds);
  checks.push(mapping.check);
  checks.push(activeBankProjectionCheck(project, bankResult));
  const sourceIdFromMapping = mapping.production.length === 1 ? mapping.production[0].timelineId : null;
  const explicitSourceId = expected.sourceTimelineId ?? sourceIdFromMapping;
  if (!isSafePositiveInteger(explicitSourceId)) {
    checks.push(blocked("explicit_ids", "source Timeline ID is not explicit and positive in the production mapping"));
  } else {
    const source = bankResult.bank?.find((timeline) => timeline.id === explicitSourceId);
    const destinationId = isObject(source?.follow)
      ? readOwn(source.follow, "next_timeline_id")
      : null;
    const explicitDestinationId = expected.destinationTimelineId ?? destinationId;
    const idErrors = [];
    if (mapping.production.length === 1 && mapping.production[0].timelineId !== explicitSourceId) {
      idErrors.push("production mapping timelineId does not match the explicit source Timeline ID");
    }
    if (project.snapshot.timeline.id !== explicitSourceId) {
      idErrors.push("snapshot.timeline.id does not match the explicit source Timeline ID");
    }
    if (!isSafePositiveInteger(explicitDestinationId)) idErrors.push("destination Timeline ID is not explicit and positive in Follow");
    if (source && !isSafePositiveInteger(destinationId)) idErrors.push("source Follow next_timeline_id is missing or invalid");
    if (bankResult.bank && isSafePositiveInteger(explicitDestinationId)
        && !bankResult.bank.some((timeline) => timeline.id === explicitDestinationId)) {
      idErrors.push("destination Timeline ID is not present in the authored Timeline bank");
    }
    if (source && isSafePositiveInteger(destinationId) && explicitDestinationId !== destinationId) {
      idErrors.push("provided destination Timeline ID does not match source Follow next_timeline_id");
    }
    checks.push(idErrors.length === 0
      ? passed("explicit_ids", `source Timeline ${explicitSourceId} and destination Timeline ${explicitDestinationId} are explicit authored IDs`)
      : blocked("explicit_ids", idErrors.join("; ")));

    if (!bankResult.bank) {
      checks.push(blocked("adjacent_bank_entries", bankResult.error));
      for (const id of [
        "follow",
        "source_measure_transition",
        "destination_first_measure",
        "destination_pedal_wait",
        "source_loop",
      ]) {
        checks.push(blocked(id, bankResult.error));
      }
    } else {
      const sourceIndex = bankResult.bank.findIndex((timeline) => timeline.id === explicitSourceId);
      const destinationIndex = bankResult.bank.findIndex((timeline) => timeline.id === explicitDestinationId);
      const adjacent = sourceIndex >= 0 && destinationIndex === sourceIndex + 1;
      checks.push(adjacent
        ? passed("adjacent_bank_entries", `Timeline bank entries ${explicitSourceId} and ${explicitDestinationId} are adjacent in authored order`)
        : blocked("adjacent_bank_entries", `Timeline bank entries ${explicitSourceId} and ${explicitDestinationId} are not adjacent`));

      const source = sourceIndex >= 0 ? bankResult.bank[sourceIndex] : null;
      const destination = destinationIndex >= 0 ? bankResult.bank[destinationIndex] : null;
      const follow = !destination
        ? { error: "destination Timeline entry is missing from the authored Timeline bank" }
        : followIntent(source, explicitDestinationId);
      checks.push(follow.error ? blocked("follow", follow.error) : passed("follow", follow.detail));

      const enabledFollowCount = bankResult.bank.filter((timeline) => timeline?.follow?.enabled === true).length;
      const sourceMeasure = firstMeasureEvidence(source, "source");
      if (follow.error) {
        checks.push(blocked("source_measure_transition", follow.error));
      } else if (enabledFollowCount !== 1) {
        checks.push(blocked("source_measure_transition", `expected exactly one enabled source meter-aware Follow transition; found ${enabledFollowCount}`));
      } else if (!isSafePositiveInteger(source?.duration_ms)) {
        checks.push(blocked("source_measure_transition", "source Timeline bank duration_ms must be finite and positive"));
      } else if (sourceMeasure.error) {
        checks.push(blocked("source_measure_transition", sourceMeasure.error));
      } else {
        checks.push(passed("source_measure_transition", `exactly one enabled Follow transition uses one source meter-aware bar (${sourceMeasure.durationMs} ms from explicit ${sourceMeasure.point.numerator}/${sourceMeasure.point.denominator} at ${sourceMeasure.point.bpm} BPM)`));
      }

      const destinationMeasure = firstMeasureEvidence(destination, "destination");
      if (destinationMeasure.error) {
        checks.push(blocked("destination_first_measure", destinationMeasure.error));
      } else if (!isSafePositiveInteger(destination?.duration_ms) || destinationMeasure.durationMs > destination.duration_ms) {
        checks.push(blocked("destination_first_measure", "destination duration_ms must be finite and cover one complete destination first measure"));
      } else {
        checks.push(passed("destination_first_measure", `destination first measure resolves to finite ${destinationMeasure.durationMs} ms within duration_ms=${destination.duration_ms}`));
      }

      if (follow.error) {
        checks.push(blocked("destination_pedal_wait", follow.error));
      } else if (destinationMeasure.error) {
        checks.push(blocked("destination_pedal_wait", destinationMeasure.error));
      } else if (!isSafePositiveInteger(destination?.duration_ms) || destinationMeasure.durationMs > destination.duration_ms) {
        checks.push(blocked("destination_pedal_wait", "destination pedal wait cannot be established without a finite valid destination measure"));
      } else {
        checks.push(passed("destination_pedal_wait", "wait_for_pedal persists paused destination-start intent; runtime pedal release control is outside this persisted project evidence", "authored-intent"));
      }

      const loop = validateLoopIntent(source);
      checks.push(loop.error
        ? blocked("source_loop", loop.error)
        : passed("source_loop", loop.detail, "authored-intent (runtime F13 unverified)"));
    }
  }
  if (!checks.some((check) => check.id === "explicit_ids")) checks.push(blocked("explicit_ids", "source/destination IDs were not resolved"));
  if (!checks.some((check) => check.id === "adjacent_bank_entries")) checks.push(blocked("adjacent_bank_entries", bankResult.error ?? "timeline bank is unavailable"));
  for (const id of REQUIRED_CHECK_IDS) {
    if (!checks.some((check) => check.id === id)) checks.push(blocked(id, "check was not safely evaluable"));
  }
  const blockedChecks = checks.filter((check) => check.status === "BLOCKED");
  return {
    status: blockedChecks.length === 0 ? "PASS" : "BLOCKED",
    checks,
    limitations: limitations(),
  };
}

export function limitations() {
  return [
    "READ-ONLY structural/authored evidence only; no media availability, decoder, hardware, MIDI, DJ Link, network, or runtime-state claim is made.",
    "The wait_for_pedal destination start and non-finite loop are persisted authored intent only; F13 runtime release/settlement is not represented or verified.",
    "Missing or ambiguous measure/tempo evidence is BLOCKED; this preflight never falls back to snapshot.clock or assumes 4/4.",
  ];
}

export async function preflightShowFile(filePath, options = {}) {
  const absolutePath = resolve(filePath);
  try {
    const text = await readFile(absolutePath, "utf8");
    return {
      file: absolutePath,
      ...preflightShowContract(text, options),
    };
  } catch (error) {
    return {
      file: absolutePath,
      status: "BLOCKED",
      checks: [blocked("sdc_shape", `cannot read .sdc without mutation: ${String(error?.message ?? error)}`)],
      limitations: limitations(),
    };
  }
}

async function main(argv) {
  const args = [...argv];
  let json = false;
  if (args[0] === "--json") {
    json = true;
    args.shift();
  }
  if (args.length !== 1 || args[0].startsWith("-")) {
    throw new Error("Usage: node tools/show-structural-preflight.mjs [--json] <project.sdc>");
  }
  const report = await preflightShowFile(args[0]);
  if (json) console.log(JSON.stringify(report, null, 2));
  else {
    console.log(`show structural preflight: ${report.status}`);
    if (report.file) console.log(`file: ${report.file}`);
    for (const check of report.checks) console.log(`[${check.status}] ${check.id}: ${check.detail}`);
    for (const limitation of report.limitations) console.log(`LIMITATION: ${limitation}`);
  }
  if (report.status !== "PASS") process.exitCode = 2;
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 2;
  });
}
