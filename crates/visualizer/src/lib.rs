use protocol::{EngineSnapshot, FixtureId, PatchedFixtureSummary, Vec3};
use serde::{Deserialize, Serialize};

const FULL_SCALE: f32 = 65_535.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VisualizerConfig {
    pub beam_length: f32,
    pub beam_radius: f32,
    pub minimum_beam_intensity: f32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            beam_length: 12.0,
            beam_radius: 0.18,
            minimum_beam_intensity: 0.01,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizerScene {
    pub fixtures: Vec<FixtureNode>,
    pub beams: Vec<BeamNode>,
    pub bounds: StageBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureNode {
    pub id: FixtureId,
    pub label: String,
    pub position: Vec3,
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub roll_deg: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeamNode {
    pub fixture_id: FixtureId,
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StageBounds {
    pub min: Vec3,
    pub max: Vec3,
}

pub fn build_visualizer_scene(
    snapshot: &EngineSnapshot,
    config: VisualizerConfig,
) -> VisualizerScene {
    let fixtures = snapshot
        .fixtures
        .iter()
        .map(fixture_node_from_summary)
        .collect::<Vec<_>>();
    let beams = fixtures
        .iter()
        .filter(|fixture| fixture.intensity >= config.minimum_beam_intensity)
        .map(|fixture| BeamNode {
            fixture_id: fixture.id,
            origin: fixture.position,
            direction: beam_direction(fixture.yaw_deg, fixture.pitch_deg),
            length: config.beam_length.max(0.0),
            radius: config.beam_radius.max(0.0),
            color: fixture.color,
            intensity: fixture.intensity,
        })
        .collect::<Vec<_>>();
    let bounds = stage_bounds(snapshot.fixtures.as_slice());

    VisualizerScene {
        fixtures,
        beams,
        bounds,
    }
}

fn fixture_node_from_summary(fixture: &PatchedFixtureSummary) -> FixtureNode {
    let dimmer = read_attribute(fixture, &["Dimmer", "Intensity"]).unwrap_or(0);
    let pan_deg = read_attribute(fixture, &["Pan"])
        .map(|value| ((value as f32 - 32_768.0) / FULL_SCALE) * 540.0)
        .unwrap_or(0.0);
    let tilt_deg = read_attribute(fixture, &["Tilt"])
        .map(|value| ((value as f32 - 32_768.0) / FULL_SCALE) * 270.0)
        .unwrap_or(0.0);
    let red = read_attribute(fixture, &["ColorRed", "Red"])
        .map(normalized_u16)
        .unwrap_or(0.34);
    let green = read_attribute(fixture, &["ColorGreen", "Green"])
        .map(normalized_u16)
        .unwrap_or(0.65);
    let blue = read_attribute(fixture, &["ColorBlue", "Blue"])
        .map(normalized_u16)
        .unwrap_or(0.96);

    FixtureNode {
        id: fixture.id,
        label: fixture.label.clone(),
        position: fixture.position,
        yaw_deg: fixture.rotation.yaw + pan_deg,
        pitch_deg: fixture.rotation.pitch + tilt_deg,
        roll_deg: fixture.rotation.roll,
        color: [red, green, blue],
        intensity: normalized_u16(dimmer),
    }
}

fn read_attribute(fixture: &PatchedFixtureSummary, names: &[&str]) -> Option<u16> {
    fixture.attribute_values.iter().find_map(|value| {
        names
            .iter()
            .any(|name| value.attribute.eq_ignore_ascii_case(name))
            .then_some(value.value)
    })
}

fn normalized_u16(value: u16) -> f32 {
    (value as f32 / FULL_SCALE).clamp(0.0, 1.0)
}

fn beam_direction(yaw_deg: f32, pitch_deg: f32) -> Vec3 {
    let yaw = yaw_deg.to_radians();
    let pitch = pitch_deg.to_radians();
    let horizontal = pitch.cos();
    normalize(Vec3 {
        x: yaw.sin() * horizontal,
        y: -pitch.sin(),
        z: yaw.cos() * horizontal,
    })
}

fn normalize(vector: Vec3) -> Vec3 {
    let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
    if length <= f32::EPSILON {
        return Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
    }
    Vec3 {
        x: vector.x / length,
        y: vector.y / length,
        z: vector.z / length,
    }
}

fn stage_bounds(fixtures: &[PatchedFixtureSummary]) -> StageBounds {
    let Some(first) = fixtures.first() else {
        return StageBounds {
            min: Vec3::default(),
            max: Vec3::default(),
        };
    };
    fixtures.iter().fold(
        StageBounds {
            min: first.position,
            max: first.position,
        },
        |bounds, fixture| StageBounds {
            min: Vec3 {
                x: bounds.min.x.min(fixture.position.x),
                y: bounds.min.y.min(fixture.position.y),
                z: bounds.min.z.min(fixture.position.z),
            },
            max: Vec3 {
                x: bounds.max.x.max(fixture.position.x),
                y: bounds.max.y.max(fixture.position.y),
                z: bounds.max.z.max(fixture.position.z),
            },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{
        AttributeControl, AttributeResolution, AttributeValueSummary, EngineSnapshot, Rotation3,
    };

    #[test]
    fn builds_fixture_nodes_and_beams_from_engine_snapshot() {
        let snapshot = EngineSnapshot {
            fixtures: vec![fixture(
                1,
                "Key",
                Vec3 {
                    x: -2.0,
                    y: 5.0,
                    z: 1.0,
                },
                vec![
                    ("Dimmer", 65_535),
                    ("Pan", 32_768),
                    ("Tilt", 32_768),
                    ("ColorRed", 65_535),
                    ("ColorGreen", 0),
                    ("ColorBlue", 0),
                ],
            )],
            ..EngineSnapshot::default()
        };

        let scene = build_visualizer_scene(&snapshot, VisualizerConfig::default());

        assert_eq!(scene.fixtures.len(), 1);
        assert_eq!(scene.beams.len(), 1);
        assert_eq!(scene.fixtures[0].label, "Key");
        assert_eq!(scene.fixtures[0].color, [1.0, 0.0, 0.0]);
        assert_eq!(scene.fixtures[0].intensity, 1.0);
        assert!((scene.beams[0].direction.z - 1.0).abs() < 0.001);
        assert_eq!(scene.bounds.min.x, -2.0);
        assert_eq!(scene.bounds.max.y, 5.0);
    }

    #[test]
    fn omits_beams_below_configured_intensity_threshold() {
        let snapshot = EngineSnapshot {
            fixtures: vec![fixture(
                1,
                "Dim",
                Vec3::default(),
                vec![("Dimmer", 10), ("Pan", 32_768), ("Tilt", 32_768)],
            )],
            ..EngineSnapshot::default()
        };

        let scene = build_visualizer_scene(
            &snapshot,
            VisualizerConfig {
                minimum_beam_intensity: 0.5,
                ..VisualizerConfig::default()
            },
        );

        assert_eq!(scene.fixtures.len(), 1);
        assert!(scene.beams.is_empty());
    }

    fn fixture(
        id: FixtureId,
        label: &str,
        position: Vec3,
        values: Vec<(&str, u16)>,
    ) -> PatchedFixtureSummary {
        let controls = values
            .iter()
            .enumerate()
            .map(|(index, (attribute, _))| AttributeControl {
                attribute: (*attribute).to_string(),
                channel_name: (*attribute).to_string(),
                geometry: None,
                offsets: vec![index as u16 + 1],
                resolution: AttributeResolution::EightBit,
                default_value: 0,
                functions: Vec::new(),
            })
            .collect::<Vec<_>>();
        let attribute_values = values
            .into_iter()
            .map(|(attribute, value)| AttributeValueSummary {
                attribute: attribute.to_string(),
                value,
            })
            .collect::<Vec<_>>();

        PatchedFixtureSummary {
            id,
            label: label.to_string(),
            profile_name: "Fixture".to_string(),
            manufacturer: "KDMX".to_string(),
            mode_name: "Default".to_string(),
            universe: 0,
            address: 1,
            group_ids: Vec::new(),
            position,
            rotation: Rotation3::default(),
            controls,
            attribute_values,
            limits: Default::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }
}
