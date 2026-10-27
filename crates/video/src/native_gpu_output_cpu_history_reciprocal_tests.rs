//! The inverse whole-owner bridge used by shared scoped CPU execution.
use super::canonical_tests::{evolved_history, scope, EvolvedHistory};
use super::*;
use crate::isf_native_gpu::await_transaction;
use crate::isf_native_gpu::tests::{request, Fixture};
use crate::native_gpu_budget::NativeAllocationPlan;
use crate::native_gpu_context::RetirementProgress;

#[test]
fn i8_cpu_history_shared_executor_alias_retains_whole_owner_without_new_charge_or_owner_chains() {
    let EvolvedHistory {
        mut fixture,
        host,
        snapshot,
        pointers,
        packed,
        ..
    } = evolved_history(1);
    let scope = snapshot.scope();
    let mut entry = NativeCpuHistoryQueue::from_snapshot(scope, snapshot)
        .unwrap_or_else(|failure| panic!("snapshot {:?}", failure.error))
        .into_entries()
        .pop()
        .unwrap();
    let original = Arc::downgrade(entry.backing());
    let before_gpu = fixture.owner.budget_snapshot();
    let before_host = host.snapshot();
    let alias = entry.native_cpu_owner().unwrap();
    let allocation = alias.charge.as_ref().unwrap().id();
    assert_eq!(alias.frame.data.as_ptr(), pointers[0]);
    assert!(std::ptr::eq(alias.frame.as_ref(), entry.frame()));
    assert_eq!(
        fixture.owner.budget_snapshot(),
        before_gpu,
        "alias must not mint another ledger allocation"
    );
    assert_eq!(
        host.snapshot(),
        before_host,
        "alias must retain, not recharge, the P host owner"
    );
    fixture.runtime.remember_native_cpu_frame(&alias);
    let mut plan = NativeAllocationPlan::new(request(99));
    plan.add_existing(allocation);
    let quote = fixture.owner.quote(&plan).unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut transaction = fixture.owner.begin_frame(request(99), reservation).unwrap();
    transaction
        .retain_resource(Arc::clone(&alias), &[allocation])
        .unwrap();
    let pending = transaction.finish();
    drop(await_transaction(&mut fixture.owner, pending, &|| false).unwrap());
    drop(alias);
    let mut hosts = Vec::new();
    for iteration in 0..3 {
        let alias = entry.native_cpu_owner().unwrap();
        let alias_weak = Arc::downgrade(&alias);
        let prior_backing = Arc::downgrade(entry.backing());
        assert_eq!(alias.charge.as_ref().unwrap().id(), allocation);
        let destination = HistoryHostBudget::new(packed);
        let imported = NativeGpuHistory::import_canonical_queue_with_owners(
            fixture.owner.context().id(),
            scope,
            destination.clone(),
            vec![CanonicalCpuHistoryFrame {
                entry: HistoryEntryId {
                    input: scope.input,
                    queue_epoch: scope.queue_epoch,
                    sequence: 100 + iteration,
                },
                frame: Arc::clone(&alias.frame),
                owner: Some(CanonicalCpuHistoryOwner::Native(Arc::clone(&alias))),
            }],
        )
        .unwrap_or_else(|failure| panic!("reciprocal import {:?}", failure.error));
        let snapshot = imported
            .begin_cpu_handover()
            .finish()
            .unwrap_or_else(|_| panic!("CPU alias"));
        let next = NativeCpuHistoryQueue::from_snapshot(scope, snapshot)
            .unwrap_or_else(|failure| panic!("CPU alias snapshot {:?}", failure.error))
            .into_entries()
            .pop()
            .unwrap();
        drop(entry);
        drop(alias);
        assert!(
            alias_weak.upgrade().is_none(),
            "re-import must not retain an intermediate G wrapper"
        );
        if iteration != 0 {
            assert!(
                prior_backing.upgrade().is_none(),
                "re-import must not retain an intermediate P wrapper"
            );
        }
        assert_eq!(next.frame().data.as_ptr(), pointers[0]);
        assert_eq!(destination.snapshot().managed_alias_bytes, packed);
        hosts.push(destination);
        entry = next;
    }
    let alias = entry.native_cpu_owner().unwrap();
    drop(entry);
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    owner.begin_stop();
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1
        }
    );
    assert_eq!(owner.budget_snapshot().managed_bytes(), packed);
    assert!(original.upgrade().is_some());
    drop(alias);
    assert!(original.upgrade().is_none());
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert!(hosts.iter().all(|host| host.snapshot().unique_bytes == 0));
}

#[test]
fn i8_cpu_history_host_only_executor_alias_keeps_the_complete_host_owner_without_a_fake_g_charge() {
    let context = GpuContextId {
        owner_id: 77,
        device_generation: 1,
    };
    let scope = scope(1, (2, 1));
    let host = HistoryHostBudget::new(8);
    let pixels = Arc::new(VideoFrame {
        layer_id: 1,
        width: 2,
        height: 1,
        pts_ms: 0,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: vec![7; 8],
    });
    let history = NativeGpuHistory::import_canonical_queue(
        context,
        scope,
        host.clone(),
        vec![(
            HistoryEntryId {
                input: scope.input,
                queue_epoch: scope.queue_epoch,
                sequence: 1,
            },
            pixels,
        )],
    )
    .unwrap();
    let snapshot = history
        .begin_cpu_handover()
        .finish()
        .unwrap_or_else(|_| panic!("CPU import"));
    let entry = NativeCpuHistoryQueue::from_snapshot(scope, snapshot)
        .unwrap_or_else(|failure| panic!("CPU snapshot {:?}", failure.error))
        .into_entries()
        .pop()
        .unwrap();
    let weak = Arc::downgrade(entry.backing());
    let alias = entry.native_cpu_owner().unwrap();
    assert!(alias.charge.is_none());
    assert!(alias.retained_owner().is_some());
    assert!(std::ptr::eq(alias.frame.as_ref(), entry.frame()));
    drop(entry);
    assert!(weak.upgrade().is_some());
    assert_eq!(host.snapshot().unique_bytes, 8);
    assert_eq!(host.snapshot().managed_alias_bytes, 0);
    drop(alias);
    assert!(weak.upgrade().is_none());
    assert_eq!(host.snapshot().unique_bytes, 0);
}
