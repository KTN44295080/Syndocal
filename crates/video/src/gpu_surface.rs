use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;

use crate::{convert_frame_to_rgba8, CpuCompositeError, VideoFrame};
use crate::{
    gpu_compositor::{
        blend_mode_index, compositor_params, create_gpu_composite_pipelines, dispatch_dimensions,
        output_mapping_params, GpuCompositePipelines,
    },
    PreparedVideoOutput,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSurfaceError {
    InvalidSize,
    Surface(String),
    Adapter(String),
    Device(String),
    Frame(CpuCompositeError),
    Present(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuSurfaceBufferStats {
    pub output_capacity_bytes: u64,
    pub layer_slots: usize,
    pub output_reallocations: u64,
    pub layer_reallocations: u64,
    pub frames_presented: u64,
}

struct NativeGpuLayerBuffers {
    source: wgpu::Buffer,
    source_capacity: u64,
    params: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl NativeGpuLayerBuffers {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        composition_buffer: &wgpu::Buffer,
        source_capacity: u64,
    ) -> Self {
        let source = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rayard native GPU layer source"),
            size: source_capacity,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rayard native GPU layer params"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group =
            create_layer_bind_group(device, layout, &source, composition_buffer, &params);
        Self {
            source,
            source_capacity,
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
            composition_buffer,
            &self.params,
        );
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
    layers: Vec<NativeGpuLayerBuffers>,
    output_reallocations: u64,
    layer_reallocations: u64,
    frames_presented: u64,
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
            "Rayard native GPU composition buffer",
            output_capacity,
        );
        let mapped = create_output_buffer(
            device,
            "Rayard native GPU mapped output buffer",
            output_capacity,
        );
        let mapping_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rayard native GPU output mapping params"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dimensions = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rayard native GPU output dimensions"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mapping_bind_group = create_mapping_bind_group(
            device,
            composite_layout,
            &composition,
            &mapped,
            &mapping_params,
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
            layers: Vec::new(),
            output_reallocations: 1,
            layer_reallocations: 0,
            frames_presented: 0,
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
            "Rayard native GPU composition buffer",
            self.output_capacity,
        );
        self.mapped = create_output_buffer(
            device,
            "Rayard native GPU mapped output buffer",
            self.output_capacity,
        );
        self.mapping_bind_group = create_mapping_bind_group(
            device,
            composite_layout,
            &self.composition,
            &self.mapped,
            &self.mapping_params,
        );
        self.output_bind_group =
            create_output_bind_group(device, output_layout, &self.mapped, &self.dimensions);
        for layer in &mut self.layers {
            layer.rebind(device, composite_layout, &self.composition);
        }
        self.output_reallocations = self.output_reallocations.saturating_add(1);
    }

    fn ensure_layer_capacity(
        &mut self,
        device: &wgpu::Device,
        composite_layout: &wgpu::BindGroupLayout,
        index: usize,
        required_bytes: u64,
    ) {
        while self.layers.len() <= index {
            let source_capacity = grown_buffer_capacity(0, required_bytes);
            self.layers.push(NativeGpuLayerBuffers::new(
                device,
                composite_layout,
                &self.composition,
                source_capacity,
            ));
            self.layer_reallocations = self.layer_reallocations.saturating_add(1);
        }
        if required_bytes > self.layers[index].source_capacity {
            let source_capacity =
                grown_buffer_capacity(self.layers[index].source_capacity, required_bytes);
            self.layers[index] = NativeGpuLayerBuffers::new(
                device,
                composite_layout,
                &self.composition,
                source_capacity,
            );
            self.layer_reallocations = self.layer_reallocations.saturating_add(1);
        }
    }

    fn stats(&self) -> GpuSurfaceBufferStats {
        GpuSurfaceBufferStats {
            output_capacity_bytes: self.output_capacity,
            layer_slots: self.layers.len(),
            output_reallocations: self.output_reallocations,
            layer_reallocations: self.layer_reallocations,
            frames_presented: self.frames_presented,
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
    composition: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rayard native GPU layer bind group"),
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
        ],
    })
}

fn create_mapping_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    composition: &wgpu::Buffer,
    mapped: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rayard native GPU output mapping bind group"),
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
        ],
    })
}

fn create_output_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    mapped: &wgpu::Buffer,
    dimensions: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rayard native GPU buffer output bind group"),
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
    buffer_path_validated: bool,
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
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Rayard native video output device"),
            ..Default::default()
        }))
        .map_err(|error| GpuSurfaceError::Device(error.to_string()))?;
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| GpuSurfaceError::Surface("No supported surface format".to_string()))?;
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rayard native video output bind group layout"),
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
            label: Some("Rayard native video output pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rayard native video output shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_surface.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rayard native video output pipeline"),
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
            label: Some("Rayard native video output sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let composite_pipelines = create_gpu_composite_pipelines(&device);
        let buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Rayard native GPU buffer output bind group layout"),
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
                label: Some("Rayard native GPU buffer output pipeline layout"),
                bind_group_layouts: &[&buffer_bind_group_layout],
                push_constant_ranges: &[],
            });
        let buffer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rayard native GPU buffer output shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_surface_buffer.wgsl").into()),
        });
        let buffer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rayard native GPU buffer output pipeline"),
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
            buffer_path_validated: false,
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
        self.gpu_buffers
            .as_ref()
            .map(NativeGpuFrameBuffers::stats)
            .unwrap_or_default()
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
                label: Some("Rayard native video output frame"),
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
            label: Some("Rayard native video output frame bind group"),
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
                label: Some("Rayard native video output encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rayard native video output pass"),
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
        let validate_submission = !self.buffer_path_validated;
        if validate_submission {
            self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        }
        let pixel_count = width
            .checked_mul(height)
            .ok_or(GpuSurfaceError::InvalidSize)?;
        let buffer_size = u64::from(pixel_count) * 4;
        let mut normalized_frames = Vec::with_capacity(prepared.plan.composition.layers.len());
        for layer in &prepared.plan.composition.layers {
            let frame = prepared
                .frames
                .iter()
                .filter(|frame| frame.layer_id == layer.layer_id)
                .max_by_key(|frame| frame.pts_ms)
                .ok_or_else(|| {
                    GpuSurfaceError::Frame(CpuCompositeError::MissingFrame {
                        layer_id: layer.layer_id,
                    })
                })?;
            normalized_frames.push(convert_frame_to_rgba8(frame)?);
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
        for (index, frame) in normalized_frames.iter().enumerate() {
            gpu_buffers.ensure_layer_capacity(
                &self.device,
                &self.composite_pipelines.bind_group_layout,
                index,
                frame.data.len() as u64,
            );
        }

        let mapping_params = output_mapping_params(width, height, &prepared.plan.mapping);
        let dimensions = dimensions_uniform(width, height);
        self.queue
            .write_buffer(&gpu_buffers.mapping_params, 0, &mapping_params);
        self.queue
            .write_buffer(&gpu_buffers.dimensions, 0, &dimensions);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Rayard native GPU output encoder"),
            });
        encoder.clear_buffer(&gpu_buffers.composition, 0, Some(buffer_size));
        encoder.clear_buffer(&gpu_buffers.mapped, 0, Some(buffer_size));

        for (index, (layer, normalized)) in prepared
            .plan
            .composition
            .layers
            .iter()
            .zip(normalized_frames.iter())
            .enumerate()
        {
            let params = compositor_params(
                width,
                height,
                normalized.width,
                normalized.height,
                layer.opacity.clamp(0.0, 1.0),
                blend_mode_index(&layer.blend_mode),
                &layer.transform,
                &layer.color,
                &layer.fx,
            );
            let layer_buffers = &gpu_buffers.layers[index];
            self.queue
                .write_buffer(&layer_buffers.source, 0, &normalized.data);
            self.queue.write_buffer(&layer_buffers.params, 0, &params);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Rayard native GPU layer pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.composite_pipelines.composite);
            pass.set_bind_group(0, &layer_buffers.bind_group, &[]);
            let (workgroups_x, workgroups_y) = dispatch_dimensions(width, height);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Rayard native GPU output mapping pass"),
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
                label: Some("Rayard native GPU buffer output pass"),
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
            self.buffer_path_validated = true;
        }
        surface_texture.present();
        gpu_buffers.frames_presented = gpu_buffers.frames_presented.saturating_add(1);
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

    fn test_device() -> Option<wgpu::Device> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Rayard reusable GPU buffer test"),
            ..Default::default()
        }))
        .ok()
        .map(|(device, _)| device)
    }

    fn output_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rayard reusable GPU buffer test output layout"),
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
        let Some(device) = test_device() else {
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

        buffers.ensure_layer_capacity(&device, &pipelines.bind_group_layout, 0, 1000);
        assert_eq!(buffers.stats().layer_reallocations, 1);
        buffers.ensure_layer_capacity(&device, &pipelines.bind_group_layout, 0, 900);
        assert_eq!(buffers.stats().layer_reallocations, 1);
        buffers.ensure_layer_capacity(&device, &pipelines.bind_group_layout, 0, 1025);
        assert_eq!(buffers.stats().layer_reallocations, 2);
        buffers.ensure_layer_capacity(&device, &pipelines.bind_group_layout, 1, 512);
        assert_eq!(buffers.stats().layer_slots, 2);
        assert_eq!(buffers.stats().layer_reallocations, 3);
    }
}
