use super::*;

fn request(id: u64) -> ProjectPublicationRequestV1 {
    ProjectPublicationRequestV1 {
        schema_version: PROJECT_PUBLICATION_REQUEST_SCHEMA_VERSION,
        origin_id: "missing_owner_fixture".into(), owner_id: "owner:retired-renderer".into(), request_id: id,
        surface: ProjectPublicationSurfaceV1::Backup,
        expected_project_epoch: 7, expected_project_revision: 11,
        expected_checkpoint_hash: "a".repeat(64), mapping_authority_hash: "a".repeat(64),
        source_path: Some(std::env::temp_dir().join("syndocal-missing-publication-original.sdc").to_string_lossy().into_owned()), reason: Some("autosave".into()),
        target_policy: ProjectPublicationTargetPolicyV1::ManagedUnique,
    }
}

fn empty() -> PersistedProjectRecoveryAuthorityState {
    PersistedProjectRecoveryAuthorityState {
        version: PROJECT_RECOVERY_AUTHORITY_STATE_VERSION, serial: 3,
        last_transition: ProjectRecoveryAuthorityTransition::ProjectPublication,
        pending_clean_save: None, publication_journal: PersistedProjectPublicationJournalV1::default(),
    }
}

#[test]
fn missing_publication_abandon_ack_and_next_sequence_survive_reply_loss() {
    let before = empty();
    let request = request(1);
    let (reply, after) = resolve_journal(&before, &request).unwrap();
    let mut after = after.unwrap();
    let MissingPublicationResolutionV1::Terminal { status, .. } = reply else { panic!("expected terminal"); };
    assert_eq!(status.state, "abandoned");
    assert_eq!(status.request, request);
    assert!(status.authority.is_none() && status.target_path.is_none() && status.backup.is_none());
    assert_eq!(after.serial, before.serial);
    assert_eq!(after.last_transition, before.last_transition);
    assert_eq!(after.publication_journal.latest_reservation_generation, 0);
    let terminal_wire = serde_json::to_value(MissingPublicationResolutionV1::Terminal { schema_version: 1, status }).unwrap();
    assert_eq!(terminal_wire["schemaVersion"], 1);
    assert_eq!(terminal_wire["kind"], "terminal");
    let restarted: PersistedProjectRecoveryAuthorityState = serde_json::from_slice(&serde_json::to_vec(&after).unwrap()).unwrap();
    let (repeat, no_write) = resolve_journal(&restarted, &request).unwrap();
    assert!(no_write.is_none());
    assert_eq!(serde_json::to_value(repeat).unwrap(), terminal_wire);
    let shape = project_publication_shape_hash_v1(&request).unwrap();
    assert!(acknowledge_project_publication_receipt_in_state_v1(&mut after, &request, &shape).unwrap());
    let (acknowledged, no_write) = resolve_journal(&after, &request).unwrap();
    assert!(no_write.is_none());
    let wire = serde_json::to_value(acknowledged).unwrap();
    assert_eq!(wire["kind"], "acknowledged");
    assert_eq!(wire["schemaVersion"], 1);
    assert_eq!(wire["shapeHash"], shape);
    assert_eq!(wire["request"], serde_json::to_value(&request).unwrap());
    let mut next = request.clone(); next.request_id = 2;
    assert_eq!(resolve_journal(&after, &next).unwrap().1.unwrap().publication_journal.origins[0].high_water_request_id, 2);
}

#[test]
fn missing_publication_rejects_unknown_old_shape_and_unresolved_origin_without_mutation() {
    let mut durable = resolve_journal(&empty(), &request(1)).unwrap().1.unwrap();
    let preserved = durable.clone();
    assert!(resolve_journal(&durable, &request(2)).is_err());
    let mut conflict = request(1); conflict.reason = Some("different".into());
    assert!(resolve_journal(&durable, &conflict).is_err());
    assert_eq!(durable, preserved);
    let shape = project_publication_shape_hash_v1(&request(1)).unwrap();
    acknowledge_project_publication_receipt_in_state_v1(&mut durable, &request(1), &shape).unwrap();
    assert!(resolve_journal(&durable, &conflict).is_err());
    // A gap above the next sequence, or an id at/below the high-water mark with
    // nothing unresolved, can never be reserved: the answer is a `stale` proof
    // that carries the journal's next sequence and changes nothing.
    let stale = |durable: &PersistedProjectRecoveryAuthorityState, id: u64| match resolve_journal(durable, &request(id)).unwrap() {
        (MissingPublicationResolutionV1::Stale { request: echoed, expected_request_id, reason, .. }, write) => {
            assert!(write.is_none(), "a stale proof must not mutate the journal");
            assert_eq!(echoed, request(id));
            (expected_request_id, reason)
        }
        other => panic!("expected a stale proof, got {other:?}"),
    };
    assert_eq!(stale(&durable, 3), (2, MissingPublicationStaleReasonV1::Unreserved));
    assert_eq!(stale(&empty(), 2), (1, MissingPublicationStaleReasonV1::Unreserved));
    durable.publication_journal.origins[0].high_water_request_id = 3;
    durable.publication_journal.origins[0].acknowledged_request_id = 3;
    assert_eq!(stale(&durable, 1), (4, MissingPublicationStaleReasonV1::Settled));
    assert_eq!(stale(&durable, 2), (4, MissingPublicationStaleReasonV1::Settled));
    assert_eq!(stale(&durable, 5), (4, MissingPublicationStaleReasonV1::Unreserved));
    let wire = serde_json::to_value(resolve_journal(&durable, &request(1)).unwrap().0).unwrap();
    assert_eq!(wire["kind"], "stale");
    assert_eq!(wire["schemaVersion"], 1);
    assert_eq!(wire["expectedRequestId"], 4);
    assert_eq!(wire["reason"], "settled");
    assert_eq!(wire["request"], serde_json::to_value(&request(1)).unwrap());
    // An origin whose acknowledged id lags its high-water mark is not provably
    // settled: the intent is kept for recovery instead of being retired.
    let mut lagging = durable.clone();
    lagging.publication_journal.origins[0].acknowledged_request_id = 1;
    assert!(resolve_journal(&lagging, &request(2)).unwrap_err().contains("lags"));
    // The exact next sequence is still the only id that may be abandoned.
    assert!(matches!(
        resolve_journal(&durable, &request(4)).unwrap().0,
        MissingPublicationResolutionV1::Terminal { .. }
    ));
    let mut pending = empty();
    pending.publication_journal.origins.push(PersistedProjectPublicationOriginV1 {
        origin_id: request(1).origin_id, high_water_request_id: 1, acknowledged_request_id: 0, acknowledged_shape_hash: None,
    });
    pending.publication_journal.latest_reservation_generation = 1;
    pending.publication_journal.pending.push(PersistedProjectPublicationPendingV1 {
        request: request(1), shape_hash: project_publication_shape_hash_v1(&request(1)).unwrap(),
        surface: ProjectPublicationSurfaceV1::Backup, project_epoch: 7, project_revision: 11,
        checkpoint_hash: "a".repeat(64), path_generation: 0, authority_disposition_generation: 0,
        recovery_authority_serial_before: 3, expected_recovery_authority_serial_after: None,
        reservation_generation: 1, source_path: request(1).source_path, reason: request(1).reason,
        phase: ProjectPublicationPendingPhaseV1::Reserved, target_path: None, staging_path: None,
        prepared_digest: None, indeterminate_error: None,
    });
    validate_project_publication_journal_v1(&pending.publication_journal).unwrap();
    assert!(resolve_journal(&pending, &request(1)).unwrap_err().contains("unresolved"));
    assert!(resolve_journal(&pending, &request(2)).unwrap_err().contains("unresolved"));
}

#[test]
fn missing_publication_owner_binding_retirement_and_failed_write_preserve_authority() {
    let harness = crate::tests::MediaAssetA6CommandHarness::new();
    let current = crate::tests::MEDIA_ASSET_A6_OWNER.to_string();
    let directory = std::env::temp_dir().join(format!("syndocal-missing-publication-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let path = directory.join("journal.json");
    let engine_before = harness.state.engine.persistence_snapshot().unwrap();
    let authority_before = {
        let coordinator = harness.state.project_coordinator.lock().unwrap();
        (coordinator.epoch, coordinator.revision, coordinator.checkpoint_hash.clone(), coordinator.recovery_authority_serial)
    };
    assert!(resolve_with_writer(&harness.state, "media-asset-a6", &path, request(1), "owner:wrong".into(), |_, _| panic!("wrong owner must not persist")).is_err());
    harness.state.project_transaction_owners.lock().unwrap().insert("another-window".into(), request(1).owner_id);
    assert!(resolve_with_writer(&harness.state, "media-asset-a6", &path, request(1), current.clone(), |_, _| panic!("live owner must not persist")).unwrap_err().contains("retired"));
    harness.state.project_transaction_owners.lock().unwrap().remove("another-window");
    let error = resolve_with_writer(&harness.state, "media-asset-a6", &path, request(1), current.clone(), |_, candidate| {
        assert!(matches!(harness.state.project_transaction_owners.try_lock(), Err(TryLockError::WouldBlock)));
        assert_eq!(candidate.publication_journal.terminals[0].request, request(1));
        Err("injected durable write failure".into())
    }).unwrap_err();
    assert!(error.contains("injected"));
    assert!(!path.exists());
    let engine_after = harness.state.engine.persistence_snapshot().unwrap();
    assert_eq!(engine_after.video, engine_before.video);
    assert_eq!(engine_after.timeline, engine_before.timeline);
    {
        let coordinator = harness.state.project_coordinator.lock().unwrap();
        assert_eq!((coordinator.epoch, coordinator.revision, coordinator.checkpoint_hash.clone(), coordinator.recovery_authority_serial), authority_before);
    }
    let reply = resolve_for_window(&harness.state, "media-asset-a6", &path, request(1), current).unwrap();
    assert!(matches!(reply, MissingPublicationResolutionV1::Terminal { .. }));
    let loaded = load_project_recovery_authority_state_from_path(&path).unwrap();
    assert_eq!(loaded.publication_journal.terminals[0].request, request(1));
    fs::remove_file(&path).unwrap();
    fs::remove_dir(&directory).unwrap();
}
