use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    sync::mpsc,
};

use naga::{
    back::wgsl::WriterFlags,
    front::glsl::{Frontend, Options},
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage,
};
use serde::Deserialize;
use serde_json::Value;
use wgpu::util::DeviceExt;

use crate::{convert_frame_to_rgba8, CpuCompositeError, VideoFrame, VideoPixelFormat};
use protocol::{VideoIsfControlKind, VideoIsfControlSummary, VideoIsfEffectSummary};

pub const ISF_MAX_SOURCE_BYTES: usize = 512 * 1024;
pub const ISF_MAX_CONTROL_INPUTS: usize = 16;

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
    pipelines: HashMap<u64, IsfGpuPipeline>,
    frame_index: u32,
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
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal ISF runtime"),
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
            frame_index: 0,
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
        if frame.width == 0 || frame.height == 0 {
            return Err(runtime_error("ISF input frame has an invalid size"));
        }
        let rgba = if frame.format == VideoPixelFormat::Rgba8 {
            frame.clone()
        } else {
            convert_frame_to_rgba8(frame).map_err(runtime_from_composite_error)?
        };
        let expected_len = frame.width as usize * frame.height as usize * 4;
        if rgba.data.len() != expected_len {
            return Err(runtime_error(
                "ISF input frame byte length does not match its dimensions",
            ));
        }

        let shader_key = source_hash(&shader.wgsl);
        if !self.pipelines.contains_key(&shader_key) {
            let pipeline = self.create_pipeline(&shader.wgsl)?;
            self.pipelines.insert(shader_key, pipeline);
        }
        let pipeline = self
            .pipelines
            .get(&shader_key)
            .expect("ISF pipeline inserted before rendering");
        let input_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syndocal ISF input"),
            size: wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &input_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(frame.width * 4),
                rows_per_image: Some(frame.height),
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );
        let input_view = input_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Syndocal ISF sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform_data = isf_uniform_data(
            frame.width,
            frame.height,
            time_seconds,
            time_delta_seconds,
            self.frame_index,
            controls,
        );
        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal ISF uniforms"),
                contents: &uniform_data,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Syndocal ISF bind group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        let output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syndocal ISF output"),
            size: wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let padded_bytes_per_row = (frame.width * 4).div_ceil(256) * 256;
        let readback_size = u64::from(padded_bytes_per_row) * u64::from(frame.height);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal ISF readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal ISF encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Syndocal ISF pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(frame.height),
                },
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let (sender, receiver) = mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result.map_err(|error| error.to_string()));
            });
        self.device
            .poll(wgpu::PollType::wait())
            .map_err(|error| runtime_error(format!("ISF GPU wait failed: {error}")))?;
        receiver
            .recv()
            .map_err(|error| runtime_error(format!("ISF readback channel failed: {error}")))?
            .map_err(|error| runtime_error(format!("ISF readback failed: {error}")))?;
        let mapped = readback.slice(..).get_mapped_range();
        let mut data = Vec::with_capacity(expected_len);
        for row in mapped.chunks_exact(padded_bytes_per_row as usize) {
            data.extend_from_slice(&row[..(frame.width * 4) as usize]);
        }
        drop(mapped);
        readback.unmap();
        self.frame_index = self.frame_index.wrapping_add(1);
        Ok(VideoFrame {
            layer_id: frame.layer_id,
            width: frame.width,
            height: frame.height,
            pts_ms: frame.pts_ms,
            duration_ms: frame.duration_ms,
            format: VideoPixelFormat::Rgba8,
            data,
        })
    }

    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    fn create_pipeline(&self, wgsl: &str) -> Result<IsfGpuPipeline, IsfRuntimeError> {
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Syndocal ISF bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Syndocal ISF pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Syndocal translated ISF fragment shader"),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Syndocal ISF pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &self.vertex_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(runtime_error(format!(
                "ISF GPU pipeline validation failed: {error}"
            )));
        }
        Ok(IsfGpuPipeline {
            bind_group_layout,
            pipeline,
        })
    }
}

pub fn resolved_isf_control_values(
    shader: &PreparedIsfShader,
    effect: &VideoIsfEffectSummary,
) -> Vec<[f32; 4]> {
    shader
        .controls
        .iter()
        .map(|definition| {
            let candidate = effect
                .controls
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
    source: String,
    source_path: Option<String>,
    shader: &PreparedIsfShader,
) -> VideoIsfEffectSummary {
    VideoIsfEffectSummary {
        enabled: true,
        label,
        source,
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
            })
            .collect(),
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
}

pub fn prepare_isf_shader(source: &str) -> Result<PreparedIsfShader, IsfPrepareError> {
    if source.len() > ISF_MAX_SOURCE_BYTES {
        return Err(error(format!(
            "ISF source exceeds the {} byte safety limit",
            ISF_MAX_SOURCE_BYTES
        )));
    }
    let (metadata_json, body) = split_isf_source(source)?;
    let metadata: IsfMetadata = serde_json::from_str(metadata_json)
        .map_err(|parse_error| error(format!("ISF metadata JSON is invalid: {parse_error}")))?;
    validate_single_pass_boundary(&metadata)?;
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
    let glsl = build_glsl_wrapper(body, &image_input_name, &controls);
    let wgsl = glsl_to_wgsl(&glsl)?;
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

fn validate_single_pass_boundary(metadata: &IsfMetadata) -> Result<(), IsfPrepareError> {
    if !metadata.imported.is_empty() {
        return Err(error("ISF imported image resources are not supported"));
    }
    if metadata.passes.len() > 1 {
        return Err(error("Multi-pass ISF shaders are not supported"));
    }
    if let Some(pass) = metadata.passes.first() {
        let persistent = matches!(&pass.persistent, Value::Bool(true))
            || pass.persistent.as_i64().is_some_and(|value| value != 0);
        if persistent
            || pass
                .target
                .as_deref()
                .is_some_and(|target| !target.trim().is_empty())
        {
            return Err(error(
                "Persistent or target-buffer ISF passes are not supported",
            ));
        }
    }
    Ok(())
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
        || matches!(
            name,
            "RENDERSIZE" | "TIME" | "TIMEDELTA" | "FRAMEINDEX" | "PASSINDEX" | "isf_FragNormCoord"
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
    image_input_name: &str,
    controls: &[IsfControlDefinition],
) -> String {
    let mut header = format!(
        r#"#version 450 core
layout(location=0) in vec2 isf_FragNormCoord;
layout(location=0) out vec4 syndocal_FragColor;
layout(set=0,binding=0) uniform texture2D syndocal_input_texture;
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
#define {image_input_name} sampler2D(syndocal_input_texture, syndocal_input_sampler)
#define IMG_THIS_PIXEL(image) texture(image, isf_FragNormCoord)
#define IMG_THIS_NORM_PIXEL(image) texture(image, isf_FragNormCoord)
#define IMG_NORM_PIXEL(image, coord) texture(image, coord)
#define IMG_PIXEL(image, coord) texture(image, (coord) / RENDERSIZE)
#define IMG_SIZE(image) RENDERSIZE
#define texture2D texture
"#
    );
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
    for value in [frame_index as i32, 0_i32, 0_i32, 0_i32] {
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
    fn rejects_multipass_imports_audio_loops_and_unsafe_identifiers() {
        let multipass = FLOAT_FILTER.replace(
            "\"INPUTS\"",
            "\"PASSES\":[{\"TARGET\":\"buffer\"}],\"INPUTS\"",
        );
        assert!(prepare_isf_shader(&multipass)
            .unwrap_err()
            .message
            .contains("target-buffer"));
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
}
