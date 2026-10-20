use super::*;

const AUDIO: &str = include_str!("isf_fixtures/audio.fs");

fn audio_frame(
    generation: u64,
    sequence: u64,
    wave: f32,
    fft: f32,
) -> protocol::LiveAudioTextureFrame {
    protocol::LiveAudioTextureFrame {
        generation,
        feature_sequence: sequence,
        sample_rate: 48_000,
        waveform: [wave; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [fft; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
    }
}

fn image_frame() -> VideoFrame {
    VideoFrame {
        layer_id: 1,
        width: 1,
        height: 1,
        pts_ms: 123,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: vec![0, 0, 0, 255],
    }
}

#[test]
fn isf_audio_prepares_bounded_mono_inputs_without_scalar_controls() {
    let shader = prepare_isf_shader(AUDIO).unwrap();
    assert!(shader.controls.is_empty());
    assert_eq!(shader.audio_inputs.len(), 2);
    assert!(!shader.audio_inputs[0].fft);
    assert!(shader.audio_inputs[1].fft);
    assert_eq!(shader.audio_inputs[0].width, 512);
    for invalid in ["0", "513", "-1", "1.5", "true", "\"512\""] {
        let source = AUDIO.replace("\"MAX\":512", &format!("\"MAX\":{invalid}"));
        assert!(prepare_isf_shader(&source)
            .unwrap_err()
            .message
            .contains("MAX"));
    }
    let missing_max = AUDIO.replace(",\"MAX\":512", "");
    assert_eq!(
        prepare_isf_shader(&missing_max).unwrap().audio_inputs[0].width,
        512
    );
}

#[test]
fn isf_audio_counts_against_shared_texture_budget_and_accepts_generators() {
    let inputs = (0..16)
        .map(|index| format!("{{\"NAME\":\"wave{index}\",\"TYPE\":\"audio\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("/*{{\"INPUTS\":[{{\"NAME\":\"inputImage\",\"TYPE\":\"image\"}},{inputs}]}}*/ void main() {{ gl_FragColor = vec4(1.0); }}");
    assert!(prepare_isf_shader(&source)
        .unwrap_err()
        .message
        .contains("too many textures"));
    let missing_image = AUDIO.replace("{\"NAME\":\"inputImage\",\"TYPE\":\"image\"},", "");
    let generator = prepare_isf_shader(&missing_image).unwrap();
    assert!(generator.image_input_names.is_empty());
    assert_eq!(generator.audio_inputs.len(), 2);
    let image_inputs = (0..4)
        .map(|index| format!("{{\"NAME\":\"image{index}\",\"TYPE\":\"image\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    for audio_count in [12, 13] {
        let audio_inputs = (0..audio_count)
            .map(|index| format!("{{\"NAME\":\"wave{index}\",\"TYPE\":\"audio\"}}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("/*{{\"INPUTS\":[{image_inputs},{audio_inputs}]}}*/ void main() {{ gl_FragColor = vec4(1.0); }}");
        let prepared = prepare_isf_shader(&source);
        if audio_count == 12 {
            let prepared = prepared.unwrap();
            assert_eq!(prepared.image_input_names.len(), 4);
            assert_eq!(prepared.audio_inputs.len(), 12);
        } else {
            assert!(prepared.unwrap_err().message.contains("too many textures"));
        }
    }
}

#[test]
fn isf_audio_fixture_uploads_real_samples_and_clears_on_stop_or_invalid_frame() {
    let mut runtime = IsfGpuRuntime::new().expect("audio texture fixture requires a GPU adapter");
    let shader = prepare_isf_shader(AUDIO).unwrap();
    let frame = image_frame();
    let render =
        |runtime: &mut IsfGpuRuntime| runtime.apply(&frame, &shader, &[], 0.0, 0.033).unwrap();
    assert_eq!(render(&mut runtime).data, [128, 0, 255, 255]);
    runtime.set_audio_frame(Some(&audio_frame(1, 1, -0.5, 0.75)));
    assert_eq!(render(&mut runtime).data, [64, 191, 255, 255]);
    // A publication is immutable per identity: unchanged identities reuse data.
    runtime.set_audio_frame(Some(&audio_frame(1, 1, 1.0, 0.0)));
    assert_eq!(render(&mut runtime).data, [64, 191, 255, 255]);
    runtime.set_audio_frame(Some(&audio_frame(1, 2, 0.5, 0.25)));
    assert_eq!(render(&mut runtime).data, [191, 64, 255, 255]);
    runtime.set_audio_frame(Some(&audio_frame(2, 1, -1.0, 1.0)));
    assert_eq!(render(&mut runtime).data, [0, 255, 255, 255]);
    runtime.set_audio_frame(None);
    assert_eq!(render(&mut runtime).data, [128, 0, 255, 255]);
    runtime.set_audio_frame(Some(&audio_frame(3, 1, -0.5, 0.75)));
    assert_eq!(render(&mut runtime).data, [64, 191, 255, 255]);
    runtime.set_audio_frame(Some(&audio_frame(3, 2, f32::NAN, 1.0)));
    assert_eq!(render(&mut runtime).data, [128, 0, 255, 255]);
    assert_eq!(render(&mut runtime).pts_ms, 123);
}

#[test]
fn isf_audio_max_resamples_waveform_and_keeps_fft_peaks() {
    let mut runtime = IsfGpuRuntime::new().expect("audio texture fixture requires a GPU adapter");
    let source = AUDIO.replace("\"MAX\":512", "\"MAX\":2").replace(
        "IMG_SIZE(audioWave).x / 512.0",
        "IMG_SIZE(audioWave).x / 2.0",
    );
    let shader = prepare_isf_shader(&source).unwrap();
    let mut audio = audio_frame(1, 1, 0.0, 0.0);
    audio.waveform[0] = -1.0;
    audio.waveform[511] = 1.0;
    audio.fft[127] = 0.75;
    runtime.set_audio_frame(Some(&audio));
    assert_eq!(
        runtime
            .apply(&image_frame(), &shader, &[], 0.0, 0.033)
            .unwrap()
            .data,
        [0, 191, 255, 255]
    );
    let last = prepare_isf_shader(&source.replace("vec2(0.0, 0.5)", "vec2(1.0, 0.5)")).unwrap();
    assert_eq!(
        runtime
            .apply(&image_frame(), &last, &[], 0.0, 0.033)
            .unwrap()
            .data,
        [255, 0, 255, 255]
    );
}

#[test]
fn isf_audio_max_one_reports_rms_of_the_complete_window() {
    let mut runtime = IsfGpuRuntime::new().expect("audio texture fixture requires a GPU adapter");
    let source = AUDIO
        .replace("\"MAX\":512", "\"MAX\":1")
        .replace("IMG_SIZE(audioWave).x / 512.0", "IMG_SIZE(audioWave).x");
    let shader = prepare_isf_shader(&source).unwrap();
    let mut audio = audio_frame(1, 1, 0.0, 0.0);
    for (index, sample) in audio.waveform.iter_mut().enumerate() {
        *sample = if index % 2 == 0 { -1.0 } else { 1.0 };
    }
    runtime.set_audio_frame(Some(&audio));
    assert_eq!(
        runtime
            .apply(&image_frame(), &shader, &[], 0.0, 0.033)
            .unwrap()
            .data,
        [255, 0, 255, 255]
    );
    runtime.set_audio_frame(None);
    assert_eq!(
        runtime
            .apply(&image_frame(), &shader, &[], 0.0, 0.033)
            .unwrap()
            .data,
        [128, 0, 255, 255]
    );
}

#[test]
fn isf_audio_combines_named_images_date_and_multipass_without_stale_uploads() {
    const SOURCE: &str = r#"/*{
      "INPUTS":[
        {"NAME":"overlayImage","TYPE":"image"},
        {"NAME":"audioWave","TYPE":"audio","MAX":2},
        {"NAME":"inputImage","TYPE":"image"},
        {"NAME":"audioSpectrum","TYPE":"audioFFT","MAX":2}
      ],
      "PASSES":[{"TARGET":"mixedBuffer"},{}]
    }*/
    void main() {
      if (PASSINDEX == 0) {
        gl_FragColor = vec4(
          (IMG_THIS_NORM_PIXEL(inputImage).r + IMG_THIS_NORM_PIXEL(overlayImage).r) * 0.5,
          IMG_NORM_PIXEL(audioWave, vec2(0.0, 0.5)).r,
          IMG_NORM_PIXEL(audioSpectrum, vec2(0.0, 0.5)).r,
          DATE.w / 1024.0);
      } else {
        gl_FragColor = IMG_THIS_NORM_PIXEL(mixedBuffer);
      }
    }"#;
    let shader = prepare_isf_shader_with_image_bindings(
        SOURCE,
        None,
        &[protocol::VideoIsfImageBinding {
            name: "overlayImage".into(),
            source: protocol::VideoIsfImageSource::Layer { layer_id: 2 },
        }],
    )
    .unwrap();
    assert_eq!(shader.image_input_names, ["inputImage", "overlayImage"]);
    let mut runtime = IsfGpuRuntime::new().expect("mixed ISF fixture requires a GPU adapter");
    let mut primary = image_frame();
    primary.data = vec![64, 0, 0, 255];
    let mut overlay = image_frame();
    overlay.layer_id = 2;
    overlay.data = vec![192, 0, 0, 255];
    let date = [2026.0, 10.0, 20.0, 512.0];
    let render = |runtime: &mut IsfGpuRuntime, overlay: &VideoFrame| {
        let images = [IsfImageInput {
            name: "overlayImage",
            frame: overlay,
        }];
        runtime
            .apply_stack_with_inputs(
                &primary,
                &[IsfStageInputs {
                    shader: &shader,
                    controls: &[],
                    images: &images,
                }],
                0.0,
                0.033,
                date,
            )
            .unwrap()
    };
    assert_eq!(render(&mut runtime, &overlay).data, [128, 128, 0, 128]);
    runtime.set_audio_frame(Some(&audio_frame(1, 1, -0.5, 0.75)));
    assert_eq!(render(&mut runtime, &overlay).data, [128, 64, 191, 128]);
    // Named image pixels can change while one immutable audio publication is reused.
    overlay.data = vec![32, 0, 0, 255];
    runtime.set_audio_frame(Some(&audio_frame(1, 1, 1.0, 0.0)));
    assert_eq!(render(&mut runtime, &overlay).data, [48, 64, 191, 128]);
    runtime.set_audio_frame(Some(&audio_frame(1, 2, 0.5, 0.25)));
    assert_eq!(render(&mut runtime, &overlay).data, [48, 191, 64, 128]);
    runtime.set_audio_frame(None);
    assert_eq!(render(&mut runtime, &overlay).data, [48, 128, 0, 128]);
    let missing = runtime
        .apply_stack_with_inputs(
            &primary,
            &[IsfStageInputs {
                shader: &shader,
                controls: &[],
                images: &[],
            }],
            0.0,
            0.033,
            date,
        )
        .unwrap_err();
    assert!(
        missing.message.contains("layer 2 is unavailable"),
        "{missing}"
    );
    let unbound = prepare_isf_shader(SOURCE).unwrap();
    let transparent = runtime
        .apply_stack_with_inputs(
            &primary,
            &[IsfStageInputs {
                shader: &unbound,
                controls: &[],
                images: &[],
            }],
            0.0,
            0.033,
            date,
        )
        .unwrap();
    assert_eq!(transparent.data, [32, 128, 0, 128]);

    let generator_source = AUDIO.replace("{\"NAME\":\"inputImage\",\"TYPE\":\"image\"},", "");
    let generator = prepare_isf_shader(&generator_source).unwrap();
    assert!(generator.image_input_names.is_empty());
    runtime.set_audio_frame(Some(&audio_frame(2, 1, 0.5, 0.25)));
    let generated = runtime
        .apply_stack_with_inputs(
            &primary,
            &[IsfStageInputs {
                shader: &generator,
                controls: &[],
                images: &[],
            }],
            0.0,
            0.033,
            date,
        )
        .unwrap();
    assert_eq!(generated.data, [191, 64, 255, 255]);
}
