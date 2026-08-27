import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const geometrySource = await readFile(new URL("../src/stageGeometry.ts", import.meta.url), "utf8");
const transpiledGeometry = ts.transpileModule(
  geometrySource.replace(
    'import { clamp01 } from "./numericHelpers";',
    "const clamp01 = (value) => Math.max(0, Math.min(1, value));",
  ),
  {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
    fileName: "stageGeometry.ts",
  },
);
const geometry = await import(
  `data:text/javascript;base64,${Buffer.from(transpiledGeometry.outputText).toString("base64")}`
);
const {
  mappingStageSvgDeltaToWorld,
  mappingStageSvgFrame,
  mappingStageSvgPointToWorld,
  mappingStageWorldToSvgPoint,
} = geometry;

const rectangularBounds = { minX: -120, maxX: 120, minZ: -30, maxZ: 30 };
const frame = mappingStageSvgFrame(rectangularBounds);
assert.equal(Number(frame.worldToSvgScale.toFixed(6)), 0.333333);

const origin = mappingStageWorldToSvgPoint(0, 0, rectangularBounds);
assert.deepEqual(origin, { x: 50, z: 50 });

const worldPoint = { x: 87.5, z: -12.25 };
const svgPoint = mappingStageWorldToSvgPoint(worldPoint.x, worldPoint.z, rectangularBounds);
const roundTrip = mappingStageSvgPointToWorld(svgPoint.x, svgPoint.z, rectangularBounds);
assert.ok(Math.abs(roundTrip.x - worldPoint.x) < 1e-9);
assert.ok(Math.abs(roundTrip.z - worldPoint.z) < 1e-9);

const fiveMetresX = mappingStageWorldToSvgPoint(5, 0, rectangularBounds).x - origin.x;
const fiveMetresZ = mappingStageWorldToSvgPoint(0, 5, rectangularBounds).z - origin.z;
assert.ok(Math.abs(fiveMetresX - fiveMetresZ) < 1e-9, "X/Z grid pitch must share one physical scale");

const worldDelta = mappingStageSvgDeltaToWorld(fiveMetresX, fiveMetresZ, rectangularBounds);
assert.ok(Math.abs(worldDelta.x - 5) < 1e-9);
assert.ok(Math.abs(worldDelta.z - 5) < 1e-9);

const shellSource = await readFile(
  new URL("../src/components/MappingEditableStageShell.tsx", import.meta.url),
  "utf8",
);
assert.match(shellSource, /data-mapping-grid-layer="minor"/);
assert.match(shellSource, /data-mapping-grid-layer="major"/);
assert.doesNotMatch(shellSource, /fill="url\(#stage-grid-minor\)"/);

const interactionSource = await readFile(
  new URL("../src/createMappingInteractionController.ts", import.meta.url),
  "utf8",
);
assert.doesNotMatch(
  interactionSource,
  /Promise\.all\(movedFixtures/,
  "multi-fixture drag persistence must not race strict transactions",
);
assert.match(interactionSource, /applyMappingFixtureTransformBatch\(\{/);

const confirmationSource = await readFile(
  new URL("../src/fixtureTransformConfirmation.ts", import.meta.url),
  "utf8",
);
const transpiledConfirmation = ts.transpileModule(confirmationSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "fixtureTransformConfirmation.ts",
});
const confirmationModuleUrl =
  `data:text/javascript;base64,${Buffer.from(transpiledConfirmation.outputText).toString("base64")}`;
const confirmation = await import(confirmationModuleUrl);
const authoritativeFixture = {
  position: { x: 10, y: 2.5, z: -4 },
  rotation: { pitch: 3, yaw: 45, roll: -2 },
};
assert.equal(confirmation.fixtureTransformMatchesExpectation(authoritativeFixture, {
  position: { x: 10, y: 2.5, z: -4 },
  rotation: { pitch: 3, yaw: 45, roll: -2 },
}), true);
assert.equal(confirmation.fixtureTransformMatchesExpectation(authoritativeFixture, {
  rotation: { pitch: 3, yaw: 46, roll: -2 },
}), false, "a stale yaw snapshot must not acknowledge persistence");
assert.equal(confirmation.fixtureTransformMatchesExpectation(authoritativeFixture, {
  position: { x: 10.009, y: 2.5, z: -4 },
}), true, "persisted hundredth-unit rounding remains accepted");
assert.equal(confirmation.fixtureTransformMatchesExpectation(undefined, {
  position: { x: 10, y: 2.5, z: -4 },
}), false, "a missing refreshed fixture must fail closed");

const batchSource = (await readFile(
  new URL("../src/mappingFixtureTransformBatch.ts", import.meta.url),
  "utf8",
)).replace(
  /from "\.\/fixtureTransformConfirmation";/,
  `from ${JSON.stringify(confirmationModuleUrl)};`,
);
const transpiledBatch = ts.transpileModule(batchSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "mappingFixtureTransformBatch.ts",
});
const batch = await import(
  `data:text/javascript;base64,${Buffer.from(transpiledBatch.outputText).toString("base64")}`
);
const fixture = (id, x = 0) => ({
  id,
  label: `Fixture ${id}`,
  position: { x, y: 0, z: 0 },
  rotation: { pitch: 0, yaw: 0, roll: 0 },
});
const transforms = [1, 2, 3].map((id) => ({
  fixture: fixture(id),
  update: { position: { x: id * 10, y: 0, z: 0 } },
}));

const partialInvokes = [];
let partialRefreshes = 0;
const partialMessages = [];
assert.equal(await batch.applyMappingFixtureTransformBatch({
  transforms,
  setFixtureTransform: async (candidate) => {
    partialInvokes.push(candidate.id);
    return candidate.id !== 2;
  },
  refreshSnapshot: async () => {
    partialRefreshes += 1;
    return { fixtures: [fixture(1, 10), fixture(2), fixture(3)] };
  },
  setMessage: (message) => partialMessages.push(message),
}), false);
assert.deepEqual(partialInvokes, [1, 2], "bulk persistence must stop after the first invoke failure");
assert.equal(partialRefreshes, 1, "bulk persistence must perform one authoritative refresh");
assert.match(partialMessages[0], /earlier fixtures may already have changed/);

const mismatchMessages = [];
assert.equal(await batch.applyMappingFixtureTransformBatch({
  transforms: transforms.slice(0, 2),
  setFixtureTransform: async () => true,
  refreshSnapshot: async () => ({ fixtures: [fixture(1, 10), fixture(2)] }),
  setMessage: (message) => mismatchMessages.push(message),
}), false, "one stale refreshed fixture must reject the whole success claim");
assert.match(mismatchMessages[0], /Fixture 2/);

const successMessages = [];
assert.equal(await batch.applyMappingFixtureTransformBatch({
  transforms: transforms.slice(0, 2),
  setFixtureTransform: async () => true,
  refreshSnapshot: async () => ({ fixtures: [fixture(1, 10), fixture(2, 20)] }),
  setMessage: (message) => successMessages.push(message),
}), true);
assert.deepEqual(successMessages, []);

const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(appSource, /const setFixtureTransform = async[\s\S]*?: Promise<boolean>/);
assert.match(appSource, /await invoke\("set_fixture_transform"[\s\S]*?return true;/);
assert.match(appSource, /const refreshed = await refreshSnapshot\(\)/);
assert.match(appSource, /refreshed\?\.fixtures\.find[\s\S]*?fixtureTransformMatchesExpectation\(confirmed, next\)/);
assert.match(appSource, /Could not confirm fixture transform/);
assert.match(appSource, /catch \(error\)[\s\S]*?setMessage\(String\(error\)\);[\s\S]*?return false;/);
assert.match(
  appSource,
  /const stageOrigin2d = createMemo\(\(\) => mappingStageWorldToSvgPoint[\s\S]*?const cueCapturePreview = createMemo[\s\S]*?const point = mappingStageWorldToSvgPoint/,
  "Cue Capture origin and fixture points must use the same one-scale mapping frame",
);

const layoutSource = await readFile(
  new URL("../src/createMappingLayoutController.ts", import.meta.url),
  "utf8",
);
assert.match(layoutSource, /applyMappingFixtureTransformBatch\(\{/);

console.log("Mapping Stage geometry checks passed (coordinate, grid-layer, and authoritative drag contracts).\n");
