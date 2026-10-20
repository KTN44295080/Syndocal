use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    path::Path,
    sync::Arc,
};

use naga::{
    back::wgsl::WriterFlags,
    front::glsl::{Frontend, Options},
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{convert_frame_to_rgba8, CpuCompositeError, VideoFrame, VideoPixelFormat};
use protocol::{VideoIsfControlKind, VideoIsfControlSummary, VideoIsfEffectSummary};

pub const ISF_MAX_SOURCE_BYTES: usize = 512 * 1024;
pub const ISF_MAX_CONTROL_INPUTS: usize = 16;
const ISF_GPU_PIPELINE_CAPACITY: usize = 64;
const ISF_MAX_PASSES: usize = 16;
const ISF_MAX_TEXTURES: usize = 16;

#[path = "isf_pass_runtime.rs"]
mod pass_runtime;
#[path = "isf_resources.rs"]
mod resources;

use resources::{IsfDimension, IsfImportedImage};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IsfPassDefinition {
    target: Option<String>,
    persistent: bool,
    float: bool,
    width: Option<IsfDimension>,
    height: Option<IsfDimension>,
}

/// Runtime-only ownership: the same source in two stages, inputs or render
/// surfaces must never share feedback. Authored IDs survive stage reordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IsfInstanceKey {
    pub namespace: (u64, Option<u64>, bool),
    pub scope: Option<protocol::VideoEffectScope>,
    pub input: Option<protocol::VideoRenderInputKey>,
    pub chain_id: Option<protocol::VideoEffectChainId>,
    pub stage_id: Option<protocol::VideoEffectStageId>,
    pub effect_id: Option<protocol::VideoEffectId>,
    pub legacy_stage_index: usize,
    pub layer_id: u64,
}

impl IsfInstanceKey {
    fn legacy(layer_id: u64, index: usize) -> Self {
        Self {
            namespace: (0, None, false),
            scope: None,
            input: None,
            chain_id: None,
            stage_id: None,
            effect_id: None,
            legacy_stage_index: index,
            layer_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsfControlKind {
    Event,
    Bool,
    Long,
    Float,
    Point2d,
    Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsfControlDefinition {
    pub name: String,
    pub kind: IsfControlKind,
    pub default: [f32; 4],
    pub minimum: [f32; 4],
    pub maximum: [f32; 4],
    pub labels: Vec<String>,
    pub values: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedIsfShader {
    pub description: Option<String>,
    pub categories: Vec<String>,
    pub image_input_name: String,
    pub controls: Vec<IsfControlDefinition>,
    pub wgsl: String,
    pub(crate) passes: Vec<IsfPassDefinition>,
    pub(crate) imported_images: Vec<IsfImportedImage>,
    pub(crate) fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsfPrepareError {
    pub message: String,
}

impl fmt::Display for IsfPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IsfPrepareError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsfRuntimeError {
    pub message: String,
}

impl fmt::Display for IsfRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IsfRuntimeError {}

pub struct IsfGpuRuntime {
    device: wgpu::Device,
    queue: wgpu::Queue,
    vertex_shader: wgpu::ShaderModule,
    pipelines: HashMap<(u64, bool, usize), IsfGpuPipeline>,
    resources: pass_runtime::IsfResourcePool,
    frame_index: u32,
    last_stack_stage_count: usize,
    last_stack_render_us: u64,
}

struct IsfGpuPipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

impl IsfGpuRuntime {
    pub fn new() -> Result<Self, IsfRuntimeError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .map_err(|error| runtime_error(format!("ISF GPU adapter unavailable: {error}")))?;
        let float_features = adapter.features() & wgpu::Features::FLOAT32_FILTERABLE;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal ISF runtime"),
            required_features: float_features,
            ..Default::default()
        }))
        .map_err(|error| runtime_error(format!("ISF GPU device unavailable: {error}")))?;
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Syndocal ISF full-screen vertex shader"),
            source: wgpu::ShaderSource::Wgsl(ISF_VERTEX_SHADER.into()),
        });
        Ok(Self {
            device,
            queue,
            vertex_shader,
            pipelines: HashMap::new(),
            resources: pass_runtime::IsfResourcePool::default(),
            frame_index: 0,
            last_stack_stage_count: 0,
            last_stack_render_us: 0,
        })
    }

    pub fn apply(
        &mut self,
        frame: &VideoFrame,
        shader: &PreparedIsfShader,
        controls: &[[f32; 4]],
        time_seconds: f32,
        time_delta_seconds: f32,
    ) -> Result<VideoFrame, IsfRuntimeError> {
        self.apply_stack(
            frame,
            &[(shader, controls)],
            time_seconds,
            time_delta_seconds,
        )
    }

    /// Applies an ordered set of ISF filters with one
    /// host upload, GPU-local ping-pong textures, and one final readback.
    pub fn apply_stack(
        &mut self,
        frame: &VideoFrame,
        stages: &[(&PreparedIsfShader, &[[f32; 4]])],
        time_seconds: f32,
        time_delta_seconds: f32,
    ) -> Result<VideoFrame, IsfRuntimeError> {
        let frame_index = self.frame_index;
        let keys = (0..stages.len())
            .map(|index| IsfInstanceKey::legacy(frame.layer_id, index))
            .collect::<Vec<_>>();
        let rendered = self.apply_stack_at_frame_index(
            frame,
            stages,
            time_seconds,
            time_delta_seconds,
            frame_index,
            &keys,
        );
        // Preserve the legacy API contract: only a successful, non-empty
        // stack advances the host frame counter.
        if rendered.is_ok() && !stages.is_empty() {
            self.frame_index = self.frame_index.wrapping_add(1);
        }
        rendered
    }

    fn apply_stack_at_frame_index(
        &mut self,
        frame: &VideoFrame,
        stages: &[(&PreparedIsfShader, &[[f32; 4]])],
        time_seconds: f32,
        time_delta_seconds: f32,
        frame_index: u32,
        keys: &[IsfInstanceKey],
    ) -> Result<VideoFrame, IsfRuntimeError> {
        self.render_pass_stack(
            frame,
            stages,
            time_seconds,
            time_delta_seconds,
            frame_index,
            keys,
        )
    }

    /// Applies every stage independently so a runtime failure bypasses only
    /// that stage and later stages still receive the last valid frame.
    pub fn apply_stack_stage_local(
        &mut self,
        frame: &VideoFrame,
        stages: &[(&PreparedIsfShader, &[[f32; 4]])],
        time_seconds: f32,
        time_delta_seconds: f32,
    ) -> (VideoFrame, Vec<(usize, IsfRuntimeError)>) {
        let keys = (0..stages.len())
            .map(|index| IsfInstanceKey::legacy(frame.layer_id, index))
            .collect::<Vec<_>>();
        self.apply_stack_stage_local_with_keys(
            frame,
            stages,
            time_seconds,
            time_delta_seconds,
            &keys,
        )
    }

    pub(crate) fn apply_stack_stage_local_with_keys(
        &mut self,
        frame: &VideoFrame,
        stages: &[(&PreparedIsfShader, &[[f32; 4]])],
        time_seconds: f32,
        time_delta_seconds: f32,
        keys: &[IsfInstanceKey],
    ) -> (VideoFrame, Vec<(usize, IsfRuntimeError)>) {
        let started = std::time::Instant::now();
        let frame_index = self.frame_index;
        let mut current = frame.clone();
        let mut failures = Vec::new();
        for (index, stage) in stages.iter().enumerate() {
            let Some(key) = keys.get(index) else {
                failures.push((index, runtime_error("ISF stage ownership key is missing")));
                continue;
            };
            match self.apply_stack_at_frame_index(
                &current,
                std::slice::from_ref(stage),
                time_seconds,
                time_delta_seconds,
                frame_index,
                std::slice::from_ref(key),
            ) {
                Ok(frame) => current = frame,
                Err(error) => failures.push((index, error)),
            }
        }
        // One stage-local chain invocation is one host frame. Advance once for
        // every non-empty chain, including all-fail calls, so failure topology
        // cannot change FRAMEINDEX. Empty chains are pure no-ops.
        if !stages.is_empty() {
            self.frame_index = self.frame_index.wrapping_add(1);
        }
        self.last_stack_stage_count = stages.len();
        self.last_stack_render_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        (current, failures)
    }

    /// Consumes one host frame for a non-empty stage-local chain whose stages
    /// all failed before GPU preparation. This keeps FRAMEINDEX independent of
    /// prepare-failure topology without changing the legacy `apply_stack`
    /// success-only advancement contract.
    pub(crate) fn advance_stage_local_chain_without_prepared_stages(&mut self) {
        self.frame_index = self.frame_index.wrapping_add(1);
        self.last_stack_stage_count = 0;
        self.last_stack_render_us = 0;
    }

    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    pub fn last_stack_stage_count(&self) -> usize {
        self.last_stack_stage_count
    }

    pub fn last_stack_render_us(&self) -> u64 {
        self.last_stack_render_us
    }

    pub fn reset_last_stack_metrics(&mut self) {
        self.last_stack_stage_count = 0;
        self.last_stack_render_us = 0;
    }

}

pub fn resolved_isf_control_values(
    shader: &PreparedIsfShader,
    effect: &VideoIsfEffectSummary,
) -> Vec<[f32; 4]> {
    resolved_isf_control_values_from_controls(shader, &effect.controls)
}

pub fn resolved_isf_control_values_from_controls(
    shader: &PreparedIsfShader,
    controls: &[VideoIsfControlSummary],
) -> Vec<[f32; 4]> {
    shader
        .controls
        .iter()
        .map(|definition| {
            let candidate = controls
                .iter()
                .find(|control| control.name == definition.name)
                .map(|control| control.value)
                .unwrap_or(definition.default);
            sanitize_control_value(
                definition.kind,
                candidate,
                definition.minimum,
                definition.maximum,
            )
        })
        .collect()
}

pub fn isf_effect_from_prepared(
    label: String,
    source: impl Into<Arc<str>>,
    source_path: Option<String>,
    shader: &PreparedIsfShader,
) -> VideoIsfEffectSummary {
    VideoIsfEffectSummary {
        enabled: true,
        label,
        source: source.into(),
        source_path,
        description: shader.description.clone(),
        categories: shader.categories.clone(),
        controls: shader
            .controls
            .iter()
            .map(|control| VideoIsfControlSummary {
                name: control.name.clone(),
                kind: protocol_control_kind(control.kind),
                value: control.default,
                default: control.default,
                minimum: control.minimum,
                maximum: control.maximum,
                labels: control.labels.clone(),
                values: control.values.clone(),
                tempo_sync: None,
            })
            .collect(),
        stack: Vec::new(),
    }
}

fn protocol_control_kind(kind: IsfControlKind) -> VideoIsfControlKind {
    match kind {
        IsfControlKind::Event => VideoIsfControlKind::Event,
        IsfControlKind::Bool => VideoIsfControlKind::Bool,
        IsfControlKind::Long => VideoIsfControlKind::Long,
        IsfControlKind::Float => VideoIsfControlKind::Float,
        IsfControlKind::Point2d => VideoIsfControlKind::Point2d,
        IsfControlKind::Color => VideoIsfControlKind::Color,
    }
}

fn sanitize_control_value(
    kind: IsfControlKind,
    mut value: [f32; 4],
    minimum: [f32; 4],
    maximum: [f32; 4],
) -> [f32; 4] {
    for index in 0..4 {
        let low = minimum[index].min(maximum[index]);
        let high = minimum[index].max(maximum[index]);
        value[index] = if value[index].is_finite() {
            value[index].clamp(low, high)
        } else {
            low
        };
    }
    match kind {
        IsfControlKind::Event | IsfControlKind::Bool => {
            value[0] = if value[0] >= 0.5 { 1.0 } else { 0.0 }
        }
        IsfControlKind::Long => value[0] = value[0].round(),
        IsfControlKind::Float | IsfControlKind::Point2d | IsfControlKind::Color => {}
    }
    value
}

const ISF_VERTEX_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
    );
    let uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = uvs[index];
    return output;
}
"#;

#[derive(Debug, Deserialize)]
struct IsfMetadata {
    #[serde(default, rename = "DESCRIPTION")]
    description: Option<String>,
    #[serde(default, rename = "CATEGORIES")]
    categories: Vec<String>,
    #[serde(default, rename = "INPUTS")]
    inputs: Vec<IsfInput>,
    #[serde(default, rename = "PASSES")]
    passes: Vec<IsfPass>,
    #[serde(default, rename = "IMPORTED")]
    imported: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct IsfInput {
    #[serde(rename = "NAME")]
    name: String,
    #[serde(rename = "TYPE")]
    kind: String,
    #[serde(default, rename = "DEFAULT")]
    default: Option<Value>,
    #[serde(default, rename = "MIN")]
    minimum: Option<Value>,
    #[serde(default, rename = "MAX")]
    maximum: Option<Value>,
    #[serde(default, rename = "LABELS")]
    labels: Vec<String>,
    #[serde(default, rename = "VALUES")]
    values: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct IsfPass {
    #[serde(default, rename = "TARGET")]
    target: Option<String>,
    #[serde(default, rename = "PERSISTENT")]
    persistent: Value,
    #[serde(default, rename = "FLOAT")]
    float: Value,
    #[serde(default, rename = "WIDTH")]
    width: Option<Value>,
    #[serde(default, rename = "HEIGHT")]
    height: Option<Value>,
}

pub fn prepare_isf_shader(source: &str) -> Result<PreparedIsfShader, IsfPrepareError> {
    prepare_isf_shader_with_path(source, None)
}

pub fn prepare_isf_shader_with_path(
    source: &str,
    source_path: Option<&Path>,
) -> Result<PreparedIsfShader, IsfPrepareError> {
    if source.len() > ISF_MAX_SOURCE_BYTES {
        return Err(error(format!(
            "ISF source exceeds the {} byte safety limit",
            ISF_MAX_SOURCE_BYTES
        )));
    }
    if let Some(path) = source_path {
        for extension in ["vs", "vert"] {
            let companion = path.with_extension(extension);
            if companion.is_file() {
                return Err(error(format!(
                    "ISF vertex shaders are not supported; companion '{}' requires a custom vertex stage",
                    companion.display()
                )));
            }
        }
    }
    let (metadata_json, body) = split_isf_source(source)?;
    let metadata: IsfMetadata = serde_json::from_str(metadata_json)
        .map_err(|parse_error| error(format!("ISF metadata JSON is invalid: {parse_error}")))?;
    let passes = prepare_passes(&metadata.passes)?;
    validate_shader_body(body)?;

    let mut image_inputs = Vec::new();
    let mut controls = Vec::new();
    let mut names = HashSet::new();
    for input in metadata.inputs {
        validate_identifier(&input.name)?;
        if !names.insert(input.name.clone()) {
            return Err(error(format!("ISF input '{}' is duplicated", input.name)));
        }
        match input.kind.as_str() {
            "image" => image_inputs.push(input.name),
            "event" | "bool" | "long" | "float" | "point2D" | "color" => {
                controls.push(control_definition(input)?)
            }
            "audio" | "audioFFT" => {
                return Err(error("ISF audio/audioFFT textures are not supported by this runtime; use Syndocal Audio FFT nodes"));
            }
            kind => return Err(error(format!("Unsupported ISF input type '{kind}'"))),
        }
    }
    if image_inputs.len() != 1 {
        return Err(error(format!(
            "ISF filter must declare exactly one image input; found {}",
            image_inputs.len()
        )));
    }
    if controls.len() > ISF_MAX_CONTROL_INPUTS {
        return Err(error(format!(
            "ISF filter declares {} controls; the safety limit is {}",
            controls.len(),
            ISF_MAX_CONTROL_INPUTS
        )));
    }
    let image_input_name = image_inputs.remove(0);
    let mut image_names = vec![image_input_name.clone()];
    for pass in &passes {
        if let Some(target) = &pass.target {
            validate_identifier(target)?;
            if !names.insert(target.clone()) {
                return Err(error(format!(
                    "ISF target '{target}' is duplicated or conflicts with an input"
                )));
            }
            image_names.push(target.clone());
        }
    }
    for name in metadata.imported.keys() {
        validate_identifier(name)?;
        if !names.insert(name.clone()) {
            return Err(error(format!(
                "ISF imported image '{name}' conflicts with an input or target"
            )));
        }
    }
    if image_names.len() + metadata.imported.len() > ISF_MAX_TEXTURES {
        return Err(error(format!(
            "ISF declares too many textures; the safety limit is {ISF_MAX_TEXTURES}"
        )));
    }
    let imported_images = resources::load_imported_images(&metadata.imported, source_path)?;
    image_names.extend(imported_images.iter().map(|image| image.name.clone()));
    let glsl = build_glsl_wrapper(body, &image_names, &controls);
    let wgsl = glsl_to_wgsl(&glsl)?;
    let mut fingerprint = DefaultHasher::new();
    source.hash(&mut fingerprint);
    for image in &imported_images {
        image.name.hash(&mut fingerprint);
        image.width.hash(&mut fingerprint);
        image.height.hash(&mut fingerprint);
        image.rgba.hash(&mut fingerprint);
    }
    Ok(PreparedIsfShader {
        description: metadata
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        categories: metadata
            .categories
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .take(16)
            .collect(),
        image_input_name,
        controls,
        wgsl,
        passes,
        imported_images,
        fingerprint: fingerprint.finish(),
    })
}

fn split_isf_source(source: &str) -> Result<(&str, &str), IsfPrepareError> {
    let comment_start = source
        .find("/*")
        .ok_or_else(|| error("ISF source is missing its top metadata comment"))?;
    if !source[..comment_start].trim().is_empty() {
        return Err(error(
            "ISF metadata comment must be the first source element",
        ));
    }
    let json_start = comment_start + 2;
    let comment_end = source[json_start..]
        .find("*/")
        .map(|offset| json_start + offset)
        .ok_or_else(|| error("ISF metadata comment is not terminated"))?;
    Ok((
        source[json_start..comment_end].trim(),
        &source[comment_end + 2..],
    ))
}

fn prepare_passes(passes: &[IsfPass]) -> Result<Vec<IsfPassDefinition>, IsfPrepareError> {
    if passes.len() > ISF_MAX_PASSES {
        return Err(error(format!(
            "ISF declares too many passes; the safety limit is {ISF_MAX_PASSES}"
        )));
    }
    if passes.is_empty() {
        return Ok(vec![IsfPassDefinition {
            target: None,
            persistent: false,
            float: false,
            width: None,
            height: None,
        }]);
    }
    passes
        .iter()
        .map(|pass| {
            let persistent = pass_flag(&pass.persistent, "PERSISTENT")?;
            let float = pass_flag(&pass.float, "FLOAT")?;
            if persistent && pass.target.is_none() {
                return Err(error("ISF PERSISTENT pass requires a named TARGET"));
            }
            Ok(IsfPassDefinition {
                target: pass.target.clone(),
                persistent,
                float,
                width: IsfDimension::parse(pass.width.as_ref())?,
                height: IsfDimension::parse(pass.height.as_ref())?,
            })
        })
        .collect()
}

fn pass_flag(value: &Value, name: &str) -> Result<bool, IsfPrepareError> {
    match value {
        Value::Null | Value::Bool(false) => Ok(false),
        Value::Bool(true) => Ok(true),
        Value::Number(value) => value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(|value| value > 0.0)
            .ok_or_else(|| {
                error(format!(
                    "ISF pass {name} must be a finite boolean or number"
                ))
            }),
        _ => Err(error(format!(
            "ISF pass {name} must be a boolean or number"
        ))),
    }
}

fn validate_shader_body(body: &str) -> Result<(), IsfPrepareError> {
    let lower = body.to_ascii_lowercase();
    for forbidden in [
        "#version",
        "#extension",
        "#include",
        "layout(",
        "uniform ",
        " buffer ",
        "imagestore",
        "atomic",
        "while(",
        "while (",
        "for(",
        "for (",
        "do{",
        "do {",
    ] {
        if lower.contains(forbidden) {
            return Err(error(format!(
                "ISF source uses forbidden or unsupported construct '{forbidden}'"
            )));
        }
    }
    if !body.contains("void main") {
        return Err(error("ISF source must define void main()"));
    }
    Ok(())
}

fn validate_identifier(name: &str) -> Result<(), IsfPrepareError> {
    let mut chars = name.chars();
    let valid_first = chars
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic());
    if !valid_first
        || !chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
        || name.starts_with("gl_")
        || name.starts_with("syndocal")
        || matches!(
            name,
            "RENDERSIZE"
                | "TIME"
                | "TIMEDELTA"
                | "FRAMEINDEX"
                | "PASSINDEX"
                | "DATE"
                | "isf_FragNormCoord"
                | "IMG_PIXEL"
                | "IMG_NORM_PIXEL"
                | "IMG_THIS_PIXEL"
                | "IMG_THIS_NORM_PIXEL"
                | "IMG_SIZE"
        )
    {
        return Err(error(format!(
            "ISF input name '{name}' is not a safe identifier"
        )));
    }
    Ok(())
}

fn control_definition(input: IsfInput) -> Result<IsfControlDefinition, IsfPrepareError> {
    let kind = match input.kind.as_str() {
        "event" => IsfControlKind::Event,
        "bool" => IsfControlKind::Bool,
        "long" => IsfControlKind::Long,
        "float" => IsfControlKind::Float,
        "point2D" => IsfControlKind::Point2d,
        "color" => IsfControlKind::Color,
        _ => unreachable!("control kind filtered before conversion"),
    };
    let fallback_default = match kind {
        IsfControlKind::Color => [0.0, 0.0, 0.0, 1.0],
        _ => [0.0; 4],
    };
    let fallback_minimum = match kind {
        IsfControlKind::Bool | IsfControlKind::Event | IsfControlKind::Color => [0.0; 4],
        _ => [-1_000.0; 4],
    };
    let fallback_maximum = match kind {
        IsfControlKind::Bool | IsfControlKind::Event | IsfControlKind::Color => [1.0; 4],
        _ => [1_000.0; 4],
    };
    let default = value_vector(input.default.as_ref(), fallback_default)?;
    let minimum = value_vector(input.minimum.as_ref(), fallback_minimum)?;
    let maximum = value_vector(input.maximum.as_ref(), fallback_maximum)?;
    if default
        .iter()
        .chain(minimum.iter())
        .chain(maximum.iter())
        .any(|value| !value.is_finite())
    {
        return Err(error(format!(
            "ISF input '{}' contains a non-finite value",
            input.name
        )));
    }
    Ok(IsfControlDefinition {
        name: input.name,
        kind,
        default,
        minimum,
        maximum,
        labels: input.labels.into_iter().take(64).collect(),
        values: input.values.into_iter().take(64).collect(),
    })
}

fn value_vector(value: Option<&Value>, fallback: [f32; 4]) -> Result<[f32; 4], IsfPrepareError> {
    let Some(value) = value else {
        return Ok(fallback);
    };
    match value {
        Value::Bool(value) => Ok([u8::from(*value) as f32, 0.0, 0.0, 0.0]),
        Value::Number(value) => value
            .as_f64()
            .map(|value| [value as f32, 0.0, 0.0, 0.0])
            .ok_or_else(|| error("ISF input number is outside the supported range")),
        Value::Array(values) if values.len() <= 4 => {
            let mut result = fallback;
            for (index, value) in values.iter().enumerate() {
                result[index] = value
                    .as_f64()
                    .map(|value| value as f32)
                    .ok_or_else(|| error("ISF input vector must contain only numbers"))?;
            }
            Ok(result)
        }
        _ => Err(error(
            "ISF input default/min/max has an unsupported value shape",
        )),
    }
}

fn build_glsl_wrapper(
    body: &str,
    image_names: &[String],
    controls: &[IsfControlDefinition],
) -> String {
    let mut header = format!(
        r#"#version 450 core
layout(location=0) in vec2 isf_FragNormCoord;
layout(location=0) out vec4 syndocal_FragColor;

layout(set=0,binding=1) uniform sampler syndocal_input_sampler;
layout(set=0,binding=2,std140) uniform SyndocalUniforms {{
    vec4 render_time;
    ivec4 frame_pass;
    vec4 controls[{ISF_MAX_CONTROL_INPUTS}];
}} syndocal;
#define gl_FragColor syndocal_FragColor
#define RENDERSIZE syndocal.render_time.xy
#define TIME syndocal.render_time.z
#define TIMEDELTA syndocal.render_time.w
#define FRAMEINDEX syndocal.frame_pass.x
#define PASSINDEX syndocal.frame_pass.y
#define IMG_THIS_PIXEL(image) texture(image, gl_FragCoord.xy / vec2(textureSize(image, 0)))
#define IMG_THIS_NORM_PIXEL(image) texture(image, isf_FragNormCoord)
#define IMG_NORM_PIXEL(image, coord) texture(image, coord)
#define IMG_PIXEL(image, coord) texture(image, (coord) / vec2(textureSize(image, 0)))
#define IMG_SIZE(image) vec2(textureSize(image, 0))
"#
    );
    for (index, name) in image_names.iter().enumerate() {
        let binding = if index == 0 { 0 } else { index + 2 };
        header.push_str(&format!(
            "layout(set=0,binding={binding}) uniform texture2D syndocal_texture_{index};\n#define {name} sampler2D(syndocal_texture_{index}, syndocal_input_sampler)\n"
        ));
    }
    header.push_str("#define texture2D texture\n");
    for (index, control) in controls.iter().enumerate() {
        let expression = match control.kind {
            IsfControlKind::Event | IsfControlKind::Bool => {
                format!("(syndocal.controls[{index}].x > 0.5)")
            }
            IsfControlKind::Long => format!("int(syndocal.controls[{index}].x)"),
            IsfControlKind::Float => format!("syndocal.controls[{index}].x"),
            IsfControlKind::Point2d => format!("syndocal.controls[{index}].xy"),
            IsfControlKind::Color => format!("syndocal.controls[{index}]"),
        };
        header.push_str(&format!("#define {} {expression}\n", control.name));
    }
    header.push('\n');
    header.push_str(body);
    header
}

fn glsl_to_wgsl(glsl: &str) -> Result<String, IsfPrepareError> {
    let mut frontend = Frontend::default();
    let module = frontend
        .parse(&Options::from(ShaderStage::Fragment), glsl)
        .map_err(|parse_errors| error(format!("ISF GLSL compile failed: {parse_errors:?}")))?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|validation_error| {
            error(format!("ISF shader validation failed: {validation_error}"))
        })?;
    naga::back::wgsl::write_string(&module, &info, WriterFlags::EXPLICIT_TYPES)
        .map_err(|write_error| error(format!("ISF WGSL translation failed: {write_error}")))
}

fn error(message: impl Into<String>) -> IsfPrepareError {
    IsfPrepareError {
        message: message.into(),
    }
}

fn runtime_error(message: impl Into<String>) -> IsfRuntimeError {
    IsfRuntimeError {
        message: message.into(),
    }
}

fn runtime_from_composite_error(error: CpuCompositeError) -> IsfRuntimeError {
    runtime_error(format!("ISF input conversion failed: {error:?}"))
}

fn source_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

fn isf_uniform_data(
    width: u32,
    height: u32,
    time_seconds: f32,
    time_delta_seconds: f32,
    frame_index: u32,
    pass_index: usize,
    controls: &[[f32; 4]],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32 + ISF_MAX_CONTROL_INPUTS * 16);
    for value in [
        width as f32,
        height as f32,
        if time_seconds.is_finite() {
            time_seconds.max(0.0)
        } else {
            0.0
        },
        if time_delta_seconds.is_finite() {
            time_delta_seconds.max(0.0)
        } else {
            0.0
        },
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [frame_index as i32, pass_index as i32, 0_i32, 0_i32] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for index in 0..ISF_MAX_CONTROL_INPUTS {
        let values = controls.get(index).copied().unwrap_or([0.0; 4]);
        for value in values {
            bytes.extend_from_slice(&if value.is_finite() { value } else { 0.0 }.to_le_bytes());
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOAT_FILTER: &str = r#"/*{
      "DESCRIPTION": "Threshold",
      "CATEGORIES": ["Color"],
      "INPUTS": [
        {"NAME":"inputImage","TYPE":"image"},
        {"NAME":"level","TYPE":"float","DEFAULT":0.5,"MIN":0.0,"MAX":1.0}
      ]
    }*/
    void main() {
      vec4 srcPixel = IMG_THIS_PIXEL(inputImage);
      float luma = (srcPixel.r + srcPixel.g + srcPixel.b) / 3.0;
      gl_FragColor = (luma > level) ? srcPixel : vec4(0.0, 0.0, 0.0, 1.0);
    }
    "#;

    #[test]
    fn prepares_single_pass_image_filter_as_valid_wgsl() {
        let prepared = prepare_isf_shader(FLOAT_FILTER).unwrap();
        assert_eq!(prepared.image_input_name, "inputImage");
        assert_eq!(prepared.controls.len(), 1);
        assert_eq!(prepared.controls[0].name, "level");
        assert_eq!(prepared.controls[0].default[0], 0.5);
        assert!(prepared.wgsl.contains("@fragment"));
        assert!(prepared.wgsl.contains("@group(0) @binding(0)"));
        assert!(prepared.wgsl.contains("@group(0) @binding(1)"));
        assert!(prepared.wgsl.contains("@group(0) @binding(2)"));
    }

    #[test]
    fn accepts_target_passes_and_rejects_audio_loops_and_unsafe_identifiers() {
        let multipass = FLOAT_FILTER.replace(
            "\"INPUTS\"",
            "\"PASSES\":[{\"TARGET\":\"buffer\"}],\"INPUTS\"",
        );
        let prepared = prepare_isf_shader(&multipass).unwrap();
        assert_eq!(prepared.passes[0].target.as_deref(), Some("buffer"));
        let audio = FLOAT_FILTER.replace(
            "{\"NAME\":\"level\",\"TYPE\":\"float\",\"DEFAULT\":0.5,\"MIN\":0.0,\"MAX\":1.0}",
            "{\"NAME\":\"audio\",\"TYPE\":\"audioFFT\"}",
        );
        assert!(prepare_isf_shader(&audio)
            .unwrap_err()
            .message
            .contains("Audio FFT"));
        assert!(prepare_isf_shader(
            &FLOAT_FILTER.replace("void main()", "void main(){ for(;;){} } void unused()")
        )
        .unwrap_err()
        .message
        .contains("for("));
        let unsafe_name = FLOAT_FILTER.replace("\"level\"", "\"TIME\"");
        assert!(prepare_isf_shader(&unsafe_name)
            .unwrap_err()
            .message
            .contains("safe identifier"));
    }

    #[test]
    fn gpu_runtime_executes_translated_threshold_filter() {
        let Ok(mut runtime) = IsfGpuRuntime::new() else {
            eprintln!("ISF GPU runtime test skipped: no adapter");
            return;
        };
        let shader = prepare_isf_shader(FLOAT_FILTER).unwrap();
        let frame = VideoFrame {
            layer_id: 9,
            width: 2,
            height: 1,
            pts_ms: 1_000,
            duration_ms: 33,
            format: VideoPixelFormat::Rgba8,
            data: vec![64, 64, 64, 255, 200, 200, 200, 255],
        };
        let output = runtime
            .apply(&frame, &shader, &[[0.5, 0.0, 0.0, 0.0]], 1.0, 0.033)
            .unwrap();
        assert_eq!(output.layer_id, 9);
        assert_eq!(output.pts_ms, 1_000);
        assert_eq!(output.data, vec![0, 0, 0, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn stage_local_chain_uses_one_frame_index_and_advances_once_even_when_all_fail() {
        let mut runtime =
            IsfGpuRuntime::new().expect("FRAMEINDEX regression requires a GPU adapter");
        const WRITE_RED_INDEX: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          vec4 pixel = IMG_THIS_PIXEL(inputImage);
          pixel.r = float(FRAMEINDEX) / 255.0;
          gl_FragColor = pixel;
        }"#;
        const WRITE_GREEN_INDEX: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          vec4 pixel = IMG_THIS_PIXEL(inputImage);
          pixel.g = float(FRAMEINDEX) / 255.0;
          gl_FragColor = pixel;
        }"#;
        let write_red = prepare_isf_shader(WRITE_RED_INDEX).unwrap();
        let write_green = prepare_isf_shader(WRITE_GREEN_INDEX).unwrap();
        let mut invalid = write_red.clone();
        invalid.wgsl = "this is not valid WGSL".to_string();
        let controls: &[[f32; 4]] = &[];
        let frame = VideoFrame {
            layer_id: 5,
            width: 1,
            height: 1,
            pts_ms: 0,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![9, 19, 29, 255],
        };

        let (first, first_failures) = runtime.apply_stack_stage_local(
            &frame,
            &[
                (&write_red, controls),
                (&invalid, controls),
                (&write_green, controls),
            ],
            0.0,
            0.016,
        );
        assert_eq!(first_failures.len(), 1);
        assert_eq!(first_failures[0].0, 1);
        assert_eq!(first.data, vec![0, 0, 29, 255]);

        let (second, second_failures) = runtime.apply_stack_stage_local(
            &frame,
            &[(&write_red, controls), (&write_green, controls)],
            0.016,
            0.016,
        );
        assert!(second_failures.is_empty());
        assert_eq!(second.data, vec![1, 1, 29, 255]);

        let (all_failed, all_failures) = runtime.apply_stack_stage_local(
            &frame,
            &[(&invalid, controls), (&invalid, controls)],
            0.032,
            0.016,
        );
        assert_eq!(all_failures.len(), 2);
        assert_eq!(all_failed, frame);

        let (after_all_fail, failures) = runtime.apply_stack_stage_local(
            &frame,
            &[(&write_red, controls), (&write_green, controls)],
            0.048,
            0.016,
        );
        assert!(failures.is_empty());
        assert_eq!(after_all_fail.data, vec![3, 3, 29, 255]);

        let legacy = runtime
            .apply_stack(&frame, &[(&write_red, controls)], 0.064, 0.016)
            .unwrap();
        assert_eq!(legacy.data, vec![4, 19, 29, 255]);
        assert!(runtime
            .apply_stack(&frame, &[(&invalid, controls)], 0.080, 0.016)
            .is_err());
        let legacy_after_failure = runtime
            .apply_stack(&frame, &[(&write_red, controls)], 0.096, 0.016)
            .unwrap();
        assert_eq!(legacy_after_failure.data, vec![5, 19, 29, 255]);
    }
}

#[cfg(test)]
#[path = "isf_completeness_tests.rs"]
mod completeness_tests;
