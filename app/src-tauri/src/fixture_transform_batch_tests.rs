use super::*;
use serde_json::json;
use std::sync::atomic::AtomicU64;

fn seed_two_stage_fixtures(state: &AppState) {
    let mut profile = sample_patch_profile();
    // Match the existing D4 transform fixture: no unregistered custom profile.
    profile.source_path = "fixture://d4-stage-fixture".to_string();
    let mut first = sample_patch_request(0, 1);
    first.label = "Batch Fixture A".to_string();
    first.profile_path = profile.source_path.clone();
    let mut second = sample_patch_request(0, 20);
    second.label = "Batch Fixture B".to_string();
    second.profile_path = profile.source_path.clone();
    state
        .engine
        .patch_fixtures_published(vec![
            engine::FixturePatchCandidate {
                fixture_id: 1,
                request: first,
                profile: profile.clone(),
            },
            engine::FixturePatchCandidate {
                fixture_id: 2,
                request: second,
                profile,
            },
        ])
        .unwrap();
    d4_reconcile_stage_authority(state);
}

fn batch_mutation() -> StageProjectMutation {
    StageProjectMutation::SetFixtureTransforms {
        transforms: vec![
            StageFixtureTransform {
                fixture_id: 1,
                position: Vec3 {
                    x: 3.0,
                    y: 4.0,
                    z: -5.0,
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
                    x: -6.0,
                    y: 7.0,
                    z: 8.0,
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

fn fixture_transform(snapshot: &EngineSnapshot, fixture_id: FixtureId) -> (Vec3, Rotation3) {
    let fixture = snapshot
        .fixtures
        .iter()
        .find(|fixture| fixture.id == fixture_id)
        .unwrap_or_else(|| panic!("fixture {fixture_id} must be present"));
    (fixture.position, fixture.rotation)
}

fn stage_image(state: &AppState) -> EngineSnapshot {
    project_snapshot_for_save(state.engine.persistence_snapshot().unwrap())
}

#[test]
fn set_fixture_transforms_request_validation_is_bounded_and_exact() {
    let valid = SetFixtureTransformBatchItem {
        fixture_id: 1,
        position: Vec3::default(),
        rotation: Rotation3::default(),
    };
    assert!(validate_fixture_transform_batch(std::slice::from_ref(&valid)).is_ok());
    assert!(validate_fixture_transform_batch(&[]).is_err());
    assert!(validate_fixture_transform_batch(&[valid.clone(), valid.clone()]).is_err());

    let mut non_finite = valid.clone();
    non_finite.rotation.yaw = f32::INFINITY;
    assert!(validate_fixture_transform_batch(&[non_finite]).is_err());

    let oversize = vec![valid; engine::MAX_FIXTURE_TRANSFORM_BATCH + 1];
    let error = validate_fixture_transform_batch(&oversize).unwrap_err();
    assert!(error.contains("at most"));
}

fn stage_history_status(state: &AppState) -> ProjectHistoryStatus {
    let coordinator = lock_project_coordinator(state).unwrap();
    project_history_status_for_coordinator(&coordinator)
}

struct BatchHistoryPlatform;

impl ProjectReplacementPlatform for BatchHistoryPlatform {
    fn advance_recovery_authority(
        &self,
        _state: &AppState,
        coordinator: &mut ProjectCoordinator,
        transition: ProjectRecoveryAuthorityTransition,
    ) -> Result<u64, String> {
        advance_project_recovery_authority_serial_with_persist(coordinator, transition, |_, _| {
            Ok(())
        })
    }

    fn fence_and_retire_outputs(&self, _state: &AppState) -> Result<(), String> {
        Ok(())
    }

    fn emit_authority_event(
        &self,
        _coordinator_effect: ProjectReplacementCoordinatorEffect,
        _result: &ProjectLoadResult,
    ) {
    }
}

fn navigate_stage_history(
    state: &AppState,
    undo: bool,
    expected_epoch: u64,
    entry_id: Option<u64>,
    checkpoint_hash: Option<String>,
) -> ProjectHistoryNavigationResult {
    let _external = lock_project_external_command_admission(state).unwrap();
    let mut coordinator = state.project_coordinator.lock().unwrap();
    navigate_project_history_with_coordinator(
        state,
        &mut coordinator,
        undo,
        Some(expected_epoch),
        entry_id,
        checkpoint_hash,
        |state, prepared, coordinator| {
            replace_prepared_project_snapshot_with_coordinator_and_platform(
                state,
                prepared,
                coordinator,
                project_replacement_plan!(
                    ProjectReplacementCoordinatorEffect::RevisionMutation,
                    false,
                    no_project_replacement_validation,
                    no_project_replacement_capture,
                    None
                ),
                &BatchHistoryPlatform,
            )
            .map(|(result, (), _)| result)
        },
    )
    .expect("stage history navigation must succeed")
}

#[test]
fn stage_fixture_transform_batch_is_one_undo_redo_entry_for_two_fixtures() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let window_label = "stage-transform-batch-history-window";
    let owner_id = "stage-transform-batch-history-owner";
    register_project_transaction_owner_for_window_label(state, window_label, owner_id.to_string())
        .unwrap();
    seed_two_stage_fixtures(state);

    let image_a = stage_image(state);
    let status_a = stage_history_status(state);
    let ticket =
        d4_begin_stage_ticket(state, window_label, owner_id, "set_fixture_transforms", 401);
    let result = d4_apply_stage_mutation(
        state,
        window_label,
        owner_id,
        "set_fixture_transforms",
        &ticket,
        json!({
            "transforms": [
                { "fixtureId": 1, "position": { "x": 3.0, "y": 4.0, "z": -5.0 } },
                { "fixtureId": 2, "position": { "x": -6.0, "y": 7.0, "z": 8.0 } }
            ]
        }),
        batch_mutation(),
        None,
        None,
        &AtomicU64::new(0),
    )
    .unwrap();
    assert_eq!(
        d4_stage_result_outcome(&result, "set_fixture_transforms"),
        StageProjectMutationReceiptOutcome::Applied
    );
    d4_commit_stage_ticket(
        state,
        window_label,
        owner_id,
        "set_fixture_transforms",
        &ticket,
    );

    let status_b = stage_history_status(state);
    assert_eq!(status_b.undo_depth, status_a.undo_depth + 1);
    assert_eq!(status_b.undo_entry_id, Some(ticket.transaction_id));
    assert_eq!(status_b.redo_depth, 0);
    let image_b = stage_image(state);
    assert_eq!(
        fixture_transform(&image_b, 1),
        (
            Vec3 {
                x: 3.0,
                y: 4.0,
                z: -5.0
            },
            Rotation3 {
                pitch: 10.0,
                yaw: 20.0,
                roll: 30.0
            }
        )
    );
    assert_eq!(
        fixture_transform(&image_b, 2),
        (
            Vec3 {
                x: -6.0,
                y: 7.0,
                z: 8.0
            },
            Rotation3 {
                pitch: -11.0,
                yaw: 42.0,
                roll: -13.0
            }
        )
    );

    let undone = navigate_stage_history(
        state,
        true,
        status_b.project_epoch,
        status_b.undo_entry_id,
        Some(status_b.checkpoint_hash.clone()),
    );
    assert_eq!(undone.history_status.undo_depth, status_a.undo_depth);
    assert_eq!(undone.history_status.redo_depth, status_a.redo_depth + 1);
    assert_eq!(
        fixture_transform(&stage_image(state), 1),
        fixture_transform(&image_a, 1)
    );
    assert_eq!(
        fixture_transform(&stage_image(state), 2),
        fixture_transform(&image_a, 2)
    );

    let redone = navigate_stage_history(
        state,
        false,
        undone.history_status.project_epoch,
        undone.history_status.redo_entry_id,
        Some(undone.history_status.redo_checkpoint_hash.clone().unwrap()),
    );
    assert_eq!(redone.history_status.undo_depth, status_a.undo_depth + 1);
    assert_eq!(redone.history_status.redo_depth, status_a.redo_depth);
    assert_eq!(
        fixture_transform(&stage_image(state), 1),
        fixture_transform(&image_b, 1)
    );
    assert_eq!(
        fixture_transform(&stage_image(state), 2),
        fixture_transform(&image_b, 2)
    );
}

#[test]
fn stage_fixture_transform_batch_missing_last_preserves_image_and_history() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let window_label = "stage-transform-batch-invalid-window";
    let owner_id = "stage-transform-batch-invalid-owner";
    register_project_transaction_owner_for_window_label(state, window_label, owner_id.to_string())
        .unwrap();
    seed_two_stage_fixtures(state);
    let before_image = stage_image(state);
    let before_status = stage_history_status(state);
    let ticket =
        d4_begin_stage_ticket(state, window_label, owner_id, "set_fixture_transforms", 402);
    let error = d4_apply_stage_mutation(
        state,
        window_label,
        owner_id,
        "set_fixture_transforms",
        &ticket,
        json!({ "transforms": [1, 999] }),
        StageProjectMutation::SetFixtureTransforms {
            transforms: vec![
                StageFixtureTransform {
                    fixture_id: 1,
                    position: Vec3 {
                        x: 99.0,
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
        },
        None,
        None,
        &AtomicU64::new(0),
    )
    .unwrap_err();
    assert!(error.contains("999") && error.contains("not found"));
    assert_eq!(stage_image(state), before_image);
    let after_failure_status = stage_history_status(state);
    assert_eq!(after_failure_status.undo_depth, before_status.undo_depth);
    assert_eq!(after_failure_status.redo_depth, before_status.redo_depth);
    assert_eq!(
        after_failure_status.undo_entry_id,
        before_status.undo_entry_id
    );
    assert_eq!(
        after_failure_status.redo_entry_id,
        before_status.redo_entry_id
    );
    cancel_project_transaction_for_window_label(
        state,
        project_transaction_cancel_request!(
            window_label,
            ticket.transaction_id,
            ticket.project_epoch,
            ticket.client_operation_id,
            ticket.shape_fingerprint,
            "set_fixture_transforms",
            ticket.schema_version,
            owner_id
        ),
    )
    .unwrap();
}
