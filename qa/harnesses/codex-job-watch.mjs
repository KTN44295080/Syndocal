// Watch a Codex companion job: print one line on terminal status or an N-minute
// log stall, then exit. Used by the supervisor loop so a hung job never sits
// silent (the 341-minute incident of 2026-07-16 was a silent wait).
//
// Usage: node qa/harnesses/codex-job-watch.mjs <jobId> [stallMinutes=5]
import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";

const JOB = process.argv[2];
const STALL_MINUTES = Number(process.argv[3] ?? 5);
if (!JOB) {
  console.log("WATCHER ERROR: no job id");
  process.exit(1);
}
const CLI = `${process.env.USERPROFILE?.replace(/\\/g, "/") ?? "C:/Users/kouty"}/.claude/plugins/cache/openai-codex/codex/1.0.6/scripts/codex-companion.mjs`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let logFile = null;
let lastSize = -1;
let stallSince = Date.now();
for (;;) {
  let status = "unknown";
  try {
    const raw = execFileSync("node", [CLI, "status", JOB, "--json"], { encoding: "utf8" });
    const job = JSON.parse(raw).job;
    status = job?.status ?? "unknown";
    logFile = job?.logFile ?? logFile;
  } catch {
    status = "status-error";
  }
  // queued/starting are pre-run states, not terminal (a job once died while
  // queued - the stall branch below catches that via log-file silence).
  const nonTerminal = ["running", "queued", "starting", "pending", "unknown", "status-error"];
  if (!nonTerminal.includes(status)) {
    console.log(`CODEX JOB TERMINAL: status=${status} job=${JOB}`);
    process.exit(0);
  }
  let size = 0;
  try {
    if (logFile) size = statSync(logFile).size;
  } catch {
    // Log may not exist yet while queued.
  }
  if (size !== lastSize) {
    lastSize = size;
    stallSince = Date.now();
  } else if (Date.now() - stallSince >= STALL_MINUTES * 60 * 1000) {
    console.log(`CODEX JOB STALLED: no log growth for ${STALL_MINUTES}min (size=${size}, lastStatus=${status}) job=${JOB}`);
    process.exit(0);
  }
  await sleep(60000);
}
