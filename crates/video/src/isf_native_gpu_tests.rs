use super::*;
use crate::isf_chain_execution::PreparedStage;
use crate::isf_runtime::{isf_effect_from_prepared, PreparedIsfShader};
use crate::native_gpu_context::{NativeGpuFactory, NativeGpuRequirements, RetirementProgress};
use crate::{
    prepare_isf_shader, ResolvedVideoEffectChain, ResolvedVideoEffectStage,
    VideoEffectFaultContext, VideoEffectStageFault,
};
use std::borrow::Cow;

pub(crate) const IDENTITY: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#;
const INCREMENT: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.r+1.0/255.0,c.gba); }"#;
const CLOCK_RED: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(float(FRAMEINDEX)/255.0,c.gba); }"#;
const CLOCK_GREEN: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.r,float(FRAMEINDEX)/255.0,c.ba); }"#;
const TIME_DATE: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.rg,(TIME+TIMEDELTA)/255.0,DATE.w/255.0); }"#;
const MULTIPASS: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{},{}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#;

pub(crate) type TestShader = (&'static str, Arc<PreparedIsfShader>);
pub(crate) struct Fixture {
    pub owner: NativeGpuOwner,
    pub runtime: IsfGpuRuntime,
}
pub(crate) fn gpu() -> Fixture {
    let factory = NativeGpuFactory::new();
    let mut owner = factory
        .create_headless(101, NativeGpuRequirements::default())
        .expect("native ISF contract tests require an actual headless adapter");
    let runtime = IsfGpuRuntime::with_native_context(&mut owner).unwrap();
    Fixture { owner, runtime }
}
pub(crate) fn request(sequence: u64) -> NativeFrameId {
    NativeFrameId {
        output_id: 101,
        project_epoch: 7,
        request_sequence: sequence,
        geometry_generation: 1,
    }
}
pub(crate) fn shader(source: &'static str) -> TestShader {
    (source, Arc::new(prepare_isf_shader(source).unwrap()))
}
pub(crate) fn jobs(
    width: u32,
    height: u32,
    specs: &[TestShader],
) -> StackBatch<PreparedNativeLayer> {
    StackBatch::from_fn(3, |index| {
        let layer = index as u64 + 1;
        let input_key = protocol::VideoRenderInputKey {
            project_render_epoch: 7,
            input_id: protocol::VideoRenderInputId(layer + 1000),
        };
        let scope = protocol::VideoEffectScope::Layer { layer_id: layer };
        let chain_id = Some(protocol::VideoEffectChainId(layer + 100));
        let source_stages = specs
            .iter()
            .enumerate()
            .map(|(stage, (source, prepared))| {
                let label = format!("layer-{layer}-stage-{stage}");
                ResolvedVideoEffectStage {
                    stage_id: Some(protocol::VideoEffectStageId(layer * 10 + stage as u64)),
                    effect_id: Some(protocol::VideoEffectId(layer * 100 + stage as u64)),
                    enabled: true,
                    effect: isf_effect_from_prepared(label.clone(), *source, None, prepared),
                    label,
                }
            })
            .collect::<Vec<_>>();
        let stages = source_stages
            .iter()
            .zip(specs)
            .enumerate()
            .map(|(stage_index, (authored, (_, prepared)))| PreparedStage {
                shader: Arc::clone(prepared),
                controls: prepared
                    .controls
                    .iter()
                    .map(|control| control.default)
                    .collect(),
                images: Vec::new(),
                fault_context: VideoEffectFaultContext {
                    scope: scope.clone(),
                    chain_id,
                    stage_id: authored.stage_id,
                    effect_id: authored.effect_id,
                    render_input_key: Some(input_key),
                    stage_index: Some(stage_index),
                    stage_label: None,
                },
            })
            .collect();
        let keys = source_stages
            .iter()
            .enumerate()
            .map(|(stage, authored)| IsfInstanceKey {
                namespace: (7, Some(101), false),
                scope: Some(scope.clone()),
                input: Some(input_key),
                chain_id,
                stage_id: authored.stage_id,
                effect_id: authored.effect_id,
                legacy_stage_index: stage,
                layer_id: layer,
            })
            .collect();
        PreparedNativeLayer {
            input: Arc::new(VideoFrame {
                layer_id: layer,
                width,
                height,
                pts_ms: layer * 1000,
                duration_ms: 1000 + layer * 1000,
                format: VideoPixelFormat::Rgba8,
                data: (0..width * height)
                    .flat_map(|pixel| {
                        [
                            11 + layer as u8 * 37,
                            pixel as u8,
                            173,
                            if pixel % 5 == 0 { 127 } else { 255 },
                        ]
                    })
                    .collect(),
            }),
            chain: PreparedChain {
                source: Cow::Owned(ResolvedVideoEffectChain {
                    chain_id,
                    scope,
                    bypassed: false,
                    stages: source_stages,
                }),
                stages,
                keys,
                faults: Vec::new(),
            },
            entry: HistoryEntryId {
                input: input_key,
                queue_epoch: 9,
                sequence: layer,
            },
            source_generation: 13,
            content_sequence: layer * 17,
            date: [2026.0, 10.0, 26.0, 64.0 + index as f32],
        }
    })
}
pub(crate) fn identity_jobs(width: u32, height: u32) -> StackBatch<PreparedNativeLayer> {
    let identity = shader(IDENTITY);
    jobs(
        width,
        height,
        &[identity.clone(), identity.clone(), identity],
    )
}
pub(crate) fn encode(
    fixture: &mut Fixture,
    sequence: u64,
    jobs: StackBatch<PreparedNativeLayer>,
) -> (EncodedIsfBatch, PendingGpuTransaction) {
    let frame = request(sequence);
    let first_clock = fixture.runtime.native_frame_index();
    let expected = jobs
        .iter()
        .enumerate()
        .map(|(index, job)| LayerResultIdentity {
            context: fixture.owner.context().id(),
            frame,
            entry: job.entry,
            layer_id: job.input.layer_id,
            source_generation: job.source_generation,
            content_sequence: job.content_sequence,
            pts_ms: job.input.pts_ms,
            duration_ms: job.input.duration_ms,
            width: job.input.width,
            height: job.input.height,
            format: job.input.format,
            origin: LayerOrigin::Executed(FrozenChainClocks {
                frame_index: first_clock.wrapping_add(index as u32),
                time_seconds_bits: (job.input.pts_ms as f32 / 1000.0).to_bits(),
                time_delta_seconds_bits: (job.input.duration_ms as f32 / 1000.0).to_bits(),
                date_bits: job.date.map(f32::to_bits),
            }),
        })
        .collect::<Vec<_>>();
    let inputs = fixture
        .runtime
        .prepare_native_inputs(frame, jobs, NativeBatchRestrictions::default())
        .unwrap();
    let quote = fixture
        .owner
        .quote(inputs.allocation_plan().unwrap())
        .unwrap();
    let admitted = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        NativeBatchAdmission::Declined { reason, .. } => {
            panic!("ordinary fixture declined: {reason:?}")
        }
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let encoded = fixture
        .runtime
        .encode_native_batch(&mut txn, admitted)
        .unwrap();
    for (candidate, expected) in encoded.candidates.iter().zip(&expected) {
        assert_eq!(
            candidate.identity(),
            expected,
            "provisional identity must preserve its actual input and reserved clocks"
        );
    }
    (encoded, txn.finish())
}
pub(crate) fn complete(
    fixture: &mut Fixture,
    mut encoded: EncodedIsfBatch,
    pending: PendingGpuTransaction,
) -> NativeIsfCompletion {
    let receipt = encoded.pending.take().unwrap();
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    fixture
        .runtime
        .complete_native_batch(encoded, phase, &mut fixture.owner, &|| false)
}
pub(crate) fn validated(
    fixture: &mut Fixture,
    sequence: u64,
    jobs: StackBatch<PreparedNativeLayer>,
) -> (StackBatch<ValidatedLayerResult>, IsfValidatedReceipt) {
    let (encoded, pending) = encode(fixture, sequence, jobs);
    match complete(fixture, encoded, pending) {
        NativeIsfCompletion::Validated { layers, receipt } => (layers, receipt),
        NativeIsfCompletion::RecoveredCpu { .. } => {
            panic!("ordinary fixture unexpectedly recovered")
        }
        NativeIsfCompletion::Cancelled { .. } => panic!("ordinary fixture unexpectedly cancelled"),
        NativeIsfCompletion::Failed { fault, retirement } => {
            panic!("ordinary fixture failed: {fault:?}; retirement: {retirement:?}")
        }
    }
}
pub(crate) fn stop(owner: &mut NativeGpuOwner) {
    let token = owner.begin_stop();
    assert_eq!(token.context(), owner.context().id());
    let started = Instant::now();
    loop {
        match owner.poll_retirement().unwrap() {
            RetirementProgress::Reaped => break,
            RetirementProgress::Pending { .. } if started.elapsed() < Duration::from_secs(5) => {
                std::thread::yield_now()
            }
            pending => panic!("test-owned resources were not reaped: {pending:?}"),
        }
    }
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(owner.budget_snapshot().reservation_count, 0);
}
fn reference(
    runtime: &mut IsfGpuRuntime,
    jobs: &[PreparedNativeLayer],
) -> Vec<(VideoFrame, Vec<VideoEffectStageFault>)> {
    jobs.iter()
        .map(|job| {
            let stages = job.stages();
            let (frame, failures) = runtime.apply_stack_stage_local_with_inputs_and_keys(
                &job.input,
                &stages,
                job.input.pts_ms as f32 / 1000.0,
                job.input.duration_ms as f32 / 1000.0,
                job.date,
                &job.chain.keys,
            );
            (frame, job.chain.clone().finish_faults(failures))
        })
        .collect()
}
fn submission_probe(owner: &mut NativeGpuOwner, sequence: u64) -> u64 {
    let frame = request(sequence);
    let plan = NativeAllocationPlan::new(frame);
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame, reservation).unwrap();
    let phase = txn.begin_phase(GpuPhase::Initialize).unwrap();
    let commands = phase.finish(Vec::new());
    let receipt = txn.submit_phase(commands).unwrap();
    let sequence = receipt.submission().sequence;
    let pending = txn.finish();
    let mut resolved = await_transaction(owner, pending, &|| false).unwrap();
    resolved
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    sequence
}

#[test]
fn isf_native_gpu_three_by_three_matches_cpu_pixels_and_input_ownership() {
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let spec = shader(INCREMENT);
    for (round, (width, height)) in [(65, 3), (128, 5)].into_iter().enumerate() {
        let prepared = jobs(width, height, &[spec.clone(), spec.clone(), spec.clone()]);
        let original: Vec<_> = prepared.iter().map(|job| Arc::clone(&job.input)).collect();
        let expected = reference(&mut cpu, &prepared);
        let before = submission_probe(&mut fixture.owner, 100 + round as u64);
        let (layers, receipt) = validated(&mut fixture, round as u64 + 1, prepared);
        assert_eq!(layers.len(), 3);
        assert_eq!(receipt.phase().phase(), GpuPhase::IsfBatch);
        assert_eq!(
            receipt.phase().submission().sequence,
            before + 1,
            "Batch3 uses one submission"
        );
        assert_eq!(fixture.runtime.native_frame_index(), (round as u32 + 1) * 3);
        for ((layer, (expected, faults)), original) in layers.iter().zip(&expected).zip(&original) {
            assert!(faults.is_empty());
            let actual = NativeGpuMaterializer::new(&mut fixture.runtime)
                .materialize_cpu(
                    CpuMaterializationRequest::ValidatedGpu {
                        entry: layer.identity.entry,
                        lease: &layer.lease,
                    },
                    &mut fixture.owner,
                    &|| false,
                )
                .unwrap();
            assert_eq!(actual.frame(), expected);
            assert_eq!(actual.descriptor().layer, layer.identity);
            assert_eq!(actual.descriptor().allocation, layer.lease.allocation_id());
            assert_eq!(actual.frame().data[0], original.data[0] + 3);
            assert_eq!(layer.identity.source_generation, 13);
            assert_eq!(layer.identity.content_sequence, original.layer_id * 17);
            assert_eq!(
                original.data[0],
                11 + original.layer_id as u8 * 37,
                "decoder input is immutable"
            );
        }
        drop(layers);
        fixture.owner.poll_retirement().unwrap();
        assert!(original.iter().all(|input| Arc::strong_count(input) == 1));
    }
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_frozen_recipe_replays_exact_clock_time_date_without_new_index() {
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let specs = [shader(CLOCK_RED), shader(CLOCK_GREEN), shader(TIME_DATE)];
    let prepared = jobs(65, 3, &specs);
    let expected = reference(&mut cpu, &prepared);
    let (first, _) = validated(&mut fixture, 1, prepared);
    for (index, layer) in first.iter().enumerate() {
        assert_eq!(layer.recipe.clocks().frame_index, index as u32);
        assert_eq!(
            layer.recipe.clocks().time_seconds_bits,
            (index as f32 + 1.0).to_bits()
        );
        assert_eq!(
            layer.recipe.clocks().date_bits[3],
            (64.0 + index as f32).to_bits()
        );
        assert_eq!(layer.recipe.pinned_pipeline_count(), 3);
    }
    let (next, _) = validated(&mut fixture, 2, jobs(65, 3, &specs));
    assert_eq!(fixture.runtime.native_frame_index(), 6);
    drop(next);
    for (layer, (expected, _)) in first.iter().zip(&expected) {
        let replay = NativeGpuMaterializer::new(&mut fixture.runtime)
            .materialize_cpu(
                CpuMaterializationRequest::FrozenReplay {
                    entry: layer.identity.entry,
                    recipe: &layer.recipe,
                },
                &mut fixture.owner,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            replay.frame(),
            expected,
            "historical pixels must retain their original clocks"
        );
        assert_eq!(replay.descriptor().layer, layer.identity);
        assert_eq!(
            fixture.runtime.native_frame_index(),
            6,
            "replay consumes no logical index"
        );
        // C1 receives an affine GPU lease and the exact validated replay phase.
        // A historical layer keeps its original identity while the replay is
        // submitted as part of a later request.
        let current_frame = request(100 + layer.identity.layer_id);
        let plan = fixture
            .runtime
            .replay_allocation_plan(current_frame, &layer.recipe)
            .unwrap();
        let quote = fixture.owner.quote(&plan).unwrap();
        let reservation = fixture.owner.reserve(quote).unwrap();
        let mut txn = fixture
            .owner
            .begin_frame(current_frame, reservation)
            .unwrap();
        let mut candidate = fixture
            .runtime
            .encode_historical_replay(&mut txn, layer.identity.entry, &layer.recipe)
            .unwrap();
        assert_eq!(candidate.identity(), &layer.identity);
        let pending_receipt = candidate.take_replay_receipt().unwrap();
        let submission = pending_receipt.submission();
        let pending = txn.finish();
        let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
        let phase = resolved.take_phase(pending_receipt).unwrap();
        let materialized = fixture
            .runtime
            .complete_historical_replay(candidate, phase, &mut fixture.owner)
            .unwrap();
        let descriptor = materialized.descriptor();
        assert_eq!(descriptor.layer, layer.identity);
        assert_eq!(descriptor.completed_submission, submission);
        assert_ne!(
            descriptor.allocation,
            layer.lease.allocation_id(),
            "replay creates a new physical image for the same history entry"
        );
        let (lease, phase) = materialized.into_parts();
        assert_eq!(lease.identity(), &descriptor.layer);
        assert_eq!(lease.allocation_id(), descriptor.allocation);
        assert_eq!(phase.submission(), descriptor.completed_submission);
        assert_eq!(phase.context(), descriptor.layer.context);
        assert_eq!(phase.frame(), current_frame);
        assert_eq!(phase.phase(), GpuPhase::HistoricalReplay);
        assert!(phase.validation().is_ok());
        let replay_pixels = NativeGpuMaterializer::new(&mut fixture.runtime)
            .materialize_cpu(
                CpuMaterializationRequest::ValidatedGpu {
                    entry: layer.identity.entry,
                    lease: &lease,
                },
                &mut fixture.owner,
                &|| false,
            )
            .unwrap();
        assert_eq!(replay_pixels.frame(), expected);
        assert_eq!(replay_pixels.descriptor().layer, descriptor.layer);
        assert_eq!(replay_pixels.descriptor().allocation, descriptor.allocation);
        assert_eq!(
            fixture.runtime.native_frame_index(),
            6,
            "explicit historical completion also consumes no current clock"
        );
    }
    drop(first);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_middle_stage_failure_preserves_neighbors_fault_identity_and_clock() {
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let increment = shader(INCREMENT);
    let mut prepared = jobs(3, 2, &[increment.clone(), increment.clone(), increment]);
    Arc::make_mut(&mut prepared[1].chain.stages[1].shader).wgsl = "not valid WGSL".into();
    let expected = reference(&mut cpu, &prepared);
    let (encoded, pending) = encode(&mut fixture, 1, prepared);
    match complete(&mut fixture, encoded, pending) {
        NativeIsfCompletion::RecoveredCpu { layers } => {
            assert_eq!(layers.len(), 3);
            for (actual, (frame, faults)) in layers.iter().zip(&expected) {
                assert_eq!(&actual.frame, frame);
                assert_eq!(&actual.faults, faults);
                assert_eq!(actual.provenance, CpuOutcomeProvenance::FixedClockRecovery);
                assert_eq!(
                    actual.charge.capacity(),
                    actual.frame.data.capacity() as u64
                );
            }
            assert!(layers[0].faults.is_empty() && layers[2].faults.is_empty());
            assert_eq!(layers[1].faults.len(), 1);
            assert_eq!(layers[1].faults[0].stage_index, Some(1));
            assert_eq!(
                layers[1].faults[0].stage_id,
                Some(protocol::VideoEffectStageId(21))
            );
            assert_eq!(
                layers[1].faults[0].effect_id,
                Some(protocol::VideoEffectId(201))
            );
            assert_eq!(
                layers[1].faults[0].stage_label.as_deref(),
                Some("layer-2-stage-1")
            );
        }
        NativeIsfCompletion::Failed { fault, retirement } => {
            panic!("fixed-clock recovery failed: {fault:?}; retirement: {retirement:?}")
        }
        _ => panic!("failed batch must return its canonical CPU recovery"),
    }
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_all_failed_stages_return_original_pixels_and_three_chain_clocks() {
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let mut bad = shader(IDENTITY);
    Arc::make_mut(&mut bad.1).wgsl = "not valid WGSL".into();
    let prepared = jobs(3, 2, &[bad.clone(), bad.clone(), bad]);
    let original: Vec<_> = prepared.iter().map(|job| Arc::clone(&job.input)).collect();
    let expected = reference(&mut cpu, &prepared);
    let (encoded, pending) = encode(&mut fixture, 1, prepared);
    match complete(&mut fixture, encoded, pending) {
        NativeIsfCompletion::RecoveredCpu { layers } => {
            for (((actual, source), (frame, faults)), index) in
                layers.iter().zip(&original).zip(&expected).zip(0..)
            {
                assert_eq!(&actual.frame, source.as_ref());
                assert_eq!(&actual.frame, frame);
                assert_eq!(&actual.faults, faults);
                assert_eq!(
                    actual
                        .faults
                        .iter()
                        .map(|fault| fault.stage_index)
                        .collect::<Vec<_>>(),
                    [Some(0), Some(1), Some(2)]
                );
                assert!(
                    matches!(actual.identity.origin, LayerOrigin::Executed(clocks) if clocks.frame_index == index)
                );
            }
        }
        NativeIsfCompletion::Failed { fault, retirement } => {
            panic!("all-failed recovery failed: {fault:?}; retirement: {retirement:?}")
        }
        _ => panic!("all-failed work must preserve the original CPU result"),
    }
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    assert!(original.iter().all(|input| Arc::strong_count(input) == 1));
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_unsupported_returns_same_prepared_inputs_without_work() {
    let mut fixture = gpu();
    for case in 0..7 {
        let mut prepared = identity_jobs(3, 2);
        let mut restrictions = NativeBatchRestrictions::default();
        let expected = match case {
            0 => {
                restrictions.callback = true;
                UnsupportedNativeFx::Callback
            }
            1 => {
                restrictions.executing_clip_or_transition = true;
                UnsupportedNativeFx::ClipOrTransitionExecution
            }
            2 => {
                restrictions.inter_layer_dependency = true;
                UnsupportedNativeFx::InterLayerDependency
            }
            3 => {
                prepared = jobs(3, 2, &[shader(MULTIPASS)]);
                UnsupportedNativeFx::Multipass
            }
            4 => {
                prepared[0].chain.stages.clear();
                prepared[0].chain.keys.clear();
                UnsupportedNativeFx::EmptyChain
            }
            5 => {
                prepared[0].chain.source.to_mut().scope =
                    protocol::VideoEffectScope::Output { output_id: 101 };
                UnsupportedNativeFx::NonLayerScope
            }
            _ => {
                prepared[0].date[0] = f32::NAN;
                UnsupportedNativeFx::AuxiliaryResource
            }
        };
        let inputs: Vec<_> = prepared.iter().map(|job| Arc::clone(&job.input)).collect();
        let before = submission_probe(&mut fixture.owner, 100 + case * 2);
        let frame = request(10 + case);
        let prepared = fixture
            .runtime
            .prepare_native_inputs(frame, prepared, restrictions)
            .unwrap();
        let empty = NativeAllocationPlan::new(frame);
        let quote = fixture.owner.quote(&empty).unwrap();
        match fixture.runtime.admit_native_batch(prepared, &quote) {
            NativeBatchAdmission::Declined { reason, prepared } => {
                assert_eq!(
                    reason,
                    NativeGpuDecline::UnsupportedChain { reason: expected }
                );
                let returned = prepared.into_layers();
                assert!(returned
                    .iter()
                    .zip(&inputs)
                    .all(|(job, source)| Arc::ptr_eq(&job.input, source)));
            }
            NativeBatchAdmission::Accepted(_) => panic!("unsupported fixture was admitted"),
        }
        assert_eq!(fixture.runtime.native_frame_index(), 0);
        assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
        let after = submission_probe(&mut fixture.owner, 101 + case * 2);
        assert_eq!(after, before + 1, "decline issued no hidden submission");
        assert!(inputs.iter().all(|input| Arc::strong_count(input) == 1));
    }
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_budget_pressure_declines_before_clock_upload_or_allocation() {
    let mut fixture = gpu();
    let mut pressure = NativeAllocationPlan::new(request(80));
    pressure
        .add_new(
            ChargeKind::CpuLastValid,
            crate::native_gpu_budget::NATIVE_MANAGED_LIMIT,
        )
        .unwrap();
    let quote = fixture.owner.quote(&pressure).unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let prepared = fixture
        .runtime
        .prepare_native_inputs(
            request(1),
            identity_jobs(65, 3),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    assert!(matches!(
        fixture.owner.quote(prepared.allocation_plan().unwrap()),
        Err(NativeGpuDecline::BudgetExceeded { .. })
    ));
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, 0);
    assert_eq!(
        fixture.owner.budget_snapshot().reserved_bytes,
        crate::native_gpu_budget::NATIVE_MANAGED_LIMIT
    );
    let txn = fixture.owner.begin_frame(request(80), reservation).unwrap();
    let pending = txn.finish();
    await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let quote = fixture
        .owner
        .quote(prepared.allocation_plan().unwrap())
        .unwrap();
    assert!(matches!(
        fixture.runtime.admit_native_batch(prepared, &quote),
        NativeBatchAdmission::Accepted(_)
    ));
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_shared_runtime_rejects_ownerless_cpu_entries() {
    let mut fixture = gpu();
    let shader = shader(IDENTITY);
    let prepared = jobs(3, 2, &[shader.clone()]);
    let input = &prepared[0].input;
    let before = submission_probe(&mut fixture.owner, 50);
    assert!(fixture
        .runtime
        .apply(input, &shader.1, &[], 0.0, 0.016)
        .is_err());
    let (pixels, faults) =
        fixture
            .runtime
            .apply_stack_stage_local(input, &[(&shader.1, &[])], 0.0, 0.016);
    assert_eq!(pixels, **input);
    assert_eq!(faults.len(), 1);
    assert!(faults[0].1.message.contains("NativeGpuOwner"));
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    let after = submission_probe(&mut fixture.owner, 51);
    assert_eq!(after, before + 1);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_cancel_after_submit_never_publishes_or_runs_recovery() {
    let mut fixture = gpu();
    let (encoded, pending) = encode(&mut fixture, 1, identity_jobs(65, 3));
    assert_eq!(encoded.candidates.len(), 3);
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    let result = await_transaction(&mut fixture.owner, pending, &|| true);
    assert!(matches!(
        result,
        Err(MaterializationFailure::Cancelled {
            retirement: Some(_)
        })
    ));
    assert!(
        fixture.owner.budget_snapshot().managed_bytes() > 0,
        "cancel retains unfinished owners"
    );
    drop(encoded);
    assert_eq!(
        fixture.runtime.native_frame_index(),
        3,
        "cancel must not start another canonical tick"
    );
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_stop_after_resolved_batch_prevents_publication_and_recovery() {
    let mut fixture = gpu();
    let (mut encoded, pending) = encode(&mut fixture, 1, identity_jobs(65, 3));
    let receipt = encoded.pending.take().unwrap();
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    assert!(
        phase.validation().is_ok(),
        "GPU completion alone is not publication authority"
    );
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    let stopped = fixture.owner.begin_stop();
    assert_eq!(stopped.context(), fixture.owner.context().id());
    let result = fixture
        .runtime
        .complete_native_batch(encoded, phase, &mut fixture.owner, &|| false);
    assert!(matches!(
        result,
        NativeIsfCompletion::Cancelled { retirement: None }
    ));
    assert_eq!(
        fixture.runtime.native_frame_index(),
        3,
        "Stop must not run fixed-clock fallback or a new chain"
    );
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_stop_after_resolved_historical_replay_prevents_new_lease() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let mut iter = layers.into_iter();
    let first = iter.next().unwrap();
    drop(iter);
    let ValidatedLayerResult { lease, recipe, .. } = first;
    drop(lease);
    let frame = request(2);
    let plan = fixture
        .runtime
        .replay_allocation_plan(frame, &recipe)
        .unwrap();
    let quote = fixture.owner.quote(&plan).unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let mut candidate = fixture
        .runtime
        .encode_historical_replay(&mut txn, recipe.identity().entry, &recipe)
        .unwrap();
    let receipt = candidate.take_replay_receipt().unwrap();
    let pending = txn.finish();
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    assert!(phase.validation().is_ok());
    assert_eq!(phase.phase(), GpuPhase::HistoricalReplay);
    assert_eq!(
        fixture.runtime.native_frame_index(),
        3,
        "replay does not consume a current clock"
    );
    let stopped = fixture.owner.begin_stop();
    assert_eq!(stopped.context(), fixture.owner.context().id());
    let result = fixture
        .runtime
        .complete_historical_replay(candidate, phase, &mut fixture.owner);
    assert!(matches!(
        result,
        Err(MaterializationFailure::Cancelled { retirement: None })
    ));
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    drop(recipe);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_replay_setup_rejection_returns_retirement_and_reclaims_reservation() {
    let mut fixture = gpu();
    let (layers, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    fixture.owner.poll_retirement().unwrap();
    let before = fixture.owner.budget_snapshot();
    let mut wrong = layers[0].identity.entry;
    wrong.sequence += 1;
    let result =
        fixture
            .runtime
            .replay_recipe_to_cpu(wrong, &layers[0].recipe, &mut fixture.owner, &|| false);
    let retirement = match result {
        Err(MaterializationFailure::Fault {
            fault,
            retirement: Some(retirement),
        }) => {
            assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
            assert_eq!(fault.phase, GpuPhase::HistoricalReplay);
            retirement
        }
        _ => {
            panic!("post-begin_frame setup rejection must return the retained transaction identity")
        }
    };
    assert_eq!(retirement.context, fixture.owner.context().id());
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    fixture.owner.poll_retirement().unwrap();
    let after = fixture.owner.budget_snapshot();
    assert_eq!(after.reserved_bytes, 0);
    assert_eq!(after.reservation_count, 0);
    assert_eq!(
        after.allocated_bytes, before.allocated_bytes,
        "setup failure must neither release old history nor leak its reservation"
    );
    assert_eq!(after.allocation_count, before.allocation_count);
    assert_eq!(fixture.owner.state(), GpuOwnerState::Active);
    drop(layers);
    stop(&mut fixture.owner);
}
