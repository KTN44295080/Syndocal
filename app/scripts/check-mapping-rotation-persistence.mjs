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

const createHarness = (initialFixtures = [fixtureWithYaw()], stageTool = "select") => {
  let fixtures = initialFixtures;
  let selectedIds = fixtures.map((fixture) => fixture.id);
  let projectEpoch = 1;
  let drag = null;
  const calls = [];
  const messages = [];
  const pending = [];
  const svg = createSvg();
  const handle = { ownerSVGElement: svg };
  const setMappingDrag = (next) => {
    drag = typeof next === "function" ? next(drag) : next;
  };
  const setSelectedMappingFixtureIds = (next) => {
    selectedIds = typeof next === "function" ? next(selectedIds) : next;
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
    mappingStageTool: () => stageTool,
    isAdditiveMappingSelectionEvent: () => false,
    selectMappingFixture: () => {},
    selectedMappingFixtureIdSet: () => new Set(selectedIds),
    selectedMappingFixtureIds: () => selectedIds,
    setSelectedMappingFixtureIds,
    selectFixture: (fixture) => { selectedIds = [fixture.id]; },
    activateFixture: () => {},
    setSelectedFixtureId: () => {},
    setSelectedStageObjectId: () => {},
    setSelectedVideoOutputId: () => {},
    clearMappingFixtureSelection: () => {},
    mappingDrag: () => drag,
    setMappingDrag,
    currentProjectAuthority: () => ({ project_epoch: projectEpoch, project_revision: 0, checkpoint_hash: "" }),
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
  const stageRotate = (pointerId = 1, clientX = 60, clientY = 50) =>
    controller.handleMappingStagePointerDown(pointer(pointerId, clientX, clientY));
  return {
    controller,
    begin,
    move,
    finish,
    stageRotate,
    setMappingDrag,
    setSelectedMappingFixtureIds,
    get drag() { return drag; },
    setProjectEpoch: (next) => { projectEpoch = next; },
    get fixtures() { return fixtures; },
    calls,
    messages,
    pending,
  };
};

const waitFor = async (predicate, message) => {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  assert(predicate(), message);
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

{
  const second = {
    ...fixtureWithYaw(120),
    id: 2,
    label: "Fixture 2",
    position: { x: 4, y: 1, z: -3 },
    rotation: { pitch: 12, yaw: 120, roll: -8 },
  };
  const harness = createHarness([fixtureWithYaw(20), second], "rotate");
  const rotate = harness.stageRotate();
  assert.equal(harness.calls.length, 1, "group rotation starts with the first selected fixture");
  assert.deepEqual(harness.calls[0].update.rotation, { pitch: 0, yaw: 90, roll: 0 });
  harness.pending[0].resolve(true);
  await waitFor(() => harness.calls.length === 2, "group rotation must submit every selected fixture");
  assert.deepEqual(harness.calls[1].update.rotation, { pitch: 12, yaw: 190, roll: -8 });
  harness.pending[1].resolve(true);
  await rotate;
  assert.deepEqual(harness.fixtures.map((fixture) => fixture.position), [
    { x: 0, y: 0, z: 0 },
    { x: 4, y: 1, z: -3 },
  ], "stage rotation must not move selected fixtures");
  assert.deepEqual(harness.messages, ["Rotated 2 selected fixtures by 70 deg."]);
}

{
  const second = { ...fixtureWithYaw(120), id: 2, label: "Fixture 2" };
  const harness = createHarness([fixtureWithYaw(20), second], "rotate");
  const rotate = harness.stageRotate();
  assert.equal(harness.calls.length, 1);
  harness.setProjectEpoch(2);
  harness.pending[0].resolve(true);
  await rotate;
  assert.equal(harness.calls.length, 1, "a project replacement must stop a pending stage rotation batch");
  assert.deepEqual(harness.messages, [], "a stale stage rotation must not report against the replacement project");
}

{
  const second = { ...fixtureWithYaw(120), id: 2, label: "Fixture 2" };
  const harness = createHarness([fixtureWithYaw(20), second], "rotate");
  const firstRotate = harness.stageRotate(1, 60, 50);
  assert.equal(harness.calls.length, 1);
  harness.begin(2);
  harness.setMappingDrag(null);
  harness.pending[0].resolve(true);
  await firstRotate;
  assert.equal(harness.calls.length, 1, "a newer mapping interaction must stop the pending stage rotation batch");
  assert.deepEqual(harness.messages, [], "a stale stage rotation must not report after a newer interaction");
}

{
  const harness = createHarness([fixtureWithYaw(20), { ...fixtureWithYaw(120), id: 2, label: "Fixture 2" }], "rotate");
  harness.setSelectedMappingFixtureIds([]);
  const rotate = harness.stageRotate();
  assert.equal(harness.calls.length, 1, "an unselected anchor keeps the stage click single-fixture path");
  assert.deepEqual(harness.calls[0].update.rotation, { pitch: 0, yaw: 90, roll: 0 });
  harness.pending[0].resolve(true);
  await rotate;
  assert.deepEqual(harness.messages, ["Set Fixture 1 yaw to 90 deg."]);
}

{
  const second = {
    ...fixtureWithYaw(350),
    id: 2,
    label: "Fixture 2",
    position: { x: 4, y: 0, z: 0 },
    rotation: { pitch: 7, yaw: 350, roll: 3 },
  };
  const harness = createHarness([fixtureWithYaw(20), second]);
  harness.begin(1);
  harness.move(1, 60, 50);
  const finish = harness.finish(1, 60, 50);
  assert.deepEqual(harness.calls[0].update.rotation, { pitch: 0, yaw: 90, roll: 0 });
  harness.pending[0].resolve(true);
  await waitFor(() => harness.calls.length === 2, "yaw drag must submit every selected fixture");
  assert.deepEqual(harness.calls[1].update.rotation, { pitch: 7, yaw: 60, roll: 3 });
  harness.pending[1].resolve(true);
  await finish;
  assert.equal(harness.drag, null);
  assert.deepEqual(harness.messages, ["Rotated 2 selected fixtures by 70 deg."]);
}

{
  const second = { ...fixtureWithYaw(120), id: 2, label: "Fixture 2" };
  const harness = createHarness([fixtureWithYaw(20), second]);
  harness.begin(1);
  harness.move(1, 60, 50);
  const finish = harness.finish(1, 60, 50);
  assert.equal(harness.calls.length, 1);
  harness.setProjectEpoch(2);
  harness.pending[0].resolve(true);
  await finish;
  assert.equal(harness.calls.length, 1, "a project replacement must stop a pending yaw batch");
  assert.equal(harness.drag, null);
  assert.deepEqual(harness.messages, [], "a stale yaw batch must not report against the replacement project");
}

{
  const second = { ...fixtureWithYaw(120), id: 2, label: "Fixture 2" };
  const harness = createHarness([fixtureWithYaw(20), second]);
  harness.begin(1);
  harness.move(1, 60, 50);
  const finish = harness.finish(1, 60, 50);
  assert.equal(harness.calls.length, 1);
  harness.setMappingDrag(null);
  harness.pending[0].resolve(true);
  await finish;
  assert.equal(harness.calls.length, 1, "a cancelled group drag must not submit later fixtures");
  assert.deepEqual(harness.messages, [], "a cancelled group drag must not report a stale result");
}

console.log("Mapping rotation persistence checks passed (group delta, preserved orientation, pending preview, duplicate events, rejection, cancellation and stale completion fencing).\n");
