import { createMemo, type Accessor, type Setter } from "solid-js";
import type { MappingAxis } from "./mappingRuntime";
import type { EngineSnapshot, PatchFixtureRequest, PatchedFixtureSummary, StageObjectSummary } from "./types";
import { type FixtureTransformExpectation } from "./fixtureTransformConfirmation";
import { applyMappingFixtureTransformBatch } from "./mappingFixtureTransformBatch";
import { mappingInstallationRotationToward } from "./mappingFixtureOrientation";

type FixtureLayoutMode = "line" | "grid" | "circle";
type FixtureTransformUpdate = FixtureTransformExpectation;

interface MappingLayoutControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  filteredFixtures: Accessor<PatchedFixtureSummary[]>;
  selectedFixtureGroupFilter: Accessor<string | null>;
  selectedMappingFixtures: Accessor<PatchedFixtureSummary[]>;
  selectedStageObject: Accessor<StageObjectSummary | null>;
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  selectedMappingFixtureIds: Accessor<number[]>;
  setSelectedMappingFixtureIds: Setter<number[]>;
  activateFixture: (fixture: PatchedFixtureSummary) => void;
  setFixtureTransform: (
    fixture: PatchedFixtureSummary,
    update: FixtureTransformUpdate,
    refresh?: boolean,
  ) => Promise<boolean>;
  snapStagePosition: (position: PatchFixtureRequest["position"]) => PatchFixtureRequest["position"];
  mappingWorldToStageObjectLocal: (
    point: { x: number; z: number },
    object: StageObjectSummary,
  ) => { x: number; z: number };
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
}

export const mappingFixtureSelectionCenter = (fixtures: PatchedFixtureSummary[]) => ({
  x: (Math.min(...fixtures.map((fixture) => fixture.position.x)) + Math.max(...fixtures.map((fixture) => fixture.position.x))) / 2,
  z: (Math.min(...fixtures.map((fixture) => fixture.position.z)) + Math.max(...fixtures.map((fixture) => fixture.position.z))) / 2,
});

export function createMappingLayoutController(options: MappingLayoutControllerOptions) {
  let orientationBusy = false;
  const applyFixtureTransforms = async (
    transforms: Array<{ fixture: PatchedFixtureSummary; update: FixtureTransformUpdate }>,
  ) => applyMappingFixtureTransformBatch({
    transforms,
    setFixtureTransform: options.setFixtureTransform,
    refreshSnapshot: options.refreshSnapshot,
    setMessage: options.setMessage,
  });

  const applySelectedInstallationRotations = async (
    build: (fixtures: PatchedFixtureSummary[]) => Array<{ fixture: PatchedFixtureSummary; update: FixtureTransformUpdate }>,
    message: string,
  ) => {
    if (orientationBusy) return;
    const fixtures = options.selectedMappingFixtures();
    if (fixtures.length === 0) { options.setMessage("向きを変更する灯体を選択してください。"); return; }
    orientationBusy = true;
    try {
      // Resolve and validate every rotation before submitting the first mutation.
      const transforms = build(fixtures);
      if (await applyFixtureTransforms(transforms)) options.setMessage(message);
    } catch (error) { options.setMessage(String(error)); }
    finally { orientationBusy = false; }
  };

  const matchSelectedMappingFixtureOrientations = async () => applySelectedInstallationRotations(fixtures => {
    const active = options.selectedFixture();
    const anchor = active && fixtures.some(fixture => fixture.id === active.id) ? active : fixtures[0];
    if (![anchor.rotation.yaw, anchor.rotation.pitch, anchor.rotation.roll].every(Number.isFinite)) {
      throw new Error("基準灯体の設置姿勢が無効です。");
    }
    return fixtures.map(fixture => ({ fixture, update: { rotation: { ...anchor.rotation } } }));
  }, "選択灯体の設置姿勢を基準灯体に揃えました。");

  const aimSelectedMappingFixtureInstallationAxes = async (target: PatchFixtureRequest["position"]) =>
    applySelectedInstallationRotations(fixtures => fixtures.map(fixture => ({ fixture,
      update: { rotation: mappingInstallationRotationToward(fixture.position, target) },
    })), "選択灯体の設置基準軸（+Z）を指定点へ向けました。DMX値は変更していません。");

  const layoutFixtures = async (
    fixtures: PatchedFixtureSummary[],
    mode: FixtureLayoutMode,
    scopeLabel: string,
  ) => {
    if (fixtures.length === 0) {
      options.setMessage(`No fixtures to arrange for ${scopeLabel}.`);
      return;
    }
    const count = fixtures.length;
    const spacing = 2;
    const radius = Math.max(3, count * 0.55);
    const columns = Math.ceil(Math.sqrt(count));
    const rows = Math.ceil(count / columns);

    const transforms = fixtures.map((fixture, index) => {
      let x = fixture.position.x;
      let z = fixture.position.z;
      if (mode === "line") {
        x = (index - (count - 1) / 2) * spacing;
        z = 0;
      } else if (mode === "grid") {
        const column = index % columns;
        const row = Math.floor(index / columns);
        x = (column - (columns - 1) / 2) * spacing;
        z = (row - (rows - 1) / 2) * spacing;
      } else {
        const angle = (index / count) * Math.PI * 2 - Math.PI / 2;
        x = Math.cos(angle) * radius;
        z = Math.sin(angle) * radius;
      }
      return { fixture, update: { position: options.snapStagePosition({ ...fixture.position, x, z }) } };
    });
    if (await applyFixtureTransforms(transforms)) {
      options.setMessage(`Applied ${mode} layout to ${count} fixture${count === 1 ? "" : "s"} ${scopeLabel}.`);
    }
  };

  const layoutFixturePositions = async (mode: FixtureLayoutMode) => {
    await layoutFixtures(
      options.filteredFixtures(),
      mode,
      options.selectedFixtureGroupFilter() ? `in ${options.selectedFixtureGroupFilter()}` : "in patch",
    );
  };

  const layoutSelectedMappingFixtures = async (mode: FixtureLayoutMode) => {
    await layoutFixtures(options.selectedMappingFixtures(), mode, "in selection");
  };

  const stageObjectLocalToWorld = (object: StageObjectSummary, localX: number, localZ: number) => {
    const angle = (object.rotation_deg * Math.PI) / 180;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    return {
      x: object.x + localX * cos - localZ * sin,
      z: object.z + localX * sin + localZ * cos,
    };
  };

  const layoutSelectedFixturesOnStageObject = async (mode: Exclude<FixtureLayoutMode, "circle">) => {
    const fixtures = options.selectedMappingFixtures();
    const object = options.selectedStageObject();
    if (fixtures.length === 0) {
      options.setMessage("Select fixtures before arranging them on a stage object.");
      return;
    }
    if (!object) {
      options.setMessage("Select a stage object before using it as a layout reference.");
      return;
    }

    const count = fixtures.length;
    const columns = mode === "line" ? count : Math.ceil(Math.sqrt(count));
    const rows = mode === "line" ? 1 : Math.ceil(count / columns);
    const usableWidth = Math.max(0.05, object.width * 0.86);
    const usableDepth = Math.max(0.05, object.depth * 0.78);

    try {
      const transforms = fixtures.map((fixture, index) => {
        const column = mode === "line" ? index : index % columns;
        const row = mode === "line" ? 0 : Math.floor(index / columns);
        const localX = columns <= 1 ? 0 : -usableWidth / 2 + (usableWidth * column) / (columns - 1);
        const localZ = rows <= 1 ? 0 : -usableDepth / 2 + (usableDepth * row) / (rows - 1);
        const point = stageObjectLocalToWorld(object, localX, localZ);
        return {
          fixture,
          update: {
            position: options.snapStagePosition({ ...fixture.position, x: point.x, z: point.z }),
            rotation: { ...fixture.rotation, yaw: Number(object.rotation_deg.toFixed(1)) },
          },
        };
      });
      if (await applyFixtureTransforms(transforms)) {
        options.setMessage(`Arranged ${count} fixture${count === 1 ? "" : "s"} on ${object.label}.`);
      }
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const fixturesInsideStageObject = (object: StageObjectSummary) => {
    const halfWidth = Math.max(0.025, object.width / 2);
    const halfDepth = Math.max(0.025, object.depth / 2);
    return options.snapshot().fixtures.filter((fixture) => {
      const local = options.mappingWorldToStageObjectLocal(
        { x: fixture.position.x, z: fixture.position.z },
        object,
      );
      return Math.abs(local.x) <= halfWidth && Math.abs(local.z) <= halfDepth;
    });
  };

  const stageObjectFixtureCounts = createMemo<Record<number, number>>(() => {
    const counts: Record<number, number> = {};
    for (const object of options.snapshot().stage_objects) {
      counts[object.id] = fixturesInsideStageObject(object).length;
    }
    return counts;
  });

  const pickFixturesInsideSelectedStageObject = (mode: "replace" | "add") => {
    const object = options.selectedStageObject();
    if (!object) {
      options.setMessage("Select a stage object before picking fixtures inside it.");
      return;
    }
    const pickedIds = fixturesInsideStageObject(object).map((fixture) => fixture.id);
    if (pickedIds.length === 0) {
      if (mode === "replace") options.setSelectedMappingFixtureIds([]);
      options.setMessage(`No fixtures inside ${object.label}.`);
      return;
    }
    const nextIds = mode === "add"
      ? [...new Set([...options.selectedMappingFixtureIds(), ...pickedIds])]
      : pickedIds;
    options.setSelectedMappingFixtureIds(nextIds);
    const active = options.snapshot().fixtures.find((fixture) => fixture.id === pickedIds[0]);
    if (active) options.activateFixture(active);
    options.setMessage(
      `${mode === "add" ? "Added" : "Selected"} ${pickedIds.length} fixture${pickedIds.length === 1 ? "" : "s"} inside ${object.label}.`,
    );
  };

  const alignSelectedMappingFixtures = async (axis: MappingAxis) => {
    const fixtures = options.selectedMappingFixtures();
    if (fixtures.length < 2) {
      options.setMessage("Select at least two fixtures to align.");
      return;
    }
    const active = options.selectedFixture();
    const anchor = active && fixtures.some((fixture) => fixture.id === active.id) ? active : fixtures[0];
    const targetValue = anchor.position[axis];
    try {
      const transforms = fixtures.map((fixture) => ({
        fixture,
        update: { position: options.snapStagePosition({ ...fixture.position, [axis]: targetValue }) },
      }));
      if (await applyFixtureTransforms(transforms)) {
        options.setMessage(`Aligned ${fixtures.length} selected fixture(s) on ${axis.toUpperCase()} ${targetValue.toFixed(2)}.`);
      }
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const distributeSelectedMappingFixtures = async (axis: MappingAxis) => {
    const fixtures = [...options.selectedMappingFixtures()].sort(
      (left, right) => left.position[axis] - right.position[axis],
    );
    if (fixtures.length < 3) {
      options.setMessage("Select at least three fixtures to distribute.");
      return;
    }
    const first = fixtures[0].position[axis];
    const last = fixtures[fixtures.length - 1].position[axis];
    if (Math.abs(last - first) < 0.001) {
      options.setMessage(`Selected fixtures need different ${axis.toUpperCase()} positions before distributing.`);
      return;
    }
    const step = (last - first) / (fixtures.length - 1);
    try {
      const transforms = fixtures.map((fixture, index) => ({
        fixture,
        update: { position: options.snapStagePosition({ ...fixture.position, [axis]: first + step * index }) },
      }));
      if (await applyFixtureTransforms(transforms)) {
        options.setMessage(`Distributed ${fixtures.length} selected fixture(s) along ${axis.toUpperCase()}.`);
      }
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const nudgeSelectedMappingFixtures = async (deltaX: number, deltaZ: number) => {
    const fixtures = options.selectedMappingFixtures();
    if (fixtures.length === 0) {
      options.setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    try {
      const transforms = fixtures.map((fixture) => ({
        fixture,
        update: {
          position: options.snapStagePosition({
            ...fixture.position,
            x: fixture.position.x + deltaX,
            z: fixture.position.z + deltaZ,
          }),
        },
      }));
      if (await applyFixtureTransforms(transforms)) {
        options.setMessage(`Nudged ${fixtures.length} selected fixture(s).`);
      }
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const normalizeFixtureYaw = (yaw: number) => Number((((yaw % 360) + 360) % 360).toFixed(1));
  const mirrorSelectedMappingFixtures = async (axis: MappingAxis) => {
    const fixtures = options.selectedMappingFixtures();
    if (fixtures.length < 2) {
      options.setMessage("Select at least two fixtures to mirror.");
      return;
    }
    const center = mappingFixtureSelectionCenter(fixtures);
    try {
      const transforms = fixtures.map((fixture) => {
        const nextPosition = axis === "x"
          ? { ...fixture.position, x: center.x * 2 - fixture.position.x }
          : { ...fixture.position, z: center.z * 2 - fixture.position.z };
        const nextYaw = axis === "x"
          ? normalizeFixtureYaw(360 - fixture.rotation.yaw)
          : normalizeFixtureYaw(180 - fixture.rotation.yaw);
        return {
          fixture,
          update: {
            position: options.snapStagePosition(nextPosition),
            rotation: { ...fixture.rotation, yaw: nextYaw },
          },
        };
      });
      if (await applyFixtureTransforms(transforms)) {
        options.setMessage(`Mirrored ${fixtures.length} selected fixture(s) across ${axis.toUpperCase()}.`);
      }
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const rotateSelectedMappingFixtures = async (degrees: number) => {
    const fixtures = options.selectedMappingFixtures();
    if (fixtures.length < 2) {
      options.setMessage("Select at least two fixtures to rotate.");
      return;
    }
    const center = mappingFixtureSelectionCenter(fixtures);
    const radians = (degrees * Math.PI) / 180;
    const cos = Math.cos(radians);
    const sin = Math.sin(radians);
    try {
      const transforms = fixtures.map((fixture) => {
        const localX = fixture.position.x - center.x;
        const localZ = fixture.position.z - center.z;
        return {
          fixture,
          update: {
            position: options.snapStagePosition({
              ...fixture.position,
              x: center.x + localX * cos - localZ * sin,
              z: center.z + localX * sin + localZ * cos,
            }),
            rotation: { ...fixture.rotation, yaw: normalizeFixtureYaw(fixture.rotation.yaw + degrees) },
          },
        };
      });
      if (await applyFixtureTransforms(transforms)) {
        options.setMessage(`Rotated ${fixtures.length} selected fixture(s) by ${degrees} deg.`);
      }
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  return {
    layoutFixturePositions,
    layoutSelectedMappingFixtures,
    layoutSelectedFixturesOnStageObject,
    stageObjectFixtureCounts,
    pickFixturesInsideSelectedStageObject,
    alignSelectedMappingFixtures,
    distributeSelectedMappingFixtures,
    nudgeSelectedMappingFixtures,
    mirrorSelectedMappingFixtures,
    rotateSelectedMappingFixtures,
    matchSelectedMappingFixtureOrientations,
    aimSelectedMappingFixtureInstallationAxes,
  };
}
