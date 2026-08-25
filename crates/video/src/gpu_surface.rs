use std::sync::Arc;

use protocol::VideoOutputMapping;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;

use crate::{convert_frame_to_rgba8, CpuCompositeError, VideoFrame};
use crate::{
    gpu_compositor::{
        blend_mode_index, compositor_params, create_frame_texture, create_gpu_composite_pipelines,
        dispatch_dimensions, gpu_frame_texture_spec, output_mapping_params,
        requested_video_device_features, write_frame_texture, write_mapping_mask_texture,
        CompositorParams, GpuCompositePipelines, GpuFrameTextureSpec,
    },
    PreparedVideoOutput, VideoOutputArtisticAdmission, VideoOutputArtisticPayload,
    VideoOutputArtisticRejection, VideoOutputArtisticRenderResult, VideoOutputPresentationContract,
};

const HARD_BLACKOUT_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSurfaceError {
    InvalidSize,
    Surface(String),
    Adapter(String),
    Device(String),
    Frame(CpuCompositeError),
    Present(String),
    /// The admission gate refused the artistic result before anything was
    /// submitted to the surface. No pixels were presented.
    Rejected(VideoOutputArtisticRejection),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuSurfaceBufferStats {
    pub output_capacity_bytes: u64,
    pub layer_slots: usize,
    pub output_reallocations: u64,
    pub layer_reallocations: u64,
    pub frames_presented: u64,
    pub hard_blackout_frames_presented: u64,
    pub compressed_layer_uploads: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GpuSubmissionValidationState {
    legacy_prepared_output_validated: bool,
    artistic_output_validated: bool,
}

struct NativeGpuLayerBuffers {
    source: wgpu::Buffer,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    texture_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    params: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl NativeGpuLayerBuffers {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        composition_buffer: &wgpu::Buffer,
        frame: &VideoFrame,
        spec: GpuFrameTextureSpec,
    ) -> Self {
        let source = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal native GPU unused layer source buffer"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let (texture, texture_view) =
            create_frame_texture(device, spec, "Syndocal native GPU layer source texture");
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal native GPU layer params"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = create_layer_bind_group(
            device,
            layout,
            &source,
            &texture_view,
            composition_buffer,
            &params,
        );
        Self {
            source,
            texture,
            texture_view,
            texture_format: spec.format,
            width: frame.width,
            height: frame.height,
            params,
            bind_group,
        }
    }

    fn rebind(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        composition_buffer: &wgpu::Buffer,
    ) {
        self.bind_group = create_layer_bind_group(
            device,
            layout,
            &self.source,
            &self.texture_view,
            composition_buffer,
            &self.params,
        );
    }

    fn matches(&self, frame: &VideoFrame, spec: GpuFrameTextureSpec) -> bool {
        self.width == frame.width
            && self.height == frame.height
            && self.texture_format == spec.format
    }
}

struct NativeGpuFrameBuffers {
    composition: wgpu::Buffer,
    mapped: wgpu::Buffer,
    output_capacity: u64,
    mapping_params: wgpu::Buffer,
    dimensions: wgpu::Buffer,
    mapping_bind_group: wgpu::BindGroup,
    output_bind_group: wgpu::BindGroup,
    mapping_dummy_texture: wgpu::Texture,
    layers: Vec<NativeGpuLayerBuffers>,
    output_reallocations: u64,
    layer_reallocations: u64,
    frames_presented: u64,
    compressed_layer_uploads: u64,
}

impl NativeGpuFrameBuffers {
    fn new(
        device: &wgpu::Device,
        composite_layout: &wgpu::BindGroupLayout,
        output_layout: &wgpu::BindGroupLayout,
        required_output_bytes: u64,
    ) -> Self {
        let output_capacity = grown_buffer_capacity(0, required_output_bytes);
        let composition = create_output_buffer(
            device,
            "Syndocal native GPU composition buffer",
            output_capacity,
        );
        let mapped = create_output_buffer(
            device,
            "Syndocal native GPU mapped output buffer",
            output_capacity,
        );
        let mapping_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal native GPU output mapping params"),
            size: 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dimensions = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal native GPU output dimensions"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mapping_dummy_texture = create_dummy_texture(device);
        let mapping_dummy_view =
            mapping_dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mapping_bind_group = create_mapping_bind_group(
            device,
            composite_layout,
            &composition,
            &mapped,
            &mapping_params,
            &mapping_dummy_view,
        );
        let output_bind_group =
            create_output_bind_group(device, output_layout, &mapped, &dimensions);
        Self {
            composition,
            mapped,
            output_capacity,
            mapping_params,
            dimensions,
            mapping_bind_group,
            output_bind_group,
            mapping_dummy_texture,
            layers: Vec::new(),
            output_reallocations: 1,
            layer_reallocations: 0,
            frames_presented: 0,
            compressed_layer_uploads: 0,
        }
    }

    fn ensure_output_capacity(
        &mut self,
        device: &wgpu::Device,
        composite_layout: &wgpu::BindGroupLayout,
        output_layout: &wgpu::BindGroupLayout,
        required_bytes: u64,
    ) {
        if required_bytes <= self.output_capacity {
            return;
        }
        self.output_capacity = grown_buffer_capacity(self.output_capacity, required_bytes);
        self.composition = create_output_buffer(
            device,
            "Syndocal native GPU composition buffer",
            self.output_capacity,
        );
        self.mapped = create_output_buffer(
            device,
            "Syndocal native GPU mapped output buffer",
            self.output_capacity,
        );
        self.mapping_bind_group = create_mapping_bind_group(
            device,
            composite_layout,
            &self.composition,
            &self.mapped,
            &self.mapping_params,
            &self
                .mapping_dummy_texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        self.output_bind_group =
            create_output_bind_group(device, output_layout, &self.mapped, &self.dimensions);
        for layer in &mut self.layers {
            layer.rebind(device, composite_layout, &self.composition);
        }
        self.output_reallocations = self.output_reallocations.saturating_add(1);
    }

    fn upload_layer_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        composite_layout: &wgpu::BindGroupLayout,
        index: usize,
        frame: &VideoFrame,
        spec: GpuFrameTextureSpec,
    ) {
        while self.layers.len() <= index {
            self.layers.push(NativeGpuLayerBuffers::new(
                device,
                composite_layout,
                &self.composition,
                frame,
                spec,
            ));
            self.layer_reallocations = self.layer_reallocations.saturating_add(1);
        }
        if !self.layers[index].matches(frame, spec) {
            self.layers[index] = NativeGpuLayerBuffers::new(
                device,
                composite_layout,
                &self.composition,
                frame,
                spec,
            );
            self.layer_reallocations = self.layer_reallocations.saturating_add(1);
        }
        write_frame_texture(queue, &self.layers[index].texture, frame, spec);
    }

    fn stats(&self) -> GpuSurfaceBufferStats {
        GpuSurfaceBufferStats {
            output_capacity_bytes: self.output_capacity,
            layer_slots: self.layers.len(),
            output_reallocations: self.output_reallocations,
            layer_reallocations: self.layer_reallocations,
            frames_presented: self.frames_presented,
            hard_blackout_frames_presented: 0,
            compressed_layer_uploads: self.compressed_layer_uploads,
        }
    }
}

fn create_output_buffer(device: &wgpu::Device, label: &'static str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_layer_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &wgpu::Buffer,
    source_texture: &wgpu::TextureView,
    composition: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Syndocal native GPU layer bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: source.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: composition.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(source_texture),
            },
        ],
    })
}

fn create_mapping_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    composition: &wgpu::Buffer,
    mapped: &wgpu::Buffer,
    params: &wgpu::Buffer,
    dummy_texture: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Syndocal native GPU output mapping bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: composition.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: mapped.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(dummy_texture),
            },
        ],
    })
}

fn create_dummy_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Syndocal native GPU unused mapping texture"),
        size: wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn create_output_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    mapped: &wgpu::Buffer,
    dimensions: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Syndocal native GPU buffer output bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: mapped.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: dimensions.as_entire_binding(),
            },
        ],
    })
}

fn grown_buffer_capacity(current: u64, required: u64) -> u64 {
    const ALIGNMENT: u64 = 256;
    let grown = if current == 0 {
        required
    } else {
        required.max(current.saturating_add(current / 2))
    };
    grown.max(4).div_ceil(ALIGNMENT) * ALIGNMENT
}

impl From<CpuCompositeError> for GpuSurfaceError {
    fn from(error: CpuCompositeError) -> Self {
        Self::Frame(error)
    }
}

pub struct GpuSurfacePresenter {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    composite_pipelines: GpuCompositePipelines,
    buffer_bind_group_layout: wgpu::BindGroupLayout,
    buffer_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    adapter_name: String,
    submission_validation: GpuSubmissionValidationState,
    hard_blackout_frames_presented: u64,
    gpu_buffers: Option<NativeGpuFrameBuffers>,
}

impl GpuSurfacePresenter {
    pub fn new<W>(window: Arc<W>, width: u32, height: u32) -> Result<Self, GpuSurfaceError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        if width == 0 || height == 0 {
            return Err(GpuSurfaceError::InvalidSize);
        }
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window)
            .map_err(|error| GpuSurfaceError::Surface(error.to_string()))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .map_err(|error| GpuSurfaceError::Adapter(error.to_string()))?;
        let adapter_name = adapter.get_info().name;
        let required_features = requested_video_device_features(&adapter);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal native video output device"),
            required_features,
            ..Default::default()
        }))
        .map_err(|error| GpuSurfaceError::Device(error.to_string()))?;
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| GpuSurfaceError::Surface("No supported surface format".to_string()))?;
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Syndocal native video output bind group layout"),
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
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Syndocal native video output pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Syndocal native video output shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_surface.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Syndocal native video output pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("fullscreen_vertex"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("frame_fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Syndocal native video output sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let composite_pipelines = create_gpu_composite_pipelines(&device);
        let buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Syndocal native GPU buffer output bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
        let buffer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Syndocal native GPU buffer output pipeline layout"),
                bind_group_layouts: &[&buffer_bind_group_layout],
                push_constant_ranges: &[],
            });
        let buffer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Syndocal native GPU buffer output shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_surface_buffer.wgsl").into()),
        });
        let buffer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Syndocal native GPU buffer output pipeline"),
            layout: Some(&buffer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &buffer_shader,
                entry_point: Some("fullscreen_vertex"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &buffer_shader,
                entry_point: Some("frame_fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Ok(Self {
            surface,
            device,
            queue,
            config,
            bind_group_layout,
            pipeline,
            composite_pipelines,
            buffer_bind_group_layout,
            buffer_pipeline,
            sampler,
            adapter_name,
            submission_validation: GpuSubmissionValidationState::default(),
            hard_blackout_frames_presented: 0,
            gpu_buffers: None,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn buffer_stats(&self) -> GpuSurfaceBufferStats {
        let mut stats = self
            .gpu_buffers
            .as_ref()
            .map(NativeGpuFrameBuffers::stats)
            .unwrap_or_default();
        stats.frames_presented = stats
            .frames_presented
            .saturating_add(self.hard_blackout_frames_presented);
        stats.hard_blackout_frames_presented = self.hard_blackout_frames_presented;
        stats
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), GpuSurfaceError> {
        if width == 0 || height == 0 {
            return Err(GpuSurfaceError::InvalidSize);
        }
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }

    pub fn present_rgba8(&mut self, frame: &VideoFrame) -> Result<(), GpuSurfaceError> {
        let frame = convert_frame_to_rgba8(frame)?;
        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some("Syndocal native video output frame"),
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &frame.data,
        );
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Syndocal native video output frame bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(|error| GpuSurfaceError::Present(error.to_string()))?
            }
            Err(error) => return Err(GpuSurfaceError::Present(error.to_string())),
        };
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal native video output encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Syndocal native video output pass"),
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    pub fn present_prepared_output(
        &mut self,
        prepared: &PreparedVideoOutput,
        width: u32,
        height: u32,
    ) -> Result<(), GpuSurfaceError> {
        if width == 0 || height == 0 {
            return Err(GpuSurfaceError::InvalidSize);
        }
        self.resize(width, height)?;
        let validate_submission = !self.submission_validation.legacy_prepared_output_validated;
        if validate_submission {
            self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        }
        let pixel_count = width
            .checked_mul(height)
            .ok_or(GpuSurfaceError::InvalidSize)?;
        let buffer_size = u64::from(pixel_count) * 4;
        let bc_supported = self
            .device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        let mut source_frames = Vec::with_capacity(prepared.plan.composition.layers.len());
        for layer in &prepared.plan.composition.layers {
            let frame = prepared
                .frames
                .iter()
                .filter(|frame| frame.layer_id == layer.layer_id)
                .max_by_key(|frame| frame.pts_ms)
                .ok_or(GpuSurfaceError::Frame(CpuCompositeError::MissingFrame {
                    layer_id: layer.layer_id,
                }))?;
            source_frames.push((frame, gpu_frame_texture_spec(frame, bc_supported)?));
        }
        if self.gpu_buffers.is_none() {
            self.gpu_buffers = Some(NativeGpuFrameBuffers::new(
                &self.device,
                &self.composite_pipelines.bind_group_layout,
                &self.buffer_bind_group_layout,
                buffer_size,
            ));
        }
        let surface_texture = self.acquire_surface_texture()?;
        let gpu_buffers = self.gpu_buffers.as_mut().expect("GPU buffers initialized");
        gpu_buffers.ensure_output_capacity(
            &self.device,
            &self.composite_pipelines.bind_group_layout,
            &self.buffer_bind_group_layout,
            buffer_size,
        );
        for (index, (frame, spec)) in source_frames.iter().enumerate() {
            gpu_buffers.upload_layer_frame(
                &self.device,
                &self.queue,
                &self.composite_pipelines.bind_group_layout,
                index,
                frame,
                *spec,
            );
        }

        let mapping_params = output_mapping_params(width, height, &prepared.plan.mapping);
        let dimensions = dimensions_uniform(width, height);
        write_mapping_mask_texture(
            &self.queue,
            &gpu_buffers.mapping_dummy_texture,
            &prepared.plan.mapping,
        );
        self.queue
            .write_buffer(&gpu_buffers.mapping_params, 0, &mapping_params);
        self.queue
            .write_buffer(&gpu_buffers.dimensions, 0, &dimensions);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal native GPU output encoder"),
            });
        encoder.clear_buffer(&gpu_buffers.composition, 0, Some(buffer_size));
        encoder.clear_buffer(&gpu_buffers.mapped, 0, Some(buffer_size));

        for (index, (layer, (frame, spec))) in prepared
            .plan
            .composition
            .layers
            .iter()
            .zip(source_frames.iter())
            .enumerate()
        {
            let params = compositor_params(CompositorParams {
                output_width: width,
                output_height: height,
                source_width: frame.width,
                source_height: frame.height,
                opacity: layer.opacity.clamp(0.0, 1.0),
                blend_mode: blend_mode_index(&layer.blend_mode),
                source_format: spec.source_format,
                transform: &layer.transform,
                color: &layer.color,
                fx: &layer.fx,
            });
            let layer_buffers = &gpu_buffers.layers[index];
            self.queue.write_buffer(&layer_buffers.params, 0, &params);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal native GPU layer pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.composite_pipelines.composite);
            pass.set_bind_group(0, &layer_buffers.bind_group, &[]);
            let (workgroups_x, workgroups_y) = dispatch_dimensions(width, height);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal native GPU output mapping pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.composite_pipelines.output_mapping);
            pass.set_bind_group(0, &gpu_buffers.mapping_bind_group, &[]);
            let (workgroups_x, workgroups_y) = dispatch_dimensions(width, height);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Syndocal native GPU buffer output pass"),
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
            pass.set_pipeline(&self.buffer_pipeline);
            pass.set_bind_group(0, &gpu_buffers.output_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        if validate_submission {
            if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
                return Err(GpuSurfaceError::Present(format!(
                    "GPU buffer presentation validation failed: {error}"
                )));
            }
            self.submission_validation.legacy_prepared_output_validated = true;
        }
        surface_texture.present();
        gpu_buffers.frames_presented = gpu_buffers.frames_presented.saturating_add(1);
        gpu_buffers.compressed_layer_uploads = gpu_buffers.compressed_layer_uploads.saturating_add(
            source_frames
                .iter()
                .filter(|(frame, _)| {
                    matches!(
                        frame.format,
                        crate::VideoPixelFormat::Dxt1
                            | crate::VideoPixelFormat::Dxt5
                            | crate::VideoPixelFormat::YcoCgDxt5
                    )
                })
                .count() as u64,
        );
        Ok(())
    }

    fn acquire_surface_texture(&self) -> Result<wgpu::SurfaceTexture, GpuSurfaceError> {
        match self.surface.get_current_texture() {
            Ok(texture) => Ok(texture),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(|error| GpuSurfaceError::Present(error.to_string()))
            }
            Err(error) => Err(GpuSurfaceError::Present(error.to_string())),
        }
    }

    /// Presents one canonical [`VideoOutputArtisticRenderResult`] produced by
    /// the shared C1/C3 artistic renderer. This is the production seam for
    /// native Display output: composition already happened in the video core,
    /// so this method never recomposites layers and applies the existing WGSL
    /// output mapping exactly once.
    ///
    /// Admission is fail-closed via [`VideoOutputPresentationContract::admit`]:
    /// stale generations, foreign outputs, wrong sizes, partial payloads, and
    /// non-fresh (LastValid/Error) evidence are rejected before the surface is
    /// touched. A hard blackout bypasses the mapping entirely and clears the
    /// swapchain image to byte-exact opaque black (`[0, 0, 0, 255]`), because
    /// mapping values such as `black_level` would lift even a black frame.
    ///
    /// Steady state performs no allocation beyond first-call GPU buffer
    /// growth: the artistic frame bytes are written into the existing
    /// composition storage buffer and mapping uniform. The frame data remains
    /// borrowed; ownership stays with the caller.
    pub fn present_output_artistic_result(
        &mut self,
        result: &VideoOutputArtisticRenderResult,
        contract: &VideoOutputPresentationContract,
    ) -> Result<(), GpuSurfaceError> {
        let presentation = resolve_output_artistic_presentation(result, contract)?;
        self.resize(contract.width, contract.height)?;
        match presentation {
            GpuArtisticPresentation::ExactBlackClear => self.present_hard_blackout_surface(),
            GpuArtisticPresentation::MapOnce { frame, mapping } => {
                self.present_mapped_artistic_buffer(frame, mapping, contract.width, contract.height)
            }
        }
    }

    fn present_hard_blackout_surface(&mut self) -> Result<(), GpuSurfaceError> {
        // Absolute blackout bypass: no texture upload, no mapping pass, no
        // stored content. Clearing the swapchain image keeps presented bytes
        // exact black regardless of mapping state or stale caches.
        let surface_texture = self.acquire_surface_texture()?;
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal native GPU hard blackout encoder"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Syndocal native GPU hard blackout pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(HARD_BLACKOUT_CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        self.hard_blackout_frames_presented = self.hard_blackout_frames_presented.saturating_add(1);
        Ok(())
    }

    fn present_mapped_artistic_buffer(
        &mut self,
        frame: &VideoFrame,
        mapping: &VideoOutputMapping,
        width: u32,
        height: u32,
    ) -> Result<(), GpuSurfaceError> {
        let validate_submission = !self.submission_validation.artistic_output_validated;
        if validate_submission {
            self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        }
        let buffer_size =
            u64::try_from(frame.data.len()).map_err(|_| GpuSurfaceError::InvalidSize)?;
        if self.gpu_buffers.is_none() {
            self.gpu_buffers = Some(NativeGpuFrameBuffers::new(
                &self.device,
                &self.composite_pipelines.bind_group_layout,
                &self.buffer_bind_group_layout,
                buffer_size,
            ));
        }
        let surface_texture = self.acquire_surface_texture()?;
        let gpu_buffers = self.gpu_buffers.as_mut().expect("GPU buffers initialized");
        gpu_buffers.ensure_output_capacity(
            &self.device,
            &self.composite_pipelines.bind_group_layout,
            &self.buffer_bind_group_layout,
            buffer_size,
        );
        // The artistic frame is already composed by the video core: one write
        // replaces the whole per-layer composite stage and the single mapping
        // dispatch below is its only consumer.
        self.queue
            .write_buffer(&gpu_buffers.composition, 0, &frame.data);
        let mapping_params = output_mapping_params(width, height, mapping);
        let dimensions = dimensions_uniform(width, height);
        write_mapping_mask_texture(&self.queue, &gpu_buffers.mapping_dummy_texture, mapping);
        self.queue
            .write_buffer(&gpu_buffers.mapping_params, 0, &mapping_params);
        self.queue
            .write_buffer(&gpu_buffers.dimensions, 0, &dimensions);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal native GPU artistic output encoder"),
            });
        encoder.clear_buffer(&gpu_buffers.mapped, 0, Some(buffer_size));
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal native GPU artistic output mapping pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.composite_pipelines.output_mapping);
            pass.set_bind_group(0, &gpu_buffers.mapping_bind_group, &[]);
            let (workgroups_x, workgroups_y) = dispatch_dimensions(width, height);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Syndocal native GPU artistic buffer output pass"),
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
            pass.set_pipeline(&self.buffer_pipeline);
            pass.set_bind_group(0, &gpu_buffers.output_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        if validate_submission {
            if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
                return Err(GpuSurfaceError::Present(format!(
                    "GPU artistic presentation validation failed: {error}"
                )));
            }
            self.submission_validation.artistic_output_validated = true;
        }
        surface_texture.present();
        let gpu_buffers = self.gpu_buffers.as_mut().expect("GPU buffers initialized");
        gpu_buffers.frames_presented = gpu_buffers.frames_presented.saturating_add(1);
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
enum GpuArtisticPresentation<'a> {
    ExactBlackClear,
    MapOnce {
        frame: &'a VideoFrame,
        mapping: &'a VideoOutputMapping,
    },
}

impl GpuArtisticPresentation<'_> {
    #[cfg(test)]
    fn mapping_dispatch_count(&self) -> u8 {
        match self {
            Self::ExactBlackClear => 0,
            Self::MapOnce { .. } => 1,
        }
    }

    #[cfg(test)]
    fn has_pixel_payload(&self) -> bool {
        matches!(self, Self::MapOnce { .. })
    }
}

fn resolve_output_artistic_presentation<'a>(
    result: &'a VideoOutputArtisticRenderResult,
    contract: &VideoOutputPresentationContract,
) -> Result<GpuArtisticPresentation<'a>, GpuSurfaceError> {
    match contract.admit(result).map_err(GpuSurfaceError::Rejected)? {
        VideoOutputArtisticAdmission::PresentHardBlackout => {
            Ok(GpuArtisticPresentation::ExactBlackClear)
        }
        VideoOutputArtisticAdmission::MapOnceAndPresent => match &result.payload {
            VideoOutputArtisticPayload::Frame(frame) => Ok(GpuArtisticPresentation::MapOnce {
                frame,
                mapping: &result.output_mapping,
            }),
            VideoOutputArtisticPayload::HardBlackout => Err(GpuSurfaceError::Rejected(
                VideoOutputArtisticRejection::AdmissionPayloadMismatch,
            )),
        },
    }
}

fn dimensions_uniform(width: u32, height: u32) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0..4].copy_from_slice(&width.to_le_bytes());
    bytes[4..8].copy_from_slice(&height.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{apply_video_output_mapping, video_output_black_frame};

    fn test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        let required_features = requested_video_device_features(&adapter);
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal reusable GPU buffer test"),
            required_features,
            ..Default::default()
        }))
        .ok()
    }

    fn test_compositor() -> Option<crate::GpuCompositor> {
        match crate::GpuCompositor::new() {
            Ok(compositor) => Some(compositor),
            Err(error) => {
                eprintln!("GPU compositor test skipped: {error:?}");
                None
            }
        }
    }

    fn output_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Syndocal reusable GPU buffer test output layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    #[test]
    fn buffer_capacity_is_aligned_and_grows_by_at_least_half() {
        assert_eq!(grown_buffer_capacity(0, 1), 256);
        assert_eq!(grown_buffer_capacity(0, 1024), 1024);
        assert_eq!(grown_buffer_capacity(1024, 1025), 1536);
        assert_eq!(grown_buffer_capacity(1024, 4096), 4096);
    }

    #[test]
    fn frame_buffers_reuse_output_and_layer_allocations_until_capacity_is_exceeded() {
        let Some((device, queue)) = test_device() else {
            eprintln!("Reusable GPU buffer test skipped: no adapter");
            return;
        };
        let pipelines = create_gpu_composite_pipelines(&device);
        let output_layout = output_layout(&device);
        let mut buffers =
            NativeGpuFrameBuffers::new(&device, &pipelines.bind_group_layout, &output_layout, 1024);
        assert_eq!(buffers.stats().output_reallocations, 1);

        buffers.ensure_output_capacity(&device, &pipelines.bind_group_layout, &output_layout, 768);
        assert_eq!(buffers.stats().output_reallocations, 1);
        buffers.ensure_output_capacity(&device, &pipelines.bind_group_layout, &output_layout, 1025);
        assert_eq!(buffers.stats().output_reallocations, 2);
        assert_eq!(buffers.stats().output_capacity_bytes, 1536);

        let frame = |width, height| VideoFrame {
            layer_id: 1,
            width,
            height,
            pts_ms: 0,
            duration_ms: 16,
            format: crate::VideoPixelFormat::Rgba8,
            data: vec![0; width as usize * height as usize * 4],
        };
        let first = frame(16, 16);
        let first_spec = gpu_frame_texture_spec(&first, false).unwrap();
        buffers.upload_layer_frame(
            &device,
            &queue,
            &pipelines.bind_group_layout,
            0,
            &first,
            first_spec,
        );
        assert_eq!(buffers.stats().layer_reallocations, 1);
        buffers.upload_layer_frame(
            &device,
            &queue,
            &pipelines.bind_group_layout,
            0,
            &first,
            first_spec,
        );
        assert_eq!(buffers.stats().layer_reallocations, 1);
        let larger = frame(32, 16);
        let larger_spec = gpu_frame_texture_spec(&larger, false).unwrap();
        buffers.upload_layer_frame(
            &device,
            &queue,
            &pipelines.bind_group_layout,
            0,
            &larger,
            larger_spec,
        );
        assert_eq!(buffers.stats().layer_reallocations, 2);
        buffers.upload_layer_frame(
            &device,
            &queue,
            &pipelines.bind_group_layout,
            1,
            &first,
            first_spec,
        );
        assert_eq!(buffers.stats().layer_slots, 2);
        assert_eq!(buffers.stats().layer_reallocations, 3);
    }

    fn artistic_rgba_frame(width: u32, height: u32) -> VideoFrame {
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                data.extend_from_slice(&[(x * 37 % 256) as u8, (y * 53 % 256) as u8, 200, 255]);
            }
        }
        VideoFrame {
            layer_id: 0,
            width,
            height,
            pts_ms: 0,
            duration_ms: 16,
            format: crate::VideoPixelFormat::Rgba8,
            data,
        }
    }

    fn artistic_result(
        payload: crate::VideoOutputArtisticPayload,
        output_mapping: VideoOutputMapping,
        freshness: crate::VideoOutputRenderFreshness,
    ) -> crate::VideoOutputArtisticRenderResult {
        crate::VideoOutputArtisticRenderResult {
            payload,
            output_mapping,
            output_mapping_identity: crate::VideoOutputMappingIdentity::from_mapping(
                &output_mapping,
            ),
            evidence: crate::VideoOutputRenderEvidence {
                project_render_epoch: 7,
                output_id: 9,
                freshness,
                error: None,
            },
        }
    }

    fn presentation_contract(width: u32, height: u32) -> VideoOutputPresentationContract {
        VideoOutputPresentationContract::new(7, 9, width, height, &VideoOutputMapping::default())
    }

    fn presentation_contract_with_mapping(
        width: u32,
        height: u32,
        mapping: &VideoOutputMapping,
    ) -> VideoOutputPresentationContract {
        VideoOutputPresentationContract::new(7, 9, width, height, mapping)
    }

    #[test]
    fn legacy_and_artistic_first_submission_validation_are_independent() {
        let mut state = GpuSubmissionValidationState::default();
        assert!(!state.legacy_prepared_output_validated);
        assert!(!state.artistic_output_validated);

        state.legacy_prepared_output_validated = true;
        assert!(state.legacy_prepared_output_validated);
        assert!(
            !state.artistic_output_validated,
            "legacy validation cannot waive the artistic path's first submission validation"
        );

        state.artistic_output_validated = true;
        assert!(state.legacy_prepared_output_validated);
        assert!(state.artistic_output_validated);
    }

    #[test]
    fn artistic_admission_resolves_blackout_bypass_or_single_mapping_fail_closed() {
        use crate::{VideoOutputArtisticPayload, VideoOutputRenderFreshness};

        let mapped = artistic_result(
            VideoOutputArtisticPayload::Frame(artistic_rgba_frame(2, 1)),
            VideoOutputMapping::default(),
            VideoOutputRenderFreshness::Fresh,
        );
        let mapped_plan =
            resolve_output_artistic_presentation(&mapped, &presentation_contract(2, 1)).unwrap();
        assert_eq!(mapped_plan.mapping_dispatch_count(), 1);
        assert!(mapped_plan.has_pixel_payload());

        let mut hostile_mapping = VideoOutputMapping::default();
        hostile_mapping.black_level = 0.9;
        let blackout = artistic_result(
            VideoOutputArtisticPayload::HardBlackout,
            hostile_mapping,
            VideoOutputRenderFreshness::Fresh,
        );
        // `black_level = 0.9` is bound into both sides of the contract, but a
        // hard blackout still has no pixel payload and schedules no mapping
        // dispatch. The only production arm is the exact-black render-pass clear.
        let blackout_plan = resolve_output_artistic_presentation(
            &blackout,
            &presentation_contract_with_mapping(2, 1, &hostile_mapping),
        )
        .unwrap();
        assert_eq!(blackout_plan.mapping_dispatch_count(), 0);
        assert!(!blackout_plan.has_pixel_payload());
        assert!(matches!(
            blackout_plan,
            GpuArtisticPresentation::ExactBlackClear
        ));
        assert_eq!(HARD_BLACKOUT_CLEAR_COLOR.r, 0.0);
        assert_eq!(HARD_BLACKOUT_CLEAR_COLOR.g, 0.0);
        assert_eq!(HARD_BLACKOUT_CLEAR_COLOR.b, 0.0);
        assert_eq!(HARD_BLACKOUT_CLEAR_COLOR.a, 1.0);

        assert_eq!(
            resolve_output_artistic_presentation(&mapped, &presentation_contract(0, 1)),
            Err(GpuSurfaceError::Rejected(
                VideoOutputArtisticRejection::ZeroAreaContract {
                    width: 0,
                    height: 1,
                }
            ))
        );

        assert_eq!(
            resolve_output_artistic_presentation(
                &blackout,
                &presentation_contract_with_mapping(u32::MAX, u32::MAX, &hostile_mapping),
            ),
            Err(GpuSurfaceError::Rejected(
                VideoOutputArtisticRejection::FrameByteLengthOverflow {
                    width: u32::MAX,
                    height: u32::MAX,
                }
            ))
        );

        let mut substituted_mapping = mapped.clone();
        substituted_mapping.output_mapping.black_level = 0.75;
        assert!(matches!(
            resolve_output_artistic_presentation(
                &substituted_mapping,
                &presentation_contract(2, 1),
            ),
            Err(GpuSurfaceError::Rejected(
                VideoOutputArtisticRejection::ResultMappingIdentityMismatch { .. }
            ))
        ));

        let mut foreign_mapping = mapped.clone();
        foreign_mapping.output_mapping.black_level = 0.75;
        foreign_mapping.output_mapping_identity =
            crate::VideoOutputMappingIdentity::from_mapping(&foreign_mapping.output_mapping);
        assert!(matches!(
            resolve_output_artistic_presentation(&foreign_mapping, &presentation_contract(2, 1)),
            Err(GpuSurfaceError::Rejected(
                VideoOutputArtisticRejection::ContractMappingIdentityMismatch { .. }
            ))
        ));

        let mut stale = mapped.clone();
        stale.evidence.project_render_epoch = 6;
        assert!(matches!(
            resolve_output_artistic_presentation(&stale, &presentation_contract(2, 1)),
            Err(GpuSurfaceError::Rejected(_))
        ));

        let mut foreign = mapped.clone();
        foreign.evidence.output_id = 8;
        assert!(matches!(
            resolve_output_artistic_presentation(&foreign, &presentation_contract(2, 1)),
            Err(GpuSurfaceError::Rejected(_))
        ));

        let mut wrong_size = mapped.clone();
        let VideoOutputArtisticPayload::Frame(frame) = &mut wrong_size.payload else {
            panic!("mapped result contains a frame")
        };
        frame.width = 4;
        assert!(matches!(
            resolve_output_artistic_presentation(&wrong_size, &presentation_contract(2, 1)),
            Err(GpuSurfaceError::Rejected(_))
        ));

        let mut partial = mapped;
        let VideoOutputArtisticPayload::Frame(frame) = &mut partial.payload else {
            panic!("mapped result contains a frame")
        };
        frame.data.pop();
        assert!(matches!(
            resolve_output_artistic_presentation(&partial, &presentation_contract(2, 1)),
            Err(GpuSurfaceError::Rejected(_))
        ));

        let mut rolled_back = blackout;
        rolled_back.evidence.freshness = VideoOutputRenderFreshness::LastValid;
        assert!(matches!(
            resolve_output_artistic_presentation(
                &rolled_back,
                &presentation_contract_with_mapping(2, 1, &hostile_mapping),
            ),
            Err(GpuSurfaceError::Rejected(_))
        ));
    }

    #[test]
    fn wgsl_output_mapping_applies_to_an_artistic_frame_exactly_once() {
        use protocol::VideoOutputMapping;

        let Some(compositor) = test_compositor() else {
            return;
        };
        let frame = artistic_rgba_frame(8, 8);
        let mut mapping = VideoOutputMapping::default();
        mapping.offset_x = 0.125;
        mapping.scale_x = 1.25;
        mapping.rotation_deg = 8.0;
        mapping.black_level = 0.15;
        mapping.edge_blend_left = 0.25;
        mapping.edge_blend_gamma = 1.7;

        let mapped_once_gpu = compositor
            .apply_output_mapping_rgba8(&frame, &mapping)
            .unwrap();
        let mapped_once_cpu = apply_video_output_mapping(frame.clone(), &mapping);
        let mapped_twice_cpu = apply_video_output_mapping(mapped_once_cpu.clone(), &mapping);
        assert_eq!(
            mapped_once_gpu.data, mapped_once_cpu.data,
            "the WGSL mapping pass must reproduce the CPU mapping exactly once"
        );
        assert_ne!(
            mapped_once_cpu.data, mapped_twice_cpu.data,
            "the test mapping must detect a double application"
        );
    }

    #[test]
    fn wgsl_mapping_would_lift_a_hard_blackout_so_the_gate_forces_the_bypass() {
        use protocol::VideoOutputMapping;

        let black = video_output_black_frame(4, 4);
        let mut hostile = VideoOutputMapping::default();
        hostile.black_level = 0.9;
        hostile.edge_blend_top = 1.0;

        let lifted_cpu = apply_video_output_mapping(black.clone(), &hostile);
        assert_ne!(
            lifted_cpu.data, black.data,
            "black_level lifts even contract-black pixels on the CPU path"
        );

        let Some(compositor) = test_compositor() else {
            return;
        };
        let lifted_gpu = compositor
            .apply_output_mapping_rgba8(&black, &hostile)
            .unwrap();
        assert_ne!(
            lifted_gpu.data, black.data,
            "the WGSL mapping pass would lift a blackout frame too"
        );
    }
}
