use super::*;
use crate::isf_native_gpu::tests::{gpu, identity_jobs, jobs, shader, stop, validated};

const CLOCK_RED: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(float(FRAMEINDEX)/255.0,c.gba); }"#;

fn selected_jobs(count: usize) -> StackBatch<PreparedNativeLayer> {
    let mut selected = StackBatch::Empty;
    for job in jobs(8, 4, &[shader(CLOCK_RED)]).into_iter().take(count) {
        selected.push(job);
    }
    selected
}

#[test]
fn isf_native_pool_240_ticks_reuse_scratch_and_keep_history_budget_steady() {
    for count in 1..=3 {
        let mut fixture = gpu();
        let mut history = std::collections::VecDeque::new();
        let mut steady = None;
        let mut scratch_ids = None;
        let mut reused = false;
        let mut seen = std::collections::HashSet::new();
        for tick in 1..=244 {
            let (layers, receipt) = validated(&mut fixture, tick, selected_jobs(count));
            for layer in &*layers {
                // Last two history batches remain referenced while encoding the
                // next tick. No resident may alias either protected generation.
                for previous in &history {
                    let previous: &StackBatch<ValidatedLayerResult> = previous;
                    assert!(previous
                        .iter()
                        .all(|old| old.lease.allocation_id() != layer.lease.allocation_id()));
                }
                if !seen.insert(layer.lease.allocation_id()) {
                    reused = true;
                }
            }
            drop(receipt);
            if tick <= 5 {
                for layer in history
                    .iter()
                    .flat_map(|batch: &StackBatch<ValidatedLayerResult>| batch.iter())
                    .chain(layers.iter())
                {
                    let pixels = NativeGpuMaterializer::new(&mut fixture.runtime)
                        .materialize_cpu(
                            CpuMaterializationRequest::ValidatedGpu {
                                entry: layer.identity.entry,
                                lease: &layer.lease,
                            },
                            &mut fixture.owner,
                            &|| false,
                        )
                        .unwrap();
                    assert_eq!(pixels.frame().data[0], layer.recipe.clocks().frame_index.min(255) as u8,
                        "pooled uniforms and retained history must preserve their frozen clock pixels");
                }
            }
            history.push_back(layers);
            if history.len() > 2 {
                history.pop_front();
            }
            let ids: Vec<_> = fixture
                .runtime
                .native_stack_pool
                .scratch
                .iter()
                .filter_map(|(_, weak)| weak.upgrade())
                .flat_map(|scratch| scratch.ids())
                .collect();
            if let Some(expected) = &scratch_ids {
                assert_eq!(
                    &ids, expected,
                    "scratch allocation changed after verified retirement"
                );
            } else {
                scratch_ids = Some(ids);
            }
            if tick >= 4 {
                // Materialization checks above have their own temporary CPU
                // allocations. Compare the persistent pool only after those
                // dropped results receive the ordinary verified ledger sweep.
                fixture.owner.poll_retirement().unwrap();
                let bytes = fixture.owner.budget_snapshot().allocated_bytes;
                if let Some(expected) = steady {
                    assert_eq!(
                        bytes, expected,
                        "resident budget grew at tick {tick}, layers {count}"
                    );
                } else {
                    steady = Some(bytes);
                }
                assert_eq!(fixture.runtime.native_stack_pool.scratch.len(), count);
                assert_eq!(fixture.runtime.native_stack_pool.finals.len(), count * 3);
            }
        }
        assert!(reused, "no retired final image was reused");
        drop(history);
        stop(&mut fixture.owner);
    }
}

#[test]
fn isf_native_pool_inflight_checkout_and_retained_lease_block_reuse() {
    let mut fixture = gpu();
    let (first, receipt) = validated(&mut fixture, 1, identity_jobs(8, 4));
    drop(receipt);
    let prepared = fixture
        .runtime
        .prepare_native_inputs(
            crate::isf_native_gpu::tests::request(2),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(prepared
        .pooled
        .iter()
        .all(|selection| selection.scratch.is_some() && selection.final_image.is_none()));
    // A second frozen preflight cannot take the same scratch checkout.
    let other = fixture
        .runtime
        .prepare_native_inputs(
            crate::isf_native_gpu::tests::request(3),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(
        matches!(
            other.allocation_plan(),
            Err(NativeGpuDecline::BudgetExceeded { .. })
        ),
        "a pending checkout must decline before allocating a second scratch set"
    );
    drop(other);
    drop(prepared);
    drop(first);
    let released = fixture
        .runtime
        .prepare_native_inputs(
            crate::isf_native_gpu::tests::request(4),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(released
        .pooled
        .iter()
        .all(|selection| selection.scratch.is_some() && selection.final_image.is_some()));
    drop(released);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_pool_invalidation_releases_idle_but_preserves_history_charge() {
    let mut fixture = gpu();
    let (layers, receipt) = validated(&mut fixture, 1, identity_jobs(8, 4));
    drop(receipt);
    let epoch = fixture.owner.context().resource_pool_epoch();
    let before = fixture.owner.budget_snapshot().allocated_bytes;
    fixture.owner.release_resource_pools();
    fixture.runtime.clear_native_stack_pool();
    fixture.owner.poll_retirement().unwrap();
    assert!(fixture.owner.context().resource_pool_epoch() > epoch);
    let retained = fixture.owner.budget_snapshot().allocated_bytes;
    assert!(
        retained > 0 && retained < before,
        "history owns its final image independently of released scratch"
    );
    let (next, receipt) = validated(&mut fixture, 2, identity_jobs(4, 2));
    for old in &*layers {
        assert!(next
            .iter()
            .all(|new| old.lease.allocation_id() != new.lease.allocation_id()));
    }
    drop(receipt);
    drop(layers);
    drop(next);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_pool_stale_epoch_rejects_before_semantic_clock_commit() {
    let mut fixture = gpu();
    let frame = crate::isf_native_gpu::tests::request(1);
    let inputs = fixture
        .runtime
        .prepare_native_inputs(
            frame,
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let quote = fixture
        .owner
        .quote(inputs.allocation_plan().unwrap())
        .unwrap();
    let batch = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        NativeBatchAdmission::Declined { reason, .. } => {
            panic!("fresh preflight declined: {reason:?}")
        }
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    fixture.owner.release_resource_pools();
    assert!(matches!(
        batch.prospective_identities(&fixture.runtime),
        Err(NativeProspectiveIdentityError::StalePool)
    ));
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let failure = fixture
        .runtime
        .encode_native_batch(&mut txn, batch)
        .err()
        .expect("stale pool epoch must reject encoding");
    assert_eq!(failure.kind, NativeGpuFaultKind::StateMismatch);
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    let pending = txn.finish();
    drop(await_transaction(&mut fixture.owner, pending, &|| false).unwrap());
    stop(&mut fixture.owner);
}
