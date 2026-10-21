use super::*;
use crate::tests::MediaAssetA6CommandHarness;

fn full_poll_cursor(state: &AppState) -> ProjectAuthorityPollCursor {
    let coordinator = state.project_coordinator.lock().unwrap();
    (
        coordinator.epoch,
        coordinator.revision,
        coordinator.checkpoint_hash.clone(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

fn seed_history(state: &AppState) -> (ProjectAuthorityCaptureStamp, ProjectHistory) {
    let mut coordinator = state.project_coordinator.lock().unwrap();
    let mut current = project_checkpoint_for_coordinator(state, &coordinator).unwrap();
    assert_eq!(current.hash, coordinator.checkpoint_hash);
    assert!(coordinator.history.pending.is_empty());
    assert!(coordinator.history.undo.is_empty());
    assert!(coordinator.history.redo.is_empty());

    // Keep the Engine's current image while recording an earlier edit and
    // one undone edit whose Redo source is that same current checkpoint.
    let mut previous = current.clone();
    previous.project.snapshot.timeline.metronome_enabled =
        !previous.project.snapshot.timeline.metronome_enabled;
    previous.hash = project_checkpoint_hash(&previous.project, &previous.mappings).unwrap();
    assert_ne!(previous.hash, current.hash);
    current.revision = checked_project_revision_after_mutation(&coordinator).unwrap();
    let mut future = previous.clone();
    future.revision = current.revision + 1;
    let undo_id = coordinator.next_transaction_id + 1;
    let redo_id = undo_id + 1;
    coordinator.history.undo.push(ProjectHistoryEntry {
        entry_id: undo_id,
        label: "Earlier edit".to_string(),
        coalesce_key: "poll-reservation-undo".to_string(),
        committed_at_unix_ms: 10,
        before: previous,
        after: current.clone(),
    });
    coordinator.history.redo.push(ProjectHistoryEntry {
        entry_id: redo_id,
        label: "Undone edit".to_string(),
        coalesce_key: "poll-reservation-redo".to_string(),
        committed_at_unix_ms: 20,
        before: current.clone(),
        after: future,
    });
    coordinator.next_transaction_id = redo_id;
    coordinator.revision = current.revision;
    coordinator.history_generation =
        checked_project_history_generation_after_change(&coordinator).unwrap();
    let publication_generation =
        checked_project_authority_publication_generation_after_change(&coordinator).unwrap();
    commit_project_authority_publication_after_preflight(
        &mut coordinator,
        publication_generation,
        ProjectAuthorityPublicationKind::HistoryNavigation,
    );
    (
        project_authority_capture_stamp(&coordinator),
        coordinator.history.clone(),
    )
}

fn prepare_candidate(state: &AppState) -> (EngineSnapshot, PreparedInternalMediaAssetCommit) {
    let snapshot = state.engine.persistence_snapshot().unwrap();
    let mut candidate = snapshot.clone();
    candidate.timeline.metronome_enabled = !candidate.timeline.metronome_enabled;
    let coordinator = state.project_coordinator.lock().unwrap();
    let before_project = project_file_for_save_from_parts(snapshot, &coordinator.ancillary);
    let before = ProjectCheckpoint {
        hash: project_checkpoint_hash(&before_project, &coordinator.mappings).unwrap(),
        project: before_project,
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
    };
    assert_eq!(before.hash, coordinator.checkpoint_hash);
    let after = ProjectCheckpoint {
        project: project_file_for_save_from_parts(candidate.clone(), &coordinator.ancillary),
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: String::new(),
    };
    let plan =
        prepare_internal_media_asset_commit(&coordinator, "Authoring", "", before, after, 30)
            .unwrap();
    assert_ne!(
        plan.next_checkpoint_hash.as_ref(),
        Some(&coordinator.checkpoint_hash)
    );
    assert!(plan.next_checkpoint_hash.is_some());
    (candidate, plan)
}

fn assert_history_entries_match(actual: &[ProjectHistoryEntry], expected: &[ProjectHistoryEntry]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.entry_id, expected.entry_id);
        assert_eq!(actual.label, expected.label);
        assert_eq!(actual.coalesce_key, expected.coalesce_key);
        assert_eq!(actual.committed_at_unix_ms, expected.committed_at_unix_ms);
        assert_eq!(actual.before, expected.before);
        assert_eq!(actual.after, expected.after);
    }
}

fn assert_reservation_preserved(
    state: &AppState,
    stamp: &ProjectAuthorityCaptureStamp,
    history: &ProjectHistory,
) {
    assert!(state.project_transaction_active.load(Ordering::Acquire));
    let coordinator = state.project_coordinator.lock().unwrap();
    assert_eq!(&project_authority_capture_stamp(&coordinator), stamp);
    assert!(coordinator.history.pending.is_empty());
    assert_history_entries_match(&coordinator.history.undo, &history.undo);
    assert_history_entries_match(&coordinator.history.redo, &history.redo);
}

#[test]
fn authority_poll_active_reservation_skips_capture_and_preserves_history() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (before, history) = seed_history(state);
    let cursor = full_poll_cursor(state);
    {
        let coordinator = state.project_coordinator.lock().unwrap();
        assert!(coordinator.history.pending.is_empty());
        reserve_output_authoring_project_transaction(state, &coordinator).unwrap();
    }

    let result = poll_project_authority_bundle_with_capture(state, cursor, || {
        panic!("an active authoring reservation must not request an Engine snapshot")
    })
    .unwrap();

    assert!(result.is_none());
    assert_reservation_preserved(state, &before, &history);
    state
        .project_transaction_active
        .store(false, Ordering::Release);
}

#[test]
fn authority_poll_reservation_started_during_capture_cannot_reconcile_candidate() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (before, history) = seed_history(state);
    let cursor = full_poll_cursor(state);
    let (candidate, _plan) = prepare_candidate(state);
    let mut captures = 0;

    let result = poll_project_authority_bundle_with_capture(state, cursor, || {
        captures += 1;
        // This is the production Engine-read boundary, outside both guards.
        assert!(state
            .project_external_command_admission
            .gate
            .try_lock()
            .is_ok());
        {
            let coordinator = state.project_coordinator.try_lock().unwrap();
            assert!(coordinator.history.pending.is_empty());
            reserve_output_authoring_project_transaction(state, &coordinator).unwrap();
        }
        Ok((candidate.clone(), candidate.clone()))
    })
    .unwrap();

    assert!(result.is_none());
    assert_eq!(captures, 1);
    assert_reservation_preserved(state, &before, &history);
    state
        .project_transaction_active
        .store(false, Ordering::Release);
}

#[test]
fn authority_poll_completed_reservation_exposes_committed_mutation_once() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (before, history) = seed_history(state);
    let cursor = full_poll_cursor(state);
    let (candidate, plan) = prepare_candidate(state);
    let expected_hash = plan.next_checkpoint_hash.clone().unwrap();
    let expected_history = plan.next_history.clone();
    assert_eq!(expected_history.undo.len(), history.undo.len() + 1);
    assert!(expected_history.redo.is_empty());
    assert_history_entries_match(
        &expected_history.undo[..history.undo.len()],
        &history.undo,
    );
    let mut plan = Some(plan);
    let mut captures = 0;

    // Substitute snapshot acquisition only. The production commit, seqlock,
    // checkpoint reconciliation and bundle projection all remain in use.
    let bundle = poll_project_authority_bundle_with_capture(state, cursor, || {
        captures += 1;
        assert!(state
            .project_external_command_admission
            .gate
            .try_lock()
            .is_ok());
        if let Some(plan) = plan.take() {
            {
                let mut coordinator = state.project_coordinator.try_lock().unwrap();
                assert!(coordinator.history.pending.is_empty());
                reserve_output_authoring_project_transaction(state, &coordinator).unwrap();
                apply_internal_media_asset_transaction_after_preflight(&mut coordinator, plan);
            }
            // Model successful terminal completion after B is committed.
            state
                .project_transaction_active
                .store(false, Ordering::Release);
        }
        Ok((candidate.clone(), candidate.clone()))
    })
    .unwrap()
    .expect("a completed reservation must converge to its committed authority");

    // The first capture started at A and ended at B. Its stamp must force a
    // retry even though the active flag has already returned to false.
    assert_eq!(captures, 2);
    assert!(plan.is_none());
    assert!(!state.project_transaction_active.load(Ordering::Acquire));
    assert_eq!(bundle.project_epoch, before.epoch);
    assert_eq!(bundle.project_revision, before.revision + 1);
    assert_eq!(bundle.checkpoint_hash, expected_hash);
    assert_eq!(
        bundle.publication_generation,
        before.publication_generation + 1
    );
    assert_eq!(
        bundle.publication_kind,
        ProjectAuthorityPublicationKind::Mutation
    );
    assert_eq!(bundle.history_generation, before.history_generation + 1);
    assert_eq!(bundle.history.undo_depth, expected_history.undo.len());
    assert_eq!(bundle.history.redo_depth, 0);
    assert!(bundle.history.can_undo);
    assert!(!bundle.history.can_redo);
    assert_eq!(bundle.history.undo_label.as_deref(), Some("Authoring"));
    assert_eq!(
        bundle.history.undo_entry_id,
        expected_history.undo.last().map(|entry| entry.entry_id)
    );
    assert_eq!(
        bundle.history.undo_checkpoint_hash.as_deref(),
        Some(expected_hash.as_str())
    );
    assert_eq!(
        bundle.snapshot.timeline.metronome_enabled,
        candidate.timeline.metronome_enabled
    );

    let coordinator = state.project_coordinator.lock().unwrap();
    assert_eq!(coordinator.revision, before.revision + 1);
    assert_eq!(coordinator.checkpoint_hash, expected_hash);
    assert_eq!(
        coordinator.publication_generation,
        before.publication_generation + 1
    );
    assert_eq!(coordinator.history_generation, before.history_generation + 1);
    assert!(coordinator.history.pending.is_empty());
    assert_history_entries_match(&coordinator.history.undo, &expected_history.undo);
    assert_history_entries_match(&coordinator.history.redo, &expected_history.redo);
    assert_eq!(
        bundle.history,
        project_history_status_for_coordinator(&coordinator)
    );
}

#[test]
fn authority_poll_aborted_reservation_retries_without_reconciling_transient_candidate() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (before, history) = seed_history(state);
    let cursor = full_poll_cursor(state);
    let original = state.engine.persistence_snapshot().unwrap();
    let (candidate, _plan) = prepare_candidate(state);
    let mut captures = 0;
    let bundle = poll_project_authority_bundle_with_capture(state, cursor, || {
        captures += 1;
        if captures == 1 {
            {
                let coordinator = state.project_coordinator.try_lock().unwrap();
                reserve_output_authoring_project_transaction(state, &coordinator).unwrap();
            }
            // The provider captured temporary B before native cleanup restored
            // A. Safe abort changes no committed metadata, but must invalidate
            // that in-flight capture even after active returns to false.
            state.project_transaction_active.store(false, Ordering::Release);
            Ok((candidate.clone(), candidate.clone()))
        } else {
            Ok((original.clone(), original.clone()))
        }
    })
    .unwrap()
    .expect("a safely aborted reservation must leave the original bundle readable");

    assert_eq!(captures, 2);
    assert!(!state.project_transaction_active.load(Ordering::Acquire));
    let coordinator = state.project_coordinator.lock().unwrap();
    assert_eq!(project_authority_capture_stamp(&coordinator), before);
    assert_history_entries_match(&coordinator.history.undo, &history.undo);
    assert_history_entries_match(&coordinator.history.redo, &history.redo);
    assert_eq!(bundle.project_revision, before.revision);
    assert_eq!(bundle.checkpoint_hash, before.checkpoint_hash);
    assert_eq!(bundle.publication_generation, before.publication_generation);
    assert_eq!(bundle.history, project_history_status_for_coordinator(&coordinator));
    assert_eq!(
        bundle.snapshot.timeline.metronome_enabled,
        original.timeline.metronome_enabled,
    );
}
