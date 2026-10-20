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
fn isf_audio_counts_against_texture_budget_and_requires_the_host_image() {
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
    assert!(prepare_isf_shader(&missing_image)
        .unwrap_err()
        .message
        .contains("exactly one image"));
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
