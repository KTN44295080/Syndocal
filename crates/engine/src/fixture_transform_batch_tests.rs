use super::*;
use std::sync::{mpsc, RwLock};
use std::time::{Duration, Instant};

fn fixture_transform_batch_runtime() -> EngineRuntime {
    let mut runtime = EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    for (fixture_id, label, address) in [(1, "Batch A", 1), (2, "Batch B", 20)] {
        runtime.apply_command(EngineCommand::PatchFixture {
            fixture_id,
            request: sample_patch_request(label, address),
            profile: sample_profile(),
        });
    }
    runtime.last_error = None;
    runtime
}

fn batch_mutation() -> StageProjectMutation {
    StageProjectMutation::SetFixtureTransforms {
        transforms: vec![
            StageFixtureTransform {
                fixture_id: 1,
                position: Vec3 {
                    x: 4.0,
                    y: 5.0,
                    z: -6.0,
                },
                rotation: Rotation3 {
                    pitch: 10.0,
                    yaw: 20.0,
                    roll: 30.0,
                },
            },
            StageFixtureTransform {
                fixture_id: 2,
                position: Vec3 {
                    x: -7.0,
                    y: 8.0,
                    z: 9.0,
                },
                rotation: Rotation3 {
                    pitch: -11.0,
                    yaw: 42.0,
                    roll: -13.0,
                },
            },
        ],
    }
}

fn publish_batch(
    runtime: &mut EngineRuntime,
    shared: &RwLock<EngineSnapshot>,
    mutation: StageProjectMutation,
) -> Result<StageProjectMutationOutcome, String> {
    let admission = ProjectSnapshotLoadAdmission::new();
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::StageProjectMutationPublished {
        mutation,
        expires_at: Instant::now() + Duration::from_secs(1),
        admission,
        ack,
    });
    runtime.publish_pending_command_acks(0, shared);
    receiver.recv().expect("batch publication acknowledgement")
}

#[test]
fn fixture_transform_batch_applies_both_fixtures_as_one_published_mutation() {
    let mut runtime = fixture_transform_batch_runtime();
    let before = runtime.build_persistence_snapshot();
    let shared = RwLock::new(before.clone());

    assert_eq!(
        publish_batch(&mut runtime, &shared, batch_mutation()),
        Ok(StageProjectMutationOutcome::Applied)
    );
    assert_eq!(runtime.fixtures[0].request.position.x, 4.0);
    assert_eq!(runtime.fixtures[0].request.rotation.yaw, 20.0);
    assert_eq!(runtime.fixtures[1].request.position.z, 9.0);
    assert_eq!(runtime.fixtures[1].request.rotation.roll, -13.0);
    let published = shared.read().unwrap().clone();
    assert_eq!(published.fixtures[0].position.x, 4.0);
    assert_eq!(published.fixtures[1].position.z, 9.0);
    assert_ne!(published, before);
}

#[test]
fn fixture_transform_batch_rejects_missing_last_without_partial_apply() {
    let mut runtime = fixture_transform_batch_runtime();
    runtime.last_error = Some("before batch".to_string());
    let before = runtime.build_persistence_snapshot();
    let shared = RwLock::new(before.clone());
    let mutation = StageProjectMutation::SetFixtureTransforms {
        transforms: vec![
            StageFixtureTransform {
                fixture_id: 1,
                position: Vec3 {
                    x: 100.0,
                    y: 0.0,
                    z: 0.0,
                },
                rotation: Rotation3::default(),
            },
            StageFixtureTransform {
                fixture_id: 999,
                position: Vec3::default(),
                rotation: Rotation3::default(),
            },
        ],
    };

    assert!(publish_batch(&mut runtime, &shared, mutation).is_err());
    assert_eq!(runtime.build_persistence_snapshot(), before);
    assert_eq!(*shared.read().unwrap(), before);
    assert_eq!(runtime.last_error.as_deref(), Some("before batch"));
}

#[test]
fn fixture_transform_batch_limit_and_unchanged_preserve_snapshot() {
    let mut runtime = fixture_transform_batch_runtime();
    let before = runtime.build_persistence_snapshot();
    let shared = RwLock::new(before.clone());
    let oversized = StageProjectMutation::SetFixtureTransforms {
        transforms: vec![
            StageFixtureTransform {
                fixture_id: 1,
                position: Vec3::default(),
                rotation: Rotation3::default(),
            };
            MAX_FIXTURE_TRANSFORM_BATCH + 1
        ],
    };
    assert!(publish_batch(&mut runtime, &shared, oversized)
        .unwrap_err()
        .contains("at most"));
    assert_eq!(runtime.build_persistence_snapshot(), before);
    assert_eq!(*shared.read().unwrap(), before);
    assert_eq!(
        publish_batch(&mut runtime, &shared, batch_mutation()),
        Ok(StageProjectMutationOutcome::Applied)
    );
    let changed = runtime.build_persistence_snapshot();
    let published = shared.read().unwrap().clone();
    assert_eq!(
        publish_batch(&mut runtime, &shared, batch_mutation()),
        Ok(StageProjectMutationOutcome::Unchanged)
    );
    assert_eq!(runtime.build_persistence_snapshot(), changed);
    assert_eq!(*shared.read().unwrap(), published);
}

#[test]
fn fixture_transform_batch_validation_rejects_invalid_shapes_before_any_write() {
    let invalid_mutations = [
        StageProjectMutation::SetFixtureTransforms {
            transforms: Vec::new(),
        },
        StageProjectMutation::SetFixtureTransforms {
            transforms: vec![
                StageFixtureTransform {
                    fixture_id: 1,
                    position: Vec3::default(),
                    rotation: Rotation3::default(),
                },
                StageFixtureTransform {
                    fixture_id: 1,
                    position: Vec3::default(),
                    rotation: Rotation3::default(),
                },
            ],
        },
        StageProjectMutation::SetFixtureTransforms {
            transforms: vec![StageFixtureTransform {
                fixture_id: 1,
                position: Vec3 {
                    x: f32::NAN,
                    y: 0.0,
                    z: 0.0,
                },
                rotation: Rotation3::default(),
            }],
        },
    ];

    for mutation in invalid_mutations {
        let mut runtime = fixture_transform_batch_runtime();
        let before = runtime.build_persistence_snapshot();
        let shared = RwLock::new(before.clone());
        assert!(publish_batch(&mut runtime, &shared, mutation).is_err());
        assert_eq!(runtime.build_persistence_snapshot(), before);
        assert_eq!(*shared.read().unwrap(), before);
    }
}

#[test]
fn fixture_transform_batch_publication_failure_restores_both_fixtures() {
    let mut runtime = fixture_transform_batch_runtime();
    let before = runtime.build_persistence_snapshot();
    let shared = RwLock::new(before.clone());
    runtime.fail_next_pending_publication = true;

    assert!(publish_batch(&mut runtime, &shared, batch_mutation()).is_err());
    assert_eq!(runtime.build_persistence_snapshot(), before);
    assert_eq!(*shared.read().unwrap(), before);
}
