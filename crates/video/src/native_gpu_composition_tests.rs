//! Pure planning/fake-port tests plus a separately labelled real-adapter oracle.
use super::*;
use crate::native_gpu_contract::{FrozenChainClocks, LayerOrigin};
use crate::{CompositionPlan, VideoFrame};
use protocol::{
    Transform2D, VideoBlendMode, VideoColorAdjust, VideoFxAdjust, VideoRenderInputId,
    VideoRenderInputKey, VideoSourceKind, VideoSourceSummary,
};
use std::sync::{mpsc, Arc};
use wgpu::util::DeviceExt;

fn context() -> GpuContextId {
    GpuContextId {
        owner_id: 7,
        device_generation: 3,
    }
}
fn frame() -> NativeFrameId {
    NativeFrameId {
        output_id: 1,
        project_epoch: 2,
        request_sequence: 9,
        geometry_generation: 4,
    }
}
pub(crate) fn identity(id: u64, extent: (u32, u32)) -> LayerResultIdentity {
    LayerResultIdentity {
        context: context(),
        frame: frame(),
        entry: HistoryEntryId {
            input: VideoRenderInputKey {
                project_render_epoch: 2,
                input_id: VideoRenderInputId(id),
            },
            queue_epoch: 5,
            sequence: 11,
        },
        layer_id: id as _,
        source_generation: 8,
        content_sequence: 21,
        pts_ms: 33,
        duration_ms: 33,
        width: extent.0,
        height: extent.1,
        format: VideoPixelFormat::Rgba8,
        origin: LayerOrigin::Executed(FrozenChainClocks {
            frame_index: 3,
            time_seconds_bits: 0.033f32.to_bits(),
            time_delta_seconds_bits: 0.033f32.to_bits(),
            date_bits: [0; 4],
        }),
    }
}
pub(crate) fn layer(
    id: u64,
    mode: VideoBlendMode,
    opacity: f32,
    transform: Transform2D,
) -> CompositionLayerPlan {
    CompositionLayerPlan {
        layer_id: id as _,
        label: format!("layer {id}"),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("fixture.mov".into()),
            name: None,
            codec: None,
            metadata: None,
        },
        blend_mode: mode,
        opacity,
        position_ms: 0,
        transform,
        color: VideoColorAdjust::default(),
        fx: VideoFxAdjust::default(),
    }
}
pub(crate) fn plan(
    plans: &[CompositionLayerPlan],
    identities: &[LayerResultIdentity],
    kinds: &[SelectedTextureKind],
    extent: (u32, u32),
    clear: [u8; 4],
) -> Result<NativeCompositionPlan, CompositionPlanFailure> {
    let layers = plans
        .iter()
        .zip(identities)
        .zip(kinds)
        .map(|((plan, identity), kind)| NativeCompositionLayer {
            plan,
            identity: *identity,
            selected_entry: identity.entry,
            kind: *kind,
        })
        .collect::<Vec<_>>();
    NativeCompositionPlan::preflight(
        context(),
        frame(),
        extent,
        clear,
        &layers,
        16384,
        128 * 1024 * 1024,
    )
}

#[test]
fn pure_preflight_checks_identity_before_encoding_and_distinguishes_decline() {
    let size = (16, 8);
    let plans = [layer(
        1,
        VideoBlendMode::Normal,
        1.0,
        Transform2D::default(),
    )];
    let original = identity(1, size);
    for mutate in [
        |id: &mut LayerResultIdentity| id.context.device_generation += 1,
        |id: &mut LayerResultIdentity| id.frame.output_id += 1,
        |id: &mut LayerResultIdentity| id.frame.project_epoch += 1,
        |id: &mut LayerResultIdentity| id.frame.geometry_generation += 1,
        |id: &mut LayerResultIdentity| id.frame.request_sequence -= 1,
        |id: &mut LayerResultIdentity| id.entry.input.project_render_epoch += 1,
        |id: &mut LayerResultIdentity| id.layer_id += 1,
    ] {
        let mut id = original;
        mutate(&mut id);
        assert!(matches!(
            plan(
                &plans,
                &[id],
                &[SelectedTextureKind::InThisTransaction],
                size,
                [0; 4]
            ),
            Err(CompositionPlanFailure::Fault(NativeGpuFault {
                kind: NativeGpuFaultKind::IdentityMismatch,
                ..
            }))
        ));
    }
    let mut historical = original;
    historical.frame.request_sequence -= 1;
    assert!(plan(
        &plans,
        &[historical],
        &[SelectedTextureKind::Validated],
        size,
        [0; 4]
    )
    .is_ok());
    let layer_input = [NativeCompositionLayer {
        plan: &plans[0],
        selected_entry: HistoryEntryId {
            sequence: original.entry.sequence + 1,
            ..original.entry
        },
        identity: original,
        kind: SelectedTextureKind::Validated,
    }];
    assert!(matches!(
        NativeCompositionPlan::preflight(
            context(),
            frame(),
            size,
            [0; 4],
            &layer_input,
            16384,
            1024
        ),
        Err(CompositionPlanFailure::Fault(_))
    ));
    let mut wrong = original;
    wrong.format = VideoPixelFormat::Bgra8;
    assert_eq!(
        plan(
            &plans,
            &[wrong],
            &[SelectedTextureKind::Validated],
            size,
            [0; 4]
        )
        .unwrap_err(),
        CompositionPlanFailure::Declined(NativeGpuDecline::UnsupportedFormat)
    );
    wrong = original;
    wrong.width -= 1;
    assert_eq!(
        plan(
            &plans,
            &[wrong],
            &[SelectedTextureKind::Validated],
            size,
            [0; 4]
        )
        .unwrap_err(),
        CompositionPlanFailure::Declined(NativeGpuDecline::UnsupportedExtent)
    );
    for opacity in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut unsupported = plans.clone();
        unsupported[0].opacity = opacity;
        assert_eq!(
            plan(
                &unsupported,
                &[original],
                &[SelectedTextureKind::Validated],
                size,
                [0; 4]
            )
            .unwrap_err(),
            CompositionPlanFailure::Declined(NativeGpuDecline::UnsupportedComposition)
        );
    }
    for transform in [
        Transform2D {
            rotation_deg: 1.0,
            ..Default::default()
        },
        Transform2D {
            x: f32::NAN,
            ..Default::default()
        },
    ] {
        let mut unsupported = plans.clone();
        unsupported[0].transform = transform;
        assert!(matches!(
            plan(
                &unsupported,
                &[original],
                &[SelectedTextureKind::Validated],
                size,
                [0; 4]
            ),
            Err(CompositionPlanFailure::Declined(
                NativeGpuDecline::UnsupportedComposition
            ))
        ));
    }
    for extent in [(0, 8), (16, 0), (u32::MAX, u32::MAX)] {
        assert!(matches!(
            plan(
                &plans,
                &[identity(1, extent)],
                &[SelectedTextureKind::Validated],
                extent,
                [0; 4]
            ),
            Err(CompositionPlanFailure::Declined(
                NativeGpuDecline::UnsupportedExtent
            ))
        ));
    }
    assert!(matches!(
        plan(&[], &[], &[], size, [0; 4]),
        Err(CompositionPlanFailure::Declined(
            NativeGpuDecline::UnsupportedComposition
        ))
    ));
}

#[derive(Default)]
struct FakePort {
    calls: Vec<String>,
    fail_layer: Option<u64>,
}
impl CompositionEncodingPort for FakePort {
    type Encoded = Vec<String>;
    type Error = &'static str;
    fn begin(&mut self, plan: &NativeCompositionPlan) -> Result<(), Self::Error> {
        self.calls.push(format!(
            "clear {:?} {:?} {} {}",
            plan.extent, plan.clear_color, plan.context.owner_id, plan.frame.request_sequence
        ));
        Ok(())
    }
    fn composite_layer(&mut self, layer: &CompositionLayerEncoding) -> Result<(), Self::Error> {
        if self.fail_layer == Some(layer.identity.layer_id as u64) {
            return Err("encode failed");
        }
        self.calls.push(format!(
            "layer {} {:?} {}",
            layer.identity.layer_id, layer.kind, layer.selected_entry.sequence
        ));
        Ok(())
    }
    fn copy_premapping_to_snapshot(&mut self, bytes: u64) -> Result<(), Self::Error> {
        self.calls.push(format!("snapshot {bytes}"));
        Ok(())
    }
    fn finish(&mut self) -> Result<Self::Encoded, Self::Error> {
        Ok(std::mem::take(&mut self.calls))
    }
}

#[test]
fn fake_port_plan_preserves_order_single_snapshot_and_stops_after_failure() {
    let size = (17, 11);
    let plans = [
        layer(2, VideoBlendMode::Normal, 0.8, Transform2D::default()),
        layer(1, VideoBlendMode::Screen, 0.2, Transform2D::default()),
    ];
    let p = plan(
        &plans,
        &[identity(2, size), identity(1, size)],
        &[
            SelectedTextureKind::Validated,
            SelectedTextureKind::InThisTransaction,
        ],
        size,
        [5, 6, 7, 8],
    )
    .unwrap();
    assert_eq!(
        p.allocations,
        CompositionAllocationPlan {
            premapping_bytes: 748,
            readback_bytes: 748,
            lookup_bytes: 224,
            uniform_bytes: 64
        }
    );
    assert_eq!(
        p.layers_bottom_first[0].lookup,
        crate::axis_aligned_source_lookup(&plans[0].transform, size, size)
    );
    let calls = p.encode(&mut FakePort::default()).unwrap();
    assert_eq!(
        calls,
        [
            "clear (17, 11) [5, 6, 7, 8] 7 9",
            "layer 2 Validated 11",
            "layer 1 InThisTransaction 11",
            "snapshot 748"
        ]
    );
    let mut failed = FakePort {
        fail_layer: Some(1),
        ..Default::default()
    };
    assert_eq!(p.encode(&mut failed), Err("encode failed"));
    assert_eq!(failed.calls.len(), 2);
}

#[test]
fn texture_shader_validates_with_pinned_naga() {
    let module = naga::front::wgsl::parse_str(TEXTURE_COMPOSITION_SHADER).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}

pub(crate) struct Headless {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture_layout: wgpu::BindGroupLayout,
    texture_pipeline: wgpu::ComputePipeline,
    clear_pipeline: wgpu::ComputePipeline,
    buffer_layout: wgpu::BindGroupLayout,
    buffer_pipeline: wgpu::ComputePipeline,
}

pub(crate) fn headless() -> Option<Headless> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    })) {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("SKIPPED real-adapter texture composition oracle: {error}");
            return None;
        }
    };
    eprintln!(
        "REAL_ADAPTER texture composition oracle: {:?}",
        adapter.get_info()
    );
    // Adapter absence may skip. An available adapter's request/validation failure
    // is a failure, never a successful fake GPU result.
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        count: None,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    };
    let uniform = wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::COMPUTE,
        count: None,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    };
    let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("P texture oracle"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            },
            storage(1, false),
            uniform,
            storage(3, true),
        ],
    });
    let buffer_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("P old buffer oracle"),
        entries: &[storage(0, true), storage(1, false), uniform],
    });
    let make = |bind_layout: &wgpu::BindGroupLayout, source: &str, entry| {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        })
    };
    let texture_pipeline = make(
        &texture_layout,
        TEXTURE_COMPOSITION_SHADER,
        "composite_layer",
    );
    let clear_pipeline = make(&texture_layout, TEXTURE_COMPOSITION_SHADER, "clear_output");
    let buffer_pipeline = make(
        &buffer_layout,
        include_str!("gpu_display_composition.wgsl"),
        "composite_layer",
    );
    Some(Headless {
        device,
        queue,
        texture_layout,
        texture_pipeline,
        clear_pipeline,
        buffer_layout,
        buffer_pipeline,
    })
}

struct RealAdapterPort<'a> {
    gpu: &'a Headless,
    encoder: Option<wgpu::CommandEncoder>,
    groups: &'a [wgpu::BindGroup],
    plan: &'a NativeCompositionPlan,
    readback: &'a wgpu::Buffer,
    output: &'a wgpu::Buffer,
    texture: bool,
    next: usize,
}
impl CompositionEncodingPort for RealAdapterPort<'_> {
    type Encoded = wgpu::CommandBuffer;
    type Error = &'static str;
    fn begin(&mut self, plan: &NativeCompositionPlan) -> Result<(), Self::Error> {
        if plan.context != self.plan.context
            || plan.frame != self.plan.frame
            || plan.extent != self.plan.extent
        {
            return Err("foreign plan");
        }
        let encoder = self.encoder.as_mut().unwrap();
        if self.texture {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.gpu.clear_pipeline);
            pass.set_bind_group(0, &self.groups[0], &[]);
            pass.dispatch_workgroups(plan.extent.0.div_ceil(8), plan.extent.1.div_ceil(8), 1);
        } else {
            if plan.clear_color != [0; 4] {
                return Err("old oracle clears transparent black only");
            }
            encoder.clear_buffer(self.output, 0, Some(plan.allocations.premapping_bytes));
        }
        Ok(())
    }
    fn composite_layer(&mut self, layer: &CompositionLayerEncoding) -> Result<(), Self::Error> {
        let expected = &self.plan.layers_bottom_first[self.next];
        if layer.selected_entry != expected.selected_entry
            || layer.identity != expected.identity
            || layer.kind != expected.kind
        {
            return Err("foreign texture identity");
        }
        let mut pass = self
            .encoder
            .as_mut()
            .unwrap()
            .begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(if self.texture {
            &self.gpu.texture_pipeline
        } else {
            &self.gpu.buffer_pipeline
        });
        pass.set_bind_group(0, &self.groups[self.next], &[]);
        pass.dispatch_workgroups(
            self.plan.extent.0.div_ceil(8),
            self.plan.extent.1.div_ceil(8),
            1,
        );
        self.next += 1;
        Ok(())
    }
    fn copy_premapping_to_snapshot(&mut self, bytes: u64) -> Result<(), Self::Error> {
        self.encoder.as_mut().unwrap().copy_buffer_to_buffer(
            self.output,
            0,
            self.readback,
            0,
            bytes,
        );
        Ok(())
    }
    fn finish(&mut self) -> Result<Self::Encoded, Self::Error> {
        Ok(self.encoder.take().unwrap().finish())
    }
}

pub(crate) fn render(
    gpu: &Headless,
    plan: &NativeCompositionPlan,
    frames: &[Arc<VideoFrame>],
    texture: bool,
) -> Vec<u8> {
    let buffer = |bytes: &[u8], usage| {
        gpu.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytes,
                usage,
            })
    };
    let output = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: plan.allocations.premapping_bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: plan.allocations.readback_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut groups = Vec::new();
    for (layer, frame) in plan.layers_bottom_first.iter().zip(frames) {
        let lookup = layer
            .lookup
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>();
        let mut params = layer.params;
        if texture {
            let source = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
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
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &source,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &frame.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(frame.width * 4),
                    rows_per_image: Some(frame.height),
                },
                source.size(),
            );
            let view = source.create_view(&wgpu::TextureViewDescriptor::default());
            let lookup = buffer(&lookup, wgpu::BufferUsages::STORAGE);
            let params = buffer(&params, wgpu::BufferUsages::UNIFORM);
            groups.push(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &gpu.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: lookup.as_entire_binding(),
                    },
                ],
            }));
        } else {
            let mut source = frame.data.clone();
            source.extend_from_slice(&lookup);
            params[24..28].copy_from_slice(&0u32.to_le_bytes());
            params[28..32].copy_from_slice(&((frame.data.len() / 4) as u32).to_le_bytes());
            let source = buffer(&source, wgpu::BufferUsages::STORAGE);
            let params = buffer(&params, wgpu::BufferUsages::UNIFORM);
            groups.push(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &gpu.buffer_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: source.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params.as_entire_binding(),
                    },
                ],
            }));
        }
    }
    let mut port = RealAdapterPort {
        gpu,
        encoder: Some(
            gpu.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
        ),
        groups: &groups,
        plan,
        readback: &readback,
        output: &output,
        texture,
        next: 0,
    };
    gpu.queue.submit(Some(plan.encode(&mut port).unwrap()));
    let (sender, receiver) = mpsc::sync_channel(1);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    gpu.device.poll(wgpu::PollType::wait()).unwrap();
    receiver.recv().unwrap().unwrap();
    let data = {
        let view = readback.slice(..).get_mapped_range();
        view.to_vec()
    };
    readback.unmap();
    data
}

fn gradient(id: u64, size: (u32, u32), seed: u8, opaque: bool) -> Arc<VideoFrame> {
    Arc::new(VideoFrame {
        layer_id: id as _,
        width: size.0,
        height: size.1,
        pts_ms: 33,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: (0..size.0 * size.1)
            .flat_map(|index| {
                let v = (index as u8).wrapping_mul(29).wrapping_add(seed);
                [v, v.wrapping_mul(3), 255 - v, if opaque { 255 } else { v }]
            })
            .collect(),
    })
}

#[test]
fn real_adapter_texture_pixels_match_existing_gpu_and_cpu_oracles() {
    let Some(gpu) = headless() else { return };
    let size = (31, 19);
    let modes = [
        VideoBlendMode::Normal,
        VideoBlendMode::Add,
        VideoBlendMode::Multiply,
        VideoBlendMode::Screen,
        VideoBlendMode::Overlay,
        VideoBlendMode::Subtract,
        VideoBlendMode::Difference,
        VideoBlendMode::Lighten,
        VideoBlendMode::Darken,
        VideoBlendMode::SoftLight,
    ];
    let transforms = [
        Transform2D::default(),
        Transform2D {
            x: 0.07,
            y: -0.035,
            scale_x: 0.83,
            scale_y: 0.6,
            crop_left: 0.13,
            crop_bottom: 0.25,
            ..Default::default()
        },
        Transform2D {
            x: 1.2,
            ..Default::default()
        },
        Transform2D {
            crop_left: 0.8,
            crop_right: 0.3,
            ..Default::default()
        },
        Transform2D {
            scale_x: -0.4,
            scale_y: 0.0,
            ..Default::default()
        },
    ];
    let frames = [gradient(1, size, 3, true), gradient(2, size, 41, false)];
    let mut cases = 0;
    for mode in modes {
        for opacity in [-0.5, 0.0, 0.01, 0.37, 0.7, 1.0, 1.5] {
            for transform in transforms {
                let layers = [
                    layer(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
                    layer(2, mode.clone(), opacity, transform),
                ];
                let plan = plan(
                    &layers,
                    &[identity(1, size), identity(2, size)],
                    &[
                        SelectedTextureKind::Validated,
                        SelectedTextureKind::InThisTransaction,
                    ],
                    size,
                    [0; 4],
                )
                .unwrap();
                let actual = render(&gpu, &plan, &frames, true);
                let old = render(&gpu, &plan, &frames, false);
                assert_eq!(actual,old,"texture/old GPU mismatch mode={mode:?} opacity={opacity} transform={transform:?}");
                let cpu_plan = CompositionPlan {
                    composition_id: 1,
                    label: "oracle".into(),
                    output_ids: vec![],
                    master_opacity: 1.0,
                    blackout: false,
                    layers: layers.to_vec(),
                };
                let cpu =
                    crate::composite_frames_rgba8(&cpu_plan, &frames, size.0, size.1).unwrap();
                // Existing oracle allows one rounding level per blend. The opaque
                // backdrop is exact, so only the top blend may differ by one level.
                for (index, (actual, expected)) in actual.iter().zip(&cpu.data).enumerate() {
                    assert!(actual.abs_diff(*expected)<=1,"CPU mismatch channel={index} GPU={actual} CPU={expected} mode={mode:?} opacity={opacity}");
                }
                cases += 1;
            }
        }
    }
    // No accidental sRGB conversion/premultiplication: opaque source exact.
    let layers = [layer(
        1,
        VideoBlendMode::Normal,
        1.0,
        Transform2D::default(),
    )];
    let p = plan(
        &layers,
        &[identity(1, size)],
        &[SelectedTextureKind::Validated],
        size,
        [0; 4],
    )
    .unwrap();
    assert_eq!(render(&gpu, &p, &frames[..1], true), frames[0].data);
    // Clear happens on GPU, including nonzero colors; uncovered pixels survive.
    let layers = [layer(
        1,
        VideoBlendMode::Normal,
        1.0,
        Transform2D {
            x: 2.0,
            ..Default::default()
        },
    )];
    let p = plan(
        &layers,
        &[identity(1, size)],
        &[SelectedTextureKind::Validated],
        size,
        [7, 19, 121, 63],
    )
    .unwrap();
    assert_eq!(
        render(&gpu, &p, &frames[..1], true),
        [7, 19, 121, 63].repeat((size.0 * size.1) as usize)
    );
    eprintln!("REAL_ADAPTER texture composition oracle: {cases} blend/opacity/sampling cases + exact RGBA + GPU clear passed; premapping only, no native/performance proof");
}

// Shared-context fixtures: G's real headless owner, real ISF candidates and
// validated leases. These are reused by the snapshot real-adapter tests.
use crate::isf_native_gpu::tests::{jobs, shader, Fixture, IDENTITY};
use crate::isf_native_gpu::{NativeBatchAdmission, NativeBatchRestrictions, PreparedNativeLayer};
use crate::isf_runtime::StackBatch;
use crate::native_gpu_context::{PendingGpuTransaction, ResolvedGpuTransaction};
use crate::native_gpu_result::{EncodedIsfBatch, NativeIsfCompletion, ValidatedLayerResult};

/// `count` independent three-stage identity chains (layer ids `4 - count..=3`)
/// whose inputs are replaced in place, preserving G's planned capacities.
pub(crate) fn identity_batch(
    size: (u32, u32),
    count: usize,
    pixel: impl Fn(u64, usize) -> [u8; 4],
) -> StackBatch<PreparedNativeLayer> {
    let identity = shader(IDENTITY);
    let mut slots: Vec<Option<PreparedNativeLayer>> = jobs(
        size.0,
        size.1,
        &[identity.clone(), identity.clone(), identity],
    )
    .into_iter()
    .map(Some)
    .collect();
    let skip = 3 - count;
    let mut batch = StackBatch::from_fn(count, |index| {
        slots[skip + index].take().expect("unique job slot")
    });
    for job in batch.iter_mut() {
        let layer = job.input.layer_id;
        let input = Arc::get_mut(&mut job.input).expect("fixture owns its input");
        for (index, rgba) in input.data.chunks_exact_mut(4).enumerate() {
            rgba.copy_from_slice(&pixel(layer, index));
        }
    }
    batch
}

/// The identities G's encode will mint (same derivation as G's own fixture).
/// I currently has no G API for prospective candidate identities; see P50.
pub(crate) fn predicted_identities(
    fixture: &Fixture,
    frame: NativeFrameId,
    batch: &[PreparedNativeLayer],
) -> Vec<LayerResultIdentity> {
    let first = fixture.runtime.native_frame_index();
    batch
        .iter()
        .enumerate()
        .map(|(index, job)| LayerResultIdentity {
            context: fixture.owner.context().id(),
            frame,
            entry: job.entry,
            layer_id: job.input.layer_id,
            source_generation: job.source_generation,
            content_sequence: job.content_sequence,
            pts_ms: job.input.pts_ms,
            duration_ms: job.input.duration_ms,
            width: job.input.width,
            height: job.input.height,
            format: job.input.format,
            origin: LayerOrigin::Executed(FrozenChainClocks {
                frame_index: first.wrapping_add(index as u32),
                time_seconds_bits: (job.input.pts_ms as f32 / 1000.0).to_bits(),
                time_delta_seconds_bits: (job.input.duration_ms as f32 / 1000.0).to_bits(),
                date_bits: job.date.map(f32::to_bits),
            }),
        })
        .collect()
}

pub(crate) struct SharedFrame {
    pub frame: NativeFrameId,
    pub isf: EncodedIsfBatch,
    pub pending: PendingGpuTransaction,
    pub encoded: Result<EncodedPremappingSnapshot, NativeGpuFault>,
    /// CPU inputs of the current candidates, bottom-first after history.
    pub candidate_inputs: Vec<Arc<VideoFrame>>,
}

/// One frame exactly as INTERFACE-01 §1.1 orders it: a single G/P quote and
/// reservation, P's split before ISF binds, G's ISF submit, then P's single
/// composition+snapshot submit, with no host wait or poll in between.
pub(crate) fn submit_shared_frame(
    fixture: &mut Fixture,
    compositor: &NativeGpuCompositor,
    sequence: u64,
    batch: StackBatch<PreparedNativeLayer>,
    history: &[&GpuLayerLease],
    plans: &[CompositionLayerPlan],
    swap_candidates: bool,
) -> SharedFrame {
    let frame = crate::isf_native_gpu::tests::request(sequence);
    let predicted = predicted_identities(fixture, frame, &batch);
    let candidate_inputs = batch.iter().map(|job| Arc::clone(&job.input)).collect();
    let extent = (batch[0].input.width, batch[0].input.height);
    // This low-level fixture bypasses the resident session's resize fence.
    // Mirror its shape configuration without resetting unchanged banks.
    let shape = std::array::from_fn(|index| {
        batch.get(index).map(|layer| {
            (
                layer.input.width,
                layer.input.height,
                layer.input.format,
                layer.chain.stages.len(),
            )
        })
    });
    fixture.owner.configure_stack_pool_shape(shape);
    let inputs = fixture
        .runtime
        .prepare_native_inputs(frame, batch, NativeBatchRestrictions::default())
        .unwrap();
    let selected = history
        .iter()
        .map(|lease| (*lease.identity(), SelectedTextureKind::Validated))
        .chain(
            predicted
                .into_iter()
                .map(|identity| (identity, SelectedTextureKind::InThisTransaction)),
        )
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), plans.len());
    let layers = plans
        .iter()
        .zip(&selected)
        .map(|(plan, (identity, kind))| NativeCompositionLayer {
            plan,
            selected_entry: identity.entry,
            identity: *identity,
            kind: *kind,
        })
        .collect::<Vec<_>>();
    let limits = fixture.owner.capabilities().limits.clone();
    let plan = NativeCompositionPlan::preflight(
        fixture.owner.context().id(),
        frame,
        extent,
        [0; 4],
        &layers,
        limits.max_texture_dimension_2d,
        u64::from(limits.max_storage_buffer_binding_size),
    )
    .unwrap();
    let mut frame_plan = inputs.allocation_plan().unwrap().clone();
    frame_plan.merge(plan.allocation_plan()).unwrap();
    for lease in history {
        frame_plan.add_existing(lease.allocation_id());
    }
    let quote = fixture.owner.quote(&frame_plan).unwrap();
    let admitted = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        NativeBatchAdmission::Declined { reason, .. } => {
            panic!("shared fixture declined: {reason:?}")
        }
    };
    let reservation = fixture.owner.reserve(quote).unwrap();
    let mut txn = fixture.owner.begin_frame(frame, reservation).unwrap();
    let composition = compositor.reserve(&mut txn, &plan).unwrap();
    let isf = fixture
        .runtime
        .encode_native_batch(&mut txn, admitted)
        .unwrap();
    let mut candidates = isf
        .candidates
        .iter()
        .map(SelectedLayerInput::InThisTransaction)
        .collect::<Vec<_>>();
    if swap_candidates {
        candidates.reverse();
    }
    let sources = history
        .iter()
        .map(|lease| SelectedLayerInput::Validated(lease))
        .chain(candidates)
        .collect::<Vec<_>>();
    let encoded =
        compositor.encode_composition_and_snapshot(&mut txn, &plan, &sources, composition);
    drop(sources);
    if let Ok(encoded) = &encoded {
        // ISF submit, then exactly the next submission is P's composition.
        let isf_submission = isf.pending.submission();
        assert_eq!(encoded.submission().context, isf_submission.context);
        assert_eq!(encoded.submission().sequence, isf_submission.sequence + 1);
        assert_eq!(encoded.frame(), frame);
    }
    SharedFrame {
        frame,
        isf,
        pending: txn.finish(),
        encoded,
        candidate_inputs,
    }
}

pub(crate) fn validate_isf(
    fixture: &mut Fixture,
    mut isf: EncodedIsfBatch,
    resolved: &mut ResolvedGpuTransaction,
) -> StackBatch<ValidatedLayerResult> {
    let receipt = isf.pending.take().unwrap();
    let phase = resolved.take_phase(receipt).unwrap();
    match fixture
        .runtime
        .complete_native_batch(isf, phase, &mut fixture.owner, &|| false)
    {
        NativeIsfCompletion::Validated { layers, .. } => layers,
        _ => panic!("ISF must validate independently of P's composition phase"),
    }
}

#[test]
fn real_shared_composition_rejects_mismatched_selection_before_its_phase() {
    use crate::isf_native_gpu::{await_transaction, tests::gpu, tests::stop};
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let size = (19, 13);
    let batch = identity_batch(size, 3, |layer, index| {
        [layer as u8 * 40, index as u8, 200, 255]
    });
    let plans = [
        layer(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
        layer(2, VideoBlendMode::Add, 1.0, Transform2D::default()),
        layer(3, VideoBlendMode::Lighten, 1.0, Transform2D::default()),
    ];
    let shared = submit_shared_frame(&mut fixture, &compositor, 1, batch, &[], &plans, true);
    let error = shared
        .encoded
        .err()
        .expect("swapped selection must be rejected");
    assert_eq!(error.kind, NativeGpuFaultKind::IdentityMismatch);
    assert_eq!(error.phase, GpuPhase::CompositionSnapshot);
    let mut resolved = await_transaction(&mut fixture.owner, shared.pending, &|| false).unwrap();
    let layers = validate_isf(&mut fixture, shared.isf, &mut resolved);
    assert_eq!(
        layers.len(),
        3,
        "ISF history is kept when composition fails"
    );
    drop(layers);
    drop(resolved);
    // Successful ISF resources may remain in G's idle bank even though P
    // rejected its selection. Release bank ownership before zero-ledger proof.
    fixture.owner.release_resource_pools();
    fixture.owner.poll_retirement().unwrap();
    // P's never-spent sub-reservation was refunded with the frame.
    assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
    stop(&mut fixture.owner);
}

#[test]
fn pure_output_pool_reuses_240_ticks_but_never_a_referenced_slot() {
    let key = (1, 4, (19, 13));
    let mut pool = CompositionOutputPool::default();
    assert!(pool.prepare(key).unwrap().is_none());
    // This owner stands for G's charged strong bank reference.
    let bank = Arc::new(7u64);
    pool.retain(key, &bank);
    for _ in 0..240 {
        let lease = pool.prepare(key).unwrap().unwrap();
        assert!(Arc::ptr_eq(&lease, &bank));
        // Ready/LastValid/registry holds the lease until safe retirement.
        assert!(pool.prepare(key).unwrap().is_none());
        drop(lease);
        assert_eq!(pool.slots.len(), 1);
    }
    let lease = Arc::clone(&bank);
    let bank2 = Arc::new(8u64);
    pool.retain(key, &bank2);
    let lease2 = Arc::clone(&bank2);
    let bank3 = Arc::new(9u64);
    pool.retain(key, &bank3);
    let lease3 = Arc::clone(&bank3);
    assert!(
        pool.prepare(key).is_err(),
        "full referenced bank is bounded"
    );
    drop(lease2);
    assert!(Arc::ptr_eq(&pool.prepare(key).unwrap().unwrap(), &bank2));
    drop((lease, lease3));
}

#[test]
fn pure_output_pool_revocation_resize_and_loss_never_recycle_old_leases() {
    for next_key in [(2, 4, (19, 13)), (1, 5, (20, 13)), (2, 5, (19, 13))] {
        let key = (1, 4, (19, 13));
        let mut pool = CompositionOutputPool::default();
        pool.prepare(key).unwrap();
        let bank = Arc::new(7u64);
        pool.retain(key, &bank);
        let ready = Arc::clone(&bank);
        // Stop/revoke/blackout/loss drops G bank ownership, increments epoch,
        // and leaves in-flight/Ready references valid until safely released.
        drop(bank);
        assert!(pool.prepare(next_key).unwrap().is_none());
        assert_eq!(*ready, 7);
        assert!(pool.slots.is_empty());
        drop(ready);
    }
    let mut pool = CompositionOutputPool::default();
    let key = (1, 4, (19, 13));
    pool.prepare(key).unwrap();
    let bank = Arc::new(7u64);
    let weak = Arc::downgrade(&bank);
    pool.retain(key, &bank);
    pool.clear();
    assert_eq!(Arc::strong_count(&bank), 1, "P metadata owns no charge");
    drop(bank);
    assert!(weak.upgrade().is_none());
}

#[test]
fn pure_output_pool_existing_alias_does_not_add_allocation_bytes() {
    let output = GpuAllocationId {
        context: context(),
        sequence: 50,
    };
    let readback = GpuAllocationId {
        context: context(),
        sequence: 51,
    };
    let fresh = composition_allocations(frame(), 748, &[], None, None).unwrap();
    let reused = composition_allocations(frame(), 748, &[], Some(readback), Some(output)).unwrap();
    assert_eq!(fresh.requested_bytes(), 748 * 3);
    assert_eq!(
        reused.requested_bytes(),
        748,
        "only owned CPU snapshot remains new"
    );
}

/// Uses the actual G/P/I submission, snapshot map, cache commit and Ready
/// constructor. No manufactured completion receipt or Ready is involved.
#[test]
fn real_shared_output_pool_reuses_allocation_ids_but_never_held_ready() {
    use crate::isf_native_gpu::tests::{gpu, request, stop};
    use crate::native_gpu_contract::PresentRejection;
    use crate::native_gpu_output::shared::{
        PendingSharedNativeOutput, SharedNativeOutputCache, SharedNativeOutputProgress,
    };
    use crate::native_gpu_output::{NativeOutputAuthority, NativeOutputRequest, OutputBoundary};
    use crate::{VideoOutputLastValidKey, VideoOutputMappingIdentity};
    use protocol::{VideoOutputKind, VideoOutputMapping};
    use std::time::{Duration, Instant};

    struct Authority;
    impl NativeOutputAuthority for Authority {
        fn revalidate(
            &self,
            _: &NativeOutputRequest,
            _: OutputBoundary,
        ) -> Result<(), PresentRejection> {
            Ok(())
        }
    }
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let size = (19, 13);
    let mut cache = SharedNativeOutputCache::new(request(1).output_id);
    let plans = [layer(
        3,
        VideoBlendMode::Normal,
        1.0,
        Transform2D::default(),
    )];
    let mut ids = Vec::new();
    let mut held_ready = None;
    for sequence in 1..=12 {
        let frame = request(sequence);
        let key = VideoOutputLastValidKey {
            project_render_epoch: frame.project_epoch,
            output_id: frame.output_id,
            width: size.0,
            height: size.1,
            composition_id: 1,
            kind: VideoOutputKind::Display,
            fullscreen: false,
            monitor_id: Some(1),
            endpoint_name: None,
        };
        let fence = cache.begin_request(key.clone(), frame).unwrap();
        let mapping = VideoOutputMapping::default();
        let request = NativeOutputRequest {
            context: fixture.owner.context().id(),
            frame,
            key,
            fence,
            pts_ms: 33,
            duration_ms: 33,
            mapping_identity: VideoOutputMappingIdentity::from_mapping(&mapping),
            mapping,
        };
        let batch = identity_batch(size, 1, |_, index| [index as u8, 42, sequence as u8, 255]);
        let identities = predicted_identities(&fixture, frame, &batch);
        let inputs = fixture
            .runtime
            .prepare_native_inputs(frame, batch, NativeBatchRestrictions::default())
            .unwrap();
        let selected = [NativeCompositionLayer {
            plan: &plans[0],
            selected_entry: identities[0].entry,
            identity: identities[0],
            kind: SelectedTextureKind::InThisTransaction,
        }];
        let limits = &fixture.owner.capabilities().limits;
        let mut plan = NativeCompositionPlan::preflight(
            request.context,
            frame,
            size,
            [0; 4],
            &selected,
            limits.max_texture_dimension_2d,
            u64::from(limits.max_storage_buffer_binding_size),
        )
        .unwrap();
        compositor
            .prepare_pooled_output(&mut plan, fixture.owner.context().resource_pool_epoch())
            .unwrap();
        let mut allocations = inputs.allocation_plan().unwrap().clone();
        allocations.merge(plan.allocation_plan()).unwrap();
        let quote = fixture.owner.quote(&allocations).unwrap();
        let admitted = match fixture.runtime.admit_native_batch(inputs, &quote) {
            NativeBatchAdmission::Accepted(admitted) => admitted,
            NativeBatchAdmission::Declined { reason, .. } => {
                panic!("pooled frame declined: {reason:?}")
            }
        };
        let reservation = fixture.owner.reserve(quote).unwrap();
        let mut pending = PendingSharedNativeOutput::submit(
            &mut fixture.owner,
            &mut fixture.runtime,
            &compositor,
            request,
            admitted,
            reservation,
            &plan,
            &[],
        )
        .unwrap_or_else(|failure| panic!("pooled submission failed: {:?}", failure.reason()));
        drop(plan);
        let started = Instant::now();
        let ready = loop {
            match pending.poll(
                &mut fixture.owner,
                &mut fixture.runtime,
                &mut cache,
                &Authority,
                &|| false,
            ) {
                SharedNativeOutputProgress::Pending(next) => {
                    assert!(started.elapsed() < Duration::from_secs(5));
                    pending = next;
                    std::thread::yield_now();
                }
                SharedNativeOutputProgress::Ready { ready, layers, isf } => {
                    drop((layers, isf));
                    break ready;
                }
                _ => panic!("pooled output failed to become Ready"),
            }
        };
        let id = ready.payload().premapping().allocation_id();
        let cached = cache
            .last_valid(&ready.cache_receipt().descriptor().key)
            .unwrap();
        for (index, pixel) in cached.data.chunks_exact(4).enumerate() {
            assert_eq!(
                pixel,
                [index as u8, 42, sequence as u8, 255],
                "reused premapping must publish this tick's pixels"
            );
        }
        if sequence > 1 && sequence <= 4 {
            assert_ne!(id, ids[0], "held real Ready must not be overwritten");
        }
        ids.push(id);
        if sequence == 1 {
            held_ready = Some(ready);
        } else {
            drop(ready);
        }
        if sequence == 4 {
            assert_eq!(ids[3], ids[1], "retired second output should be reused");
            drop(held_ready.take());
        }
        fixture.owner.poll_retirement().unwrap();
    }
    assert_eq!(
        ids[4], ids[0],
        "first slot is reusable only after real Ready release"
    );
    assert_ne!(ids[1], ids[2], "LastValid pins the preceding output");
    assert!(
        ids.iter().all(|id| ids[..3].contains(id)),
        "bounded bank never grows after warmup"
    );
    cache.blackout().unwrap();
    fixture.owner.release_resource_pools();
    compositor.release_pooled_outputs();
    stop(&mut fixture.owner);
}
