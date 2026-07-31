export const stageFixtureLabelMaxCharacters = 12;
export const stageFixtureLabelDeclutterThreshold = 25;
export const stageFixtureLabelRestoreZoom = 1.5;

export const stageFixtureLabelFootprintGap = 1.25;
// Keep T24's conservative 4-unit planning envelope even though the renderer
// counter-scales the visible label text to a fixed screen-space size.
const stageFixtureLabelCharacterWidth = 4.2;
const stageFixtureLabelStrokeAllowance = 1.2;
const stageFixtureLabelHeight = 6;
const stageFixtureLabelCollisionCellSize = 8;

export interface StageLabelViewport {
  x: number;
  z: number;
  width: number;
  height: number;
}

export interface StageLabelFixture {
  id: number;
  label: string;
  x: number;
  z: number;
  width: number;
  height: number;
  addressOrder: number;
  highlighted: boolean;
}

export interface StageLabelRect {
  x: number;
  z: number;
  width: number;
  height: number;
}

export interface StageFixtureLabelLayout {
  fixtureId: number;
  displayLabel: string;
  fullLabel: string;
  x: number;
  z: number;
  rect: StageLabelRect;
}

export interface StageFixtureLabelLayoutResult {
  labels: StageFixtureLabelLayout[];
  labelByFixtureId: ReadonlyMap<number, StageFixtureLabelLayout>;
  visibleFixtureCount: number;
  candidateCount: number;
  decluttered: boolean;
}

interface StageFixtureLabelCandidate extends StageFixtureLabelLayout {
  addressOrder: number;
  picked: boolean;
  hovered: boolean;
  highlighted: boolean;
}

interface PlanStageFixtureLabelsOptions {
  fixtures: readonly StageLabelFixture[];
  viewport: StageLabelViewport;
  zoom: number;
  showLabels: boolean;
  pickedFixtureIds?: ReadonlySet<number>;
  pickedFixtureId?: number | null;
  hoveredFixtureId?: number | null;
}

const emptyStageFixtureLabelLayoutResult: StageFixtureLabelLayoutResult = {
  labels: [],
  labelByFixtureId: new Map(),
  visibleFixtureCount: 0,
  candidateCount: 0,
  decluttered: false,
};

export const shortenStageFixtureLabel = (label: string) => {
  const characters = Array.from(label);
  return characters.length <= stageFixtureLabelMaxCharacters
    ? label
    : `${characters.slice(0, stageFixtureLabelMaxCharacters).join("")}…`;
};

export const stageLabelRectsOverlap = (left: StageLabelRect, right: StageLabelRect) =>
  left.x < right.x + right.width &&
  left.x + left.width > right.x &&
  left.z < right.z + right.height &&
  left.z + left.height > right.z;

const fixtureIntersectsViewport = (fixture: StageLabelFixture, viewport: StageLabelViewport) => {
  const halfWidth = Math.max(0, fixture.width / 2);
  const halfHeight = Math.max(0, fixture.height / 2);
  return (
    fixture.x + halfWidth >= viewport.x &&
    fixture.x - halfWidth <= viewport.x + viewport.width &&
    fixture.z + halfHeight >= viewport.z &&
    fixture.z - halfHeight <= viewport.z + viewport.height
  );
};

const stageFixtureLabelPosition = (fixture: StageLabelFixture) => ({
  x: fixture.x + Math.max(0, fixture.width) / 2 + stageFixtureLabelFootprintGap,
  z: fixture.z - Math.max(0, fixture.height) / 2 - stageFixtureLabelFootprintGap,
});

const stageFixtureLabelRect = (fixture: StageLabelFixture, displayLabel: string): StageLabelRect => {
  const position = stageFixtureLabelPosition(fixture);
  return {
    x: position.x - stageFixtureLabelStrokeAllowance / 2,
    z:
      position.z -
      stageFixtureLabelHeight +
      stageFixtureLabelStrokeAllowance / 2,
    width: displayLabel.length * stageFixtureLabelCharacterWidth + stageFixtureLabelStrokeAllowance,
    height: stageFixtureLabelHeight,
  };
};

const compareStageFixtureLabelPriority = (
  left: StageFixtureLabelCandidate,
  right: StageFixtureLabelCandidate,
) =>
  Number(right.picked) - Number(left.picked) ||
  Number(right.hovered) - Number(left.hovered) ||
  Number(right.highlighted) - Number(left.highlighted) ||
  left.addressOrder - right.addressOrder ||
  left.fixtureId - right.fixtureId;

const gridCellRange = (rect: StageLabelRect) => ({
  minX: Math.floor(rect.x / stageFixtureLabelCollisionCellSize),
  maxX: Math.floor((rect.x + rect.width) / stageFixtureLabelCollisionCellSize),
  minZ: Math.floor(rect.z / stageFixtureLabelCollisionCellSize),
  maxZ: Math.floor((rect.z + rect.height) / stageFixtureLabelCollisionCellSize),
});

export const planStageFixtureLabels = (
  options: PlanStageFixtureLabelsOptions,
): StageFixtureLabelLayoutResult => {
  if (!options.showLabels) {
    return emptyStageFixtureLabelLayoutResult;
  }

  const visibleFixtures: StageLabelFixture[] = [];
  for (const fixture of options.fixtures) {
    if (fixtureIntersectsViewport(fixture, options.viewport)) {
      visibleFixtures.push(fixture);
    }
  }

  const zoom = Number.isFinite(options.zoom) ? options.zoom : 1;
  const decluttered =
    visibleFixtures.length > stageFixtureLabelDeclutterThreshold &&
    zoom < stageFixtureLabelRestoreZoom;
  const candidates: StageFixtureLabelCandidate[] = [];
  for (const fixture of visibleFixtures) {
    const picked =
      options.pickedFixtureId === fixture.id ||
      options.pickedFixtureIds?.has(fixture.id) === true;
    const hovered = options.hoveredFixtureId === fixture.id;
    if (decluttered && !picked && !hovered && !fixture.highlighted) {
      continue;
    }
    const displayLabel = shortenStageFixtureLabel(fixture.label);
    const position = stageFixtureLabelPosition(fixture);
    const rect = stageFixtureLabelRect(fixture, displayLabel);
    candidates.push({
      fixtureId: fixture.id,
      displayLabel,
      fullLabel: fixture.label,
      x: position.x,
      z: position.z,
      rect,
      addressOrder: fixture.addressOrder,
      picked,
      hovered,
      highlighted: fixture.highlighted,
    });
  }
  candidates.sort(compareStageFixtureLabelPriority);

  const accepted: StageFixtureLabelLayout[] = [];
  const occupiedCells = new Map<string, StageFixtureLabelLayout[]>();
  for (const candidate of candidates) {
    const cells = gridCellRange(candidate.rect);
    let overlaps = false;
    for (let cellX = cells.minX; cellX <= cells.maxX && !overlaps; cellX += 1) {
      for (let cellZ = cells.minZ; cellZ <= cells.maxZ && !overlaps; cellZ += 1) {
        const bucket = occupiedCells.get(`${cellX}:${cellZ}`);
        if (bucket?.some((label) => stageLabelRectsOverlap(label.rect, candidate.rect))) {
          overlaps = true;
        }
      }
    }
    if (overlaps) {
      continue;
    }

    accepted.push(candidate);
    for (let cellX = cells.minX; cellX <= cells.maxX; cellX += 1) {
      for (let cellZ = cells.minZ; cellZ <= cells.maxZ; cellZ += 1) {
        const key = `${cellX}:${cellZ}`;
        const bucket = occupiedCells.get(key);
        if (bucket) {
          bucket.push(candidate);
        } else {
          occupiedCells.set(key, [candidate]);
        }
      }
    }
  }

  return {
    labels: accepted,
    labelByFixtureId: new Map(accepted.map((label) => [label.fixtureId, label])),
    visibleFixtureCount: visibleFixtures.length,
    candidateCount: candidates.length,
    decluttered,
  };
};
