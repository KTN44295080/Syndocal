use std::sync::mpsc;

use protocol::{Transform2D, VideoBlendMode, VideoColorAdjust, VideoFxAdjust, VideoLayerId};
use wgpu::util::DeviceExt;

use crate::{
    convert_frame_to_rgba8, CompositionPlan, CpuCompositeError, VideoFrame, VideoPixelFormat,
};

const WORKGROUP_SIZE: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuCompositeError {
    Cpu(CpuCompositeError),
    Adapter(String),
    Device(String),
    UnsupportedLayerProcessing { layer_id: VideoLayerId },
    FrameDimensionsMismatch { layer_id: VideoLayerId },
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
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    adapter_name: String,
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
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Rayard video GPU compositor"),
            ..Default::default()
        }))
        .map_err(|error| GpuCompositeError::Device(error.to_string()))?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rayard video GPU compositor bind group layout"),
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
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rayard video GPU compositor pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rayard video GPU compositor shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_compositor.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Rayard video GPU compositor pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("composite_layer"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            bind_group_layout,
            pipeline,
            adapter_name,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
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
                label: Some("Rayard video GPU compositor output"),
                contents: &vec![0; buffer_size as usize],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        for layer in &plan.layers {
            if layer.transform != Transform2D::default()
                || layer.color != VideoColorAdjust::default()
                || layer.fx != VideoFxAdjust::default()
            {
                return Err(GpuCompositeError::UnsupportedLayerProcessing {
                    layer_id: layer.layer_id,
                });
            }
            let frame = frames
                .iter()
                .filter(|frame| frame.layer_id == layer.layer_id)
                .max_by_key(|frame| frame.pts_ms)
                .ok_or(CpuCompositeError::MissingFrame {
                    layer_id: layer.layer_id,
                })?;
            if frame.width != width || frame.height != height {
                return Err(GpuCompositeError::FrameDimensionsMismatch {
                    layer_id: layer.layer_id,
                });
            }
            let normalized = convert_frame_to_rgba8(frame)?;
            let source_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Rayard video GPU compositor source"),
                    contents: &normalized.data,
                    usage: wgpu::BufferUsages::STORAGE,
                });
            let params = compositor_params(
                pixel_count,
                layer.opacity.clamp(0.0, 1.0),
                blend_mode_index(&layer.blend_mode),
            );
            let params_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Rayard video GPU compositor params"),
                    contents: &params,
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Rayard video GPU compositor bind group"),
                layout: &self.bind_group_layout,
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
                ],
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Rayard video GPU compositor encoder"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Rayard video GPU compositor pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(pixel_count.div_ceil(WORKGROUP_SIZE), 1, 1);
            }
            self.queue.submit(Some(encoder.finish()));
        }

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rayard video GPU compositor readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Rayard video GPU compositor readback encoder"),
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
}

fn blend_mode_index(mode: &VideoBlendMode) -> u32 {
    match mode {
        VideoBlendMode::Normal => 0,
        VideoBlendMode::Add => 1,
        VideoBlendMode::Multiply => 2,
        VideoBlendMode::Screen => 3,
    }
}

fn compositor_params(pixel_count: u32, opacity: f32, blend_mode: u32) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0..4].copy_from_slice(&pixel_count.to_le_bytes());
    bytes[4..8].copy_from_slice(&opacity.to_bits().to_le_bytes());
    bytes[8..12].copy_from_slice(&blend_mode.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use protocol::{VideoSourceKind, VideoSourceSummary};

    use super::*;
    use crate::{composite_rgba8, CompositionLayerPlan};

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
    fn gpu_compositor_rejects_unimplemented_layer_processing() {
        let Some(compositor) = compositor() else {
            return;
        };
        let mut layer = layer_plan(1, VideoBlendMode::Normal, 1.0);
        layer.transform.x = 0.25;
        let error = compositor
            .composite_rgba8(
                &plan(vec![layer]),
                &[frame(1, 1, 1, VideoPixelFormat::Rgba8, vec![1, 2, 3, 255])],
                1,
                1,
            )
            .unwrap_err();

        assert_eq!(
            error,
            GpuCompositeError::UnsupportedLayerProcessing { layer_id: 1 }
        );
    }
}
