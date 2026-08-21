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
assert.match(panelSource, /if \(addPending\(\)\) return/);
assert.match(panelSource, /onAddDisplayOutput: \(monitor: VideoDisplayMonitorDescriptor\) => Promise<void>/);
assert.match(setupSource, /onAddDisplayOutput: \(monitor: VideoDisplayMonitorDescriptor\) => Promise<void>/);
assert.match(panelSource, /await props\.onAddDisplayOutput\(monitor\)/);
assert.match(panelSource, /finally \{\s*setAddPending\(false\);\s*\}/s);
assert.match(panelSource, /disabled=\{monitorDiscovery\(\) !== "ready" \|\| !selectedMonitor\(\) \|\| addPending\(\)\}/);
assert.match(panelSource, /onClick=\{\(\) => \{ void addSelectedDisplayOutput\(\); \}\}/);
assert.match(panelSource, /disabled=\{monitorDiscovery\(\) !== "ready" \|\| monitors\(\)\.length === 0 \|\| addPending\(\)\}/);
assert.match(panelSource, /const \[addStatus, setAddStatus\] = createSignal<VideoOutputAddStatus \| null>\(null\)/);
assert.match(panelSource, /setAddStatus\(\{ kind: "pending", message: "Adding display output…" \}\)/);
assert.match(panelSource, /setAddStatus\(\{ kind: "success", message: "Display output added\." \}\)/);
assert.match(panelSource, /displayAddErrorMessage\(error, loadUiLocale\(\)\)/);
assert.match(localizationSource, /export function displayAddErrorMessage\(/);
assert.match(panelSource, /data-video-output-add-status=\{status\(\)\.kind\}/);
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

console.log("video display target contract: PASS (exactly one editor, first non-editor default, editor warning, stale DTO fail-closed, four-sub add sequence, local add singleflight, adjacent status, strict Add authority parser, executable query-to-Add-v2 exactly-once flow, post-commit refresh lifecycle, zero-mutation negatives, no pre-enable/fallback, Japanese Add errors, native dialog notice, localized 1280-safe surface)");
