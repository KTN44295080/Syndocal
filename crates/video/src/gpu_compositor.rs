use std::sync::mpsc;

use protocol::{Transform2D, VideoBlendMode, VideoColorAdjust, VideoFxAdjust, VideoOutputMapping};
use wgpu::util::DeviceExt;

use crate::{
    convert_frame_to_rgba8, finite_or, sanitize_color_adjust, sanitize_fx_adjust,
    video_output_mapping_scale, CompositionPlan, CpuCompositeError, VideoFrame, VideoPixelFormat,
};

const WORKGROUP_WIDTH: u32 = 8;
const WORKGROUP_HEIGHT: u32 = 8;

pub(crate) fn dispatch_dimensions(width: u32, height: u32) -> (u32, u32) {
    (
        width.div_ceil(WORKGROUP_WIDTH),
        height.div_ceil(WORKGROUP_HEIGHT),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuCompositeError {
    Cpu(CpuCompositeError),
    Adapter(String),
    Device(String),
    BufferMap(String),
}

impl From<CpuCompositeError> for GpuCompositeError {
    fn from(error: CpuCompositeError) -> Self {
        Self::Cpu(error)
    }
}

pub struct GpuCompositor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: GpuCompositePipelines,
    adapter_name: String,
}

pub(crate) struct GpuCompositePipelines {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub composite: wgpu::ComputePipeline,
    pub output_mapping: wgpu::ComputePipeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GpuFrameTextureSpec {
    pub format: wgpu::TextureFormat,
    pub texture_width: u32,
    pub texture_height: u32,
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
    pub source_format: u32,
    pub expected_len: usize,
}

pub(crate) fn requested_video_device_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let supported = adapter.features();
    if supported.contains(wgpu::Features::TEXTURE_COMPRESSION_BC) {
        wgpu::Features::TEXTURE_COMPRESSION_BC
    } else {
        wgpu::Features::empty()
    }
}

pub(crate) fn gpu_frame_texture_spec(
    frame: &VideoFrame,
    bc_supported: bool,
) -> Result<GpuFrameTextureSpec, CpuCompositeError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    let rgba_expected_len = (frame.width as usize)
        .checked_mul(frame.height as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    let block_width = frame.width.checked_add(3).map(|width| width / 4);
    let block_height = frame.height.checked_add(3).map(|height| height / 4);
    let padded_width = block_width.and_then(|blocks| blocks.checked_mul(4));
    let padded_height = block_height.and_then(|blocks| blocks.checked_mul(4));
    let dxt1_expected_len = block_width
        .map(|blocks| blocks as usize)
        .and_then(|width| block_height.map(|height| (width, height as usize)))
        .and_then(|(width, height)| width.checked_mul(height))
        .and_then(|blocks| blocks.checked_mul(8));
    let dxt5_expected_len = block_width
        .map(|blocks| blocks as usize)
        .and_then(|width| block_height.map(|height| (width, height as usize)))
        .and_then(|(width, height)| width.checked_mul(height))
        .and_then(|blocks| blocks.checked_mul(16));
    let (
        format,
        texture_width,
        texture_height,
        bytes_per_row,
        rows_per_image,
        source_format,
        expected_len,
    ) = match frame.format {
        VideoPixelFormat::Rgba8 => (
            wgpu::TextureFormat::Rgba8Unorm,
            Some(frame.width),
            Some(frame.height),
            frame.width.checked_mul(4),
            Some(frame.height),
            1,
            rgba_expected_len,
        ),
        VideoPixelFormat::Bgra8 => (
            wgpu::TextureFormat::Bgra8Unorm,
            Some(frame.width),
            Some(frame.height),
            frame.width.checked_mul(4),
            Some(frame.height),
            1,
            rgba_expected_len,
        ),
        VideoPixelFormat::Dxt1 => (
            wgpu::TextureFormat::Bc1RgbaUnorm,
            padded_width,
            padded_height,
            frame.width.div_ceil(4).checked_mul(8),
            Some(frame.height.div_ceil(4)),
            1,
            dxt1_expected_len,
        ),
        VideoPixelFormat::Dxt5 => (
            wgpu::TextureFormat::Bc3RgbaUnorm,
            padded_width,
            padded_height,
            frame.width.div_ceil(4).checked_mul(16),
            Some(frame.height.div_ceil(4)),
            1,
            dxt5_expected_len,
        ),
        VideoPixelFormat::YcoCgDxt5 => (
            wgpu::TextureFormat::Bc3RgbaUnorm,
            padded_width,
            padded_height,
            frame.width.div_ceil(4).checked_mul(16),
            Some(frame.height.div_ceil(4)),
            2,
            dxt5_expected_len,
        ),
    };
    if matches!(
        frame.format,
        VideoPixelFormat::Dxt1 | VideoPixelFormat::Dxt5 | VideoPixelFormat::YcoCgDxt5
    ) && !bc_supported
    {
        return Err(CpuCompositeError::UnsupportedFrameFormat {
            layer_id: frame.layer_id,
            format: frame.format,
        });
    }
    let (Some(texture_width), Some(texture_height), Some(bytes_per_row), Some(expected_len)) =
        (texture_width, texture_height, bytes_per_row, expected_len)
    else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    if frame.data.len() != expected_len {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    Ok(GpuFrameTextureSpec {
        format,
        texture_width,
        texture_height,
        bytes_per_row,
        rows_per_image: rows_per_image.expect("frame rows are present"),
        source_format,
        expected_len,
    })
}

pub(crate) fn create_frame_texture(
    device: &wgpu::Device,
    spec: GpuFrameTextureSpec,
    label: &'static str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: spec.texture_width,
            height: spec.texture_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: spec.format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

pub(crate) fn write_frame_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    frame: &VideoFrame,
    spec: GpuFrameTextureSpec,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &frame.data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(spec.bytes_per_row),
            rows_per_image: Some(spec.rows_per_image),
        },
        wgpu::Extent3d {
            width: spec.texture_width,
            height: spec.texture_height,
            depth_or_array_layers: 1,
        },
    );
}

pub(crate) fn create_gpu_composite_pipelines(device: &wgpu::Device) -> GpuCompositePipelines {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Syndocal video GPU compositor bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Syndocal video GPU compositor pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Syndocal video GPU compositor shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("gpu_compositor.wgsl").into()),
    });
    let composite = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Syndocal video GPU compositor pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("composite_layer"),
        compilation_options: Default::default(),
        cache: None,
    });
    let output_mapping_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Syndocal video GPU output mapping shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("gpu_output_mapping.wgsl").into()),
    });
    let output_mapping = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Syndocal video GPU output mapping pipeline"),
        layout: Some(&pipeline_layout),
        module: &output_mapping_shader,
        entry_point: Some("map_output"),
        compilation_options: Default::default(),
        cache: None,
    });
    GpuCompositePipelines {
        bind_group_layout,
        composite,
        output_mapping,
    }
}

impl GpuCompositor {
    pub fn new() -> Result<Self, GpuCompositeError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .map_err(|error| GpuCompositeError::Adapter(error.to_string()))?;
        let adapter_name = adapter.get_info().name;
        let required_features = requested_video_device_features(&adapter);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal video GPU compositor"),
            required_features,
            ..Default::default()
        }))
        .map_err(|error| GpuCompositeError::Device(error.to_string()))?;

        let pipelines = create_gpu_composite_pipelines(&device);

        Ok(Self {
            device,
            queue,
            pipelines,
            adapter_name,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn supports_bc_texture(&self) -> bool {
        self.device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    }

    pub fn composite_rgba8(
        &self,
        plan: &CompositionPlan,
        frames: &[VideoFrame],
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, GpuCompositeError> {
        let pixel_count = width
            .checked_mul(height)
            .filter(|count| *count > 0)
            .ok_or(CpuCompositeError::InvalidOutputSize)?;
        let buffer_size = u64::from(pixel_count) * 4;
        let output_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal video GPU compositor output"),
                contents: &vec![0; buffer_size as usize],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        for layer in &plan.layers {
            let frame = frames
                .iter()
                .filter(|frame| frame.layer_id == layer.layer_id)
                .max_by_key(|frame| frame.pts_ms)
                .ok_or(CpuCompositeError::MissingFrame {
                    layer_id: layer.layer_id,
                })?;
            let texture_spec = gpu_frame_texture_spec(
                frame,
                self.device
                    .features()
                    .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
            )?;
            let (source_texture, source_texture_view) = create_frame_texture(
                &self.device,
                texture_spec,
                "Syndocal video GPU compositor source texture",
            );
            write_frame_texture(&self.queue, &source_texture, frame, texture_spec);
            let source_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Syndocal video GPU compositor unused source buffer"),
                    contents: &[0; 4],
                    usage: wgpu::BufferUsages::STORAGE,
                });
            let params = compositor_params(
                width,
                height,
                frame.width,
                frame.height,
                layer.opacity.clamp(0.0, 1.0),
                blend_mode_index(&layer.blend_mode),
                texture_spec.source_format,
                &layer.transform,
                &layer.color,
                &layer.fx,
            );
            let params_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Syndocal video GPU compositor params"),
                    contents: &params,
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Syndocal video GPU compositor bind group"),
                layout: &self.pipelines.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: source_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&source_texture_view),
                    },
                ],
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Syndocal video GPU compositor encoder"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Syndocal video GPU compositor pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.composite);
                pass.set_bind_group(0, &bind_group, &[]);
                let (workgroups_x, workgroups_y) = dispatch_dimensions(width, height);
                pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
            }
            self.queue.submit(Some(encoder.finish()));
        }

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal video GPU compositor readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal video GPU compositor readback encoder"),
            });
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &readback, 0, buffer_size);
        self.queue.submit(Some(encoder.finish()));

        let (sender, receiver) = mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result.map_err(|error| error.to_string()));
            });
        self.device
            .poll(wgpu::PollType::wait())
            .map_err(|error| GpuCompositeError::BufferMap(error.to_string()))?;
        receiver
            .recv()
            .map_err(|error| GpuCompositeError::BufferMap(error.to_string()))?
            .map_err(GpuCompositeError::BufferMap)?;
        let data = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();

        Ok(VideoFrame {
            layer_id: 0,
            width,
            height,
            pts_ms: plan
                .layers
                .iter()
                .filter_map(|layer| {
                    frames
                        .iter()
                        .filter(|frame| frame.layer_id == layer.layer_id)
                        .map(|frame| frame.pts_ms)
                        .max()
                })
                .max()
                .unwrap_or(0),
            duration_ms: 0,
            format: VideoPixelFormat::Rgba8,
            data,
        })
    }

    pub fn apply_output_mapping_rgba8(
        &self,
        frame: &VideoFrame,
        mapping: &VideoOutputMapping,
    ) -> Result<VideoFrame, GpuCompositeError> {
        let normalized = convert_frame_to_rgba8(frame)?;
        let pixel_count = normalized
            .width
            .checked_mul(normalized.height)
            .filter(|count| *count > 0)
            .ok_or(CpuCompositeError::InvalidOutputSize)?;
        let buffer_size = u64::from(pixel_count) * 4;
        let source_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal video GPU output mapping source"),
                contents: &normalized.data,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let output_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal video GPU output mapping output"),
                contents: &vec![0; buffer_size as usize],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });
        let params = output_mapping_params(normalized.width, normalized.height, mapping);
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal video GPU output mapping params"),
                contents: &params,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let dummy_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syndocal video GPU output mapping unused texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Syndocal video GPU output mapping bind group"),
            layout: &self.pipelines.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&dummy_texture_view),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal video GPU output mapping encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal video GPU output mapping pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.output_mapping);
            pass.set_bind_group(0, &bind_group, &[]);
            let (workgroups_x, workgroups_y) =
                dispatch_dimensions(normalized.width, normalized.height);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        let data = self.readback_rgba8(&output_buffer, buffer_size)?;

        Ok(VideoFrame {
            format: VideoPixelFormat::Rgba8,
            data,
            ..normalized
        })
    }

    fn readback_rgba8(
        &self,
        source_buffer: &wgpu::Buffer,
        buffer_size: u64,
    ) -> Result<Vec<u8>, GpuCompositeError> {
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal video GPU readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal video GPU readback encoder"),
            });
        encoder.copy_buffer_to_buffer(source_buffer, 0, &readback, 0, buffer_size);
        self.queue.submit(Some(encoder.finish()));

        let (sender, receiver) = mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result.map_err(|error| error.to_string()));
            });
        self.device
            .poll(wgpu::PollType::wait())
            .map_err(|error| GpuCompositeError::BufferMap(error.to_string()))?;
        receiver
            .recv()
            .map_err(|error| GpuCompositeError::BufferMap(error.to_string()))?
            .map_err(GpuCompositeError::BufferMap)?;
        let data = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();
        Ok(data)
    }
}

pub(crate) fn blend_mode_index(mode: &VideoBlendMode) -> u32 {
    match mode {
        VideoBlendMode::Normal => 0,
        VideoBlendMode::Add => 1,
        VideoBlendMode::Multiply => 2,
        VideoBlendMode::Screen => 3,
    }
}

pub(crate) fn compositor_params(
    output_width: u32,
    output_height: u32,
    source_width: u32,
    source_height: u32,
    opacity: f32,
    blend_mode: u32,
    source_format: u32,
    transform: &Transform2D,
    color: &VideoColorAdjust,
    fx: &VideoFxAdjust,
) -> [u8; 128] {
    let color = sanitize_color_adjust(*color);
    let fx = sanitize_fx_adjust(*fx);
    let crop_left = transform.crop_left.clamp(0.0, 1.0);
    let crop_top = transform.crop_top.clamp(0.0, 1.0);
    let crop_right = (1.0 - transform.crop_right.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let crop_bottom = (1.0 - transform.crop_bottom.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let values = [
        output_width,
        output_height,
        source_width,
        source_height,
        opacity.to_bits(),
        blend_mode,
        source_format,
        0,
        transform.x.to_bits(),
        transform.y.to_bits(),
        transform.scale_x.max(0.001).to_bits(),
        transform.scale_y.max(0.001).to_bits(),
        (-transform.rotation_deg.to_radians()).to_bits(),
        crop_left.to_bits(),
        crop_top.to_bits(),
        crop_right.to_bits(),
        crop_bottom.to_bits(),
        color.brightness.to_bits(),
        color.contrast.to_bits(),
        color.hue_deg.to_radians().to_bits(),
        color.saturation.to_bits(),
        color.gamma.to_bits(),
        0,
        0,
        fx.pixelate.to_bits(),
        fx.blur.to_bits(),
        fx.glow.to_bits(),
        fx.edge.to_bits(),
        fx.key_red.to_bits(),
        fx.key_green.to_bits(),
        fx.key_blue.to_bits(),
        fx.key_threshold.to_bits(),
    ];
    let mut bytes = [0; 128];
    for (index, value) in values.into_iter().enumerate() {
        let start = index * 4;
        bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(crate) fn output_mapping_params(
    width: u32,
    height: u32,
    mapping: &VideoOutputMapping,
) -> [u8; 80] {
    let output_aspect = (width as f32 / height as f32).max(0.001);
    let (scale_x, scale_y) = video_output_mapping_scale(mapping, output_aspect);
    let values = [
        width,
        height,
        0,
        0,
        finite_or(mapping.offset_x, 0.0).to_bits(),
        finite_or(mapping.offset_y, 0.0).to_bits(),
        scale_x.to_bits(),
        scale_y.to_bits(),
        (-finite_or(mapping.rotation_deg, 0.0).to_radians()).to_bits(),
        finite_or(mapping.lens_distortion, 0.0)
            .clamp(-1.0, 1.0)
            .to_bits(),
        finite_or(mapping.keystone_x, 0.0)
            .clamp(-1.0, 1.0)
            .to_bits(),
        finite_or(mapping.keystone_y, 0.0)
            .clamp(-1.0, 1.0)
            .to_bits(),
        finite_or(mapping.corner_top_left_x, 0.0).to_bits(),
        finite_or(mapping.corner_top_left_y, 0.0).to_bits(),
        finite_or(mapping.corner_top_right_x, 0.0).to_bits(),
        finite_or(mapping.corner_top_right_y, 0.0).to_bits(),
        finite_or(mapping.corner_bottom_left_x, 0.0).to_bits(),
        finite_or(mapping.corner_bottom_left_y, 0.0).to_bits(),
        finite_or(mapping.corner_bottom_right_x, 0.0).to_bits(),
        finite_or(mapping.corner_bottom_right_y, 0.0).to_bits(),
    ];
    let mut bytes = [0; 80];
    for (index, value) in values.into_iter().enumerate() {
        let start = index * 4;
        bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use protocol::{
        VideoLayerId, VideoOutputAspectMode, VideoOutputMapping, VideoSourceKind,
        VideoSourceSummary,
    };

    use super::*;
    use crate::{apply_video_output_mapping, composite_rgba8, CompositionLayerPlan};

    #[test]
    fn dispatch_dimensions_cover_hd_and_4k_without_exceeding_gpu_limits() {
        assert_eq!(dispatch_dimensions(1920, 1080), (240, 135));
        assert_eq!(dispatch_dimensions(3840, 2160), (480, 270));
        assert!(dispatch_dimensions(3840, 2160).0 <= 65_535);
        assert!(dispatch_dimensions(3840, 2160).1 <= 65_535);
    }

    fn compositor() -> Option<GpuCompositor> {
        match GpuCompositor::new() {
            Ok(compositor) => {
                assert!(!compositor.adapter_name().trim().is_empty());
                Some(compositor)
            }
            Err(error) => {
                eprintln!("GPU compositor test skipped: {error:?}");
                None
            }
        }
    }

    fn layer_plan(
        layer_id: VideoLayerId,
        blend_mode: VideoBlendMode,
        opacity: f32,
    ) -> CompositionLayerPlan {
        CompositionLayerPlan {
            layer_id,
            label: format!("Video Layer {layer_id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("layer-{layer_id}.mov")),
                name: None,
                codec: None,
                metadata: None,
            },
            blend_mode,
            opacity,
            position_ms: 0,
            transform: Transform2D::default(),
            color: VideoColorAdjust::default(),
            fx: VideoFxAdjust::default(),
        }
    }

    fn plan(layers: Vec<CompositionLayerPlan>) -> CompositionPlan {
        CompositionPlan {
            composition_id: 1,
            label: "Main".to_string(),
            output_ids: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
            layers,
        }
    }

    fn frame(
        layer_id: VideoLayerId,
        width: u32,
        height: u32,
        format: VideoPixelFormat,
        data: Vec<u8>,
    ) -> VideoFrame {
        VideoFrame {
            layer_id,
            width,
            height,
            pts_ms: layer_id,
            duration_ms: 16,
            format,
            data,
        }
    }

    fn assert_gpu_matches_cpu(
        compositor: &GpuCompositor,
        plan: &CompositionPlan,
        frames: &[VideoFrame],
        width: u32,
        height: u32,
    ) {
        let cpu = composite_rgba8(plan, frames, width, height).unwrap();
        let gpu = compositor
            .composite_rgba8(plan, frames, width, height)
            .unwrap();
        assert_eq!(gpu.width, cpu.width);
        assert_eq!(gpu.height, cpu.height);
        assert_eq!(gpu.pts_ms, cpu.pts_ms);
        assert_eq!(gpu.format, VideoPixelFormat::Rgba8);
        assert_eq!(gpu.data, cpu.data);
    }

    fn assert_gpu_matches_cpu_with_channel_tolerance(
        compositor: &GpuCompositor,
        plan: &CompositionPlan,
        frames: &[VideoFrame],
        width: u32,
        height: u32,
        tolerance: u8,
    ) {
        let cpu = composite_rgba8(plan, frames, width, height).unwrap();
        let gpu = compositor
            .composite_rgba8(plan, frames, width, height)
            .unwrap();
        assert_eq!(gpu.width, cpu.width);
        assert_eq!(gpu.height, cpu.height);
        assert_eq!(gpu.pts_ms, cpu.pts_ms);
        assert_eq!(gpu.format, VideoPixelFormat::Rgba8);
        assert_eq!(gpu.data.len(), cpu.data.len());
        for (index, (gpu_channel, cpu_channel)) in gpu.data.iter().zip(&cpu.data).enumerate() {
            assert!(
                gpu_channel.abs_diff(*cpu_channel) <= tolerance,
                "channel {index} differs: GPU={gpu_channel}, CPU={cpu_channel}, tolerance={tolerance}"
            );
        }
    }

    fn assert_gpu_mapping_matches_cpu(
        compositor: &GpuCompositor,
        frame: &VideoFrame,
        mapping: &VideoOutputMapping,
    ) {
        let cpu = apply_video_output_mapping(frame.clone(), mapping);
        let gpu = compositor
            .apply_output_mapping_rgba8(frame, mapping)
            .unwrap();
        assert_eq!(gpu.width, cpu.width);
        assert_eq!(gpu.height, cpu.height);
        assert_eq!(gpu.pts_ms, cpu.pts_ms);
        assert_eq!(gpu.format, VideoPixelFormat::Rgba8);
        assert_eq!(gpu.data, cpu.data);
    }

    #[test]
    fn gpu_compositor_matches_cpu_for_rgba_bgra_dxt1_and_dxt5() {
        let Some(compositor) = compositor() else {
            return;
        };

        let rgba = frame(1, 1, 1, VideoPixelFormat::Rgba8, vec![10, 20, 30, 255]);
        assert_gpu_matches_cpu(
            &compositor,
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[rgba],
            1,
            1,
        );

        let bgra = frame(2, 1, 1, VideoPixelFormat::Bgra8, vec![30, 20, 10, 255]);
        assert_gpu_matches_cpu(
            &compositor,
            &plan(vec![layer_plan(2, VideoBlendMode::Normal, 1.0)]),
            &[bgra],
            1,
            1,
        );

        if !compositor.supports_bc_texture() {
            eprintln!("Compressed texture checks skipped: adapter has no BC texture support");
            return;
        }

        let mut dxt1_block = Vec::new();
        dxt1_block.extend_from_slice(&0xf800u16.to_le_bytes());
        dxt1_block.extend_from_slice(&0x001fu16.to_le_bytes());
        dxt1_block.extend_from_slice(&0u32.to_le_bytes());
        let dxt1 = frame(3, 2, 2, VideoPixelFormat::Dxt1, dxt1_block);
        assert_gpu_matches_cpu(
            &compositor,
            &plan(vec![layer_plan(3, VideoBlendMode::Normal, 1.0)]),
            &[dxt1],
            2,
            2,
        );

        let mut dxt5_block = vec![128, 0];
        dxt5_block.extend_from_slice(&[0; 6]);
        dxt5_block.extend_from_slice(&0xf800u16.to_le_bytes());
        dxt5_block.extend_from_slice(&0x001fu16.to_le_bytes());
        dxt5_block.extend_from_slice(&0u32.to_le_bytes());
        let dxt5 = frame(4, 1, 1, VideoPixelFormat::Dxt5, dxt5_block);
        assert_gpu_matches_cpu(
            &compositor,
            &plan(vec![layer_plan(4, VideoBlendMode::Normal, 1.0)]),
            &[dxt5],
            1,
            1,
        );

        let mut hap_q_block = vec![100, 100, 0, 0, 0, 0, 0, 0];
        hap_q_block.extend_from_slice(&0x8400u16.to_le_bytes());
        hap_q_block.extend_from_slice(&0x8400u16.to_le_bytes());
        hap_q_block.extend_from_slice(&0u32.to_le_bytes());
        let hap_q = frame(5, 1, 1, VideoPixelFormat::YcoCgDxt5, hap_q_block);
        assert_gpu_matches_cpu_with_channel_tolerance(
            &compositor,
            &plan(vec![layer_plan(5, VideoBlendMode::Normal, 1.0)]),
            &[hap_q],
            1,
            1,
            1,
        );
    }

    #[test]
    fn gpu_compositor_matches_cpu_blend_modes_and_opacity() {
        let Some(compositor) = compositor() else {
            return;
        };
        let base = frame(1, 1, 1, VideoPixelFormat::Rgba8, vec![100, 80, 40, 255]);
        let top = frame(2, 1, 1, VideoPixelFormat::Rgba8, vec![80, 200, 240, 192]);

        for blend_mode in [
            VideoBlendMode::Normal,
            VideoBlendMode::Add,
            VideoBlendMode::Multiply,
            VideoBlendMode::Screen,
        ] {
            assert_gpu_matches_cpu(
                &compositor,
                &plan(vec![
                    layer_plan(1, VideoBlendMode::Normal, 1.0),
                    layer_plan(2, blend_mode, 0.65),
                ]),
                &[base.clone(), top.clone()],
                1,
                1,
            );
        }
    }

    #[test]
    fn gpu_compositor_matches_cpu_transform_crop_and_source_size() {
        let Some(compositor) = compositor() else {
            return;
        };
        let mut layer = layer_plan(1, VideoBlendMode::Normal, 0.8);
        layer.transform = Transform2D {
            x: 0.125,
            y: -0.125,
            scale_x: 0.75,
            scale_y: 0.5,
            rotation_deg: 90.0,
            crop_left: 0.25,
            crop_top: 0.0,
            crop_right: 0.0,
            crop_bottom: 0.25,
        };
        let mut data = Vec::new();
        for value in 0..12u8 {
            data.extend_from_slice(&[
                value.saturating_mul(17),
                255u8.saturating_sub(value.saturating_mul(13)),
                value.saturating_mul(7),
                64u8.saturating_add(value.saturating_mul(11)),
            ]);
        }
        assert_gpu_matches_cpu(
            &compositor,
            &plan(vec![layer]),
            &[frame(1, 4, 3, VideoPixelFormat::Rgba8, data)],
            8,
            6,
        );
    }

    #[test]
    fn gpu_compositor_matches_cpu_color_adjustments() {
        let Some(compositor) = compositor() else {
            return;
        };
        let mut layer = layer_plan(1, VideoBlendMode::Normal, 1.0);
        layer.color = VideoColorAdjust {
            brightness: 0.08,
            contrast: 1.15,
            hue_deg: 35.0,
            saturation: 0.75,
            gamma: 1.3,
        };
        assert_gpu_matches_cpu_with_channel_tolerance(
            &compositor,
            &plan(vec![layer]),
            &[frame(
                1,
                2,
                2,
                VideoPixelFormat::Rgba8,
                vec![
                    12, 67, 201, 255, 240, 80, 30, 200, 90, 180, 45, 128, 130, 140, 150, 64,
                ],
            )],
            2,
            2,
            1,
        );
    }

    #[test]
    fn gpu_compositor_matches_cpu_pixelate_and_blur() {
        let Some(compositor) = compositor() else {
            return;
        };
        let mut layer = layer_plan(1, VideoBlendMode::Normal, 1.0);
        layer.fx.pixelate = 2.0;
        layer.fx.blur = 1.0;
        let mut data = Vec::new();
        for value in 0..16u8 {
            data.extend_from_slice(&[
                value.saturating_mul(13),
                value.saturating_mul(7),
                255u8.saturating_sub(value.saturating_mul(11)),
                255,
            ]);
        }
        assert_gpu_matches_cpu(
            &compositor,
            &plan(vec![layer]),
            &[frame(1, 4, 4, VideoPixelFormat::Rgba8, data)],
            4,
            4,
        );
    }

    #[test]
    fn gpu_compositor_matches_cpu_glow_edge_and_color_key() {
        let Some(compositor) = compositor() else {
            return;
        };
        let mut layer = layer_plan(1, VideoBlendMode::Normal, 1.0);
        layer.fx = VideoFxAdjust {
            glow: 0.75,
            edge: 0.35,
            key_red: 0.0,
            key_green: 1.0,
            key_blue: 0.0,
            key_threshold: 0.4,
            ..VideoFxAdjust::default()
        };
        assert_gpu_matches_cpu_with_channel_tolerance(
            &compositor,
            &plan(vec![layer]),
            &[frame(
                1,
                3,
                3,
                VideoPixelFormat::Rgba8,
                vec![
                    0, 255, 0, 255, 240, 240, 240, 255, 10, 20, 30, 255, 20, 220, 15, 180, 255,
                    255, 255, 220, 100, 30, 200, 128, 0, 0, 0, 255, 180, 210, 230, 255, 40, 50, 60,
                    64,
                ],
            )],
            3,
            3,
            1,
        );
    }

    #[test]
    fn gpu_output_mapping_matches_cpu_aspect_modes() {
        let Some(compositor) = compositor() else {
            return;
        };
        let source = frame(
            9,
            8,
            6,
            VideoPixelFormat::Rgba8,
            (0..48u8)
                .flat_map(|value| {
                    [
                        value.saturating_mul(5),
                        value.saturating_mul(3),
                        255u8.saturating_sub(value.saturating_mul(4)),
                        255,
                    ]
                })
                .collect(),
        );
        for aspect_mode in [
            VideoOutputAspectMode::Stretch,
            VideoOutputAspectMode::Fit,
            VideoOutputAspectMode::Fill,
        ] {
            assert_gpu_mapping_matches_cpu(
                &compositor,
                &source,
                &VideoOutputMapping {
                    aspect_ratio: 16.0 / 9.0,
                    aspect_mode,
                    ..VideoOutputMapping::default()
                },
            );
        }
    }

    #[test]
    fn gpu_output_mapping_matches_cpu_projection_corrections() {
        let Some(compositor) = compositor() else {
            return;
        };
        let source = frame(
            10,
            12,
            8,
            VideoPixelFormat::Rgba8,
            (0..96u8)
                .flat_map(|value| {
                    [
                        value.saturating_mul(2),
                        255u8.saturating_sub(value.saturating_mul(2)),
                        value,
                        255,
                    ]
                })
                .collect(),
        );
        let mapping = VideoOutputMapping {
            offset_x: 0.04,
            offset_y: -0.03,
            scale_x: 0.92,
            scale_y: 0.88,
            rotation_deg: 4.0,
            aspect_ratio: 4.0 / 3.0,
            aspect_mode: VideoOutputAspectMode::Fit,
            lens_distortion: 0.12,
            keystone_x: 0.06,
            keystone_y: -0.04,
            corner_top_left_x: -0.02,
            corner_top_left_y: 0.01,
            corner_top_right_x: 0.03,
            corner_top_right_y: -0.02,
            corner_bottom_right_x: 0.02,
            corner_bottom_right_y: 0.03,
            corner_bottom_left_x: -0.01,
            corner_bottom_left_y: -0.03,
            ..VideoOutputMapping::default()
        };
        assert_gpu_mapping_matches_cpu(&compositor, &source, &mapping);
    }
}
