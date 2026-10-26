//! Real G owners (headless shared context, real Batch3 ISF, real readback and
//! historical replay) through P's sealed history publication and charged CPU
//! handover. Pixel oracle: the identity chain returns its exact input.
use super::*;
use crate::isf_native_gpu::tests::{gpu, identity_jobs, request, stop, validated, Fixture};
use crate::isf_native_gpu::{
    await_transaction, CpuMaterializationRequest, NativeGpuMaterializer, PreparedNativeLayer,
};
use crate::isf_runtime::StackBatch;
use crate::native_gpu_budget::{ChargeKind, NativeAllocationPlan, NativeGpuBudget};
use crate::native_gpu_contract::{GpuContextId, HistoryEntryId, MaterializationFailure};
use crate::native_gpu_history::{
    CpuHistoryMaterializationRequest, HistoryMaterializationRequest, HistoryScope,
};
use protocol::{VideoRenderInputId, VideoRenderInputKey};
use std::collections::HashMap;

/// Frame `k` of the three fixture inputs: entry sequence k, PTS k*100 and a
/// per-frame marker byte, so every history entry has distinct exact pixels.
fn frame_jobs(k: u64, size: (u32, u32)) -> StackBatch<PreparedNativeLayer> {
    let mut jobs = identity_jobs(size.0, size.1);
    for job in jobs.iter_mut() {
        job.entry.sequence = k;
        let input = Arc::get_mut(&mut job.input).expect("fixture owns its input");
        input.pts_ms = k * 100;
        input.data[2] = k as u8;
    }
    jobs
}
fn scope(layer: u64, size: (u32, u32)) -> HistoryScope {
    HistoryScope {
        frame_scope: request(1),
        input: VideoRenderInputKey {
            project_render_epoch: 7,
            input_id: VideoRenderInputId(layer + 1000),
        },
        layer_id: layer,
        queue_epoch: 9,
        source_generation: 13,
        extent: size,
    }
}
fn histories(
    fixture: &Fixture,
    size: (u32, u32),
    host: &HistoryHostBudget,
) -> Vec<NativeGpuHistory> {
    (1..=3)
        .map(|layer| {
            NativeGpuHistory::new(
                fixture.owner.context().id(),
                scope(layer, size),
                host.clone(),
            )
            .unwrap()
        })
        .collect()
}
/// Exact input pixels by (layer, frame) and the summed input capacity.
type Oracle = HashMap<(u64, u64), Vec<u8>>;
fn run_frame(
    fixture: &mut Fixture,
    k: u64,
    size: (u32, u32),
    oracle: &mut Oracle,
) -> (Vec<ValidatedLayerResult>, IsfValidatedReceipt, u64) {
    let jobs = frame_jobs(k, size);
    let capacity = jobs.iter().map(|j| j.input.data.capacity() as u64).sum();
    for job in jobs.iter() {
        oracle.insert((job.input.layer_id, k), job.input.data.clone());
    }
    let (layers, receipt) = validated(fixture, k, jobs);
    (layers.into_iter().collect(), receipt, capacity)
}
/// Seal every layer with its own receipt, record a valid composition and
/// commit it into the history of its input.
fn publish(
    histories: &mut [NativeGpuHistory],
    layers: Vec<ValidatedLayerResult>,
    receipt: &IsfValidatedReceipt,
    host: &HistoryHostBudget,
) {
    for layer in layers {
        let mut sealed = match NativeGpuValidatedEntries::seal_isf_layer(receipt, layer, host) {
            Ok(sealed) => sealed,
            Err(failure) => panic!("seal {:?}", failure.error),
        };
        assert_eq!(sealed.frame(), receipt.phase().frame());
        sealed.record_composition(true).unwrap();
        let meta = sealed.metadata();
        let history = histories
            .iter_mut()
            .find(|h| h.scope().input == meta[0].id.input)
            .unwrap();
        let plan = history.plan(&meta, meta[0].pts_ms).unwrap();
        match history.commit_validated(plan, sealed) {
            Ok(commit) => assert!(commit.output_phases_valid),
            Err(failure) => panic!("commit {:?}", failure.error),
        }
    }
}
fn cpu_snapshot_bytes(fixture: &mut Fixture) -> u64 {
    fixture.owner.poll_retirement().unwrap();
    fixture
        .owner
        .budget_snapshot()
        .for_kind(ChargeKind::CpuSnapshot)
        .active_bytes
}

#[test]
fn real_isf_layers_seal_once_and_stale_forged_foreign_or_unbudgeted_owners_return_intact() {
    let mut fixture = gpu();
    let size = (5, 3);
    let host = HistoryHostBudget::new(1 << 20);
    let mut histories = histories(&fixture, size, &host);
    let mut oracle = Oracle::new();
    let (mut first, receipt1, cap1) = run_frame(&mut fixture, 1, size, &mut oracle);
    let (second, receipt2, cap2) = run_frame(&mut fixture, 2, size, &mut oracle);

    // Stale: a frame-1 layer cannot be sealed with frame 2's receipt.
    let stale = NativeGpuValidatedEntries::seal_isf_layer(&receipt2, first.remove(0), &host)
        .err()
        .unwrap();
    assert_eq!(stale.error, HistoryError::IdentityMismatch);
    assert_eq!(stale.outcome.identity.frame, request(1));
    assert_eq!(
        host.snapshot().charges,
        0,
        "nothing charged before the check"
    );
    // Forged: a rewritten claimed identity no longer matches lease/recipe.
    let mut forged = stale.outcome;
    forged.identity.content_sequence += 1;
    let rejected = NativeGpuValidatedEntries::seal_isf_layer(&receipt1, forged, &host)
        .err()
        .unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    let mut restored = rejected.outcome;
    restored.identity.content_sequence -= 1;
    // Unbudgeted: the host budget refuses before any entry exists.
    let none = HistoryHostBudget::new(0);
    let refused = NativeGpuValidatedEntries::seal_isf_layer(&receipt1, restored, &none)
        .err()
        .unwrap();
    assert_eq!(refused.error, HistoryError::HostBudgetExceeded);
    assert_eq!(none.snapshot().charges, 0);
    let restored = refused.outcome;
    assert_eq!(restored.identity, *restored.lease.identity());

    // Foreign generation: a sealed entry cannot enter another context.
    let sealed = NativeGpuValidatedEntries::seal_isf_layer(&receipt1, restored, &host)
        .ok()
        .unwrap();
    let mut foreign_context = fixture.owner.context().id();
    foreign_context.device_generation += 1;
    let mut foreign = NativeGpuHistory::new(foreign_context, scope(1, size), host.clone()).unwrap();
    let plan = foreign.plan(&sealed.metadata(), 0).unwrap();
    let rejected = foreign.commit_validated(plan, sealed).err().unwrap();
    assert_eq!(rejected.error, HistoryError::IdentityMismatch);
    assert!(foreign.metadata().is_empty());
    let sealed = rejected.entries;
    // ...and is still the intact sealed owner for its own history.
    let history = &mut histories[0];
    let plan = history.plan(&sealed.metadata(), 100).unwrap();
    history.commit_validated(plan, sealed).ok().unwrap();
    publish(&mut histories, first, &receipt1, &host);
    publish(&mut histories, second, &receipt2, &host);
    let charged = host.snapshot();
    assert_eq!(
        charged.unique_bytes,
        cap1 + cap2,
        "recipe inputs, once each"
    );
    assert_eq!(charged.managed_alias_bytes, 0);
    for history in &histories {
        assert_eq!(history.metadata().len(), 2);
        assert_eq!(history.resident_count(), 2);
    }

    // Frame 3 trims frame 1's resident; its exact recipe stays for replay.
    let (third, receipt3, cap3) = run_frame(&mut fixture, 3, size, &mut oracle);
    publish(&mut histories, third, &receipt3, &host);
    assert_eq!(host.snapshot().unique_bytes, cap1 + cap2 + cap3);
    let history = &mut histories[1];
    assert_eq!(history.resident_count(), 2);
    let entry = HistoryEntryId {
        input: history.scope().input,
        queue_epoch: 9,
        sequence: 1,
    };
    let plan = history.plan(&[], 100).unwrap();
    let selection = history.select_materialization_requests(&plan).unwrap();
    assert!(selection.cold_materialization_required);
    let materialized = match selection.selected.unwrap() {
        HistoryMaterializationRequest::Replay { entry: id, frozen } => {
            assert_eq!(id, entry);
            let current = request(100);
            let plan = fixture
                .runtime
                .replay_allocation_plan(current, frozen)
                .unwrap();
            let quote = fixture.owner.quote(&plan).unwrap();
            let reservation = fixture.owner.reserve(quote).unwrap();
            let mut txn = fixture.owner.begin_frame(current, reservation).unwrap();
            let mut candidate = fixture
                .runtime
                .encode_historical_replay(&mut txn, entry, frozen)
                .unwrap();
            let receipt = candidate.take_replay_receipt().unwrap();
            let pending = txn.finish();
            let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
            let phase = resolved.take_phase(receipt).unwrap();
            fixture
                .runtime
                .complete_historical_replay(candidate, phase, &mut fixture.owner)
                .unwrap()
        }
        _ => panic!("an evicted executed entry replays its frozen recipe"),
    };
    let change = history
        .install_materialized(entry, materialized)
        .ok()
        .unwrap();
    assert_eq!(change.retired.len(), 1, "resident cap stays two");
    drop(change);
    assert_eq!(history.resident_count(), 2);
    let pin = history.pin(entry).unwrap();
    let pixels = NativeGpuMaterializer::new(&mut fixture.runtime)
        .materialize_cpu(
            CpuMaterializationRequest::ValidatedGpu {
                entry,
                lease: pin.resource(),
            },
            &mut fixture.owner,
            &|| false,
        )
        .unwrap();
    assert_eq!(pixels.frame().data, oracle[&(2, 1)]);
    drop(pixels);
    drop(pin);
    drop(histories);
    drop(foreign);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().charges, 0);
    stop(&mut fixture.owner);
}

#[test]
fn real_full_eight_handover_moves_charged_cpu_owners_and_survives_cancel() {
    let mut fixture = gpu();
    let size = (4, 2);
    let host = HistoryHostBudget::new(1 << 20);
    let mut histories = histories(&fixture, size, &host);
    let mut oracle = Oracle::new();
    for k in 1..=8 {
        let (layers, receipt, _) = run_frame(&mut fixture, k, size, &mut oracle);
        publish(&mut histories, layers, &receipt, &host);
    }
    let packed = u64::from(size.0 * size.1 * 4);
    assert_eq!(cpu_snapshot_bytes(&mut fixture), 0);
    let mut snapshots = Vec::new();
    let mut converted = 0u64;
    for history in histories {
        assert_eq!(history.metadata().len(), 8);
        let layer = history.scope().layer_id;
        let mut handover = history.begin_cpu_handover();
        assert_eq!(handover.pending(), 8);
        let mut cancelled = false;
        while let Some(entry) = handover.next_entry().map(|request| match request {
            CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, .. }
            | CpuHistoryMaterializationRequest::ReplayToCpu { entry, .. } => entry,
        }) {
            let cancel_now = layer == 2 && entry.sequence == 4 && !cancelled;
            let result = {
                let request = match handover.next_entry().unwrap() {
                    CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, resident } => {
                        CpuMaterializationRequest::ValidatedGpu {
                            entry,
                            lease: resident,
                        }
                    }
                    CpuHistoryMaterializationRequest::ReplayToCpu { entry, frozen } => {
                        CpuMaterializationRequest::FrozenReplay {
                            entry,
                            recipe: frozen,
                        }
                    }
                };
                let cancel = move || cancel_now;
                NativeGpuMaterializer::new(&mut fixture.runtime).materialize_cpu(
                    request,
                    &mut fixture.owner,
                    &cancel,
                )
            };
            let materialized = match result {
                Ok(materialized) => materialized,
                Err(MaterializationFailure::Cancelled { .. }) if cancel_now => {
                    // Cancelled mid-barrier: 3 CPU owners + 5 recipe/resident
                    // owners stay together; nothing is published.
                    cancelled = true;
                    assert_eq!(handover.pending(), 5);
                    let before = cpu_snapshot_bytes(&mut fixture);
                    handover = match handover.finish() {
                        Err(kept) => kept,
                        Ok(_) => panic!("partial history disguised as success"),
                    };
                    assert_eq!(cpu_snapshot_bytes(&mut fixture), before);
                    continue;
                }
                Err(failure) => panic!("materialization {failure:?}"),
            };
            let recipes_before = host.snapshot().unique_bytes;
            handover
                .accept_materialized(entry, materialized)
                .ok()
                .unwrap();
            converted += 1;
            drop(handover.take_retired());
            // The CPU allocation's G charge lives in the history owner.
            assert_eq!(cpu_snapshot_bytes(&mut fixture), converted * packed);
            assert_eq!(host.snapshot().managed_alias_bytes, converted * packed);
            assert!(host.snapshot().unique_bytes <= recipes_before + packed);
        }
        if layer == 2 {
            assert!(cancelled);
        }
        let snapshot = match handover.finish() {
            Ok(snapshot) => snapshot,
            Err(_) => panic!("complete eight-slot barrier"),
        };
        let entries: Vec<_> = snapshot.entries().collect();
        assert_eq!(entries.len(), 8);
        for (k, (identity, backing)) in (1..=8).zip(entries) {
            assert_eq!(identity.entry.sequence, k);
            assert_eq!(backing.frame().layer_id, layer);
            assert_eq!(backing.frame().data, oracle[&(layer, k)]);
            match backing.origin() {
                CpuBackingOrigin::Materialized { descriptor, charge } => {
                    assert_eq!(descriptor.layer.entry, identity.entry);
                    assert_eq!(charge.kind(), ChargeKind::CpuSnapshot);
                    assert_eq!(charge.capacity(), packed);
                }
                _ => panic!("materialized CPU owner lost its proof or charge"),
            }
        }
        snapshots.push(snapshot);
    }
    // All recipe inputs released; only the 24 charged CPU owners remain.
    assert_eq!(cpu_snapshot_bytes(&mut fixture), 24 * packed);
    let host_now = host.snapshot();
    assert_eq!(host_now.allocations, 24);
    assert_eq!(host_now.managed_alias_bytes, host_now.unique_bytes);
    assert_eq!(host_now.unmanaged_bytes(), 0);

    // CPU -> native moves the same owners; the barrier back skips readback.
    let context = fixture.owner.context().id();
    for (index, snapshot) in snapshots.into_iter().enumerate() {
        let held: Vec<_> = snapshot.entries().map(|(_, b)| Arc::clone(b)).collect();
        let imported = NativeGpuHistory::import_cpu_history(snapshot, context).unwrap();
        let handover = imported.begin_cpu_handover();
        assert!(handover.next_entry().is_none());
        let snapshot = match handover.finish() {
            Ok(snapshot) => snapshot,
            Err(_) => panic!(),
        };
        for ((_, backing), held) in snapshot.entries().zip(&held) {
            assert!(Arc::ptr_eq(backing, held));
        }
        drop(held);
        let owners = snapshot.into_entries();
        let remaining = (24 - 8 * index as u64) * packed;
        assert_eq!(cpu_snapshot_bytes(&mut fixture), remaining);
        drop(owners);
        assert_eq!(cpu_snapshot_bytes(&mut fixture), remaining - 8 * packed);
    }
    // Pixels and charges were released together, only by their last owner.
    assert_eq!(cpu_snapshot_bytes(&mut fixture), 0);
    assert_eq!(host.snapshot().unique_bytes, 0);
    stop(&mut fixture.owner);
}

#[test]
fn recovered_cpu_outcome_seals_with_its_real_charge_and_returns_faults() {
    let context = GpuContextId {
        owner_id: 77,
        device_generation: 1,
    };
    let frame = request(5);
    let mut budget = NativeGpuBudget::new(context);
    let mut charge = |bytes: u64| {
        let mut plan = NativeAllocationPlan::new(frame);
        plan.add_new(ChargeKind::CpuSnapshot, bytes).unwrap();
        let quote = budget.quote(&plan).unwrap();
        let mut reservation = budget.reserve(quote).unwrap();
        let id = budget.next_allocation_id().unwrap();
        budget
            .bind_allocation(&mut reservation, id, ChargeKind::CpuSnapshot, bytes)
            .unwrap()
    };
    let size = (2, 2);
    let identity = |sequence: u64| LayerResultIdentity {
        context,
        frame,
        entry: HistoryEntryId {
            input: scope(1, size).input,
            queue_epoch: 9,
            sequence,
        },
        layer_id: 1,
        source_generation: 13,
        content_sequence: 17,
        pts_ms: sequence * 100,
        duration_ms: 2000,
        width: size.0,
        height: size.1,
        format: VideoPixelFormat::Rgba8,
        origin: LayerOrigin::Executed(crate::native_gpu_contract::FrozenChainClocks {
            frame_index: 0,
            time_seconds_bits: 0,
            time_delta_seconds_bits: 0,
            date_bits: [0; 4],
        }),
    };
    let outcome = |sequence: u64, charge| CanonicalCpuLayerOutcome {
        identity: identity(sequence),
        frame: VideoFrame {
            layer_id: 1,
            width: size.0,
            height: size.1,
            pts_ms: sequence * 100,
            duration_ms: 2000,
            format: VideoPixelFormat::Rgba8,
            data: vec![sequence as u8; 16],
        },
        faults: Vec::new(),
        provenance: CpuOutcomeProvenance::FixedClockRecovery,
        charge,
    };
    let host = HistoryHostBudget::new(1 << 10);
    // Another frame's outcome (stale) and an undersized charge are refused.
    let stale =
        NativeGpuValidatedEntries::seal_recovered_cpu(request(4), outcome(1, charge(16)), &host)
            .err()
            .unwrap();
    assert_eq!(stale.error, HistoryError::IdentityMismatch);
    let small = NativeGpuValidatedEntries::seal_recovered_cpu(frame, outcome(1, charge(8)), &host)
        .err()
        .unwrap();
    assert_eq!(small.error, HistoryError::IdentityMismatch);
    assert_eq!(small.outcome.charge.capacity(), 8);
    assert_eq!(host.snapshot().charges, 0);
    let (sealed, faults) =
        match NativeGpuValidatedEntries::seal_recovered_cpu(frame, stale.outcome, &host) {
            Ok(v) => v,
            Err(failure) => panic!("seal {:?}", failure.error),
        };
    assert!(faults.is_empty());
    assert_eq!(sealed.phase(), HistoryPhaseOutcome::RecoveredCpu);
    let mut history = NativeGpuHistory::new(
        context,
        HistoryScope {
            frame_scope: frame,
            ..scope(1, size)
        },
        host.clone(),
    )
    .unwrap();
    let plan = history.plan(&sealed.metadata(), 100).unwrap();
    let commit = history.commit_validated(plan, sealed).ok().unwrap();
    assert!(!commit.output_phases_valid);
    let live = budget
        .snapshot()
        .for_kind(ChargeKind::CpuSnapshot)
        .active_bytes;
    assert_eq!(host.snapshot().managed_alias_bytes, 16);
    let handover = history.begin_cpu_handover();
    let snapshot = match handover.finish() {
        Ok(snapshot) => snapshot,
        Err(_) => panic!("recovered CPU backing needs no readback"),
    };
    let (_, backing) = snapshot.entries().next().unwrap();
    match backing.origin() {
        CpuBackingOrigin::Recovered { charge } => assert_eq!(charge.capacity(), 16),
        _ => panic!(),
    }
    assert_eq!(backing.frame().data, vec![1; 16]);
    assert_eq!(
        budget
            .snapshot()
            .for_kind(ChargeKind::CpuSnapshot)
            .active_bytes,
        live
    );
    drop(snapshot);
    drop(small);
    assert_eq!(host.snapshot().unique_bytes, 0);
}
