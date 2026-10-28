use super::*;
use crate::isf_native_gpu::tests::{encode, gpu, identity_jobs, request, stop, validated, Fixture};
use crate::isf_native_gpu::{await_transaction, NativeBatchAdmission, NativeBatchRestrictions};
use crate::native_gpu_budget::{ChargeKind, NativeAllocationPlan};
use crate::native_gpu_contract::{GpuPhase, MaterializationFailure, NativeGpuFaultKind};

/// G keeps each validated batch's idle ISF bank pooled for the next tick with
/// the same shape (2607a001): per layer two scratch images plus one uniform
/// per stage, and the final image the lease also references. Prove that bank's
/// exact ledger, then release it, so the assertions that follow measure only
/// lease, read-pin and transaction ownership (the same isolation the snapshot,
/// shared-output and surface wait helpers apply before exact charge checks).
fn release_idle_stack_bank(fixture: &mut Fixture, layers: &[ValidatedLayerResult]) {
    fixture.owner.poll_retirement().unwrap();
    let pixels: u64 = layers
        .iter()
        .map(|layer| u64::from(layer.identity.width) * u64::from(layer.identity.height) * 4)
        .sum();
    let stages: u64 = layers
        .iter()
        .map(|layer| layer.recipe.job().chain.stages.len() as u64)
        .sum();
    let snapshot = fixture.owner.budget_snapshot();
    let active = |kind| snapshot.for_kind(kind).active_bytes;
    assert_eq!(fixture.owner.pooled_resource_count(), layers.len() * 2);
    assert_eq!(active(ChargeKind::IsfScratch), pixels * 2);
    assert_eq!(active(ChargeKind::ResidentLayer), pixels);
    let uniforms = active(ChargeKind::CachedEffectResource);
    assert!(
        uniforms > 0 && uniforms % stages == 0,
        "{uniforms} uniform bytes for {stages} stages"
    );
    assert_eq!(snapshot.allocated_bytes, pixels * 3 + uniforms);
    assert_eq!(
        snapshot.allocation_count,
        layers.len() * 3 + stages as usize
    );
    fixture.owner.release_resource_pools();
    assert_eq!(fixture.owner.pooled_resource_count(), 0);
}

#[test]
fn native_gpu_result_candidate_and_lease_reject_other_device_or_transaction() {
    let mut fixture = gpu();
    let mut other = gpu();
    let (mut encoded, pending) = encode(&mut fixture, 1, identity_jobs(65, 3));
    assert_ne!(fixture.owner.context().id(), other.owner.context().id());
    let frame = request(2);
    let plan = NativeAllocationPlan::new(frame);
    let quote = other.owner.quote(&plan).unwrap();
    let reservation = other.owner.reserve(quote).unwrap();
    let mut txn = other.owner.begin_frame(frame, reservation).unwrap();
    let rejected = encoded.candidates[0].pin_for_read(&mut txn).err().unwrap();
    assert_eq!(rejected.kind, NativeGpuFaultKind::IdentityMismatch);
    let empty = txn.finish();
    await_transaction(&mut other.owner, empty, &|| false).unwrap();

    let receipt = encoded.pending.take().unwrap();
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    let quote = fixture.owner.quote(&plan).unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let rejected = encoded.candidates[0].pin_for_read(&mut txn).err().unwrap();
    assert_eq!(rejected.kind, NativeGpuFaultKind::IdentityMismatch);
    let empty = txn.finish();
    await_transaction(&mut fixture.owner, empty, &|| false).unwrap();

    let layers =
        match fixture
            .runtime
            .complete_native_batch(encoded, phase, &mut fixture.owner, &|| false)
        {
            NativeIsfCompletion::Validated { layers, .. } => layers,
            _ => panic!("identity rejection must not invalidate the originating work"),
        };
    let quote = other.owner.quote(&plan).unwrap();
    let reservation = other.owner.reserve(quote).unwrap();
    let mut txn = other.owner.begin_frame(frame, reservation).unwrap();
    let rejected = layers[0].lease.pin_for_read(&mut txn).err().unwrap();
    assert_eq!(rejected.kind, NativeGpuFaultKind::IdentityMismatch);
    let empty = txn.finish();
    await_transaction(&mut other.owner, empty, &|| false).unwrap();
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    assert_eq!(other.runtime.native_frame_index(), 0);
    drop(layers);
    stop(&mut fixture.owner);
    stop(&mut other.owner);
}

#[test]
fn native_gpu_result_real_receipt_is_single_use_and_wrong_phase_rejects_candidate() {
    let mut fixture = gpu();
    let frame = request(1);
    let inputs = fixture
        .runtime
        .prepare_native_inputs(
            frame,
            identity_jobs(65, 3),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let quote = fixture
        .owner
        .quote(inputs.allocation_plan().unwrap())
        .unwrap();
    let admitted = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        _ => panic!("ordinary fixture should be admitted"),
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let mut encoded = fixture
        .runtime
        .encode_native_batch(&mut txn, admitted)
        .unwrap();
    let phase = txn.begin_phase(GpuPhase::CompositionSnapshot).unwrap();
    let commands = phase.finish(Vec::new());
    let wrong_receipt = txn.submit_phase(commands).unwrap();
    let pending = txn.finish();
    let isf_receipt = encoded.pending.take().unwrap();
    assert_eq!(
        encoded.pending.take().err().unwrap().kind,
        NativeGpuFaultKind::StateMismatch
    );
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let wrong_phase = resolved.take_phase(wrong_receipt).unwrap();
    assert!(
        wrong_phase.validation().is_ok(),
        "a real successful unrelated phase is insufficient"
    );
    let isf_phase = resolved.take_phase(isf_receipt).unwrap();
    let mut candidates = encoded.candidates.into_iter();
    let rejected = candidates
        .next()
        .unwrap()
        .validate(&wrong_phase)
        .err()
        .unwrap();
    assert_eq!(rejected.kind, NativeGpuFaultKind::IdentityMismatch);
    let mut leases = Vec::new();
    for candidate in candidates {
        leases.push(candidate.validate(&isf_phase).unwrap());
    }
    assert_eq!(leases.len(), 2);
    let validated = IsfValidatedReceipt::from_phase(isf_phase).unwrap();
    assert_eq!(validated.phase().phase(), GpuPhase::IsfBatch);
    assert_eq!(
        IsfValidatedReceipt::from_phase(wrong_phase)
            .err()
            .unwrap()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    drop(leases);
    drop(encoded.jobs);
    drop(encoded.bindings);
    drop(encoded.inputs);
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_validated_layers_retain_only_three_final_allocations() {
    let mut fixture = gpu();
    let width = 65;
    let height = 3;
    let pixels = u64::from(width) * u64::from(height) * 4;
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(width, height));
    release_idle_stack_bank(&mut fixture, &layers);
    fixture.owner.poll_retirement().unwrap();
    let snapshot = fixture.owner.budget_snapshot();
    assert_eq!(snapshot.reserved_bytes, 0);
    assert_eq!(snapshot.allocated_bytes, pixels * 3);
    assert_eq!(snapshot.allocation_count, 3);
    assert_eq!(
        snapshot.for_kind(ChargeKind::ResidentLayer).active_bytes,
        pixels * 3
    );
    for kind in [
        ChargeKind::IsfScratch,
        ChargeKind::QueueUploadStaging,
        ChargeKind::ActiveCpuInput,
        ChargeKind::SnapshotReadback,
        ChargeKind::CpuSnapshot,
    ] {
        let retained = snapshot.for_kind(kind);
        assert_eq!(
            retained.active_bytes
                + retained.retired_bytes
                + retained.quarantined_bytes
                + retained.reserved_bytes,
            0,
            "a final lease must not retain {kind:?}"
        );
    }
    assert!(layers
        .iter()
        .all(|layer| layer.lease.allocation.image.width == width));
    drop(layers);
    fixture.owner.poll_retirement().unwrap();
    assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_read_pin_keeps_image_after_lease_drop_until_completed_and_released() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    release_idle_stack_bank(&mut fixture, &layers);
    let mut retained_lease = None;
    for layer in layers {
        if retained_lease.is_none() {
            retained_lease = Some(layer.lease);
        }
    }
    let lease = retained_lease.unwrap();
    let identity = *lease.identity();
    let weak = Arc::downgrade(&lease.allocation);
    let allocation = lease.allocation_id();
    fixture.owner.poll_retirement().unwrap();
    let pixels = u64::from(identity.width) * u64::from(identity.height) * 4;
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, pixels);
    let pitch = (identity.width * 4).div_ceil(256) * 256;
    let padded = u64::from(pitch) * u64::from(identity.height);
    let frame = request(2);
    let mut plan = NativeAllocationPlan::new(frame);
    plan.add_existing(allocation);
    plan.add_new(ChargeKind::SnapshotReadback, padded).unwrap();
    let quote = fixture.owner.quote(&plan).unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let read = lease.pin_for_read(&mut txn).unwrap();
    assert_eq!(read.id(), allocation);
    drop(lease);
    assert!(
        weak.upgrade().is_some(),
        "read ownership outlives the historical lease"
    );
    let mut phase = txn.begin_phase(GpuPhase::CompositionSnapshot).unwrap();
    let charge = phase
        .bind_allocation(ChargeKind::SnapshotReadback, padded)
        .unwrap();
    let buffer = phase.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("lease ownership real copy"),
        size: padded,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let buffer_owner = Arc::new((buffer, charge));
    phase
        .retain_resource(Arc::clone(&buffer_owner), &[buffer_owner.1.id()])
        .unwrap();
    let mut commands = phase
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("lease ownership real copy"),
        });
    commands.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: read.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer_owner.0,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch),
                rows_per_image: Some(identity.height),
            },
        },
        wgpu::Extent3d {
            width: identity.width,
            height: identity.height,
            depth_or_array_layers: 1,
        },
    );
    let encoded = phase.finish(vec![commands.finish()]);
    let receipt = txn.submit_phase(encoded).unwrap();
    let pending = txn.finish();
    assert_eq!(
        fixture.owner.budget_snapshot().allocated_bytes,
        pixels + padded
    );
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    resolved
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    drop(buffer_owner);
    fixture.owner.poll_retirement().unwrap();
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, pixels);
    assert!(
        weak.upgrade().is_some(),
        "completed read handle still owns its physical image"
    );
    drop(read);
    fixture.owner.poll_retirement().unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_materialization_checks_exact_entry_and_cancellation_before_copy() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let layer = &layers[0];
    let before = fixture.owner.budget_snapshot();
    let mut wrong = layer.identity.entry;
    wrong.sequence += 1;
    let rejected = fixture.runtime.materialize_validated_layer(
        wrong,
        &layer.lease,
        &mut fixture.owner,
        &|| false,
    );
    assert!(matches!(
        rejected,
        Err(MaterializationFailure::Fault {
            fault: NativeGpuFault {
                kind: NativeGpuFaultKind::IdentityMismatch,
                ..
            },
            retirement: None
        })
    ));
    assert_eq!(fixture.owner.budget_snapshot(), before);
    let cancelled = fixture.runtime.materialize_validated_layer(
        layer.identity.entry,
        &layer.lease,
        &mut fixture.owner,
        &|| true,
    );
    assert!(matches!(
        cancelled,
        Err(MaterializationFailure::Cancelled { retirement: None })
    ));
    assert_eq!(fixture.owner.budget_snapshot(), before);
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    let materialized = fixture
        .runtime
        .materialize_validated_layer(
            layer.identity.entry,
            &layer.lease,
            &mut fixture.owner,
            &|| false,
        )
        .unwrap();
    let (frame, descriptor, charge) = materialized.into_parts();
    assert_eq!(descriptor.layer, layer.identity);
    assert_eq!(descriptor.allocation, layer.lease.allocation_id());
    assert_eq!(
        descriptor.completed_submission.context,
        layer.identity.context
    );
    assert_eq!(frame, *layer.recipe.job().input);
    assert_eq!(charge.kind(), ChargeKind::CpuSnapshot);
    assert_eq!(charge.capacity(), frame.data.capacity() as u64);
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    drop(frame);
    drop(charge);
    drop(layers);
    stop(&mut fixture.owner);
}

fn completed_cpu_copy(
    fixture: &mut crate::isf_native_gpu::tests::Fixture,
    lease: &GpuLayerLease,
    frame: NativeFrameId,
    other_phase: Option<GpuPhase>,
) -> (
    PendingCpuMaterialization,
    ResolvedGpuPhase,
    Option<ResolvedGpuPhase>,
) {
    let identity = lease.identity();
    let padded = checked_upload_staging_bytes(identity.width, identity.height, Some(256)).unwrap();
    let pixels = u64::from(identity.width) * u64::from(identity.height) * 4;
    let mut plan = NativeAllocationPlan::new(frame);
    plan.add_existing(lease.allocation_id());
    plan.add_new(ChargeKind::SnapshotReadback, padded).unwrap();
    plan.add_new(ChargeKind::CpuSnapshot, pixels).unwrap();
    let reservation = fixture
        .owner
        .reserve(fixture.owner.quote(&plan).unwrap())
        .unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let (copy, receipt) = PendingCpuMaterialization::encode(&mut txn, lease).unwrap();
    let other_receipt = other_phase.map(|kind| {
        let phase = txn.begin_phase(kind).unwrap();
        let encoded = phase.finish(Vec::new());
        txn.submit_phase(encoded).unwrap()
    });
    let pending = txn.finish();
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    assert!(phase.validation().is_ok());
    let other = other_receipt.map(|receipt| resolved.take_phase(receipt).unwrap());
    (copy, phase, other)
}

fn completed_empty_phase(
    fixture: &mut crate::isf_native_gpu::tests::Fixture,
    frame: NativeFrameId,
    kind: GpuPhase,
) -> ResolvedGpuPhase {
    let plan = NativeAllocationPlan::new(frame);
    let reservation = fixture
        .owner
        .reserve(fixture.owner.quote(&plan).unwrap())
        .unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let phase = txn.begin_phase(kind).unwrap();
    let encoded = phase.finish(Vec::new());
    let receipt = txn.submit_phase(encoded).unwrap();
    let pending = txn.finish();
    await_transaction(&mut fixture.owner, pending, &|| false)
        .unwrap()
        .take_phase(receipt)
        .unwrap()
}

fn completed_charge(
    fixture: &mut crate::isf_native_gpu::tests::Fixture,
    frame: NativeFrameId,
    kind: ChargeKind,
    capacity: u64,
) -> AllocationCharge {
    let mut plan = NativeAllocationPlan::new(frame);
    plan.add_new(kind, capacity).unwrap();
    let reservation = fixture
        .owner
        .reserve(fixture.owner.quote(&plan).unwrap())
        .unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let charge = txn.bind_allocation(kind, capacity).unwrap();
    let pending = txn.finish();
    await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    charge
}

#[test]
fn native_gpu_result_cpu_copy_rejects_real_success_of_another_phase() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let (copy, actual, other) = completed_cpu_copy(
        &mut fixture,
        &layers[0].lease,
        request(2),
        Some(GpuPhase::CompositionSnapshot),
    );
    let unrelated = other.unwrap();
    assert_eq!(unrelated.transaction(), actual.transaction());
    assert!(unrelated.validation().is_ok());
    assert!(matches!(
        copy.complete(unrelated, &mut fixture.owner, &|| false),
        Err(MaterializationFailure::Fault {
            fault: NativeGpuFault {
                kind: NativeGpuFaultKind::IdentityMismatch,
                ..
            },
            retirement: None,
        })
    ));
    drop(actual);
    drop(layers);
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_cpu_copy_rejects_another_successful_historical_transaction() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    for (copy_sequence, replacement_sequence) in [(2, 2), (3, 4)] {
        let (copy, actual, _) =
            completed_cpu_copy(&mut fixture, &layers[0].lease, request(copy_sequence), None);
        let unrelated = completed_empty_phase(
            &mut fixture,
            request(replacement_sequence),
            GpuPhase::HistoricalReplay,
        );
        assert!(unrelated.validation().is_ok());
        assert_ne!(unrelated.transaction(), actual.transaction());
        assert_ne!(unrelated.submission(), actual.submission());
        assert!(matches!(
            copy.complete(unrelated, &mut fixture.owner, &|| false),
            Err(MaterializationFailure::Fault {
                fault: NativeGpuFault {
                    kind: NativeGpuFaultKind::IdentityMismatch,
                    ..
                },
                retirement: None,
            })
        ));
        drop(actual);
        fixture.owner.poll_retirement().unwrap();
    }
    drop(layers);
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_cpu_copy_rejects_foreign_wrong_kind_and_undersized_charge() {
    let mut fixture = gpu();
    let mut other = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let pixels = 65 * 3 * 4;
    for (index, kind, capacity, foreign) in [
        (0, ChargeKind::CpuSnapshot, pixels, true),
        (1, ChargeKind::ActiveCpuInput, pixels, false),
        (2, ChargeKind::CpuSnapshot, pixels - 1, false),
    ] {
        let (mut copy, phase, _) =
            completed_cpu_copy(&mut fixture, &layers[0].lease, request(10 + index), None);
        let replacement = completed_charge(
            if foreign { &mut other } else { &mut fixture },
            request(20 + index),
            kind,
            capacity,
        );
        // Only this child test can corrupt the private affine ticket. All
        // replacement charges still come from real broker admissions.
        copy.cpu_charge = Some(replacement);
        assert!(matches!(
            copy.complete(phase, &mut fixture.owner, &|| false),
            Err(MaterializationFailure::Fault {
                fault: NativeGpuFault {
                    kind: NativeGpuFaultKind::IdentityMismatch,
                    ..
                },
                retirement: None,
            })
        ));
        fixture.owner.poll_retirement().unwrap();
        other.owner.poll_retirement().unwrap();
    }
    drop(layers);
    stop(&mut fixture.owner);
    stop(&mut other.owner);
}

#[test]
fn native_gpu_result_cpu_copy_cancelled_after_gpu_completion_does_not_publish_or_retain_map() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let (copy, phase, _) = completed_cpu_copy(&mut fixture, &layers[0].lease, request(2), None);
    let weak = Arc::downgrade(&copy.readback);
    assert!(matches!(
        copy.complete(phase, &mut fixture.owner, &|| true),
        Err(MaterializationFailure::Cancelled { retirement: None })
    ));
    assert!(weak.upgrade().is_none());
    fixture.owner.poll_retirement().unwrap();
    assert_eq!(
        fixture
            .owner
            .budget_snapshot()
            .for_kind(ChargeKind::SnapshotReadback)
            .active_bytes,
        0
    );
    assert_eq!(
        fixture
            .owner
            .budget_snapshot()
            .for_kind(ChargeKind::CpuSnapshot)
            .active_bytes,
        0
    );
    drop(layers);
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_readback_map_error_and_pending_cancel_clean_up_without_poisoning_owner() {
    let mut fixture = gpu();
    for valid_usage in [false, true] {
        let frame = request(if valid_usage { 2 } else { 1 });
        let mut plan = NativeAllocationPlan::new(frame);
        plan.add_new(ChargeKind::SnapshotReadback, 256).unwrap();
        let reservation = fixture
            .owner
            .reserve(fixture.owner.quote(&plan).unwrap())
            .unwrap();
        let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
        let mut phase = txn.begin_phase(GpuPhase::HistoricalReplay).unwrap();
        let charge = phase
            .bind_allocation(ChargeKind::SnapshotReadback, 256)
            .unwrap();
        let usage = if valid_usage {
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ
        } else {
            wgpu::BufferUsages::COPY_DST
        };
        let buffer = phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback cleanup callback-error regression"),
            size: 256,
            usage,
            mapped_at_creation: false,
        });
        let retained = Arc::new((buffer, charge));
        phase
            .retain_resource(Arc::clone(&retained), &[retained.1.id()])
            .unwrap();
        // Invalid usage supplies an actual map callback error under this phase's
        // validation scope. The valid case cancels before polling completion.
        let mut map = Some(NativeReadbackMap::start(&retained.0));
        let encoded = phase.finish(Vec::new());
        let receipt = txn.submit_phase(encoded).unwrap();
        let pending = txn.finish();
        if valid_usage {
            drop(map.take());
        }
        let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
        let phase = resolved.take_phase(receipt).unwrap();
        if valid_usage {
            assert!(phase.validation().is_ok());
        } else {
            assert_eq!(
                phase.validation().unwrap_err().kind,
                NativeGpuFaultKind::Validation
            );
            assert!(map.as_mut().unwrap().try_complete().is_err());
            map.as_mut().unwrap().unmap();
            drop(map.take());
        }
        drop(retained);
        // A second unmap would create an uncaptured NotMapped error here.
        fixture.owner.poll_retirement().unwrap();
        assert_eq!(
            fixture.owner.state(),
            crate::native_gpu_contract::GpuOwnerState::Active
        );
        assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
    }
    stop(&mut fixture.owner);
}

#[test]
fn native_gpu_result_read_view_samples_product_composition_shader_and_matches_cpu_pixels() {
    use crate::native_gpu_composition::{
        NativeCompositionLayer, NativeCompositionPlan, SelectedTextureKind,
        TEXTURE_COMPOSITION_SHADER,
    };
    use protocol::{
        Transform2D, VideoBlendMode, VideoColorAdjust, VideoFxAdjust, VideoSourceKind,
        VideoSourceSummary,
    };
    use wgpu::util::DeviceExt;

    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    release_idle_stack_bank(&mut fixture, &layers);
    let mut layers = layers.into_iter();
    let first = layers.next().unwrap();
    drop(layers);
    let identity = first.identity;
    let source = Arc::clone(&first.recipe.job().input);
    let weak = Arc::downgrade(&first.lease.allocation);
    let pixels = u64::from(identity.width) * u64::from(identity.height) * 4;
    let frame = request(2);
    let layer_plan = crate::CompositionLayerPlan {
        layer_id: identity.layer_id,
        label: "G resident sampling oracle".into(),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("fixture.mov".into()),
            name: None,
            codec: None,
            metadata: None,
        },
        blend_mode: VideoBlendMode::Normal,
        opacity: 1.0,
        position_ms: identity.pts_ms,
        transform: Transform2D::default(),
        color: VideoColorAdjust::default(),
        fx: VideoFxAdjust::default(),
    };
    let cpu_plan = crate::CompositionPlan {
        composition_id: 1,
        label: "CPU sampling oracle".into(),
        output_ids: vec![],
        master_opacity: 1.0,
        blackout: false,
        layers: vec![layer_plan.clone()],
    };
    let expected = crate::composite_frames_rgba8(
        &cpu_plan,
        std::slice::from_ref(&source),
        identity.width,
        identity.height,
    )
    .unwrap();
    let plan = NativeCompositionPlan::preflight(
        identity.context,
        frame,
        (identity.width, identity.height),
        [0; 4],
        &[NativeCompositionLayer {
            plan: &layer_plan,
            selected_entry: identity.entry,
            identity,
            kind: SelectedTextureKind::Validated,
        }],
        fixture.owner.capabilities().limits.max_texture_dimension_2d,
        u64::from(
            fixture
                .owner
                .capabilities()
                .limits
                .max_storage_buffer_binding_size,
        ),
    )
    .unwrap();
    let mut allocation_plan = NativeAllocationPlan::new(frame);
    allocation_plan.add_existing(first.lease.allocation_id());
    for (kind, bytes) in [
        (ChargeKind::CompositionBuffer, pixels),
        (ChargeKind::SnapshotReadback, pixels),
        (ChargeKind::MappingBuffer, plan.allocations.lookup_bytes),
        (
            ChargeKind::CachedEffectResource,
            plan.allocations.uniform_bytes,
        ),
        (ChargeKind::CpuSnapshot, pixels),
    ] {
        allocation_plan.add_new(kind, bytes).unwrap();
    }
    let reservation = fixture
        .owner
        .reserve(fixture.owner.quote(&allocation_plan).unwrap())
        .unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let read = first.lease.pin_for_read(&mut txn).unwrap();
    drop(first);
    assert!(
        weak.upgrade().is_some(),
        "the sampling read must retain the dropped lease's texture"
    );
    let mut phase = txn.begin_phase(GpuPhase::CompositionSnapshot).unwrap();
    let mut charges = Vec::new();
    for (kind, bytes) in [
        (ChargeKind::CompositionBuffer, pixels),
        (ChargeKind::SnapshotReadback, pixels),
        (ChargeKind::MappingBuffer, plan.allocations.lookup_bytes),
        (
            ChargeKind::CachedEffectResource,
            plan.allocations.uniform_bytes,
        ),
        (ChargeKind::CpuSnapshot, pixels),
    ] {
        charges.push(phase.bind_allocation(kind, bytes).unwrap());
    }
    struct SamplingBuffers {
        output: wgpu::Buffer,
        readback: wgpu::Buffer,
        lookup: wgpu::Buffer,
        params: wgpu::Buffer,
        charges: Vec<AllocationCharge>,
    }
    let lookup: Vec<u8> = plan.layers_bottom_first[0]
        .lookup
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    let resources = Arc::new(SamplingBuffers {
        output: phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("sampling oracle charged composition target"),
            size: pixels,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
        readback: phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("sampling oracle charged readback"),
            size: pixels,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
        lookup: phase
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sampling oracle identity lookup"),
                contents: &lookup,
                usage: wgpu::BufferUsages::STORAGE,
            }),
        params: phase
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sampling oracle product params"),
                contents: &plan.layers_bottom_first[0].params,
                usage: wgpu::BufferUsages::UNIFORM,
            }),
        charges,
    });
    let ids: Vec<_> = resources.charges.iter().map(AllocationCharge::id).collect();
    phase.retain_resource(Arc::clone(&resources), &ids).unwrap();
    let shader = phase
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("actual P texture composition shader"),
            source: wgpu::ShaderSource::Wgsl(TEXTURE_COMPOSITION_SHADER.into()),
        });
    let pipeline = phase
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("G read-use view sampling oracle"),
            layout: None,
            module: &shader,
            entry_point: Some("composite_layer"),
            compilation_options: Default::default(),
            cache: None,
        });
    let bind_group = phase
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("G read-use view consumed by P shader"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(read.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resources.output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: resources.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: resources.lookup.as_entire_binding(),
                },
            ],
        });
    let mut encoder = phase
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("resident texture composition sample"),
        });
    encoder.clear_buffer(&resources.output, 0, None);
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("product Normal opacity 1 sampling"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(identity.width.div_ceil(8), identity.height.div_ceil(8), 1);
    }
    encoder.copy_buffer_to_buffer(&resources.output, 0, &resources.readback, 0, pixels);
    let encoded = phase.finish(vec![encoder.finish()]);
    let receipt = txn.submit_phase(encoded).unwrap();
    let pending = txn.finish();
    let mut map = NativeReadbackMap::start(&resources.readback);
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    resolved
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    let started = Instant::now();
    while map.try_complete().unwrap().is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "sampling map did not complete"
        );
        fixture.owner.poll_retirement().unwrap();
        std::thread::yield_now();
    }
    let mapped = resources.readback.slice(..).get_mapped_range();
    let actual = mapped.to_vec();
    drop(mapped);
    map.unmap();
    drop(map);
    assert_eq!(actual.len(), expected.data.len());
    assert!(source.data.chunks_exact(4).any(|pixel| pixel[3] == 127));
    for (index, (actual, expected)) in actual.iter().zip(&expected.data).enumerate() {
        assert!(
            actual.abs_diff(*expected) <= 1,
            "product sampling CPU parity channel {index}: {actual} != {expected}"
        );
    }
    for (actual, source) in actual.chunks_exact(4).zip(source.data.chunks_exact(4)) {
        assert_eq!(
            actual[3], source[3],
            "Normal over transparent preserves alpha"
        );
        if source[3] == 255 {
            assert_eq!(actual, source, "opaque source is byte-exact");
        }
    }
    drop(actual);
    drop(bind_group);
    drop(pipeline);
    drop(shader);
    drop(resources);
    fixture.owner.poll_retirement().unwrap();
    assert!(
        weak.upgrade().is_some(),
        "completed GPU read handle still retains its texture"
    );
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, pixels);
    drop(read);
    fixture.owner.poll_retirement().unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
    stop(&mut fixture.owner);
}
