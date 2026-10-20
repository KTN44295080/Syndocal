use super::{sanitize_video_isf_effect, video_isf_stage_from_root};
use protocol::{VideoIsfImageBinding, VideoIsfImageSource};

fn bound_effect() -> protocol::VideoIsfEffectSummary {
    let source = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"other","TYPE":"image"}]}*/
void main() { gl_FragColor = IMG_THIS_NORM_PIXEL(other); }"#;
    let prepared = video::prepare_isf_shader(source).unwrap();
    let mut effect = video::isf_effect_from_prepared("Image input".into(), source, None, &prepared);
    effect.image_bindings = vec![VideoIsfImageBinding {
        name: "other".into(),
        source: VideoIsfImageSource::Layer { layer_id: 42 },
    }];
    effect
}

#[test]
fn isf_image_bindings_survive_native_sanitization_and_stack_projection() {
    let mut effect = bound_effect();
    effect.stack.push(video_isf_stage_from_root(effect.clone()));
    let canonical = sanitize_video_isf_effect(effect.clone()).unwrap();
    assert_eq!(canonical.image_bindings, effect.image_bindings);
    assert_eq!(
        canonical.stack[0].image_bindings,
        effect.stack[0].image_bindings
    );
    let json = serde_json::to_vec(&canonical).unwrap();
    let roundtrip: protocol::VideoIsfEffectSummary = serde_json::from_slice(&json).unwrap();
    assert_eq!(roundtrip, canonical);
}

#[test]
fn isf_image_bindings_reject_undeclared_names_at_native_admission() {
    let mut effect = bound_effect();
    effect.image_bindings[0].name = "missing".into();
    assert!(sanitize_video_isf_effect(effect).is_err());
}
