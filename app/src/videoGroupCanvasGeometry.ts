import type { Transform2D } from "./types";

export type GroupCanvasPoint = { x: number; y: number };
export type GroupCanvasHandle = "move" | "rotate" | "nw" | "ne" | "se" | "sw";
export const groupCanvasCorners = ["nw", "ne", "se", "sw"] as const;
const signs = { nw: [-1, -1], ne: [1, -1], se: [1, 1], sw: [-1, 1] } as const;
const clamp = (n: number, min: number, max: number) => Math.max(min, Math.min(max, n));
const angle = (degrees: number) => ((degrees + 180) % 360 + 360) % 360 - 180;
export const rotateGroupPoint = (p: GroupCanvasPoint, degrees: number): GroupCanvasPoint => {
  const radians = degrees * Math.PI / 180;
  return { x: p.x * Math.cos(radians) - p.y * Math.sin(radians), y: p.x * Math.sin(radians) + p.y * Math.cos(radians) };
};
// The compositor rotates normalized composition coordinates, then translates.
// Keep this order (including non-square output aspect) instead of CSS rotation.
export const groupCanvasHandlePoint = (t: Transform2D, handle: GroupCanvasHandle): GroupCanvasPoint => {
  const local = handle === "move" ? { x: 0, y: 0 } : handle === "rotate"
    ? { x: 0, y: -t.scale_y / 2 - 0.16 }
    : { x: signs[handle][0] * t.scale_x / 2, y: signs[handle][1] * t.scale_y / 2 };
  const p = rotateGroupPoint(local, t.rotation_deg);
  return { x: p.x + t.x, y: p.y + t.y };
};

/** A gesture is derived from its immutable start, never accumulated deltas. */
export const dragGroupTransform = (t: Transform2D, handle: GroupCanvasHandle, start: GroupCanvasPoint, point: GroupCanvasPoint, shift: boolean): Transform2D => {
  if (![point.x, point.y, start.x, start.y].every(Number.isFinite)) return { ...t };
  const delta = { x: point.x - start.x, y: point.y - start.y };
  if (handle === "move") return { ...t, x: clamp(t.x + delta.x, -1, 1), y: clamp(t.y + delta.y, -1, 1) };
  if (handle === "rotate") {
    const a = Math.atan2(start.y - t.y, start.x - t.x);
    const b = Math.atan2(point.y - t.y, point.x - t.x);
    const degrees = t.rotation_deg + angle((b - a) * 180 / Math.PI);
    return { ...t, rotation_deg: angle(shift ? Math.round(degrees / 15) * 15 : degrees) };
  }
  const local = rotateGroupPoint(delta, -t.rotation_deg);
  const [sx, sy] = signs[handle];
  const x = t.scale_x + 2 * sx * local.x, y = t.scale_y + 2 * sy * local.y;
  if (!shift) return { ...t, scale_x: clamp(x, 0.05, 3), scale_y: clamp(y, 0.05, 3) };
  const factor = clamp((x * t.scale_x + y * t.scale_y) / (t.scale_x ** 2 + t.scale_y ** 2), Math.max(0.05 / t.scale_x, 0.05 / t.scale_y), Math.min(3 / t.scale_x, 3 / t.scale_y));
  return { ...t, scale_x: t.scale_x * factor, scale_y: t.scale_y * factor };
};
export const nudgeGroupTransform = (t: Transform2D, key: string, shift: boolean): Transform2D => {
  const step = shift ? 0.1 : 0.01;
  return { ...t, x: clamp(t.x + (key === "ArrowRight" ? step : key === "ArrowLeft" ? -step : 0), -1, 1), y: clamp(t.y + (key === "ArrowDown" ? step : key === "ArrowUp" ? -step : 0), -1, 1) };
};
