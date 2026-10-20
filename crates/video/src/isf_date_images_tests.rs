use super::*;

const DATE_FIXTURE: &str = include_str!("isf_fixtures/date.fs");
const GENERATOR: &str = include_str!("isf_fixtures/generator.fs");
const TWO_IMAGES: &str = include_str!("isf_fixtures/two_images.fs");
const FIXED_DATE: [f32; 4] = [2026.0, 10.0, 20.0, 43200.25];

fn t32_frame(width: u32, height: u32, pixel: [u8; 4]) -> VideoFrame {
    VideoFrame {
        layer_id: 32,
        width,
        height,
        pts_ms: 1000,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: pixel.repeat(width as usize * height as usize),
    }
}

fn t32_gpu() -> IsfGpuRuntime {
    IsfGpuRuntime::new().expect("T32 known-pixel acceptance requires a GPU adapter")
}

fn t32_pixels(frame: &VideoFrame, pixel: [u8; 4]) {
    assert!(
        frame.data.chunks_exact(4).all(|actual| actual == pixel),
        "{:?}",
        frame.data
    );
}

#[test]
fn isf_date_injected_clock_is_frozen_across_frames_and_passes() {
    let shader = prepare_isf_shader(DATE_FIXTURE).unwrap();
    let input = t32_frame(2, 1, [255, 255, 255, 255]);
    let stages = [IsfStageInputs {
        shader: &shader,
        controls: &[],
        images: &[],
    }];
    let mut runtime = t32_gpu();
    for time in [0.0, 12.0] {
        let output = runtime
            .apply_stack_with_inputs(&input, &stages, time, 0.016, FIXED_DATE)
            .unwrap();
        t32_pixels(&output, [26, 10, 20, 64]);
        assert_eq!(
            (output.layer_id, output.pts_ms, output.duration_ms),
            (32, 1000, 16)
        );
    }
    let output = runtime
        .apply_stack_with_inputs(&input, &stages, 20.0, 0.016, [2030.0, 12.0, 1.0, 43200.5])
        .unwrap();
    t32_pixels(&output, [30, 12, 1, 128]);
    assert!(runtime
        .apply_stack_with_inputs(&input, &stages, 0.0, 0.016, [f32::NAN; 4])
        .unwrap_err()
        .message
        .contains("DATE"));
}

#[test]
fn isf_zero_image_generator_ignores_host_pixels_and_uses_render_size() {
    let shader = prepare_isf_shader(GENERATOR).unwrap();
    assert!(shader.image_input_name.is_empty());
    assert!(shader.image_input_names.is_empty());
    let output = t32_gpu()
        .apply(&t32_frame(8, 4, [1, 2, 3, 4]), &shader, &[], 0.0, 0.016)
        .unwrap();
    t32_pixels(&output, [128, 64, 191, 255]);
    assert_eq!((output.width, output.height), (8, 4));
}

#[test]
fn isf_two_images_bind_by_name_with_transparent_default_and_real_dimensions() {
    let shader = prepare_isf_shader(TWO_IMAGES).unwrap();
    assert_eq!(shader.image_input_names, ["inputImage", "overlayImage"]);
    let input = t32_frame(2, 1, [0, 0, 255, 255]);
    let overlay = t32_frame(1, 1, [255, 0, 0, 255]);
    let images = [IsfImageInput {
        name: "overlayImage",
        frame: &overlay,
    }];
    let mut runtime = t32_gpu();
    let output = runtime
        .apply_stack_with_inputs(
            &input,
            &[IsfStageInputs {
                shader: &shader,
                controls: &[],
                images: &images,
            }],
            0.0,
            0.016,
            FIXED_DATE,
        )
        .unwrap();
    t32_pixels(&output, [128, 0, 128, 255]);
    let output = runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap();
    t32_pixels(&output, [0, 0, 128, 128]);
    let size_source = TWO_IMAGES.replace(
        "mix(IMG_THIS_NORM_PIXEL(inputImage), IMG_THIS_NORM_PIXEL(overlayImage), 0.5)",
        "vec4(IMG_SIZE(overlayImage) / 255.0, 0.0, 1.0)",
    );
    let sizes = prepare_isf_shader(&size_source).unwrap();
    let output = runtime
        .apply_stack_with_inputs(
            &input,
            &[IsfStageInputs {
                shader: &sizes,
                controls: &[],
                images: &images,
            }],
            0.0,
            0.016,
            FIXED_DATE,
        )
        .unwrap();
    t32_pixels(&output, [1, 1, 0, 255]);
}

#[test]
fn isf_image_input_bounds_and_binding_names_fail_explicitly() {
    let source = |count| {
        format!(
            "/*{{\"INPUTS\":[{}]}}*/ void main() {{ gl_FragColor = vec4(1.0); }}",
            (0..count)
                .map(|index| format!("{{\"NAME\":\"image{index}\",\"TYPE\":\"image\"}}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    assert_eq!(
        prepare_isf_shader(&source(4))
            .unwrap()
            .image_input_names
            .len(),
        4
    );
    assert!(prepare_isf_shader(&source(5))
        .unwrap_err()
        .message
        .contains("image inputs"));
    let invalid = protocol::VideoIsfImageBinding {
        name: "missingImage".into(),
        source: protocol::VideoIsfImageSource::None {},
    };
    assert!(
        prepare_isf_shader_with_image_bindings(TWO_IMAGES, None, &[invalid])
            .unwrap_err()
            .message
            .contains("declared image input")
    );
    let mut runtime = t32_gpu();
    let shader = prepare_isf_shader(TWO_IMAGES).unwrap();
    let input = t32_frame(1, 1, [0, 0, 0, 255]);
    let bad = VideoFrame {
        data: Vec::new(),
        ..input.clone()
    };
    let images = [IsfImageInput {
        name: "overlayImage",
        frame: &bad,
    }];
    assert!(runtime
        .apply_stack_with_inputs(
            &input,
            &[IsfStageInputs {
                shader: &shader,
                controls: &[],
                images: &images
            }],
            0.0,
            0.016,
            FIXED_DATE
        )
        .unwrap_err()
        .message
        .contains("byte length"));
    let good_images = [IsfImageInput {
        name: "overlayImage",
        frame: &input,
    }; 2];
    assert!(runtime
        .apply_stack_with_inputs(
            &input,
            &[IsfStageInputs {
                shader: &shader,
                controls: &[],
                images: &good_images
            }],
            0.0,
            0.016,
            FIXED_DATE
        )
        .unwrap_err()
        .message
        .contains("duplicated"));
}

#[test]
fn isf_dynamic_image_updates_preserve_feedback_and_missing_input_never_reuses_old_pixels() {
    let source = r#"/*{
        "INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"overlayImage","TYPE":"image"}],
        "PASSES":[{"TARGET":"historyBuffer","PERSISTENT":true},{}]
    }*/
    void main() {
        if (PASSINDEX == 0) {
            gl_FragColor = vec4(IMG_THIS_NORM_PIXEL(historyBuffer).r + IMG_THIS_NORM_PIXEL(overlayImage).r, 0.0, 0.0, 1.0);
        } else { gl_FragColor = IMG_THIS_NORM_PIXEL(historyBuffer); }
    }"#;
    let shader = prepare_isf_shader(source).unwrap();
    let mut runtime = t32_gpu();
    let input = t32_frame(1, 1, [0, 0, 0, 255]);
    for (red, expected) in [(1, 1), (2, 3)] {
        let overlay = t32_frame(1, 1, [red, 0, 0, 255]);
        let images = [IsfImageInput {
            name: "overlayImage",
            frame: &overlay,
        }];
        let output = runtime
            .apply_stack_with_inputs(
                &input,
                &[IsfStageInputs {
                    shader: &shader,
                    controls: &[],
                    images: &images,
                }],
                0.0,
                0.016,
                FIXED_DATE,
            )
            .unwrap();
        t32_pixels(&output, [expected, 0, 0, 255]);
    }
    t32_pixels(
        &runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap(),
        [3, 0, 0, 255],
    );
}

#[test]
fn isf_authored_binding_identity_resets_shader_resources_and_explicit_none_is_transparent() {
    let binding = |layer_id| protocol::VideoIsfImageBinding {
        name: "overlayImage".into(),
        source: protocol::VideoIsfImageSource::Layer { layer_id },
    };
    let first = prepare_isf_shader_with_image_bindings(TWO_IMAGES, None, &[binding(1)]).unwrap();
    let second = prepare_isf_shader_with_image_bindings(TWO_IMAGES, None, &[binding(2)]).unwrap();
    assert_ne!(first.fingerprint, second.fingerprint);
    let shader = prepare_isf_shader_with_image_bindings(
        TWO_IMAGES,
        None,
        &[protocol::VideoIsfImageBinding {
            name: "inputImage".into(),
            source: protocol::VideoIsfImageSource::None {},
        }],
    )
    .unwrap();
    t32_pixels(
        &t32_gpu()
            .apply(
                &t32_frame(1, 1, [255, 255, 255, 255]),
                &shader,
                &[],
                0.0,
                0.016,
            )
            .unwrap(),
        [0, 0, 0, 0],
    );
}

#[test]
fn isf_bound_still_uses_t23_decode_and_rejects_missing_or_escaping_paths() {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    struct Pack(std::path::PathBuf);
    impl Drop for Pack {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pack =
        Pack(std::env::temp_dir().join(format!("syndocal-t32-isf-{}-{nonce}", std::process::id())));
    fs::create_dir(&pack.0).unwrap();
    let source_path = pack.0.join("two_images.fs");
    fs::write(&source_path, TWO_IMAGES).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save(pack.0.join("still.png"))
        .unwrap();
    let binding = |path: &str| protocol::VideoIsfImageBinding {
        name: "overlayImage".into(),
        source: protocol::VideoIsfImageSource::Imported { path: path.into() },
    };
    let shader = prepare_isf_shader_with_image_bindings(
        TWO_IMAGES,
        Some(&source_path),
        &[binding("still.png")],
    )
    .unwrap();
    assert_eq!(shader.bound_images.len(), 1);
    assert!(shader.imported_images.is_empty());
    let output = t32_gpu()
        .apply(&t32_frame(2, 1, [0, 0, 255, 255]), &shader, &[], 0.0, 0.016)
        .unwrap();
    t32_pixels(&output, [128, 0, 128, 255]);
    assert!(prepare_isf_shader_with_image_bindings(
        TWO_IMAGES,
        Some(&source_path),
        &[binding("missing.png")]
    )
    .is_err());
    assert!(prepare_isf_shader_with_image_bindings(
        TWO_IMAGES,
        Some(&source_path),
        &[binding("../still.png")]
    )
    .is_err());
    assert!(
        prepare_isf_shader_with_image_bindings(TWO_IMAGES, None, &[binding("still.png")]).is_err()
    );
}

#[test]
fn isf_secondary_bgra_conversion_matches_primary_frame_conversion() {
    let shader = prepare_isf_shader(TWO_IMAGES).unwrap();
    let input = t32_frame(1, 1, [0, 0, 255, 255]);
    let mut overlay = t32_frame(1, 1, [0, 0, 255, 255]);
    overlay.format = VideoPixelFormat::Bgra8;
    let images = [IsfImageInput {
        name: "overlayImage",
        frame: &overlay,
    }];
    let output = t32_gpu()
        .apply_stack_with_inputs(
            &input,
            &[IsfStageInputs {
                shader: &shader,
                controls: &[],
                images: &images,
            }],
            0.0,
            0.016,
            FIXED_DATE,
        )
        .unwrap();
    t32_pixels(&output, [128, 0, 128, 255]);
}

#[test]
fn isf_explicit_layer_binding_requires_current_named_frame() {
    let shader = prepare_isf_shader_with_image_bindings(
        TWO_IMAGES,
        None,
        &[protocol::VideoIsfImageBinding {
            name: "overlayImage".into(),
            source: protocol::VideoIsfImageSource::Layer { layer_id: 42 },
        }],
    )
    .unwrap();
    let input = t32_frame(1, 1, [0, 0, 255, 255]);
    let mut runtime = t32_gpu();
    let error = runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap_err();
    assert!(
        error.message.contains("layer 42 is unavailable"),
        "{}",
        error.message
    );
    let images = [IsfImageInput {
        name: "overlayImage",
        frame: &input,
    }];
    let output = runtime
        .apply_stack_with_inputs(
            &input,
            &[IsfStageInputs {
                shader: &shader,
                controls: &[],
                images: &images,
            }],
            0.0,
            0.016,
            FIXED_DATE,
        )
        .unwrap();
    t32_pixels(&output, [0, 0, 255, 255]);
}
