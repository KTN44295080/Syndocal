//! Fake resource/recipe/charge tests: these establish policy and ownership,
//! not GPU byte equality (see the adapter tests for real G owners).
use super::*;
use crate::native_gpu_contract::{FrozenChainClocks, GpuSubmissionId};
use protocol::VideoRenderInputId;
use std::sync::atomic::{AtomicUsize, Ordering};

struct FakeRecipe {
    input: Arc<VideoFrame>,
    clocks: FrozenChainClocks,
    drops: Arc<AtomicUsize>,
}
impl Drop for FakeRecipe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl HistoryHostFootprint for FakeRecipe {
    fn host_allocations(&self) -> Vec<HostAllocation> {
        vec![HostAllocation::shared_frame(&self.input, false)]
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
/// Stands in for G's AllocationCharge of a CPU readback/recovery Vec.
struct FakeCharge {
    allocation: GpuAllocationId,
    drops: Arc<AtomicUsize>,
}
impl Drop for FakeCharge {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
struct FakeCpu {
    frame: VideoFrame,
    descriptor: MaterializationDescriptor,
    charge: FakeCharge,
}
impl MaterializedCpuLayer for FakeCpu {
    type Charge = FakeCharge;
    fn frame(&self) -> &VideoFrame {
        &self.frame
    }
    fn descriptor(&self) -> &MaterializationDescriptor {
        &self.descriptor
    }
    fn into_charged_parts(self) -> (VideoFrame, MaterializationDescriptor, FakeCharge) {
        (self.frame, self.descriptor, self.charge)
    }
}
struct FakeResident {
    lease: FakeLease,
    descriptor: MaterializationDescriptor,
}
impl MaterializedResident for FakeResident {
    type Resident = FakeLease;
    fn descriptor(&self) -> MaterializationDescriptor {
        self.descriptor
    }
    fn resident(&self) -> &FakeLease {
        &self.lease
    }
    fn into_resident(self) -> FakeLease {
        self.lease
    }
}
type History = NativeHistory<FakeRecipe, FakeLease, FakeCharge>;
type Batch = ValidatedHistoryEntries<FakeRecipe, FakeLease, FakeCharge>;
type FakeEntry = Entry<FakeRecipe, FakeLease, FakeCharge>;

#[test]
fn cpu_snapshot_host_validation_rejects_foreign_ledger_without_consuming_backing() {
    let e = env();
    let identity = identity(1, 0);
    let pixels = Arc::new(pixels(identity));
    let pointer = pixels.data.as_ptr();
    let backing = e.cpu_backing(Arc::clone(&pixels), CpuBackingOrigin::Imported);
    let mut snapshot = CpuHistorySnapshot {
        scope: scope(),
        host: e.host.clone(),
        entries: vec![(identity, backing)],
    };
    assert_eq!(snapshot.validate_host_accounting(), Ok(()));
    snapshot.host = HistoryHostBudget::new(1 << 20);
    assert_eq!(
        snapshot.validate_host_accounting(),
        Err(HistoryError::IdentityMismatch)
    );
    assert_eq!(snapshot.entries[0].1.frame().data.as_ptr(), pointer);
    assert_eq!(
        e.host.snapshot().unique_bytes,
        pixels.data.capacity() as u64
    );
    drop(snapshot);
    assert_eq!(e.host.snapshot().unique_bytes, 0);
}

/// Drop counters for leases (d), recipes (r) and G charges (c), one host budget.
struct Env {
    d: Arc<AtomicUsize>,
    r: Arc<AtomicUsize>,
    c: Arc<AtomicUsize>,
    host: HistoryHostBudget,
}
fn env() -> Env {
    Env {
        d: Arc::new(AtomicUsize::new(0)),
        r: Arc::new(AtomicUsize::new(0)),
        c: Arc::new(AtomicUsize::new(0)),
        host: HistoryHostBudget::new(1 << 20),
    }
}
fn n(counter: &Arc<AtomicUsize>) -> usize {
    counter.load(Ordering::SeqCst)
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
fn scope() -> HistoryScope {
    HistoryScope {
        frame_scope: frame(),
        input: input(),
        layer_id: 9,
        queue_epoch: 5,
        source_generation: 10,
        extent: (1, 1),
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
fn descriptor(i: LayerResultIdentity) -> MaterializationDescriptor {
    MaterializationDescriptor {
        layer: i,
        allocation: GpuAllocationId {
            context: i.context,
            sequence: i.entry.sequence,
        },
        completed_submission: GpuSubmissionId {
            context: i.context,
            sequence: 99,
        },
    }
}
impl Env {
    fn history(&self) -> History {
        History::new(context(), scope(), self.host.clone()).unwrap()
    }
    fn lease(&self, i: LayerResultIdentity) -> FakeLease {
        FakeLease {
            identity: i,
            allocation: descriptor(i).allocation,
            drops: Arc::clone(&self.d),
        }
    }
    fn charge(&self, i: LayerResultIdentity) -> FakeCharge {
        FakeCharge {
            allocation: GpuAllocationId {
                context: i.context,
                sequence: 1000 + i.entry.sequence,
            },
            drops: Arc::clone(&self.c),
        }
    }
    /// A replay entry whose raw input is a fresh 4-byte host allocation.
    fn entry(&self, i: LayerResultIdentity, resident: bool) -> FakeEntry {
        self.entry_with_input(i, resident, Arc::new(pixels(i)))
    }
    fn entry_with_input(
        &self,
        i: LayerResultIdentity,
        resident: bool,
        input: Arc<VideoFrame>,
    ) -> FakeEntry {
        let recipe = FakeRecipe {
            input,
            clocks: clocks(i.entry.sequence),
            drops: Arc::clone(&self.r),
        };
        let replay_host = self.host.charge_all(recipe.host_allocations()).unwrap();
        Entry {
            identity: i,
            resource_identity: i,
            backing: HistoryBacking::Replay(recipe),
            replay_host,
            resident: resident.then(|| Arc::new(self.lease(i))),
        }
    }
    fn cpu_backing(
        &self,
        frame: Arc<VideoFrame>,
        origin: CpuBackingOrigin<FakeCharge>,
    ) -> Arc<CpuHistoryBacking<FakeCharge>> {
        let host = self
            .host
            .charge_all(vec![HostAllocation::shared_frame(&frame, false)])
            .unwrap()
            .pop()
            .unwrap();
        Arc::new(CpuHistoryBacking {
            frame,
            origin,
            host,
        })
    }
    fn cpu_entry(
        &self,
        i: LayerResultIdentity,
        backing: Arc<CpuHistoryBacking<FakeCharge>>,
    ) -> FakeEntry {
        Entry {
            identity: i,
            resource_identity: i,
            backing: HistoryBacking::Cpu(backing),
            replay_host: Vec::new(),
            resident: None,
        }
    }
    fn exact(&self, i: LayerResultIdentity) -> FakeCpu {
        FakeCpu {
            frame: pixels(i),
            descriptor: descriptor(i),
            charge: self.charge(i),
        }
    }
    fn resident(&self, i: LayerResultIdentity) -> FakeResident {
        FakeResident {
            lease: self.lease(i),
            descriptor: descriptor(i),
        }
    }
}
fn batch(entries: Vec<FakeEntry>, phase: HistoryPhaseOutcome) -> Batch {
    ValidatedHistoryEntries {
        frame: frame(),
        phase,
        entries,
    }
}
const VALID: HistoryPhaseOutcome = HistoryPhaseOutcome::IsfValid {
    composition_valid: true,
};
fn commit(h: &mut History, entries: Vec<FakeEntry>, target: u64) -> HistoryCommit<FakeLease> {
    let meta: Vec<_> = entries.iter().map(Entry::meta).collect();
    let plan = h.plan(&meta, target).unwrap();
    match h.commit_validated(plan, batch(entries, VALID)) {
        Ok(v) => v,
        Err(e) => panic!("commit error {:?}", e.error),
    }
}
fn eight(e: &Env, h: &mut History) {
    commit(
        h,
        (1..=8)
            .map(|s| e.entry(identity(s, s * 33), true))
            .collect(),
        264,
    );
}
#[test]
fn provisional_selection_is_not_public_and_predecessor_beats_new_decode() {
    let e = env();
    let mut h = e.history();
    commit(&mut h, vec![e.entry(identity(1, 0), true)], 0);
    let incoming = e.entry(identity(2, 33), true);
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
    let e = env();
    let mut h = e.history();
    let first = e.entry(identity(1, 0), true);
    let stale = h.plan(&[first.meta()], 0).unwrap();
    commit(&mut h, vec![e.entry(identity(2, 33), true)], 0);
    let rejected = match h.commit_validated(stale, batch(vec![first], VALID)) {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(rejected.error, HistoryError::StateMismatch);
    assert_eq!(rejected.plan.incoming()[0].id, id(1));
    assert_eq!(rejected.entries.entries.len(), 1);
    assert_eq!(n(&e.d), 0);
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
    let other = e.history().plan(&[], 0).unwrap().bind_revision(h.revision);
    assert!(matches!(
        h.select_materialization_requests(&other),
        Err(HistoryError::StateMismatch)
    ));
}
#[test]
fn isf_valid_compose_failure_commits_history_but_isf_failure_and_cancel_never_do() {
    let e = env();
    let mut h = e.history();
    let first = e.entry(identity(1, 0), true);
    let plan = h.plan(&[first.meta()], 0).unwrap();
    let mut entries = batch(vec![first], VALID);
    entries.record_composition(false).unwrap();
    assert_eq!(
        entries.phase(),
        HistoryPhaseOutcome::IsfValid {
            composition_valid: false
        }
    );
    let receipt = match h.commit_validated(plan, entries) {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert!(!receipt.output_phases_valid);
    assert_eq!(h.metadata()[0].id, id(1));
    for (phase, error) in [
        (HistoryPhaseOutcome::IsfFailed, HistoryError::IsfFailed),
        (HistoryPhaseOutcome::Cancelled, HistoryError::Cancelled),
    ] {
        let next = e.entry(identity(2, 33), true);
        let plan = h.plan(&[next.meta()], 33).unwrap();
        let mut entries = batch(vec![next], phase);
        assert_eq!(
            entries.record_composition(true),
            Err(HistoryError::StateMismatch)
        );
        let rejected = match h.commit_validated(plan, entries) {
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
    let e = env();
    let mut h = e.history();
    commit(
        &mut h,
        vec![
            e.entry(identity(1, 0), true),
            e.entry(identity(2, 33), true),
        ],
        0,
    );
    let pin = h.pin(id(1)).unwrap();
    assert_eq!(pin.entry, id(1));
    assert_eq!(pin.resource().allocation.sequence, 1);
    let receipt = commit(&mut h, vec![e.entry(identity(3, 66), true)], 66);
    assert_eq!(h.resident_count(), 2);
    assert_eq!(receipt.retired.len(), 1);
    assert_eq!(receipt.retired[0].entry, id(2));
    assert_eq!(receipt.retired[0].resource().allocation.sequence, 2);
    assert_eq!(n(&e.d), 0); // physical old2 + incoming1 = 3
    drop(receipt);
    assert_eq!(n(&e.d), 1);
    let receipt = commit(&mut h, vec![e.entry(identity(4, 0), true)], 0);
    assert!(receipt.retired.iter().any(|r| r.entry == id(1)));
    drop(receipt);
    assert_eq!(pin.resource().identity.entry, id(1));
    let before = n(&e.d);
    drop(pin);
    assert_eq!(n(&e.d), before + 1);
}
#[test]
fn replay_preserves_immutable_clock_uniform_date_and_requires_preflight_materialization() {
    let e = env();
    let mut h = e.history();
    eight(&e, &mut h);
    assert_eq!(h.metadata().len(), 8);
    assert_eq!(h.resident_count(), 2);
    let plan = h.plan(&[], 33).unwrap();
    let s = h.select_materialization_requests(&plan).unwrap();
    assert!(s.cold_materialization_required);
    match s.selected.unwrap() {
        HistoryMaterializationRequest::Replay { entry, frozen } => {
            assert_eq!(entry, id(1));
            assert_eq!(frozen.clocks, clocks(1));
            assert_eq!(frozen.input.data, pixels(identity(1, 33)).data);
        }
        _ => panic!(),
    }
}
#[test]
fn late_and_foreign_materializations_return_affine_owners_and_valid_attach_invalidates_plan() {
    let e = env();
    let mut h = e.history();
    commit(&mut h, vec![e.entry(identity(1, 0), false)], 0);
    let pending = h.plan(&[], 0).unwrap();
    let attached = match h.install_materialized(id(1), e.resident(identity(1, 0))) {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert!(attached.retired.is_empty());
    assert_eq!(h.resident_count(), 1);
    assert!(matches!(
        h.select_materialization_requests(&pending),
        Err(HistoryError::StateMismatch)
    ));
    commit(&mut h, vec![e.entry(identity(2, 0), false)], 0);
    let rejected = match h.install_materialized(id(1), e.resident(identity(1, 0))) {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(rejected.materialized.lease.identity.entry, id(1));
    assert_eq!(rejected.materialized.descriptor.layer.entry, id(1));
    let mut foreign = identity(2, 0);
    foreign.context.device_generation += 1;
    let foreign_rejected = h
        .install_materialized(id(2), e.resident(foreign))
        .err()
        .unwrap();
    assert_eq!(foreign_rejected.materialized.lease.identity, foreign);
    let mut bad = e.resident(identity(2, 0));
    bad.descriptor.allocation.sequence += 1;
    let bad_rejected = h.install_materialized(id(2), bad).err().unwrap();
    assert_eq!(bad_rejected.error, HistoryError::IdentityMismatch);
    assert_eq!(
        n(&e.d),
        1,
        "only the evicted resident; rejected owners are returned"
    );
}
#[test]
fn all_eight_handover_is_incremental_charged_no_partial_publish_and_import_moves_owners() {
    let e = env();
    let mut h = e.history();
    eight(&e, &mut h);
    let inputs = e.host.snapshot();
    assert_eq!(inputs.unique_bytes, 32);
    assert_eq!(inputs.managed_alias_bytes, 0);
    let mut handover = h.begin_cpu_handover();
    assert!(handover.take_retired().is_empty());
    assert_eq!((handover.len(), handover.pending()), (8, 8));
    for seq in 1..=8 {
        match handover.next_entry().unwrap() {
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
            .accept_materialized(id(seq), e.exact(wrong))
            .err()
            .unwrap();
        assert_eq!(rejected.error, HistoryError::IdentityMismatch);
        // The complete owner comes back unsplit: pixels, proof data, charge.
        assert_eq!(
            rejected.materialized.frame.data[0],
            wrong.content_sequence as u8
        );
        assert_eq!(rejected.materialized.descriptor.layer, wrong);
        drop(rejected);
        assert_eq!(
            n(&e.c),
            seq as usize,
            "only the rejected test charge dropped"
        );
        assert_eq!(n(&e.r), (seq - 1) as usize);
        assert!(handover
            .accept_materialized(id(seq), e.exact(identity(seq, seq * 33)))
            .is_ok());
        // One raw recipe and its host charge released per converted entry.
        assert_eq!(n(&e.r), seq as usize);
        let snapshot = e.host.snapshot();
        assert_eq!(snapshot.unique_bytes, 32, "4B recipe input -> 4B CPU");
        assert_eq!(snapshot.managed_alias_bytes, 4 * seq);
        assert_eq!(snapshot.reserved_bytes, 0);
        assert_eq!(handover.pending(), (8 - seq) as usize);
        drop(handover.take_retired());
        if seq < 8 {
            handover = match handover.finish() {
                Err(v) => v,
                Ok(_) => panic!("partial publish"),
            };
        }
    }
    assert!(
        e.host.snapshot().high_water_bytes <= 36,
        "never a full double"
    );
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    assert_eq!(snapshot.entries().count(), 8);
    assert_eq!(n(&e.c), 8, "accepted charges live with their pixels");
    for (i, backing) in snapshot.entries() {
        match backing.origin() {
            CpuBackingOrigin::Materialized { descriptor, charge } => {
                assert_eq!(descriptor.layer.entry, i.entry);
                assert_eq!(charge.allocation.sequence, 1000 + i.entry.sequence);
            }
            _ => panic!("materialized owner lost its proof/charge"),
        }
    }
    let held: Vec<_> = snapshot.entries().map(|(_, b)| Arc::clone(b)).collect();
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
            assert!(std::ptr::eq(frame, held[0].frame()));
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
    // Existing CPU backing is moved, never re-read or re-charged.
    for ((_, backing), held) in snapshot.entries().zip(&held) {
        assert!(Arc::ptr_eq(backing, held));
    }
    assert_eq!(n(&e.c), 8);
    drop(held);
    let owners = snapshot.into_entries();
    assert_eq!(owners.len(), 8);
    assert_eq!(n(&e.c), 8, "the canonical queue keeps whole owners");
    drop(owners);
    assert_eq!(n(&e.c), 16);
    assert_eq!(
        e.host.snapshot(),
        HistoryHostSnapshot {
            limit: 1 << 20,
            high_water_bytes: e.host.snapshot().high_water_bytes,
            ..HistoryHostSnapshot::default()
        }
    );
}
#[test]
fn cancelled_or_faulted_handover_keeps_mixed_owners_and_resumes_exactly() {
    let e = env();
    let mut h = e.history();
    eight(&e, &mut h);
    let mut handover = h.begin_cpu_handover();
    for seq in 1..=3 {
        handover
            .accept_materialized(id(seq), e.exact(identity(seq, seq * 33)))
            .ok()
            .unwrap();
    }
    // A cancelled/faulted materialization of entry 4 produced no owner: the
    // barrier is not complete and publishes nothing.
    assert_eq!(handover.pending(), 5);
    let mut handover = match handover.finish() {
        Err(v) => v,
        Ok(_) => panic!("partial history disguised as success"),
    };
    // Mixed ownership is intact: 3 CPU owners with charges, 5 recipes, and
    // the GPU residents not yet read back.
    assert_eq!((n(&e.r), n(&e.c)), (3, 0));
    assert_eq!(n(&e.d), 6, "only the six residents trimmed at commit");
    assert_eq!(e.host.snapshot().managed_alias_bytes, 12);
    // Another lane's request or a foreign generation cannot be accepted.
    let mut foreign = identity(4, 132);
    foreign.context.device_generation += 1;
    let rejected = handover
        .accept_materialized(id(4), e.exact(foreign))
        .err()
        .unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    drop(rejected);
    // A budget refusal also keeps the materialized owner whole.
    let full = HistoryHostBudget::new(0);
    let mut tight = handover;
    tight.history.host = full.clone();
    let refused = tight
        .accept_materialized(id(4), e.exact(identity(4, 132)))
        .err()
        .unwrap();
    assert_eq!(refused.error, HistoryError::HostBudgetExceeded);
    assert_eq!(refused.materialized.descriptor.layer.entry, id(4));
    assert_eq!(full.snapshot().reserved_bytes, 0);
    drop(refused);
    tight.history.host = e.host.clone();
    let mut handover = tight;
    for seq in 4..=8 {
        handover
            .accept_materialized(id(seq), e.exact(identity(seq, seq * 33)))
            .ok()
            .unwrap();
    }
    drop(handover.take_retired());
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    let order: Vec<_> = snapshot.entries().map(|(i, _)| i.entry.sequence).collect();
    assert_eq!(order, (1..=8).collect::<Vec<_>>());
    assert_eq!(n(&e.r), 8);
    assert_eq!(n(&e.c), 2, "the two refused test owners only");
    drop(snapshot);
    assert_eq!(n(&e.c), 10);
    assert_eq!(e.host.snapshot().unique_bytes, 0);
}
#[test]
fn mixed_cpu_backing_skips_readback_and_failure_keeps_unconverted_recipes() {
    let e = env();
    let mut h = e.history();
    let i = identity(1, 0);
    let cpu = e.cpu_backing(
        Arc::new(pixels(i)),
        CpuBackingOrigin::Recovered {
            charge: e.charge(i),
        },
    );
    let mut resident = e.cpu_entry(i, Arc::clone(&cpu));
    resident.resident = Some(Arc::new(e.lease(i)));
    commit(&mut h, vec![resident, e.entry(identity(2, 33), false)], 0);
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
    assert_eq!(n(&e.r), 0);
    assert_eq!(Arc::strong_count(&cpu), 2);
    drop(failed);
    assert_eq!(n(&e.r), 1);
    assert_eq!(n(&e.c), 0, "the shared CPU owner keeps its charge");
    drop(cpu);
    assert_eq!(n(&e.c), 1);
}
#[test]
fn host_budget_managed_alias_drop_restores_imported_unmanaged_bytes() {
    let frame = Arc::new(pixels(identity(1, 0)));
    let bytes = frame.data.capacity() as u64;
    let host = HistoryHostBudget::new(bytes);
    let imported = host
        .charge_all(vec![HostAllocation::shared_frame(&frame, false)])
        .unwrap();
    let first = host
        .charge_all(vec![HostAllocation::shared_frame(&frame, true)])
        .unwrap();
    let second = host
        .charge_all(vec![HostAllocation::shared_frame(&frame, true)])
        .unwrap();
    assert_eq!(host.snapshot().unique_bytes, bytes);
    assert_eq!(host.snapshot().managed_alias_bytes, bytes);
    assert_eq!(host.snapshot().unmanaged_bytes(), 0);
    drop(first);
    assert_eq!(
        host.snapshot().managed_alias_bytes,
        bytes,
        "another managed alias is still live"
    );
    drop(second);
    assert_eq!(
        host.snapshot().unique_bytes,
        bytes,
        "the original imported backing survives"
    );
    assert_eq!(host.snapshot().managed_alias_bytes, 0);
    assert_eq!(host.snapshot().unmanaged_bytes(), bytes);
    assert_eq!(host.snapshot().charges, 1);
    drop(imported);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().charges, 0);
}

#[test]
fn host_budget_counts_aliases_once_rejects_overflow_atomically_and_releases_on_last_owner() {
    let e = env();
    let one_layer = HistoryHostBudget::planning_limit(1, (2560, 1600)).unwrap();
    assert_eq!(one_layer, 9 * 16_384_000);
    assert_eq!(
        HistoryHostBudget::planning_limit(3, (2560, 1600)),
        Some(3 * 9 * 16_384_000)
    );
    assert_eq!(
        HistoryHostBudget::planning_limit(2, (u32::MAX, u32::MAX)),
        None
    );
    // Two layers (two histories) whose recipes share one decoded input.
    let shared = Arc::new(pixels(identity(1, 0)));
    let mut a = e.history();
    let other_scope = HistoryScope {
        layer_id: 10,
        ..scope()
    };
    let mut b = History::new(context(), other_scope, e.host.clone()).unwrap();
    commit(
        &mut a,
        vec![e.entry_with_input(identity(1, 0), false, Arc::clone(&shared))],
        0,
    );
    let mut on_b = identity(1, 0);
    on_b.layer_id = 10;
    commit(
        &mut b,
        vec![e.entry_with_input(on_b, false, Arc::clone(&shared))],
        0,
    );
    let snapshot = e.host.snapshot();
    assert_eq!(
        (
            snapshot.unique_bytes,
            snapshot.allocations,
            snapshot.charges
        ),
        (4, 1, 2)
    );
    // Atomic refusal: neither of two fresh allocations is admitted.
    let tight = HistoryHostBudget::new(6);
    let first = Arc::new(pixels(identity(2, 0)));
    let second = Arc::new(pixels(identity(3, 0)));
    assert!(matches!(
        tight.charge_all(vec![
            HostAllocation::shared_frame(&first, false),
            HostAllocation::shared_frame(&second, false),
        ]),
        Err(HistoryError::HostBudgetExceeded)
    ));
    assert_eq!(tight.snapshot().unique_bytes, 0);
    let kept = tight
        .charge_all(vec![
            HostAllocation::shared_frame(&first, false),
            HostAllocation::shared_frame(&first, true),
        ])
        .unwrap();
    assert_eq!(tight.snapshot().unique_bytes, 4);
    assert_eq!(tight.snapshot().managed_alias_bytes, 4);
    drop(kept);
    assert_eq!(tight.snapshot().unique_bytes, 0);
    assert_eq!(tight.snapshot().managed_alias_bytes, 0);
    // Releasing one alias keeps the bytes; the last owner releases them.
    drop(a);
    assert_eq!(e.host.snapshot().unique_bytes, 4);
    drop(b);
    assert_eq!(e.host.snapshot().unique_bytes, 0);
    assert_eq!(n(&e.r), 2);
    // A history only accepts charges from its own session ledger.
    let mut h = e.history();
    let foreign = HistoryHostBudget::new(1 << 20);
    let recipe = FakeRecipe {
        input: Arc::new(pixels(identity(4, 0))),
        clocks: clocks(4),
        drops: Arc::clone(&e.r),
    };
    let entry = Entry {
        identity: identity(4, 0),
        resource_identity: identity(4, 0),
        replay_host: foreign.charge_all(recipe.host_allocations()).unwrap(),
        backing: HistoryBacking::Replay(recipe),
        resident: None,
    };
    let plan = h.plan(&[entry.meta()], 0).unwrap();
    let rejected = h
        .commit_validated(plan, batch(vec![entry], VALID))
        .err()
        .unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert!(h.metadata().is_empty());
}
#[test]
fn recovered_cpu_phase_commits_cpu_backing_only_and_never_a_recipe() {
    let e = env();
    let mut h = e.history();
    let i = identity(1, 0);
    let recovered = e.cpu_entry(
        i,
        e.cpu_backing(
            Arc::new(pixels(i)),
            CpuBackingOrigin::Recovered {
                charge: e.charge(i),
            },
        ),
    );
    let plan = h.plan(&[recovered.meta()], 0).unwrap();
    let entries = batch(vec![recovered], HistoryPhaseOutcome::RecoveredCpu);
    assert_eq!(entries.frame(), frame());
    assert_eq!(entries.metadata()[0].id, id(1));
    let receipt = h.commit_validated(plan, entries).ok().unwrap();
    assert!(!receipt.output_phases_valid, "GPU composition never valid");
    let plan = h.plan(&[], 0).unwrap();
    match h
        .select_materialization_requests(&plan)
        .unwrap()
        .selected
        .unwrap()
    {
        HistoryMaterializationRequest::UploadCpuPostFx { entry, frame } => {
            assert_eq!(entry, id(1));
            assert_eq!(frame.data, pixels(i).data);
        }
        _ => panic!(),
    }
    // A recipe, a resident or an imported/foreign CPU origin is refused.
    for case in 0..3 {
        let i = identity(2 + case, 33 * (case + 1));
        let mut entry = match case {
            0 => e.entry(i, false),
            1 => e.cpu_entry(
                i,
                e.cpu_backing(Arc::new(pixels(i)), CpuBackingOrigin::Imported),
            ),
            _ => e.cpu_entry(
                i,
                e.cpu_backing(
                    Arc::new(pixels(i)),
                    CpuBackingOrigin::Recovered {
                        charge: e.charge(i),
                    },
                ),
            ),
        };
        if case == 2 {
            entry.resident = Some(Arc::new(e.lease(i)));
        }
        let plan = h.plan(&[entry.meta()], 0).unwrap();
        let rejected = h
            .commit_validated(plan, batch(vec![entry], HistoryPhaseOutcome::RecoveredCpu))
            .err()
            .unwrap();
        assert_eq!(rejected.error, HistoryError::StateMismatch);
    }
    assert_eq!(h.metadata().len(), 1);
}
#[test]
fn canonical_cpu_queue_imports_every_survivor_as_charged_cpu_backing() {
    let e = env();
    let frames: Vec<_> = (1..=8)
        .map(|s| (id(s), Arc::new(pixels(identity(s, s * 33)))))
        .collect();
    let held: Vec<_> = frames.iter().map(|(_, f)| Arc::clone(f)).collect();
    let h = History::import_canonical_queue(context(), scope(), e.host.clone(), frames).unwrap();
    assert_eq!(h.scope(), scope());
    assert_eq!(h.metadata().len(), 8);
    assert_eq!(e.host.snapshot().unique_bytes, 32);
    assert_eq!(e.host.snapshot().managed_alias_bytes, 0);
    assert!(h.entries.iter().all(|e| matches!(
        &e.backing,
        HistoryBacking::Cpu(b) if matches!(b.origin(), CpuBackingOrigin::Imported)
    )));
    assert!(h.entries.iter().zip(&held).all(
        |(e, f)| matches!(&e.backing, HistoryBacking::Cpu(b) if std::ptr::eq(b.frame(), &**f))
    ));
    // The next native entry continues after the imported sequence.
    assert!(matches!(
        h.plan(
            &[HistoryMeta {
                id: id(8),
                pts_ms: 999,
                duration_ms: 1
            }],
            0
        ),
        Err(HistoryError::StateMismatch)
    ));
    // Unordered, over-capacity, foreign or over-budget input is refused whole.
    let mut unordered: Vec<_> = (1..=2)
        .map(|s| (id(s), Arc::new(pixels(identity(s, 66 - s * 33)))))
        .collect();
    assert_eq!(
        History::import_canonical_queue(context(), scope(), e.host.clone(), unordered.clone())
            .err(),
        Some(HistoryError::InvalidPolicy)
    );
    unordered.truncate(1);
    let mut wrong = id(1);
    wrong.queue_epoch += 1;
    unordered[0].0 = wrong;
    assert_eq!(
        History::import_canonical_queue(context(), scope(), e.host.clone(), unordered).err(),
        Some(HistoryError::IdentityMismatch)
    );
    let nine: Vec<_> = (1..=9)
        .map(|s| (id(s), Arc::new(pixels(identity(s, s * 33)))))
        .collect();
    assert_eq!(
        History::import_canonical_queue(context(), scope(), e.host.clone(), nine).err(),
        Some(HistoryError::InvalidPolicy)
    );
    let small = HistoryHostBudget::new(31);
    let frames: Vec<_> = held
        .iter()
        .enumerate()
        .map(|(s, f)| (id(s as u64 + 1), Arc::clone(f)))
        .collect();
    assert_eq!(
        History::import_canonical_queue(context(), scope(), small.clone(), frames).err(),
        Some(HistoryError::HostBudgetExceeded)
    );
    assert_eq!(small.snapshot().unique_bytes, 0);
    drop(h);
    assert_eq!(e.host.snapshot().unique_bytes, 0);
}
#[test]
fn invalid_results_pixels_and_checked_counters_never_publish() {
    let e = env();
    let mut h = e.history();
    let bad_scope = HistoryScope {
        extent: (0, 1),
        ..scope()
    };
    assert!(History::new(context(), bad_scope, e.host.clone()).is_err());
    let mut i = identity(1, 0);
    i.source_generation += 1;
    let entry = e.entry(i, true);
    let plan = h.plan(&[entry.meta()], 0).unwrap();
    assert!(h.commit_validated(plan, batch(vec![entry], VALID)).is_err());
    let i = identity(1, 0);
    let mut bad = pixels(i);
    bad.data.clear();
    let entry = e.cpu_entry(i, e.cpu_backing(Arc::new(bad), CpuBackingOrigin::Imported));
    let plan = h.plan(&[entry.meta()], 0).unwrap();
    let err = match h.commit_validated(plan, batch(vec![entry], VALID)) {
        Err(v) => v,
        Ok(_) => panic!(),
    };
    assert_eq!(err.error, HistoryError::InvalidPixels);
    assert!(h.metadata().is_empty());
    h.revision = u64::MAX;
    let entry = e.entry(i, false);
    let plan = h.plan(&[entry.meta()], 0).unwrap();
    let err = match h.commit_validated(plan, batch(vec![entry], VALID)) {
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
    let e = env();
    for geometry in [false, true] {
        let mut h = e.history();
        let mut foreign = identity(1, 0);
        if geometry {
            foreign.frame.geometry_generation += 1;
        } else {
            foreign.frame.output_id += 1;
        }
        let entry = e.entry(foreign, true);
        let plan = h.plan(&[entry.meta()], 0).unwrap();
        let mut incoming = batch(vec![entry], VALID);
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
    let e = env();
    let mut h = e.history();
    let mut original = identity(1, 0);
    original.layer_id = 999;
    commit(&mut h, vec![e.entry(original, true)], 0);
    let plan = h.plan(&[], 0).unwrap();
    let selected = h.select_materialization_requests(&plan).unwrap();
    assert_eq!(selected.layer_id, 9);
    assert_eq!(selected.selected_identity.unwrap().layer_id, 9);
    match selected.selected.unwrap() {
        HistoryMaterializationRequest::Resident(lease) => assert_eq!(lease.identity(), &original),
        _ => panic!(),
    }
    let mut handover = h.begin_cpu_handover();
    let owned = e.exact(original);
    let ptr = owned.frame.data.as_ptr();
    assert!(handover.accept_materialized(id(1), owned).is_ok());
    drop(handover.take_retired());
    let snapshot = match handover.finish() {
        Ok(v) => v,
        Err(_) => panic!(),
    };
    let (i, b) = snapshot.entries().next().unwrap();
    assert_eq!(i.layer_id, 9);
    assert_eq!(b.frame().layer_id, 9);
    assert_eq!(b.frame().data.as_ptr(), ptr);
    assert_eq!(snapshot.scope(), scope());
    let mut h = History::import_cpu_history(snapshot, context()).unwrap();
    let mut original = identity(2, 33);
    original.layer_id = 888;
    let shared = e.cpu_backing(Arc::new(pixels(original)), CpuBackingOrigin::Imported);
    let ptr = shared.frame().data.as_ptr();
    commit(&mut h, vec![e.cpu_entry(original, Arc::clone(&shared))], 33);
    let selection = h
        .select_materialization_requests(&h.plan(&[], 33).unwrap())
        .unwrap();
    assert_eq!(selection.selected_identity.unwrap().layer_id, 9);
    match selection.selected.unwrap() {
        HistoryMaterializationRequest::UploadCpuPostFx { frame, .. } => {
            assert!(std::ptr::eq(frame, shared.frame()));
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
