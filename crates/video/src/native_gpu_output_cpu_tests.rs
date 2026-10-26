use super::*;
use crate::isf_native_gpu::tests::{gpu, identity_jobs, validated, Fixture};
use crate::isf_native_gpu::{CpuMaterializationRequest, NativeGpuMaterializer};
use crate::native_gpu_context::{NativeGpuOwner, RetirementProgress};

fn cpu_charge_bytes(owner: &NativeGpuOwner) -> u64 {
    let snapshot = owner.budget_snapshot();
    let cpu = snapshot.for_kind(ChargeKind::CpuSnapshot);
    cpu.active_bytes + cpu.retired_bytes + cpu.quarantined_bytes
}

#[test]
fn i3_cpu_history_real_materialization_keeps_charge_with_last_arc_through_stop() {
    let mut fixture = gpu();
    let prepared = identity_jobs(65, 3);
    let expected_pixels = Arc::clone(&prepared[0].input);
    let (layers, receipt) = validated(&mut fixture, 1, prepared);
    let layer = &layers[0];
    let materialization = NativeGpuMaterializer::new(&mut fixture.runtime)
        .materialize_cpu(
            CpuMaterializationRequest::ValidatedGpu {
                entry: layer.identity.entry,
                lease: &layer.lease,
            },
            &mut fixture.owner,
            &|| false,
        )
        .unwrap();
    let descriptor = *materialization.descriptor();
    let pixels = materialization.frame().data.as_ptr();
    let capacity = materialization.frame().data.capacity() as u64;
    let charged =
        match ChargedCpuHistoryFrame::from_materialization(layer.identity, materialization) {
            Ok(charged) => charged,
            Err(rejected) => panic!("real materialization was rejected: {:?}", rejected.fault()),
        };
    assert_eq!(charged.frame(), expected_pixels.as_ref());
    assert_eq!(
        charged.frame().data.as_ptr(),
        pixels,
        "handover moves the Vec"
    );
    assert_eq!(charged.descriptor(), &descriptor);
    assert_eq!(charged.charged_capacity_bytes(), capacity);
    assert_eq!(
        charged.cpu_allocation_id().context,
        descriptor.layer.context
    );
    assert_ne!(
        charged.cpu_allocation_id(),
        descriptor.allocation,
        "GPU source identity is not the charged CPU allocation",
    );
    let alias = Arc::clone(&charged);
    let second_alias = Arc::clone(&charged);
    fixture.owner.poll_retirement().unwrap();
    assert_eq!(
        cpu_charge_bytes(&fixture.owner),
        capacity,
        "aliases charge once",
    );
    assert_eq!(Arc::strong_count(&charged), 3);
    drop(charged);
    assert_eq!(alias.frame().data.as_ptr(), pixels);
    drop(second_alias);
    assert_eq!(Arc::strong_count(&alias), 1);
    drop(expected_pixels);
    drop(layers);
    drop(receipt);
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    let retirement = owner.begin_stop();
    assert_eq!(retirement.context(), descriptor.layer.context);
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1,
        },
        "Stop cannot report Reaped while the CPU history alias is alive",
    );
    assert_eq!(owner.budget_snapshot().managed_bytes(), capacity);
    assert_eq!(cpu_charge_bytes(&owner), capacity);
    assert_eq!(alias.frame().data.as_ptr(), pixels);
    assert_eq!(alias.descriptor(), &descriptor);
    drop(alias);
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

#[test]
fn i3_cpu_history_wrong_expected_entry_retains_real_evidence_and_charge() {
    let mut fixture = gpu();
    let (layers, receipt) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let layer = &layers[0];
    let materialization = NativeGpuMaterializer::new(&mut fixture.runtime)
        .materialize_cpu(
            CpuMaterializationRequest::ValidatedGpu {
                entry: layer.identity.entry,
                lease: &layer.lease,
            },
            &mut fixture.owner,
            &|| false,
        )
        .unwrap();
    let descriptor = *materialization.descriptor();
    let pixels = materialization.frame().data.as_ptr();
    let capacity = materialization.frame().data.capacity() as u64;
    let mut stale_expected = layer.identity;
    stale_expected.entry.sequence += 1;
    let rejected =
        match ChargedCpuHistoryFrame::from_materialization(stale_expected, materialization) {
            Ok(_) => panic!("a different requested history entry cannot accept real evidence"),
            Err(rejected) => rejected,
        };
    assert_eq!(rejected.fault().kind, NativeGpuFaultKind::IdentityMismatch);
    assert_eq!(rejected.fault().context, descriptor.layer.context);
    assert_eq!(rejected.fault().frame, Some(descriptor.layer.frame));
    assert_eq!(
        rejected.descriptor(),
        &descriptor,
        "failure keeps the actual proof",
    );
    assert_eq!(
        rejected.frame().data.as_ptr(),
        pixels,
        "failure keeps original pixels",
    );
    assert_eq!(rejected.frame(), layer.recipe.job().input.as_ref());
    assert_eq!(rejected.charged_capacity_bytes(), capacity);
    drop(layers);
    drop(receipt);
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    owner.begin_stop();
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1,
        },
    );
    assert_eq!(owner.budget_snapshot().managed_bytes(), capacity);
    assert_eq!(cpu_charge_bytes(&owner), capacity);
    assert_eq!(rejected.frame().data.as_ptr(), pixels);
    drop(rejected);
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}
