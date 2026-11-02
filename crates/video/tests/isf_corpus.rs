//! Authored compatibility patterns model common community ISF techniques; they
//! are original fixtures, not a claim that third-party shader packs were tested.
//! Canonical pixel outputs are the reference for the resident executor tests.
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use video::{
    prepare_isf_shader_with_image_bindings, prepare_isf_shader_with_path, IsfGpuRuntime,
    IsfStageInputs, VideoFrame, VideoPixelFormat,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
struct FixtureDirectory(PathBuf);
impl FixtureDirectory {
    fn new() -> Self {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/isf-corpus")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&directory).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([64, 128, 192, 255]))
            .save(directory.join("noise.png"))
            .unwrap();
        Self(directory)
    }
    fn shader(&self) -> PathBuf {
        self.0.join("pattern.fs")
    }
}
impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

struct Pattern {
    name: &'static str,
    source: String,
    expected: [u8; 4],
    input: [u8; 4],
    image_binding: bool,
}
fn pattern(
    name: &'static str,
    metadata: serde_json::Value,
    body: &str,
    expected: [u8; 4],
) -> Pattern {
    Pattern {
        name,
        source: format!("/*{metadata}*/\nvoid main() {{ {body} }}"),
        expected,
        input: [32, 64, 96, 255],
        image_binding: false,
    }
}
fn supported_patterns() -> Vec<Pattern> {
    let primary = json!([{"NAME":"inputImage","TYPE":"image"}]);
    let mut chroma = pattern(
        "chroma-key-second-image",
        json!({"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"background","TYPE":"image"}]}),
        "vec4 c = IMG_THIS_PIXEL(inputImage); gl_FragColor = c.g > 0.9 ? IMG_THIS_PIXEL(background) : c;",
        [64, 128, 192, 255],
    );
    chroma.input = [0, 255, 0, 255];
    chroma.image_binding = true;
    vec![
        pattern("single-pass-generator", json!({}), "gl_FragColor = vec4(0.25, 0.5, 0.75, 1.0);", [64, 128, 191, 255]),
        pattern("image-identity", json!({"INPUTS":primary}), "gl_FragColor = IMG_THIS_PIXEL(inputImage);", [32, 64, 96, 255]),
        pattern("two-pass-gaussian-blur", json!({"INPUTS":primary,"PASSES":[{"TARGET":"horizontal"},{}]}),
            "vec2 d = vec2(1.0) / RENDERSIZE; if (PASSINDEX == 0) { gl_FragColor = (IMG_NORM_PIXEL(inputImage, isf_FragNormCoord - vec2(d.x,0.0)) + 2.0 * IMG_THIS_PIXEL(inputImage) + IMG_NORM_PIXEL(inputImage, isf_FragNormCoord + vec2(d.x,0.0))) * 0.25; } else { gl_FragColor = (IMG_NORM_PIXEL(horizontal, isf_FragNormCoord - vec2(0.0,d.y)) + 2.0 * IMG_THIS_PIXEL(horizontal) + IMG_NORM_PIXEL(horizontal, isf_FragNormCoord + vec2(0.0,d.y))) * 0.25; }", [32, 64, 96, 255]),
        pattern("persistent-feedback-trail", json!({"PASSES":[{"TARGET":"history","PERSISTENT":true},{}]}),
            "if (PASSINDEX == 0) { gl_FragColor = vec4(IMG_THIS_PIXEL(history).r + 1.0 / 255.0, 0.0, 0.0, 1.0); } else { gl_FragColor = IMG_THIS_PIXEL(history); }", [1, 0, 0, 255]),
        pattern("audio-fft-spectrum", json!({"INPUTS":[{"NAME":"spectrum","TYPE":"audioFFT","MAX":8}]}),
            "gl_FragColor = vec4(IMG_NORM_PIXEL(spectrum,vec2(0.5)).rrr, 1.0);", [191, 191, 191, 255]),
        pattern("audio-waveform", json!({"INPUTS":[{"NAME":"wave","TYPE":"audio","MAX":8}]}),
            "gl_FragColor = vec4(IMG_NORM_PIXEL(wave,vec2(0.5)).rrr, 1.0);", [64, 64, 64, 255]),
        pattern("imported-noise-texture", json!({"IMPORTED":{"noise":{"PATH":"noise.png"}}}),
            "gl_FragColor = IMG_NORM_PIXEL(noise,isf_FragNormCoord);", [64, 128, 192, 255]),
        chroma,
        pattern("point-control", json!({"INPUTS":[{"NAME":"center","TYPE":"point2D","DEFAULT":[0.25,0.5]}]}),
            "gl_FragColor = vec4(center,0.0,1.0);", [64, 128, 0, 255]),
        pattern("color-control", json!({"INPUTS":[{"NAME":"tint","TYPE":"color","DEFAULT":[0.25,0.5,0.75,1.0]}]}),
            "gl_FragColor = tint;", [64, 128, 191, 255]),
        pattern("boolean-switch", json!({"INPUTS":[{"NAME":"enabled","TYPE":"bool","DEFAULT":true}]}),
            "gl_FragColor = enabled ? vec4(1.0,0.0,0.0,1.0) : vec4(0.0);", [255, 0, 0, 255]),
        pattern("long-menu", json!({"INPUTS":[{"NAME":"mode","TYPE":"long","DEFAULT":2,"VALUES":[0,1,2],"LABELS":["A","B","C"]}]}),
            "gl_FragColor = vec4(float(mode) / 4.0,0.0,0.0,1.0);", [128, 0, 0, 255]),
        pattern("date-and-time", json!({}),
            "gl_FragColor = vec4(DATE.y / 16.0, DATE.z / 8.0, TIME / 4.0, 1.0);", [175, 64, 128, 255]),
        pattern("four-pass-scaled-pyramid", json!({"INPUTS":primary,"PASSES":[{"TARGET":"quarter","WIDTH":"$WIDTH / 2","HEIGHT":"$HEIGHT / 2"},{"TARGET":"filtered"},{"TARGET":"combined"},{}]}),
            "if (PASSINDEX == 0) { gl_FragColor = IMG_NORM_PIXEL(inputImage,isf_FragNormCoord); } else if (PASSINDEX == 1) { gl_FragColor = IMG_NORM_PIXEL(quarter,isf_FragNormCoord); } else if (PASSINDEX == 2) { gl_FragColor = IMG_THIS_PIXEL(filtered); } else { gl_FragColor = IMG_THIS_PIXEL(combined); }", [32, 64, 96, 255]),
    ]
}

#[derive(Deserialize)]
struct RejectedPattern {
    name: String,
    source: String,
    raw: String,
}
fn rejected_patterns() -> Vec<RejectedPattern> {
    serde_json::from_str(include_str!("fixtures/isf-corpus-rejections.json")).unwrap()
}

#[test]
fn isf_corpus_twenty_patterns_have_known_outputs_or_exact_rejections() {
    let directory = FixtureDirectory::new();
    let patterns = supported_patterns();
    let rejections = rejected_patterns();
    assert_eq!((patterns.len(), rejections.len()), (14, 6));
    let mut runtime = IsfGpuRuntime::new().expect("ISF corpus requires a GPU adapter");
    runtime.set_audio_frame(Some(&protocol::LiveAudioTextureFrame {
        generation: 1,
        feature_sequence: 1,
        sample_rate: 48_000,
        waveform: [-0.5; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [0.75; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
    }));
    for pattern in patterns {
        let bindings = if pattern.image_binding {
            vec![protocol::VideoIsfImageBinding {
                name: "background".into(),
                source: protocol::VideoIsfImageSource::Imported {
                    path: "noise.png".into(),
                },
            }]
        } else {
            Vec::new()
        };
        let shader = prepare_isf_shader_with_image_bindings(
            &pattern.source,
            Some(&directory.shader()),
            &bindings,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", pattern.name));
        let input = VideoFrame {
            layer_id: 17,
            width: 4,
            height: 4,
            pts_ms: 2_000,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: pattern.input.repeat(16),
        };
        // The runtime takes resolved control values; an authored stage starts
        // at each input's declared DEFAULT.
        let defaults = shader
            .controls
            .iter()
            .map(|control| control.default)
            .collect::<Vec<_>>();
        let render = |runtime: &mut IsfGpuRuntime| {
            runtime
                .apply_stack_with_inputs(
                    &input,
                    &[IsfStageInputs {
                        shader: &shader,
                        controls: &defaults,
                        images: &[],
                    }],
                    2.0,
                    0.016,
                    [2026.0, 11.0, 2.0, 100.0],
                )
                .unwrap_or_else(|error| panic!("{}: {error}", pattern.name))
        };
        let output = render(&mut runtime);
        assert_eq!((output.width, output.height), (4, 4), "{}", pattern.name);
        for pixel in output.data.chunks_exact(4) {
            assert_eq!(pixel, pattern.expected, "{}", pattern.name);
        }
        if pattern.name == "persistent-feedback-trail" {
            let second = render(&mut runtime);
            for pixel in second.data.chunks_exact(4) {
                assert_eq!(pixel, [2, 0, 0, 255], "feedback persists across frames");
            }
        }
    }
    for pattern in rejections {
        let error =
            prepare_isf_shader_with_path(&pattern.source, Some(&directory.shader())).unwrap_err();
        assert_eq!(error.message, pattern.raw, "{}", pattern.name);
    }
}

#[test]
fn isf_corpus_companion_vertex_and_resource_ceilings_fail_closed() {
    let directory = FixtureDirectory::new();
    let source = "/*{}*/ void main() { gl_FragColor = vec4(1.0); }";
    for extension in ["vs", "vert"] {
        let companion = directory.shader().with_extension(extension);
        fs::write(&companion, "void main() { gl_Position = vec4(0.0); }").unwrap();
        let error = prepare_isf_shader_with_path(source, Some(&directory.shader())).unwrap_err();
        assert!(error
            .message
            .starts_with("ISF vertex shaders are not supported; companion '"));
        fs::remove_file(companion).unwrap();
    }
    for (kind, count, expected) in [
        (
            "audioFFT",
            17,
            "ISF declares too many textures; the safety limit is 16",
        ),
        (
            "image",
            5,
            "ISF declares too many image inputs; the safety limit is 4",
        ),
        (
            "float",
            17,
            "ISF filter declares 17 controls; the safety limit is 16",
        ),
    ] {
        let inputs: Vec<_> = (0..count)
            .map(|index| json!({"NAME":format!("value{index}"),"TYPE":kind}))
            .collect();
        let source = pattern(
            "over-ceiling",
            json!({"INPUTS":inputs}),
            "gl_FragColor = vec4(1.0);",
            [255; 4],
        )
        .source;
        assert_eq!(
            prepare_isf_shader_with_path(&source, Some(&directory.shader()))
                .unwrap_err()
                .message,
            expected
        );
    }
}
