import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const compileTs = (source, fileName) => ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName,
}).outputText;

const dataModule = (source) => `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const layoutSource = await readFile(new URL("../src/workspaceLayoutStorage.ts", import.meta.url), "utf8");
const layoutUrl = dataModule(compileTs(layoutSource, "workspaceLayoutStorage.ts"));
const profileSource = (await readFile(new URL("../src/workspaceProfiles.ts", import.meta.url), "utf8"))
  .replace('from "./workspaceLayoutStorage"', `from "${layoutUrl}"`);
const workspace = await import(dataModule(compileTs(profileSource, "workspaceProfiles.ts")));
const operatorSource = await readFile(new URL("../src/operatorPolicy.ts", import.meta.url), "utf8");
const operator = await import(dataModule(compileTs(operatorSource, "operatorPolicy.ts")));
const tauriSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const protocolSource = await readFile(new URL("../../crates/protocol/src/lib.rs", import.meta.url), "utf8");
const operationsSource = await readFile(new URL("../src/components/WorkspaceOperationsMenu.tsx", import.meta.url), "utf8");
const overlaySource = await readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8");

const baseLayout = {
  workspace_tab: "control",
  setup_sub_tab: "patch",
  control_mode: "live",
  timeline_desk_surface: "show",
  timeline_context_drawer: "none",
  edit_desk_surface: "attributes",
  control_category: "dimmer",
  top_split_ratio: 0.6,
  lower_split_ratio: 0.4,
  selections_drawer_open: false,
};
const placement = { pane: "timeline", x: -1920, y: 0, width: 1280, height: 720, maximized: false };
const parsed = workspace.namedWorkspacesFromUnknown([{
  id: "workspace-123456",
  name: "FOH",
  layout: baseLayout,
  pane_windows: [placement],
}]);
assert.equal(parsed.length, 1);
assert.deepEqual(parsed[0].pane_windows, [placement]);
assert.equal(workspace.namedWorkspacesFromUnknown([{ ...parsed[0], pane_windows: [placement, placement] }]).length, 0);
assert.equal(workspace.namedWorkspacesFromUnknown([{ ...parsed[0], pane_windows: [{ ...placement, width: 100 }] }]).length, 0);
assert.equal(workspace.namedWorkspacesFromUnknown([{ ...parsed[0], pane_windows: [{ ...placement, pane: "unknown" }] }]).length, 0);
assert.equal(workspace.namedWorkspacesFromUnknown(Array.from({ length: 20 }, (_, index) => ({
  ...parsed[0],
  id: `workspace-${String(index).padStart(8, "0")}`,
}))).length, 16);
const updated = workspace.upsertNamedWorkspace(parsed, "FOH", { ...baseLayout, control_mode: "mixer" }, []);
assert.equal(updated.length, 1);
assert.equal(updated[0].layout.control_mode, "mixer");
assert.throws(() => workspace.upsertNamedWorkspace([], "", baseLayout, []), /1 to 48/);

const policy = await operator.createOperatorPolicy("correct horse battery staple", "Partial", true);
assert.equal(operator.operatorPolicyFromUnknown(policy)?.lock_mode, "Partial");
assert.equal(await operator.verifyOperatorPassword(policy, "correct horse battery staple"), true);
assert.equal(await operator.verifyOperatorPassword(policy, "incorrect password"), false);
assert.doesNotMatch(JSON.stringify(policy), /correct horse|password/i);
assert.equal(operator.operatorPolicyFromUnknown({ ...policy, credential: { ...policy.credential, iterations: 99 } }), null);
assert.equal(operator.operatorPolicyFromUnknown({ ...policy, credential: { ...policy.credential, verifier_b64: "AA==" } }), null);
assert.equal(operator.operatorCommandAllowed(null, "patch_fixture", true), true);
assert.equal(operator.operatorCommandAllowed("Partial", "trigger_cue", false), true);
assert.equal(operator.operatorCommandAllowed("Partial", "patch_fixture", true), false);
assert.equal(operator.operatorCommandAllowed("Full", "trigger_cue", false), false);
assert.equal(operator.operatorCommandAllowed("Full", "set_all_blackout", false), true);

assert.match(protocolSource, /skip_serializing_if = "Option::is_none"[\s\S]*?operator_policy/);
assert.match(tauriSource, /capture_pane_window_placements/);
assert.match(tauriSource, /pane_placement_intersects_monitor/);
assert.match(tauriSource, /Templates are reusable creation aids, not credential carriers/);
assert.match(operationsSource, /Local to this device/);
assert.match(operationsSource, /Show-operation guard, not operating-system security/);
assert.match(overlaySource, /Emergency blackout controls/);

console.log("workspace and operator policy: 27 assertions passed");
