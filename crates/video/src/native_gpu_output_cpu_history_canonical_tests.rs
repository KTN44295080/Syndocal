//! Real readback owners through the I8 all-survivor canonical replacement seam.
use super::*;
use crate::isf_native_gpu::tests::{gpu, identity_jobs, request, validated, Fixture};
use crate::isf_native_gpu::{CpuMaterializationRequest, NativeGpuMaterializer};
use crate::native_gpu_budget::ChargeKind;
use crate::native_gpu_context::RetirementProgress;
use crate::native_gpu_history::adapter::NativeGpuValidatedEntries;
use crate::native_gpu_history::CpuHistoryMaterializationRequest;
use crate::native_gpu_output::history::NativeOutputHistorySession;
use crate::FrameQueue;
use protocol::{VideoRenderInputId, VideoRenderInputKey};

pub(super) fn scope(layer: u64, extent: (u32, u32)) -> HistoryScope {
    HistoryScope {
        frame_scope: request(1),
        input: VideoRenderInputKey {
            project_render_epoch: 7,
            input_id: VideoRenderInputId(layer + 1000),
        },
        layer_id: layer,
        queue_epoch: 9,
        source_generation: 13,
        extent,
    }
}

pub(super) struct EvolvedHistory {
    pub(super) fixture: Fixture,
    pub(super) host: HistoryHostBudget,
    pub(super) snapshot: CpuHistorySnapshot<AllocationCharge>,
    pub(super) pointers: Vec<*const u8>,
    pub(super) oracle: Vec<Vec<u8>>,
    pub(super) packed: u64,
}

pub(super) fn evolved_history(count: u64) -> EvolvedHistory {
    let mut fixture = gpu();
    let extent = (65, 3);
    let session = NativeOutputHistorySession::new(
        fixture.owner.context().id(),
        (1..=3).map(|layer| scope(layer, extent)).collect(),
        extent,
    )
    .unwrap();
    let host = session.host_budget().clone();
    let mut history = session.into_histories().into_iter().next().unwrap();
    assert!(history.metadata().is_empty());
    let mut oracle = Vec::new();
    for sequence in 1..=count {
        let mut jobs = identity_jobs(extent.0, extent.1);
        for job in jobs.iter_mut() {
            job.entry.sequence = sequence;
            let pixels = Arc::get_mut(&mut job.input).unwrap();
            pixels.pts_ms = sequence * 100;
            pixels.data[2] = sequence as u8;
        }
        oracle.push(jobs[0].input.data.clone());
        let (layers, receipt) = validated(&mut fixture, sequence, jobs);
        let sealed = NativeGpuValidatedEntries::seal_isf_layer(
            &receipt,
            layers.into_iter().next().unwrap(),
            &host,
        )
        .unwrap_or_else(|failure| panic!("seal {:?}", failure.error));
        let plan = history.plan(&sealed.metadata(), sequence * 100).unwrap();
        drop(
            history
                .commit_validated(plan, sealed)
                .unwrap_or_else(|failure| panic!("commit {:?}", failure.error)),
        );
    }
    let clocks = fixture.runtime.native_frame_index();
    let mut handover = history.begin_cpu_handover();
    let mut pointers = Vec::new();
    while handover.pending() != 0 {
        let (entry, request) = match handover.next_entry().unwrap() {
            CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, resident } => (
                entry,
                CpuMaterializationRequest::ValidatedGpu {
                    entry,
                    lease: resident,
                },
            ),
            CpuHistoryMaterializationRequest::ReplayToCpu { entry, frozen } => (
                entry,
                CpuMaterializationRequest::FrozenReplay {
                    entry,
                    recipe: frozen,
                },
            ),
        };
        let materialized = NativeGpuMaterializer::new(&mut fixture.runtime)
            .materialize_cpu(request, &mut fixture.owner, &|| false)
            .unwrap();
        pointers.push(materialized.frame().data.as_ptr());
        accept_cpu_history_materialization(&mut handover, entry, materialized)
            .unwrap_or_else(|failure| panic!("handover {:?}", failure.error));
        drop(handover.take_retired());
    }
    let snapshot = handover
        .finish()
        .unwrap_or_else(|_| panic!("complete real handover must publish"));
    assert_eq!(fixture.runtime.native_frame_index(), clocks);
    let packed = u64::from(extent.0 * extent.1 * 4);
    fixture.owner.poll_retirement().unwrap();
    assert_eq!(
        fixture
            .owner
            .budget_snapshot()
            .for_kind(ChargeKind::CpuSnapshot)
            .active_bytes,
        count * packed
    );
    assert_eq!(host.snapshot().unique_bytes, count * packed);
    assert_eq!(host.snapshot().managed_alias_bytes, count * packed);
    EvolvedHistory {
        fixture,
        host,
        snapshot,
        pointers,
        oracle,
        packed,
    }
}

#[test]
fn i8_cpu_history_first_evolved_survivor_replaces_an_empty_canonical_queue_without_pixel_copy() {
    let EvolvedHistory {
        fixture,
        host,
        snapshot,
        pointers,
        oracle,
        packed,
    } = evolved_history(1);
    let scope = snapshot.scope();
    let history = NativeCpuHistoryQueue::from_snapshot(scope, snapshot)
        .unwrap_or_else(|failure| panic!("snapshot {:?}", failure.error));
    let whole_owner = Arc::clone(history.entries().next().unwrap().backing());
    let weak = Arc::downgrade(&whole_owner);
    let mut canonical = FrameQueue::new_for_input(scope.input, scope.layer_id, 8);
    assert_eq!(canonical.len(), 0);
    assert!(canonical.accepts_native_history_replacement(&history));
    let before = host.snapshot();
    canonical.install_native_history_replacement(history);
    assert_eq!(
        host.snapshot(),
        before,
        "publication is an alias, not another charge"
    );
    assert_eq!(canonical.len(), 1);
    let selected = canonical.frame_at_or_before(100).unwrap();
    assert_eq!(selected.data.as_ptr(), pointers[0]);
    assert_eq!(selected.data, oracle[0]);
    assert!(Arc::ptr_eq(
        canonical.native_owners[0].as_ref().unwrap().backing(),
        &whole_owner,
    ));
    assert_eq!(canonical.drop_before(101), 1);
    assert!(
        weak.upgrade().is_some(),
        "the whole consumer owner outlives its slot"
    );
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
    assert_eq!(host.snapshot().unique_bytes, packed);
    assert_eq!(whole_owner.frame().data.as_ptr(), pointers[0]);
    drop(whole_owner);
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(host.snapshot().unique_bytes, 0);
}

#[test]
fn i8_cpu_history_all_eight_replacement_rejections_preserve_the_complete_real_owner_set() {
    let EvolvedHistory {
        fixture,
        host,
        snapshot,
        pointers,
        oracle,
        packed,
    } = evolved_history(8);
    let scope = snapshot.scope();
    let original_backings = snapshot
        .entries()
        .map(|(_, backing)| Arc::downgrade(backing))
        .collect::<Vec<_>>();
    let mut foreign_scope = scope;
    foreign_scope.source_generation += 1;
    let before = host.snapshot();
    let rejected = NativeCpuHistoryQueue::from_snapshot(foreign_scope, snapshot)
        .err()
        .unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(host.snapshot(), before);
    for ((_, backing), original) in rejected.snapshot.entries().zip(&original_backings) {
        assert!(Arc::ptr_eq(backing, &original.upgrade().unwrap()));
    }
    let history = NativeCpuHistoryQueue::from_snapshot(scope, rejected.snapshot)
        .unwrap_or_else(|failure| panic!("snapshot {:?}", failure.error));
    let small = FrameQueue::new_for_input(scope.input, scope.layer_id, 7);
    assert!(!small.accepts_native_history_replacement(&history));
    let foreign = FrameQueue::new_for_input(scope.input, scope.layer_id + 1, 8);
    assert!(!foreign.accepts_native_history_replacement(&history));
    let mut exhausted = FrameQueue::new_for_input(scope.input, scope.layer_id, 8);
    exhausted.policy_revision = u64::MAX;
    assert!(!exhausted.accepts_native_history_replacement(&history));
    assert_eq!(history.len(), 8);
    assert_eq!(
        host.snapshot(),
        before,
        "rejected targets cannot mutate or uncharge history"
    );
    let mut canonical = FrameQueue::new_for_input(scope.input, scope.layer_id, 8);
    assert!(canonical.accepts_native_history_replacement(&history));
    canonical.install_native_history_replacement(history);
    for (index, pixels) in canonical.iter().enumerate() {
        assert_eq!(pixels.data.as_ptr(), pointers[index]);
        assert_eq!(pixels.data, oracle[index]);
        assert_eq!(
            canonical.native_entry_ids[index].unwrap().sequence,
            index as u64 + 1
        );
    }
    let fourth = Arc::clone(canonical.native_owners[3].as_ref().unwrap().backing());
    let replacement = Arc::new(VideoFrame {
        layer_id: scope.layer_id,
        width: scope.extent.0,
        height: scope.extent.1,
        pts_ms: 400,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: vec![77; packed as usize],
    });
    canonical.push_shared(replacement);
    assert!(canonical.native_owners[3].is_none());
    assert_eq!(
        host.snapshot().unique_bytes,
        8 * packed,
        "selected whole owner retains the replaced allocation"
    );
    drop(fourth);
    assert!(original_backings[3].upgrade().is_none());
    assert_eq!(host.snapshot().unique_bytes, 7 * packed);
    assert_eq!(canonical.drop_before(500), 4);
    assert_eq!(canonical.len(), 4);
    assert_eq!(host.snapshot().unique_bytes, 4 * packed);
    for (index, pixels) in canonical.iter().enumerate() {
        assert_eq!(pixels.data.as_ptr(), pointers[index + 4]);
    }
    drop(canonical);
    assert!(original_backings
        .iter()
        .all(|owner| owner.upgrade().is_none()));
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().charges, 0);
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

#[test]
fn i8_cpu_history_reimport_keeps_real_charges_and_returns_all_owners_before_sequence_commit() {
    let EvolvedHistory {
        fixture,
        host,
        snapshot,
        pointers,
        packed,
        ..
    } = evolved_history(1);
    let scope = snapshot.scope();
    let mut queue = NativeCpuHistoryQueue::from_snapshot(scope, snapshot)
        .unwrap_or_else(|failure| panic!("snapshot {:?}", failure.error))
        .into_entries();
    let (pixels, entry) = queue.pop().unwrap().into_canonical_parts();
    let original = Arc::clone(entry.backing());
    let mut sequence = CpuHistoryEntrySequence::new(scope, Some(20));
    let sources = || {
        vec![(
            Arc::clone(&pixels),
            Some(CanonicalCpuHistoryOwner::History(Arc::clone(&original))),
        )]
    };
    let tight = HistoryHostBudget::new(0);
    let before = host.snapshot();
    let rejected = import_canonical_cpu_history_with_binding(
        fixture.owner.context().id(),
        scope,
        &tight,
        &mut sequence,
        sources(),
    )
    .err()
    .unwrap();
    assert_eq!(rejected.error, HistoryError::HostBudgetExceeded);
    assert_eq!(sequence.last_issued(), Some(20));
    assert!(Arc::ptr_eq(&rejected.frames[0].0, &pixels));
    match rejected.frames[0].1.as_ref().unwrap() {
        CanonicalCpuHistoryOwner::History(backing) => assert!(Arc::ptr_eq(backing, &original)),
        _ => panic!("the whole original history owner must return"),
    }
    assert_eq!(tight.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot(), before);
    drop(rejected);
    let (history, ids) = import_canonical_cpu_history_with_binding(
        fixture.owner.context().id(),
        scope,
        &host,
        &mut sequence,
        sources(),
    )
    .unwrap_or_else(|failure| panic!("import {:?}", failure.error));
    assert_eq!(ids[0].sequence, 21);
    assert_eq!(sequence.last_issued(), Some(21));
    assert_eq!(
        host.snapshot(),
        before,
        "same-ledger re-entry keeps the exact backing"
    );
    let same = history
        .begin_cpu_handover()
        .finish()
        .unwrap_or_else(|_| panic!("CPU re-entry"));
    assert!(Arc::ptr_eq(same.entries().next().unwrap().1, &original));
    drop(same);

    let mut current = Arc::clone(&original);
    let mut destination = HistoryHostBudget::new(packed);
    for iteration in 0..3 {
        let prior = Arc::downgrade(&current);
        let source = CanonicalCpuHistoryFrame {
            entry: HistoryEntryId {
                sequence: 30 + iteration,
                input: scope.input,
                queue_epoch: scope.queue_epoch,
            },
            frame: Arc::clone(&pixels),
            owner: Some(CanonicalCpuHistoryOwner::History(current)),
        };
        if iteration != 0 {
            destination = HistoryHostBudget::new(packed);
        }
        let imported = NativeGpuHistory::import_canonical_queue_with_owners(
            fixture.owner.context().id(),
            scope,
            destination.clone(),
            vec![source],
        )
        .unwrap_or_else(|failure| panic!("cross-ledger import {:?}", failure.error));
        let imported = imported
            .begin_cpu_handover()
            .finish()
            .unwrap_or_else(|_| panic!("CPU re-entry"));
        current = imported.into_entries().pop().unwrap().1;
        assert_eq!(current.frame().data.as_ptr(), pointers[0]);
        assert_eq!(destination.snapshot().unique_bytes, packed);
        assert_eq!(destination.snapshot().managed_alias_bytes, packed);
        assert_eq!(destination.snapshot().unmanaged_bytes(), 0);
        match current.origin() {
            crate::native_gpu_history::CpuBackingOrigin::Canonical { owner, .. } => {
                let retained: Arc<dyn std::any::Any + Send + Sync> = original.clone();
                assert!(
                    Arc::ptr_eq(owner, &retained),
                    "re-entry flattens to the original charged owner"
                );
            }
            _ => panic!("cross-ledger alias retains its original complete owner"),
        }
        if iteration != 0 {
            assert!(
                prior.upgrade().is_none(),
                "obsolete intermediate backing must not form an owner chain"
            );
        }
    }
    drop(entry);
    drop(original);
    drop(pixels);
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
    assert_eq!(host.snapshot().unique_bytes, packed);
    drop(current);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(destination.snapshot().unique_bytes, 0);
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
}

#[test]
fn i8_cpu_history_native_cpu_owner_aliases_are_pointer_checked_and_host_deduplicated() {
    use crate::native_gpu_result::NativeCpuFrameOwner;
    let mut fixture = gpu();
    let extent = (65, 3);
    let (layers, _receipt) = validated(&mut fixture, 1, identity_jobs(extent.0, extent.1));
    let layer = layers.into_iter().next().unwrap();
    let materialized = NativeGpuMaterializer::new(&mut fixture.runtime)
        .materialize_cpu(
            CpuMaterializationRequest::ValidatedGpu {
                entry: layer.identity.entry,
                lease: &layer.lease,
            },
            &mut fixture.owner,
            &|| false,
        )
        .unwrap();
    let (pixels, _, charge) = materialized.into_parts();
    let pixels = Arc::new(pixels);
    let original = Arc::new(NativeCpuFrameOwner::new(
        Arc::clone(&pixels),
        Some(Arc::new(charge)),
    ));
    let packed = pixels.data.len() as u64;
    let host = HistoryHostBudget::new(packed);
    let scope1 = scope(1, extent);
    let make_input = |scope: HistoryScope, sequence| CanonicalCpuHistoryFrame {
        entry: HistoryEntryId {
            input: scope.input,
            queue_epoch: scope.queue_epoch,
            sequence,
        },
        frame: Arc::clone(&pixels),
        owner: Some(CanonicalCpuHistoryOwner::Native(Arc::clone(&original))),
    };
    let forged_pixels = Arc::new((*pixels).clone());
    let mut forged = make_input(scope1, 1);
    forged.frame = Arc::clone(&forged_pixels);
    let rejected = NativeGpuHistory::import_canonical_queue_with_owners(
        fixture.owner.context().id(),
        scope1,
        host.clone(),
        vec![make_input(scope1, 1), forged],
    )
    .err()
    .unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(
        rejected.frames.len(),
        2,
        "a later invalid slot returns every input owner"
    );
    assert!(Arc::ptr_eq(&rejected.frames[0].frame, &pixels));
    assert!(Arc::ptr_eq(&rejected.frames[1].frame, &forged_pixels));
    match rejected.frames[1].owner.as_ref().unwrap() {
        CanonicalCpuHistoryOwner::Native(owner) => assert!(Arc::ptr_eq(owner, &original)),
        _ => panic!("rejection must keep the complete G owner"),
    }
    assert_eq!(host.snapshot().charges, 0);
    drop(rejected);
    drop(forged_pixels);
    let first = NativeGpuHistory::import_canonical_queue_with_owners(
        fixture.owner.context().id(),
        scope1,
        host.clone(),
        vec![make_input(scope1, 1)],
    )
    .unwrap_or_else(|failure| panic!("native CPU import {:?}", failure.error));
    let scope2 = scope(2, extent);
    let second = NativeGpuHistory::import_canonical_queue_with_owners(
        fixture.owner.context().id(),
        scope2,
        host.clone(),
        vec![make_input(scope2, 1)],
    )
    .unwrap_or_else(|failure| panic!("native CPU alias {:?}", failure.error));
    assert_eq!(host.snapshot().unique_bytes, packed);
    assert_eq!(host.snapshot().allocations, 1);
    assert_eq!(host.snapshot().charges, 2);
    assert_eq!(host.snapshot().managed_alias_bytes, packed);
    drop(first);
    assert_eq!(host.snapshot().unique_bytes, packed);
    drop(original);
    drop(pixels);
    drop(layer);
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
    drop(second);
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(host.snapshot().unique_bytes, 0);
}

#[test]
fn i8_cpu_history_uncharged_canonical_aliases_remain_unmanaged() {
    use crate::native_gpu_result::NativeCpuFrameOwner;
    let context = GpuContextId {
        owner_id: 77,
        device_generation: 1,
    };
    let scope1 = scope(1, (2, 1));
    let scope2 = scope(2, (2, 1));
    let pixels = Arc::new(VideoFrame {
        layer_id: 1,
        width: 2,
        height: 1,
        pts_ms: 0,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: vec![7; 8],
    });
    let original_host = HistoryHostBudget::new(8);
    let imported = NativeGpuHistory::import_canonical_queue(
        context,
        scope1,
        original_host.clone(),
        vec![(
            HistoryEntryId {
                input: scope1.input,
                queue_epoch: scope1.queue_epoch,
                sequence: 1,
            },
            Arc::clone(&pixels),
        )],
    )
    .unwrap();
    let imported = imported
        .begin_cpu_handover()
        .finish()
        .unwrap_or_else(|_| panic!("imported CPU history"));
    let backing = imported.into_entries().pop().unwrap().1;
    let host = HistoryHostBudget::new(8);
    let first = NativeGpuHistory::import_canonical_queue_with_owners(
        context,
        scope1,
        host.clone(),
        vec![CanonicalCpuHistoryFrame {
            entry: HistoryEntryId {
                input: scope1.input,
                queue_epoch: scope1.queue_epoch,
                sequence: 2,
            },
            frame: Arc::clone(&pixels),
            owner: Some(CanonicalCpuHistoryOwner::History(backing)),
        }],
    )
    .unwrap_or_else(|failure| panic!("imported alias {:?}", failure.error));
    let native = Arc::new(NativeCpuFrameOwner::new(Arc::clone(&pixels), None));
    let second = NativeGpuHistory::import_canonical_queue_with_owners(
        context,
        scope2,
        host.clone(),
        vec![CanonicalCpuHistoryFrame {
            entry: HistoryEntryId {
                input: scope2.input,
                queue_epoch: scope2.queue_epoch,
                sequence: 1,
            },
            frame: Arc::clone(&pixels),
            owner: Some(CanonicalCpuHistoryOwner::Native(native)),
        }],
    )
    .unwrap_or_else(|failure| panic!("uncharged alias {:?}", failure.error));
    assert_eq!(host.snapshot().unique_bytes, 8);
    assert_eq!(host.snapshot().allocations, 1);
    assert_eq!(host.snapshot().managed_alias_bytes, 0);
    assert_eq!(host.snapshot().unmanaged_bytes(), 8);
    drop(first);
    assert_eq!(original_host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().unique_bytes, 8);
    drop(second);
    assert_eq!(host.snapshot().unique_bytes, 0);
}
