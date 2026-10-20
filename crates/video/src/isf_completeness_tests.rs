use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const MULTIPASS: &str = include_str!("isf_fixtures/multipass.fs");
const PERSISTENT: &str = include_str!("isf_fixtures/persistent.fs");
const FLOAT: &str = include_str!("isf_fixtures/float.fs");
const IMPORTED: &str = include_str!("isf_fixtures/imported.fs");
const POINT: &str = include_str!("isf_fixtures/point2d.fs");
const COLOR: &str = include_str!("isf_fixtures/color.fs");
const EVENT: &str = include_str!("isf_fixtures/event.fs");

fn frame(width: u32, height: u32) -> VideoFrame {
    VideoFrame {
        layer_id: 23,
        width,
        height,
        pts_ms: 1_000,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: [0, 0, 0, 255].repeat(width as usize * height as usize),
    }
}

fn gpu() -> IsfGpuRuntime {
    IsfGpuRuntime::new().expect("T23 known-output ISF tests require a GPU adapter")
}

fn assert_pixels(frame: &VideoFrame, expected: [u8; 4]) {
    for pixel in frame.data.chunks_exact(4) {
        assert_eq!(pixel, expected);
    }
}

#[test]
fn multipass_target_dimensions_passindex_and_image_size_have_known_output() {
    let shader = prepare_isf_shader(MULTIPASS).unwrap();
    let output = gpu().apply(&frame(8, 4), &shader, &[], 1.0, 0.016).unwrap();
    assert_eq!((output.width, output.height), (8, 4));
    assert_pixels(&output, [128, 128, 255, 255]);
}

#[test]
fn persistent_feedback_survives_frames_but_not_resize_or_instance_change() {
    let shader = prepare_isf_shader(PERSISTENT).unwrap();
    let mut runtime = gpu();
    let input = frame(1, 1);
    let mut key = IsfInstanceKey::legacy(input.layer_id, 0);
    key.namespace = (1, Some(42), false);
    let render = |runtime: &mut IsfGpuRuntime, input: &VideoFrame, key: &IsfInstanceKey| {
        let (output, failures) = runtime.apply_stack_stage_local_with_keys(
            input,
            &[(&shader, &[])],
            1.0,
            0.016,
            std::slice::from_ref(key),
        );
        assert!(failures.is_empty(), "{failures:?}");
        output
    };
    assert_pixels(&render(&mut runtime, &input, &key), [1, 0, 0, 255]);
    assert_pixels(&render(&mut runtime, &input, &key), [2, 0, 0, 255]);
    let mut preview = key.clone();
    preview.namespace.2 = true;
    assert_pixels(&render(&mut runtime, &input, &preview), [1, 0, 0, 255]);
    assert_pixels(&render(&mut runtime, &input, &key), [3, 0, 0, 255]);
    let mut other_stage = key.clone();
    other_stage.legacy_stage_index = 1;
    assert_pixels(&render(&mut runtime, &input, &other_stage), [1, 0, 0, 255]);
    let mut other_input = key.clone();
    other_input.input = Some(protocol::VideoRenderInputKey {
        project_render_epoch: 1,
        input_id: protocol::VideoRenderInputId(99),
    });
    assert_pixels(&render(&mut runtime, &input, &other_input), [1, 0, 0, 255]);
    key.namespace.0 = 2;
    assert_pixels(&render(&mut runtime, &input, &key), [1, 0, 0, 255]);
    assert_pixels(&render(&mut runtime, &frame(2, 1), &key), [1, 0, 0, 255]);
}

#[test]
fn nonpersistent_target_is_cleared_each_frame() {
    let shader =
        prepare_isf_shader(&PERSISTENT.replace("\"PERSISTENT\":true", "\"PERSISTENT\":false"))
            .unwrap();
    let mut runtime = gpu();
    for index in 0..3 {
        let output = runtime
            .apply(&frame(1, 1), &shader, &[], index as f32, 0.016)
            .unwrap();
        assert_pixels(&output, [1, 0, 0, 255]);
    }
}

#[test]
fn float_target_preserves_negative_hdr_and_full_32_bit_precision() {
    let shader = prepare_isf_shader(FLOAT).unwrap();
    let mut runtime = gpu();
    eprintln!(
        "T23 FLOAT32_FILTERABLE={}",
        runtime
            .device
            .features()
            .contains(wgpu::Features::FLOAT32_FILTERABLE)
    );
    let result = runtime.apply(&frame(2, 1), &shader, &[], 0.0, 0.016);
    if runtime
        .device
        .features()
        .contains(wgpu::Features::FLOAT32_FILTERABLE)
    {
        assert_pixels(&result.unwrap(), [255, 128, 128, 255]);
    } else {
        assert!(result
            .unwrap_err()
            .message
            .contains("32-bit float texture filtering"));
    }
}

#[test]
fn final_float_target_is_converted_to_host_dimensions_and_rgba8() {
    let source = r#"/*{
        "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
        "PASSES":[{"TARGET":"resultBuffer","WIDTH":"1","HEIGHT":"1","FLOAT":true}]
    }*/
    void main() { gl_FragColor = vec4(0.25, 0.5, 0.75, 1.0); }"#;
    let shader = prepare_isf_shader(source).unwrap();
    let mut runtime = gpu();
    let result = runtime.apply(&frame(4, 2), &shader, &[], 0.0, 0.016);
    if runtime
        .device
        .features()
        .contains(wgpu::Features::FLOAT32_FILTERABLE)
    {
        let output = result.unwrap();
        assert_eq!(
            (output.width, output.height, output.format),
            (4, 2, VideoPixelFormat::Rgba8)
        );
        assert_pixels(&output, [64, 128, 191, 255]);
    } else {
        assert!(result
            .unwrap_err()
            .message
            .contains("32-bit float texture filtering"));
    }
}

struct FixtureDirectory(PathBuf);
impl FixtureDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("t23-isf-fixtures");
        let path = root.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn shader(&self) -> PathBuf {
        let path = self.0.join("filter.fs");
        fs::write(&path, IMPORTED).unwrap();
        path
    }

    fn image(&self, data: &[u8]) {
        image::save_buffer_with_format(
            self.0.join("two-pixels.png"),
            data,
            2,
            1,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .unwrap();
    }
}
impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove only the test-owned fixture directory");
    }
}

#[test]
fn imported_image_pixels_use_the_image_dimensions_and_shader_relative_path() {
    let directory = FixtureDirectory::new();
    directory.image(&[255, 0, 0, 255, 0, 255, 0, 255]);
    let shader_path = directory.shader();
    let shader = prepare_isf_shader_with_path(IMPORTED, Some(&shader_path)).unwrap();
    let output = gpu().apply(&frame(4, 1), &shader, &[], 0.0, 0.016).unwrap();
    assert_eq!(
        output.data,
        [0, 255, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
    );
}

#[test]
fn shader_cache_keeps_identical_sources_in_different_pack_directories_separate() {
    let first = FixtureDirectory::new();
    first.image(&[255, 0, 0, 255, 0, 255, 0, 255]);
    let second = FixtureDirectory::new();
    second.image(&[0, 0, 255, 255, 255, 255, 255, 255]);
    let first_path = first.shader();
    let second_path = second.shader();
    let source = Arc::<str>::from(IMPORTED);
    let mut renderer = crate::VideoPreviewRenderer::new(crate::VideoRuntimeConfig::default());
    let a = renderer
        .cached_isf_shader(&source, first_path.to_str(), &[])
        .unwrap();
    let b = renderer
        .cached_isf_shader(&source, second_path.to_str(), &[])
        .unwrap();
    assert_ne!(a.fingerprint, b.fingerprint);
    assert_eq!(renderer.isf_shader_cache_count(), 2);
    let cached = renderer
        .cached_isf_shader(&source, first_path.to_str(), &[])
        .unwrap();
    assert!(Arc::ptr_eq(&a, &cached));
}

#[test]
fn imported_images_missing_assets_and_companion_vertex_shaders_fail_closed() {
    assert!(prepare_isf_shader(IMPORTED)
        .unwrap_err()
        .message
        .contains("ISF imported image"));
    let directory = FixtureDirectory::new();
    let path = directory.shader();
    assert!(prepare_isf_shader_with_path(IMPORTED, Some(&path))
        .unwrap_err()
        .message
        .contains("ISF imported image"));
    directory.image(&[255; 8]);
    fs::write(path.with_extension("vs"), "void main() {}").unwrap();
    assert!(prepare_isf_shader_with_path(IMPORTED, Some(&path))
        .unwrap_err()
        .message
        .contains("vertex shaders"));
}

#[test]
fn point2d_default_and_control_override_have_known_output() {
    let shader = prepare_isf_shader(POINT).unwrap();
    let effect = isf_effect_from_prepared("Point".into(), POINT, None, &shader);
    let defaults = resolved_isf_control_values(&shader, &effect, None);
    assert_pixels(
        &gpu()
            .apply(&frame(1, 1), &shader, &defaults, 0.0, 0.016)
            .unwrap(),
        [64, 191, 0, 255],
    );
    assert_pixels(
        &gpu()
            .apply(&frame(1, 1), &shader, &[[0.5, 0.25, 0.0, 0.0]], 0.0, 0.016)
            .unwrap(),
        [128, 64, 0, 255],
    );
}

#[test]
fn color_input_preserves_all_four_components() {
    let shader = prepare_isf_shader(COLOR).unwrap();
    let effect = isf_effect_from_prepared("Color".into(), COLOR, None, &shader);
    let defaults = resolved_isf_control_values(&shader, &effect, None);
    assert_pixels(
        &gpu()
            .apply(&frame(1, 1), &shader, &defaults, 0.0, 0.016)
            .unwrap(),
        [64, 128, 191, 128],
    );
}

#[test]
fn event_input_is_identical_in_all_passes_and_obeys_host_reset() {
    let shader = prepare_isf_shader(EVENT).unwrap();
    assert_eq!(shader.controls[0].kind, IsfControlKind::Event);
    let mut runtime = gpu();
    assert_pixels(
        &runtime
            .apply(&frame(1, 1), &shader, &[[1.0, 0.0, 0.0, 0.0]], 0.0, 0.016)
            .unwrap(),
        [255, 0, 0, 255],
    );
    assert_pixels(
        &runtime
            .apply(&frame(1, 1), &shader, &[[0.0; 4]], 0.016, 0.016)
            .unwrap(),
        [0, 0, 0, 255],
    );
}

#[test]
fn invalid_pass_metadata_and_oversized_dimensions_are_rejected() {
    for replacement in [
        r#""PASSES":[{"TARGET":"TIME"}]"#,
        r#""PASSES":[{"PERSISTENT":true}]"#,
        r#""PASSES":[{"TARGET":"inputImage"}]"#,
        r#""PASSES":[{"TARGET":"a"},{"TARGET":"a"}]"#,
    ] {
        let source = format!(
            r#"/*{{"INPUTS":[{{"NAME":"inputImage","TYPE":"image"}}],{replacement}}}*/ void main() {{ gl_FragColor = vec4(1.0); }}"#
        );
        assert!(prepare_isf_shader(&source).is_err(), "{source}");
    }
    let source = MULTIPASS.replace("$WIDTH / 2", "1000000");
    let shader = prepare_isf_shader(&source).unwrap();
    let error = gpu()
        .apply(&frame(8, 4), &shader, &[], 0.0, 0.016)
        .unwrap_err();
    assert!(error.message.contains("ISF pass size"), "{error}");
}

#[test]
fn this_pixel_and_this_norm_pixel_differ_for_a_resized_imported_image() {
    let directory = FixtureDirectory::new();
    directory.image(&[255, 0, 0, 255, 0, 255, 0, 255]);
    let shader_path = directory.shader();
    let metadata = IMPORTED.split("void main()").next().unwrap();
    let pixel_source =
        format!("{metadata}void main() {{ gl_FragColor = IMG_THIS_PIXEL(testImage); }}");
    let norm_source = pixel_source.replace("IMG_THIS_PIXEL", "IMG_THIS_NORM_PIXEL");
    let pixel = prepare_isf_shader_with_path(&pixel_source, Some(&shader_path)).unwrap();
    let norm = prepare_isf_shader_with_path(&norm_source, Some(&shader_path)).unwrap();
    let mut runtime = gpu();
    assert_eq!(
        runtime
            .apply(&frame(4, 1), &pixel, &[], 0.0, 0.016)
            .unwrap()
            .data,
        [255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255],
    );
    assert_eq!(
        runtime
            .apply(&frame(4, 1), &norm, &[], 0.0, 0.016)
            .unwrap()
            .data,
        [255, 0, 0, 255, 191, 64, 0, 255, 64, 191, 0, 255, 0, 255, 0, 255],
    );
}

#[test]
fn unnamed_intermediates_are_rejected_before_exceeding_gpu_memory_budget() {
    let pass = serde_json::json!({"WIDTH":4096,"HEIGHT":4096});
    let metadata = serde_json::json!({
        "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
        "PASSES":vec![pass; 5]
    });
    let source = format!("/*{metadata}*/ void main() {{ gl_FragColor=vec4(1.0); }}");
    let shader = prepare_isf_shader(&source).unwrap();
    let mut runtime = gpu();
    let error = runtime
        .apply(&frame(1, 1), &shader, &[], 0.0, 0.016)
        .unwrap_err();
    assert!(
        error.message.contains("GPU buffer memory budget"),
        "{}",
        error.message
    );
    let working = prepare_isf_shader(POINT).unwrap();
    assert_pixels(
        &runtime
            .apply(
                &frame(1, 1),
                &working,
                &[working.controls[0].default],
                0.0,
                0.016,
            )
            .unwrap(),
        [64, 191, 0, 255],
    );
}
