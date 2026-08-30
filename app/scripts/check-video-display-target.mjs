import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const hash = (character) => character.repeat(64);

const [panelSource, setupSource, appSource, controllerSource, invokeSource, manifestSource, localizationSource, stylesSource, backendSource] = await Promise.all([
  read("src/components/VideoOutputCreatePanel.tsx"),
  read("src/components/SetupVideoPanel.tsx"),
  read("src/App.tsx"),
  read("src/outputControlController.ts"),
  read("src/tauriInvokeCommands.ts"),
  read("src/tauri-invoke-manifest.json"),
  read("src/uiLocalization.ts"),
  read("src/styles.css"),
  read("src-tauri/src/main.rs"),
]);

const controllerRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  controllerSource,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "outputControlController.ts",
  },
).outputText).toString("base64")}`);
const localizationRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  localizationSource,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "uiLocalization.ts",
  },
).outputText).toString("base64")}`);
const displayAddHelperStart = appSource.indexOf("const staleDisplayAddRecoveryMessages = new Set([");
const displayAddHelperEnd = appSource.indexOf("\n\nexport default function App", displayAddHelperStart);
assert.ok(
  displayAddHelperStart >= 0 && displayAddHelperEnd > displayAddHelperStart,
  "display Add post-commit refresh helper boundary must remain discoverable",
);
const displayAddHelperRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  `${appSource.slice(displayAddHelperStart, displayAddHelperEnd)}\nexport { displayAddPostCommitRefreshResult };`,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: "displayAddPostCommitRefresh.ts",
  },
).outputText).toString("base64")}`);

const displayNumberFromNativeName = (name) => {
  const match = name.trim().match(/^(?:\\\\\\.\\)?DISPLAY([1-9]\d*)$/i);
  if (!match) return undefined;
  const displayNumber = Number(match[1]);
  return Number.isSafeInteger(displayNumber) ? displayNumber : undefined;
};

const targetSummary = (monitor) => ({
  label: `Display ${displayNumberFromNativeName(monitor.name) ?? monitor.index + 1}`,
  displayName: monitor.name.trim() || `Display ${monitor.index + 1}`,
  width: monitor.physicalWidth,
  height: monitor.physicalHeight,
  fullscreen: true,
});

const descriptor = (index, options = {}) => ({
  index,
  identity: hash(String.fromCharCode(97 + index)),
  name: `DISPLAY${index + 1}`,
  physicalWidth: 1920 + index * 160,
  physicalHeight: 1080,
  positionX: index * 1920,
  positionY: 0,
  scaleFactor: 1,
  isEditorMonitor: false,
  ...options,
});

// This mirrors the production boundary: a missing or malformed DTO is never
// partially accepted, and order is retained after validation.
const isValidDescriptor = (value) => value && typeof value === "object"
  && Number.isInteger(value.index) && value.index >= 0 && value.index <= 255
  && typeof value.identity === "string" && /^[0-9a-f]{64}$/.test(value.identity)
  && typeof value.name === "string"
  && Number.isInteger(value.physicalWidth) && value.physicalWidth > 0
  && Number.isInteger(value.physicalHeight) && value.physicalHeight > 0
  && Number.isInteger(value.positionX) && Number.isInteger(value.positionY)
  && Number.isFinite(value.scaleFactor) && value.scaleFactor > 0
  && typeof value.isEditorMonitor === "boolean";
const normalize = (value) => {
  if (!Array.isArray(value)) return [];
  const valid = value.filter(isValidDescriptor);
  if (valid.length !== value.length
    || new Set(valid.map((monitor) => monitor.index)).size !== valid.length
    || new Set(valid.map((monitor) => monitor.identity)).size !== valid.length
    || valid.filter((monitor) => monitor.isEditorMonitor).length !== 1) return [];
  return valid;
};
const initialTarget = (monitors) => monitors.find((monitor) => !monitor.isEditorMonitor) ?? monitors[0];

const fixtures = [
  descriptor(0, { name: "EDITOR", isEditorMonitor: true, physicalWidth: 2560, physicalHeight: 1440 }),
  descriptor(1, { name: "DISPLAY2" }),
  descriptor(2, { name: "DISPLAY3" }),
  descriptor(3, { name: "DISPLAY4" }),
  descriptor(4, { name: "DISPLAY5" }),
];
assert.equal(normalize(fixtures).length, 5, "editor + four sub targets must be accepted in detected order");

// Requirement 1: default is the first non-editor target, independent of any
// old primary flag or numeric index.
assert.equal(initialTarget(fixtures).index, 1, "default target must be the first non-editor display");
assert.notEqual(initialTarget(fixtures).index, fixtures[0].index, "editor display must not be the default when a sub target exists");

// Requirement 2: selecting an editor remains allowed but exposes the warning;
// a single editor-only target remains valid, while ambiguous editor metadata
// fails closed instead of silently selecting an arbitrary output.
const selectedEditor = fixtures[0];
assert.equal(selectedEditor.isEditorMonitor, true, "editor selection must remain selectable");
const singleEditor = [selectedEditor];
assert.equal(normalize(singleEditor).length, 1, "a single editor target must remain valid");
assert.equal(initialTarget(singleEditor).index, 0, "single editor target must be the explicit fallback");
assert.deepEqual(normalize(fixtures.map((monitor) => ({ ...monitor, isEditorMonitor: true }))), [], "multiple editor targets must fail closed");
assert.deepEqual(normalize(fixtures.map((monitor) => ({ ...monitor, isEditorMonitor: false }))), [], "missing editor target must fail closed");
assert.match(panelSource, /data-editor-monitor=\{monitor\.isEditorMonitor \? "true" : undefined\}/);
assert.match(panelSource, /data-video-output-editor-warning/);
assert.match(panelSource, /This editor screen overlaps the Syndocal control surface\. Choose a different screen when possible\./);
assert.doesNotMatch(panelSource, /disabled=\{[^}]*isEditorMonitor/);

// Requirement 3: the add callback remains the native-owned action, with an
// explicit UI note beside it; no frontend confirmation route is introduced.
assert.match(panelSource, /const \[addPending, setAddPending\] = createSignal\(false\)/);
assert.match(panelSource, /const addSelectedDisplayOutput = async \(\) =>/);
assert.match(panelSource, /if \(addPending\(\) \|\| props\.kind !== "Display"\) return/);
assert.match(panelSource, /let addEpoch = 0/);
assert.match(panelSource, /let activeAddFlight: symbol \| null = null/);
assert.match(panelSource, /const invalidateDisplayAdd = \(clearStatus = false\) =>/);
assert.match(panelSource, /addEpoch \+= 1;[\s\S]*setAddDiagnostic\(null\)/);
assert.match(panelSource, /onAddDisplayOutput: \(monitor: VideoDisplayMonitorDescriptor\) => Promise<void>/);
assert.match(setupSource, /onAddDisplayOutput: \(monitor: VideoDisplayMonitorDescriptor\) => Promise<void>/);
assert.match(panelSource, /await props\.onAddDisplayOutput\(monitor\)/);
assert.match(panelSource, /const flight = Symbol\("display-add"\)/);
assert.match(panelSource, /const operationEpoch = \+\+addEpoch/);
assert.match(panelSource, /activeAddFlight = flight/);
assert.match(panelSource, /finally \{[\s\S]*activeAddFlight === flight[\s\S]*setAddPending\(false\);[\s\S]*\}/s);
assert.match(panelSource, /disabled=\{monitorDiscovery\(\) !== "ready" \|\| !selectedMonitor\(\) \|\| addPending\(\)\}/);
assert.match(panelSource, /onClick=\{\(\) => \{ void addSelectedDisplayOutput\(\); \}\}/);
assert.match(panelSource, /disabled=\{monitorDiscovery\(\) !== "ready" \|\| monitors\(\)\.length === 0 \|\| addPending\(\)\}/);
assert.match(panelSource, /const \[addStatus, setAddStatus\] = createSignal<VideoOutputAddStatus \| null>\(null\)/);
assert.match(panelSource, /const \[addDiagnostic, setAddDiagnostic\] = createSignal<VideoOutputAddDiagnostic \| null>\(null\)/);
assert.match(panelSource, /setAddStatus\(\{ kind: "pending", message: "Adding display output…" \}\)/);
assert.match(panelSource, /setAddStatus\(\{ kind: "success", message: "Display output added\." \}\)/);
assert.match(panelSource, /displayAddDiagnosticText\(error\)/);
assert.match(panelSource, /monitorIdentity: monitor\.identity/);
assert.match(panelSource, /width: target\.width/);
assert.match(panelSource, /height: target\.height/);
const addEpochGuards = panelSource.match(/if \(operationEpoch !== addEpoch \|\| props\.kind !== "Display"\) return/g) ?? [];
assert.equal(addEpochGuards.length, 2, "await and catch must both require the captured Add epoch and Display kind");
assert.match(panelSource, /displayAddErrorMessage\(error, loadUiLocale\(\)\)/);
assert.match(localizationSource, /export function displayAddErrorMessage\(/);
assert.match(localizationSource, /export const displayAddDiagnosticMaxLength = 2048/);
assert.match(localizationSource, /export function displayAddDiagnosticText\(error: unknown\): string \| null/);
assert.match(localizationSource, /\.replace\(\/\\s\+\/gu, " "\)/);
assert.match(panelSource, /data-video-output-add-status=\{status\(\)\.kind\}/);
assert.match(panelSource, /data-video-output-add-diagnostic/);
assert.match(panelSource, /<summary>Display add diagnostics<\/summary>/);
assert.match(panelSource, /<dt>Raw error<\/dt>/);
assert.match(panelSource, /<dt>Monitor identity<\/dt>/);
assert.match(panelSource, /<dt>Display dimensions<\/dt>/);
assert.match(panelSource, /videoOutputAddDiagnosticValue/);
assert.match(panelSource, /videoOutputAddDiagnosticCode/);
assert.match(panelSource, /data-video-output-add-diagnostic-clear/);
assert.match(panelSource, /onClick=\{\(\) => setAddDiagnostic\(null\)\}/);
assert.doesNotMatch(panelSource, /data-video-output-add-diagnostic[^>]*open/);
const diagnosticSurfaceStart = panelSource.indexOf("data-video-output-add-diagnostic");
const diagnosticSurfaceEnd = panelSource.indexOf("</details>", diagnosticSurfaceStart);
assert.doesNotMatch(panelSource.slice(diagnosticSurfaceStart, diagnosticSurfaceEnd), /console\.|fetch\(|invokeCommand|XMLHttpRequest/iu);
const diagnosticHelperStart = localizationSource.indexOf("const safeDisplayAddErrorText");
const diagnosticHelperEnd = localizationSource.indexOf("/** Convert AddDisplay failures", diagnosticHelperStart);
assert.ok(diagnosticHelperStart >= 0 && diagnosticHelperEnd > diagnosticHelperStart, "Add diagnostic helper boundary must remain discoverable");
assert.match(localizationSource.slice(diagnosticHelperStart, diagnosticHelperEnd), /safeDisplayAddErrorText/);
assert.match(localizationSource.slice(diagnosticHelperStart, diagnosticHelperEnd), /displayAddAuthorizationPattern/);
assert.match(localizationSource.slice(diagnosticHelperStart, diagnosticHelperEnd), /displayAddSensitiveValuePattern/);
assert.match(localizationSource.slice(diagnosticHelperStart, diagnosticHelperEnd), /displayAddBearerBasicPattern/);
assert.match(localizationSource, /api\(\?:\[_-\]\|\\s\|%20\)\*key/);
assert.match(localizationSource.slice(diagnosticHelperStart, diagnosticHelperEnd), /catch \{\s*return null;\s*\}/s);
assert.doesNotMatch(localizationSource.slice(diagnosticHelperStart, diagnosticHelperEnd), /console\.|fetch\(|XMLHttpRequest/iu);
assert.match(stylesSource, /\.videoOutputAddDiagnosticCode/);
assert.match(stylesSource, /\.videoOutputAddDiagnosticValue/);
assert.match(stylesSource, /videoOutputAddDiagnostic(?:Code|Value)[\s\S]*overflow-wrap: anywhere/);
assert.match(stylesSource, /videoOutputAddDiagnostic(?:Code|Value)[\s\S]*word-break: break-word/);
assert.match(stylesSource, /videoOutputAddDiagnostic(?:Body|Code|Value)[\s\S]*max-width: 100%/);
assert.match(stylesSource, /videoOutputAddDiagnostic(?:Body|Code|Value)[\s\S]*min-width: 0/);
assert.match(localizationSource, /"Clear diagnostics":/);
assert.match(panelSource, /data-video-output-native-dialog-note/);
assert.match(panelSource, /The native display confirmation dialog will appear when this output is added\./);

// The UI-side guard is a local singleflight: two clicks in the same pending
// window must yield one callback, while the guard is released in finally.
const pendingInvokes = [];
let addPending = false;
const addOnce = async () => {
  if (addPending) return;
  addPending = true;
  try {
    pendingInvokes.push("add");
    await Promise.resolve();
  } finally {
    addPending = false;
  }
};
await Promise.all([addOnce(), addOnce()]);
assert.equal(pendingInvokes.length, 1, "double-click must invoke add exactly once");

// A controlled Promise proves that a stale Add cannot commit after an ABA
// kind transition. The old flight stays the singleflight owner until it
// settles, and only its own finally may release the pending guard.
const createDisplayAddEpochHarness = () => {
  let epoch = 0;
  let pending = false;
  let activeFlight = null;
  let kind = "Display";
  let status = null;
  let diagnostic = null;
  let invokes = 0;
  const invalidate = () => {
    epoch += 1;
    diagnostic = null;
    if (!pending) status = null;
  };
  const setKind = (nextKind) => {
    kind = nextKind;
    epoch += 1;
    if (kind !== "Display") {
      diagnostic = null;
      status = null;
    }
  };
  const add = async (result) => {
    if (pending || kind !== "Display") return;
    const flight = Symbol("display-add");
    const capturedEpoch = ++epoch;
    activeFlight = flight;
    pending = true;
    invokes += 1;
    diagnostic = null;
    status = "pending";
    try {
      await result;
      if (capturedEpoch !== epoch || kind !== "Display") return;
      diagnostic = null;
      status = "success";
    } catch {
      if (capturedEpoch !== epoch || kind !== "Display") return;
      diagnostic = "error";
      status = "error";
    } finally {
      if (activeFlight === flight) {
        activeFlight = null;
        pending = false;
        if (capturedEpoch !== epoch || kind !== "Display") status = null;
      }
    }
  };
  return {
    add,
    invalidate,
    setKind,
    get pending() { return pending; },
    get status() { return status; },
    get diagnostic() { return diagnostic; },
    get invokes() { return invokes; },
  };
};
let releaseAba;
const abaResult = new Promise((resolve) => { releaseAba = resolve; });
const abaHarness = createDisplayAddEpochHarness();
const oldAbaFlight = abaHarness.add(abaResult);
assert.equal(abaHarness.pending, true, "Add must remain pending while its controlled Promise is unresolved");
abaHarness.invalidate();
abaHarness.setKind("NdiSender");
abaHarness.setKind("Display");
const blockedAbaFlight = abaHarness.add(Promise.resolve());
await blockedAbaFlight;
assert.equal(abaHarness.invokes, 1, "ABA must not start a second Add while the old flight is pending");
releaseAba();
await oldAbaFlight;
assert.equal(abaHarness.pending, false, "the settled old flight must release its own pending guard");
assert.equal(abaHarness.status, null, "stale ABA success must not commit status");
assert.equal(abaHarness.diagnostic, null, "stale ABA success must not commit diagnostics");
await abaHarness.add(Promise.resolve());
assert.equal(abaHarness.invokes, 2, "a fresh Display Add may start after the old flight settles");
assert.equal(abaHarness.status, "success", "fresh Add must commit after ABA settles");

// Requirement 4: stale DTOs fail closed rather than silently falling back to
// a primary/index-derived target.
const staleDto = { ...fixtures[1] };
delete staleDto.isEditorMonitor;
assert.deepEqual(normalize([staleDto]), [], "missing editor classification must fail closed");
assert.deepEqual(normalize([{ ...fixtures[1], isEditorMonitor: "false" }]), [], "wrong editor classification type must fail closed");
assert.deepEqual(normalize([{ ...fixtures[1], identity: "stale" }]), [], "stale identity shape must fail closed");
assert.match(panelSource, /typeof monitor\.isEditorMonitor === "boolean"/);
assert.match(appSource, /typeof monitor\.isEditorMonitor !== "boolean"/);

// Four sub-target add sequence: each selected identity is passed through as a
// distinct target in detected order, preserving its authoritative dimensions.
const subTargets = fixtures.filter((monitor) => !monitor.isEditorMonitor);
const addCalls = [];
for (const monitor of subTargets) {
  addCalls.push({ identity: monitor.identity, width: monitor.physicalWidth, height: monitor.physicalHeight });
}
assert.deepEqual(addCalls.map((call) => call.identity), subTargets.map((monitor) => monitor.identity));
assert.equal(new Set(addCalls.map((call) => call.identity)).size, 4, "four sub targets must remain distinct across add sequence");
assert.deepEqual(addCalls.map(({ width, height }) => [width, height]), subTargets.map(({ physicalWidth, physicalHeight }) => [physicalWidth, physicalHeight]));

// The backend v2 Add path owns same-owner orphan recovery and Add in one
// durable candidate. The renderer may pass one recoverable orphan authority,
// but must reject ambiguous/mixed candidates before issuing any Add invoke.
const lease = (id, generation) => ({ lease_id: `lease-${id.repeat(16).slice(0, 16)}`, generation });
const selectDisplayAddLease = (statuses) => {
  const matches = statuses.filter((status) =>
    (status.status === "held_active" || status.status === "expired_recoverable" || status.status === "held_orphaned")
    && status.resources.join(",") === "lighting,video");
  if (matches.length !== 1) throw new Error("ambiguous display-add lease");
  return matches[0].authority;
};
const orphanOnly = [{ status: "expired_recoverable", authority: lease("a", 7), resources: ["lighting", "video"] }];
const orphanAuthority = selectDisplayAddLease(orphanOnly);
assert.equal(orphanAuthority.lease_id, "lease-aaaaaaaaaaaaaaaa", "expired orphan must reach the atomic Add adapter");
let addInvokes = 0;
const invokeAtomicAdd = (statuses) => {
  const authority = selectDisplayAddLease(statuses);
  addInvokes += 1;
  return authority;
};
invokeAtomicAdd(orphanOnly);
assert.equal(addInvokes, 1, "expired orphan Add path must invoke canonical Add exactly once");
const foreignLikeAmbiguity = [
  ...orphanOnly,
  { status: "held_active", authority: lease("b", 2), resources: ["lighting", "video"] },
];
assert.throws(() => invokeAtomicAdd(foreignLikeAmbiguity), /ambiguous display-add lease/);
assert.equal(addInvokes, 1, "multiple or foreign-like Both candidates must fail before Add invoke");
assert.throws(() => invokeAtomicAdd([
  { status: "held_orphaned", authority: lease("c", 3), resources: ["video"] },
]), /ambiguous display-add lease/);
assert.equal(addInvokes, 1, "split/subset lease must fail before Add invoke");

// Exercise the production TypeScript parser against the stable camelCase
// backend DTO, including every accepted status and strict fail-closed shapes.
const displayAddOperationId = "syndocal.query.output.display.add.authority.v1";
const displayAddAuthority = { lease_id: "lease-aaaaaaaaaaaaaaaa", generation: 7 };
const queryDisplayAdd = (response) => controllerRuntime.queryDisplayAddLeaseAuthority(
  async (command) => {
    assert.equal(command, "query_display_add_lease_authority_v1");
    return response;
  },
);
for (const status of ["held_active", "expired_recoverable", "held_orphaned"]) {
  const parsed = await queryDisplayAdd({
    operationId: displayAddOperationId,
    status,
    authority: displayAddAuthority,
    resources: ["lighting", "video"],
  });
  assert.equal(parsed.status, status);
  assert.deepEqual(controllerRuntime.selectExactBothLeaseForDisplayAdd(parsed), displayAddAuthority);
}
const unavailable = await queryDisplayAdd({
  operationId: displayAddOperationId,
  status: "unavailable",
  authority: null,
  resources: [],
});
assert.equal(unavailable.authority, null);
assert.throws(() => controllerRuntime.selectExactBothLeaseForDisplayAdd(unavailable), /active or recoverable Both/);
for (const invalid of [
  { operationId: displayAddOperationId, status: "unknown", authority: displayAddAuthority, resources: ["lighting", "video"] },
  { operationId: displayAddOperationId, status: "held_active", authority: null, resources: ["lighting", "video"] },
  { operationId: displayAddOperationId, status: "held_active", authority: displayAddAuthority, resources: [] },
  { operationId: displayAddOperationId, status: "held_active", authority: displayAddAuthority, resources: ["video", "lighting"] },
  { operationId: displayAddOperationId, status: "unavailable", authority: displayAddAuthority, resources: [] },
  { operationId: displayAddOperationId, status: "unavailable", authority: null, resources: ["lighting", "video"] },
  { operationId: displayAddOperationId, status: "held_orphaned", authority: displayAddAuthority, resources: ["lighting", "video"], extra: true },
]) {
  await assert.rejects(queryDisplayAdd(invalid), /Display Add lease authority response was invalid/);
}

const localizedBusyError = localizationRuntime.displayAddErrorMessage(
  new Error("Display Add authority query is busy"),
  "ja",
);
assert.equal(
  localizedBusyError,
  "ディスプレイ出力の権限確認が混み合っています。少し待ってから再試行してください。",
  "busy Add errors must be mapped to Japanese operator copy",
);
assert.doesNotMatch(localizedBusyError, /Display Add authority query is busy/);
const localizedUnknownError = localizationRuntime.displayAddErrorMessage(
  new Error("opaque backend detail: 7f3d"),
  "ja",
);
assert.equal(
  localizedUnknownError,
  "ディスプレイ出力を追加できませんでした。選択した画面と出力状態を確認して再試行してください。",
  "unknown Add errors must use deterministic Japanese fallback",
);
assert.doesNotMatch(localizedUnknownError, /opaque backend detail/);
assert.equal(localizationRuntime.translateUiText("Clear diagnostics", "ja"), "診断情報をクリア");

const diagnosticMaxLength = localizationRuntime.displayAddDiagnosticMaxLength;
assert.equal(diagnosticMaxLength, 2048, "Add diagnostics must keep the explicit 2048-character cap");
assert.equal(
  localizationRuntime.displayAddDiagnosticText(new Error("Display Add authority query is busy")),
  "Display Add authority query is busy",
  "busy Add raw error must remain available in the local diagnostic",
);
assert.equal(
  localizationRuntime.displayAddDiagnosticText(new Error("opaque backend detail: 7f3d")),
  "opaque backend detail: 7f3d",
  "unknown Add raw error must remain available in the local diagnostic",
);
assert.equal(
  localizationRuntime.displayAddDiagnosticText(new Error("  line one\n\tline two  ")),
  "line one line two",
  "Add raw error whitespace must normalize to one bounded line",
);
assert.equal(
  localizationRuntime.displayAddDiagnosticText(new Error(" \n\t ")),
  null,
  "blank Add raw errors must not create an empty diagnostic",
);
const oversizedDiagnostic = "x".repeat(diagnosticMaxLength + 17);
const boundedDiagnostic = localizationRuntime.displayAddDiagnosticText(new Error(oversizedDiagnostic));
assert.equal(boundedDiagnostic.length, diagnosticMaxLength, "oversized Add raw errors must be capped");
assert.equal(boundedDiagnostic, oversizedDiagnostic.slice(0, diagnosticMaxLength), "diagnostic cap must retain the leading raw text exactly");
assert.equal(localizationRuntime.displayAddDiagnosticText(undefined), null, "missing Add errors must not create a diagnostic");
const secretDiagnostics = [
  ["Authorization: Bearer bearer-secret-123", ["bearer-secret-123"]],
  ["authorization=Basic basic-secret-456", ["basic-secret-456"]],
  ["token=token-secret-789&secret:secret-value", ["token-secret-789", "secret-value"]],
  ["credential: credential-value password='password-value' api_key:api-key-value", ["credential-value", "password-value", "api-key-value"]],
  ["https://example.test/path?token=query-token-value&password=query-password-value&ok=1", ["query-token-value", "query-password-value"]],
  ["API Key: spaced-api-key-value", ["spaced-api-key-value"]],
  ["X-API Key: x-api-key-value", ["x-api-key-value"]],
  ["api key=plain-spaced-api-key-value", ["plain-spaced-api-key-value"]],
  ["https://example.test/path?api%20key=query-spaced-api-key-value&ok=1", ["query-spaced-api-key-value"]],
  ["apikey=bare-api-key-value", ["bare-api-key-value"]],
  ["https://example.test/path?apikey=query-bare-api-key-value&ok=1", ["query-bare-api-key-value"]],
  ["Bearer standalone-secret-321", ["standalone-secret-321"]],
  ["Basic: colon-secret-654", ["colon-secret-654"]],
  ['"token":"json-secret-987"', ["json-secret-987"]],
  ["t\u200Boken=hidden-secret-246", ["hidden-secret-246"]],
];
for (const [rawSecretMessage, rawSecrets] of secretDiagnostics) {
  const sanitized = localizationRuntime.displayAddDiagnosticText(new Error(rawSecretMessage));
  assert.ok(sanitized?.includes("[REDACTED]"), "secret-bearing Add diagnostic must expose a redaction marker: " + rawSecretMessage);
  for (const rawSecret of rawSecrets) {
    assert(!sanitized?.includes(rawSecret), "secret-bearing Add diagnostic must not retain the raw value: " + rawSecretMessage);
  }
}
const oversizedSecret = "s".repeat(diagnosticMaxLength + 17);
assert.equal(
  localizationRuntime.displayAddDiagnosticText(new Error("token=" + oversizedSecret)),
  "token=[REDACTED]",
  "secret values must be redacted before the diagnostic length cap",
);
const bidiDiagnostic = localizationRuntime.displayAddDiagnosticText(new Error("left\u202Eright\u200B"));
assert.equal(bidiDiagnostic, "leftright", "bidi and zero-width format controls must be removed from Add diagnostics");
assert.equal(localizationRuntime.displayAddDiagnosticText(new Error("left\u0000right")), "left right", "C0 controls must be made safe in Add diagnostics");
assert.equal(localizationRuntime.displayAddDiagnosticText(new Error("\u202E\u200B")), null, "invisible-only Add errors must not create a diagnostic");
const throwingStringError = { toString() { throw new Error("conversion failed"); } };
assert.equal(localizationRuntime.displayAddDiagnosticText(throwingStringError), null, "throwing String conversion must fail closed");
assert.equal(
  localizationRuntime.displayAddErrorMessage(throwingStringError, "ja"),
  "ディスプレイ出力を追加できませんでした。選択した画面と出力状態を確認して再試行してください。",
  "throwing String conversion must not prevent stable summary rendering",
);
const pendingClearIndex = panelSource.indexOf("setAddPending(true);");
const pendingClearSource = panelSource.slice(pendingClearIndex, panelSource.indexOf("try {", pendingClearIndex));
assert.match(pendingClearSource, /setAddDiagnostic\(null\)/, "starting Add must clear prior diagnostics");
const successBranchStart = panelSource.indexOf("await props.onAddDisplayOutput(monitor);");
const successBranchEnd = panelSource.indexOf("} catch (error)", successBranchStart);
assert.match(panelSource.slice(successBranchStart, successBranchEnd), /setAddDiagnostic\(null\);\s*setAddStatus\(\{ kind: "success"/, "successful Add must clear diagnostics");
assert.match(panelSource, /const kind = props\.kind;[\s\S]*addEpoch \+= 1;[\s\S]*if \(kind === "Display"\) return;[\s\S]*setAddDiagnostic\(null\);[\s\S]*setAddStatus\(null\);/, "switching away from Display must invalidate and clear Add diagnostics and status");
assert.match(panelSource, /onMount\(\(\) => \{[\s\S]*invalidateDisplayAdd\(true\);/, "display discovery reload must invalidate and clear prior Add evidence");
assert.equal((panelSource.match(/invalidateDisplayAdd\(true\)/g) ?? []).length, 3, "discovery start, success, and failure must invalidate Add epoch");
const selectMonitorStart = panelSource.indexOf("const selectMonitor =");
const selectMonitorEnd = panelSource.indexOf("const addSelectedDisplayOutput", selectMonitorStart);
assert.match(panelSource.slice(selectMonitorStart, selectMonitorEnd), /invalidateDisplayAdd\(\)/, "changing monitor must invalidate and clear diagnostics");

// Execute the production Add adapter end-to-end with a deterministic Tauri
// invoke seam. This exercises query -> authority fence -> canonical Add v2,
// rather than only checking source text. The receipt shape is intentionally
// complete so the production receipt parser is part of the gate.
const runtimeLease = { lease_id: "lease-aaaaaaaaaaaaaaaa", generation: 7 };
const runtimeFence = {
  process_incarnation: 1,
  session_incarnation: 2,
  project_epoch: 0,
  project_revision: 0,
  project_checkpoint_hash: "c".repeat(64),
  project_publication_generation: 0,
  output_epoch: 1,
  output_generation: 1,
  safety_blackout_epoch: 1,
  safety_blackout_generation: 1,
};
const runtimeAuthorityResponse = {
  operation_id: "syndocal.query.output.control.authority.v1",
  fence: runtimeFence,
};
const runtimeAddAction = (lease = runtimeLease) => ({
  kind: "add_display",
  spec: {
    label: "Display 2",
    monitor_identity: hash("d"),
    monitor_index: 1,
    width: 1920,
    height: 1080,
    fullscreen: true,
  },
  lease,
});
const runtimeAddReceipt = (requestId) => ({
  type: "receipt",
  receipt: {
    operation_id: "syndocal.output.display.add.v2",
    request_id: requestId,
    shape_sha256: "a".repeat(64),
    argument_fingerprint: "b".repeat(64),
    audit_sequence: 8,
    fence_before: runtimeFence,
    fence_after: { ...runtimeFence, output_generation: runtimeFence.output_generation + 1 },
    outcome: "applied",
    lease_result: {
      authority: { ...runtimeLease, generation: runtimeLease.generation + 1 },
      resources: ["lighting", "video"],
      phase: "held_active",
      outcome: "authorized",
      audit_sequence: 8,
      changes: [{
        lease_id: runtimeLease.lease_id,
        before_generation: runtimeLease.generation,
        after_generation: runtimeLease.generation + 1,
        before_resources: ["lighting", "video"],
        after_resources: ["lighting", "video"],
        before_phase: "held_active",
        after_phase: "held_active",
      }],
    },
  },
});
const runtimeCalls = [];
let runtimeAddInvokes = 0;
const runtimeInvoke = async (command, args) => {
  runtimeCalls.push(command);
  if (command === "query_display_add_lease_authority_v1") {
    return {
      operationId: displayAddOperationId,
      status: "held_active",
      authority: runtimeLease,
      resources: ["lighting", "video"],
    };
  }
  if (command === "query_output_control_authority_v1") return runtimeAuthorityResponse;
  if (command === "add_display_output_v2") {
    runtimeAddInvokes += 1;
    return runtimeAddReceipt(args.request.request_id);
  }
  throw new Error(`unexpected Add invoke: ${command}`);
};
const runtimeQuery = await controllerRuntime.queryDisplayAddLeaseAuthority(runtimeInvoke);
await controllerRuntime.executeDisplayAddOutputControl(
  runtimeInvoke,
  runtimeAddAction(),
  runtimeQuery,
);
assert.equal(runtimeAddInvokes, 1, "valid Add must invoke add_display_output_v2 exactly once");
assert.deepEqual(
  runtimeCalls,
  [
    "query_display_add_lease_authority_v1",
    "query_output_control_authority_v1",
    "add_display_output_v2",
  ],
  "valid Add must not fall back to the public lease query or pre-enable",
);
assert(!runtimeCalls.includes("query_output_lease_authority_v1"), "Add must not invoke the old public lease query");
assert(!runtimeCalls.includes("enable_output_control_v2"), "Add must not invoke a separate Enable mutation");

const rejectedAddRun = async (queryResponse, action = runtimeAddAction()) => {
  const calls = [];
  let addInvokes = 0;
  const invoke = async (command, args) => {
    calls.push(command);
    if (command === "query_display_add_lease_authority_v1") return queryResponse;
    if (command === "query_output_control_authority_v1") return runtimeAuthorityResponse;
    if (command === "add_display_output_v2") {
      addInvokes += 1;
      return runtimeAddReceipt(args.request.request_id);
    }
    throw new Error(`unexpected rejected Add invoke: ${command}`);
  };
  try {
    const parsed = await controllerRuntime.queryDisplayAddLeaseAuthority(invoke);
    await controllerRuntime.executeDisplayAddOutputControl(invoke, action, parsed);
  } catch {
    // Expected fail-closed result; assertions below prove no mutation invoke.
  }
  return { calls, addInvokes };
};
const unavailableRun = await rejectedAddRun({
  operationId: displayAddOperationId,
  status: "unavailable",
  authority: null,
  resources: [],
});
assert.equal(unavailableRun.addInvokes, 0, "unavailable authority must not mutate output");
assert.deepEqual(unavailableRun.calls, ["query_display_add_lease_authority_v1"]);
const malformedRun = await rejectedAddRun({
  operationId: displayAddOperationId,
  status: "held_active",
  authority: runtimeLease,
  resources: ["lighting", "video"],
  foreign_owner: true,
});
assert.equal(malformedRun.addInvokes, 0, "extra/foreign-like DTO keys must not mutate output");
assert.deepEqual(malformedRun.calls, ["query_display_add_lease_authority_v1"]);
const foreignLeaseRun = await rejectedAddRun(
  {
    operationId: displayAddOperationId,
    status: "held_active",
    authority: runtimeLease,
    resources: ["lighting", "video"],
  },
  runtimeAddAction({ lease_id: "lease-bbbbbbbbbbbbbbbb", generation: 7 }),
);
assert.equal(foreignLeaseRun.addInvokes, 0, "authority/action mismatch must not mutate output");
assert.deepEqual(
  foreignLeaseRun.calls,
  ["query_display_add_lease_authority_v1"],
  "foreign-like mismatch must fail before fence or Add",
);

for (const monitor of fixtures) {
  const target = targetSummary(monitor);
  assert.equal(target.width, monitor.physicalWidth, `${monitor.name} must use detected width`);
  assert.equal(target.height, monitor.physicalHeight, `${monitor.name} must use detected height`);
}
const windowsDeviceName = { ...fixtures[1], name: "\\\\.\\DISPLAY2" };
assert.equal(targetSummary(windowsDeviceName).label, "Display 2", "Windows device syntax must not leak into saved label");

assert.match(panelSource, /videoDisplayTargetSummary/);
assert.match(panelSource, /monitor\.index <= 255/);
assert.match(appSource, /videoDisplayTargetSummary/);
assert.match(appSource, /monitor\.index > 255/);
const addFunctionStart = appSource.indexOf("const addDisplayVideoOutput = async");
const addFunctionEnd = appSource.indexOf("const removeVideoOutput = async", addFunctionStart);
assert.ok(addFunctionStart >= 0 && addFunctionEnd > addFunctionStart, "display add function boundary must remain discoverable");
const addFunctionSource = appSource.slice(addFunctionStart, addFunctionEnd);
assert.match(appSource, /const staleDisplayAddRecoveryMessages = new Set\(\[/);
assert.match(appSource, /const isStaleDisplayAddRecoveryMessage = \(text: string\) =>/);
assert.match(appSource, /const displayAddPostCommitRefreshResult = \(/);
assert.match(appSource, /const finishDisplayAddPostCommitRefresh = \(/);
assert.match(addFunctionSource, /queryDisplayAddLeaseAuthority\(invoke\)/);
assert.match(addFunctionSource, /selectExactBothLeaseForDisplayAdd\(displayAddAuthority\)/);
assert.match(addFunctionSource, /executeDisplayAddOutputControl\(/);
const executeDisplayAddIndex = addFunctionSource.indexOf("await executeDisplayAddOutputControl(");
const refreshDisplayAddIndex = addFunctionSource.indexOf("await refreshSnapshotAndVideoOutputRenderPlans();");
const finishDisplayAddIndex = addFunctionSource.indexOf("finishDisplayAddPostCommitRefresh(");
assert.ok(executeDisplayAddIndex >= 0, "canonical Add execution must remain in the Add callback");
assert.ok(refreshDisplayAddIndex > executeDisplayAddIndex, "post-commit refresh must follow terminal Add execution");
assert.ok(finishDisplayAddIndex > refreshDisplayAddIndex, "refresh result must be classified after refresh settles");
assert.match(addFunctionSource, /const statusBeforeRefresh = appStatus\(\)\.text/);
assert.match(addFunctionSource, /let refreshError: unknown/);
assert.match(addFunctionSource, /catch \(error\) \{[\s\S]*refreshError = error;[\s\S]*\}/);
assert.doesNotMatch(addFunctionSource, /queryOutputLeaseAuthority\(invoke\)/, "display Add must not fall back to the public lease query");
assert.doesNotMatch(addFunctionSource, /enableOutput|enable_output_control_v2/, "frontend add must not publish a separate enable before atomic backend add");
assert.doesNotMatch(addFunctionSource, /setMessage\(/, "display add errors must be rendered beside Add, not only in the global status line");

// Execute the exact extracted post-commit classifier rather than mirroring it
// in this checker. The fixture models the production order: canonical Add
// resolves first, refresh settles second, then the UI classifies the result.
const runCommittedDisplayAddFixture = async ({ mutation, refresh, before, after }) => {
  let refreshCalls = 0;
  await mutation();
  let refreshError;
  try {
    refreshCalls += 1;
    await refresh();
  } catch (error) {
    refreshError = error;
  }
  return {
    refreshCalls,
    panelStatus: "success",
    result: displayAddHelperRuntime.displayAddPostCommitRefreshResult(before, after, refreshError),
  };
};
const staleTransactionMessage = "Project transaction is active; retry after Display output publication";
const stalePublicationMessage = "Error: Display output publication is active; retry the output ownership transition";
const staleRejected = await runCommittedDisplayAddFixture({
  mutation: async () => {},
  refresh: async () => { throw new Error(staleTransactionMessage); },
  before: staleTransactionMessage,
  after: staleTransactionMessage,
});
assert.equal(staleRejected.panelStatus, "success", "known stale refresh rejection must not turn Add into a panel error");
assert.equal(staleRejected.refreshCalls, 1);
assert.deepEqual(staleRejected.result, { kind: "clear" }, "known transaction refresh rejection must clear stale global status");
const staleAfterRefresh = await runCommittedDisplayAddFixture({
  mutation: async () => {},
  refresh: async () => {},
  before: stalePublicationMessage,
  after: stalePublicationMessage,
});
assert.deepEqual(staleAfterRefresh.result, { kind: "clear" }, "stale status observed after refresh must clear after settle");
const unknownRefresh = await runCommittedDisplayAddFixture({
  mutation: async () => {},
  refresh: async () => { throw new Error("backend refresh unavailable"); },
  before: "Ready",
  after: "Ready",
});
assert.equal(unknownRefresh.panelStatus, "success", "unknown refresh failure must not be reported as Add mutation failure");
assert.equal(unknownRefresh.result.kind, "refresh_pending");
assert.match(unknownRefresh.result.message, /Display output added; refresh pending: Error: backend refresh unavailable/);
const unknownRefreshObserved = await runCommittedDisplayAddFixture({
  mutation: async () => {},
  refresh: async () => {},
  before: "Ready",
  after: "Error: backend refresh unavailable",
});
assert.equal(unknownRefreshObserved.result.kind, "refresh_pending", "caught refresh errors surfaced by the controller must remain truthful");
const unrelatedBeforeRefresh = await runCommittedDisplayAddFixture({
  mutation: async () => {},
  refresh: async () => {},
  before: "Project save failed",
  after: "Project save failed",
});
assert.deepEqual(unrelatedBeforeRefresh.result, { kind: "preserve" }, "unrelated existing status must not be hidden");
let fixtureRefreshCalls = 0;
await assert.rejects(
  runCommittedDisplayAddFixture({
    mutation: async () => { throw new Error("native Add failed after confirmation"); },
    refresh: async () => { fixtureRefreshCalls += 1; },
    before: "Ready",
    after: "Ready",
  }),
  /native Add failed after confirmation/,
  "mutation failure must reject the outer Add callback",
);
assert.equal(fixtureRefreshCalls, 0, "mutation failure must not run post-commit refresh classification");
assert.match(controllerSource, /export function selectExactBothLeaseForDisplayAdd\(/);
assert.match(controllerSource, /export async function queryDisplayAddLeaseAuthority\(/);
assert.match(controllerSource, /query_display_add_lease_authority_v1/);
assert.match(controllerSource, /operationId/);
assert.match(controllerSource, /hasExactKeys\(value, \["operationId", "status", "authority", "resources"\]\)/);
assert.match(controllerSource, /export async function executeDisplayAddOutputControl\(/);
assert.match(controllerSource, /skipPublicLeaseQuery: true/);
assert.match(controllerSource, /value\.status !== "held_active"[\s\S]*value\.status !== "expired_recoverable"[\s\S]*value\.status !== "held_orphaned"/);
assert.match(controllerSource, /Exactly one active or recoverable Both output lease is required/);
assert.match(controllerSource, /action\.kind === "add_display"[\s\S]*selected\[0\]\.status !== "held_orphaned"/);
assert.match(panelSource, /const initialVideoDisplayMonitor/);
assert.match(panelSource, /monitors\.find\(\(monitor\) => !monitor\.isEditorMonitor\)/);
assert.doesNotMatch(panelSource, /find\(\(monitor\) => monitor\.primary\)/);
assert.doesNotMatch(panelSource, /props\.monitorId\)/);
assert.match(panelSource, /selectedMonitorIdentity\(\) \?\? ""/);
assert.match(panelSource, /value=\{monitor\.identity\}/);
assert.match(appSource, /selectOnlyActiveOutputLease\(leaseQuery, \["lighting", "video"\]\)/);
const invokeTuple = [...invokeSource.matchAll(/^\s+"([^"]+)",$/gm)].map((match) => match[1]);
const invokeManifest = JSON.parse(manifestSource);
assert.deepEqual(invokeTuple, [...invokeTuple].sort(), "frontend invoke tuple must remain sorted");
assert.deepEqual(invokeManifest, [...invokeManifest].sort(), "frontend invoke manifest must remain sorted");
assert.ok(invokeTuple.includes("query_display_add_lease_authority_v1"), "new display Add query missing from typed tuple");
assert.ok(invokeManifest.includes("query_display_add_lease_authority_v1"), "new display Add query missing from manifest");
assert.match(appSource, /label: target\.label/);
assert.match(appSource, /width: target\.width[\s\S]*height: target\.height[\s\S]*fullscreen: target\.fullscreen/);

// Existing native identity/dimension fences remain part of this UI contract;
// this checker deliberately does not edit or invoke the backend.
assert.match(backendSource, /async fn list_video_display_monitors\(\s*window: WebviewWindow,\s*\)/s);
assert.match(backendSource, /QueryDisplayConfig\(/);
assert.match(backendSource, /DISPLAYCONFIG_TARGET_DEVICE_NAME/);
assert.match(backendSource, /Display output monitor identity is stale/);
assert.match(backendSource, /VIDEO_DISPLAY_MONITOR_IDENTITY_DOMAIN: &str = "syndocal\.display-monitor\.v2"/);
assert.match(backendSource, /fn native_video_output_logical_size\([\s\S]*?physical_width: u32[\s\S]*?scale_factor: f64/);
assert.match(backendSource, /native_video_output_logical_size\(output\.width, output\.height, monitor\.scale_factor\)/);
assert.match(backendSource, /fn activate_native_video_output_window_after_show\([\s\S]*?set_fullscreen\(true\)[\s\S]*?wait_for_native_video_output_physical_size/);
assert.match(backendSource, /fn native_video_output_activation_geometry\([\s\S]*?PhysicalSize \{[\s\S]*?width: u32,[\s\S]*?height: u32,[\s\S]*?\}/);
assert.match(backendSource, /fn wait_for_native_video_output_physical_size_with<[\s\S]*?did not converge/);
assert.match(backendSource, /set_size\(tauri::PhysicalSize::new\([\s\S]*?output\.width/);
assert.match(backendSource, /fn native_video_output_render_extent\([\s\S]*?window\.inner_size\(\)/);
assert.doesNotMatch(backendSource, /let fixed_size = initial_size_override/);
assert.match(backendSource, /native_video_output_window_builder_dpi_matrix_round_trips_physical_extent/);
assert.match(backendSource, /native_video_output_nonfullscreen_activation_uses_physical_size/);
assert.match(backendSource, /native_video_output_post_show_convergence_retries_then_accepts_physical_extent/);
assert.match(backendSource, /native_video_output_post_show_convergence_times_out_and_fails_closed/);
assert.match(backendSource, /native_video_output_topology_change_after_hidden_staging_is_rejected/);
assert.match(backendSource, /native_output_qa_driver_uses_canonical_v2_add_path_for_four_sub_displays/);
assert.doesNotMatch(
  backendSource,
  /#\[tauri::command\]\s*(?:async\s+)?fn\s+native_output_qa_driver/,
  "the native output QA driver must remain test-only and unreachable as a Tauri command",
);
assert.match(localizationSource, /"This editor screen overlaps the Syndocal control surface\. Choose a different screen when possible\."/);
assert.match(localizationSource, /"· Editor screen": "· エディター画面"/);
assert.match(localizationSource, /"The native display confirmation dialog will appear when this output is added\."/);
assert.match(localizationSource, /"Adding display output…":/);
assert.match(localizationSource, /"Display output added\.":/);
assert.match(stylesSource, /\.videoOutputQuickCreate/);
assert.match(stylesSource, /grid-template-columns: minmax\(0, 1fr\) auto/);
assert.match(stylesSource, /\.videoOutputQuickCreate > button[\s\S]*min-height: 36px/);

console.log("video display target contract: PASS (exactly one editor, first non-editor default, editor warning, stale DTO fail-closed, four-sub add sequence, local add singleflight, adjacent status, bounded/redacted local Add diagnostics with raw-error retention, bidi removal, safe conversion, clear lifecycle/button, strict Add authority parser, executable query-to-Add-v2 exactly-once flow, post-commit refresh lifecycle, zero-mutation negatives, no pre-enable/fallback, Japanese Add errors, native dialog notice, localized 1280-safe surface)");
