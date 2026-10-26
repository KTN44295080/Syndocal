use super::*;

fn context() -> GpuContextId {
    GpuContextId {
        owner_id: 7,
        device_generation: 1,
    }
}

fn frame(sequence: u64) -> NativeFrameId {
    NativeFrameId {
        output_id: 3,
        project_epoch: 11,
        request_sequence: sequence,
        geometry_generation: 1,
    }
}

fn plan(kind: ChargeKind, bytes: u64) -> NativeAllocationPlan {
    let mut plan = NativeAllocationPlan::new(frame(1));
    plan.add_new(kind, bytes).unwrap();
    plan
}

fn allocate(budget: &mut NativeGpuBudget, kind: ChargeKind, bytes: u64) -> AllocationCharge {
    let quote = budget.quote(&plan(kind, bytes)).unwrap();
    let mut reservation = budget.reserve(quote).unwrap();
    let id = budget.next_allocation_id().unwrap();
    let charge = budget
        .bind_allocation(&mut reservation, id, kind, bytes)
        .unwrap();
    budget.cancel_reservation(reservation).unwrap();
    charge
}

fn submission(sequence: u64) -> GpuSubmissionId {
    GpuSubmissionId {
        context: context(),
        sequence,
    }
}

#[test]
fn exact_512_mib_is_reserved_before_allocation_and_never_double_charged() {
    let mut budget = NativeGpuBudget::new(context());
    let plan = plan(ChargeKind::IsfScratch, NATIVE_MANAGED_LIMIT);
    let quote = budget.quote(&plan).unwrap();
    assert_eq!(quote.context(), context());
    assert_eq!(quote.frame(), frame(1));
    assert_eq!(quote.revision(), 0);
    assert!(quote.matches_plan(&plan));
    let mut reservation = budget.reserve(quote).unwrap();
    assert_eq!(reservation.context(), context());
    assert_eq!(reservation.frame(), frame(1));
    assert_eq!(reservation.capacity_bytes(), NATIVE_MANAGED_LIMIT);
    let reserved = budget.snapshot();
    assert_eq!(reserved.allocated_bytes, 0);
    assert_eq!(reserved.reserved_bytes, NATIVE_MANAGED_LIMIT);
    let id = budget.next_allocation_id().unwrap();
    let charge = budget
        .bind_allocation(
            &mut reservation,
            id,
            ChargeKind::IsfScratch,
            NATIVE_MANAGED_LIMIT,
        )
        .unwrap();
    assert_eq!(charge.id(), id);
    assert_eq!(charge.kind(), ChargeKind::IsfScratch);
    assert_eq!(charge.capacity(), NATIVE_MANAGED_LIMIT);
    assert_eq!(reservation.capacity_bytes(), 0);
    let bound = budget.snapshot();
    assert_eq!(bound.allocated_bytes, NATIVE_MANAGED_LIMIT);
    assert_eq!(bound.reserved_bytes, 0);
    assert_eq!(bound.managed_bytes(), NATIVE_MANAGED_LIMIT);
    assert_eq!(bound.high_water_bytes, NATIVE_MANAGED_LIMIT);
    budget.cancel_reservation(reservation).unwrap();
    drop(charge);
    assert_eq!(budget.snapshot().allocated_bytes, NATIVE_MANAGED_LIMIT);
    let receipt = budget.try_reap(id).unwrap().unwrap();
    assert_eq!(budget.snapshot().allocated_bytes, NATIVE_MANAGED_LIMIT);
    budget.release_reaped(receipt).unwrap();
    assert_eq!(budget.snapshot().managed_bytes(), 0);
    assert_eq!(budget.snapshot().high_water_bytes, NATIVE_MANAGED_LIMIT);
}

#[test]
fn over_budget_and_overflow_are_early_declines_without_ledger_mutation() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::CpuLastValid, 64);
    let before = budget.snapshot();
    assert_eq!(
        budget
            .quote(&plan(ChargeKind::IsfScratch, NATIVE_MANAGED_LIMIT))
            .unwrap_err(),
        NativeGpuDecline::BudgetExceeded {
            live: 64,
            requested: NATIVE_MANAGED_LIMIT,
            limit: NATIVE_MANAGED_LIMIT
        }
    );
    assert_eq!(budget.snapshot(), before);
    let mut overflow = plan(ChargeKind::ActiveCpuInput, u64::MAX);
    assert!(matches!(
        overflow.add_new(ChargeKind::CpuSnapshot, 1),
        Err(NativeGpuDecline::BudgetExceeded {
            requested: u64::MAX,
            ..
        })
    ));
    assert_eq!(overflow.requested_bytes(), u64::MAX);
    assert_eq!(budget.snapshot(), before);
    assert_eq!(
        NativeAllocationPlan::new(frame(1)).add_new(ChargeKind::CpuSnapshot, 0),
        Err(NativeGpuDecline::UnsupportedExtent)
    );
    drop(charge);
}

#[test]
fn bind_requires_exact_capacity_kind_and_unique_issued_identity() {
    let mut budget = NativeGpuBudget::new(context());
    let mut reservation = budget
        .reserve(budget.quote(&plan(ChargeKind::MappingBuffer, 768)).unwrap())
        .unwrap();
    let id = budget.next_allocation_id().unwrap();
    let before = budget.snapshot();
    // A 513-byte logical requirement may allocate a 768-byte grown buffer.
    for (kind, capacity) in [
        (ChargeKind::MappingBuffer, 513),
        (ChargeKind::MappingBuffer, 1024),
        (ChargeKind::CompositionBuffer, 768),
    ] {
        let fault = budget
            .bind_allocation(&mut reservation, id, kind, capacity)
            .unwrap_err();
        assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
        assert_eq!(budget.snapshot(), before);
        assert_eq!(reservation.capacity_bytes(), 768);
    }
    let forged = GpuAllocationId {
        context: context(),
        sequence: id.sequence + 1,
    };
    assert_eq!(
        budget
            .bind_allocation(&mut reservation, forged, ChargeKind::MappingBuffer, 768)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    let charge = budget
        .bind_allocation(&mut reservation, id, ChargeKind::MappingBuffer, 768)
        .unwrap();
    assert_eq!(
        budget
            .bind_allocation(&mut reservation, id, ChargeKind::MappingBuffer, 768)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    budget.cancel_reservation(reservation).unwrap();
    drop(charge);
    let receipt = budget.try_reap(id).unwrap().unwrap();
    budget.release_reaped(receipt).unwrap();
    let mut next = budget
        .reserve(budget.quote(&plan(ChargeKind::MappingBuffer, 768)).unwrap())
        .unwrap();
    assert_eq!(
        budget
            .bind_allocation(&mut next, id, ChargeKind::MappingBuffer, 768)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    budget.cancel_reservation(next).unwrap();
}

#[test]
fn stale_quotes_and_same_numeric_context_from_other_ledger_are_rejected() {
    let mut budget = NativeGpuBudget::new(context());
    let mut other = NativeGpuBudget::new(context());
    let plan = plan(ChargeKind::CpuSnapshot, 64);
    let stale = budget.quote(&plan).unwrap();
    let reservation = budget.reserve(budget.quote(&plan).unwrap()).unwrap();
    assert_eq!(
        budget.reserve(stale).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    let foreign = other.quote(&plan).unwrap();
    let before = budget.snapshot();
    assert_eq!(
        budget.reserve(foreign).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    assert_eq!(budget.snapshot(), before);
    let foreign_reservation = other.reserve(other.quote(&plan).unwrap()).unwrap();
    assert_eq!(
        budget
            .validate_reservation(&foreign_reservation, frame(1))
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    other.cancel_reservation(foreign_reservation).unwrap();
    budget.cancel_reservation(reservation).unwrap();
}

#[test]
fn admitted_plan_identity_survives_split_but_rejects_substitution_or_consumption() {
    let mut budget = NativeGpuBudget::new(context());
    let resident = allocate(&mut budget, ChargeKind::ResidentLayer, 64);
    let mut g_plan = plan(ChargeKind::IsfScratch, 256);
    g_plan.add_existing(resident.id());
    let p_plan = plan(ChargeKind::CompositionBuffer, 512);
    let mut complete = g_plan.clone();
    complete.merge(&p_plan).unwrap();
    let mut reservation = budget.reserve(budget.quote(&complete).unwrap()).unwrap();
    let child = budget.split_reservation(&mut reservation, &p_plan).unwrap();
    let before = budget.snapshot();
    assert!(budget.validate_reserved_plan(&reservation, &g_plan).is_ok());
    assert!(budget
        .validate_reserved_plan(&reservation, &complete)
        .is_ok());

    let mut same_shape = plan(ChargeKind::IsfScratch, 256);
    same_shape.add_existing(resident.id());
    assert_eq!(
        budget
            .validate_reserved_plan(&reservation, &same_shape)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    let mut wrong_frame = g_plan.clone();
    wrong_frame.frame = frame(2);
    assert_eq!(
        budget
            .validate_reserved_plan(&reservation, &wrong_frame)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    assert_eq!(
        budget
            .validate_reserved_plan(&child, &p_plan)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::StateMismatch
    );
    assert_eq!(budget.snapshot(), before);

    let id = budget.next_allocation_id().unwrap();
    let scratch = budget
        .bind_allocation(&mut reservation, id, ChargeKind::IsfScratch, 256)
        .unwrap();
    assert_eq!(
        budget
            .validate_reserved_plan(&reservation, &g_plan)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    assert!(budget.validate_reserved_plan(&reservation, &p_plan).is_ok());
    let mut aliases = NativeAllocationPlan::new(frame(1));
    aliases.add_existing(resident.id());
    assert!(budget
        .validate_reserved_plan(&reservation, &aliases)
        .is_ok());
    budget.mark_retired(resident.id()).unwrap();
    assert_eq!(
        budget
            .validate_reserved_plan(&reservation, &aliases)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    budget.cancel_reservation_tree(reservation).unwrap();
    drop((child, scratch, resident));
}

#[test]
fn frame_and_geometry_mismatch_reject_and_dropped_reservation_stays_accounted() {
    let mut budget = NativeGpuBudget::new(context());
    let reservation = budget
        .reserve(
            budget
                .quote(&plan(ChargeKind::SnapshotReadback, 128))
                .unwrap(),
        )
        .unwrap();
    let mut resized = frame(1);
    resized.geometry_generation += 1;
    for wrong in [frame(2), resized] {
        let fault = budget
            .validate_reservation(&reservation, wrong)
            .unwrap_err();
        assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
        assert_eq!(fault.frame, Some(wrong));
    }
    drop(reservation);
    assert_eq!(budget.snapshot().reserved_bytes, 128);
    assert_eq!(budget.snapshot().reservation_count, 1);
}

#[test]
fn composition_subreservation_is_exact_once_and_does_not_create_a_second_budget() {
    let mut budget = NativeGpuBudget::new(context());
    let mut g = plan(ChargeKind::IsfScratch, 64);
    let mut p = plan(ChargeKind::CompositionBuffer, 128);
    p.add_new(ChargeKind::SnapshotReadback, 256).unwrap();
    g.merge(&p).unwrap();
    // Merging the identical source plan again aliases its existing planned IDs.
    g.merge(&p).unwrap();
    assert_eq!(g.requested_bytes(), 448);
    assert_eq!(g.frame(), frame(1));
    let mut parent = budget.reserve(budget.quote(&g).unwrap()).unwrap();
    let before = budget.snapshot().managed_bytes();
    let mut child = budget.split_reservation(&mut parent, &p).unwrap();
    assert_eq!(parent.capacity_bytes(), 64);
    assert_eq!(child.capacity_bytes(), 384);
    assert_eq!(budget.snapshot().managed_bytes(), before);
    assert_eq!(
        budget.split_reservation(&mut parent, &p).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch
    );
    let id = budget.next_allocation_id().unwrap();
    let charge = budget
        .bind_allocation(&mut child, id, ChargeKind::CompositionBuffer, 128)
        .unwrap();
    budget.cancel_reservation(child).unwrap();
    budget.cancel_reservation(parent).unwrap();
    assert_eq!(budget.snapshot().allocated_bytes, 128);
    assert_eq!(budget.snapshot().reserved_bytes, 0);
    assert_eq!(budget.snapshot().high_water_bytes, 448);
    drop(charge);
}

#[test]
fn equal_byte_subplan_with_wrong_plan_identity_is_rejected_without_consumption() {
    let mut budget = NativeGpuBudget::new(context());
    let original = plan(ChargeKind::MappingBuffer, 256);
    let impostor = plan(ChargeKind::MappingBuffer, 256);
    let quote = budget.quote(&original).unwrap();
    assert!(!quote.matches_plan(&impostor));
    let mut reservation = budget.reserve(quote).unwrap();
    let before = budget.snapshot();
    assert_eq!(
        budget
            .split_reservation(&mut reservation, &impostor)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::StateMismatch
    );
    assert_eq!(budget.snapshot(), before);
    budget.cancel_reservation(reservation).unwrap();
}

#[test]
fn root_cancellation_reclaims_abandoned_subreservations_but_keeps_allocated_bytes() {
    let mut budget = NativeGpuBudget::new(context());
    let g = plan(ChargeKind::IsfScratch, 64);
    let p = plan(ChargeKind::CompositionBuffer, 128);
    let snapshot = plan(ChargeKind::SnapshotReadback, 256);
    let mut combined = NativeAllocationPlan::new(frame(1));
    combined.merge(&g).unwrap();
    combined.merge(&p).unwrap();
    combined.merge(&snapshot).unwrap();
    let mut root = budget.reserve(budget.quote(&combined).unwrap()).unwrap();
    let abandoned = budget.split_reservation(&mut root, &p).unwrap();
    let mut escaped = budget.split_reservation(&mut root, &snapshot).unwrap();
    let id = budget.next_allocation_id().unwrap();
    let charge = budget
        .bind_allocation(&mut root, id, ChargeKind::IsfScratch, 64)
        .unwrap();
    drop(abandoned);
    assert_eq!(budget.snapshot().reservation_count, 3);
    assert_eq!(budget.snapshot().managed_bytes(), 448);
    budget.cancel_reservation_tree(root).unwrap();
    assert_eq!(budget.snapshot().reservation_count, 0);
    assert_eq!(budget.snapshot().reserved_bytes, 0);
    assert_eq!(budget.snapshot().allocated_bytes, 64);
    let late_id = budget.next_allocation_id().unwrap();
    assert_eq!(
        budget
            .bind_allocation(&mut escaped, late_id, ChargeKind::SnapshotReadback, 256)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    drop(charge);
    let receipt = budget.try_reap(id).unwrap().unwrap();
    budget.release_reaped(receipt).unwrap();
    assert_eq!(budget.snapshot().managed_bytes(), 0);
}

#[test]
fn subreservation_cannot_cancel_its_parent_family() {
    let mut budget = NativeGpuBudget::new(context());
    let child_plan = plan(ChargeKind::MappingBuffer, 64);
    let mut root_plan = NativeAllocationPlan::new(frame(1));
    root_plan.merge(&child_plan).unwrap();
    let mut root = budget.reserve(budget.quote(&root_plan).unwrap()).unwrap();
    let child = budget.split_reservation(&mut root, &child_plan).unwrap();
    assert!(budget.validate_frame_reservation(&root, frame(1)).is_ok());
    assert!(budget.validate_reservation(&child, frame(1)).is_ok());
    assert_eq!(
        budget
            .validate_frame_reservation(&child, frame(1))
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::StateMismatch
    );
    assert_eq!(
        budget.cancel_reservation_tree(child).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch
    );
    assert_eq!(budget.snapshot().reservation_count, 2);
    assert_eq!(budget.snapshot().reserved_bytes, 64);
    budget.cancel_reservation_tree(root).unwrap();
    assert_eq!(budget.snapshot().managed_bytes(), 0);
}

#[test]
fn same_frame_subreservation_from_another_admission_is_rejected() {
    let mut budget = NativeGpuBudget::new(context());
    let child_plan = plan(ChargeKind::CompositionBuffer, 64);
    let mut total = NativeAllocationPlan::new(frame(1));
    total.merge(&child_plan).unwrap();
    let mut first = budget.reserve(budget.quote(&total).unwrap()).unwrap();
    let mut second = budget.reserve(budget.quote(&total).unwrap()).unwrap();
    let child = budget.split_reservation(&mut first, &child_plan).unwrap();
    let foreign_child = budget.split_reservation(&mut second, &child_plan).unwrap();
    assert!(budget.validate_subreservation(&first, &child).is_ok());
    let before = budget.snapshot();
    for wrong in [&foreign_child, &second, &first] {
        assert_eq!(
            budget
                .validate_subreservation(&first, wrong)
                .unwrap_err()
                .kind,
            NativeGpuFaultKind::IdentityMismatch
        );
        assert_eq!(budget.snapshot(), before);
    }
    budget.cancel_reservation_tree(first).unwrap();
    budget.cancel_reservation_tree(second).unwrap();
    assert_eq!(budget.snapshot().managed_bytes(), 0);
}

#[test]
fn merged_quote_covers_exact_g_subplan_but_not_foreign_slots_aliases_or_frames() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::ResidentLayer, 64);
    let mut g = plan(ChargeKind::IsfScratch, 256);
    g.add_existing(charge.id());
    let p = plan(ChargeKind::CompositionBuffer, 512);
    let mut combined = NativeAllocationPlan::new(frame(1));
    combined.merge(&g).unwrap();
    combined.merge(&p).unwrap();
    let quote = budget.quote(&combined).unwrap();
    assert!(!quote.matches_plan(&g));
    assert!(quote.covers_plan(&g));
    assert!(quote.covers_plan(&p));
    assert!(quote.covers_plan(&combined));
    let impostor = plan(ChargeKind::IsfScratch, 256);
    assert!(!quote.covers_plan(&impostor));
    let mut wrong_frame = g.clone();
    wrong_frame.frame.geometry_generation += 1;
    assert!(!quote.covers_plan(&wrong_frame));
    let mut unquoted_alias = g.clone();
    unquoted_alias.add_existing(GpuAllocationId {
        context: context(),
        sequence: charge.id().sequence + 1,
    });
    assert!(!quote.covers_plan(&unquoted_alias));
    drop(charge);
    // The quote never creates an accounting-only pin that could outlive the
    // physical resource. Reserve revalidates the actual owner's liveness.
    assert_eq!(
        budget.reserve(quote).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
}

#[test]
fn retired_or_unowned_alias_cannot_enter_a_new_frame_plan() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::ResidentLayer, 64);
    let mut aliases = NativeAllocationPlan::new(frame(2));
    aliases.add_existing(charge.id());
    assert!(budget.quote(&aliases).is_ok());
    budget.mark_retired(charge.id()).unwrap();
    assert_eq!(
        budget.quote(&aliases).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    drop(charge);
    assert_eq!(
        budget.quote(&aliases).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    assert_eq!(budget.snapshot().allocated_bytes, 64);
}

#[test]
fn aliases_are_counted_once_and_a_read_pin_prevents_release_after_gpu_completion() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::ResidentLayer, 1024);
    let id = charge.id();
    let pin = charge.pin();
    assert_eq!(pin.id(), id);
    let mut aliases = NativeAllocationPlan::new(frame(2));
    aliases.add_existing(id);
    aliases.add_existing(id);
    let reservation = budget.reserve(budget.quote(&aliases).unwrap()).unwrap();
    assert_eq!(budget.snapshot().allocated_bytes, 1024);
    assert_eq!(budget.snapshot().allocation_count, 1);
    budget.cancel_reservation(reservation).unwrap();
    budget.record_use(id, submission(1)).unwrap();
    budget.mark_retired(id).unwrap();
    budget.acknowledge_completed(submission(1)).unwrap();
    drop(charge);
    assert!(budget.try_reap(id).unwrap().is_none());
    assert_eq!(
        budget
            .snapshot()
            .for_kind(ChargeKind::ResidentLayer)
            .retired_bytes,
        1024
    );
    drop(pin);
    let receipt = budget.try_reap(id).unwrap().unwrap();
    budget.release_reaped(receipt).unwrap();
    assert_eq!(budget.snapshot().managed_bytes(), 0);
}

#[test]
fn last_submission_not_first_snapshot_controls_reap_even_after_all_read_owners_drop() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::CompositionBuffer, 4096);
    let id = charge.id();
    budget.record_use(id, submission(1)).unwrap();
    budget.acknowledge_completed(submission(1)).unwrap();
    // Mapping/present is a later submission than the completed snapshot.
    budget.record_use(id, submission(2)).unwrap();
    drop(charge);
    assert!(budget.try_reap(id).unwrap().is_none());
    assert_eq!(budget.snapshot().allocated_bytes, 4096);
    budget.acknowledge_completed(submission(2)).unwrap();
    let receipt = budget.try_reap(id).unwrap().unwrap();
    assert!(budget.try_reap(id).unwrap().is_none());
    budget.release_reaped(receipt).unwrap();
    assert_eq!(budget.snapshot().allocated_bytes, 0);
}

#[test]
fn foreign_submission_and_retired_geometry_cannot_bypass_live_peak() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(
        &mut budget,
        ChargeKind::ResidentLayer,
        NATIVE_MANAGED_LIMIT / 2,
    );
    let id = charge.id();
    let foreign = GpuSubmissionId {
        context: GpuContextId {
            owner_id: 99,
            ..context()
        },
        sequence: 1,
    };
    assert_eq!(
        budget.record_use(id, foreign).unwrap_err().kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    assert_eq!(
        budget.acknowledge_completed(foreign).unwrap_err().kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    budget.record_use(id, submission(2)).unwrap();
    assert_eq!(
        budget.record_use(id, submission(1)).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch
    );
    budget.mark_retired(id).unwrap();
    let mut new_frame = frame(2);
    new_frame.geometry_generation = 2;
    let mut resized = NativeAllocationPlan::new(new_frame);
    resized
        .add_new(ChargeKind::ResidentLayer, NATIVE_MANAGED_LIMIT / 2 + 1)
        .unwrap();
    assert!(
        matches!(budget.quote(&resized), Err(NativeGpuDecline::BudgetExceeded { live, .. }) if live == NATIVE_MANAGED_LIMIT / 2)
    );
    drop(charge);
    assert!(budget.try_reap(id).unwrap().is_none());
    budget.acknowledge_completed(submission(2)).unwrap();
    let receipt = budget.try_reap(id).unwrap().unwrap();
    budget.release_reaped(receipt).unwrap();
    assert!(budget.quote(&resized).is_ok());
}

#[test]
fn old_generation_retained_bytes_share_the_same_session_limit() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::IsfScratch, NATIVE_MANAGED_LIMIT);
    let id = charge.id();
    budget.record_use(id, submission(1)).unwrap();
    budget.mark_retired(id).unwrap();
    let next = GpuContextId {
        device_generation: 2,
        ..context()
    };
    budget.advance_generation(next).unwrap();
    assert_eq!(budget.snapshot().context, next);
    assert_eq!(budget.snapshot().managed_bytes(), NATIVE_MANAGED_LIMIT);
    assert!(matches!(
        budget.quote(&plan(ChargeKind::CpuSnapshot, 4)),
        Err(NativeGpuDecline::BudgetExceeded { .. })
    ));
    assert_eq!(
        budget.record_use(id, submission(2)).unwrap_err().kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    drop(charge);
    budget.acknowledge_completed(submission(1)).unwrap();
    let receipt = budget.try_reap(id).unwrap().unwrap();
    budget.release_reaped(receipt).unwrap();
    assert_eq!(budget.snapshot().managed_bytes(), 0);
    let new_id = budget.next_allocation_id().unwrap();
    assert_eq!(new_id.context, next);
    assert!(new_id.sequence > id.sequence);
}

#[test]
fn cleanup_failure_quarantines_charges_even_if_callbacks_later_report_completion() {
    let mut budget = NativeGpuBudget::new(context());
    let charge = allocate(&mut budget, ChargeKind::QueueUploadStaging, 512);
    let id = charge.id();
    budget.record_use(id, submission(1)).unwrap();
    let reservation = budget
        .reserve(budget.quote(&plan(ChargeKind::CpuSnapshot, 128)).unwrap())
        .unwrap();
    budget.quarantine();
    let retained = budget.snapshot();
    assert_eq!(retained.managed_bytes(), 640);
    assert_eq!(
        retained
            .for_kind(ChargeKind::QueueUploadStaging)
            .quarantined_bytes,
        512
    );
    assert_eq!(
        budget.quote(&plan(ChargeKind::CpuSnapshot, 4)).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    assert_eq!(
        budget
            .advance_generation(GpuContextId {
                device_generation: 2,
                ..context()
            })
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::StateMismatch
    );
    drop(charge);
    budget.acknowledge_completed(submission(1)).unwrap();
    assert!(budget.try_reap(id).unwrap().is_none());
    budget.cancel_reservation(reservation).unwrap();
    assert_eq!(budget.snapshot().allocated_bytes, 512);
    assert_eq!(budget.snapshot().reserved_bytes, 0);
}

#[test]
fn checked_pitch_capacity_and_kind_accounting_cover_the_entire_plan() {
    assert_eq!(checked_upload_staging_bytes(129, 2, Some(256)), Ok(1536));
    assert_eq!(checked_upload_staging_bytes(129, 2, Some(512)), Ok(2048));
    assert_eq!(checked_upload_staging_bytes(129, 2, Some(4)), Ok(1536));
    for pitch in [None, Some(0), Some(3), Some(u64::MAX)] {
        assert_eq!(
            checked_upload_staging_bytes(2560, 1600, pitch),
            Err(NativeGpuDecline::UnknownUploadPitch)
        );
    }
    assert_eq!(
        checked_upload_staging_bytes(0, 1, Some(256)),
        Err(NativeGpuDecline::UnsupportedExtent)
    );
    assert_eq!(
        checked_upload_staging_bytes(u32::MAX, u32::MAX, Some(512)),
        Err(NativeGpuDecline::UnsupportedExtent)
    );
    assert_eq!(
        checked_upload_staging_bytes(2560, 1600, Some(256)),
        Ok(16_384_000)
    );
    let mut budget = NativeGpuBudget::new(context());
    let kinds = [
        ChargeKind::IsfScratch,
        ChargeKind::ResidentLayer,
        ChargeKind::CompositionBuffer,
        ChargeKind::MappingBuffer,
        ChargeKind::SnapshotReadback,
        ChargeKind::QueueUploadStaging,
        ChargeKind::ActiveCpuInput,
        ChargeKind::CpuSnapshot,
        ChargeKind::CpuLastValid,
        ChargeKind::CachedEffectResource,
    ];
    let mut all = NativeAllocationPlan::new(frame(1));
    for kind in kinds {
        all.add_new(kind, 256).unwrap();
    }
    let mut reservation = budget.reserve(budget.quote(&all).unwrap()).unwrap();
    let mut charges = Vec::new();
    for kind in kinds {
        let id = budget.next_allocation_id().unwrap();
        charges.push(
            budget
                .bind_allocation(&mut reservation, id, kind, 256)
                .unwrap(),
        );
        assert_eq!(budget.snapshot().for_kind(kind).active_bytes, 256);
        assert_eq!(budget.snapshot().for_kind(kind).high_water_bytes, 256);
    }
    assert_eq!(budget.live_allocation_ids().count(), kinds.len());
    assert_eq!(budget.snapshot().managed_bytes(), 2560);
    budget.cancel_reservation(reservation).unwrap();
    drop(charges);
}

#[test]
fn sequence_exhaustion_is_typed_and_never_wraps_or_resets_the_ledger() {
    let mut budget = NativeGpuBudget::new(context());
    budget.allocation_sequence = u64::MAX;
    assert_eq!(
        budget.next_allocation_id().unwrap_err().kind,
        NativeGpuFaultKind::SequenceExhausted
    );
    assert_eq!(budget.allocation_sequence, u64::MAX);
    budget.reservation_sequence = u64::MAX;
    assert_eq!(
        budget.quote(&plan(ChargeKind::CpuSnapshot, 4)).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    budget.reservation_sequence = 0;
    budget.context.device_generation = u64::MAX;
    assert_eq!(
        budget.advance_generation(context()).unwrap_err().kind,
        NativeGpuFaultKind::SequenceExhausted
    );
    budget.context = context();
    budget.revision = u64::MAX;
    assert_eq!(
        budget.quote(&plan(ChargeKind::CpuSnapshot, 4)).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    assert_eq!(budget.snapshot().managed_bytes(), 0);
}
