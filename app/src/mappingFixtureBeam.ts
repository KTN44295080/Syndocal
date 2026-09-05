import { cumulativeGeometryMatrix } from "./numericHelpers";
import { rotateMappingFixtureDirection } from "./mappingFixtureOrientation";
import { fixtureVisualKind } from "./fixtureVisuals";
import type { AttributeValueSummary, PatchedFixtureSummary, Vec3 } from "./types";

/** Same output-DMX precedence and 8/16-bit expansion as live fixture color. */
export function mappingFixtureLiveMovement(
  fixture: PatchedFixtureSummary,
  previews: ReadonlyMap<number, number[]>,
  liveAttributes: AttributeValueSummary[] = fixture.attribute_values,
): { pan: number | undefined; tilt: number | undefined } {
  const values = new Map(liveAttributes.map(value => [value.attribute.toLowerCase(), value.value]));
  const read = (attribute: string) => {
    const control = fixture.controls.find(control => control.attribute.toLowerCase() === attribute);
    const preview = previews.get(fixture.universe);
    if (preview !== undefined && control) {
      const byte = (offset: number | undefined) => {
        const value = offset ? preview[fixture.address + offset - 2] ?? 0 : 0;
        return Math.max(0, Math.min(255, Number.isFinite(value) ? Math.round(value) : 0));
      };
      const coarse = byte(control.offsets[0]);
      return control.resolution === "SixteenBit" ? coarse * 256 + byte(control.offsets[1]) : coarse * 257;
    }
    return values.get(attribute) ?? control?.default_value;
  };
  return { pan: read("pan"), tilt: read("tilt") };
}

const unit = (v: Vec3): Vec3 | null => {
  const size = Math.hypot(v.x, v.y, v.z);
  return Number.isFinite(size) && size > 1e-8 ? { x: v.x / size, y: v.y / size, z: v.z / size } : null;
};
const cross = (a: Vec3, b: Vec3): Vec3 => ({ x: a.y * b.z - a.z * b.y, y: a.z * b.x - a.x * b.z, z: a.x * b.y - a.y * b.x });

export interface MappingBeamShape { direction: Vec3; angle: number; description: string }

/** Display geometry only. Missing optics use an explicitly labelled schematic;
 * authored optics are never replaced by a fixture-name guess. */
export function mappingFixtureBeamShape(fixture: PatchedFixtureSummary, yaw: number, pan: number | undefined, tilt: number | undefined, geometryName?: string): MappingBeamShape | null {
  const geometry = geometryName ? fixture.geometries.find(g => g.name === geometryName)
    : fixture.geometries.find(g => g.kind.toLowerCase() === "beam");
  const matrix = geometry ? cumulativeGeometryMatrix(geometry, new Map(fixture.geometries.map(g => [g.name, g]))) : null;
  const local = matrix ? { x: matrix[2], y: matrix[6], z: matrix[10] } : { x: 0, y: 0, z: 1 };
  const head = rotateMappingFixtureDirection(local, {
    yaw: pan === undefined ? 0 : (pan - 32768) / 65535 * 540,
    pitch: tilt === undefined ? 0 : (tilt - 32768) / 65535 * 270, roll: 0,
  });
  const direction = unit(rotateMappingFixtureDirection(head, { ...fixture.rotation, yaw }));
  const authored = geometry?.beam_angle_deg ?? geometry?.field_angle_deg;
  const kind = fixtureVisualKind(fixture);
  const spot = /spot|beam/i.test(`${fixture.profile_name} ${fixture.label}`);
  const angle = authored ?? (kind === "laser" ? 2 : kind === "moving" || kind === "point" || spot ? 10 : 30);
  if (!direction || !Number.isFinite(angle) || angle <= 0 || angle >= 179) return null;
  return { direction, angle, description: authored == null ? `概略ビーム ${angle}°（プロファイル角度なし）`
    : `プロファイルの${geometry?.beam_angle_deg == null ? "フィールド角" : "ビーム角"} ${angle}°` };
}

/** Orthographic X/Z projection of a cone; a vertical beam becomes a footprint,
 * not a falsely horizontal ray. Length remains a presentation extent. */
export function mappingBeamPoints(x: number, z: number, shape: MappingBeamShape | null): string {
  if (!shape) return "";
  const direction = shape.direction, length = 52;
  const radius = length * Math.tan(shape.angle * Math.PI / 360);
  const u = unit(cross(direction, Math.abs(direction.y) < .9 ? { x: 0, y: 1, z: 0 } : { x: 1, y: 0, z: 0 }))!;
  const v = cross(direction, u);
  const points = [{ x, z }, ...Array.from({ length: 16 }, (_, index) => {
    const angle = index * Math.PI / 8;
    return { x: x + direction.x * length + radius * (u.x * Math.cos(angle) + v.x * Math.sin(angle)),
      z: z + direction.z * length + radius * (u.z * Math.cos(angle) + v.z * Math.sin(angle)) };
  })].sort((a, b) => a.x - b.x || a.z - b.z);
  const turn = (a: typeof points[number], b: typeof points[number], c: typeof points[number]) => (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x);
  const half = (ordered: typeof points) => {
    const hull: typeof points = [];
    for (const point of ordered) { while (hull.length > 1 && turn(hull.at(-2)!, hull.at(-1)!, point) <= 0) hull.pop(); hull.push(point); }
    return hull.slice(0, -1);
  };
  return [...half(points), ...half([...points].reverse())].map(p => `${p.x},${p.z}`).join(" ");
}
