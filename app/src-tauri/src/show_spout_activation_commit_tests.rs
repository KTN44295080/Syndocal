use super::*;

fn disabled_fixture() -> MediaAssetA6CommandHarness {
    let harness = MediaAssetA6CommandHarness::new();
    install_show_spout_reset_fixture(&harness.state, ShowSpoutResetFixtureKind::CurrentV2);
    let mut snapshot = harness.state.engine.persistence_snapshot().unwrap();
    use_authored_video_snapshot(&mut snapshot);
    snapshot.authored_video = None;
    for output in &mut snapshot.video.outputs {
        output.enabled = false;
    }
    harness
        .state
        .engine
        .load_project_snapshot_and_wait(snapshot)
        .unwrap();
    let mut coordinator = lock_project_coordinator(&harness.state).unwrap();
    reconcile_project_checkpoint_for_coordinator(&harness.state, &mut coordinator).unwrap();
    drop(coordinator);
    harness
}

#[test]
fn show_spout_disabled_activation_commits_exact_project_successor_and_preserves_undo() {
    let harness = disabled_fixture();
    let state = &harness.state;
    validate_current_show_spout_outputs_action(state).unwrap();
    let activation = derive_current_show_spout_outputs(state).unwrap();
    assert_eq!(
        activation.project_change,
        show_spout_project_commit::ShowSpoutProjectChange::EnableExisting
    );
    let mut coordinator = lock_project_coordinator(state).unwrap();
    let before_revision = coordinator.revision;
    let before_generation = coordinator.publication_generation;
    let before_hash = coordinator.checkpoint_hash.clone();
    let before_project = project_file_for_save_from_parts(
        state.engine.persistence_snapshot().unwrap(),
        &coordinator.ancillary,
    );
    let checkpoint = ProjectCheckpoint {
        project: before_project,
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: before_hash.clone(),
    };
    coordinator.next_transaction_id = coordinator.next_transaction_id.max(1);
    coordinator.history.undo.push(ProjectHistoryEntry {
        entry_id: 1,
        label: "Earlier user edit".into(),
        coalesce_key: String::new(),
        committed_at_unix_ms: 0,
        before: checkpoint.clone(),
        after: checkpoint,
    });
    let plan = show_spout_project_commit::prepare(
        state,
        &coordinator,
        &activation.expected,
        activation.prior_disabled.as_ref(),
        activation.project_change,
    )
    .unwrap();
    assert_eq!(
        coordinator.revision, before_revision,
        "preflight cannot publish project changes"
    );
    let safety = state.engine.safety_blackout_authority();
    state
        .engine
        .enable_show_spout_outputs_published(
            activation.expected.background.clone(),
            activation.expected.foreground.clone(),
            safety.epoch,
            safety.generation,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
    plan.verify(state, &coordinator).unwrap();
    plan.commit(&mut coordinator);
    assert_eq!(coordinator.revision, before_revision + 1);
    assert_eq!(coordinator.publication_generation, before_generation + 1);
    assert_ne!(coordinator.checkpoint_hash, before_hash);
    assert_eq!(coordinator.history.undo.len(), 2);
    assert_eq!(coordinator.history.undo[0].label, "Earlier user edit");
    let entry = coordinator.history.undo.last().unwrap();
    assert!(entry
        .before
        .project
        .snapshot
        .video
        .outputs
        .iter()
        .all(|output| !output.enabled));
    assert!(entry
        .after
        .project
        .snapshot
        .video
        .outputs
        .iter()
        .all(|output| output.enabled));
    assert_eq!(
        entry
            .before
            .project
            .snapshot
            .video
            .outputs
            .iter()
            .map(|output| output.id)
            .collect::<Vec<_>>(),
        entry
            .after
            .project
            .snapshot
            .video
            .outputs
            .iter()
            .map(|output| output.id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn show_spout_disabled_activation_compensation_restores_authored_checkpoint() {
    let harness = disabled_fixture();
    let state = &harness.state;
    let activation = derive_current_show_spout_outputs(state).unwrap();
    let coordinator = lock_project_coordinator(state).unwrap();
    let plan = show_spout_project_commit::prepare(
        state,
        &coordinator,
        &activation.expected,
        activation.prior_disabled.as_ref(),
        activation.project_change,
    )
    .unwrap();
    let safety = state.engine.safety_blackout_authority();
    state
        .engine
        .enable_show_spout_outputs_published(
            activation.expected.background.clone(),
            activation.expected.foreground.clone(),
            safety.epoch,
            safety.generation,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
    let failure = compensate_show_spout_activation_candidate(
        state,
        &activation.expected,
        activation.prior_disabled.as_ref(),
        activation.project_change,
        "injected pre-handoff failure".into(),
        Ok(()),
    );
    assert!(matches!(
        failure,
        OutputLeaseCandidateCommitFailure::SafeAbort(_)
    ));
    plan.verify_rollback(state, &coordinator).unwrap();
    assert!(!state
        .project_external_command_admission
        .project_transaction_publication_faulted
        .load(Ordering::Acquire));
}
