//! Fake resource/recipe tests: these establish policy, not GPU byte equality.
use super::*;
use crate::native_gpu_contract::{FrozenChainClocks, GpuSubmissionId, MaterializationDescriptor};
use protocol::VideoRenderInputId;
use std::sync::atomic::{AtomicUsize, Ordering};

struct FakeRecipe {
    bytes: Vec<u8>,
    clocks: FrozenChainClocks,
    drops: Arc<AtomicUsize>,
}
impl Drop for FakeRecipe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
struct FakeLease {
    identity: LayerResultIdentity,
    allocation: GpuAllocationId,
    drops: Arc<AtomicUsize>,
}
impl Drop for FakeLease {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl HistoryResident for FakeLease {
    fn identity(&self) -> &LayerResultIdentity {
        &self.identity
    }
    fn allocation(&self) -> GpuAllocationId {
        self.allocation
    }
}
type History = NativeHistory<FakeRecipe, FakeLease>;
fn counter() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}
fn context() -> GpuContextId {
    GpuContextId {
        owner_id: 1,
        device_generation: 2,
    }
}
fn input() -> VideoRenderInputKey {
    VideoRenderInputKey {
        project_render_epoch: 3,
        input_id: VideoRenderInputId(4),
    }
}
fn id(seq: u64) -> HistoryEntryId {
    HistoryEntryId {
        input: input(),
        queue_epoch: 5,
        sequence: seq,
    }
}
fn frame() -> NativeFrameId {
    NativeFrameId {
        output_id: 6,
        project_epoch: 3,
        request_sequence: 7,
        geometry_generation: 8,
    }
}
fn clocks(seq: u64) -> FrozenChainClocks {
    FrozenChainClocks {
        frame_index: seq as u32,
        time_seconds_bits: (seq as f32).to_bits(),
        time_delta_seconds_bits: (-0.0f32).to_bits(),
        date_bits: [0x7fc01234, 1, 2, seq as u32],
    }
}
fn identity(seq: u64, pts: u64) -> LayerResultIdentity {
    LayerResultIdentity {
        context: context(),
        frame: frame(),
        entry: id(seq),
        layer_id: 9,
        source_generation: 10,
        content_sequence: seq,
        pts_ms: pts,
        duration_ms: 33,
        width: 1,
        height: 1,
        format: VideoPixelFormat::Rgba8,
        origin: LayerOrigin::Executed(clocks(seq)),
    }
}
fn pixels(i: LayerResultIdentity) -> VideoFrame {
    VideoFrame {
        layer_id: i.layer_id,
        width: i.width,
        height: i.height,
        pts_ms: i.pts_ms,
        duration_ms: i.duration_ms,
        format: i.format,
        data: vec![i.content_sequence as u8, 17, 23, 255],
    }
}
fn lease(i: LayerResultIdentity, drops: &Arc<AtomicUsize>) -> FakeLease {
    FakeLease {
        identity: i,
        allocation: GpuAllocationId {
            context: i.context,
            sequence: i.entry.sequence,
        },
        drops: Arc::clone(drops),
    }
}
fn recipe(i: LayerResultIdentity, drops: &Arc<AtomicUsize>) -> FakeRecipe {
    FakeRecipe {
        bytes: pixels(i).data,
        clocks: clocks(i.entry.sequence),
        drops: Arc::clone(drops),
    }
}
fn entry(
    i: LayerResultIdentity,
    resident: bool,
    drops: &Arc<AtomicUsize>,
    recipes: &Arc<AtomicUsize>,
) -> Entry<FakeRecipe, FakeLease> {
    Entry {
        identity: i,
        resource_identity: i,
        backing: HistoryBacking::Replay(recipe(i, recipes)),
        resident: resident.then(|| Arc::new(lease(i, drops))),
    }
}
fn history() -> History {
    History::new(context(), frame(), input(), 9, 5, 10, (1, 1)).unwrap()
}
fn batch(
    entries: Vec<Entry<FakeRecipe, FakeLease>>,
    phase: HistoryPhaseOutcome,
) -> ValidatedHistoryEntries<FakeRecipe, FakeLease> {
    ValidatedHistoryEntries {
        frame: frame(),
        phase,
        entries,
    }
}
fn commit(
    h: &mut History,
    entries: Vec<Entry<FakeRecipe, FakeLease>>,
    target: u64,
) -> HistoryCommit<FakeLease> {
    let meta: Vec<_> = entries.iter().map(Entry::meta).collect();
    let plan = h.plan(&meta, target).unwrap();
    match h.commit_validated(
        plan,
        batch(
            entries,
            HistoryPhaseOutcome::IsfValid {
                composition_valid: true,
            },
        ),
    ) {
        Ok(v) => v,
        Err(e) => panic!("commit error {:?}", e.error),
    }
}
fn proof(i: LayerResultIdentity) -> MaterializationProof {
    MaterializationProof::for_test(MaterializationDescriptor {
        layer: i,
        allocation: GpuAllocationId {
            context: i.context,
            sequence: i.entry.sequence,
        },
        completed_submission: GpuSubmissionId {
            context: i.context,
            sequence: 99,
        },
    })
}
fn exact(i: LayerResultIdentity) -> ExactCpuLayer {
    ExactCpuLayer::for_test(pixels(i), proof(i))
}
#[test]
fn provisional_selection_is_not_public_and_predecessor_beats_new_decode() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    commit(&mut h, vec![entry(identity(1, 0), true, &d, &r)], 0);
    let incoming = entry(identity(2, 33), true, &d, &r);
    let plan = h.plan(&[incoming.meta()], 16).unwrap();
    let s = h.select_materialization_requests(&plan).unwrap();
    assert!(!s.cold_materialization_required);
    match s.selected.unwrap() {
        HistoryMaterializationRequest::Resident(l) => assert_eq!(l.identity.entry, id(1)),
        _ => panic!(),
    }
    assert_eq!(h.metadata().len(), 1);
    assert_eq!(h.metadata()[0].id, id(1));
    let s = h
        .select_materialization_requests(&h.plan(&[incoming.meta()], 40).unwrap())
        .unwrap();
    match s.selected.unwrap() {
        HistoryMaterializationRequest::Incoming { entry } => assert_eq!(entry, id(2)),
        _ => panic!(),
    }
    assert_eq!(h.metadata().len(), 1);
}
#[test]
fn stale_revision_sequence_epoch_and_foreign_base_are_rejected_with_owners() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    let e = entry(identity(1, 0), true, &d, &r);
    let stale = h.plan(&[e.meta()], 0).unwrap();
    commit(&mut h, vec![entry(identity(2, 33), true, &d, &r)], 0);
    let rejected = match h.commit_validated(
        stale,
        batch(
            vec![e],
            HistoryPhaseOutcome::IsfValid {
                composition_valid: true,
            },
        ),
    ) {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(rejected.error, HistoryError::StateMismatch);
    assert_eq!(rejected.plan.incoming()[0].id, id(1));
    assert_eq!(rejected.entries.entries.len(), 1);
    assert_eq!(d.load(Ordering::SeqCst), 0);
    assert!(matches!(
        h.plan(
            &[HistoryMeta {
                id: id(2),
                pts_ms: 99,
                duration_ms: 1
            }],
            0
        ),
        Err(HistoryError::StateMismatch)
    ));
    let mut foreign = id(3);
    foreign.queue_epoch += 1;
    assert!(matches!(
        h.plan(
            &[HistoryMeta {
                id: foreign,
                pts_ms: 99,
                duration_ms: 1
            }],
            0
        ),
        Err(HistoryError::IdentityMismatch)
    ));
    let other = history().plan(&[], 0).unwrap().bind_revision(h.revision);
    assert!(matches!(
        h.select_materialization_requests(&other),
        Err(HistoryError::StateMismatch)
    ));
}
#[test]
fn isf_valid_compose_failure_commits_history_but_isf_failure_and_cancel_never_do() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    let e = entry(identity(1, 0), true, &d, &r);
    let plan = h.plan(&[e.meta()], 0).unwrap();
    let receipt = match h.commit_validated(
        plan,
        batch(
            vec![e],
            HistoryPhaseOutcome::IsfValid {
                composition_valid: false,
            },
        ),
    ) {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert!(!receipt.output_phases_valid);
    assert_eq!(h.metadata()[0].id, id(1));
    for (phase, error) in [
        (HistoryPhaseOutcome::IsfFailed, HistoryError::IsfFailed),
        (HistoryPhaseOutcome::Cancelled, HistoryError::Cancelled),
    ] {
        let e = entry(identity(2, 33), true, &d, &r);
        let plan = h.plan(&[e.meta()], 33).unwrap();
        let rejected = match h.commit_validated(plan, batch(vec![e], phase)) {
            Err(e) => e,
            Ok(_) => panic!(),
        };
        assert_eq!(rejected.error, error);
        assert_eq!(h.metadata().len(), 1);
        assert_eq!(rejected.entries.entries[0].identity.entry, id(2));
    }
}
#[test]
fn pin_priority_resident_two_and_evicted_physical_owner_survives_until_retirement() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    commit(
        &mut h,
        vec![
            entry(identity(1, 0), true, &d, &r),
            entry(identity(2, 33), true, &d, &r),
        ],
        0,
    );
    let pin = h.pin(id(1)).unwrap();
    assert_eq!(pin.entry, id(1));
    assert_eq!(pin.resource().allocation.sequence, 1);
    let receipt = commit(&mut h, vec![entry(identity(3, 66), true, &d, &r)], 66);
    assert_eq!(h.resident_count(), 2);
    assert_eq!(receipt.retired.len(), 1);
    assert_eq!(receipt.retired[0].entry, id(2));
    assert_eq!(receipt.retired[0].resource().allocation.sequence, 2);
    assert_eq!(d.load(Ordering::SeqCst), 0); // physical old2 + incoming1 = 3
    drop(receipt);
    assert_eq!(d.load(Ordering::SeqCst), 1);
    let receipt = commit(&mut h, vec![entry(identity(4, 0), true, &d, &r)], 0);
    assert!(receipt.retired.iter().any(|r| r.entry == id(1)));
    drop(receipt);
    assert_eq!(pin.resource().identity.entry, id(1));
    let before = d.load(Ordering::SeqCst);
    drop(pin);
    assert_eq!(d.load(Ordering::SeqCst), before + 1);
}
#[test]
fn replay_preserves_immutable_clock_uniform_date_and_requires_preflight_materialization() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    commit(
        &mut h,
        (1..=8)
            .map(|s| entry(identity(s, s * 33), true, &d, &r))
            .collect(),
        264,
    );
    assert_eq!(h.metadata().len(), 8);
    assert_eq!(h.resident_count(), 2);
    let plan = h.plan(&[], 33).unwrap();
    let s = h.select_materialization_requests(&plan).unwrap();
    assert!(s.cold_materialization_required);
    match s.selected.unwrap() {
        HistoryMaterializationRequest::Replay { entry, frozen } => {
            assert_eq!(entry, id(1));
            assert_eq!(frozen.clocks, clocks(1));
            assert_eq!(frozen.bytes, pixels(identity(1, 33)).data);
        }
        _ => panic!(),
    }
}
#[test]
fn late_and_foreign_materializations_return_affine_owners_and_valid_attach_invalidates_plan() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    commit(&mut h, vec![entry(identity(1, 0), false, &d, &r)], 0);
    let pending = h.plan(&[], 0).unwrap();
    let attached =
        match h.install_materialized(id(1), lease(identity(1, 0), &d), proof(identity(1, 0))) {
            Ok(v) => v,
            Err(_) => panic!(),
        };
    assert!(attached.retired.is_empty());
    assert_eq!(h.resident_count(), 1);
    assert!(matches!(
        h.select_materialization_requests(&pending),
        Err(HistoryError::StateMismatch)
    ));
    commit(&mut h, vec![entry(identity(2, 0), false, &d, &r)], 0);
    let rejected =
        match h.install_materialized(id(1), lease(identity(1, 0), &d), proof(identity(1, 0))) {
            Err(v) => v,
            Ok(_) => panic!(),
        };
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(rejected.resident.identity.entry, id(1));
    assert_eq!(rejected.proof.descriptor().layer.entry, id(1));
    let mut foreign = identity(2, 0);
    foreign.context.device_generation += 1;
    assert!(h
        .install_materialized(id(2), lease(foreign, &d), proof(foreign))
        .is_err());
    let mut bad = proof(identity(2, 0)).descriptor().to_owned();
    bad.allocation.sequence += 1;
    assert!(h
        .install_materialized(
            id(2),
            lease(identity(2, 0), &d),
            MaterializationProof::for_test(bad)
        )
        .is_err());
}
#[test]
fn all_eight_handover_is_incremental_no_partial_publish_and_import_preserves_cpu_arcs() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    commit(
        &mut h,
        (1..=8)
            .map(|s| entry(identity(s, s * 33), true, &d, &r))
            .collect(),
        264,
    );
    let mut handover = h.begin_cpu_handover();
    assert!(handover.take_retired().is_empty());
    for seq in 1..=8 {
        let request = handover.next_entry().unwrap();
        match request {
            CpuHistoryMaterializationRequest::ReplayToCpu { entry, frozen } => {
                assert_eq!(entry, id(seq));
                assert_eq!(frozen.clocks, clocks(seq));
            }
            CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, resident } => {
                assert_eq!(entry, id(seq));
                assert_eq!(resident.identity.entry, id(seq));
            }
        }
        let mut wrong = identity(seq, seq * 33);
        wrong.content_sequence += 100;
        let rejected = handover
            .accept_materialized(id(seq), exact(wrong))
            .err()
            .unwrap();
        assert_eq!(rejected.error, HistoryError::IdentityMismatch);
        assert_eq!(rejected.exact.frame().data[0], wrong.content_sequence as u8);
        assert_eq!(r.load(Ordering::SeqCst), (seq - 1) as usize);
        assert!(handover
            .accept_materialized(id(seq), exact(identity(seq, seq * 33)))
            .is_ok());
        assert_eq!(r.load(Ordering::SeqCst), seq as usize);
        drop(handover.take_retired());
        if seq < 8 {
            handover = match handover.finish() {
                Err(v) => v,
                Ok(_) => panic!("partial publish"),
            };
        }
    }
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert_eq!(snapshot.entries().count(), 8);
    let held: Vec<_> = snapshot.entries().map(|(_, f)| Arc::clone(f)).collect();
    let mut other = context();
    other.device_generation += 1;
    let imported = History::import_cpu_history(snapshot, other).unwrap();
    assert_eq!(imported.metadata().len(), 8);
    assert_eq!(imported.resident_count(), 0);
    let plan = imported.plan(&[], 33).unwrap();
    let selection = imported.select_materialization_requests(&plan).unwrap();
    assert!(selection.cold_materialization_required);
    match selection.selected.unwrap() {
        HistoryMaterializationRequest::UploadCpuPostFx { entry, frame } => {
            assert_eq!(entry, id(1));
            assert!(Arc::ptr_eq(frame, &held[0]));
        }
        _ => panic!(),
    }
    assert!(imported
        .entries
        .iter()
        .all(|e| e.identity.origin == LayerOrigin::ImportedCpu));
    let mut handover = imported.begin_cpu_handover();
    assert!(handover.next_entry().is_none());
    assert!(handover.take_retired().is_empty());
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert_eq!(snapshot.entries().count(), 8);
}
#[test]
fn mixed_cpu_backing_skips_readback_and_failure_keeps_unconverted_recipes() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    let i = identity(1, 0);
    let cpu = Arc::new(pixels(i));
    commit(
        &mut h,
        vec![
            Entry {
                identity: i,
                resource_identity: i,
                backing: HistoryBacking::Cpu(Arc::clone(&cpu)),
                resident: Some(Arc::new(lease(i, &d))),
            },
            entry(identity(2, 33), false, &d, &r),
        ],
        0,
    );
    let mut handover = h.begin_cpu_handover();
    assert_eq!(handover.take_retired().len(), 1);
    match handover.next_entry().unwrap() {
        CpuHistoryMaterializationRequest::ReplayToCpu { entry, .. } => assert_eq!(entry, id(2)),
        _ => panic!(),
    }
    let failed = match handover.finish() {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(r.load(Ordering::SeqCst), 0);
    assert_eq!(Arc::strong_count(&cpu), 2);
    drop(failed);
    assert_eq!(r.load(Ordering::SeqCst), 1);
}
#[test]
fn invalid_results_pixels_and_checked_counters_never_publish() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    assert!(History::new(context(), frame(), input(), 9, 5, 10, (0, 1)).is_err());
    let mut i = identity(1, 0);
    i.source_generation += 1;
    let e = entry(i, true, &d, &r);
    let plan = h.plan(&[e.meta()], 0).unwrap();
    assert!(h
        .commit_validated(
            plan,
            batch(
                vec![e],
                HistoryPhaseOutcome::IsfValid {
                    composition_valid: true
                }
            )
        )
        .is_err());
    let i = identity(1, 0);
    let mut bad = pixels(i);
    bad.data.clear();
    let e = Entry {
        identity: i,
        resource_identity: i,
        backing: HistoryBacking::Cpu(Arc::new(bad)),
        resident: None,
    };
    let plan = h.plan(&[e.meta()], 0).unwrap();
    let err = match h.commit_validated(
        plan,
        batch(
            vec![e],
            HistoryPhaseOutcome::IsfValid {
                composition_valid: true,
            },
        ),
    ) {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(err.error, HistoryError::InvalidPixels);
    assert!(h.metadata().is_empty());
    h.revision = u64::MAX;
    let e = entry(i, false, &d, &r);
    let plan = h.plan(&[e.meta()], 0).unwrap();
    let err = match h.commit_validated(
        plan,
        batch(
            vec![e],
            HistoryPhaseOutcome::IsfValid {
                composition_valid: true,
            },
        ),
    ) {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(err.error, HistoryError::SequenceExhausted);
    assert!(h.metadata().is_empty());
    h.revision = 0;
    h.last_sequence = Some(u64::MAX);
    assert!(matches!(
        h.plan(
            &[HistoryMeta {
                id: id(0),
                pts_ms: 0,
                duration_ms: 1
            }],
            0
        ),
        Err(HistoryError::SequenceExhausted)
    ));
}

#[test]
fn foreign_output_and_same_size_stale_geometry_are_rejected() {
    let (d, r) = (counter(), counter());
    for geometry in [false, true] {
        let mut h = history();
        let mut foreign = identity(1, 0);
        if geometry {
            foreign.frame.geometry_generation += 1;
        } else {
            foreign.frame.output_id += 1;
        }
        let e = entry(foreign, true, &d, &r);
        let plan = h.plan(&[e.meta()], 0).unwrap();
        let mut incoming = batch(
            vec![e],
            HistoryPhaseOutcome::IsfValid {
                composition_valid: true,
            },
        );
        incoming.frame = foreign.frame;
        let rejected = match h.commit_validated(plan, incoming) {
            Err(e) => e,
            Ok(_) => panic!(),
        };
        assert_eq!(rejected.error, HistoryError::IdentityMismatch);
        assert_eq!(rejected.entries.entries[0].identity, foreign);
        assert!(h.metadata().is_empty());
    }
}
#[test]
fn queue_normalizes_layer_metadata_without_cloning_pixels_or_rewriting_gpu_proofs() {
    let (d, r) = (counter(), counter());
    let mut h = history();
    let mut original = identity(1, 0);
    original.layer_id = 999;
    commit(&mut h, vec![entry(original, true, &d, &r)], 0);
    let plan = h.plan(&[], 0).unwrap();
    let selected = h.select_materialization_requests(&plan).unwrap();
    assert_eq!(selected.layer_id, 9);
    assert_eq!(selected.selected_identity.unwrap().layer_id, 9);
    match selected.selected.unwrap() {
        HistoryMaterializationRequest::Resident(lease) => assert_eq!(lease.identity(), &original),
        _ => panic!(),
    }
    let mut handover = h.begin_cpu_handover();
    let owned = exact(original);
    let ptr = owned.frame().data.as_ptr();
    assert!(handover.accept_materialized(id(1), owned).is_ok());
    drop(handover.take_retired());
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    let (i, f) = snapshot.entries().next().unwrap();
    assert_eq!(i.layer_id, 9);
    assert_eq!(f.layer_id, 9);
    assert_eq!(f.data.as_ptr(), ptr);
    let mut h = History::import_cpu_history(snapshot, context()).unwrap();
    let mut original = identity(2, 33);
    original.layer_id = 888;
    let shared = Arc::new(pixels(original));
    let ptr = shared.data.as_ptr();
    commit(
        &mut h,
        vec![Entry {
            identity: original,
            resource_identity: original,
            backing: HistoryBacking::Cpu(Arc::clone(&shared)),
            resident: None,
        }],
        33,
    );
    let selection = h
        .select_materialization_requests(&h.plan(&[], 33).unwrap())
        .unwrap();
    assert_eq!(selection.selected_identity.unwrap().layer_id, 9);
    match selection.selected.unwrap() {
        HistoryMaterializationRequest::UploadCpuPostFx { frame, .. } => {
            assert!(Arc::ptr_eq(frame, &shared));
            assert_eq!(frame.data.as_ptr(), ptr);
        }
        _ => panic!(),
    }
    let mut handover = h.begin_cpu_handover();
    assert!(handover.next_entry().is_none());
    drop(handover.take_retired());
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert_eq!(snapshot.entries().last().unwrap().0.layer_id, 9);
    assert!(History::import_cpu_history(snapshot, context()).is_ok());
}
