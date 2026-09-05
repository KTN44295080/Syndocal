import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const compiled = new Map();
const compile = async (url) => {
  if (compiled.has(url.href)) return compiled.get(url.href);
  let code = ts.transpileModule(await readFile(url, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
    fileName: url.pathname,
  }).outputText;
  const imports = [...code.matchAll(/from\s+"(\.[^"]+)"/g)];
  for (const match of imports) {
    const specifier = match[1];
    const child = new URL(specifier.endsWith(".ts") ? specifier : `${specifier}.ts`, url);
    code = code.replace(`from "${specifier}"`, `from "${await compile(child)}"`);
  }
  const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
  compiled.set(url.href, dataUrl);
  return dataUrl;
};

const { createMappingInteractionController } = await import(
  await compile(new URL("../src/createMappingInteractionController.ts", import.meta.url)),
);

const bounds = { minX: -10, maxX: 10, minZ: -10, maxZ: 10 };
const fixtureWithYaw = (yaw = 20) => ({
  id: 1,
  label: "Fixture 1",
  position: { x: 0, y: 0, z: 0 },
  rotation: { pitch: 0, yaw, roll: 0 },
});

const createSvg = () => {
  const captured = new Set();
  return {
    ownerSVGElement: null,
    getScreenCTM: () => null,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 100, height: 100 }),
    setPointerCapture: (pointerId) => captured.add(pointerId),
    hasPointerCapture: (pointerId) => captured.has(pointerId),
    releasePointerCapture: (pointerId) => captured.delete(pointerId),
  };
};

const createHarness = () => {
  let fixtures = [fixtureWithYaw()];
  let drag = null;
  const calls = [];
  const messages = [];
  const pending = [];
  const svg = createSvg();
  const handle = { ownerSVGElement: svg };
  const setMappingDrag = (next) => {
    drag = typeof next === "function" ? next(drag) : next;
  };
  const controller = createMappingInteractionController({
    snapshot: () => ({ fixtures, stage_objects: [], video: { outputs: [] } }),
    mappingViewportBox: () => ({ x: 0, z: 0, width: 100, height: 100 }),
    stageWorldBounds: () => bounds,
    zoomMappingViewportAtPoint: () => {},
    selectedFixture: () => fixtures[0],
    snapStagePoint: (point) => point,
    setFixtureTransform: async (fixture, update) => {
      const call = { fixture, update };
      calls.push(call);
      let resolve;
      const completion = new Promise((complete) => { resolve = complete; });
      pending.push({ call, resolve: (result) => {
        if (result) {
          fixtures = fixtures.map((candidate) => candidate.id === fixture.id
            ? { ...candidate, ...update }
            : candidate);
        }
        resolve(result);
      } });
      return completion;
    },
    mappingStageTool: () => "select",
    isAdditiveMappingSelectionEvent: () => false,
    selectMappingFixture: () => {},
    selectedMappingFixtureIdSet: () => new Set([1]),
    selectedMappingFixtureIds: () => [1],
    setSelectedMappingFixtureIds: () => {},
    selectFixture: () => {},
    activateFixture: () => {},
    setSelectedFixtureId: () => {},
    setSelectedStageObjectId: () => {},
    setSelectedVideoOutputId: () => {},
    clearMappingFixtureSelection: () => {},
    mappingDrag: () => drag,
    setMappingDrag,
    mappingMarquee: () => null,
    setMappingMarquee: () => {},
    mappingViewportPanDrag: () => null,
    setMappingViewportPanDrag: () => {},
    normalizedMappingViewportZoom: () => 1,
    setMappingViewport: () => {},
    setMappingStageCursorWorld: () => {},
    mappingOutputHandleAngleDeg: (center, point) =>
      (Math.atan2(point.z - center.z, point.x - center.x) * 180) / Math.PI,
    mappingOutputHandleDistance: (center, point) => Math.hypot(point.x - center.x, point.z - center.z),
    mappingFixtureYawFromPoint: (center, point) => {
      const dx = point.x - center.x;
      const dz = point.z - center.z;
      return Math.abs(dx) < 0.001 && Math.abs(dz) < 0.001
        ? null
        : Number((((Math.atan2(dz, dx) * 180) / Math.PI + 90 + 360) % 360).toFixed(1));
    },
    dragWorldDelta: (currentDrag) => ({
      x: currentDrag.currentWorld.x - currentDrag.startWorld.x,
      z: currentDrag.currentWorld.z - currentDrag.startWorld.z,
    }),
    snapStagePosition: (position) => position,
    refreshSnapshot: async () => ({ fixtures }),
    setMessage: (message) => messages.push(message),
    isMappingStageObjectDrag: () => false,
    mappingStageObjectPreview: (object) => object,
    setStageObject: async () => {},
    mappingVideoOutputPreviewMapping: (currentDrag, output) => output.mapping,
    setVideoOutputMapping: async () => true,
    visualizerFixtures: () => [],
  });
  const pointer = (pointerId, clientX, clientY, currentTarget = svg) => ({
    button: 0,
    pointerId,
    clientX,
    clientY,
    currentTarget,
    preventDefault: () => {},
    stopPropagation: () => {},
  });
  const begin = (pointerId, clientX = 50, clientY = 50) =>
    controller.beginMappingFixtureYawDrag(pointer(pointerId, clientX, clientY, handle), 1);
  const move = (pointerId, clientX, clientY) =>
    controller.handleMappingStagePointerMove(pointer(pointerId, clientX, clientY));
  const finish = (pointerId, clientX = 60, clientY = 50, type = "pointerup") =>
    controller.finishMappingStageDrag({ ...pointer(pointerId, clientX, clientY), type });
  return {
    controller,
    begin,
    move,
    finish,
    setMappingDrag,
    get drag() { return drag; },
    get fixtures() { return fixtures; },
    calls,
    messages,
    pending,
  };
};

{
  const harness = createHarness();
  harness.begin(1);
  harness.move(1, 60, 50);
  await harness.finish(1, 60, 50, "pointercancel");
  assert.equal(harness.drag, null, "pointercancel must discard an uncommitted yaw preview");
  assert.equal(harness.calls.length, 0, "pointercancel must not persist a transform");
}

{
  const harness = createHarness();
  harness.begin(1);
  harness.move(1, 60, 50);
  const beforeFinish = harness.drag;
  const finish = harness.finish(1);
  assert.equal(harness.calls.length, 1);
  assert.equal(harness.drag, beforeFinish, "the preview must remain while persistence is pending");
  const previewClient = harness.drag.currentClient;
  harness.move(1, 90, 50);
  assert.deepEqual(harness.drag.currentClient, previewClient, "pending pointermove must not overwrite the commit intent");
  await harness.finish(1, 60, 50);
  assert.equal(harness.calls.length, 1, "duplicate pointerup must not issue a second transform");
  harness.pending[0].resolve(true);
  await finish;
  assert.equal(harness.drag, null, "a confirmed transform releases the preview lease");
  assert.equal(harness.fixtures[0].rotation.yaw, 90);
  assert.deepEqual(harness.messages, ["Set Fixture 1 yaw to 90 deg."]);
}

{
  const harness = createHarness();
  harness.begin(1);
  harness.move(1, 60, 50);
  const finish = harness.finish(1);
  assert.equal(harness.drag?.kind, "fixtureYaw");
  harness.pending[0].resolve(false);
  await finish;
  assert.equal(harness.drag, null, "a rejected transform visibly reverts the preview");
  assert.equal(harness.fixtures[0].rotation.yaw, 20);
}

{
  const harness = createHarness();
  harness.begin(1);
  harness.move(1, 60, 50);
  const firstFinish = harness.finish(1);
  const firstDrag = harness.drag;
  harness.begin(2);
  harness.move(2, 70, 50);
  const secondDrag = harness.drag;
  assert.notEqual(secondDrag, firstDrag);
  const secondFinish = harness.finish(2, 70, 50);
  assert.equal(harness.calls.length, 2);
  harness.pending[0].resolve(true);
  await firstFinish;
  assert.equal(harness.drag, secondDrag, "a stale completion must not clear a newer drag");
  assert.deepEqual(harness.messages, [], "a stale completion must not report a newer intent");
  harness.pending[1].resolve(true);
  await secondFinish;
  assert.equal(harness.drag, null);
  assert.deepEqual(harness.messages, ["Set Fixture 1 yaw to 90 deg."]);
}

{
  const harness = createHarness();
  harness.begin(1);
  harness.move(1, 60, 50);
  const finish = harness.finish(1);
  harness.setMappingDrag(null);
  harness.pending[0].resolve(true);
  await finish;
  assert.equal(harness.drag, null, "cancelled/project-replaced interactions remain cancelled after completion");
  assert.deepEqual(harness.messages, [], "a cancelled interaction must not publish a stale message");
}

console.log("Mapping rotation persistence checks passed (pending preview, duplicate events, rejection, cancellation and stale completion fencing).\n");
