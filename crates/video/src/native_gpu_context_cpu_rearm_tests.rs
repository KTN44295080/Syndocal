//! Real readback -> canonical queue -> replacement-device CPU execution.
use super::*;
use crate::isf_native_gpu::tests::{identity_jobs, request, validated, Fixture};
use crate::isf_native_gpu::{
    await_transaction, CpuMaterializationRequest, NativeGpuMaterializer,
    PreparedNativeCpuInvocation,
};
use crate::isf_runtime::IsfGpuRuntime;
use crate::native_gpu_history::adapter::{
    CanonicalCpuHistoryFrame, CanonicalCpuHistoryOwner, NativeGpuHistory, NativeGpuValidatedEntries,
};
use crate::native_gpu_history::{
    CpuHistoryMaterializationRequest, HistoryHostBudget, HistoryScope,
};
use crate::native_gpu_output::cpu::history::{
    accept_cpu_history_materialization, NativeCpuHistoryQueue,
};
use crate::FrameQueue;

#[test]
fn i8_cpu_history_real_rearm_preserves_readback_owner_through_queue_and_scoped_cpu_fallback() {
    let factory = NativeGpuFactory::new();
    let mut owner = factory
        .create_headless(101, NativeGpuRequirements::default())
        .unwrap();
    let runtime = IsfGpuRuntime::with_native_context(&mut owner).unwrap();
    let mut fixture = Fixture { owner, runtime };
    let jobs = identity_jobs(65, 3);
    let oracle = jobs[0].input.data.clone();
    let chain = jobs[0].chain.clone();
    let date = jobs[0].date;
    let (layers, receipt) = validated(&mut fixture, 1, jobs);
    let mut layers = layers.into_iter();
    let first = layers.next().unwrap();
    // A real second texture remains alive solely to test stale GPU rejection.
    let stale_gpu = Arc::new(layers.next().unwrap().lease);
    drop(layers);
    let identity = first.identity;
    let scope = HistoryScope {
        frame_scope: identity.frame,
        input: identity.entry.input,
        layer_id: identity.layer_id,
        queue_epoch: identity.entry.queue_epoch,
        source_generation: identity.source_generation,
        extent: (identity.width, identity.height),
    };
    let packed = u64::from(identity.width * identity.height * 4);
    let host = HistoryHostBudget::new(crate::native_gpu_budget::NATIVE_MANAGED_LIMIT);
    let mut history =
        NativeGpuHistory::new(fixture.owner.context().id(), scope, host.clone()).unwrap();
    let sealed = NativeGpuValidatedEntries::seal_isf_layer(&receipt, first, &host)
        .unwrap_or_else(|failure| panic!("seal {:?}", failure.error));
    let plan = history.plan(&sealed.metadata(), identity.pts_ms).unwrap();
    drop(
        history
            .commit_validated(plan, sealed)
            .unwrap_or_else(|failure| panic!("commit {:?}", failure.error)),
    );
    let mut handover = history.begin_cpu_handover();
    let (entry, lease) = match handover.next_entry().unwrap() {
        CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, resident } => (entry, resident),
        CpuHistoryMaterializationRequest::ReplayToCpu { .. } => {
            panic!("first survivor is resident")
        }
    };
    let materialized = NativeGpuMaterializer::new(&mut fixture.runtime)
        .materialize_cpu(
            CpuMaterializationRequest::ValidatedGpu { entry, lease },
            &mut fixture.owner,
            &|| false,
        )
        .unwrap();
    let pixels = materialized.frame().data.as_ptr();
    accept_cpu_history_materialization(&mut handover, entry, materialized)
        .unwrap_or_else(|failure| panic!("materialization {:?}", failure.error));
    drop(handover.take_retired());
    let snapshot = handover
        .finish()
        .unwrap_or_else(|_| panic!("complete handover"));
    let history = NativeCpuHistoryQueue::from_snapshot(scope, snapshot)
        .unwrap_or_else(|failure| panic!("snapshot {:?}", failure.error));
    let mut canonical = FrameQueue::new_for_input(scope.input, scope.layer_id, 8);
    canonical.install_native_history_replacement(history);
    let backing = Arc::downgrade(canonical.native_owners[0].as_ref().unwrap().backing());
    let alias = canonical.native_owners[0]
        .as_ref()
        .unwrap()
        .native_cpu_owner()
        .unwrap();
    let allocation = alias.charge.as_ref().unwrap().id();
    let old_context = fixture.owner.context().id();
    assert_eq!(allocation.context, old_context);
    assert_eq!(alias.frame.data.as_ptr(), pixels);
    assert_eq!(alias.frame.data, oracle);
    assert_eq!(host.snapshot().managed_alias_bytes, packed);

    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    owner.begin_stop();
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 2
        }
    );
    let old_bytes = owner.budget_snapshot().managed_bytes();
    let replacement = factory
        .create_headless(101, NativeGpuRequirements::default())
        .unwrap();
    owner
        .rearm_with_replacement(replacement)
        .unwrap_or_else(|(fault, _)| panic!("CPU history re-arm: {fault:?}"));
    let current = owner.context().id();
    assert!(current.device_generation > old_context.device_generation);
    assert_eq!(owner.budget_snapshot().managed_bytes(), old_bytes);

    // P also accepts the original, charged CPU allocation on the new device.
    let imported = NativeGpuHistory::import_canonical_queue_with_owners(
        current,
        scope,
        host.clone(),
        vec![CanonicalCpuHistoryFrame {
            entry: identity.entry,
            frame: Arc::clone(&alias.frame),
            owner: Some(CanonicalCpuHistoryOwner::Native(Arc::clone(&alias))),
        }],
    )
    .unwrap_or_else(|failure| panic!("re-arm import {:?}", failure.error));
    let imported = imported
        .begin_cpu_handover()
        .finish()
        .unwrap_or_else(|_| panic!("CPU import"));
    assert!(Arc::ptr_eq(
        imported.entries().next().unwrap().1,
        &backing.upgrade().unwrap()
    ));
    drop(imported);
    assert_eq!(host.snapshot().unique_bytes, packed);

    let rejected_ids = [
        stale_gpu.allocation_id(),
        GpuAllocationId {
            context: GpuContextId {
                owner_id: current.owner_id + 1,
                ..old_context
            },
            ..allocation
        },
        GpuAllocationId {
            context: GpuContextId {
                device_generation: current.device_generation + 1,
                ..current
            },
            ..allocation
        },
    ];
    let before_rejection = owner.budget_snapshot();
    for id in rejected_ids {
        let mut rejected = NativeAllocationPlan::new(request(90));
        rejected.add_existing(id);
        assert!(matches!(
            owner.quote(&rejected),
            Err(NativeGpuDecline::QuoteExpired)
        ));
        assert_eq!(owner.budget_snapshot(), before_rejection);
    }
    let mut aliases = NativeAllocationPlan::new(request(90));
    aliases.add_existing(allocation);
    let reservation = owner.reserve(owner.quote(&aliases).unwrap()).unwrap();
    let mut transaction = owner.begin_frame(request(90), reservation).unwrap();
    transaction.validate_plan(&aliases).unwrap();
    transaction.validate_root_plan(&aliases).unwrap();
    for invalid in rejected_ids {
        assert!(transaction
            .retain_resource(Arc::clone(&alias), &[allocation, invalid])
            .is_err());
        let index = transaction.owner.core.index(transaction.id()).unwrap();
        assert!(
            transaction.owner.core.frames[index].allocations.is_empty(),
            "mixed rejection cannot publish a prefix"
        );
        assert!(transaction.owner.core.frames[index].resources.is_empty());
    }
    assert!(stale_gpu.pin_for_read(&mut transaction).is_err());
    transaction
        .retain_resource(Arc::clone(&alias), &[allocation])
        .unwrap();
    let mut phase = transaction.begin_phase(GpuPhase::IsfBatch).unwrap();
    phase
        .retain_resource(Arc::clone(&alias), &[allocation])
        .unwrap();
    let encoder = phase
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let encoded = phase.finish(vec![encoder.finish()]);
    let receipt = transaction.submit_phase(encoded).unwrap();
    assert_eq!(receipt.submission().context, current);
    assert_eq!(
        receipt.submission().sequence,
        1,
        "submission sequences reset at re-arm"
    );
    let pending = transaction.finish();
    await_transaction(&mut owner, pending, &|| false)
        .unwrap()
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();

    let mut runtime = IsfGpuRuntime::with_native_context(&mut owner).unwrap();
    runtime.remember_native_cpu_frame(&alias);
    let invocation = PreparedNativeCpuInvocation::new(Arc::clone(&alias.frame), chain, date, None);
    let prepared = runtime
        .prepare_native_cpu_inputs(request(91), invocation)
        .unwrap();
    let quote = owner.quote(prepared.allocation_plan().unwrap()).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    assert_eq!(canonical.drop_before(identity.pts_ms + 1), 1);
    drop(canonical);
    assert!(backing.upgrade().is_some());
    let result = runtime
        .render_native_cpu_fallback(&mut owner, prepared, reservation, &|| false)
        .unwrap_or_else(|failure| panic!("re-armed scoped CPU execution: {failure:?}"));
    assert_eq!(result.outcomes().len(), 1);
    assert!(result.outcomes()[0].faults().is_empty());
    assert_eq!(result.outcomes()[0].frame().data, oracle);
    assert_eq!(alias.frame.data.as_ptr(), pixels);
    assert_eq!(alias.charge.as_ref().unwrap().id(), allocation);
    assert_eq!(host.snapshot().managed_alias_bytes, packed);
    assert_eq!(
        owner
            .budget_snapshot()
            .for_kind(ChargeKind::ActiveCpuInput)
            .active_bytes,
        0,
        "the re-armed executor must alias the existing CPU pixels, not charge a duplicate input"
    );
    drop(result);
    drop(runtime);
    drop(stale_gpu);
    owner.begin_stop();
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1
        }
    );
    assert_eq!(owner.budget_snapshot().managed_bytes(), packed);

    // Re-arm once more without using the pixels on that third device. Reaping
    // now must still consult the completed second-device read, not origin ID 1.
    let replacement = factory
        .create_headless(101, NativeGpuRequirements::default())
        .unwrap();
    owner
        .rearm_with_replacement(replacement)
        .unwrap_or_else(|(fault, _)| panic!("second CPU history re-arm: {fault:?}"));
    let mut plan = NativeAllocationPlan::new(request(92));
    plan.add_existing(allocation);
    assert!(owner.quote(&plan).is_ok());
    assert_eq!(alias.frame.data.as_ptr(), pixels);
    drop(alias);
    assert!(backing.upgrade().is_none());
    assert_eq!(host.snapshot().unique_bytes, 0);
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}
