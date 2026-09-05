import type { PatchFixtureRequest } from "./types";

/** Right-handed world transform matching crates/visualizer rotate_vec3. */
export function rotateMappingFixtureDirection(
  vector: PatchFixtureRequest["position"], rotation: PatchFixtureRequest["rotation"],
): PatchFixtureRequest["position"] {
  const pitch = rotation.pitch * Math.PI / 180, yaw = rotation.yaw * Math.PI / 180, roll = rotation.roll * Math.PI / 180;
  const py = vector.y * Math.cos(pitch) - vector.z * Math.sin(pitch);
  const pz = vector.y * Math.sin(pitch) + vector.z * Math.cos(pitch);
  const yx = vector.x * Math.cos(yaw) + pz * Math.sin(yaw);
  const yz = -vector.x * Math.sin(yaw) + pz * Math.cos(yaw);
  return { x: yx * Math.cos(roll) - py * Math.sin(roll), y: yx * Math.sin(roll) + py * Math.cos(roll), z: yz };
}

/** Aim the fixture's mounting +Z axis, not a profile geometry or live DMX head.
 * Matches visualizer rotate_vec3's pitch -> yaw -> roll order with roll = 0. */
export function mappingInstallationRotationToward(
  position: PatchFixtureRequest["position"], target: PatchFixtureRequest["position"],
): PatchFixtureRequest["rotation"] {
  if (![position.x, position.y, position.z, target.x, target.y, target.z].every(Number.isFinite)) {
    throw new Error("指定点と灯体位置は有限の数値で指定してください。");
  }
  const dx = target.x - position.x, dy = target.y - position.y, dz = target.z - position.z;
  const horizontal = Math.hypot(dx, dz), distance = Math.hypot(horizontal, dy);
  if (!Number.isFinite(distance) || distance < 1e-6) throw new Error("灯体と同じ位置は向きの指定点にできません。");
  return {
    yaw: horizontal < 1e-6 ? 0 : Math.atan2(dx, dz) * 180 / Math.PI,
    pitch: -Math.atan2(dy, horizontal) * 180 / Math.PI,
    roll: 0,
  };
}
