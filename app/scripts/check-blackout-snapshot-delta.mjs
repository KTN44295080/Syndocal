import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/engineSnapshotLiveState.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
});
const { mergeEngineSnapshotSyncResponse } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`
);
const current = {
  blackout: true, authored_blackout: true, safety_blackout_engaged: true,
  video: { blackout: false, media_assets: [] },
};
for (const field of ["authored_blackout", "safety_blackout_engaged"]) {
  let previous = current;
  for (const enabled of [false, true]) {
    const merged = mergeEngineSnapshotSyncResponse(previous, {
      revision: 2, delta: { [field]: enabled },
    });
    assert.equal(merged[field], enabled, `${field} must apply even when effective blackout is unchanged`);
    assert.equal(merged.blackout, true);
    assert.strictEqual(merged.video, current.video, "a lighting/S0 delta must not clone or alter the video section");
    const other = field === "authored_blackout" ? "safety_blackout_engaged" : "authored_blackout";
    assert.equal(merged[other], true, "the independent blackout cause must survive a partial delta");
    const unchanged = mergeEngineSnapshotSyncResponse(merged, { revision: 3, delta: {} });
    assert.deepEqual(unchanged, merged, "omitted fields must preserve the last accepted state");
    previous = merged;
  }
}
console.log("blackout snapshot delta: independent authored/S0 changes and video reference preservation passed");
