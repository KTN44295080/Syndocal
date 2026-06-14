// Stage decoration object helpers extracted from App.tsx: the kinds list, default fill
// color per kind, and the SVG class name. Pure; depends only on StageObjectKind.
import type { StageObjectKind } from "./types";

export const stageObjectKinds: StageObjectKind[] = ["Stage", "Truss", "Screen", "Riser", "Mask"];

export const stageObjectDefaultColor = (kind: StageObjectKind) =>
  kind === "Screen"
    ? "#4cb7ff"
    : kind === "Truss"
      ? "#f2c14e"
      : kind === "Riser"
        ? "#9b8cff"
        : kind === "Mask"
          ? "#727a84"
          : "#5dd64c";

export const stageObjectClass = (object: { kind: StageObjectKind }) => `stageObject kind-${object.kind.toLowerCase()}`;
