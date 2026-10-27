use super::*;
use crate::isf_native_gpu::tests::{gpu, jobs, request, shader, stop, Fixture, IDENTITY};
use crate::isf_native_gpu::{await_transaction, PreparedNativeWork};
use crate::native_gpu_contract::{LayerOrigin, MaterializationFailure};
use crate::native_gpu_result::{CpuOutcomeProvenance, NativeIsfCompletion};

const DATE: [f32; 4] = [2026.0, 10.0, 26.0, 42.0];
const CLOCK_DATE: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { gl_FragColor=vec4(float(FRAMEINDEX)/255.0, DATE.w/255.0, TIME/255.0,1.0); }"#;
const AUDIO_DATE: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"spectrum","TYPE":"audioFFT"}]}*/
void main() { gl_FragColor=vec4(float(FRAMEINDEX)/255.0, DATE.w/255.0, IMG_NORM_PIXEL(spectrum,vec2(0.5)).r,1.0); }"#;
const INCREMENT: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.r+1.0/255.0,c.gba); }"#;

fn renderer(runtime: IsfGpuRuntime) -> crate::VideoPreviewRenderer {
    let mut renderer = crate::VideoPreviewRenderer::new(crate::VideoRuntimeConfig::default());
    renderer.isf_runtime = Some(Ok(runtime));
    renderer.isf_render_namespace = (7, Some(101), false);
    renderer.set_isf_date_override(Some(DATE));
    renderer
}

fn layer_requests(specs: &[&'static str]) -> StackBatch<NativeLayerPreparation> {
    let specs = specs
        .iter()
        .map(|source| shader(source))
        .collect::<Vec<_>>();
    jobs(3, 2, &specs).map(|job| NativeLayerPreparation {
        input: job.input,
        chain: job.chain.source.into_owned(),
        entry: job.entry,
        source_generation: job.source_generation,
        content_sequence: job.content_sequence,
    })
}

fn accepted<'runtime>(
    prepared: NativePreparedContinuation<'runtime>,
    owner: &mut NativeGpuOwner,
) -> (NativeAdmittedContinuation<'runtime>, FrameReservation) {
    let quote = owner.quote(prepared.allocation_plan().unwrap()).unwrap();
    let admitted = match prepared.admit(&quote) {
        NativeContinuationAdmission::Accepted(admitted) => admitted,
        NativeContinuationAdmission::Declined { reason, .. } => {
            panic!("eligible affine invocation declined: {reason:?}")
        }
    };
    (admitted, owner.reserve(quote).unwrap())
}

fn execute(
    admitted: NativeAdmittedContinuation<'_>,
    owner: &mut NativeGpuOwner,
    reservation: FrameReservation,
) -> NativeIsfCompletion {
    admitted.with_runtime(|runtime, batch| {
        let mut txn = owner.begin_frame(request(1), reservation).unwrap();
        let mut encoded = runtime.encode_native_batch(&mut txn, batch).unwrap();
        let pending = txn.finish();
        let receipt = encoded.pending.take().unwrap();
        let mut resolved = await_transaction(owner, pending, &|| false).unwrap();
        let phase = resolved.take_phase(receipt).unwrap();
        runtime.complete_native_batch(encoded, phase, owner, &|| false)
    })
}

fn run_cpu(
    continuation: NativeCpuContinuation<'_>,
    owner: &mut NativeGpuOwner,
) -> NativeCpuResults {
    let quote = owner
        .quote(continuation.allocation_plan().unwrap())
        .unwrap();
    let reservation = owner.reserve(quote).unwrap();
    continuation.run(owner, reservation, &|| false).unwrap()
}

fn clean(renderer: crate::VideoPreviewRenderer, owner: &mut NativeGpuOwner) {
    drop(renderer);
    stop(owner);
}

#[test]
fn isf_native_gpu_g4_continuation_queries_before_encode_share_renderer_clock_and_pixels() {
    let Fixture { mut owner, runtime } = gpu();
    let context = Arc::clone(owner.context());
    let mut renderer = renderer(runtime);
    let input = layer_requests(&[CLOCK_DATE]);
    let original = input
        .iter()
        .map(|job| Arc::clone(&job.input))
        .collect::<Vec<_>>();
    let before = owner.budget_snapshot();
    let prepared = renderer
        .prepare_native_layer_continuation(
            request(1),
            input,
            NativeBatchRestrictions::default(),
            None,
        )
        .unwrap();
    assert!(Arc::ptr_eq(
        prepared.runtime.native_context().unwrap(),
        &context
    ));
    let first = prepared.prospective_identities().unwrap();
    for _ in 0..3 {
        assert_eq!(&*prepared.prospective_identities().unwrap(), &*first);
        assert!(prepared.allocation_plan().is_ok());
    }
    assert_eq!(prepared.runtime.native_frame_index(), 0);
    assert_eq!(owner.budget_snapshot(), before);
    let PreparedNativeWork::Layers(jobs) = &prepared.inputs.work else {
        panic!("resident work");
    };
    for (job, original) in jobs.iter().zip(&original) {
        assert!(Arc::ptr_eq(&job.input, original), "no input copy or decode");
        assert_eq!(job.date, DATE);
        assert_eq!(job.chain.keys[0].input, Some(job.entry.input));
    }
    let (admitted, reservation) = accepted(prepared, &mut owner);
    let NativeIsfCompletion::Validated { layers, receipt } =
        execute(admitted, &mut owner, reservation)
    else {
        panic!("real batch must validate");
    };
    assert!(receipt.phase().validation().is_ok());
    assert_eq!(
        renderer.isf_shader_cache_clock, 3,
        "prepare once per authored chain"
    );
    let runtime = renderer.isf_runtime.as_mut().unwrap().as_mut().unwrap();
    assert_eq!(runtime.native_frame_index(), 3);
    for (index, layer) in layers.iter().enumerate() {
        assert_eq!(layer.identity, first[index]);
        assert!(
            matches!(layer.identity.origin, LayerOrigin::Executed(clocks)
            if clocks.frame_index == index as u32 && clocks.date_bits == DATE.map(f32::to_bits))
        );
        let copy = runtime
            .materialize_validated_layer(layer.identity.entry, &layer.lease, &mut owner, &|| false)
            .unwrap();
        assert!(copy
            .frame()
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [index as u8, 42, (index + 1) as u8, 255]));
    }
    assert_eq!(
        runtime.native_frame_index(),
        3,
        "materialization is not another tick"
    );
    drop(layers);
    clean(renderer, &mut owner);
}

#[test]
fn isf_native_gpu_g4_decline_keeps_frozen_audio_date_keys_and_prepared_cpu_work() {
    let Fixture {
        mut owner,
        mut runtime,
    } = gpu();
    let old_audio = protocol::LiveAudioTextureFrame {
        generation: 1,
        feature_sequence: 1,
        sample_rate: 48_000,
        waveform: [0.0; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [0.0; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
    };
    runtime.set_audio_frame(Some(&old_audio));
    let mut renderer = renderer(runtime);
    let mut frozen_audio = old_audio.clone();
    frozen_audio.feature_sequence = 2;
    frozen_audio.fft.fill(128.0 / 255.0);
    renderer.set_isf_audio_frame(Some(&frozen_audio));
    let input = layer_requests(&[AUDIO_DATE]);
    let originals = input
        .iter()
        .map(|job| Arc::clone(&job.input))
        .collect::<Vec<_>>();
    let prepared = renderer
        .prepare_native_layer_continuation(
            request(1),
            input,
            NativeBatchRestrictions::default(),
            None,
        )
        .unwrap();
    assert_eq!(prepared.inputs.audio.as_ref(), Some(&frozen_audio));
    assert_eq!(
        prepared.runtime.native_audio_snapshot().as_ref(),
        Some(&old_audio)
    );
    assert!(
        prepared.allocation_plan().is_err(),
        "audio work declines resident Batch3"
    );
    assert_eq!(prepared.runtime.native_frame_index(), 0);
    let quote = owner.quote(&NativeAllocationPlan::new(request(1))).unwrap();
    let continuation = match prepared.admit(&quote) {
        NativeContinuationAdmission::Declined {
            reason,
            continuation,
        } => {
            assert!(matches!(reason, NativeGpuDecline::UnsupportedChain { .. }));
            continuation
        }
        _ => panic!("audio inputs must keep canonical CPU continuation"),
    };
    let cpu = continuation.into_cpu().unwrap().refresh().unwrap();
    assert_eq!(cpu.runtime.native_frame_index(), 0);
    let results = run_cpu(cpu, &mut owner);
    assert_eq!(results.outcomes().len(), 3);
    for (index, outcome) in results.outcomes().iter().enumerate() {
        assert!(outcome.faults().is_empty());
        assert_eq!(outcome.clocks().unwrap().frame_index, index as u32);
        assert_eq!(outcome.clocks().unwrap().date_bits, DATE.map(f32::to_bits));
        assert_eq!(
            outcome.scope(),
            &protocol::VideoEffectScope::Layer {
                layer_id: originals[index].layer_id,
            }
        );
        assert!(outcome
            .frame()
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [index as u8, 42, 128, 255]));
        assert!(outcome
            .allocation_ids()
            .iter()
            .all(|id| id.context == owner.context().id()));
    }
    assert_eq!(
        renderer.isf_shader_cache_clock, 3,
        "decline/refresh never reprepare"
    );
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        3
    );
    drop(results);
    clean(renderer, &mut owner);
}

#[test]
fn isf_native_gpu_g4_unexecuted_quote_admission_can_return_without_reserving_another_tick() {
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let prepared = renderer
        .prepare_native_layer_continuation(
            request(1),
            layer_requests(&[CLOCK_DATE]),
            NativeBatchRestrictions::default(),
            None,
        )
        .unwrap();
    let identities = prepared.prospective_identities().unwrap();
    let quote = owner.quote(prepared.allocation_plan().unwrap()).unwrap();
    let NativeContinuationAdmission::Accepted(admitted) = prepared.admit(&quote) else {
        panic!("eligible resident quote");
    };
    let other = NativeAllocationPlan::new(request(9));
    let other_quote = owner.quote(&other).unwrap();
    let other_reservation = owner.reserve(other_quote).unwrap();
    assert!(matches!(
        owner.reserve(quote),
        Err(NativeGpuDecline::QuoteExpired)
    ));
    owner
        .discard_unsubmitted_reservation(other_reservation)
        .unwrap();
    let returned = admitted.return_unexecuted();
    assert_eq!(returned.runtime.native_frame_index(), 0);
    assert_eq!(&*returned.prospective_identities().unwrap(), &*identities);
    let results = run_cpu(returned.into_cpu().unwrap(), &mut owner);
    for (index, outcome) in results.outcomes().iter().enumerate() {
        assert_eq!(outcome.clocks().unwrap().frame_index, index as u32);
        assert!(outcome
            .frame()
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [index as u8, 42, (index + 1) as u8, 255]));
    }
    assert_eq!(renderer.isf_shader_cache_clock, 3);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        3
    );
    drop(results);
    clean(renderer, &mut owner);
}

#[test]
fn isf_native_gpu_g4_group_cpu_rejection_returns_owner_and_exact_preparation_faults() {
    let Fixture { mut owner, runtime } = gpu();
    let mut foreign = gpu();
    let mut renderer = renderer(runtime);
    let mut requests = layer_requests(&[IDENTITY, IDENTITY, IDENTITY]);
    let job = requests.drain().next().unwrap();
    let input = job.input;
    let key = job.entry.input;
    let mut chain = job.chain;
    chain.scope = protocol::VideoEffectScope::Group {
        group_id: protocol::VideoLayerGroupId(99),
    };
    chain.stages[1].effect.source = Arc::from("this is not a GLSL shader");
    let mut reference = crate::VideoPreviewRenderer::new(crate::VideoRuntimeConfig::default());
    reference.isf_render_namespace = (7, Some(101), false);
    reference.set_isf_date_override(Some(DATE));
    let (expected_frame, expected_faults) = reference
        .execute_effect_chain_cow(&chain, Cow::Borrowed(input.as_ref()), Some(key), None)
        .unwrap();
    let expected_frame = expected_frame.into_owned();
    let cpu = renderer
        .prepare_native_cpu_continuation(
            request(1),
            chain.clone(),
            Arc::clone(&input),
            Some(key),
            None,
        )
        .unwrap();
    let foreign_quote = foreign.owner.quote(cpu.allocation_plan().unwrap()).unwrap();
    let foreign_reservation = foreign.owner.reserve(foreign_quote).unwrap();
    let cpu = match cpu.run(&mut owner, foreign_reservation, &|| false) {
        Err(NativeCpuContinuationFailure::Admission {
            fault,
            reservation,
            continuation,
        }) => {
            assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
            assert_eq!(continuation.runtime.native_frame_index(), 0);
            foreign
                .owner
                .discard_unsubmitted_reservation(reservation)
                .unwrap();
            continuation
        }
        other => panic!("wrong reservation must preserve the CPU invocation: {other:?}"),
    };
    let result = run_cpu(cpu, &mut owner);
    assert_eq!(result.outcomes().len(), 1);
    let outcome = &result.outcomes()[0];
    assert_eq!(outcome.frame().as_ref(), &expected_frame);
    assert_eq!(outcome.faults(), expected_faults.as_slice());
    assert_eq!(outcome.scope(), &chain.scope);
    assert_eq!(outcome.faults().len(), 1);
    assert_eq!(outcome.faults()[0].stage_index, Some(1));
    assert_eq!(outcome.clocks().unwrap().frame_index, 0);
    assert_eq!(
        renderer.isf_shader_cache_clock, 3,
        "retry admission did not reprepare"
    );
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        1
    );
    drop(result);
    clean(renderer, &mut owner);
    drop(foreign.runtime);
    stop(&mut foreign.owner);
}

#[test]
fn isf_native_gpu_g4_admitted_recovery_preserves_faults_and_original_three_clocks() {
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let requests = layer_requests(&[INCREMENT, INCREMENT, INCREMENT]);
    let original = requests
        .iter()
        .map(|job| Arc::clone(&job.input))
        .collect::<Vec<_>>();
    let mut prepared = renderer
        .prepare_native_layer_continuation(
            request(1),
            requests,
            NativeBatchRestrictions::default(),
            None,
        )
        .unwrap();
    let PreparedNativeWork::Layers(jobs) = &mut prepared.inputs.work else {
        panic!("layers");
    };
    Arc::make_mut(&mut Arc::get_mut(&mut jobs[1]).unwrap().chain.stages[1].shader).wgsl =
        "not valid WGSL".into();
    let (admitted, reservation) = accepted(prepared, &mut owner);
    let NativeIsfCompletion::RecoveredCpu { layers } = execute(admitted, &mut owner, reservation)
    else {
        panic!("must retain fixed-clock recovery");
    };
    for (index, layer) in layers.iter().enumerate() {
        assert_eq!(layer.provenance, CpuOutcomeProvenance::FixedClockRecovery);
        let increment = if index == 1 { 2 } else { 3 };
        for (actual, before) in layer
            .frame
            .data
            .chunks_exact(4)
            .zip(original[index].data.chunks_exact(4))
        {
            assert_eq!(
                actual,
                [before[0] + increment, before[1], before[2], before[3]]
            );
        }
        assert!(
            matches!(layer.identity.origin, LayerOrigin::Executed(clocks)
            if clocks.frame_index == index as u32)
        );
    }
    assert!(layers[0].faults.is_empty() && layers[2].faults.is_empty());
    assert_eq!(layers[1].faults.len(), 1);
    let fault = &layers[1].faults[0];
    assert_eq!(
        fault.scope,
        protocol::VideoEffectScope::Layer { layer_id: 2 }
    );
    assert_eq!(fault.stage_id, Some(protocol::VideoEffectStageId(21)));
    assert_eq!(fault.effect_id, Some(protocol::VideoEffectId(201)));
    assert_eq!(fault.stage_index, Some(1));
    assert_eq!(fault.stage_label.as_deref(), Some("layer-2-stage-1"));
    assert_eq!(renderer.isf_shader_cache_clock, 9);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        3
    );
    drop(layers);
    clean(renderer, &mut owner);
}

#[test]
fn isf_native_gpu_g4_cancellation_before_prepare_and_after_submit_never_restarts_tick() {
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let before = owner.budget_snapshot();
    match renderer.prepare_native_layer_continuation(
        request(1),
        layer_requests(&[IDENTITY]),
        NativeBatchRestrictions::default(),
        Some(&|| true),
    ) {
        Err(NativePreparationFailure::Runtime(error)) => {
            assert!(matches!(error, VideoRuntimeError::Cancelled));
        }
        _ => panic!("cancel must remain a runtime cancellation"),
    }
    assert_eq!(renderer.isf_shader_cache_clock, 0);
    assert_eq!(owner.budget_snapshot(), before);
    let prepared = renderer
        .prepare_native_layer_continuation(
            request(1),
            layer_requests(&[IDENTITY]),
            NativeBatchRestrictions::default(),
            None,
        )
        .unwrap();
    let (admitted, reservation) = accepted(prepared, &mut owner);
    let terminal = admitted.with_runtime(|runtime, batch| {
        let mut txn = owner.begin_frame(request(1), reservation).unwrap();
        let encoded = runtime.encode_native_batch(&mut txn, batch).unwrap();
        let pending = txn.finish();
        assert_eq!(runtime.native_frame_index(), 3);
        let terminal = await_transaction(&mut owner, pending, &|| true);
        assert!(owner.budget_snapshot().managed_bytes() > 0);
        drop(encoded);
        terminal
    });
    assert!(matches!(
        terminal,
        Err(MaterializationFailure::Cancelled {
            retirement: Some(_)
        })
    ));
    assert_eq!(renderer.isf_shader_cache_clock, 3);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        3
    );
    clean(renderer, &mut owner);
}

#[test]
fn isf_native_gpu_g4_continuation_refuses_missing_runtime_and_cpu_cancel_keeps_exact_outcome() {
    let mut standalone = crate::VideoPreviewRenderer::new(crate::VideoRuntimeConfig::default());
    match standalone.prepare_native_layer_continuation(
        request(1),
        layer_requests(&[IDENTITY]),
        NativeBatchRestrictions::default(),
        None,
    ) {
        Err(NativePreparationFailure::Fault(fault)) => {
            assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
        }
        _ => panic!("must not silently create an independent runtime/device"),
    }
    assert!(standalone.isf_runtime.is_none());
    assert_eq!(standalone.isf_shader_cache_clock, 0);
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let mut requests = layer_requests(&[IDENTITY]);
    let job = requests.drain().next().unwrap();
    let cpu = renderer
        .prepare_native_cpu_continuation(
            request(1),
            job.chain,
            job.input,
            Some(job.entry.input),
            None,
        )
        .unwrap();
    let quote = owner.quote(cpu.allocation_plan().unwrap()).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    match cpu.run(&mut owner, reservation, &|| true) {
        Err(NativeCpuContinuationFailure::Cancelled { retirement }) => {
            assert!(retirement.is_none())
        }
        other => panic!("CPU cancel must not become a retry or ordinary success: {other:?}"),
    }
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        0
    );
    assert_eq!(renderer.isf_shader_cache_clock, 1);
    clean(renderer, &mut owner);
}

#[test]
fn isf_native_gpu_g4_cpu_decline_refresh_resumes_original_preparation_once() {
    fn submission_probe(owner: &mut NativeGpuOwner, sequence: u64) -> u64 {
        let frame = request(sequence);
        let quote = owner.quote(&NativeAllocationPlan::new(frame)).unwrap();
        let reservation = owner.reserve(quote).unwrap();
        let mut txn = owner.begin_frame(frame, reservation).unwrap();
        let phase = txn.begin_phase(GpuPhase::Initialize).unwrap();
        let encoded = phase.finish(Vec::new());
        let receipt = txn.submit_phase(encoded).unwrap();
        let submission = receipt.submission().sequence;
        let pending = txn.finish();
        await_transaction(owner, pending, &|| false)
            .unwrap()
            .take_phase(receipt)
            .unwrap()
            .into_validation()
            .unwrap();
        submission
    }

    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let frozen_audio = protocol::LiveAudioTextureFrame {
        generation: 3,
        feature_sequence: 11,
        sample_rate: 48_000,
        waveform: [0.0; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [128.0 / 255.0; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
    };
    renderer.set_isf_audio_frame(Some(&frozen_audio));
    let mut requests = layer_requests(&[AUDIO_DATE, IDENTITY, IDENTITY]);
    let job = requests.drain().next().unwrap();
    let input = job.input;
    let original = input.as_ref().clone();
    let key = job.entry.input;
    let mut chain = job.chain;
    chain.scope = protocol::VideoEffectScope::Group {
        group_id: protocol::VideoLayerGroupId(99),
    };
    chain.stages[1].effect.source = Arc::from("this is not a GLSL shader");
    let mut reference = crate::VideoPreviewRenderer::new(crate::VideoRuntimeConfig::default());
    reference.isf_render_namespace = (7, Some(101), false);
    let expected = reference
        .prepare_effect_chain(Cow::Owned(chain.clone()), input.layer_id, Some(key), None)
        .unwrap();
    assert_eq!(expected.faults.len(), 1);

    // Defensive adapter fixture: the public affine API prevents another runtime
    // call while its continuation exists. Prepare genuine lower-level inputs,
    // change the real cache before constructing that borrow, and then exercise
    // the adapter's return/refresh path without a tick during the continuation.
    // Evicting an existing charged alias instead would fail broker admission
    // before the CPU bridge reaches its cache-stamp decline.
    let invocation = renderer
        .prepare_native_cpu_invocation(chain.clone(), Arc::clone(&input), Some(key), None)
        .unwrap();
    let runtime = renderer.isf_runtime.as_ref().unwrap().as_ref().unwrap();
    let inputs = runtime
        .prepare_native_cpu_inputs(request(2), invocation)
        .unwrap();
    let initial_cache = runtime.native_cpu_cache_stamp();
    assert_eq!(runtime.native_frame_index(), 0);
    assert!(inputs.allocation_plan().is_ok());
    assert_eq!(renderer.isf_shader_cache_clock, 3);
    let retained_input_refs = Arc::strong_count(&input);
    assert!(retained_input_refs > 1);

    let mut later_audio = frozen_audio.clone();
    later_audio.feature_sequence += 1;
    later_audio.fft.fill(64.0 / 255.0);
    renderer.set_isf_audio_frame(Some(&later_audio));
    renderer.set_isf_date_override(Some([2026.0, 10.0, 26.0, 93.0]));
    renderer.isf_render_namespace = (8, Some(101), false);
    let mut warm_requests = layer_requests(&[AUDIO_DATE]);
    let warm = warm_requests.drain().next().unwrap();
    let prime = renderer
        .prepare_native_cpu_continuation(
            request(1),
            warm.chain,
            warm.input,
            Some(warm.entry.input),
            None,
        )
        .unwrap();
    let prime = run_cpu(prime, &mut owner);
    assert!(prime.outcomes()[0].faults().is_empty());
    assert!(prime.outcomes()[0]
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [0, 93, 64, 255]));
    drop(prime);
    assert_eq!(renderer.isf_shader_cache_clock, 4);
    let runtime = renderer.isf_runtime.as_mut().unwrap().as_mut().unwrap();
    let warmed_cache = runtime.native_cpu_cache_stamp();
    assert_ne!(warmed_cache, initial_cache);
    assert_eq!(runtime.native_frame_index(), 1);
    let cpu = NativeCpuContinuation { runtime, inputs };
    let before = submission_probe(&mut owner, 81);
    let quote = owner.quote(cpu.allocation_plan().unwrap()).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let cpu = match cpu.run(&mut owner, reservation, &|| false) {
        Err(NativeCpuContinuationFailure::Declined {
            reason,
            continuation,
        }) => {
            assert_eq!(reason, NativeGpuDecline::QuoteExpired);
            continuation
        }
        other => panic!("a real cache change must return unexecuted CPU work: {other:?}"),
    };
    assert_eq!(cpu.runtime.native_frame_index(), 1);
    assert_eq!(cpu.runtime.native_cpu_cache_stamp(), warmed_cache);
    assert_eq!(owner.budget_snapshot().reserved_bytes, 0);
    assert_eq!(submission_probe(&mut owner, 82), before + 1);
    assert_eq!(Arc::strong_count(&input), retained_input_refs);
    assert_eq!(input.as_ref(), &original);
    let cpu = cpu.refresh().unwrap();
    assert_eq!(cpu.inputs.frame, request(2));
    assert_eq!(cpu.runtime.native_frame_index(), 1);
    assert_eq!(Arc::strong_count(&input), retained_input_refs);
    let result = run_cpu(cpu, &mut owner);
    assert_eq!(result.outcomes().len(), 1);
    let outcome = &result.outcomes()[0];
    assert_eq!(outcome.faults(), expected.faults.as_slice());
    assert_eq!(outcome.faults()[0].render_input_key, Some(key));
    assert_eq!(outcome.faults()[0].stage_index, Some(1));
    assert_eq!(outcome.scope(), &chain.scope);
    let clocks = outcome.clocks().unwrap();
    assert_eq!(clocks.frame_index, 1);
    assert_eq!(clocks.date_bits, DATE.map(f32::to_bits));
    assert_eq!(
        clocks.time_seconds_bits,
        (original.pts_ms as f32 / 1000.0).to_bits()
    );
    assert_eq!(
        clocks.time_delta_seconds_bits,
        (original.duration_ms as f32 / 1000.0).to_bits()
    );
    assert!(outcome
        .frame()
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [1, 42, 128, 255]));
    assert_eq!(
        renderer.isf_shader_cache_clock, 4,
        "refresh did not reprepare"
    );
    let runtime = renderer.isf_runtime.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        runtime.native_frame_index(),
        2,
        "resume executes exactly once"
    );
    assert_eq!(
        runtime.native_audio_snapshot().as_ref(),
        Some(&frozen_audio)
    );
    assert_eq!(renderer.isf_audio_frame.as_ref(), Some(&later_audio));
    assert_eq!(input.as_ref(), &original);
    drop(result);
    owner.poll_retirement().unwrap();
    assert_eq!(Arc::strong_count(&input), 1);
    clean(renderer, &mut owner);
}
