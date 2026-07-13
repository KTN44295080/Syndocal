use protocol::VideoIsfEffectSummary;

use crate::{isf_effect_from_prepared, prepare_isf_shader, IsfPrepareError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinIsfPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

pub const BUILTIN_ISF_PRESETS: &[BuiltinIsfPreset] = &[
    BuiltinIsfPreset {
        id: "invert",
        label: "Invert",
        source: r#"/*{
  "DESCRIPTION":"Invert RGB while preserving alpha.",
  "CATEGORIES":["Color","Syndocal Built-in"],
  "INPUTS":[{"NAME":"inputImage","TYPE":"image"}]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  gl_FragColor = vec4(vec3(1.0) - src.rgb, src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "monochrome",
        label: "Monochrome",
        source: r#"/*{
  "DESCRIPTION":"Mix the source toward luminance.",
  "CATEGORIES":["Color","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"amount","TYPE":"float","DEFAULT":1.0,"MIN":0.0,"MAX":1.0}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float luma = dot(src.rgb, vec3(0.2126, 0.7152, 0.0722));
  gl_FragColor = vec4(mix(src.rgb, vec3(luma), amount), src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "threshold",
        label: "Threshold",
        source: r#"/*{
  "DESCRIPTION":"Luminance threshold with adjustable softness.",
  "CATEGORIES":["Color","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"level","TYPE":"float","DEFAULT":0.5,"MIN":0.0,"MAX":1.0},
    {"NAME":"softness","TYPE":"float","DEFAULT":0.02,"MIN":0.0,"MAX":0.5}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float luma = dot(src.rgb, vec3(0.2126, 0.7152, 0.0722));
  float mask = smoothstep(level - softness, level + softness, luma);
  gl_FragColor = vec4(vec3(mask), src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "posterize",
        label: "Posterize",
        source: r#"/*{
  "DESCRIPTION":"Reduce the number of RGB levels.",
  "CATEGORIES":["Color","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"levels","TYPE":"float","DEFAULT":6.0,"MIN":2.0,"MAX":32.0}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float count = max(2.0, floor(levels));
  vec3 rgb = floor(src.rgb * count) / (count - 1.0);
  gl_FragColor = vec4(clamp(rgb, 0.0, 1.0), src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "rgb-split",
        label: "RGB Split",
        source: r#"/*{
  "DESCRIPTION":"Offset red and blue channels horizontally.",
  "CATEGORIES":["Glitch","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"amount","TYPE":"float","DEFAULT":0.015,"MIN":0.0,"MAX":0.2}
  ]
}*/
void main() {
  vec2 uv = isf_FragNormCoord;
  vec4 center = IMG_NORM_PIXEL(inputImage, uv);
  float red = IMG_NORM_PIXEL(inputImage, uv + vec2(amount, 0.0)).r;
  float blue = IMG_NORM_PIXEL(inputImage, uv - vec2(amount, 0.0)).b;
  gl_FragColor = vec4(red, center.g, blue, center.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "mirror",
        label: "Mirror",
        source: r#"/*{
  "DESCRIPTION":"Mirror horizontally, vertically, or both.",
  "CATEGORIES":["Geometry","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"mode","TYPE":"long","DEFAULT":0,"MIN":0,"MAX":2,"LABELS":["Horizontal","Vertical","Both"],"VALUES":[0,1,2]}
  ]
}*/
void main() {
  vec2 uv = isf_FragNormCoord;
  if (mode == 0 || mode == 2) uv.x = abs(uv.x * 2.0 - 1.0);
  if (mode == 1 || mode == 2) uv.y = abs(uv.y * 2.0 - 1.0);
  gl_FragColor = IMG_NORM_PIXEL(inputImage, uv);
}
"#,
    },
    BuiltinIsfPreset {
        id: "kaleidoscope",
        label: "Kaleidoscope",
        source: r#"/*{
  "DESCRIPTION":"Polar kaleidoscope with segment and rotation controls.",
  "CATEGORIES":["Geometry","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"segments","TYPE":"float","DEFAULT":8.0,"MIN":2.0,"MAX":24.0},
    {"NAME":"rotation","TYPE":"float","DEFAULT":0.0,"MIN":-3.14159,"MAX":3.14159}
  ]
}*/
void main() {
  vec2 point = isf_FragNormCoord - vec2(0.5);
  float radius = length(point);
  float slice = 6.28318530718 / max(2.0, floor(segments));
  float angle = atan(point.y, point.x) + rotation;
  angle = abs(mod(angle + slice * 0.5, slice) - slice * 0.5);
  vec2 uv = vec2(cos(angle), sin(angle)) * radius + vec2(0.5);
  gl_FragColor = IMG_NORM_PIXEL(inputImage, uv);
}
"#,
    },
    BuiltinIsfPreset {
        id: "zoom",
        label: "Zoom",
        source: r#"/*{
  "DESCRIPTION":"Zoom around an adjustable center point.",
  "CATEGORIES":["Geometry","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"zoom","TYPE":"float","DEFAULT":1.5,"MIN":0.25,"MAX":8.0},
    {"NAME":"center","TYPE":"point2D","DEFAULT":[0.5,0.5],"MIN":[0.0,0.0],"MAX":[1.0,1.0]}
  ]
}*/
void main() {
  vec2 uv = (isf_FragNormCoord - center) / max(0.001, zoom) + center;
  gl_FragColor = IMG_NORM_PIXEL(inputImage, uv);
}
"#,
    },
    BuiltinIsfPreset {
        id: "rotate",
        label: "Rotate",
        source: r#"/*{
  "DESCRIPTION":"Rotate the source around an adjustable center.",
  "CATEGORIES":["Geometry","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"angle","TYPE":"float","DEFAULT":0.5,"MIN":-3.14159,"MAX":3.14159},
    {"NAME":"center","TYPE":"point2D","DEFAULT":[0.5,0.5],"MIN":[0.0,0.0],"MAX":[1.0,1.0]}
  ]
}*/
void main() {
  vec2 point = isf_FragNormCoord - center;
  float sine = sin(angle);
  float cosine = cos(angle);
  vec2 uv = vec2(cosine * point.x - sine * point.y, sine * point.x + cosine * point.y) + center;
  gl_FragColor = IMG_NORM_PIXEL(inputImage, uv);
}
"#,
    },
    BuiltinIsfPreset {
        id: "strobe",
        label: "Strobe",
        source: r#"/*{
  "DESCRIPTION":"Time-based black-frame strobe.",
  "CATEGORIES":["Temporal","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"rate","TYPE":"float","DEFAULT":8.0,"MIN":0.5,"MAX":30.0},
    {"NAME":"duty","TYPE":"float","DEFAULT":0.5,"MIN":0.05,"MAX":0.95}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float active = step(fract(TIME * rate), duty);
  gl_FragColor = vec4(src.rgb * active, src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "scanlines",
        label: "Scanlines",
        source: r#"/*{
  "DESCRIPTION":"Dark horizontal scanlines.",
  "CATEGORIES":["Stylize","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"density","TYPE":"float","DEFAULT":180.0,"MIN":8.0,"MAX":720.0},
    {"NAME":"amount","TYPE":"float","DEFAULT":0.35,"MIN":0.0,"MAX":1.0}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float line = 0.5 + 0.5 * sin(isf_FragNormCoord.y * density * 6.28318530718);
  gl_FragColor = vec4(src.rgb * mix(1.0 - amount, 1.0, line), src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "colorize",
        label: "Colorize",
        source: r#"/*{
  "DESCRIPTION":"Tint luminance with a selected color.",
  "CATEGORIES":["Color","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"tint","TYPE":"color","DEFAULT":[0.1,0.7,1.0,1.0],"MIN":[0.0,0.0,0.0,0.0],"MAX":[1.0,1.0,1.0,1.0]},
    {"NAME":"amount","TYPE":"float","DEFAULT":0.75,"MIN":0.0,"MAX":1.0}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float luma = dot(src.rgb, vec3(0.2126, 0.7152, 0.0722));
  vec3 colored = luma * tint.rgb;
  gl_FragColor = vec4(mix(src.rgb, colored, amount), src.a * mix(1.0, tint.a, amount));
}
"#,
    },
    BuiltinIsfPreset {
        id: "vignette",
        label: "Vignette",
        source: r#"/*{
  "DESCRIPTION":"Darken the edges around the frame center.",
  "CATEGORIES":["Stylize","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"amount","TYPE":"float","DEFAULT":0.7,"MIN":0.0,"MAX":1.0},
    {"NAME":"softness","TYPE":"float","DEFAULT":0.35,"MIN":0.01,"MAX":1.0}
  ]
}*/
void main() {
  vec4 src = IMG_THIS_PIXEL(inputImage);
  float distanceFromCenter = length(isf_FragNormCoord - vec2(0.5));
  float mask = 1.0 - smoothstep(0.5 - softness * 0.5, 0.5 + softness * 0.5, distanceFromCenter);
  gl_FragColor = vec4(src.rgb * mix(1.0, mask, amount), src.a);
}
"#,
    },
    BuiltinIsfPreset {
        id: "glitch-shift",
        label: "Glitch Shift",
        source: r#"/*{
  "DESCRIPTION":"Animated horizontal slice displacement.",
  "CATEGORIES":["Glitch","Syndocal Built-in"],
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"amount","TYPE":"float","DEFAULT":0.08,"MIN":0.0,"MAX":0.5},
    {"NAME":"bands","TYPE":"float","DEFAULT":18.0,"MIN":2.0,"MAX":96.0},
    {"NAME":"speed","TYPE":"float","DEFAULT":8.0,"MIN":0.0,"MAX":30.0}
  ]
}*/
void main() {
  vec2 uv = isf_FragNormCoord;
  float band = floor(uv.y * max(2.0, floor(bands)));
  float frame = floor(TIME * speed);
  float noise = fract(sin(band * 12.9898 + frame * 78.233) * 43758.5453);
  uv.x += (noise - 0.5) * amount;
  gl_FragColor = IMG_NORM_PIXEL(inputImage, uv);
}
"#,
    },
];

pub fn builtin_isf_effect(id: &str) -> Result<Option<VideoIsfEffectSummary>, IsfPrepareError> {
    let Some(preset) = BUILTIN_ISF_PRESETS.iter().find(|preset| preset.id == id) else {
        return Ok(None);
    };
    let prepared = prepare_isf_shader(preset.source)?;
    Ok(Some(isf_effect_from_prepared(
        preset.label.to_string(),
        preset.source.to_string(),
        None,
        &prepared,
    )))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn all_builtin_isf_presets_are_unique_and_compile() {
        let mut ids = HashSet::new();
        let mut labels = HashSet::new();
        assert!(BUILTIN_ISF_PRESETS.len() >= 14);
        for preset in BUILTIN_ISF_PRESETS {
            assert!(ids.insert(preset.id), "duplicate preset id {}", preset.id);
            assert!(
                labels.insert(preset.label),
                "duplicate preset label {}",
                preset.label
            );
            let prepared = prepare_isf_shader(preset.source)
                .unwrap_or_else(|error| panic!("{} did not compile: {error}", preset.id));
            assert_eq!(prepared.image_input_name, "inputImage");
            assert!(prepared
                .categories
                .iter()
                .any(|category| category == "Syndocal Built-in"));
        }
    }

    #[test]
    fn builtin_isf_effect_uses_canonical_controls_and_rejects_unknown_id() {
        let effect = builtin_isf_effect("rgb-split").unwrap().unwrap();
        assert_eq!(effect.label, "RGB Split");
        assert_eq!(effect.controls.len(), 1);
        assert_eq!(effect.controls[0].name, "amount");
        assert!(builtin_isf_effect("missing").unwrap().is_none());
    }
}
