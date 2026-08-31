import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const checkerPath = resolve(scriptDirectory, "check-control-upper-workspaces-browser.mjs");
const viewports = ["3840x2160", "2560x1440", "1920x1080", "1280x720"];
// The checker has bounded deadlines for every owned teardown operation. This
// larger process-level limit is only a secondary defense for an unexpected
// synchronous hang and must never turn a timed-out child into a pass.
const checkerTimeoutOverride = process.env.SYNDOCAL_CONTROL_UPPER_WORKSPACES_TEST_TIMEOUT_MS;
const checkerTimeoutMs = checkerTimeoutOverride === undefined ? 300_000 : Number(checkerTimeoutOverride);
if (!Number.isSafeInteger(checkerTimeoutMs) || checkerTimeoutMs <= 0) {
  throw new Error(
    `SYNDOCAL_CONTROL_UPPER_WORKSPACES_TEST_TIMEOUT_MS must be a positive integer; received ${JSON.stringify(checkerTimeoutOverride)}`,
  );
}
let allPassed = true;
const forbiddenAggregateEnv = "SYNDOCAL_CONTROL_UPPER_WORKSPACES_CLEANUP_SELF_TEST";
if (process.env[forbiddenAggregateEnv] !== undefined) {
  console.error(
    `check:control-upper-workspaces refuses test-only ${forbiddenAggregateEnv}; `
    + "run the checker directly for its focused self-test so the production aggregate cannot report a blind pass",
  );
  allPassed = false;
}

const recoveryQueryTimeoutMs = 15_000;
const recoveryTaskkillTimeoutMs = 30_000;
const maxRecoveryBrowserProcesses = 64;
const normalizeProcessNeedle = (value) => String(value).replaceAll("/", "\\").toLowerCase();
const queryProcessRecords = (needle, label, processNames = []) => {
  if (process.platform !== "win32") return [];
  const processFilter = processNames.length > 0
    ? ` -Filter ${JSON.stringify(processNames.map((name) => `Name='${name}'`).join(" OR "))}`
    : "";
  const script = "$ErrorActionPreference='Stop';"
    + `$needle=${JSON.stringify(String(needle))};`
    + `Get-CimInstance Win32_Process${processFilter} -ErrorAction Stop | `
    + "Where-Object { $_.Name -notin @('powershell.exe','taskkill.exe') -and $_.CommandLine "
    + "-and $_.CommandLine.ToLowerInvariant().Contains($needle.ToLowerInvariant()) } | "
    + "Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress";
  const result = spawnSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], {
    encoding: "utf8",
    timeout: recoveryQueryTimeoutMs,
    windowsHide: true,
    maxBuffer: 1_048_576,
  });
  if (result.error) throw new Error(`${label} failed: ${result.error.message}`);
  if (result.status !== 0 || result.signal !== null) {
    throw new Error(`${label} failed with status ${result.status ?? "null"} signal ${result.signal ?? "none"}`);
  }
  const output = String(result.stdout ?? "").trim();
  if (!output) return [];
  const parsed = JSON.parse(output);
  return (Array.isArray(parsed) ? parsed : [parsed])
    .filter((record) => Number.isSafeInteger(Number(record?.ProcessId)) && Number(record.ProcessId) > 0)
    .map((record) => ({
      pid: Number(record.ProcessId),
      parentPid: Number(record.ParentProcessId),
      name: String(record.Name ?? ""),
      commandLine: String(record.CommandLine ?? ""),
    }));
};
const runRecoveryTaskkill = (pid, label) => {
  const result = spawnSync("taskkill.exe", ["/PID", String(pid), "/T", "/F"], {
    stdio: "ignore",
    timeout: recoveryTaskkillTimeoutMs,
    windowsHide: true,
  });
  if (result.error) return `${label} failed: ${result.error.message}`;
  if (result.status !== 0 || result.signal !== null) {
    return `${label} failed with status ${result.status ?? "null"} signal ${result.signal ?? "none"}`;
  }
  return null;
};
const recoverTimedOutChecker = (result, runToken, viewport) => {
  const failures = [];
  if (process.platform !== "win32") {
    if (Number.isSafeInteger(result?.pid) && result.pid > 0) {
      try {
        process.kill(result.pid, "SIGTERM");
      } catch (error) {
        if (error?.code !== "ESRCH") failures.push(`checker PID ${result.pid} termination failed: ${error.message}`);
      }
    }
    return failures;
  }

  const checkerNeedle = normalizeProcessNeedle(checkerPath);
  const checkerRecords = queryProcessRecords(checkerNeedle, `${viewport} timed-out checker query`, ["node.exe"]);
  const checkerPid = Number.isSafeInteger(result?.pid) && result.pid > 0 ? result.pid : null;
  const checkerRecord = checkerPid === null ? null : checkerRecords.find(({ pid }) => pid === checkerPid);
  if (checkerRecord) {
    const failure = runRecoveryTaskkill(checkerRecord.pid, `${viewport} timed-out checker tree PID ${checkerRecord.pid}`);
    if (failure && queryProcessRecords(checkerNeedle, `${viewport} timed-out checker post-kill query`, ["node.exe"]).some(({ pid }) => pid === checkerRecord.pid)) {
      failures.push(failure);
    }
  }

  const browserNeedle = normalizeProcessNeedle(runToken);
  let browserRecords = queryProcessRecords(browserNeedle, `${viewport} timed-out browser query`, ["chrome.exe", "msedge.exe"]);
  if (browserRecords.length > maxRecoveryBrowserProcesses) {
    failures.push(`${viewport} timed-out browser query returned ${browserRecords.length} processes; refusing an unbounded cleanup sweep`);
    browserRecords = [];
  }
  for (let round = 0; browserRecords.length > 0 && round < 3; round += 1) {
    browserRecords.sort((left, right) => {
      const leftWorker = left.commandLine.includes("--type=") ? 1 : 0;
      const rightWorker = right.commandLine.includes("--type=") ? 1 : 0;
      return leftWorker - rightWorker || left.pid - right.pid;
    });
    for (const record of browserRecords) {
      const failure = runRecoveryTaskkill(record.pid, `${viewport} timed-out browser tree PID ${record.pid}`);
      if (failure) {
        const stillOwned = queryProcessRecords(browserNeedle, `${viewport} timed-out browser post-kill query`, ["chrome.exe", "msedge.exe"])
          .some(({ pid }) => pid === record.pid);
        if (stillOwned) failures.push(failure);
      }
    }
    browserRecords = queryProcessRecords(browserNeedle, `${viewport} timed-out browser drain query`, ["chrome.exe", "msedge.exe"]);
  }
  if (browserRecords.length > 0) {
    failures.push(`${viewport} timed-out browser descendants remain after bounded tree cleanup: ${browserRecords.map(({ pid }) => pid).join(",")}`);
  }

  const remainingChecker = queryProcessRecords(checkerNeedle, `${viewport} timed-out checker final query`, ["node.exe"]);
  if (remainingChecker.length > 0) {
    failures.push(`${viewport} timed-out checker descendants remain after bounded tree cleanup: ${remainingChecker.map(({ pid }) => pid).join(",")}`);
  }
  return failures;
};

for (const viewport of allPassed ? viewports : []) {
  const runToken = `aggregate-${process.pid}-${viewport}-${randomUUID()}`;
  const childEnv = {
    ...process.env,
    SYNDOCAL_CONTROL_UPPER_WORKSPACES_RUN_TOKEN: runToken,
  };
  let result;
  try {
    result = spawnSync(process.execPath, [checkerPath], {
      cwd: resolve(scriptDirectory, ".."),
      env: { ...childEnv, SYNDOCAL_CONTROL_VIEWPORT: viewport },
      stdio: "inherit",
      windowsHide: true,
      timeout: checkerTimeoutMs,
    });
  } catch (error) {
    console.error(`control-upper-workspaces ${viewport} failed to spawn: ${error instanceof Error ? error.message : String(error)}`);
    allPassed = false;
    break;
  }

  if (result.error) {
    if (result.error.code === "ETIMEDOUT") {
      console.error(
        `control-upper-workspaces ${viewport} failed closed: checker exceeded `
        + `${checkerTimeoutMs} ms secondary timeout; inner cleanup did not complete; refusing to claim pass`,
      );
      let recoveryFailures;
      try {
        recoveryFailures = recoverTimedOutChecker(result, runToken, viewport);
      } catch (error) {
        recoveryFailures = [`timeout recovery inspection failed: ${error instanceof Error ? error.message : String(error)}`];
      }
      if (recoveryFailures.length > 0) {
        console.error(`control-upper-workspaces ${viewport} timeout cleanup failed closed: ${recoveryFailures.join("; ")}`);
      } else {
        console.error(`control-upper-workspaces ${viewport} timeout cleanup verified checker/browser tree recovery for run ${runToken}`);
      }
    } else {
      console.error(`control-upper-workspaces ${viewport} failed to spawn: ${result.error.message}`);
    }
    allPassed = false;
    break;
  }
  if (result.status === null) {
    console.error(`control-upper-workspaces ${viewport} failed closed: child status was null (signal=${result.signal ?? "none"})`);
    allPassed = false;
    break;
  }
  if (result.status !== 0) {
    console.error(`control-upper-workspaces ${viewport} failed with status ${result.status}`);
    allPassed = false;
    break;
  }
}

if (allPassed) {
  console.log(`check:control-upper-workspaces passed: ${viewports.join(", ")}`);
} else {
  process.exitCode = 1;
}
