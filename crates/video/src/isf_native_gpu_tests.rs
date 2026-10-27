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
    // Match the production cold shape boundary without invalidating reuse on
    // ordinary ticks with unchanged geometry and stage shape.
    let mut shape = [None; 3];
    for (slot, job) in shape.iter_mut().zip(jobs.iter()) {
        *slot = Some((
            job.input.width,
            job.input.height,
            job.input.format,
            job.chain.stages.len(),
        ));
    }
    fixture.owner.configure_stack_pool_shape(shape);
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
    let before_queries = fixture.owner.budget_snapshot();
    let prospective = inputs.prospective_identities(&fixture.runtime).unwrap();
    assert_eq!(&*prospective, expected.as_slice());
    for _ in 0..3 {
        assert_eq!(
            &*inputs.prospective_identities(&fixture.runtime).unwrap(),
            &*prospective,
            "repeated preflight queries must keep the same complete identity"
        );
    }
    assert_eq!(fixture.runtime.native_frame_index(), first_clock);
    assert_eq!(fixture.owner.budget_snapshot(), before_queries);
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
    for ((candidate, expected), prospective) in encoded
        .candidates
        .iter()
        .zip(&expected)
        .zip(prospective.iter())
    {
        assert_eq!(
            candidate.identity(),
            expected,
            "provisional identity must preserve its actual input and reserved clocks"
        );
        assert_eq!(candidate.identity(), prospective);
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

fn g2_admitted(
    runtime: &IsfGpuRuntime,
    prepared: PreparedNativeInputs,
    quote: &BudgetQuote,
) -> PreparedNativeBatch {
    match runtime.admit_native_batch(prepared, quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        NativeBatchAdmission::Declined { reason, .. } => {
            panic!("eligible G2 fixture declined: {reason:?}")
        }
    }
}

#[test]
fn isf_native_gpu_g2_prospective_queries_keep_full_identity_and_batch3_advances_three_twice() {
    let mut fixture = gpu();
    let specs = [shader(CLOCK_RED), shader(CLOCK_GREEN), shader(TIME_DATE)];
    let mut retained: Vec<(StackBatch<ValidatedLayerResult>, Vec<LayerResultIdentity>)> =
        Vec::new();
    for round in 0..2 {
        let before = submission_probe(&mut fixture.owner, 80 + round * 2);
        let prepared = jobs(65, 3, &specs);
        let originals = prepared
            .iter()
            .map(|job| Arc::clone(&job.input))
            .collect::<Vec<_>>();
        // encode independently calculates every expected field, queries the
        // same prepared inputs repeatedly, then checks all actual candidates.
        let (layers, receipt) = validated(&mut fixture, round + 1, prepared);
        assert_eq!(receipt.phase().submission().sequence, before + 1);
        assert_eq!(fixture.runtime.native_frame_index(), (round as u32 + 1) * 3);
        for (index, layer) in layers.iter().enumerate() {
            assert_eq!(
                layer.recipe.clocks().frame_index,
                round as u32 * 3 + index as u32
            );
        }
        for (old_layers, identities) in &retained {
            assert_eq!(
                old_layers
                    .iter()
                    .map(|layer| layer.identity)
                    .collect::<Vec<_>>(),
                *identities,
                "later prospective queries must not rewrite validated history"
            );
        }
        let identities = layers
            .iter()
            .map(|layer| layer.identity)
            .collect::<Vec<_>>();
        retained.push((layers, identities));
        assert!(originals.iter().all(|input| Arc::strong_count(input) > 1));
    }
    drop(retained);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_g2_resourceful_and_cpu_prospective_queries_return_typed_declines_without_work() {
    let mut fixture = gpu();
    let before = submission_probe(&mut fixture.owner, 80);
    let budget = fixture.owner.budget_snapshot();
    for source in [MULTIPASS, CPU_BRIDGE_PERSISTENT] {
        let prepared = jobs(3, 2, &[shader(source)]);
        let originals = prepared
            .iter()
            .map(|job| Arc::clone(&job.input))
            .collect::<Vec<_>>();
        let prepared = fixture
            .runtime
            .prepare_native_inputs(request(1), prepared, NativeBatchRestrictions::default())
            .unwrap();
        for _ in 0..3 {
            assert_eq!(
                prepared
                    .prospective_identities(&fixture.runtime)
                    .unwrap_err(),
                NativeProspectiveIdentityError::Declined(NativeGpuDecline::UnsupportedChain {
                    reason: UnsupportedNativeFx::Multipass,
                })
            );
        }
        let quote = fixture
            .owner
            .quote(&NativeAllocationPlan::new(request(1)))
            .unwrap();
        match fixture.runtime.admit_native_batch(prepared, &quote) {
            NativeBatchAdmission::Declined { reason, prepared } => {
                assert_eq!(
                    reason,
                    NativeGpuDecline::UnsupportedChain {
                        reason: UnsupportedNativeFx::Multipass,
                    }
                );
                let returned = prepared.into_layers();
                assert!(returned
                    .iter()
                    .zip(&originals)
                    .all(|(job, original)| Arc::ptr_eq(&job.input, original)));
            }
            NativeBatchAdmission::Accepted(_) => panic!("resourceful work entered Batch3"),
        }
        assert!(originals.iter().all(|input| Arc::strong_count(input) == 1));
    }
    let job = jobs(3, 2, &[shader(MULTIPASS)]).into_iter().next().unwrap();
    let original = job.input.as_ref().clone();
    let invocation =
        PreparedNativeCpuInvocation::new(Arc::clone(&job.input), job.chain.clone(), job.date, None);
    let prepared = fixture
        .runtime
        .prepare_native_cpu_inputs(request(2), invocation)
        .unwrap();
    for _ in 0..3 {
        assert_eq!(
            prepared
                .prospective_identities(&fixture.runtime)
                .unwrap_err(),
            NativeProspectiveIdentityError::CpuFallback
        );
    }
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    assert_eq!(fixture.owner.budget_snapshot(), budget);
    assert_eq!(*job.input, original);
    drop(prepared);
    assert_eq!(Arc::strong_count(&job.input), 1);
    assert_eq!(submission_probe(&mut fixture.owner, 81), before + 1);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_g2_clock_advance_after_query_declines_even_with_a_fresh_covering_quote() {
    let mut fixture = gpu();
    let jobs = identity_jobs(3, 2);
    // Establish the cold workload shape before freezing either reservation.
    // The intervening validated tick then changes clocks alone, not pool epoch.
    fixture
        .owner
        .configure_stack_pool_shape([Some((3, 2, VideoPixelFormat::Rgba8, 3)); 3]);
    let originals = jobs
        .iter()
        .map(|job| Arc::clone(&job.input))
        .collect::<Vec<_>>();
    let prepared = fixture
        .runtime
        .prepare_native_inputs(request(1), jobs, NativeBatchRestrictions::default())
        .unwrap();
    let first = prepared.prospective_identities(&fixture.runtime).unwrap();
    assert!(matches!(first[0].origin, LayerOrigin::Executed(clocks) if clocks.frame_index == 0));
    let unqueried = fixture
        .runtime
        .prepare_native_inputs(
            request(3),
            identity_jobs(3, 2),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let (history, _) = validated(&mut fixture, 2, identity_jobs(3, 2));
    let before = submission_probe(&mut fixture.owner, 80);
    let budget = fixture.owner.budget_snapshot();
    assert_eq!(
        unqueried
            .prospective_identities(&fixture.runtime)
            .unwrap_err(),
        NativeProspectiveIdentityError::StaleClock,
        "clocks freeze during preparation even if the first query comes later"
    );
    for _ in 0..3 {
        assert_eq!(
            prepared
                .prospective_identities(&fixture.runtime)
                .unwrap_err(),
            NativeProspectiveIdentityError::StaleClock
        );
    }
    // Quote after the intervening render: refusal must come from the clock
    // reservation, not an obsolete ledger revision or a different plan.
    let quote = fixture
        .owner
        .quote(prepared.allocation_plan().unwrap())
        .unwrap();
    assert!(quote.covers_plan(prepared.allocation_plan().unwrap()));
    match fixture.runtime.admit_native_batch(prepared, &quote) {
        NativeBatchAdmission::Declined { reason, prepared } => {
            assert_eq!(reason, NativeGpuDecline::QuoteExpired);
            let returned = prepared.into_layers();
            assert!(returned
                .iter()
                .zip(&originals)
                .all(|(job, original)| Arc::ptr_eq(&job.input, original)));
        }
        NativeBatchAdmission::Accepted(_) => panic!("stale prospective clocks were admitted"),
    }
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    assert_eq!(fixture.owner.budget_snapshot(), budget);
    assert!(originals.iter().all(|input| Arc::strong_count(input) == 1));
    assert_eq!(submission_probe(&mut fixture.owner, 81), before + 1);
    drop(history);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_g2_clock_advance_after_admission_rejects_before_phase_binding_or_submission() {
    let mut fixture = gpu();
    fixture
        .owner
        .configure_stack_pool_shape([Some((3, 2, VideoPixelFormat::Rgba8, 3)); 3]);
    let jobs = identity_jobs(3, 2);
    let originals = jobs
        .iter()
        .map(|job| Arc::clone(&job.input))
        .collect::<Vec<_>>();
    let prepared = fixture
        .runtime
        .prepare_native_inputs(request(1), jobs, NativeBatchRestrictions::default())
        .unwrap();
    prepared.prospective_identities(&fixture.runtime).unwrap();
    let plan = prepared.allocation_plan().unwrap().clone();
    let quote = fixture.owner.quote(&plan).unwrap();
    let admitted = g2_admitted(&fixture.runtime, prepared, &quote);
    let reservation = fixture.owner.reserve(quote).unwrap();
    let (history, _) = validated(&mut fixture, 2, identity_jobs(3, 2));
    let before = submission_probe(&mut fixture.owner, 80);
    let budget = fixture.owner.budget_snapshot();
    let mut txn = fixture.owner.begin_frame(request(1), reservation).unwrap();
    let fault = match fixture.runtime.encode_native_batch(&mut txn, admitted) {
        Err(fault) => fault,
        Ok(_) => panic!("admission must not authorize stale clocks at encode"),
    };
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    assert!(
        fault.message.contains("StaleClock"),
        "pool invalidation must not mask clock rejection"
    );
    txn.validate_plan(&plan).unwrap();
    let pending = txn.finish();
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    assert_eq!(fixture.owner.budget_snapshot(), budget);
    assert!(originals.iter().all(|input| Arc::strong_count(input) == 1));
    await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    assert_eq!(submission_probe(&mut fixture.owner, 81), before + 1);
    drop(history);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_g2_foreign_context_and_same_context_other_runtime_reject_before_gpu_work() {
    let mut fixture = gpu();
    let mut sibling = IsfGpuRuntime::with_native_context(&mut fixture.owner).unwrap();
    let mut foreign = gpu();
    assert!(Arc::ptr_eq(
        fixture.runtime.native_context().unwrap(),
        sibling.native_context().unwrap()
    ));
    assert_ne!(fixture.owner.context().id(), foreign.owner.context().id());
    for same_context in [true, false] {
        let before = submission_probe(&mut fixture.owner, 80);
        let foreign_before = submission_probe(&mut foreign.owner, 80);
        let budget = fixture.owner.budget_snapshot();
        let foreign_budget = foreign.owner.budget_snapshot();
        let prepared = fixture
            .runtime
            .prepare_native_inputs(
                request(1),
                identity_jobs(3, 2),
                NativeBatchRestrictions::default(),
            )
            .unwrap();
        let prospective = prepared.prospective_identities(&fixture.runtime).unwrap();
        let wrong_runtime = if same_context {
            &sibling
        } else {
            &foreign.runtime
        };
        let expected = if same_context {
            NativeProspectiveIdentityError::StaleClock
        } else {
            NativeProspectiveIdentityError::ContextMismatch
        };
        for _ in 0..3 {
            assert_eq!(
                prepared.prospective_identities(wrong_runtime).unwrap_err(),
                expected
            );
        }
        let plan = prepared.allocation_plan().unwrap().clone();
        let quote = fixture.owner.quote(&plan).unwrap();
        let prepared = match wrong_runtime.admit_native_batch(prepared, &quote) {
            NativeBatchAdmission::Declined { reason, prepared } => {
                assert_eq!(reason, NativeGpuDecline::QuoteExpired);
                prepared
            }
            NativeBatchAdmission::Accepted(_) => panic!("another runtime adopted prepared clocks"),
        };
        assert_eq!(
            &*prepared.prospective_identities(&fixture.runtime).unwrap(),
            &*prospective
        );
        assert_eq!(fixture.owner.budget_snapshot(), budget);
        assert_eq!(foreign.owner.budget_snapshot(), foreign_budget);
        let admitted = g2_admitted(&fixture.runtime, prepared, &quote);
        let reservation = fixture.owner.reserve(quote).unwrap();
        let reserved = fixture.owner.budget_snapshot();
        let mut txn = fixture.owner.begin_frame(request(1), reservation).unwrap();
        let result = if same_context {
            sibling.encode_native_batch(&mut txn, admitted)
        } else {
            foreign.runtime.encode_native_batch(&mut txn, admitted)
        };
        let fault = match result {
            Err(fault) => fault,
            Ok(_) => panic!("another runtime consumed the admitted batch"),
        };
        assert_eq!(
            fault.kind,
            if same_context {
                NativeGpuFaultKind::StateMismatch
            } else {
                NativeGpuFaultKind::IdentityMismatch
            }
        );
        txn.validate_plan(&plan).unwrap();
        let pending = txn.finish();
        assert_eq!(fixture.owner.budget_snapshot(), reserved);
        assert_eq!(foreign.owner.budget_snapshot(), foreign_budget);
        assert_eq!(fixture.runtime.native_frame_index(), 0);
        assert_eq!(sibling.native_frame_index(), 0);
        assert_eq!(foreign.runtime.native_frame_index(), 0);
        await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
        assert_eq!(submission_probe(&mut fixture.owner, 81), before + 1);
        assert_eq!(submission_probe(&mut foreign.owner, 81), foreign_before + 1);
    }
    stop(&mut fixture.owner);
    stop(&mut foreign.owner);
}

#[test]
fn isf_native_gpu_g2_p_slots_must_be_split_before_isf_but_history_aliases_remain_accepted() {
    let mut fixture = gpu();
    let (history, _) = validated(&mut fixture, 1, identity_jobs(65, 3));
    let old_identities = history
        .iter()
        .map(|layer| layer.identity)
        .collect::<Vec<_>>();
    for split in 0..3 {
        let frame = request(split + 2);
        let before = submission_probe(&mut fixture.owner, 80 + split * 2);
        let first_clock = fixture.runtime.native_frame_index();
        let prepared = fixture
            .runtime
            .prepare_native_inputs(
                frame,
                identity_jobs(65, 3),
                NativeBatchRestrictions::default(),
            )
            .unwrap();
        let prospective = prepared.prospective_identities(&fixture.runtime).unwrap();
        let g_plan = prepared.allocation_plan().unwrap().clone();
        let mut p_plan = NativeAllocationPlan::new(frame);
        p_plan
            .add_new(
                ChargeKind::QueueUploadStaging,
                checked_upload_staging_bytes(
                    65,
                    3,
                    fixture.owner.context().capabilities().upload_row_pitch,
                )
                .unwrap(),
            )
            .unwrap();
        p_plan.add_existing(history[0].lease.allocation_id());
        // P's upload deliberately precedes equal-kind/equal-capacity G uploads.
        // Family coverage alone cannot keep the root binder from spending it.
        let mut combined = p_plan.clone();
        combined.merge(&g_plan).unwrap();
        let quote = fixture.owner.quote(&combined).unwrap();
        let admitted = g2_admitted(&fixture.runtime, prepared, &quote);
        let reservation = fixture.owner.reserve(quote).unwrap();
        let reserved = fixture.owner.budget_snapshot();
        let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
        let child = match split {
            0 => None,
            1 => Some(txn.split_reservation(&g_plan).unwrap()),
            _ => Some(txn.split_reservation(&p_plan).unwrap()),
        };
        txn.validate_plan(&g_plan).unwrap();
        let result = fixture.runtime.encode_native_batch(&mut txn, admitted);
        if split != 2 {
            assert_eq!(
                result
                    .as_ref()
                    .err()
                    .expect("unsafe root must decline")
                    .kind,
                NativeGpuFaultKind::StateMismatch
            );
            txn.validate_plan(&g_plan).unwrap();
        }
        let pending = txn.finish();
        match result {
            Err(_) => {
                assert_ne!(split, 2, "correctly split root was refused");
                assert_eq!(fixture.runtime.native_frame_index(), first_clock);
                let after = fixture.owner.budget_snapshot();
                assert_eq!(after.allocated_bytes, reserved.allocated_bytes);
                assert_eq!(after.allocation_count, reserved.allocation_count);
                assert_eq!(after.reserved_bytes, reserved.reserved_bytes);
                assert_eq!(after.by_kind, reserved.by_kind);
                await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
                assert_eq!(
                    submission_probe(&mut fixture.owner, 81 + split * 2),
                    before + 1,
                    "root-plan refusal must not open or abandon an ISF phase"
                );
            }
            Ok(encoded) => {
                assert_eq!(split, 2);
                assert_eq!(
                    encoded
                        .candidates
                        .iter()
                        .map(|candidate| *candidate.identity())
                        .collect::<Vec<_>>(),
                    prospective.iter().copied().collect::<Vec<_>>()
                );
                match complete(&mut fixture, encoded, pending) {
                    NativeIsfCompletion::Validated { layers, receipt } => {
                        assert_eq!(receipt.phase().submission().sequence, before + 1);
                        assert_eq!(layers.len(), 3);
                        assert_eq!(fixture.runtime.native_frame_index(), first_clock + 3);
                        drop(layers);
                    }
                    _ => panic!("correct P split with retained history must validate"),
                }
            }
        }
        drop(child);
        assert_eq!(
            history
                .iter()
                .map(|layer| layer.identity)
                .collect::<Vec<_>>(),
            old_identities
        );
    }
    assert_eq!(fixture.runtime.native_frame_index(), 6);
    drop(history);
    stop(&mut fixture.owner);
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
    // One pass keeps target-specific rejection independent of the Multipass case.
    const PERSISTENT_TARGET: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{"TARGET":"historyBuffer","PERSISTENT":true}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#;
    const CUSTOM_TARGET: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{"TARGET":"customBuffer"}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#;
    let mut fixture = gpu();
    for case in 0..9 {
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
            6 => {
                prepared[0].date[0] = f32::NAN;
                UnsupportedNativeFx::AuxiliaryResource
            }
            7 => {
                prepared = jobs(3, 2, &[shader(PERSISTENT_TARGET)]);
                UnsupportedNativeFx::PersistentTarget
            }
            _ => {
                prepared = jobs(3, 2, &[shader(CUSTOM_TARGET)]);
                UnsupportedNativeFx::CustomTarget
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
    // Rejected replay invalidates idle pooled scratch/finals. Referenced
    // history remains charged; compare its exact bytes rather than requiring
    // the invalidated idle bank to survive this failure.
    let history_bytes: u64 = layers
        .iter()
        .map(|layer| u64::from(layer.identity.width) * u64::from(layer.identity.height) * 4)
        .sum();
    let resident_bytes = |snapshot: &crate::native_gpu_budget::NativeGpuBudgetSnapshot| {
        let kind = snapshot.for_kind(ChargeKind::ResidentLayer);
        kind.active_bytes + kind.retired_bytes + kind.quarantined_bytes
    };
    assert_eq!(resident_bytes(&before), history_bytes);
    assert_eq!(
        resident_bytes(&after),
        history_bytes,
        "setup failure released referenced history"
    );
    assert_eq!(
        after.allocated_bytes, history_bytes,
        "setup failure leaked an allocation beyond referenced history"
    );
    assert_eq!(after.allocation_count, layers.len());
    for layer in &*layers {
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
        assert_eq!(
            actual.frame(),
            &*layer.recipe.job().input,
            "setup rejection altered preserved history pixels"
        );
    }
    assert_eq!(fixture.owner.state(), GpuOwnerState::Active);
    drop(layers);
    stop(&mut fixture.owner);
}

// Shared-owner canonical CPU bridge (§4.5). These oracles reuse the existing
// fixture shaders and CPU entry points rather than a second shader executor.
const CPU_BRIDGE_DATE: [f32; 4] = [2026.0, 10.0, 26.0, 512.0];
const CPU_BRIDGE_MULTIPASS: &str = include_str!("isf_fixtures/multipass.fs");
const CPU_BRIDGE_PERSISTENT: &str = include_str!("isf_fixtures/persistent.fs");
const CPU_BRIDGE_FLOAT: &str = include_str!("isf_fixtures/float.fs");
const CPU_BRIDGE_IMPORTED: &str = include_str!("isf_fixtures/imported.fs");
const CPU_BRIDGE_TWO_IMAGES: &str = include_str!("isf_fixtures/two_images.fs");

fn cpu_bridge_input(width: u32, height: u32, pixel: [u8; 4]) -> Arc<VideoFrame> {
    Arc::new(VideoFrame {
        layer_id: 1,
        width,
        height,
        pts_ms: 1000,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: pixel
            .into_iter()
            .cycle()
            .take((width * height * 4) as usize)
            .collect(),
    })
}
fn cpu_bridge_chain(
    scope: protocol::VideoEffectScope,
    sources: &[&str],
) -> ResolvedVideoEffectChain {
    ResolvedVideoEffectChain {
        chain_id: Some(protocol::VideoEffectChainId(901)),
        scope,
        bypassed: false,
        stages: sources
            .iter()
            .enumerate()
            .map(|(index, source)| ResolvedVideoEffectStage {
                stage_id: Some(protocol::VideoEffectStageId(910 + index as u64)),
                effect_id: Some(protocol::VideoEffectId(920 + index as u64)),
                enabled: true,
                label: format!("cpu-stage-{index}"),
                effect: protocol::VideoIsfEffectSummary {
                    enabled: true,
                    label: format!("cpu-stage-{index}"),
                    source: Arc::from(*source),
                    source_path: None,
                    description: None,
                    categories: Vec::new(),
                    controls: Vec::new(),
                    image_bindings: Vec::new(),
                    stack: Vec::new(),
                },
            })
            .collect(),
    }
}
fn cpu_bridge_renderer() -> crate::VideoPreviewRenderer {
    let mut renderer = crate::VideoPreviewRenderer::new(crate::VideoRuntimeConfig::default());
    renderer.isf_render_namespace = (7, Some(101), false);
    renderer.set_isf_date_override(Some(CPU_BRIDGE_DATE));
    renderer
}
fn cpu_bridge_input_key() -> protocol::VideoRenderInputKey {
    protocol::VideoRenderInputKey {
        project_render_epoch: 7,
        input_id: protocol::VideoRenderInputId(1001),
    }
}
fn cpu_bridge_oracle(
    renderer: &mut crate::VideoPreviewRenderer,
    chain: &ResolvedVideoEffectChain,
    input: &Arc<VideoFrame>,
) -> (VideoFrame, Vec<VideoEffectStageFault>) {
    let (frame, faults) = renderer
        .apply_resolved_effect_chain_cow_with_cancellation(
            chain,
            Cow::Borrowed(input.as_ref()),
            Some(cpu_bridge_input_key()),
            None,
        )
        .unwrap();
    (frame.into_owned(), faults)
}
fn cpu_bridge_run_prepared(
    fixture: &mut Fixture,
    prepared: PreparedNativeInputs,
) -> NativeCpuResults {
    let context = Arc::clone(fixture.owner.context());
    assert!(Arc::ptr_eq(
        fixture.runtime.native_context().unwrap(),
        &context
    ));
    let quote = fixture
        .owner
        .quote(prepared.allocation_plan().unwrap())
        .unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let result = fixture.runtime.render_native_cpu_fallback(
        &mut fixture.owner,
        prepared,
        reservation,
        &|| false,
    );
    let result = result
        .unwrap_or_else(|error| panic!("canonical shared-owner CPU bridge failed: {error:?}"));
    assert!(Arc::ptr_eq(
        fixture.runtime.native_context().unwrap(),
        &context
    ));
    for outcome in result.outcomes() {
        assert!(outcome
            .allocation_ids()
            .iter()
            .all(|allocation| allocation.context == context.id()));
    }
    result
}
fn cpu_bridge_run(
    fixture: &mut Fixture,
    preparer: &mut crate::VideoPreviewRenderer,
    sequence: u64,
    chain: ResolvedVideoEffectChain,
    input: Arc<VideoFrame>,
) -> NativeCpuResults {
    let invocation = preparer
        .prepare_native_cpu_invocation(chain, input, Some(cpu_bridge_input_key()), None)
        .unwrap();
    let prepared = fixture
        .runtime
        .prepare_native_cpu_inputs(request(sequence), invocation)
        .unwrap();
    cpu_bridge_run_prepared(fixture, prepared)
}
fn cpu_bridge_assert(
    result: &NativeCpuResults,
    expected: &(VideoFrame, Vec<VideoEffectStageFault>),
    scope: &protocol::VideoEffectScope,
) {
    assert_eq!(
        result.outcomes().len(),
        1,
        "a group/output invocation stays one result"
    );
    let actual = &result.outcomes()[0];
    assert_eq!(actual.frame().as_ref(), &expected.0);
    assert_eq!(actual.faults(), expected.1.as_slice());
    assert_eq!(actual.scope(), scope);
}
fn cpu_bridge_stop(fixture: Fixture) {
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime); // runtime-owned feedback remains charged until its owner retires
    stop(&mut owner);
}
fn cpu_bridge_scope(job: &mut PreparedNativeLayer, scope: protocol::VideoEffectScope) {
    job.chain.source.to_mut().scope = scope.clone();
    for key in &mut job.chain.keys {
        key.scope = Some(scope.clone());
    }
    for stage in &mut job.chain.stages {
        stage.fault_context.scope = scope.clone();
    }
}
fn cpu_bridge_runtime_oracle(
    runtime: &mut IsfGpuRuntime,
    job: &PreparedNativeLayer,
) -> (VideoFrame, Vec<VideoEffectStageFault>) {
    let images = job
        .chain
        .stages
        .iter()
        .map(|stage| {
            stage
                .images
                .iter()
                .map(|(name, frame)| crate::isf_runtime::IsfImageInput { name, frame })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let stages = job
        .chain
        .stages
        .iter()
        .zip(&images)
        .map(|(stage, images)| IsfStageInputs {
            shader: &stage.shader,
            controls: &stage.controls,
            images,
        })
        .collect::<Vec<_>>();
    let (frame, failures) = runtime.apply_stack_stage_local_with_inputs_and_keys(
        &job.input,
        &stages,
        job.input.pts_ms as f32 / 1000.0,
        job.input.duration_ms as f32 / 1000.0,
        job.date,
        &job.chain.keys,
    );
    (frame, job.chain.clone().finish_faults(failures))
}
fn cpu_bridge_run_job(
    fixture: &mut Fixture,
    sequence: u64,
    job: &PreparedNativeLayer,
) -> NativeCpuResults {
    let invocation =
        PreparedNativeCpuInvocation::new(Arc::clone(&job.input), job.chain.clone(), job.date, None);
    let prepared = fixture
        .runtime
        .prepare_native_cpu_inputs(request(sequence), invocation)
        .unwrap();
    cpu_bridge_run_prepared(fixture, prepared)
}

#[test]
fn isf_native_gpu_cpu_bridge_group_composition_output_match_canonical_multipass_and_float() {
    let mut fixture = gpu();
    let mut preparer = cpu_bridge_renderer();
    let mut reference = cpu_bridge_renderer();
    let input = cpu_bridge_input(8, 4, [0, 0, 0, 255]);
    let scopes = [
        protocol::VideoEffectScope::Group {
            group_id: protocol::VideoLayerGroupId(7),
        },
        protocol::VideoEffectScope::Composition { composition_id: 5 },
        protocol::VideoEffectScope::Output { output_id: 101 },
    ];
    for (index, scope) in scopes.into_iter().enumerate() {
        let chain = cpu_bridge_chain(scope.clone(), &[CPU_BRIDGE_MULTIPASS]);
        let expected = cpu_bridge_oracle(&mut reference, &chain, &input);
        let result = cpu_bridge_run(
            &mut fixture,
            &mut preparer,
            index as u64 + 1,
            chain,
            Arc::clone(&input),
        );
        cpu_bridge_assert(&result, &expected, &scope);
        assert!(result.outcomes()[0].faults().is_empty());
        assert!(result.outcomes()[0]
            .frame()
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [128, 128, 255, 255]));
        assert_eq!(
            result.outcomes()[0].clocks().unwrap().frame_index,
            index as u32
        );
        assert_eq!(fixture.runtime.native_frame_index(), index as u32 + 1);
    }
    let scope = protocol::VideoEffectScope::Output { output_id: 101 };
    let chain = cpu_bridge_chain(scope.clone(), &[CPU_BRIDGE_FLOAT]);
    let input = cpu_bridge_input(2, 1, [0, 0, 0, 255]);
    let expected = cpu_bridge_oracle(&mut reference, &chain, &input);
    let result = cpu_bridge_run(&mut fixture, &mut preparer, 4, chain, input);
    cpu_bridge_assert(&result, &expected, &scope);
    let float_supported = fixture
        .owner
        .capabilities()
        .enabled_features
        .contains(wgpu::Features::FLOAT32_FILTERABLE);
    eprintln!(
        "native CPU FLOAT32_FILTERABLE={float_supported}; pixel oracle exercised only when true"
    );
    if float_supported {
        assert!(result.outcomes()[0].faults().is_empty());
        assert!(result.outcomes()[0]
            .frame()
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 128, 128, 255]));
    } else {
        assert_eq!(result.outcomes()[0].faults().len(), 1);
        assert!(result.outcomes()[0].faults()[0]
            .message
            .contains("32-bit float texture filtering"));
    }
    assert_eq!(fixture.runtime.native_frame_index(), 4);
    drop(result);
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_feedback_failure_and_dynamic_images_match_canonical_ticks() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let increment = shader(INCREMENT);
    let mut invalid = shader(IDENTITY);
    Arc::make_mut(&mut invalid.1).wgsl = "not valid WGSL".into();
    let mut job = jobs(
        3,
        2,
        &[
            increment.clone(),
            increment.clone(),
            shader(CPU_BRIDGE_PERSISTENT),
            invalid,
            increment.clone(),
            increment,
        ],
    )
    .into_iter()
    .next()
    .unwrap();
    cpu_bridge_scope(
        &mut job,
        protocol::VideoEffectScope::Group {
            group_id: protocol::VideoLayerGroupId(7),
        },
    );
    for tick in 0..3 {
        let expected = cpu_bridge_runtime_oracle(&mut reference, &job);
        let result = cpu_bridge_run_job(&mut fixture, tick + 1, &job);
        cpu_bridge_assert(&result, &expected, &job.chain.source.scope);
        let actual = &result.outcomes()[0];
        assert!(actual
            .frame()
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [tick as u8 + 3, 0, 0, 255]));
        assert_eq!(actual.faults().len(), 1);
        assert_eq!(actual.faults()[0].stage_index, Some(3));
        assert_eq!(
            actual.faults()[0].stage_id,
            Some(protocol::VideoEffectStageId(13))
        );
        assert_eq!(
            actual.faults()[0].effect_id,
            Some(protocol::VideoEffectId(103))
        );
        assert_eq!(
            actual.faults()[0].stage_label.as_deref(),
            Some("layer-1-stage-3")
        );
        assert_eq!(actual.clocks().unwrap().frame_index, tick as u32);
        assert_eq!(fixture.runtime.native_frame_index(), tick as u32 + 1);
    }
    const DYNAMIC: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"overlayImage","TYPE":"image"}],
        "PASSES":[{"TARGET":"historyBuffer","PERSISTENT":true},{}]}*/
        void main() {
          if(PASSINDEX==0) { gl_FragColor=vec4(IMG_THIS_NORM_PIXEL(historyBuffer).r + IMG_THIS_NORM_PIXEL(overlayImage).r,0.0,0.0,1.0); }
          else { gl_FragColor=IMG_THIS_NORM_PIXEL(historyBuffer); }
        }"#;
    let mut job = jobs(1, 1, &[shader(DYNAMIC)]).into_iter().next().unwrap();
    cpu_bridge_scope(
        &mut job,
        protocol::VideoEffectScope::Group {
            group_id: protocol::VideoLayerGroupId(8),
        },
    );
    for (index, (red, expected_red)) in [(Some(1), 1), (Some(2), 3), (None, 3), (Some(4), 7)]
        .into_iter()
        .enumerate()
    {
        job.chain.stages[0].images = red
            .map(|red| {
                (
                    "overlayImage".into(),
                    cpu_bridge_input(1, 1, [red, 0, 0, 255]),
                )
            })
            .into_iter()
            .collect();
        let expected = cpu_bridge_runtime_oracle(&mut reference, &job);
        let result = cpu_bridge_run_job(&mut fixture, index as u64 + 4, &job);
        cpu_bridge_assert(&result, &expected, &job.chain.source.scope);
        assert_eq!(result.outcomes()[0].frame().data, [expected_red, 0, 0, 255]);
        assert!(result.outcomes()[0].faults().is_empty());
        assert_eq!(fixture.runtime.native_frame_index(), index as u32 + 4);
    }
    cpu_bridge_stop(fixture);
}

struct CpuBridgePack(std::path::PathBuf);
impl CpuBridgePack {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("native-cpu-bridge-fixtures")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("imported.fs"), CPU_BRIDGE_IMPORTED).unwrap();
        std::fs::write(path.join("bound.fs"), CPU_BRIDGE_TWO_IMAGES).unwrap();
        image::save_buffer_with_format(
            path.join("two-pixels.png"),
            &[255, 0, 0, 255, 0, 255, 0, 255],
            2,
            1,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .unwrap();
        image::save_buffer_with_format(
            path.join("still.png"),
            &[255, 0, 0, 255],
            1,
            1,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .unwrap();
        Self(path)
    }
}
impl Drop for CpuBridgePack {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0)
            .expect("remove only the test-owned native CPU image fixture");
    }
}

#[test]
fn isf_native_gpu_cpu_bridge_imported_and_bound_images_use_canonical_pack_pixels() {
    let pack = CpuBridgePack::new();
    let mut fixture = gpu();
    let mut preparer = cpu_bridge_renderer();
    let mut reference = cpu_bridge_renderer();
    let scope = protocol::VideoEffectScope::Output { output_id: 101 };
    let mut imported = cpu_bridge_chain(scope.clone(), &[CPU_BRIDGE_IMPORTED]);
    imported.stages[0].effect.source_path =
        Some(pack.0.join("imported.fs").to_string_lossy().into_owned());
    let input = cpu_bridge_input(4, 1, [0, 0, 0, 255]);
    let expected = cpu_bridge_oracle(&mut reference, &imported, &input);
    let result = cpu_bridge_run(&mut fixture, &mut preparer, 1, imported, input);
    cpu_bridge_assert(&result, &expected, &scope);
    assert_eq!(
        result.outcomes()[0].frame().data,
        [0, 255, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
    );
    drop(result);
    let mut bound = cpu_bridge_chain(scope.clone(), &[CPU_BRIDGE_TWO_IMAGES]);
    bound.stages[0].effect.source_path =
        Some(pack.0.join("bound.fs").to_string_lossy().into_owned());
    bound.stages[0].effect.image_bindings = vec![protocol::VideoIsfImageBinding {
        name: "overlayImage".into(),
        source: protocol::VideoIsfImageSource::Imported {
            path: "still.png".into(),
        },
    }];
    let input = cpu_bridge_input(2, 1, [0, 0, 255, 255]);
    let expected = cpu_bridge_oracle(&mut reference, &bound, &input);
    let result = cpu_bridge_run(&mut fixture, &mut preparer, 2, bound, input);
    cpu_bridge_assert(&result, &expected, &scope);
    assert!(result.outcomes()[0]
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [128, 0, 128, 255]));
    assert_eq!(fixture.runtime.native_frame_index(), 2);
    drop(result);
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_audio_images_date_multipass_never_reuses_stale_inputs() {
    const MIXED: &str = r#"/*{"INPUTS":[{"NAME":"overlayImage","TYPE":"image"},{"NAME":"audioWave","TYPE":"audio","MAX":2},
      {"NAME":"inputImage","TYPE":"image"},{"NAME":"audioSpectrum","TYPE":"audioFFT","MAX":2}],
      "PASSES":[{"TARGET":"mixedBuffer"},{}]}*/
      void main() {
        if(PASSINDEX==0) gl_FragColor=vec4((IMG_THIS_NORM_PIXEL(inputImage).r+IMG_THIS_NORM_PIXEL(overlayImage).r)*0.5,
          IMG_NORM_PIXEL(audioWave,vec2(0.0,0.5)).r,IMG_NORM_PIXEL(audioSpectrum,vec2(0.0,0.5)).r,DATE.w/1024.0);
        else gl_FragColor=IMG_THIS_NORM_PIXEL(mixedBuffer);
      }"#;
    let mut fixture = gpu();
    let mut preparer = cpu_bridge_renderer();
    let mut reference = cpu_bridge_renderer();
    let scope = protocol::VideoEffectScope::Composition { composition_id: 5 };
    let mut chain = cpu_bridge_chain(scope.clone(), &[MIXED]);
    chain.stages[0].effect.image_bindings = vec![protocol::VideoIsfImageBinding {
        name: "overlayImage".into(),
        source: protocol::VideoIsfImageSource::Layer { layer_id: 2 },
    }];
    let input = cpu_bridge_input(1, 1, [64, 0, 0, 255]);
    let audio = |sequence, wave, fft| protocol::LiveAudioTextureFrame {
        generation: 1,
        feature_sequence: sequence,
        sample_rate: 48_000,
        waveform: [wave; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [fft; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
    };
    let samples = [
        (Some(audio(1, -0.5, 0.75)), 192, [128, 64, 191, 128]),
        (Some(audio(1, 0.5, 0.25)), 32, [48, 64, 191, 128]),
        (Some(audio(2, 0.5, 0.25)), 32, [48, 191, 64, 128]),
        (None, 32, [48, 128, 0, 128]),
        (Some(audio(3, f32::NAN, 1.0)), 32, [48, 128, 0, 128]),
    ];
    for (index, (audio, red, known)) in samples.into_iter().enumerate() {
        let mut overlay = cpu_bridge_input(1, 1, [red, 0, 0, 255]);
        Arc::make_mut(&mut overlay).layer_id = 2;
        for renderer in [&mut preparer, &mut reference] {
            renderer
                .isf_frame_inputs
                .layers
                .insert(2, Ok(Arc::clone(&overlay)));
            renderer.set_isf_audio_frame(audio.as_ref());
        }
        let expected = cpu_bridge_oracle(&mut reference, &chain, &input);
        let result = cpu_bridge_run(
            &mut fixture,
            &mut preparer,
            index as u64 + 1,
            chain.clone(),
            Arc::clone(&input),
        );
        cpu_bridge_assert(&result, &expected, &scope);
        assert_eq!(result.outcomes()[0].frame().data, known);
        assert!(result.outcomes()[0].faults().is_empty());
        assert_eq!(fixture.runtime.native_frame_index(), index as u32 + 1);
    }
    assert_eq!(input.data, [64, 0, 0, 255]);
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_noop_prepare_and_runtime_failure_keep_pixels_faults_and_clocks() {
    let mut fixture = gpu();
    let mut preparer = cpu_bridge_renderer();
    let mut reference = cpu_bridge_renderer();
    let scope = protocol::VideoEffectScope::Output { output_id: 101 };
    let mut bypassed = cpu_bridge_chain(scope.clone(), &["not valid ISF"]);
    bypassed.bypassed = true;
    let mut disabled = cpu_bridge_chain(scope.clone(), &["not valid ISF"]);
    disabled.stages[0].enabled = false;
    let cases = [
        (cpu_bridge_chain(scope.clone(), &[]), false, 0, 0),
        (bypassed, false, 0, 0),
        (disabled, false, 0, 0),
        (
            cpu_bridge_chain(
                scope.clone(),
                &["not valid ISF first", "not valid ISF second"],
            ),
            true,
            2,
            1,
        ),
    ];
    for (index, (chain, bgra, fault_count, clock)) in cases.into_iter().enumerate() {
        let mut input = cpu_bridge_input(1, 1, [10, 20, 30, 177]);
        if bgra {
            Arc::make_mut(&mut input).format = VideoPixelFormat::Bgra8;
        }
        let before = input.as_ref().clone();
        let expected = cpu_bridge_oracle(&mut reference, &chain, &input);
        let result = cpu_bridge_run(
            &mut fixture,
            &mut preparer,
            index as u64 + 1,
            chain,
            Arc::clone(&input),
        );
        cpu_bridge_assert(&result, &expected, &scope);
        assert_eq!(result.outcomes()[0].frame().as_ref(), &before);
        assert_eq!(result.outcomes()[0].faults().len(), fault_count);
        assert_eq!(fixture.runtime.native_frame_index(), clock);
        if clock == 0 {
            assert!(result.outcomes()[0].clocks().is_none());
        } else {
            assert_eq!(result.outcomes()[0].clocks().unwrap().frame_index, 0);
        }
        assert_eq!(input.as_ref(), &before);
    }
    let input = cpu_bridge_input(1, 1, [10, 20, 30, 177]);
    let chain = cpu_bridge_chain(scope.clone(), &[IDENTITY, IDENTITY]);
    for renderer in [&mut preparer, &mut reference] {
        renderer.set_isf_date_override(Some([2026.0, 13.0, 26.0, 0.0]));
    }
    let expected = cpu_bridge_oracle(&mut reference, &chain, &input);
    let result = cpu_bridge_run(&mut fixture, &mut preparer, 5, chain, Arc::clone(&input));
    cpu_bridge_assert(&result, &expected, &scope);
    assert_eq!(result.outcomes()[0].frame().as_ref(), input.as_ref());
    assert_eq!(result.outcomes()[0].faults().len(), 2);
    assert!(result.outcomes()[0]
        .faults()
        .iter()
        .all(|fault| fault.message.contains("DATE")));
    assert_eq!(fixture.runtime.native_frame_index(), 2);
    drop(result);
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_reuses_all_three_declined_prepared_jobs_without_duplicate_ticks() {
    let mut fixture = gpu();
    let mut reference_runtime = IsfGpuRuntime::new().unwrap();
    let spec = shader(CPU_BRIDGE_MULTIPASS);
    let prepared_jobs = jobs(8, 4, &[spec.clone()]);
    let inputs = prepared_jobs
        .iter()
        .map(|job| Arc::clone(&job.input))
        .collect::<Vec<_>>();
    let originals = inputs
        .iter()
        .map(|input| input.as_ref().clone())
        .collect::<Vec<_>>();
    let expected = reference(&mut reference_runtime, &prepared_jobs);
    let prepared = fixture
        .runtime
        .prepare_native_inputs(
            request(1),
            prepared_jobs,
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let empty = NativeAllocationPlan::new(request(1));
    let empty_quote = fixture.owner.quote(&empty).unwrap();
    let declined = match fixture.runtime.admit_native_batch(prepared, &empty_quote) {
        NativeBatchAdmission::Declined {
            reason:
                NativeGpuDecline::UnsupportedChain {
                    reason: UnsupportedNativeFx::Multipass,
                },
            prepared,
        } => prepared,
        _ => panic!("resourceful Batch3 must decline before execution"),
    };
    let input_refs = inputs.iter().map(Arc::strong_count).collect::<Vec<_>>();
    let shader_refs = Arc::strong_count(&spec.1);
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    let prepared = fixture
        .runtime
        .prepare_native_cpu_fallback(declined)
        .unwrap();
    assert!(
        inputs
            .iter()
            .zip(input_refs)
            .all(|(input, previous)| Arc::strong_count(input) >= previous),
        "conversion must retain every original prepared input Arc"
    );
    assert_eq!(
        Arc::strong_count(&spec.1),
        shader_refs,
        "conversion reuses the prepared shader Arcs"
    );
    let result = cpu_bridge_run_prepared(&mut fixture, prepared);
    assert_eq!(result.outcomes().len(), 3);
    for (index, ((actual, expected), original)) in result
        .outcomes()
        .iter()
        .zip(&expected)
        .zip(&originals)
        .enumerate()
    {
        assert_eq!(actual.frame().as_ref(), &expected.0);
        assert_eq!(actual.faults(), expected.1.as_slice());
        assert_eq!(actual.frame().layer_id, original.layer_id);
        assert_eq!(
            actual.scope(),
            &protocol::VideoEffectScope::Layer {
                layer_id: original.layer_id
            }
        );
        assert_eq!(actual.clocks().unwrap().frame_index, index as u32);
        assert_eq!(inputs[index].as_ref(), original);
    }
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    drop(result);
    assert!(inputs.iter().all(|input| Arc::strong_count(input) == 1));
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_budget_rejects_before_execution_then_reuses_prepared_input() {
    let mut fixture = gpu();
    let mut preparer = cpu_bridge_renderer();
    let input = cpu_bridge_input(8, 4, [17, 23, 41, 255]);
    let chain = cpu_bridge_chain(
        protocol::VideoEffectScope::Group {
            group_id: protocol::VideoLayerGroupId(7),
        },
        &[CPU_BRIDGE_MULTIPASS],
    );
    let invocation = preparer
        .prepare_native_cpu_invocation(
            chain.clone(),
            Arc::clone(&input),
            Some(cpu_bridge_input_key()),
            None,
        )
        .unwrap();
    let prepared = fixture
        .runtime
        .prepare_native_cpu_inputs(request(1), invocation)
        .unwrap();
    let wrong_invocation = preparer
        .prepare_native_cpu_invocation(
            chain,
            Arc::clone(&input),
            Some(cpu_bridge_input_key()),
            None,
        )
        .unwrap();
    let wrong_inputs = fixture
        .runtime
        .prepare_native_cpu_inputs(request(1), wrong_invocation)
        .unwrap();
    let mut foreign = gpu();
    let before = submission_probe(&mut foreign.owner, 70);
    let quote = foreign
        .owner
        .quote(wrong_inputs.allocation_plan().unwrap())
        .unwrap();
    let foreign_reservation = foreign.owner.reserve(quote).unwrap();
    let rejected = fixture.runtime.render_native_cpu_fallback(
        &mut foreign.owner,
        wrong_inputs,
        foreign_reservation,
        &|| false,
    );
    assert!(matches!(
        rejected,
        Err(NativeCpuRenderFailure::Fault {
            fault: NativeGpuFault {
                kind: NativeGpuFaultKind::IdentityMismatch,
                ..
            },
            retirement: None,
        })
    ));
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    assert_eq!(foreign.owner.budget_snapshot().reserved_bytes, 0);
    assert_eq!(foreign.owner.budget_snapshot().allocated_bytes, 0);
    assert_eq!(
        submission_probe(&mut foreign.owner, 71),
        before + 1,
        "foreign-owner rejection cannot dispatch canonical CPU work"
    );
    cpu_bridge_stop(foreign);
    let mut pressure = NativeAllocationPlan::new(request(80));
    pressure
        .add_new(
            ChargeKind::CpuLastValid,
            crate::native_gpu_budget::NATIVE_MANAGED_LIMIT,
        )
        .unwrap();
    let reservation = fixture
        .owner
        .reserve(fixture.owner.quote(&pressure).unwrap())
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
    assert_eq!(input.data[..4], [17, 23, 41, 255]);
    let pending = fixture
        .owner
        .begin_frame(request(80), reservation)
        .unwrap()
        .finish();
    await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let result = cpu_bridge_run_prepared(&mut fixture, prepared);
    assert_eq!(result.outcomes().len(), 1);
    assert!(result.outcomes()[0]
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [128, 128, 255, 255]));
    assert_eq!(fixture.runtime.native_frame_index(), 1);
    drop(result);
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_admission_and_stale_cache_return_reusable_inputs_and_tokens() {
    let mut fixture = gpu();
    let job = jobs(8, 4, &[shader(CPU_BRIDGE_MULTIPASS)])
        .into_iter()
        .next()
        .unwrap();
    let original = job.input.as_ref().clone();
    let make_invocation = || {
        PreparedNativeCpuInvocation::new(Arc::clone(&job.input), job.chain.clone(), job.date, None)
    };
    let first = fixture
        .runtime
        .prepare_native_cpu_inputs(request(1), make_invocation())
        .unwrap();
    let equal_shape = fixture
        .runtime
        .prepare_native_cpu_inputs(request(1), make_invocation())
        .unwrap();
    let bytes = first.allocation_plan().unwrap().requested_bytes();
    assert_eq!(
        bytes,
        equal_shape.allocation_plan().unwrap().requested_bytes()
    );
    let before = submission_probe(&mut fixture.owner, 80);
    let quote = fixture
        .owner
        .quote(equal_shape.allocation_plan().unwrap())
        .unwrap();
    let wrong_envelope = fixture.owner.reserve(quote).unwrap();
    drop(equal_shape);
    let (returned, reservation) = match fixture.runtime.render_native_cpu_fallback(
        &mut fixture.owner,
        first,
        wrong_envelope,
        &|| false,
    ) {
        Err(NativeCpuRenderFailure::Admission {
            fault,
            prepared,
            reservation,
        }) => {
            assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
            (prepared, reservation)
        }
        other => {
            panic!("wrong envelope must return the untouched inputs and affine token: {other:?}")
        }
    };
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, 0);
    assert_eq!(fixture.owner.budget_snapshot().reserved_bytes, bytes);
    assert_eq!(reservation.context(), fixture.owner.context().id());
    assert_eq!(reservation.frame(), request(1));
    assert_eq!(reservation.capacity_bytes(), bytes);
    assert_eq!(returned.allocation_plan().unwrap().frame(), request(1));
    assert!(
        Arc::strong_count(&job.input) > 1,
        "returned invocation retains the original input Arc"
    );
    fixture
        .owner
        .discard_unsubmitted_reservation(reservation)
        .unwrap();
    assert_eq!(fixture.owner.budget_snapshot().reserved_bytes, 0);
    assert_eq!(submission_probe(&mut fixture.owner, 81), before + 1);
    let returned = fixture
        .runtime
        .prepare_native_cpu_fallback(returned)
        .unwrap();
    let first_result = cpu_bridge_run_prepared(&mut fixture, returned);
    assert_eq!(fixture.runtime.native_frame_index(), 1);
    assert!(first_result.outcomes()[0].faults().is_empty());
    assert!(first_result.outcomes()[0]
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [128, 128, 255, 255]));
    drop(first_result);

    let stale = fixture
        .runtime
        .prepare_native_cpu_inputs(request(2), make_invocation())
        .unwrap();
    let cache_before = fixture.runtime.native_cpu_cache_stamp();
    let mut preparer = cpu_bridge_renderer();
    let prime = cpu_bridge_run(
        &mut fixture,
        &mut preparer,
        3,
        cpu_bridge_chain(
            protocol::VideoEffectScope::Group {
                group_id: protocol::VideoLayerGroupId(7),
            },
            &[CPU_BRIDGE_PERSISTENT],
        ),
        cpu_bridge_input(8, 4, [0, 0, 0, 255]),
    );
    assert!(prime.outcomes()[0].faults().is_empty());
    drop(prime);
    assert_ne!(
        fixture.runtime.native_cpu_cache_stamp(),
        cache_before,
        "the stale plan must be caused by an actual canonical resourceful call"
    );
    let clock = fixture.runtime.native_frame_index();
    assert_eq!(clock, 2);
    let before = submission_probe(&mut fixture.owner, 82);
    let quote = fixture
        .owner
        .quote(stale.allocation_plan().unwrap())
        .unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    let returned = match fixture.runtime.render_native_cpu_fallback(
        &mut fixture.owner,
        stale,
        reservation,
        &|| false,
    ) {
        Err(NativeCpuRenderFailure::Declined { reason, prepared }) => {
            assert_eq!(reason, NativeGpuDecline::QuoteExpired);
            prepared
        }
        other => {
            panic!("a changed resource cache must decline with the original invocation: {other:?}")
        }
    };
    assert_eq!(fixture.runtime.native_frame_index(), clock);
    assert_eq!(fixture.owner.budget_snapshot().reserved_bytes, 0);
    assert_eq!(submission_probe(&mut fixture.owner, 83), before + 1);
    assert_eq!(job.input.as_ref(), &original);
    assert!(Arc::strong_count(&job.input) > 1);
    let shader_refs = Arc::strong_count(&job.chain.stages[0].shader);
    let returned = fixture
        .runtime
        .prepare_native_cpu_fallback(returned)
        .unwrap();
    assert_eq!(
        Arc::strong_count(&job.chain.stages[0].shader),
        shader_refs,
        "replanning retains the prepared shader rather than preparing a replacement"
    );
    assert!(Arc::strong_count(&job.input) > 1);
    let result = cpu_bridge_run_prepared(&mut fixture, returned);
    assert_eq!(result.outcomes().len(), 1);
    assert!(result.outcomes()[0].faults().is_empty());
    assert_eq!(result.outcomes()[0].clocks().unwrap().frame_index, clock);
    assert_eq!(fixture.runtime.native_frame_index(), clock + 1);
    assert!(result.outcomes()[0]
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [128, 128, 255, 255]));
    assert_eq!(job.input.as_ref(), &original);
    drop(result);
    assert_eq!(Arc::strong_count(&job.input), 1);
    cpu_bridge_stop(fixture);
}

#[test]
fn isf_native_gpu_cpu_bridge_handover_stop_and_post_submit_cancel_retain_charged_owners() {
    let mut fixture = gpu();
    let mut preparer = cpu_bridge_renderer();
    let source = cpu_bridge_input(8, 4, [17, 23, 41, 255]);
    let chain = cpu_bridge_chain(
        protocol::VideoEffectScope::Output { output_id: 101 },
        &[CPU_BRIDGE_MULTIPASS],
    );
    let result = cpu_bridge_run(
        &mut fixture,
        &mut preparer,
        1,
        chain.clone(),
        Arc::clone(&source),
    );
    assert!(result.outcomes()[0]
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [128, 128, 255, 255]));
    let cpu_bytes = result.outcomes()[0].frame().data.capacity() as u64;
    let ids = result.outcomes()[0].allocation_ids();
    assert!(
        !ids.is_empty(),
        "owned post-FX CPU result has a live ledger owner"
    );
    let handed_over = result.outcomes()[0].charged_owner();
    assert!(Arc::ptr_eq(
        &handed_over.frame,
        result.outcomes()[0].frame()
    ));
    drop(result);
    let Fixture { mut owner, runtime } = fixture;
    drop(runtime);
    owner.begin_stop();
    assert!(matches!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending { .. }
    ));
    let snapshot = owner.budget_snapshot();
    let cpu = snapshot.for_kind(ChargeKind::CpuSnapshot);
    assert!(
        cpu.active_bytes + cpu.retired_bytes + cpu.quarantined_bytes >= cpu_bytes,
        "handover owner must keep packed CPU capacity charged after Stop"
    );
    assert!(ids.iter().all(|id| id.context == owner.context().id()));
    assert!(handed_over
        .frame
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [128, 128, 255, 255]));
    drop(handed_over);
    stop(&mut owner);

    // Discover the after-submit cancellation boundary from real broker
    // submission IDs rather than claiming that an arbitrary callback count is
    // after GPU work. Each attempt has a fresh owner and an untouched input.
    let mut observed_post_submit_cancel = false;
    for cancel_at in 1..=16 {
        let mut fixture = gpu();
        let mut preparer = cpu_bridge_renderer();
        let source = cpu_bridge_input(8, 4, [17, 23, 41, 255]);
        let original = source.as_ref().clone();
        let invocation = preparer
            .prepare_native_cpu_invocation(
                chain.clone(),
                Arc::clone(&source),
                Some(cpu_bridge_input_key()),
                None,
            )
            .unwrap();
        let prepared = fixture
            .runtime
            .prepare_native_cpu_inputs(request(1), invocation)
            .unwrap();
        let before = submission_probe(&mut fixture.owner, 50);
        let quote = fixture
            .owner
            .quote(prepared.allocation_plan().unwrap())
            .unwrap();
        let reservation = fixture.owner.reserve(quote).unwrap();
        let checks = std::cell::Cell::new(0);
        let cancel = || {
            let next = checks.get() + 1;
            checks.set(next);
            next >= cancel_at
        };
        let result = fixture.runtime.render_native_cpu_fallback(
            &mut fixture.owner,
            prepared,
            reservation,
            &cancel,
        );
        let retirement = match &result {
            Err(NativeCpuRenderFailure::Cancelled { retirement }) => *retirement,
            Err(error) => panic!("cancellation must remain typed Cancelled: {error:?}"),
            Ok(_) => None,
        };
        let snapshot = fixture.owner.budget_snapshot();
        let retained = snapshot.allocated_bytes;
        let render_bytes = [ChargeKind::IsfScratch, ChargeKind::SnapshotReadback]
            .into_iter()
            .map(|kind| {
                let charge = snapshot.for_kind(kind);
                charge.active_bytes + charge.retired_bytes + charge.quarantined_bytes
            })
            .sum::<u64>();
        let clock = fixture.runtime.native_frame_index();
        let was_cancelled = result.is_err();
        assert_eq!(source.as_ref(), &original);
        drop(result);
        // Cancellation returns ownership to the retirement registry. Drain
        // that transaction before issuing another frame through this owner;
        // the one-output-inflight guard must remain active until it retires.
        let drain_started = Instant::now();
        loop {
            match fixture.owner.poll_retirement().unwrap() {
                RetirementProgress::Pending {
                    transactions: 0, ..
                }
                | RetirementProgress::Reaped => break,
                RetirementProgress::Pending { .. } => {
                    assert!(
                        drain_started.elapsed() < Duration::from_secs(5),
                        "cancelled CPU transaction must drain before a new frame"
                    );
                    std::thread::yield_now();
                }
            }
        }
        let after = submission_probe(&mut fixture.owner, 51);
        let submitted_during_bridge = after - before - 1;
        eprintln!("native CPU cancellation checkpoint={cancel_at} checks={} cancelled={was_cancelled} submissions={submitted_during_bridge} render_bytes={render_bytes} clock={clock} retirement={retirement:?}", checks.get());
        if was_cancelled && submitted_during_bridge > 0 && retirement.is_some() && render_bytes > 0
        {
            assert_eq!(retirement.unwrap().context, fixture.owner.context().id());
            assert!(
                retained > 0,
                "inflight/cancelled CPU resources remain charged before drain"
            );
            assert_eq!(
                clock, 1,
                "cancel never retries the original canonical chain"
            );
            observed_post_submit_cancel = true;
        }
        cpu_bridge_stop(fixture);
        if observed_post_submit_cancel {
            break;
        }
    }
    assert!(
        observed_post_submit_cancel,
        "must observe a real submitted canonical CPU transaction cancelled without output"
    );
}
