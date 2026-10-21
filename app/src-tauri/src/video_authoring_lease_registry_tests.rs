use super::*;

const PROCESS: u64 = 71;
const PREVIOUS_PROJECT: &str = "project_epoch:1";
const CURRENT_PROJECT: &str = "project_epoch:2";
const TTL_MS: u64 = 100;

fn test_owner(incarnation: u64) -> OutputLeaseOwner {
    OutputLeaseOwner::new("local-ui", "main", PROCESS, incarnation).unwrap()
}

fn video() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Video]).unwrap()
}

fn lighting() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Lighting]).unwrap()
}

fn both() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Lighting, OutputLeaseResource::Video])
        .unwrap()
}

fn seeded(
    resources: OutputLeaseResources,
) -> (OutputLeaseRegistry, OutputLeaseOwner, OutputLeaseLeaseView) {
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let owner = test_owner(1);
    let grant = registry
        .acquire(owner.clone(), resources, PREVIOUS_PROJECT, 0, TTL_MS)
        .unwrap();
    let view = registry.lease_view(grant.lease_id).unwrap();
    (registry, owner, view)
}

fn request(request_id: u64, action: OutputLeaseRequestAction) -> OutputLeaseRequest {
    OutputLeaseRequest::from_action("local-ui", "video-authoring-registry", request_id, action)
        .unwrap()
}

fn replacement(
    previous: &OutputLeaseLeaseView,
    owner: &OutputLeaseOwner,
    project_identity: &str,
) -> OutputLeaseRequestAction {
    OutputLeaseRequestAction::ReplacePreviousProjectForAuthoring {
        lease_id: previous.lease_id,
        owner: owner.clone(),
        expected_generation: previous.snapshot.generation,
        project_identity: project_identity.to_string(),
        ttl_ms: TTL_MS,
    }
}

fn orphan_previous(
    registry: &mut OutputLeaseRegistry,
    previous: &OutputLeaseLeaseView,
) -> OutputLeaseLeaseView {
    let receipt = registry
        .submit_request(
            &request(
                1,
                OutputLeaseRequestAction::ProjectOrphan {
                    project_identity: PREVIOUS_PROJECT.to_string(),
                    selected_lease_ids: vec![previous.lease_id],
                    resource_scope: None,
                },
            ),
            1,
        )
        .unwrap();
    assert_eq!(receipt.outcome, Ok(OutputLeaseOperationOutcome::ProjectOrphaned));
    let orphan = registry.lease_view(previous.lease_id).unwrap();
    assert_eq!(orphan.snapshot.phase, OutputLeasePhase::HeldOrphaned);
    orphan
}

fn assert_authority_unchanged(before: &OutputLeaseRegistry, after: &OutputLeaseRegistry) {
    // A rejected request may retain its terminal receipt, but no claim, index
    // entry, generation, project binding or allocated id may change.
    assert_eq!(after.leases, before.leases);
    assert_eq!(after.resource_index, before.resource_index);
    assert_eq!(after.next_lease_id, before.next_lease_id);
}

fn assert_replaced(
    registry: &OutputLeaseRegistry,
    previous: &OutputLeaseLeaseView,
    owner: &OutputLeaseOwner,
    receipt: &OutputLeaseRequestReceipt,
) {
    assert_eq!(receipt.outcome, Ok(OutputLeaseOperationOutcome::Acquired));
    let new_id = receipt.lease_id.unwrap();
    assert_ne!(new_id, previous.lease_id);
    let current = registry.lease_view(new_id).unwrap();
    let retired = registry.lease_view(previous.lease_id).unwrap();
    assert_eq!(current.project_identity, CURRENT_PROJECT);
    assert_eq!(current.snapshot.phase, OutputLeasePhase::HeldActive);
    assert_eq!(current.snapshot.owner.as_ref(), Some(owner));
    assert_eq!(current.snapshot.resources, previous.snapshot.resources);
    assert_eq!(retired.project_identity, previous.project_identity);
    assert_eq!(retired.snapshot.phase, OutputLeasePhase::Unclaimed);
    assert_eq!(retired.snapshot.generation, previous.snapshot.generation + 1);
    assert_eq!(retired.snapshot.owner, None);
    assert_eq!(retired.snapshot.resources, None);
    assert_eq!(retired.snapshot.expires_at_monotonic_ms, None);
    let mut affected = vec![new_id, previous.lease_id];
    affected.sort_unstable();
    assert_eq!(receipt.affected_lease_ids, affected);
    assert_eq!(receipt.owner.as_ref(), Some(owner));
    assert_eq!(receipt.resources, previous.snapshot.resources);
    assert_eq!(receipt.generation_before, None);
    assert_eq!(receipt.generation_after, Some(current.snapshot.generation));
    assert_eq!(
        receipt.changes,
        vec![
            OutputLeaseChange {
                lease_id: new_id,
                before: None,
                after: Some(current.snapshot.clone()),
            },
            OutputLeaseChange {
                lease_id: previous.lease_id,
                before: Some(previous.snapshot.clone()),
                after: Some(retired.snapshot),
            },
        ]
    );
    assert_eq!(validate_persisted_receipt(receipt), Ok(()));
    for resource in previous.snapshot.resources.as_ref().unwrap().as_slice() {
        assert_eq!(
            registry.resource_index.get(resource),
            Some(&BTreeSet::from([new_id]))
        );
    }
    assert!(!registry
        .resource_index
        .values()
        .any(|ids| ids.contains(&previous.lease_id)));
    assert_eq!(
        registry.query_video_authoring_lease(owner, registry.last_now_ms),
        Ok(Some(current))
    );
}

#[test]
fn video_authoring_lease_replaces_previous_active_video_and_both_with_exact_receipts() {
    for resources in [video(), both()] {
        let (mut registry, owner, previous) = seeded(resources);
        let receipt = registry
            .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), 1)
            .unwrap();
        assert_replaced(&registry, &previous, &owner, &receipt);
    }
}

#[test]
fn video_authoring_lease_replaces_previous_orphan_without_recovering_old_id() {
    for resources in [video(), both()] {
        let (mut registry, owner, previous) = seeded(resources);
        let previous = orphan_previous(&mut registry, &previous);
        let before_query = registry.clone();
        assert_eq!(
            registry.query_video_authoring_lease(&owner, 2),
            Ok(Some(previous.clone()))
        );
        assert_eq!(registry, before_query);
        let receipt = registry
            .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), 2)
            .unwrap();
        assert_replaced(&registry, &previous, &owner, &receipt);
    }
}

#[test]
fn video_authoring_lease_expired_active_query_is_pure_and_original_generation_replaces() {
    for resources in [video(), both()] {
        let (mut registry, owner, previous) = seeded(resources);
        let before = registry.clone();
        // The generic query hides this claim, but its indexed ownership remains.
        assert!(registry.query_owner_leases(&owner, TTL_MS).unwrap().is_empty());
        assert_eq!(
            registry.query_video_authoring_lease(&owner, TTL_MS),
            Ok(Some(previous.clone()))
        );
        assert_eq!(registry, before);
        let receipt = registry
            .submit_request(
                &request(2, replacement(&previous, &owner, CURRENT_PROJECT)),
                TTL_MS,
            )
            .unwrap();
        assert_replaced(&registry, &previous, &owner, &receipt);
    }
}

#[test]
fn video_authoring_lease_generic_recover_keeps_previous_project_binding_and_id() {
    let (mut registry, owner, previous) = seeded(both());
    let orphan = orphan_previous(&mut registry, &previous);
    let receipt = registry
        .submit_request(
            &request(
                2,
                OutputLeaseRequestAction::Recover {
                    lease_id: orphan.lease_id,
                    owner: owner.clone(),
                    expected_generation: orphan.snapshot.generation,
                    ttl_ms: TTL_MS,
                },
            ),
            2,
        )
        .unwrap();
    assert_eq!(receipt.outcome, Ok(OutputLeaseOperationOutcome::Recovered));
    assert_eq!(receipt.lease_id, Some(previous.lease_id));
    let recovered = registry.lease_view(previous.lease_id).unwrap();
    assert_eq!(recovered.project_identity, PREVIOUS_PROJECT);
    assert_eq!(recovered.snapshot.phase, OutputLeasePhase::HeldActive);
    assert_eq!(recovered.snapshot.generation, orphan.snapshot.generation + 1);
    assert_eq!(registry.leases.len(), 1);
    assert_eq!(
        registry.query_video_authoring_lease(&owner, 2),
        Ok(Some(recovered))
    );
}

#[test]
fn video_authoring_lease_rejects_current_project_foreign_owner_process_and_stale_generation() {
    let (mut baseline, owner, active) = seeded(both());
    let orphan = orphan_previous(&mut baseline, &active);
    let foreign_process = OutputLeaseOwner::new("local-ui", "main", PROCESS + 1, 1).unwrap();
    let cases = [
        (replacement(&orphan, &owner, PREVIOUS_PROJECT), OutputLeaseError::InvalidProject),
        (replacement(&orphan, &test_owner(2), CURRENT_PROJECT), OutputLeaseError::StaleOwner),
        (replacement(&orphan, &foreign_process, CURRENT_PROJECT), OutputLeaseError::StaleOwner),
        (replacement(&active, &owner, CURRENT_PROJECT), OutputLeaseError::StaleGeneration),
    ];
    for (action, error) in cases {
        let mut registry = baseline.clone();
        let before = registry.clone();
        let receipt = registry.submit_request(&request(2, action), 2).unwrap();
        assert_eq!(receipt.outcome, Err(error));
        assert!(receipt.changes.is_empty());
        assert_eq!(validate_persisted_receipt(&receipt), Ok(()));
        assert_authority_unchanged(&before, &registry);
    }
}

#[test]
fn video_authoring_lease_rejects_lighting_only_scope_without_releasing_it() {
    let (mut registry, owner, previous) = seeded(lighting());
    let before = registry.clone();
    let receipt = registry
        .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), 1)
        .unwrap();
    assert_eq!(receipt.outcome, Err(OutputLeaseError::ResourceConflict));
    assert!(receipt.changes.is_empty());
    assert_authority_unchanged(&before, &registry);
}

#[test]
fn video_authoring_lease_replacement_preserves_disjoint_lighting_authority() {
    let (mut registry, owner, previous) = seeded(video());
    let lighting = registry
        .acquire(test_owner(2), lighting(), PREVIOUS_PROJECT, 0, TTL_MS)
        .unwrap();
    let lighting_before = registry.lease_view(lighting.lease_id).unwrap();
    let receipt = registry
        .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), 1)
        .unwrap();
    assert_replaced(&registry, &previous, &owner, &receipt);
    assert_eq!(registry.lease_view(lighting.lease_id).unwrap(), lighting_before);
    assert_eq!(
        registry.resource_index.get(&OutputLeaseResource::Lighting),
        Some(&BTreeSet::from([lighting.lease_id]))
    );
}

#[test]
fn video_authoring_lease_query_is_exact_owner_process_and_ignores_disjoint_lighting() {
    let (mut registry, owner, previous) = seeded(video());
    registry.acquire(owner.clone(), lighting(), PREVIOUS_PROJECT, 0, TTL_MS).unwrap();
    registry.advance_now(10).unwrap();
    let before = registry.clone();
    assert_eq!(registry.query_video_authoring_lease(&owner, 10), Ok(Some(previous.clone())));
    for other in [
        test_owner(2),
        OutputLeaseOwner::new("other-principal", "main", PROCESS, 1).unwrap(),
        OutputLeaseOwner::new("local-ui", "other-window", PROCESS, 1).unwrap(),
    ] {
        assert_eq!(registry.query_video_authoring_lease(&other, 10), Ok(None));
    }
    assert_eq!(
        registry.query_video_authoring_lease(
            &OutputLeaseOwner::new("local-ui", "main", PROCESS + 1, 1).unwrap(),
            10,
        ),
        Err(OutputLeaseError::StaleOwner)
    );
    assert_eq!(registry.query_video_authoring_lease(&owner, 9), Err(OutputLeaseError::ClockRollback));
    assert_eq!(registry, before);
    let mut stale_record = registry.clone();
    stale_record
        .leases
        .get_mut(&previous.lease_id)
        .unwrap()
        .state
        .process_session_incarnation = PROCESS + 1;
    let before = stale_record.clone();
    assert_eq!(
        stale_record.query_video_authoring_lease(&owner, 10),
        Err(OutputLeaseError::StaleOwner)
    );
    assert_eq!(stale_record, before);
    let (lighting_only, owner, _) = seeded(lighting());
    let before = lighting_only.clone();
    assert_eq!(lighting_only.query_video_authoring_lease(&owner, TTL_MS), Ok(None));
    assert_eq!(lighting_only, before);
    let empty = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    assert_eq!(empty.query_video_authoring_lease(&owner, 0), Ok(None));
}

#[test]
fn video_authoring_lease_query_and_replace_fail_closed_on_index_corruption_and_ambiguity() {
    let (baseline, owner, previous) = seeded(video());
    let mut cases = Vec::new();
    let mut missing = baseline.clone();
    missing.resource_index.remove(&OutputLeaseResource::Video);
    cases.push(missing);
    let mut dangling = baseline.clone();
    dangling.resource_index.get_mut(&OutputLeaseResource::Video).unwrap().insert(OutputLeaseId(900));
    cases.push(dangling);
    let mut wrong_scope = baseline.clone();
    wrong_scope.resource_index.insert(OutputLeaseResource::Lighting, BTreeSet::from([previous.lease_id]));
    cases.push(wrong_scope);
    // Both same-owner ambiguity and a competing owner's claim must reject.
    for competitor_owner in [owner.clone(), test_owner(2)] {
        let mut conflicting = baseline.clone();
        let mut record = conflicting.leases.get(&previous.lease_id).unwrap().clone();
        record.state.snapshot.owner = Some(competitor_owner);
        conflicting.leases.insert(OutputLeaseId(900), record);
        conflicting.index_resources(OutputLeaseId(900), &video());
        cases.push(conflicting);
    }
    // A Both turnover must also detect a conflicting Lighting-only claim.
    let (mut both_registry, _, both_previous) = seeded(both());
    let mut lighting_record = both_registry.leases.get(&both_previous.lease_id).unwrap().clone();
    lighting_record.state.snapshot.owner = Some(test_owner(2));
    lighting_record.state.snapshot.resources = Some(lighting());
    both_registry.leases.insert(OutputLeaseId(900), lighting_record);
    both_registry.index_resources(OutputLeaseId(900), &lighting());
    cases.push(both_registry);
    for mut registry in cases {
        let previous = registry.lease_view(previous.lease_id).unwrap();
        let before = registry.clone();
        assert_eq!(
            registry.query_video_authoring_lease(&owner, TTL_MS),
            Err(OutputLeaseError::ResourceConflict)
        );
        assert_eq!(registry, before);
        let receipt = registry
            .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), TTL_MS)
            .unwrap();
        assert_eq!(receipt.outcome, Err(OutputLeaseError::ResourceConflict));
        assert!(receipt.changes.is_empty());
        assert_authority_unchanged(&before, &registry);
    }
    // A dangling index must not turn an empty registry into an apparent vacancy.
    let mut empty = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    empty.index_resources(OutputLeaseId(900), &video());
    let before = empty.clone();
    assert_eq!(empty.query_video_authoring_lease(&owner, 0), Err(OutputLeaseError::ResourceConflict));
    assert_eq!(empty, before);
}

#[test]
fn video_authoring_lease_acquire_id_exhaustion_rolls_back_the_old_relinquish() {
    let (mut registry, owner, previous) = seeded(both());
    registry.next_lease_id = u64::MAX;
    let before = registry.clone();
    let receipt = registry
        .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), 1)
        .unwrap();
    assert_eq!(receipt.outcome, Err(OutputLeaseError::LeaseIdExhausted));
    assert!(receipt.changes.is_empty());
    assert_authority_unchanged(&before, &registry);
}

#[test]
fn video_authoring_lease_acquire_capacity_failure_rolls_back_the_old_relinquish() {
    let (mut registry, owner, previous) = seeded(video());
    for _ in 1..MAX_OUTPUT_LEASES {
        let retired = registry
            .acquire(owner.clone(), lighting(), PREVIOUS_PROJECT, 0, TTL_MS)
            .unwrap();
        registry
            .relinquish_output_lease(retired.lease_id, &owner, retired.snapshot.generation, 0)
            .unwrap();
    }
    assert_eq!(registry.leases.len(), MAX_OUTPUT_LEASES);
    let before = registry.clone();
    let receipt = registry
        .submit_request(&request(2, replacement(&previous, &owner, CURRENT_PROJECT)), 1)
        .unwrap();
    assert_eq!(receipt.outcome, Err(OutputLeaseError::LeaseCapacity));
    assert!(receipt.changes.is_empty());
    assert_authority_unchanged(&before, &registry);
}

#[test]
fn video_authoring_lease_durable_failure_rolls_back_every_registry_field_before_retry() {
    let (mut registry, owner, previous) = seeded(both());
    let request = request(2, replacement(&previous, &owner, CURRENT_PROJECT));
    let before = registry.clone();
    let mut observed = None;
    assert_eq!(
        registry.submit_request_with_commit(&request, 1, |receipt| {
            assert_eq!(receipt.outcome, Ok(OutputLeaseOperationOutcome::Acquired));
            assert_eq!(receipt.changes.len(), 2);
            assert_eq!(validate_persisted_receipt(receipt), Ok(()));
            observed = Some(receipt.clone());
            Err(OutputLeaseError::RequestCapacity)
        }),
        Err(OutputLeaseError::RequestCapacity)
    );
    assert_eq!(registry, before);
    let receipt = registry.submit_request(&request, 1).unwrap();
    assert_eq!(receipt, observed.unwrap());
    assert_replaced(&registry, &previous, &owner, &receipt);
}

#[test]
fn video_authoring_lease_replay_is_exact_and_all_action_fields_are_shape_bound() {
    let (mut registry, owner, previous) = seeded(both());
    let action = replacement(&previous, &owner, CURRENT_PROJECT);
    let original = request(2, action.clone());
    assert_eq!(original.shape.operation, "replace_previous_project_for_authoring");
    let receipt = registry.submit_request(&original, 1).unwrap();
    let before_replay = registry.clone();
    for now_ms in [2, TTL_MS + 2] {
        let replay = registry
            .submit_request_with_commit(&original, now_ms, |_| {
                panic!("a retained request must not commit or allocate again")
            })
            .unwrap();
        assert_eq!(replay, receipt);
        assert_eq!(registry, before_replay);
    }
    for field in 0..5 {
        let mut changed = action.clone();
        let OutputLeaseRequestAction::ReplacePreviousProjectForAuthoring {
            lease_id,
            owner,
            expected_generation,
            project_identity,
            ttl_ms,
        } = &mut changed
        else {
            unreachable!()
        };
        match field {
            0 => *lease_id = OutputLeaseId(900),
            1 => *owner = test_owner(2),
            2 => *expected_generation += 1,
            3 => *project_identity = "project_epoch:3".to_string(),
            4 => *ttl_ms += 1,
            _ => unreachable!(),
        }
        let conflict = request(2, changed);
        assert_ne!(conflict.shape_hash, original.shape_hash);
        assert_eq!(registry.submit_request(&conflict, 2), Err(OutputLeaseError::Conflict));
        assert_eq!(registry, before_replay);
    }
    assert_eq!(
        registry.submit_request(&original, OUTPUT_LEASE_RECEIPT_TTL_MS + 2),
        Err(OutputLeaseError::ReceiptNotRetained)
    );
    assert_eq!(registry, before_replay);
    // Even a different request id cannot use the now-unclaimed old identity.
    let rejected = registry.submit_request(&request(3, action), 2).unwrap();
    assert!(rejected.outcome.is_err());
    assert!(rejected.changes.is_empty());
    assert_authority_unchanged(&before_replay, &registry);
}
