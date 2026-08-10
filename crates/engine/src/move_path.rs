use protocol::{MoveEffectRequest, MoveInterpolation, MovePathPoint};

const SMOOTH_SAMPLES_PER_SEGMENT: usize = 32;
const MIN_PATH_LENGTH: f32 = 1.0e-6;

#[derive(Debug, Clone)]
pub(super) struct CompiledMovePath {
    kind: CompiledMovePathKind,
}

#[derive(Debug, Clone)]
enum CompiledMovePathKind {
    Constant(MovePathPoint),
    ArcLength {
        samples: Vec<MoveArcSample>,
        total_length: f32,
    },
    DaslightLine {
        points: Vec<MovePathPoint>,
    },
    DaslightPolygon {
        points: Vec<MovePathPoint>,
    },
    DaslightPoints {
        points: Vec<MovePathPoint>,
    },
    TwoPointCircle {
        center_x: f64,
        center_y: f64,
        radius: f64,
        start_angle: f64,
    },
    Circle {
        segments: Vec<CompiledCircleSegment>,
    },
}

#[derive(Debug, Clone, Copy)]
struct MoveArcSample {
    distance: f32,
    point: MovePathPoint,
}

#[derive(Debug, Clone, Copy)]
enum CompiledCircleSegment {
    Arc(CompiledCircularArc),
    /// Daslight joins opposite signed adjacent circumcircles as two half-arcs.
    /// The second half is the first half reflected through the chord midpoint.
    Inflection {
        first_half: CompiledCircularArc,
        midpoint_x: f64,
        midpoint_y: f64,
    },
    Linear {
        from: MovePathPoint,
        to: MovePathPoint,
    },
}

#[derive(Debug, Clone, Copy)]
struct CompiledCircularArc {
    center_x: f64,
    center_y: f64,
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
}

impl CompiledMovePath {
    pub(super) fn compile(request: &MoveEffectRequest) -> Result<Self, String> {
        if request.points.len() < 2 {
            return Err("Move effect requires at least two path points".to_string());
        }
        if matches!(
            request.interpolation,
            MoveInterpolation::Circle | MoveInterpolation::DaslightCircle
        ) {
            if request.interpolation == MoveInterpolation::DaslightCircle
                && all_points_equal(&request.points)
            {
                return Ok(Self {
                    kind: CompiledMovePathKind::Constant(request.points[0]),
                });
            }
            return compile_circle_path(&request.points);
        }
        match request.interpolation {
            MoveInterpolation::DaslightCurve => return compile_daslight_curve(&request.points),
            MoveInterpolation::DaslightLine => {
                let mut points = request.points.clone();
                points.extend(
                    request.points[1..request.points.len() - 1]
                        .iter()
                        .rev()
                        .copied(),
                );
                return Ok(Self {
                    kind: CompiledMovePathKind::DaslightLine { points },
                });
            }
            MoveInterpolation::DaslightPolygon => {
                return Ok(Self {
                    kind: CompiledMovePathKind::DaslightPolygon {
                        points: request.points.clone(),
                    },
                });
            }
            MoveInterpolation::DaslightPoints => {
                return Ok(Self {
                    kind: CompiledMovePathKind::DaslightPoints {
                        points: request.points.clone(),
                    },
                });
            }
            MoveInterpolation::Line | MoveInterpolation::Smooth | MoveInterpolation::Circle => {}
            MoveInterpolation::DaslightCircle => unreachable!("Daslight Circle compiled above"),
        }
        let mut points = Vec::with_capacity(request.points.len());
        for point in &request.points {
            if points
                .last()
                .is_some_and(|previous| point_distance(*previous, *point) <= MIN_PATH_LENGTH)
            {
                continue;
            }
            points.push(*point);
        }
        if request.closed
            && points.len() > 1
            && point_distance(points[0], points[points.len() - 1]) <= MIN_PATH_LENGTH
        {
            points.pop();
        }
        if points.len() < 2 {
            return Err("Move effect path must contain at least two distinct points".to_string());
        }

        let segment_count = if request.closed {
            points.len()
        } else {
            points.len() - 1
        };
        let samples_per_segment = match request.interpolation {
            MoveInterpolation::Line => 1,
            MoveInterpolation::Smooth => SMOOTH_SAMPLES_PER_SEGMENT,
            MoveInterpolation::Circle => unreachable!("Circle paths compile analytically"),
            MoveInterpolation::DaslightCircle
            | MoveInterpolation::DaslightCurve
            | MoveInterpolation::DaslightLine
            | MoveInterpolation::DaslightPolygon
            | MoveInterpolation::DaslightPoints => {
                unreachable!("Daslight paths compile through their exact evaluator")
            }
        };
        let mut samples = Vec::with_capacity(
            segment_count
                .saturating_mul(samples_per_segment)
                .saturating_add(1),
        );
        let first = segment_point(&points, request.closed, request.interpolation, 0, 0.0);
        samples.push(MoveArcSample {
            distance: 0.0,
            point: first,
        });

        let mut previous = first;
        let mut total_length = 0.0_f32;
        for segment in 0..segment_count {
            for sample_index in 1..=samples_per_segment {
                let t = sample_index as f32 / samples_per_segment as f32;
                let point =
                    segment_point(&points, request.closed, request.interpolation, segment, t);
                if !point.x.is_finite() || !point.y.is_finite() {
                    return Err(
                        "Move effect path interpolation produced a non-finite point".to_string()
                    );
                }
                let distance = point_distance(previous, point);
                previous = point;
                if distance <= MIN_PATH_LENGTH {
                    continue;
                }
                total_length += distance;
                samples.push(MoveArcSample {
                    distance: total_length,
                    point,
                });
            }
        }

        if total_length <= MIN_PATH_LENGTH || samples.len() < 2 {
            return Err("Move effect path must contain at least two distinct points".to_string());
        }

        Ok(Self {
            kind: CompiledMovePathKind::ArcLength {
                samples,
                total_length,
            },
        })
    }

    pub(super) fn sample(&self, progress: f32) -> MovePathPoint {
        let progress = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match &self.kind {
            CompiledMovePathKind::Constant(point) => *point,
            CompiledMovePathKind::ArcLength {
                samples,
                total_length,
            } => sample_arc_length_path(samples, *total_length, progress),
            CompiledMovePathKind::TwoPointCircle {
                center_x,
                center_y,
                radius,
                start_angle,
            } => {
                let angle = start_angle + std::f64::consts::TAU * f64::from(progress);
                clamped_circle_point(*center_x, *center_y, *radius, angle)
            }
            CompiledMovePathKind::Circle { segments } => {
                let scaled = f64::from(progress) * segments.len() as f64;
                let (segment_index, local_progress) = if progress >= 1.0 {
                    (segments.len() - 1, 1.0)
                } else {
                    let segment_index = scaled.floor() as usize;
                    (segment_index, scaled - segment_index as f64)
                };
                sample_circle_segment(segments[segment_index], local_progress)
            }
            CompiledMovePathKind::DaslightLine { .. }
            | CompiledMovePathKind::DaslightPolygon { .. }
            | CompiledMovePathKind::DaslightPoints { .. } => {
                unreachable!("Daslight paths require a frame count")
            }
        }
    }

    pub(super) fn sample_daslight_phase(&self, phase: f32) -> MovePathPoint {
        let phase = if phase.is_finite() {
            phase.rem_euclid(1.0)
        } else {
            0.0
        };
        match &self.kind {
            CompiledMovePathKind::Constant(point) => *point,
            CompiledMovePathKind::ArcLength {
                samples,
                total_length,
            } => {
                let doubled = 2.0 * f64::from(phase);
                let progress = if doubled <= 1.0 {
                    doubled
                } else {
                    2.0 - doubled
                } as f32;
                sample_arc_length_path(samples, *total_length, progress)
            }
            CompiledMovePathKind::DaslightLine { points } => sample_equal_time_edges(points, phase),
            CompiledMovePathKind::DaslightPolygon { points } => {
                sample_equal_time_edges(points, phase)
            }
            CompiledMovePathKind::DaslightPoints { points } => {
                let index = (phase * points.len() as f32).floor() as usize;
                points[index.min(points.len() - 1)]
            }
            CompiledMovePathKind::TwoPointCircle { .. } | CompiledMovePathKind::Circle { .. } => {
                self.sample(phase)
            }
        }
    }
}

fn all_points_equal(points: &[MovePathPoint]) -> bool {
    points.first().is_none_or(|first| {
        points
            .iter()
            .all(|point| point_distance(*first, *point) <= MIN_PATH_LENGTH)
    })
}

fn sample_equal_time_edges(points: &[MovePathPoint], phase: f32) -> MovePathPoint {
    let scaled = points.len() as f64 * f64::from(phase.rem_euclid(1.0));
    let index = scaled.floor() as usize % points.len();
    let next = (index + 1) % points.len();
    lerp_point(points[index], points[next], (scaled - index as f64) as f32)
}

fn compile_daslight_curve(points: &[MovePathPoint]) -> Result<CompiledMovePath, String> {
    if all_points_equal(points) {
        return Ok(CompiledMovePath {
            kind: CompiledMovePathKind::Constant(points[0]),
        });
    }
    let mut samples = Vec::with_capacity((points.len() - 1).saturating_mul(16) + 1);
    samples.push(MoveArcSample {
        distance: 0.0,
        point: points[0],
    });
    let mut previous = points[0];
    let mut total_length = 0.0_f32;
    for segment in 0..points.len() - 1 {
        let p0 = if segment == 0 {
            points[segment]
        } else {
            points[segment - 1]
        };
        let p1 = points[segment];
        let p2 = points[segment + 1];
        let p3 = points.get(segment + 2).copied().unwrap_or(p2);
        for slice in 1..=16 {
            let point = uniform_catmull_rom(p0, p1, p2, p3, slice as f32 / 16.0);
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err("Daslight Curve produced a non-finite point".to_string());
            }
            let distance = point_distance(previous, point);
            previous = point;
            if distance <= MIN_PATH_LENGTH {
                continue;
            }
            total_length += distance;
            samples.push(MoveArcSample {
                distance: total_length,
                point,
            });
        }
    }
    if total_length <= MIN_PATH_LENGTH || samples.len() < 2 {
        return Ok(CompiledMovePath {
            kind: CompiledMovePathKind::Constant(points[0]),
        });
    }
    Ok(CompiledMovePath {
        kind: CompiledMovePathKind::ArcLength {
            samples,
            total_length,
        },
    })
}

fn uniform_catmull_rom(
    p0: MovePathPoint,
    p1: MovePathPoint,
    p2: MovePathPoint,
    p3: MovePathPoint,
    t: f32,
) -> MovePathPoint {
    let squared = t * t;
    let cubed = squared * t;
    let h00 = 2.0 * cubed - 3.0 * squared + 1.0;
    let h10 = cubed - 2.0 * squared + t;
    let h01 = -2.0 * cubed + 3.0 * squared;
    let h11 = cubed - squared;
    MovePathPoint {
        x: h00 * p1.x + h10 * 0.5 * (p2.x - p0.x) + h01 * p2.x + h11 * 0.5 * (p3.x - p1.x),
        y: h00 * p1.y + h10 * 0.5 * (p2.y - p0.y) + h01 * p2.y + h11 * 0.5 * (p3.y - p1.y),
    }
}

fn sample_arc_length_path(
    samples: &[MoveArcSample],
    total_length: f32,
    progress: f32,
) -> MovePathPoint {
    if progress <= 0.0 {
        return samples[0].point;
    }
    if progress >= 1.0 {
        return samples[samples.len() - 1].point;
    }

    let target_distance = progress * total_length;
    let mut lower = 0_usize;
    let mut upper = samples.len() - 1;
    while lower + 1 < upper {
        let middle = lower + (upper - lower) / 2;
        if samples[middle].distance < target_distance {
            lower = middle;
        } else {
            upper = middle;
        }
    }

    let from = samples[lower];
    let to = samples[upper];
    let distance = (to.distance - from.distance).max(MIN_PATH_LENGTH);
    let t = ((target_distance - from.distance) / distance).clamp(0.0, 1.0);
    MovePathPoint {
        x: from.point.x + (to.point.x - from.point.x) * t,
        y: from.point.y + (to.point.y - from.point.y) * t,
    }
}

fn compile_circle_path(points: &[MovePathPoint]) -> Result<CompiledMovePath, String> {
    if points.len() == 2 {
        let from = points[0];
        let to = points[1];
        let center_x = (f64::from(from.x) + f64::from(to.x)) * 0.5;
        let center_y = (f64::from(from.y) + f64::from(to.y)) * 0.5;
        let radius = (f64::from(from.x) - center_x).hypot(f64::from(from.y) - center_y);
        if radius <= f64::from(MIN_PATH_LENGTH) {
            return Err("Move effect path must contain at least two distinct points".to_string());
        }
        let start_angle = (f64::from(from.y) - center_y)
            .atan2(f64::from(from.x) - center_x)
            .rem_euclid(std::f64::consts::TAU);
        return Ok(CompiledMovePath {
            kind: CompiledMovePathKind::TwoPointCircle {
                center_x,
                center_y,
                radius,
                start_angle,
            },
        });
    }

    let mut segments = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        let after = points[(index + 2) % points.len()];
        let mut incoming_radius = signed_circumradius(previous, from, to);
        let mut outgoing_radius = signed_circumradius(from, to, after);
        // The recovered evaluator substitutes the adjacent non-degenerate
        // radius when only one wrapped triple is collinear.
        if outgoing_radius == 0.0 {
            outgoing_radius = incoming_radius;
        }
        if incoming_radius == 0.0 {
            incoming_radius = outgoing_radius;
        }
        let average_radius = (incoming_radius.abs() + outgoing_radius.abs()) * 0.5;
        let signed_average = average_radius.copysign(incoming_radius);
        let opposite_signs = incoming_radius.is_sign_positive()
            != outgoing_radius.is_sign_positive()
            && incoming_radius != 0.0
            && outgoing_radius != 0.0;
        let segment = if opposite_signs {
            let midpoint = MovePathPoint {
                x: (from.x + to.x) * 0.5,
                y: (from.y + to.y) * 0.5,
            };
            compile_circular_arc(from, midpoint, signed_average * 0.5)
                .map(|first_half| CompiledCircleSegment::Inflection {
                    first_half,
                    midpoint_x: f64::from(midpoint.x),
                    midpoint_y: f64::from(midpoint.y),
                })
                .unwrap_or(CompiledCircleSegment::Linear { from, to })
        } else {
            compile_circular_arc(from, to, signed_average)
                .map(CompiledCircleSegment::Arc)
                .unwrap_or(CompiledCircleSegment::Linear { from, to })
        };
        segments.push(segment);
    }
    Ok(CompiledMovePath {
        kind: CompiledMovePathKind::Circle { segments },
    })
}

fn signed_circumradius(a: MovePathPoint, b: MovePathPoint, c: MovePathPoint) -> f64 {
    let ax = f64::from(a.x);
    let ay = f64::from(a.y);
    let bx = f64::from(b.x);
    let by = f64::from(b.y);
    let cx = f64::from(c.x);
    let cy = f64::from(c.y);
    let determinant = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if determinant == 0.0 || !determinant.is_finite() {
        return 0.0;
    }
    let a_squared = ax * ax + ay * ay;
    let b_squared = bx * bx + by * by;
    let c_squared = cx * cx + cy * cy;
    let center_x =
        (a_squared * (by - cy) + b_squared * (cy - ay) + c_squared * (ay - by)) / determinant;
    let center_y =
        (a_squared * (cx - bx) + b_squared * (ax - cx) + c_squared * (bx - ax)) / determinant;
    let radius = (ax - center_x).hypot(ay - center_y);
    if radius.is_finite() {
        radius.copysign(determinant)
    } else {
        0.0
    }
}

fn compile_circular_arc(
    from: MovePathPoint,
    to: MovePathPoint,
    signed_radius: f64,
) -> Option<CompiledCircularArc> {
    if signed_radius == 0.0 || !signed_radius.is_finite() {
        return None;
    }
    let from_x = f64::from(from.x);
    let from_y = f64::from(from.y);
    let to_x = f64::from(to.x);
    let to_y = f64::from(to.y);
    let chord_x = to_x - from_x;
    let chord_y = to_y - from_y;
    let chord_length = chord_x.hypot(chord_y);
    if chord_length == 0.0 {
        return None;
    }
    let radius = signed_radius.abs();
    let height_squared = radius * radius - chord_length * chord_length * 0.25;
    if height_squared < 0.0 {
        return None;
    }
    let midpoint_x = (from_x + to_x) * 0.5;
    let midpoint_y = (from_y + to_y) * 0.5;
    let height = height_squared.sqrt();
    let side = signed_radius.signum();
    let center_x = midpoint_x - side * chord_y / chord_length * height;
    let center_y = midpoint_y + side * chord_x / chord_length * height;
    let start_angle = (from_y - center_y)
        .atan2(from_x - center_x)
        .rem_euclid(std::f64::consts::TAU);
    let end_angle = (to_y - center_y)
        .atan2(to_x - center_x)
        .rem_euclid(std::f64::consts::TAU);
    let mut sweep_angle = end_angle - start_angle;
    if signed_radius > 0.0 && sweep_angle < 0.0 {
        sweep_angle += std::f64::consts::TAU;
    } else if signed_radius < 0.0 && sweep_angle > 0.0 {
        sweep_angle -= std::f64::consts::TAU;
    }
    Some(CompiledCircularArc {
        center_x,
        center_y,
        radius,
        start_angle,
        sweep_angle,
    })
}

fn sample_circle_segment(segment: CompiledCircleSegment, progress: f64) -> MovePathPoint {
    match segment {
        CompiledCircleSegment::Arc(arc) => {
            let (x, y) = sample_circular_arc_raw(arc, progress);
            clamped_move_point(x, y)
        }
        CompiledCircleSegment::Inflection {
            first_half,
            midpoint_x,
            midpoint_y,
        } => {
            if progress <= 0.5 {
                let (x, y) = sample_circular_arc_raw(first_half, progress * 2.0);
                clamped_move_point(x, y)
            } else {
                let (x, y) = sample_circular_arc_raw(first_half, (1.0 - progress) * 2.0);
                clamped_move_point(2.0 * midpoint_x - x, 2.0 * midpoint_y - y)
            }
        }
        CompiledCircleSegment::Linear { from, to } => lerp_point(from, to, progress as f32),
    }
}

fn sample_circular_arc_raw(arc: CompiledCircularArc, progress: f64) -> (f64, f64) {
    let angle = arc.start_angle + arc.sweep_angle * progress.clamp(0.0, 1.0);
    let (sine, cosine) = angle.sin_cos();
    (
        arc.center_x + cosine * arc.radius,
        arc.center_y + sine * arc.radius,
    )
}

fn clamped_circle_point(center_x: f64, center_y: f64, radius: f64, angle: f64) -> MovePathPoint {
    let (x, y) = {
        let (sine, cosine) = angle.sin_cos();
        (center_x + cosine * radius, center_y + sine * radius)
    };
    clamped_move_point(x, y)
}

fn clamped_move_point(x: f64, y: f64) -> MovePathPoint {
    MovePathPoint {
        x: x.clamp(0.0, 1.0) as f32,
        y: y.clamp(0.0, 1.0) as f32,
    }
}

pub(super) fn move_rotation(rotation_degrees: f32) -> (f32, f32) {
    let radians = rotation_degrees.to_radians();
    (radians.cos(), radians.sin())
}

pub(super) fn transform_move_delta(
    request: &MoveEffectRequest,
    point: MovePathPoint,
    cosine: f32,
    sine: f32,
) -> MovePathPoint {
    let local_x = (point.x - 0.5) * request.size_x;
    let local_y = (point.y - 0.5) * request.size_y;
    MovePathPoint {
        // Positive editor degrees rotate clockwise on the screen. The protocol
        // uses an upward-positive Y axis, so its signed matrix is mirrored here.
        x: local_x * cosine + local_y * sine,
        y: -local_x * sine + local_y * cosine,
    }
}

fn segment_point(
    points: &[MovePathPoint],
    closed: bool,
    interpolation: MoveInterpolation,
    segment: usize,
    t: f32,
) -> MovePathPoint {
    let point_count = points.len();
    let from_index = segment.min(point_count - 1);
    let to_index = if from_index + 1 < point_count {
        from_index + 1
    } else {
        0
    };
    let from = points[from_index];
    let to = points[to_index];
    match interpolation {
        MoveInterpolation::Line => lerp_point(from, to, t),
        MoveInterpolation::Smooth => {
            let previous = if from_index == 0 {
                if closed {
                    points[point_count - 1]
                } else {
                    from
                }
            } else {
                points[from_index - 1]
            };
            let next_index = to_index + 1;
            let next = if next_index < point_count {
                points[next_index]
            } else if closed {
                points[next_index % point_count]
            } else {
                to
            };
            centripetal_catmull_rom(previous, from, to, next, t)
        }
        MoveInterpolation::Circle => unreachable!("Circle paths compile analytically"),
        MoveInterpolation::DaslightCircle
        | MoveInterpolation::DaslightCurve
        | MoveInterpolation::DaslightLine
        | MoveInterpolation::DaslightPolygon
        | MoveInterpolation::DaslightPoints => {
            unreachable!("Daslight paths compile through their exact evaluator")
        }
    }
}

fn lerp_point(from: MovePathPoint, to: MovePathPoint, t: f32) -> MovePathPoint {
    MovePathPoint {
        x: from.x + (to.x - from.x) * t,
        y: from.y + (to.y - from.y) * t,
    }
}

fn centripetal_catmull_rom(
    p0: MovePathPoint,
    p1: MovePathPoint,
    p2: MovePathPoint,
    p3: MovePathPoint,
    t: f32,
) -> MovePathPoint {
    let t0 = 0.0_f32;
    let t1 = t0 + centripetal_step(p0, p1);
    let t2 = t1 + centripetal_step(p1, p2);
    let t3 = t2 + centripetal_step(p2, p3);
    let at = t1 + (t2 - t1) * t.clamp(0.0, 1.0);
    let a1 = parameterized_lerp(p0, p1, t0, t1, at);
    let a2 = parameterized_lerp(p1, p2, t1, t2, at);
    let a3 = parameterized_lerp(p2, p3, t2, t3, at);
    let b1 = parameterized_lerp(a1, a2, t0, t2, at);
    let b2 = parameterized_lerp(a2, a3, t1, t3, at);
    parameterized_lerp(b1, b2, t1, t2, at)
}

fn centripetal_step(from: MovePathPoint, to: MovePathPoint) -> f32 {
    point_distance(from, to).sqrt().max(1.0e-4)
}

fn parameterized_lerp(
    from: MovePathPoint,
    to: MovePathPoint,
    from_t: f32,
    to_t: f32,
    at: f32,
) -> MovePathPoint {
    let denominator = (to_t - from_t).max(1.0e-6);
    let weight = (at - from_t) / denominator;
    lerp_point(from, to, weight)
}

fn point_distance(from: MovePathPoint, to: MovePathPoint) -> f32 {
    (to.x - from.x).hypot(to.y - from.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{EffectBlendMode, MoveCoordinateMode, MoveDirection};

    fn request(points: Vec<MovePathPoint>) -> MoveEffectRequest {
        MoveEffectRequest {
            label: "Move test".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            beam_targets: Vec::new(),
            points,
            closed: false,
            interpolation: MoveInterpolation::Line,
            coordinate_mode: MoveCoordinateMode::Absolute,
            center_x: 0.5,
            center_y: 0.5,
            size_x: 1.0,
            size_y: 1.0,
            rotation_degrees: 0.0,
            period_ms: 1_000,
            clock_sync: None,
            direction: MoveDirection::Forward,
            phase: 0.0,
            fixture_spread: 0.0,
            symmetry: false,
            blend_mode: EffectBlendMode::Override,
        }
    }

    #[test]
    fn line_path_uses_arc_length_instead_of_point_index() {
        let request = request(vec![
            MovePathPoint { x: 0.0, y: 0.0 },
            MovePathPoint { x: 0.1, y: 0.0 },
            MovePathPoint { x: 1.0, y: 0.0 },
        ]);
        let path = CompiledMovePath::compile(&request).unwrap();

        let halfway = path.sample(0.5);

        assert!((halfway.x - 0.5).abs() < 1.0e-5);
        assert_eq!(halfway.y, 0.0);
    }

    #[test]
    fn closed_path_finishes_at_its_first_point() {
        let mut request = request(vec![
            MovePathPoint { x: 0.2, y: 0.2 },
            MovePathPoint { x: 0.8, y: 0.2 },
            MovePathPoint { x: 0.8, y: 0.8 },
        ]);
        request.closed = true;
        let path = CompiledMovePath::compile(&request).unwrap();

        let endpoint = path.sample(1.0);
        assert!((endpoint.x - request.points[0].x).abs() < 1.0e-5);
        assert!((endpoint.y - request.points[0].y).abs() < 1.0e-5);
    }

    #[test]
    fn smooth_path_is_finite_and_preserves_open_endpoints() {
        let mut request = request(vec![
            MovePathPoint { x: 0.1, y: 0.2 },
            MovePathPoint { x: 0.3, y: 0.9 },
            MovePathPoint { x: 0.9, y: 0.7 },
        ]);
        request.interpolation = MoveInterpolation::Smooth;
        let path = CompiledMovePath::compile(&request).unwrap();

        let middle = path.sample(0.5);
        assert!(middle.x.is_finite() && middle.y.is_finite());
        assert_eq!(path.sample(0.0), request.points[0]);
        assert_eq!(path.sample(1.0), request.points[2]);
    }

    #[test]
    fn circle_square_uses_exact_equal_time_quarter_arcs() {
        let mut request = request(vec![
            MovePathPoint { x: 0.25, y: 0.5 },
            MovePathPoint { x: 0.5, y: 0.75 },
            MovePathPoint { x: 0.75, y: 0.5 },
            MovePathPoint { x: 0.5, y: 0.25 },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let first_arc_midpoint = path.sample(0.125);
        assert!((first_arc_midpoint.x - 0.323_223_3).abs() < 1.0e-6);
        assert!((first_arc_midpoint.y - 0.676_776_7).abs() < 1.0e-6);
        assert_eq!(path.sample(0.25), request.points[1]);
        assert_eq!(path.sample(1.0), request.points[0]);
    }

    #[test]
    fn circle_uses_equal_control_segment_time_with_unequal_radii() {
        let mut request = request(vec![
            MovePathPoint { x: 0.08, y: 0.12 },
            MovePathPoint { x: 0.18, y: 0.82 },
            MovePathPoint { x: 0.92, y: 0.72 },
            MovePathPoint { x: 0.66, y: 0.18 },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let CompiledMovePathKind::Circle { segments } = &path.kind else {
            panic!("Circle must retain analytical segments");
        };
        let radius = |segment: &CompiledCircleSegment| match segment {
            CompiledCircleSegment::Arc(arc) => arc.radius,
            CompiledCircleSegment::Inflection { first_half, .. } => first_half.radius,
            CompiledCircleSegment::Linear { .. } => 0.0,
        };
        assert!((radius(&segments[0]) - radius(&segments[1])).abs() > 1.0e-3);
        let unequal_arc_midpoint = path.sample(0.375);
        assert!((unequal_arc_midpoint.x - 0.574_851_0).abs() < 2.0e-6);
        assert!((unequal_arc_midpoint.y - 0.953_897_36).abs() < 2.0e-6);
        for (index, point) in request.points.iter().enumerate() {
            let sampled = path.sample(index as f32 / request.points.len() as f32);
            assert!((sampled.x - point.x).abs() < 1.0e-6, "point {index} X");
            assert!((sampled.y - point.y).abs() < 1.0e-6, "point {index} Y");
        }
    }

    #[test]
    fn circle_opposite_signed_curvature_uses_recovered_mirrored_half_arcs() {
        let mut request = request(vec![
            MovePathPoint {
                x: 0.459_197_01,
                y: 0.546_359_5,
            },
            MovePathPoint {
                x: 0.266_559_27,
                y: 0.064_987_13,
            },
            MovePathPoint {
                x: 0.419_338_2,
                y: 0.116_473_87,
            },
            MovePathPoint {
                x: 0.452_318_88,
                y: 0.677_404_64,
            },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let first_quarter = path.sample(0.0625);
        let third_quarter = path.sample(0.1875);
        assert!((first_quarter.x - 0.442_961_54).abs() < 2.0e-6);
        assert!((first_quarter.y - 0.413_240_9).abs() < 2.0e-6);
        assert!((third_quarter.x - 0.282_794_74).abs() < 2.0e-6);
        assert!((third_quarter.y - 0.198_105_68).abs() < 2.0e-6);
        assert!(
            (first_quarter.x + third_quarter.x - request.points[0].x - request.points[1].x).abs()
                < 2.0e-6
        );
        assert!(
            (first_quarter.y + third_quarter.y - request.points[0].y - request.points[1].y).abs()
                < 2.0e-6
        );
    }

    #[test]
    fn circle_inflection_reflects_raw_arc_before_final_output_clamp() {
        let mut request = request(vec![
            MovePathPoint {
                x: 0.058_411_848,
                y: 0.102_016_88,
            },
            MovePathPoint {
                x: 0.132_351_59,
                y: 0.955_008_1,
            },
            MovePathPoint {
                x: 0.094_556_22,
                y: 0.951_056_5,
            },
            MovePathPoint {
                x: 0.837_385_65,
                y: 0.753_317_95,
            },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let reflected = path.sample(0.1875);
        assert!((reflected.x - 0.247_971_55).abs() < 2.0e-6);
        assert!((reflected.y - 0.730_135_7).abs() < 2.0e-6);
        assert!((reflected.x - 0.190_763_44).abs() > 0.05);
    }

    #[test]
    fn two_point_circle_is_a_full_analytical_rotation() {
        let mut request = request(vec![
            MovePathPoint { x: 0.25, y: 0.5 },
            MovePathPoint { x: 0.75, y: 0.5 },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let quarter = path.sample(0.25);
        assert!((quarter.x - 0.5).abs() < 1.0e-6);
        assert!((quarter.y - 0.25).abs() < 1.0e-6);
        assert_eq!(path.sample(0.5), request.points[1]);
        assert_eq!(path.sample(1.0), request.points[0]);
    }

    #[test]
    fn degenerate_circle_segments_fall_back_to_equal_time_lines() {
        let mut request = request(vec![
            MovePathPoint { x: 0.1, y: 0.5 },
            MovePathPoint { x: 0.3, y: 0.5 },
            MovePathPoint { x: 0.8, y: 0.5 },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let first_midpoint = path.sample(1.0 / 6.0);
        assert!((first_midpoint.x - 0.2).abs() < 1.0e-6);
        assert!((first_midpoint.y - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn circle_preserves_duplicate_points_as_equal_time_degenerate_segments() {
        let mut request = request(vec![
            MovePathPoint { x: 0.1, y: 0.5 },
            MovePathPoint { x: 0.1, y: 0.5 },
            MovePathPoint { x: 0.9, y: 0.5 },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Circle;
        let path = CompiledMovePath::compile(&request).unwrap();

        let CompiledMovePathKind::Circle { segments } = &path.kind else {
            panic!("Circle must retain its raw point segmentation");
        };
        assert_eq!(segments.len(), 3);
        let duplicate_midpoint = path.sample(1.0 / 6.0);
        assert!((duplicate_midpoint.x - request.points[0].x).abs() < 1.0e-6);
        assert!((duplicate_midpoint.y - request.points[0].y).abs() < 1.0e-6);
        let duplicate_boundary = path.sample(1.0 / 3.0);
        assert!((duplicate_boundary.x - request.points[1].x).abs() < 1.0e-6);
        assert!((duplicate_boundary.y - request.points[1].y).abs() < 1.0e-6);
    }

    #[test]
    fn zero_length_path_is_rejected() {
        let request = request(vec![
            MovePathPoint { x: 0.5, y: 0.5 },
            MovePathPoint { x: 0.5, y: 0.5 },
        ]);

        assert_eq!(
            CompiledMovePath::compile(&request).unwrap_err(),
            "Move effect path must contain at least two distinct points"
        );
    }

    #[test]
    fn transform_delta_applies_scale_then_rotation() {
        let mut request = request(vec![
            MovePathPoint { x: 0.5, y: 0.5 },
            MovePathPoint { x: 1.0, y: 0.5 },
        ]);
        request.center_x = 0.25;
        request.center_y = 0.75;
        request.size_x = 0.5;
        request.rotation_degrees = 90.0;

        let (cosine, sine) = move_rotation(request.rotation_degrees);
        let transformed =
            transform_move_delta(&request, MovePathPoint { x: 1.0, y: 0.5 }, cosine, sine);

        assert!(transformed.x.abs() < 1.0e-5);
        assert!((transformed.y + 0.25).abs() < 1.0e-5);
    }

    #[test]
    fn duplicate_terminal_and_near_duplicate_points_are_stable() {
        let mut request = request(vec![
            MovePathPoint { x: 0.2, y: 0.2 },
            MovePathPoint {
                x: 0.200_000_1,
                y: 0.2,
            },
            MovePathPoint { x: 0.8, y: 0.2 },
            MovePathPoint { x: 0.2, y: 0.2 },
        ]);
        request.closed = true;
        request.interpolation = MoveInterpolation::Smooth;

        let path = CompiledMovePath::compile(&request).unwrap();

        for index in 0..=100 {
            let point = path.sample(index as f32 / 100.0);
            assert!(point.x.is_finite() && point.y.is_finite());
        }
    }

    #[test]
    fn non_finite_progress_falls_back_to_path_start() {
        let request = request(vec![
            MovePathPoint { x: 0.2, y: 0.2 },
            MovePathPoint { x: 0.8, y: 0.8 },
        ]);
        let path = CompiledMovePath::compile(&request).unwrap();

        assert_eq!(path.sample(f32::NAN), request.points[0]);
    }

    #[test]
    fn corrected_daslight_line_reaches_both_endpoints_continuously() {
        let mut request = request(vec![
            MovePathPoint { x: 0.0, y: 0.25 },
            MovePathPoint { x: 1.0, y: 0.75 },
        ]);
        request.interpolation = MoveInterpolation::DaslightLine;
        let path = CompiledMovePath::compile(&request).unwrap();

        let expected_x = [0.0, 0.5, 1.0, 0.5];
        for (phase, expected) in [0.0, 0.25, 0.5, 0.75].into_iter().zip(expected_x) {
            let point = path.sample_daslight_phase(phase);
            assert!((point.x - expected).abs() < 1.0e-6, "phase {phase}");
        }
    }

    #[test]
    fn daslight_polygon_uses_equal_time_per_authored_edge() {
        let mut request = request(vec![
            MovePathPoint { x: 0.0, y: 0.0 },
            MovePathPoint { x: 1.0, y: 0.0 },
            MovePathPoint { x: 1.0, y: 1.0 },
            MovePathPoint { x: 0.0, y: 1.0 },
        ]);
        request.interpolation = MoveInterpolation::DaslightPolygon;
        let path = CompiledMovePath::compile(&request).unwrap();

        assert_eq!(
            path.sample_daslight_phase(0.125),
            MovePathPoint { x: 0.5, y: 0.0 }
        );
        assert_eq!(
            path.sample_daslight_phase(0.25),
            MovePathPoint { x: 1.0, y: 0.0 }
        );
        assert_eq!(
            path.sample_daslight_phase(0.875),
            MovePathPoint { x: 0.0, y: 0.5 }
        );
    }

    #[test]
    fn corrected_daslight_points_holds_each_authored_vertex_for_equal_time() {
        let mut request = request(vec![
            MovePathPoint { x: 0.1, y: 0.2 },
            MovePathPoint { x: 0.5, y: 0.6 },
            MovePathPoint { x: 0.9, y: 1.0 },
        ]);
        request.interpolation = MoveInterpolation::DaslightPoints;
        let path = CompiledMovePath::compile(&request).unwrap();

        assert_eq!(path.sample_daslight_phase(0.0), request.points[0]);
        assert_eq!(path.sample_daslight_phase(0.32), request.points[0]);
        assert_eq!(path.sample_daslight_phase(0.34), request.points[1]);
        assert_eq!(path.sample_daslight_phase(0.67), request.points[2]);
        assert_eq!(path.sample_daslight_phase(0.99), request.points[2]);
    }

    #[test]
    fn daslight_curve_uses_uniform_catmull_rom_and_arc_length_triangle_time() {
        let mut request = request(vec![
            MovePathPoint {
                x: 1.0 / 6.0,
                y: 0.25,
            },
            MovePathPoint {
                x: 1.0 / 3.0,
                y: 0.75,
            },
            MovePathPoint {
                x: 2.0 / 3.0,
                y: 0.25,
            },
            MovePathPoint {
                x: 5.0 / 6.0,
                y: 0.75,
            },
        ]);
        request.interpolation = MoveInterpolation::DaslightCurve;
        let path = CompiledMovePath::compile(&request).unwrap();

        let halfway = path.sample_daslight_phase(0.25);
        assert!((halfway.x - 0.5).abs() < 1.0e-5);
        assert!((halfway.y - 0.5).abs() < 1.0e-5);
        assert_eq!(path.sample_daslight_phase(0.5), request.points[3]);
        assert_eq!(path.sample_daslight_phase(0.0), request.points[0]);
        assert_eq!(path.sample_daslight_phase(0.75), halfway);
    }
}
