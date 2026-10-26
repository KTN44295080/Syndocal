use super::*;
use crate::isf_native_gpu::tests::{gpu, identity_jobs, request, validated, Fixture};
use crate::isf_native_gpu::{CpuMaterializationRequest, NativeGpuMaterializer};
use crate::native_gpu_budget::ChargeKind;
use crate::native_gpu_context::{NativeGpuOwner, RetirementProgress};
use crate::native_gpu_history::adapter::NativeGpuValidatedEntries;
use crate::native_gpu_history::{CpuBackingOrigin, CpuHistoryMaterializationRequest};
use crate::native_gpu_output::history::NativeOutputHistorySession;
use crate::{FrameQueue, FrameQueuePush};
use protocol::{VideoRenderInputId, VideoRenderInputKey};

fn scope(layer: u64, extent: (u32, u32)) -> HistoryScope {
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

fn context() -> GpuContextId {
    GpuContextId {
        owner_id: 101,
        device_generation: 1,
    }
}

fn frame(pts_ms: u64, marker: u8) -> Arc<VideoFrame> {
    Arc::new(VideoFrame {
        layer_id: 1,
        width: 2,
        height: 1,
        pts_ms,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: vec![marker, 13, 17, 255, 19, 23, 29, 255],
    })
}

fn snapshot(history: NativeGpuHistory) -> CpuHistorySnapshot<AllocationCharge> {
    let handover = history.begin_cpu_handover();
    assert_eq!(handover.pending(), 0);
    handover
        .finish()
        .unwrap_or_else(|_| panic!("complete imported CPU history must finish"))
}

fn receive(snapshot: CpuHistorySnapshot<AllocationCharge>) -> NativeCpuHistoryQueue {
    NativeCpuHistoryQueue::from_snapshot(snapshot.scope(), snapshot)
        .unwrap_or_else(|rejected| panic!("receive {:?}", rejected.error))
}

fn import(
    scope: HistoryScope,
    host: &HistoryHostBudget,
    sequence: &mut CpuHistoryEntrySequence,
    frames: Vec<Arc<VideoFrame>>,
) -> NativeGpuHistory {
    import_legacy_cpu_history(context(), scope, host, sequence, frames)
        .unwrap_or_else(|rejected| panic!("legacy import {:?}", rejected.error))
}

fn cpu_bytes(owner: &NativeGpuOwner) -> u64 {
    let budget = owner.budget_snapshot();
    let cpu = budget.for_kind(ChargeKind::CpuSnapshot);
    cpu.active_bytes + cpu.retired_bytes + cpu.quarantined_bytes
}

#[test]
fn i5_cpu_history_real_full_eight_handover_last_valid_alias_holds_charge_through_stop() {
    let mut fixture = gpu();
    let extent = (65, 3);
    // Consume the actual I output-session owner into its per-input histories;
    // every handover keeps that same ledger rather than creating a CPU budget.
    let session = NativeOutputHistorySession::new(
        fixture.owner.context().id(),
        (1..=3).map(|layer| scope(layer, extent)).collect(),
        extent,
    )
    .unwrap();
    let host = session.host_budget().clone();
    let mut history = session.into_histories().into_iter().next().unwrap();
    let scope = history.scope();
    let mut oracle = Vec::new();
    for sequence in 1..=8 {
        let mut jobs = identity_jobs(extent.0, extent.1);
        for job in jobs.iter_mut() {
            job.entry.sequence = sequence;
            let frame = Arc::get_mut(&mut job.input).unwrap();
            frame.pts_ms = sequence * 100;
            frame.data[2] = sequence as u8;
        }
        oracle.push(jobs[0].input.data.clone());
        let (layers, receipt) = validated(&mut fixture, sequence, jobs);
        let layer = layers.into_iter().next().unwrap();
        let sealed = NativeGpuValidatedEntries::seal_isf_layer(&receipt, layer, &host)
            .unwrap_or_else(|failure| panic!("seal {:?}", failure.error));
        // ISF-valid history is useful even without an output composition. This
        // fixture does not fabricate a successful composition receipt.
        let plan = history.plan(&sealed.metadata(), sequence * 100).unwrap();
        let commit = history
            .commit_validated(plan, sealed)
            .unwrap_or_else(|failure| panic!("commit {:?}", failure.error));
        assert!(!commit.output_phases_valid);
        drop(commit);
    }
    assert_eq!(history.metadata().len(), NATIVE_HISTORY_CAPACITY);
    let clocks = fixture.runtime.native_frame_index();
    let mut handover = history.begin_cpu_handover();
    assert_eq!(handover.len(), NATIVE_HISTORY_CAPACITY);
    let mut pointers = Vec::new();
    let mut replayed = 0;
    let mut read_residents = 0;
    while handover.pending() != 0 {
        let (entry, materialized) = {
            let (entry, request) = match handover.next_entry().unwrap() {
                CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, resident } => {
                    read_residents += 1;
                    (
                        entry,
                        CpuMaterializationRequest::ValidatedGpu {
                            entry,
                            lease: resident,
                        },
                    )
                }
                CpuHistoryMaterializationRequest::ReplayToCpu { entry, frozen } => {
                    replayed += 1;
                    (
                        entry,
                        CpuMaterializationRequest::FrozenReplay {
                            entry,
                            recipe: frozen,
                        },
                    )
                }
            };
            let materialized = NativeGpuMaterializer::new(&mut fixture.runtime)
                .materialize_cpu(request, &mut fixture.owner, &|| false)
                .unwrap();
            (entry, materialized)
        };
        let pointer = materialized.frame().data.as_ptr();
        pointers.push(pointer);
        let materialized = if entry.sequence == 4 {
            let descriptor = *materialized.descriptor();
            let host_before = host.snapshot();
            let pending_before = handover.pending();
            let rejected = accept_cpu_history_materialization(
                &mut handover,
                HistoryEntryId {
                    sequence: entry.sequence + 100,
                    ..entry
                },
                materialized,
            )
            .err()
            .expect("a different requested entry must be refused");
            assert_eq!(rejected.error, HistoryError::IdentityMismatch);
            assert_eq!(rejected.materialized.frame().data.as_ptr(), pointer);
            assert_eq!(rejected.materialized.descriptor(), &descriptor);
            assert_eq!(handover.pending(), pending_before);
            assert_eq!(host.snapshot(), host_before);
            handover = handover
                .finish()
                .err()
                .expect("a partial barrier must retain its mixed owners");
            rejected.materialized
        } else {
            materialized
        };
        accept_cpu_history_materialization(&mut handover, entry, materialized)
            .unwrap_or_else(|failure| panic!("accept {:?}", failure.error));
        drop(handover.take_retired());
    }
    assert_eq!(replayed, 6);
    assert_eq!(read_residents, 2);
    assert_eq!(fixture.runtime.native_frame_index(), clocks);
    let snapshot = handover
        .finish()
        .unwrap_or_else(|_| panic!("all eight conversions must finish"));
    let queue = receive(snapshot);
    assert_eq!(queue.scope(), scope);
    assert_eq!(queue.len(), 8);
    let packed = u64::from(extent.0 * extent.1 * 4);
    for (index, entry) in queue.entries().enumerate() {
        assert_eq!(entry.identity().entry.sequence, index as u64 + 1);
        assert_eq!(entry.frame().data, oracle[index]);
        assert_eq!(entry.frame().data.as_ptr(), pointers[index]);
        match entry.backing().origin() {
            CpuBackingOrigin::Materialized { descriptor, charge } => {
                assert_eq!(descriptor.layer.entry, entry.identity().entry);
                assert_eq!(charge.kind(), ChargeKind::CpuSnapshot);
                assert_eq!(charge.capacity(), packed);
            }
            _ => panic!("real materialization must keep its actual G charge"),
        }
    }
    fixture.owner.poll_retirement().unwrap();
    assert_eq!(cpu_bytes(&fixture.owner), 8 * packed);
    assert_eq!(host.snapshot().unique_bytes, 8 * packed);
    assert_eq!(host.snapshot().managed_alias_bytes, 8 * packed);
    assert_eq!(host.snapshot().unmanaged_bytes(), 0);

    let selected = queue.select(450).unwrap();
    assert_eq!(selected.identity().entry.sequence, 4);
    let consumer_alias = Arc::clone(selected.backing());
    let weak = Arc::downgrade(selected.backing());
    let mut last_valid = NativeCpuHistoryLastValid::new(scope);
    last_valid
        .retain(selected)
        .unwrap_or_else(|failure| panic!("LastValid {:?}", failure.error));
    drop(queue);
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    owner.begin_stop();
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1,
        }
    );
    assert_eq!(cpu_bytes(&owner), packed);
    assert_eq!(owner.budget_snapshot().managed_bytes(), packed);
    assert_eq!(host.snapshot().unique_bytes, packed);
    assert_eq!(last_valid.get().unwrap().frame().data.as_ptr(), pointers[3]);
    last_valid.clear();
    assert!(last_valid.get().is_none());
    assert_eq!(consumer_alias.frame().data.as_ptr(), pointers[3]);
    assert_eq!(host.snapshot().unique_bytes, packed);
    assert!(weak.upgrade().is_some());
    drop(consumer_alias);
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().charges, 0);
}

fn assert_matches_canonical(queue: &NativeCpuHistoryQueue, canonical: &FrameQueue) {
    assert_eq!(queue.len(), canonical.len());
    for (entry, frame) in queue.entries().zip(canonical.iter()) {
        assert_eq!(entry.frame(), frame);
    }
    for target in [0, 16, 20, 33, 99, 131, 165, 230, 250, 500] {
        let expected = canonical
            .frame_at_or_before(target)
            .or_else(|| canonical.nearest(target));
        assert_eq!(
            queue
                .select(target)
                .as_ref()
                .map(OwnedCpuHistoryEntry::frame),
            expected
        );
        assert_eq!(
            queue
                .nearest(target)
                .as_ref()
                .map(OwnedCpuHistoryEntry::frame),
            canonical.nearest(target)
        );
    }
}

#[test]
fn i5_cpu_history_queue_matches_canonical_replacement_old_insert_and_selection() {
    let scope = scope(1, (2, 1));
    let host = HistoryHostBudget::new(1024);
    let mut sequence = CpuHistoryEntrySequence::new(scope, Some(40));
    let frames: Vec<_> = (0..8).map(|n| frame(n * 33, n as u8)).collect();
    let mut canonical = FrameQueue::new_for_input(scope.input, scope.layer_id, 8);
    for frame in &frames {
        canonical.push_shared(Arc::clone(frame));
    }
    let mut queue = receive(snapshot(import(scope, &host, &mut sequence, frames)));
    assert_eq!(queue.metadata()[0].id.sequence, 41);
    assert_eq!(sequence.last_issued(), Some(48));
    assert_matches_canonical(&queue, &canonical);
    let stale = queue.select(0).unwrap();
    let old_replaced = queue.select(99).unwrap();
    let weak_replaced = Arc::downgrade(old_replaced.backing());
    let mut last_valid = NativeCpuHistoryLastValid::new(scope);
    last_valid.retain(old_replaced).ok().unwrap();
    for (pts, marker) in [(99, 90), (5, 91), (5, 92), (300, 93)] {
        let frame = frame(pts, marker);
        let pointer = frame.data.as_ptr();
        let expected = canonical.push_shared(Arc::clone(&frame));
        let incoming = receive(snapshot(import(scope, &host, &mut sequence, vec![frame])))
            .into_entries()
            .pop()
            .unwrap();
        let result = queue.push(incoming).ok().unwrap();
        assert_eq!(
            matches!(result, HistoryPushKind::DroppedOldest { .. }),
            expected == FrameQueuePush::DroppedOldest
        );
        assert_eq!(queue.select(pts).unwrap().frame().data.as_ptr(), pointer);
        assert_matches_canonical(&queue, &canonical);
    }
    assert_eq!(last_valid.get().unwrap().frame().data[0], 3);
    assert_eq!(queue.select(99).unwrap().frame().data[0], 90);
    assert!(weak_replaced.upgrade().is_some());
    assert_eq!(queue.drop_before(132), canonical.drop_before(132));
    assert_eq!(queue.metadata()[0].pts_ms, 132, "equality survives");
    assert_matches_canonical(&queue, &canonical);
    assert_eq!(queue.drop_before(500), canonical.drop_before(500));
    let stale_backing = Arc::clone(stale.backing());
    let rejected = queue.push(stale).err().unwrap();
    assert_eq!(rejected.error, HistoryError::StateMismatch);
    assert!(Arc::ptr_eq(rejected.entry.backing(), &stale_backing));
    assert_eq!(queue.len(), 0);
    last_valid.clear();
    assert!(weak_replaced.upgrade().is_none());
}

#[test]
fn i5_cpu_history_snapshot_and_last_valid_rejections_return_whole_scoped_owners() {
    let scope = scope(1, (2, 1));
    let host = HistoryHostBudget::new(64);
    let mut sequence = CpuHistoryEntrySequence::new(scope, None);
    let published = snapshot(import(scope, &host, &mut sequence, vec![frame(0, 1)]));
    let held = Arc::clone(published.entries().next().unwrap().1);
    let before = host.snapshot();
    let mut foreign_scope = scope;
    foreign_scope.source_generation += 1;
    let rejected = NativeCpuHistoryQueue::from_snapshot(foreign_scope, published)
        .err()
        .unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(host.snapshot(), before);
    assert!(Arc::ptr_eq(
        rejected.snapshot.entries().next().unwrap().1,
        &held
    ));
    let queue = receive(rejected.snapshot);
    let selected = queue.select(0).unwrap();
    let mut last_valid = NativeCpuHistoryLastValid::new(scope);
    last_valid.retain(selected.clone()).ok().unwrap();
    let mut foreign_last_valid = NativeCpuHistoryLastValid::new(foreign_scope);
    let rejected = foreign_last_valid.retain(selected).err().unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert!(foreign_last_valid.get().is_none());
    assert!(Arc::ptr_eq(rejected.entry.backing(), &held));
    assert!(Arc::ptr_eq(last_valid.get().unwrap().backing(), &held));
    let mut foreign_sequence = CpuHistoryEntrySequence::new(foreign_scope, Some(9));
    let incoming = receive(snapshot(import(
        foreign_scope,
        &host,
        &mut foreign_sequence,
        vec![frame(33, 2)],
    )))
    .into_entries()
    .pop()
    .unwrap();
    let rejected = last_valid.retain(incoming.clone()).err().unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert!(Arc::ptr_eq(last_valid.get().unwrap().backing(), &held));
    assert!(Arc::ptr_eq(rejected.entry.backing(), incoming.backing()));
    let mut queue = queue;
    let rejected = queue.push(incoming).err().unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(rejected.entry.identity().source_generation, 14);
    assert_eq!(queue.len(), 1);
}

#[test]
fn i5_cpu_history_legacy_import_shares_session_budget_and_preserves_rejected_sources() {
    let scope1 = scope(1, (2, 1));
    let scope2 = scope(2, (2, 1));
    let host = HistoryHostBudget::new(8);
    let pixels = frame(0, 1);
    let mut sequence1 = CpuHistoryEntrySequence::new(scope1, Some(20));
    let mut sequence2 = CpuHistoryEntrySequence::new(scope2, Some(90));
    let first = import(scope1, &host, &mut sequence1, vec![Arc::clone(&pixels)]);
    let second = import(scope2, &host, &mut sequence2, vec![Arc::clone(&pixels)]);
    assert_eq!(sequence1.last_issued(), Some(21));
    assert_eq!(sequence2.last_issued(), Some(91));
    assert_eq!(host.snapshot().unique_bytes, 8);
    assert_eq!(host.snapshot().allocations, 1);
    assert_eq!(host.snapshot().charges, 2);
    assert_eq!(host.snapshot().managed_alias_bytes, 0);
    let queue = receive(snapshot(second));
    let selected = queue.select(0).unwrap();
    assert_eq!(selected.identity().layer_id, 2);
    assert_eq!(
        selected.frame().layer_id,
        1,
        "diagnostic tag stays borrowed"
    );
    assert_eq!(selected.frame().data.as_ptr(), pixels.data.as_ptr());

    let refused_pixels = frame(33, 2);
    let before = host.snapshot();
    let rejected = import_legacy_cpu_history(
        context(),
        scope1,
        &host,
        &mut sequence1,
        vec![Arc::clone(&refused_pixels)],
    )
    .err()
    .unwrap();
    assert_eq!(rejected.error, HistoryError::HostBudgetExceeded);
    assert!(Arc::ptr_eq(&rejected.frames[0], &refused_pixels));
    assert_eq!(sequence1.last_issued(), Some(21));
    assert_eq!(host.snapshot(), before);
    drop(rejected);
    let mut exhausted = CpuHistoryEntrySequence::new(scope1, Some(u64::MAX - 1));
    let rejected = import_legacy_cpu_history(
        context(),
        scope1,
        &host,
        &mut exhausted,
        vec![Arc::clone(&refused_pixels), Arc::clone(&pixels)],
    )
    .err()
    .unwrap();
    assert_eq!(rejected.error, HistoryError::SequenceExhausted);
    assert_eq!(rejected.frames.len(), 2);
    assert!(Arc::ptr_eq(&rejected.frames[0], &refused_pixels));
    assert!(Arc::ptr_eq(&rejected.frames[1], &pixels));
    assert_eq!(exhausted.last_issued(), Some(u64::MAX - 1));
    assert_eq!(host.snapshot(), before);
    drop(first);
    drop(queue);
    assert_eq!(
        host.snapshot().unique_bytes,
        8,
        "selected whole owner survives"
    );
    drop(selected);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().charges, 0);
}
