// Included in `main.rs` `mod tests`: a freshly opened project publishes the
// engine's canonical checkpoint, so the first authority read has nothing to
// reconcile and the UI shows it unmodified, while a later edit still advances
// the authority revision.

struct ProjectLoadCheckpointPlatform;

impl ProjectReplacementPlatform for ProjectLoadCheckpointPlatform {
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

/// Open the bundled phase-1 sample like File > Open / the command line: the
/// same preparation as `load_project_from_file_with_control_mappings_in_scope_and_disposition`
/// (prepare + canonicalize, CleanAtPath) and the shared ExternalCaller
/// replacement core. The sample predates the Timeline bank and Cue numbering,
/// so the engine migrates it on load. Returns the raw file-image hash too.
fn open_legacy_sample_project(state: &AppState) -> (ProjectLoadResult, String) {
    let path = Path::new("C:/shows/phase1-mini-show.sdc");
    let (project, mappings) = project_and_control_mappings_from_value(
        parse_project_json(PHASE1_SAMPLE_PROJECT_JSON).unwrap(),
    )
    .unwrap();
    let mut prepared = prepare_project_load(
        project,
        mappings,
        path.to_string_lossy().to_string(),
        Some(path),
    )
    .unwrap();
    let raw_file_hash = project_checkpoint_hash(
        &project_file_from_prepared_load(&prepared),
        &prepared.mappings,
    )
    .unwrap();
    canonicalize_prepared_project_snapshot(&mut prepared).unwrap();
    prepared.authority_disposition = ProjectAuthorityDisposition::CleanAtPath;
    let result = replace_prepared_project_snapshot_with_platform(
        state,
        prepared,
        ProjectSnapshotReplacementScope::ExternalCaller,
        None,
        &ProjectLoadCheckpointPlatform,
    )
    .unwrap();
    (result, raw_file_hash)
}

fn reconcile_like_authority_poll(
    state: &AppState,
) -> (ProjectCheckpoint, u64, ProjectAuthorityPublicationKind) {
    let _admission = lock_project_external_command_admission(state).unwrap();
    let mut coordinator = state.project_coordinator.lock().unwrap();
    let checkpoint = reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).unwrap();
    (
        checkpoint,
        coordinator.revision,
        coordinator.last_publication_kind,
    )
}

#[test]
fn opened_legacy_project_publishes_engine_checkpoint_and_stays_unmodified_until_edited() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (result, raw_file_hash) = open_legacy_sample_project(state);
    let authority = result.authority.clone().unwrap();
    assert_eq!(
        authority.authority_disposition,
        ProjectAuthorityDisposition::CleanAtPath
    );
    assert_eq!(
        authority.publication_kind,
        ProjectAuthorityPublicationKind::IdentityReplacement
    );
    assert_eq!(authority.checkpoint_hash, result.checkpoint_hash);
    assert_eq!(result.project_revision, 0);

    // The first authority read (event, reply or poll) has nothing to reconcile:
    // no revision bump and no Mutation publication, so the UI stays clean.
    let (checkpoint, revision, kind) = reconcile_like_authority_poll(state);
    assert_ne!(
        checkpoint.hash, raw_file_hash,
        "precondition: the engine migrates this file shape on load"
    );
    assert_eq!(
        checkpoint.hash, result.checkpoint_hash,
        "the opened identity must publish the engine's canonical checkpoint"
    );
    assert_eq!(revision, 0);
    assert_eq!(kind, ProjectAuthorityPublicationKind::IdentityReplacement);

    // A real edit afterwards still publishes a new revision (UI: modified).
    state.engine.set_timeline_audio_master(250, true).unwrap();
    let (edited, revision, kind) = reconcile_like_authority_poll(state);
    assert_ne!(edited.hash, result.checkpoint_hash);
    assert_eq!(revision, 1);
    assert_eq!(kind, ProjectAuthorityPublicationKind::Mutation);
}
