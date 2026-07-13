use protocol::{MoveEffectRequest, MoveInterpolation, MovePathPoint};

const SMOOTH_SAMPLES_PER_SEGMENT: usize = 32;
const MIN_PATH_LENGTH: f32 = 1.0e-6;

#[derive(Debug, Clone)]
pub(super) struct CompiledMovePath {
    samples: Vec<MoveArcSample>,
    total_length: f32,
}

#[derive(Debug, Clone, Copy)]
struct MoveArcSample {
    distance: f32,
    point: MovePathPoint,
}

impl CompiledMovePath {
    pub(super) fn compile(request: &MoveEffectRequest) -> Result<Self, String> {
        if request.points.len() < 2 {
            return Err("Move effect requires at least two path points".to_string());
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
            samples,
            total_length,
        })
    }

    pub(super) fn sample(&self, progress: f32) -> MovePathPoint {
        let progress = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if progress <= 0.0 {
            return self.samples[0].point;
        }
        if progress >= 1.0 {
            return self.samples[self.samples.len() - 1].point;
        }

        let target_distance = progress * self.total_length;
        let mut lower = 0_usize;
        let mut upper = self.samples.len() - 1;
        while lower + 1 < upper {
            let middle = lower + (upper - lower) / 2;
            if self.samples[middle].distance < target_distance {
                lower = middle;
            } else {
                upper = middle;
            }
        }

        let from = self.samples[lower];
        let to = self.samples[upper];
        let distance = (to.distance - from.distance).max(MIN_PATH_LENGTH);
        let t = ((target_distance - from.distance) / distance).clamp(0.0, 1.0);
        MovePathPoint {
            x: from.point.x + (to.point.x - from.point.x) * t,
            y: from.point.y + (to.point.y - from.point.y) * t,
        }
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
}
