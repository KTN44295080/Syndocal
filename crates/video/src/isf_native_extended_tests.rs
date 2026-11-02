use super::*;

struct TestDir(std::path::PathBuf);
impl TestDir {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "syndocal-isf-resident-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Polls until no transaction or allocation of the live owner remains. A live
/// owner reports `Reaped` only after Stop.
fn drain(fixture: &mut Fixture, what: &str) {
    let started = Instant::now();
    loop {
        match fixture.owner.poll_retirement().unwrap() {
            RetirementProgress::Pending {
                transactions: 0,
                allocations: 0,
            } => return,
            RetirementProgress::Pending { .. } if started.elapsed() < Duration::from_secs(5) => {
                std::thread::yield_now()
            }
            other => panic!("{what} did not reap: {other:?}"),
        }
    }
}

fn first_job(source: TestShader) -> StackBatch<PreparedNativeLayer> {
    let mut result = StackBatch::Empty;
    result.push(jobs(8, 4, &[source]).into_iter().next().unwrap());
    result
}

fn pixels(fixture: &mut Fixture, layer: &ValidatedLayerResult) -> VideoFrame {
    NativeGpuMaterializer::new(&mut fixture.runtime)
        .materialize_cpu(
            CpuMaterializationRequest::ValidatedGpu {
                entry: layer.identity.entry,
                lease: &layer.lease,
            },
            &mut fixture.owner,
            &|| false,
        )
        .unwrap()
        .frame()
        .clone()
}

#[test]
fn isf_native_gpu_resident_multipass_custom_sizes_and_float_match_canonical() {
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let mut cases = vec![shader(CPU_BRIDGE_MULTIPASS), shader(MULTIPASS)];
    if fixture
        .owner
        .capabilities()
        .enabled_features
        .contains(wgpu::Features::FLOAT32_FILTERABLE)
    {
        cases.push(shader(CPU_BRIDGE_FLOAT));
    }
    for (index, source) in cases.into_iter().enumerate() {
        fixture.owner.release_resource_pools();
        let prepared = first_job(source);
        let expected = reference(&mut cpu, &prepared);
        let (layers, receipt) = validated(&mut fixture, index as u64 + 1, prepared);
        assert!(expected[0].1.is_empty());
        assert_eq!(pixels(&mut fixture, &layers[0]), expected[0].0);
        // The execution itself has no readback allocation; only the explicit
        // materialization above requests one after resident validation.
        drop(receipt);
        drop(layers);
    }
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_named_final_target_is_published_exactly() {
    const NAMED: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
        "PASSES":[{"TARGET":"resultBuffer","WIDTH":"$WIDTH","HEIGHT":"$HEIGHT"}]}*/
        void main(){ gl_FragColor=vec4(0.25,0.5,0.75,0.5); }"#;
    let mut fixture = gpu();
    let (layers, receipt) = validated(&mut fixture, 1, first_job(shader(NAMED)));
    assert!(pixels(&mut fixture, &layers[0])
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [64, 128, 191, 128]));
    drop((layers, receipt));
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_feedback_tracks_ticks_and_keeps_all_eight_history_images_exact() {
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let mut history = std::collections::VecDeque::new();
    for tick in 1..=20 {
        let prepared = first_job(shader(CPU_BRIDGE_PERSISTENT));
        let expected = reference(&mut cpu, &prepared);
        let (layers, receipt) = validated(&mut fixture, tick, prepared);
        assert_eq!(pixels(&mut fixture, &layers[0]), expected[0].0);
        assert_eq!(expected[0].0.data[0], tick as u8);
        drop(receipt);
        history.push_back((layers, expected[0].0.clone()));
        if history.len() > 8 {
            history.pop_front();
        }
        for (layers, expected) in history.iter().rev() {
            assert_eq!(
                &pixels(&mut fixture, &layers[0]),
                expected,
                "feedback history is the original validated image, never re-executed state"
            );
            assert!(matches!(
                fixture
                    .runtime
                    .replay_allocation_plan(request(100 + tick), &layers[0].recipe),
                Err(NativeGpuDecline::UnsupportedChain {
                    reason: UnsupportedNativeFx::PersistentTarget
                })
            ));
        }
    }
    drop(history);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_audio_fft_upload_is_current_and_silence_resets() {
    const AUDIO: &str = r#"/*{"INPUTS":[{"NAME":"wave","TYPE":"audio","MAX":512},
        {"NAME":"spectrum","TYPE":"audioFFT","MAX":512}]}*/
        void main(){ gl_FragColor=vec4(IMG_NORM_PIXEL(wave,vec2(0.0,0.5)).r,
            IMG_NORM_PIXEL(spectrum,vec2(0.0,0.5)).r,0.0,1.0); }"#;
    let mut fixture = gpu();
    for (tick, wave, fft, expected) in [
        (1, -0.5, 0.75, [64, 191, 0, 255]),
        (2, 0.5, 0.25, [191, 64, 0, 255]),
        (3, 0.0, 0.0, [128, 0, 0, 255]),
    ] {
        let audio = protocol::LiveAudioTextureFrame {
            generation: 1,
            feature_sequence: tick,
            sample_rate: 48_000,
            waveform: [wave; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
            fft: [fft; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        };
        fixture
            .runtime
            .set_audio_frame((tick != 3).then_some(&audio));
        let (layers, receipt) = validated(&mut fixture, tick, first_job(shader(AUDIO)));
        assert!(pixels(&mut fixture, &layers[0])
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == expected));
        drop((layers, receipt));
    }
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_imported_png_and_bound_second_still_match_canonical() {
    let dir = TestDir::new();
    let image = image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
    image.save(dir.path().join("two-pixels.png")).unwrap();
    let source_path = dir.path().join("effect.fs");
    let imported = Arc::new(
        crate::prepare_isf_shader_with_path(CPU_BRIDGE_IMPORTED, Some(&source_path)).unwrap(),
    );
    let mut fixture = gpu();
    let mut cpu = IsfGpuRuntime::new().unwrap();
    let prepared = first_job((CPU_BRIDGE_IMPORTED, imported));
    let expected = reference(&mut cpu, &prepared);
    let (layers, receipt) = validated(&mut fixture, 1, prepared);
    assert_eq!(pixels(&mut fixture, &layers[0]), expected[0].0);
    drop((layers, receipt));
    fixture.owner.release_resource_pools();
    const SECOND: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},
        {"NAME":"other","TYPE":"image"}]}*/
        void main(){gl_FragColor=IMG_NORM_PIXEL(other,vec2(0.25,0.5));}"#;
    let bindings = [protocol::VideoIsfImageBinding {
        name: "other".into(),
        source: protocol::VideoIsfImageSource::Imported {
            path: "two-pixels.png".into(),
        },
    }];
    let bound = Arc::new(
        crate::prepare_isf_shader_with_image_bindings(SECOND, Some(&source_path), &bindings)
            .unwrap(),
    );
    let prepared = first_job((SECOND, bound));
    let expected = reference(&mut cpu, &prepared);
    let (layers, receipt) = validated(&mut fixture, 2, prepared);
    assert_eq!(pixels(&mut fixture, &layers[0]), expected[0].0);
    drop((layers, receipt));
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_cancel_and_abandoned_encode_release_feedback_before_reuse() {
    let mut fixture = gpu();
    let (encoded, pending) = encode(&mut fixture, 1, first_job(shader(CPU_BRIDGE_PERSISTENT)));
    assert!(matches!(
        await_transaction(&mut fixture.owner, pending, &|| true),
        Err(MaterializationFailure::Cancelled { .. })
    ));
    drop(encoded);
    drain(&mut fixture, "cancelled feedback");
    // Cancel invalidates the G-owned pool. A later valid tick starts from
    // transparent history and never sees an unvalidated feedback write.
    let (layers, receipt) = validated(&mut fixture, 2, first_job(shader(CPU_BRIDGE_PERSISTENT)));
    assert_eq!(pixels(&mut fixture, &layers[0]).data[0], 1);
    drop((layers, receipt));
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_gpu_validation_failure_does_not_commit_or_replay_feedback() {
    let mut fixture = gpu();
    let make = || {
        let mut result = StackBatch::Empty;
        result.push(
            jobs(8, 4, &[shader(CPU_BRIDGE_PERSISTENT), shader(IDENTITY)])
                .into_iter()
                .next()
                .unwrap(),
        );
        result
    };
    let (old, receipt) = validated(&mut fixture, 1, make());
    let expected = pixels(&mut fixture, &old[0]);
    drop(receipt);
    let mut bad = make();
    Arc::make_mut(&mut bad[0].chain.stages[1].shader).wgsl = "invalid WGSL".into();
    let (encoded, pending) = encode(&mut fixture, 2, bad);
    assert!(matches!(
        complete(&mut fixture, encoded, pending),
        NativeIsfCompletion::Failed { .. }
    ));
    assert_eq!(pixels(&mut fixture, &old[0]), expected);
    drop(old);
    let (layers, receipt) = validated(&mut fixture, 3, make());
    assert_eq!(
        pixels(&mut fixture, &layers[0]).data[0],
        1,
        "failed GPU scopes invalidate feedback; recovery cannot invent another stateful tick"
    );
    drop((layers, receipt));
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_abandoned_transaction_reaps_pending_state_and_restarts_cleanly() {
    let mut fixture = gpu();
    fixture.owner.configure_stack_pool_shape([
        Some((8, 4, VideoPixelFormat::Rgba8, 1)),
        None,
        None,
    ]);
    let prepared = fixture
        .runtime
        .prepare_native_inputs(
            request(1),
            first_job(shader(CPU_BRIDGE_PERSISTENT)),
            NativeBatchRestrictions::default(),
        )
        .unwrap();
    let quote = fixture
        .owner
        .quote(prepared.allocation_plan().unwrap())
        .unwrap();
    let NativeBatchAdmission::Accepted(admitted) =
        fixture.runtime.admit_native_batch(prepared, &quote)
    else {
        panic!("feedback fixture must admit")
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(request(1), reservation).unwrap();
    let encoded = fixture
        .runtime
        .encode_native_batch(&mut txn, admitted)
        .unwrap();
    // Deliberately omit finish(): the broker must keep all queued owners until
    // the abandoned transaction drains, while invalidating idle selection.
    drop(txn);
    drop(encoded);
    drain(&mut fixture, "abandoned feedback");
    let (layers, receipt) = validated(&mut fixture, 2, first_job(shader(CPU_BRIDGE_PERSISTENT)));
    assert_eq!(pixels(&mut fixture, &layers[0]).data[0], 1);
    drop((layers, receipt));
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_mixed_neutral_and_extended_pool_keeps_eight_generations() {
    let mut fixture = gpu();
    let mut history = std::collections::VecDeque::new();
    for tick in 1..=20 {
        let mut prepared = jobs(8, 4, &[shader(CPU_BRIDGE_PERSISTENT)]);
        let neutral = jobs(8, 4, &[shader(IDENTITY)]).into_iter().nth(1).unwrap();
        prepared[1] = neutral;
        let (layers, receipt) = validated(&mut fixture, tick, prepared);
        assert_eq!(pixels(&mut fixture, &layers[0]).data[0], tick as u8);
        assert_eq!(pixels(&mut fixture, &layers[2]).data[0], tick as u8);
        assert_eq!(pixels(&mut fixture, &layers[1]).data[0], 85);
        drop(receipt);
        history.push_back(layers);
        if history.len() > 8 {
            history.pop_front();
        }
    }
    drop(history);
    stop(&mut fixture.owner);
}

#[test]
fn isf_native_gpu_resident_stage_memory_and_additional_pass_ceiling_decline_without_work() {
    const TOO_LARGE: &str = r#"/*{"PASSES":[{"TARGET":"giantBuffer","WIDTH":8192,"HEIGHT":8192,"PERSISTENT":true}]}*/
        void main(){ gl_FragColor=vec4(0.0); }"#;
    const FOUR: &str = r#"/*{"PASSES":[{},{},{},{}]}*/ void main(){gl_FragColor=vec4(0.0);}"#;
    let mut fixture = gpu();
    let before = fixture.owner.budget_snapshot();
    for (prepared, reason) in [
        (
            first_job(shader(TOO_LARGE)),
            UnsupportedNativeFx::AuxiliaryResource,
        ),
        (jobs(8, 4, &[shader(FOUR)]), UnsupportedNativeFx::Multipass),
    ] {
        let prepared = fixture
            .runtime
            .prepare_native_inputs(request(1), prepared, NativeBatchRestrictions::default())
            .unwrap();
        assert_eq!(
            prepared.allocation_plan().unwrap_err(),
            NativeGpuDecline::UnsupportedChain { reason }
        );
        assert_eq!(fixture.runtime.native_frame_index(), 0);
        assert_eq!(fixture.owner.budget_snapshot(), before);
    }
    stop(&mut fixture.owner);
}
