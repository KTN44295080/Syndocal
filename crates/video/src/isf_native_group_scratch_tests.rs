use super::*;
use crate::isf_native_gpu::tests::{
    gpu, identity_jobs, jobs, request, shader, stop, validated, Fixture, IDENTITY,
};
use crate::native_gpu_composition::{
    tests::{identity_batch, layer, validate_isf},
    NativeCompositionGroup, NativeCompositionLayer, NativeCompositionPlan, NativeGpuCompositor,
    ResidentGroupEffect, SelectedLayerInput, SelectedTextureKind,
};
use crate::native_gpu_result::{CpuOutcomeProvenance, NativeIsfCompletion};
use protocol::{Transform2D, VideoBlendMode, VideoLayerGroupId, VideoLayerGroupSummary};

const MULTIPASS: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{},{}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#;

#[test]
fn isf_native_group_scratch_cold_extent_and_stateful_fallback_preserve_admission() {
    let mut fixture = gpu();
    let cold = fixture
        .runtime
        .prepare_native_inputs(
            request(1),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(cold.group_fx_scratch((8, 4)).is_none());
    fixture
        .owner
        .quote(cold.allocation_plan().unwrap())
        .unwrap();
    drop(cold);
    let (warm, receipt) = validated(&mut fixture, 1, identity_jobs(8, 4));
    drop(receipt);
    let prepared = fixture
        .runtime
        .prepare_native_inputs(
            request(2),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(prepared.group_fx_scratch((16, 8)).is_none());
    fixture
        .owner
        .quote(prepared.allocation_plan().unwrap())
        .unwrap();
    let scratch = prepared.group_fx_scratch((8, 4)).unwrap();
    assert!(scratch.matches(fixture.owner.context().id(), request(2), (8, 4)));
    assert!(!scratch.matches(fixture.owner.context().id(), request(3), (8, 4)));
    assert!(warm
        .iter()
        .all(|image| !scratch.ids().contains(&image.lease.allocation_id())));
    let before = fixture.owner.budget_snapshot();
    let mut aliases = NativeAllocationPlan::new(request(2));
    for _ in 0..2 {
        for id in scratch.ids() {
            aliases.add_existing(id);
        }
    }
    assert_eq!(aliases.requested_bytes(), 0);
    fixture.owner.quote(&aliases).unwrap();
    assert_eq!(fixture.owner.budget_snapshot(), before);
    drop((scratch, prepared, warm));
    fixture.owner.release_resource_pools();
    fixture.runtime.clear_native_stack_pool();
    fixture.owner.poll_retirement().unwrap();
    let (stateful, receipt) = validated(&mut fixture, 3, jobs(8, 4, &[shader(MULTIPASS)]));
    drop(receipt);
    let prepared = fixture
        .runtime
        .prepare_native_inputs(
            request(4),
            jobs(8, 4, &[shader(MULTIPASS)]),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(prepared
        .pooled
        .iter()
        .all(|selection| selection.scratch.is_some()));
    assert!(prepared.group_fx_scratch((8, 4)).is_none());
    fixture
        .owner
        .quote(prepared.allocation_plan().unwrap())
        .unwrap();
    drop((prepared, stateful));
    stop(&mut fixture.owner);
}

fn group_plan(
    fixture: &Fixture,
    inputs: &PreparedNativeInputs,
    extent: (u32, u32),
) -> NativeCompositionPlan {
    let identities = inputs.prospective_identities(&fixture.runtime).unwrap();
    let plans = (1..=3)
        .map(|id| layer(id, VideoBlendMode::Normal, 1.0, Transform2D::default()))
        .collect::<Vec<_>>();
    let selected = plans
        .iter()
        .zip(identities.iter())
        .map(|(plan, identity)| NativeCompositionLayer {
            plan,
            selected_entry: identity.entry,
            identity: *identity,
            kind: SelectedTextureKind::InThisTransaction,
        })
        .collect::<Vec<_>>();
    let group = VideoLayerGroupSummary {
        id: VideoLayerGroupId(91),
        label: "scratch fallback".into(),
        composition_id: 1,
        layer_ids: vec![1, 2, 3],
        enabled: true,
        opacity: 1.0,
        blend_mode: VideoBlendMode::Normal,
        solo: false,
        bypassed: false,
        transform: None,
    };
    let effect = ResidentGroupEffect {
        stages: jobs(8, 4, &[shader(IDENTITY)])[0].chain.stages.clone(),
        date: [2026.0, 11.0, 3.0, 1.0],
    };
    let limits = &fixture.owner.capabilities().limits;
    NativeCompositionPlan::preflight_with_groups(
        fixture.owner.context().id(),
        inputs.frame,
        extent,
        [0; 4],
        &selected,
        &[NativeCompositionGroup {
            group: &group,
            fx: Some(Arc::new(effect)),
        }],
        limits.max_texture_dimension_2d,
        u64::from(limits.max_storage_buffer_binding_size),
    )
    .unwrap()
}

#[test]
fn isf_native_group_scratch_unsubmitted_plan_releases_before_cpu_pool_handover() {
    let mut fixture = gpu();
    let (warm, receipt) = validated(&mut fixture, 1, identity_jobs(8, 4));
    drop((warm, receipt));
    let inputs = fixture
        .runtime
        .prepare_native_inputs(
            request(2),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let scratch = inputs.group_fx_scratch((8, 4)).unwrap();
    let weak = Arc::downgrade(&scratch);
    let mut plan = group_plan(&fixture, &inputs, (8, 4));
    plan.reuse_group_fx_scratch(Some(scratch)).unwrap();
    // CPU preparation drops its input selections; there is no GPU submission.
    drop(inputs);
    fixture.owner.release_resource_pools();
    fixture.runtime.clear_native_stack_pool();
    fixture.owner.poll_retirement().unwrap();
    assert!(
        weak.upgrade().is_some(),
        "unsubmitted plan still pins scratch"
    );
    assert!(fixture.owner.budget_snapshot().allocated_bytes > 0);
    // The quote/admit/reserve decline branches must drop the plan before the
    // canonical CPU route quotes its handover envelope.
    drop(plan);
    fixture.owner.poll_retirement().unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, 0);
    stop(&mut fixture.owner);
}

/// Real G/P encode and packed readback. The opaque uniform top layer makes
/// output bytes independently predictable even when the output extent differs.
fn grouped_tick(
    fixture: &mut Fixture,
    compositor: &NativeGpuCompositor,
    sequence: u64,
    extent: (u32, u32),
    expected_borrow: bool,
) -> Vec<u8> {
    let frame = request(sequence);
    let batch = identity_batch((8, 4), 3, |id, _| [id as u8 * 30, 71, 173, 255]);
    let inputs = fixture
        .runtime
        .prepare_native_inputs(frame, batch, NativeBatchRestrictions::default())
        .unwrap();
    let mut plan = group_plan(fixture, &inputs, extent);
    let own_requested = plan.allocation_plan().requested_bytes();
    let scratch = inputs.group_fx_scratch(extent);
    assert_eq!(scratch.is_some(), expected_borrow);
    plan.reuse_group_fx_scratch(scratch).unwrap();
    assert_eq!(
        own_requested - plan.allocation_plan().requested_bytes(),
        if expected_borrow {
            u64::from(extent.0) * u64::from(extent.1) * 8
        } else {
            0
        }
    );
    let mut allocation = inputs.allocation_plan().unwrap().clone();
    allocation.merge(plan.allocation_plan()).unwrap();
    let quote = fixture.owner.quote(&allocation).unwrap();
    let batch = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        _ => panic!("compatible or fallback grouped frame must admit"),
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let composition = compositor.reserve(&mut txn, &plan).unwrap();
    let isf = fixture
        .runtime
        .encode_native_batch(&mut txn, batch)
        .unwrap();
    let candidates = isf
        .candidates
        .iter()
        .map(SelectedLayerInput::InThisTransaction)
        .collect::<Vec<_>>();
    let encoded = compositor
        .encode_composition_and_snapshot(
            &mut txn,
            &plan,
            &candidates,
            composition,
            Some(&mut fixture.runtime),
        )
        .unwrap();
    drop(candidates);
    let pending = txn.finish();
    drop(plan);
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let layers = validate_isf(fixture, isf, &mut resolved);
    let parts = encoded.into_parts();
    resolved
        .take_phase(parts.receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    parts
        .readback
        .buffer()
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    fixture
        .owner
        .context()
        .device()
        .poll(wgpu::PollType::wait())
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let bytes = parts
        .readback
        .buffer()
        .slice(..)
        .get_mapped_range()
        .to_vec();
    parts.readback.buffer().unmap();
    drop((
        parts.output,
        parts.readback,
        parts.cpu_charge,
        layers,
        resolved,
    ));
    bytes
}

#[test]
fn isf_native_group_scratch_incompatible_extent_keeps_own_targets_and_resident_pixels() {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    fixture
        .owner
        .configure_stack_pool_shape([Some((8, 4, VideoPixelFormat::Rgba8, 3)); 3]);
    // Cold compatible frame uses old owned pair; steady compatible frame aliases.
    let cold = grouped_tick(&mut fixture, &compositor, 1, (8, 4), false);
    let borrowed = grouped_tick(&mut fixture, &compositor, 2, (8, 4), true);
    assert_eq!(cold, [90, 71, 173, 255].repeat(8 * 4));
    assert_eq!(borrowed, cold, "reuse must preserve every packed RGBA byte");
    for sequence in 3..=5 {
        let fallback = grouped_tick(&mut fixture, &compositor, sequence, (16, 8), false);
        assert_eq!(fallback, [90, 71, 173, 255].repeat(16 * 8));
    }
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_group_scratch_failure_pins_dependent_submission_through_fixed_clock_recovery() {
    let mut fixture = gpu();
    let (warm, receipt) = validated(&mut fixture, 1, identity_jobs(8, 4));
    drop((warm, receipt));
    // A distinct shader fingerprint prevents warm identity pipelines masking
    // the malformed shader and guarantees real stage-local recovery.
    const DIFFERENT: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage) + vec4(0.0); }"#;
    let mut bad = shader(DIFFERENT);
    Arc::make_mut(&mut bad.1).wgsl = "not valid WGSL".into();
    let batch = jobs(8, 4, &[shader(IDENTITY), bad, shader(IDENTITY)]);
    let original = batch
        .iter()
        .map(|job| job.input.data.clone())
        .collect::<Vec<_>>();
    let inputs = fixture
        .runtime
        .prepare_native_inputs(request(2), batch, NativeBatchRestrictions::default())
        .unwrap();
    let scratch = inputs.group_fx_scratch((8, 4)).unwrap();
    let ids = scratch.ids();
    let weak = Arc::downgrade(&scratch);
    let mut allocation = inputs.allocation_plan().unwrap().clone();
    for id in ids {
        allocation.add_existing(id);
    }
    let quote = fixture.owner.quote(&allocation).unwrap();
    let batch = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        _ => panic!("failure fixture must reach admitted execution"),
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(request(2), reservation).unwrap();
    let mut encoded = fixture
        .runtime
        .encode_native_batch(&mut txn, batch)
        .unwrap();
    let mut phase = txn.begin_phase(GpuPhase::CompositionSnapshot).unwrap();
    phase.retain_resource(Arc::clone(&scratch), &ids).unwrap();
    let mut commands = phase.device().create_command_encoder(&Default::default());
    let images = scratch.owner.resources.group_scratch_images();
    {
        let _pass = commands.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("borrowed scratch dependent failure write"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &images[1].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    phase.record_failure(
        NativeGpuFaultKind::Validation,
        "dependent composition failure",
    );
    let commands = phase.finish(vec![commands.finish()]);
    let failed = txn.submit_phase(commands).unwrap();
    drop(scratch);
    let pending = txn.finish();
    assert!(
        weak.upgrade().is_some(),
        "dependent phase owns its borrowed pair"
    );
    let blocked = fixture
        .runtime
        .prepare_native_inputs(
            request(3),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(
        matches!(
            blocked.allocation_plan(),
            Err(NativeGpuDecline::BudgetExceeded { .. })
        ),
        "pending dependency cannot allow pool scratch checkout"
    );
    drop(blocked);
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    assert_eq!(
        resolved
            .take_phase(failed)
            .unwrap()
            .into_validation()
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::Validation
    );
    assert!(
        weak.upgrade().is_none(),
        "only verified whole-transaction drain releases loan"
    );
    let receipt = encoded.pending.take().unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    assert!(phase.validation().is_err());
    match fixture
        .runtime
        .complete_native_batch(encoded, phase, &mut fixture.owner, &|| false)
    {
        NativeIsfCompletion::RecoveredCpu { layers } => {
            assert_eq!(layers.len(), 3);
            for (index, layer) in layers.iter().enumerate() {
                assert_eq!(layer.provenance, CpuOutcomeProvenance::FixedClockRecovery);
                assert_eq!(layer.frame.data, original[index]);
                assert_eq!(layer.faults.len(), 1);
            }
        }
        _ => panic!("admitted failed batch must recover after dependencies drain"),
    }
    assert_eq!(
        fixture.runtime.native_frame_index(),
        6,
        "recovery consumes no new logical clocks"
    );
    // Failed validation invalidates G's cache epoch. The drained pair must not
    // be resurrected; a new cold bank is quoted normally, then becomes reusable.
    let cold = fixture
        .runtime
        .prepare_native_inputs(
            request(4),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(cold
        .pooled
        .iter()
        .all(|selection| selection.scratch.is_none()));
    assert!(cold.group_fx_scratch((8, 4)).is_none());
    fixture
        .owner
        .quote(cold.allocation_plan().unwrap())
        .unwrap();
    drop(cold);
    let (next, receipt) = validated(&mut fixture, 4, identity_jobs(8, 4));
    drop((next, receipt));
    let reusable = fixture
        .runtime
        .prepare_native_inputs(
            request(5),
            identity_jobs(8, 4),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let next_scratch = reusable.group_fx_scratch((8, 4)).unwrap();
    assert!(next_scratch.ids().iter().all(|id| !ids.contains(id)));
    drop((next_scratch, reusable, resolved));
    stop(&mut fixture.owner);
}
