//! Ledger oracles for the CPU-only cross-generation alias exception.
use super::*;

fn context(generation: u64) -> GpuContextId {
    GpuContextId {
        owner_id: 101,
        device_generation: generation,
    }
}

fn frame() -> NativeFrameId {
    NativeFrameId {
        output_id: 101,
        project_epoch: 7,
        request_sequence: 1,
        geometry_generation: 1,
    }
}

fn submission(generation: u64, sequence: u64) -> GpuSubmissionId {
    GpuSubmissionId {
        context: context(generation),
        sequence,
    }
}

fn allocate(budget: &mut NativeGpuBudget, kind: ChargeKind) -> AllocationCharge {
    let mut plan = NativeAllocationPlan::new(frame());
    plan.add_new(kind, 32).unwrap();
    let mut reservation = budget.reserve(budget.quote(&plan).unwrap()).unwrap();
    let id = budget.next_allocation_id().unwrap();
    let charge = budget
        .bind_allocation(&mut reservation, id, kind, 32)
        .unwrap();
    budget.cancel_reservation(reservation).unwrap();
    charge
}

fn alias_plan(id: GpuAllocationId) -> NativeAllocationPlan {
    let mut plan = NativeAllocationPlan::new(frame());
    plan.add_existing(id);
    plan
}

#[test]
fn i8_cpu_history_rearm_preserves_cpu_alias_identity_completion_and_final_pin_accounting() {
    for kind in [
        ChargeKind::ActiveCpuInput,
        ChargeKind::CpuSnapshot,
        ChargeKind::CpuLastValid,
    ] {
        let mut budget = NativeGpuBudget::new(context(1));
        let charge = allocate(&mut budget, kind);
        let id = charge.id();
        let pin = charge.pin();
        let plan = alias_plan(id);
        budget.record_use(id, submission(1, 99)).unwrap();
        budget.acknowledge_completed(submission(1, 99)).unwrap();
        budget.rearm_generation(context(7)).unwrap();
        let before = budget.snapshot();
        assert_eq!(before.for_kind(kind).retired_bytes, 32);
        assert!(budget.can_retain_allocation(id));
        let quote = budget.quote(&plan).unwrap();
        assert_eq!(
            budget.snapshot(),
            before,
            "quoting an old CPU owner is read-only"
        );
        let reservation = budget.reserve(quote).unwrap();
        budget.validate_reserved_plan(&reservation, &plan).unwrap();
        budget.validate_root_plan(&reservation, &plan).unwrap();
        budget.record_use(id, submission(7, 1)).unwrap();
        budget.cancel_reservation(reservation).unwrap();
        assert_eq!(budget.snapshot().allocation_count, 1);
        assert_eq!(budget.snapshot().managed_bytes(), 32);
        assert!(budget.record_use(id, submission(1, 100)).is_err());
        budget.acknowledge_completed(submission(7, 1)).unwrap();

        // There is no allocation born in generation 7, but its completed use
        // of generation 1's pixels is the proof needed after a second re-arm.
        budget.rearm_generation(context(13)).unwrap();
        assert_eq!(budget.completed.get(&context(7)), Some(&1));
        assert!(budget.quote(&plan).is_ok());
        budget.record_use(id, submission(13, 1)).unwrap();
        drop(charge);
        assert!(budget.try_reap(id).unwrap().is_none());
        drop(pin);
        assert!(
            budget.try_reap(id).unwrap().is_none(),
            "old completion does not complete the new use"
        );
        budget.acknowledge_completed(submission(7, 99)).unwrap();
        assert!(budget.try_reap(id).unwrap().is_none());
        budget.acknowledge_completed(submission(13, 1)).unwrap();
        let receipt = budget.try_reap(id).unwrap().unwrap();
        budget.release_reaped(receipt).unwrap();
        assert_eq!(budget.snapshot().managed_bytes(), 0);
        assert!(!budget.completed.contains_key(&context(1)));
        assert!(!budget.completed.contains_key(&context(7)));
    }
}

#[test]
fn i8_cpu_history_rearm_rejects_gpu_foreign_future_unbound_dead_and_uncompleted_aliases() {
    let mut budget = NativeGpuBudget::new(context(1));
    let gpu = [
        ChargeKind::IsfScratch,
        ChargeKind::ResidentLayer,
        ChargeKind::CompositionBuffer,
        ChargeKind::MappingBuffer,
        ChargeKind::SnapshotReadback,
        ChargeKind::QueueUploadStaging,
        ChargeKind::CachedEffectResource,
    ]
    .map(|kind| allocate(&mut budget, kind));
    let unbound = budget.next_allocation_id().unwrap();
    let cpu = allocate(&mut budget, ChargeKind::CpuSnapshot);
    let dead = allocate(&mut budget, ChargeKind::ActiveCpuInput);
    let dead_id = dead.id();
    drop(dead);
    budget.record_use(cpu.id(), submission(1, 9)).unwrap();
    let mut foreign = NativeGpuBudget::new(GpuContextId {
        owner_id: 202,
        ..context(1)
    });
    let foreign_charge = allocate(&mut foreign, ChargeKind::CpuSnapshot);
    let mut future = NativeGpuBudget::new(context(99));
    let future_charge = allocate(&mut future, ChargeKind::CpuSnapshot);
    let mut other_ledger = NativeGpuBudget::new(context(2));
    let other_charge = allocate(&mut other_ledger, ChargeKind::CpuSnapshot);
    budget.rearm_generation(context(7)).unwrap();

    let rejected = gpu
        .iter()
        .map(AllocationCharge::id)
        .chain([
            unbound,
            dead_id,
            foreign_charge.id(),
            future_charge.id(),
            other_charge.id(),
            cpu.id(),
        ])
        .collect::<Vec<_>>();
    let before = budget.snapshot();
    for id in rejected {
        assert!(!budget.can_retain_allocation(id), "invalid alias {id:?}");
        assert!(matches!(
            budget.quote(&alias_plan(id)),
            Err(NativeGpuDecline::QuoteExpired)
        ));
        assert!(budget.record_use(id, submission(7, 1)).is_err());
        assert_eq!(
            budget.snapshot(),
            before,
            "rejection cannot mutate or refund owners"
        );
    }
    budget.acknowledge_completed(submission(7, 99)).unwrap();
    assert!(
        !budget.can_retain_allocation(cpu.id()),
        "another generation cannot complete old work"
    );
    budget.acknowledge_completed(submission(1, 9)).unwrap();
    assert!(budget.can_retain_allocation(cpu.id()));
    budget.record_use(cpu.id(), submission(7, 100)).unwrap();
    budget.quarantine();
    assert_eq!(
        budget
            .snapshot()
            .for_kind(ChargeKind::CpuSnapshot)
            .quarantined_bytes,
        32,
        "failure quarantines an old CPU owner used by the failed new generation"
    );
    budget.rearm_generation(context(13)).unwrap();
    budget.acknowledge_completed(submission(7, 100)).unwrap();
    assert!(!budget.can_retain_allocation(cpu.id()));
    assert!(budget.quote(&alias_plan(cpu.id())).is_err());
    let id = cpu.id();
    drop(cpu);
    assert!(
        budget.try_reap(id).unwrap().is_none(),
        "re-arm and a late callback cannot clear quarantine"
    );
}
