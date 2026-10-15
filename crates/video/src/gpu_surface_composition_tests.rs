//! The presenter's GPU composition passes, read back without a surface: the
//! mapped buffer matches compositing the same layers on the CPU and mapping
//! that frame on the CPU (the same source pixels, DXT decoded exactly, a
//! blend rounded at most one level the other way), and a slot that already
//! holds the same shared frame is not uploaded again.
use std::sync::mpsc;

use protocol::{
    Transform2D, VideoBlendMode, VideoColorAdjust, VideoFxAdjust, VideoLayerId,
    VideoOutputAspectMode, VideoSourceKind, VideoSourceSummary,
};

use super::*;
use crate::{
    apply_video_output_mapping, composite_frames_rgba8, gpu_compositor::GpuCompositePipelines,
    CompositionLayerPlan, CompositionPlan, VideoOutputGpuLayer, VideoOutputMappingIdentity,
    VideoOutputRenderEvidence, VideoOutputRenderFreshness, VideoPixelFormat,
};

struct Headless {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: GpuCompositePipelines,
    composition_pipeline: GpuCompositionPipeline,
    output_layout: wgpu::BindGroupLayout,
}

fn headless() -> Option<Headless> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    })) {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("GPU composition test skipped: {error}");
            return None;
        }
    };
    let (device, queue) =
        match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal GPU composition test device"),
            required_features: requested_video_device_features(&adapter),
            ..Default::default()
        })) {
            Ok(pair) => pair,
            Err(error) => {
                eprintln!("GPU composition test skipped: {error}");
                return None;
            }
        };
    let pipelines = create_gpu_composite_pipelines(&device);
    let composition_pipeline = create_gpu_composition_pipeline(&device);
    let output_layout = create_buffer_output_bind_group_layout(&device);
    Some(Headless {
        device,
        queue,
        pipelines,
        composition_pipeline,
        output_layout,
    })
}

fn layer_plan(
    layer_id: VideoLayerId,
    blend_mode: VideoBlendMode,
    opacity: f32,
    transform: Transform2D,
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
        transform,
        color: VideoColorAdjust::default(),
        fx: VideoFxAdjust::default(),
    }
}

fn gradient(
    layer_id: VideoLayerId,
    width: u32,
    height: u32,
    format: VideoPixelFormat,
    seed: u8,
) -> Arc<VideoFrame> {
    let data = (0..width * height)
        .flat_map(|index| {
            let value = (index as u8).wrapping_mul(29).wrapping_add(seed);
            [
                value,
                255u8.wrapping_sub(value.wrapping_mul(3)),
                value.wrapping_mul(7).wrapping_add(seed),
                160u8.wrapping_add(value / 3),
            ]
        })
        .collect();
    Arc::new(VideoFrame {
        layer_id,
        width,
        height,
        pts_ms: u64::from(seed),
        duration_ms: 16,
        format,
        data,
    })
}

/// Deterministic noise, so DXT blocks use every mode and index.
fn noise(seed: u32, len: usize) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

fn dxt_frame(
    layer_id: VideoLayerId,
    (width, height): (u32, u32),
    format: VideoPixelFormat,
    seed: u32,
) -> Arc<VideoFrame> {
    let block_bytes = if format == VideoPixelFormat::Dxt1 {
        8
    } else {
        16
    };
    let blocks = width.div_ceil(4) as usize * height.div_ceil(4) as usize;
    Arc::new(VideoFrame {
        layer_id,
        width,
        height,
        pts_ms: 0,
        duration_ms: 33,
        format,
        data: noise(seed, blocks * block_bytes),
    })
}

fn composition(
    layers: Vec<(CompositionLayerPlan, Arc<VideoFrame>)>,
    output_mapping: VideoOutputMapping,
) -> VideoOutputGpuComposition {
    VideoOutputGpuComposition {
        layers: layers
            .into_iter()
            .map(|(plan, frame)| VideoOutputGpuLayer { plan, frame })
            .collect(),
        output_mapping,
        output_mapping_identity: VideoOutputMappingIdentity::from_mapping(&output_mapping),
        evidence: VideoOutputRenderEvidence {
            project_render_epoch: 1,
            output_id: 1,
            freshness: VideoOutputRenderFreshness::Fresh,
            error: None,
        },
    }
}

/// What the CPU artistic path renders, before the presenter maps it.
fn cpu_reference_unmapped(
    composition: &VideoOutputGpuComposition,
    width: u32,
    height: u32,
) -> VideoFrame {
    let plan = CompositionPlan {
        composition_id: 1,
        label: "Main".to_string(),
        output_ids: Vec::new(),
        master_opacity: 1.0,
        blackout: false,
        layers: composition
            .layers
            .iter()
            .map(|layer| layer.plan.clone())
            .collect(),
    };
    let frames = composition
        .layers
        .iter()
        .map(|layer| Arc::clone(&layer.frame))
        .collect::<Vec<_>>();
    composite_frames_rgba8(&plan, &frames, width, height).unwrap()
}

/// What the CPU artistic path renders and the presenter then maps.
fn cpu_reference(composition: &VideoOutputGpuComposition, width: u32, height: u32) -> Vec<u8> {
    let composed = cpu_reference_unmapped(composition, width, height);
    apply_video_output_mapping(composed, &composition.output_mapping).data
}

fn new_buffers(gpu: &Headless, (width, height): (u32, u32)) -> NativeGpuFrameBuffers {
    NativeGpuFrameBuffers::new(
        &gpu.device,
        &gpu.pipelines.bind_group_layout,
        &gpu.output_layout,
        u64::from(width) * u64::from(height) * 4,
    )
}

/// Runs the presenter's composition passes and reads `mapped` back, with
/// how many compressed frames were uploaded.
fn render(
    gpu: &Headless,
    buffers: &mut NativeGpuFrameBuffers,
    composition: &VideoOutputGpuComposition,
    width: u32,
    height: u32,
) -> (Vec<u8>, u64) {
    let size = u64::from(width) * u64::from(height) * 4;
    let (encoder, compressed_uploads) = encode(gpu, buffers, composition, (width, height));
    let mut encoder = encoder;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Syndocal GPU composition test readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&buffers.mapped, 0, &readback, 0, size);
    gpu.queue.submit(Some(encoder.finish()));
    let (sender, receiver) = mpsc::sync_channel(1);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    gpu.device.poll(wgpu::PollType::wait()).unwrap();
    receiver.recv().unwrap().unwrap();
    let data = readback.slice(..).get_mapped_range().to_vec();
    readback.unmap();
    (data, compressed_uploads)
}

/// What `present_output_gpu_composition` does before its surface pass.
fn encode(
    gpu: &Headless,
    buffers: &mut NativeGpuFrameBuffers,
    composition: &VideoOutputGpuComposition,
    (width, height): (u32, u32),
) -> (wgpu::CommandEncoder, u64) {
    buffers.ensure_output_capacity(
        &gpu.device,
        &gpu.pipelines.bind_group_layout,
        &gpu.output_layout,
        u64::from(width) * u64::from(height) * 4,
    );
    buffers
        .prepare_composition_slots(
            &gpu.device,
            &gpu.composition_pipeline,
            composition,
            (width, height),
        )
        .unwrap();
    buffers.encode_gpu_composition(
        &gpu.device,
        &gpu.queue,
        (&gpu.composition_pipeline, &gpu.pipelines.output_mapping),
        composition,
        (width, height),
    )
}

/// The GPU samples the CPU's source pixels and rounds each blend at most
/// one level the other way, so a channel can be `levels` (the number of
/// blends) apart at most; anything more is a defect.
fn assert_within_levels(gpu: &[u8], cpu: &[u8], levels: u8) {
    assert_eq!(gpu.len(), cpu.len());
    let beyond = gpu
        .iter()
        .zip(cpu)
        .enumerate()
        .filter(|(_, (gpu, cpu))| gpu.abs_diff(**cpu) > levels)
        .map(|(index, (gpu, cpu))| (index, *gpu, *cpu))
        .collect::<Vec<_>>();
    assert!(
        beyond.is_empty(),
        "{} of {} channels differ by more than {levels} level(s) (index, GPU, CPU): {:?}",
        beyond.len(),
        gpu.len(),
        &beyond[..beyond.len().min(24)]
    );
}

fn mapping() -> VideoOutputMapping {
    VideoOutputMapping {
        offset_x: 0.04,
        offset_y: -0.03,
        scale_x: 0.92,
        scale_y: 0.88,
        rotation_deg: 4.0,
        aspect_ratio: 4.0 / 3.0,
        aspect_mode: VideoOutputAspectMode::Fit,
        edge_blend_left: 0.18,
        edge_blend_gamma: 2.0,
        black_level: 0.04,
        keystone_x: 0.06,
        ..VideoOutputMapping::default()
    }
}

#[test]
fn presenter_composition_matches_the_cpu_render_mapped_once() {
    let Some(gpu) = headless() else {
        return;
    };
    let (width, height) = (192, 108);
    let scene = composition(
        vec![
            (
                layer_plan(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
                gradient(1, 192, 108, VideoPixelFormat::Rgba8, 3),
            ),
            (
                layer_plan(
                    2,
                    VideoBlendMode::Screen,
                    0.7,
                    Transform2D {
                        x: 0.07,
                        y: -0.035,
                        scale_x: 0.83,
                        scale_y: 0.6,
                        crop_left: 0.13,
                        crop_bottom: 0.25,
                        ..Transform2D::default()
                    },
                ),
                gradient(2, 71, 45, VideoPixelFormat::Rgba8, 41),
            ),
            (
                layer_plan(3, VideoBlendMode::Add, 0.45, Transform2D::default()),
                gradient(3, 96, 54, VideoPixelFormat::Bgra8, 97),
            ),
            (
                layer_plan(4, VideoBlendMode::Multiply, 0.6, Transform2D::default()),
                dxt_frame(4, (61, 37), VideoPixelFormat::Dxt5, 11),
            ),
            (
                layer_plan(
                    5,
                    VideoBlendMode::Normal,
                    0.5,
                    Transform2D {
                        x: -0.2,
                        scale_x: 0.5,
                        scale_y: 0.5,
                        ..Transform2D::default()
                    },
                ),
                dxt_frame(5, (64, 36), VideoPixelFormat::Dxt1, 23),
            ),
        ],
        mapping(),
    );
    let mut buffers = new_buffers(&gpu, (width, height));
    let (mapped, _) = render(&gpu, &mut buffers, &scene, width, height);
    assert_within_levels(&mapped, &cpu_reference(&scene, width, height), 5);
}

#[test]
fn dxt_frames_decode_exactly_as_on_the_cpu() {
    let Some(gpu) = headless() else {
        return;
    };
    for format in [VideoPixelFormat::Dxt1, VideoPixelFormat::Dxt5] {
        for (source, output) in [((37, 23), (37, 23)), ((37, 23), (90, 50))] {
            let scene = composition(
                vec![(
                    layer_plan(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
                    dxt_frame(1, source, format, 7),
                )],
                VideoOutputMapping::default(),
            );
            let mut buffers = new_buffers(&gpu, output);
            let (mapped, _) = render(&gpu, &mut buffers, &scene, output.0, output.1);
            let cpu = cpu_reference(&scene, output.0, output.1);
            // An opaque sample is copied, so it shows the decoded pixel itself.
            for (index, (gpu_pixel, cpu_pixel)) in
                mapped.chunks_exact(4).zip(cpu.chunks_exact(4)).enumerate()
            {
                if cpu_pixel[3] == 255 {
                    assert_eq!(gpu_pixel, cpu_pixel, "{format:?} pixel {index}");
                }
            }
            assert_within_levels(&mapped, &cpu, 1);
        }
    }
}

#[test]
fn a_slot_holding_the_same_shared_frame_is_not_uploaded_again() {
    let Some(gpu) = headless() else {
        return;
    };
    let (width, height) = (16, 8);
    let base = gradient(1, 16, 8, VideoPixelFormat::Rgba8, 5);
    let top = gradient(2, 16, 8, VideoPixelFormat::Rgba8, 60);
    let scene = |top: &Arc<VideoFrame>| {
        composition(
            vec![
                (
                    layer_plan(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
                    Arc::clone(&base),
                ),
                (
                    layer_plan(2, VideoBlendMode::Normal, 0.5, Transform2D::default()),
                    Arc::clone(top),
                ),
            ],
            VideoOutputMapping::default(),
        )
    };
    let uploaded = |buffers: &NativeGpuFrameBuffers| {
        buffers
            .composition_slots
            .iter()
            .map(|slot| slot.uploaded.as_ref().map(Arc::as_ptr))
            .collect::<Vec<_>>()
    };
    let mut buffers = new_buffers(&gpu, (width, height));
    let first = scene(&top);
    let (mapped, _) = render(&gpu, &mut buffers, &first, width, height);
    assert_within_levels(&mapped, &cpu_reference(&first, width, height), 2);
    assert_eq!(
        uploaded(&buffers),
        vec![Some(Arc::as_ptr(&base)), Some(Arc::as_ptr(&top))]
    );

    // The same frames again: nothing is reallocated and the result holds.
    let reallocations = buffers.layer_reallocations;
    let (mapped, _) = render(&gpu, &mut buffers, &first, width, height);
    assert_within_levels(&mapped, &cpu_reference(&first, width, height), 2);
    assert_eq!(buffers.layer_reallocations, reallocations);

    // A new frame replaces only its own slot.
    let next = gradient(2, 16, 8, VideoPixelFormat::Rgba8, 200);
    let second = scene(&next);
    let (mapped, _) = render(&gpu, &mut buffers, &second, width, height);
    assert_within_levels(&mapped, &cpu_reference(&second, width, height), 2);
    assert_ne!(mapped, cpu_reference(&first, width, height));
    assert_eq!(
        uploaded(&buffers),
        vec![Some(Arc::as_ptr(&base)), Some(Arc::as_ptr(&next))]
    );

    // A larger output grows the lookups and replaces the composition
    // buffer; the slots follow it.
    let outputs = buffers.output_reallocations;
    let (mapped, _) = render(&gpu, &mut buffers, &second, 64, 40);
    assert!(buffers.output_reallocations > outputs);
    assert_within_levels(&mapped, &cpu_reference(&second, 64, 40), 2);

    // Fewer layers drop the unused slot, and forgetting a failed submission
    // lets go of the frames without dropping the reusable buffers.
    let alone = composition(
        vec![(
            layer_plan(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
            Arc::clone(&base),
        )],
        VideoOutputMapping::default(),
    );
    let (mapped, _) = render(&gpu, &mut buffers, &alone, width, height);
    assert_within_levels(&mapped, &cpu_reference(&alone, width, height), 1);
    assert_eq!(uploaded(&buffers), vec![Some(Arc::as_ptr(&base))]);
    buffers.forget_composition_uploads();
    assert_eq!(uploaded(&buffers), vec![None]);
    gpu.queue.write_buffer(
        &buffers.composition_slots[0].source,
        0,
        &vec![0; base.data.len()],
    );
    let allocations = buffers.layer_reallocations;
    let (mapped, _) = render(&gpu, &mut buffers, &alone, width, height);
    assert_eq!(buffers.layer_reallocations, allocations);
    assert_within_levels(&mapped, &cpu_reference(&alone, width, height), 1);
    buffers.forget_composition_uploads();
    drop((first, second, alone));
    assert_eq!(Arc::strong_count(&base), 1);
}

#[test]
fn compressed_frames_count_only_when_uploaded() {
    let Some(gpu) = headless() else {
        return;
    };
    let (width, height) = (8, 4);
    let dxt1 = dxt_frame(1, (8, 4), VideoPixelFormat::Dxt1, 3);
    let scene = composition(
        vec![
            (
                layer_plan(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
                Arc::clone(&dxt1),
            ),
            (
                layer_plan(2, VideoBlendMode::Add, 0.5, Transform2D::default()),
                gradient(2, 8, 4, VideoPixelFormat::Rgba8, 9),
            ),
        ],
        VideoOutputMapping::default(),
    );
    let mut buffers = new_buffers(&gpu, (width, height));
    let (mapped, compressed) = render(&gpu, &mut buffers, &scene, width, height);
    assert_within_levels(&mapped, &cpu_reference(&scene, width, height), 2);
    assert_eq!(compressed, 1);
    let (_, compressed) = render(&gpu, &mut buffers, &scene, width, height);
    assert_eq!(compressed, 0);
}

fn percentiles(mut samples: Vec<std::time::Duration>) -> (f64, f64) {
    samples.sort();
    let at = |fraction: f64| {
        samples[((samples.len() - 1) as f64 * fraction).round() as usize].as_secs_f64() * 1_000.0
    };
    (at(0.5), at(0.95))
}

fn wait_idle(gpu: &Headless) {
    gpu.device.poll(wgpu::PollType::wait()).unwrap();
}

fn benchmark_frame(
    layer_id: VideoLayerId,
    (width, height): (u32, u32),
    format: VideoPixelFormat,
    seed: u8,
) -> Arc<VideoFrame> {
    match format {
        VideoPixelFormat::Dxt1 => {
            let blocks = width.div_ceil(4) as usize * height.div_ceil(4) as usize;
            let data = (0..blocks)
                .flat_map(|index| {
                    let value = (index as u16)
                        .wrapping_mul(97)
                        .wrapping_add(u16::from(seed));
                    let mut block = [0_u8; 8];
                    // colour0 > colour1: four opaque colours.
                    block[0..2].copy_from_slice(&(value | 0x8000).to_le_bytes());
                    block[2..4].copy_from_slice(&(value & 0x7fff).to_le_bytes());
                    block[4..8].copy_from_slice(&0xE4E4_E4E4u32.to_le_bytes());
                    block
                })
                .collect();
            Arc::new(VideoFrame {
                layer_id,
                width,
                height,
                pts_ms: u64::from(seed),
                duration_ms: 33,
                format,
                data,
            })
        }
        // Decoded video is opaque.
        _ => {
            let mut frame = gradient(layer_id, width, height, format, seed);
            let pixels = Arc::get_mut(&mut frame).expect("a new frame");
            for pixel in pixels.data.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            frame
        }
    }
}

/// The Display's two paths at the sizes the GPU path is for, without a
/// surface. CPU path: composite on the CPU, keep the last-valid copy, upload
/// the composite and map it (`present_output_artistic_result`). GPU path:
/// upload the layers that changed with their lookups, composite and map on
/// the GPU (`present_output_gpu_composition`). Clips advance every other
/// tick, as 30 fps media on a 60 Hz output. "Submit" is the render thread's
/// time; "done" waits for the GPU as well.
#[test]
#[ignore = "benchmark: cargo test --release -p video --features libav --lib gpu_composition_benchmark -- --ignored --nocapture"]
fn gpu_composition_benchmark() {
    let Some(gpu) = headless() else {
        return;
    };
    let uhd = (3_840, 2_160);
    let hd = (1_920, 1_080);
    let scenarios = [
        ("4K out, 2 x 4K RGBA", uhd, 2, VideoPixelFormat::Rgba8, uhd),
        ("4K out, 4 x 4K RGBA", uhd, 4, VideoPixelFormat::Rgba8, uhd),
        ("4K out, 8 x 4K RGBA", uhd, 8, VideoPixelFormat::Rgba8, uhd),
        ("4K out, 4 x 1080p DXT1", uhd, 4, VideoPixelFormat::Dxt1, hd),
        (
            "1080p out, 6 x 1080p RGBA",
            hd,
            6,
            VideoPixelFormat::Rgba8,
            hd,
        ),
        (
            "1080p out, 8 x 1080p DXT1",
            hd,
            8,
            VideoPixelFormat::Dxt1,
            hd,
        ),
    ];
    const WARMUP: usize = 8;
    const TICKS: usize = 120;
    for (name, (width, height), layer_count, format, source) in scenarios {
        let plans = (1..=layer_count as VideoLayerId)
            .map(|id| match id {
                1 => layer_plan(id, VideoBlendMode::Normal, 1.0, Transform2D::default()),
                _ if id % 2 == 0 => {
                    layer_plan(id, VideoBlendMode::Screen, 0.6, Transform2D::default())
                }
                _ => layer_plan(id, VideoBlendMode::Add, 0.4, Transform2D::default()),
            })
            .collect::<Vec<_>>();
        let sets = [11_u8, 173].map(|seed| {
            plans
                .iter()
                .map(|plan| {
                    benchmark_frame(
                        plan.layer_id,
                        source,
                        format,
                        seed.wrapping_add(plan.layer_id as u8),
                    )
                })
                .collect::<Vec<_>>()
        });
        let scene = |tick: usize| {
            composition(
                plans
                    .iter()
                    .cloned()
                    .zip(sets[(tick / 2) % 2].iter().cloned())
                    .collect(),
                VideoOutputMapping::default(),
            )
        };
        let size = u64::from(width) * u64::from(height) * 4;
        let mut buffers = new_buffers(&gpu, (width, height));

        let (mut cpu_submit, mut cpu_done) = (Vec::new(), Vec::new());
        for tick in 0..WARMUP + TICKS {
            let scene = scene(tick);
            let started = std::time::Instant::now();
            let composed = cpu_reference_unmapped(&scene, width, height);
            let last_valid = composed.clone();
            gpu.queue
                .write_buffer(&buffers.composition, 0, &composed.data);
            let mapping = &scene.output_mapping;
            write_mapping_mask_texture(&gpu.queue, &buffers.mapping_dummy_texture, mapping);
            gpu.queue.write_buffer(
                &buffers.mapping_params,
                0,
                &output_mapping_params(width, height, mapping),
            );
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            encoder.clear_buffer(&buffers.mapped, 0, Some(size));
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&gpu.pipelines.output_mapping);
                pass.set_bind_group(0, &buffers.mapping_bind_group, &[]);
                let (x, y) = dispatch_dimensions(width, height);
                pass.dispatch_workgroups(x, y, 1);
            }
            gpu.queue.submit(Some(encoder.finish()));
            let submitted = started.elapsed();
            wait_idle(&gpu);
            if tick >= WARMUP {
                cpu_submit.push(submitted);
                cpu_done.push(started.elapsed());
            }
            drop(last_valid);
        }

        let (mut gpu_submit, mut gpu_done) = (Vec::new(), Vec::new());

        for tick in 0..WARMUP + TICKS {
            let scene = scene(tick);
            let started = std::time::Instant::now();
            let (encoder, _) = encode(&gpu, &mut buffers, &scene, (width, height));
            gpu.queue.submit(Some(encoder.finish()));
            let submitted = started.elapsed();
            wait_idle(&gpu);
            if tick >= WARMUP {
                gpu_submit.push(submitted);
                gpu_done.push(started.elapsed());
            }
        }
        let (cpu_submit, cpu_done) = (percentiles(cpu_submit), percentiles(cpu_done));
        let (gpu_submit, gpu_done) = (percentiles(gpu_submit), percentiles(gpu_done));
        eprintln!(
            "BENCH {name}: CPU path submit {:.1}/{:.1} ms, done {:.1}/{:.1} ms; \
             GPU path submit {:.1}/{:.1} ms, done {:.1}/{:.1} ms (p50/p95)",
            cpu_submit.0,
            cpu_submit.1,
            cpu_done.0,
            cpu_done.1,
            gpu_submit.0,
            gpu_submit.1,
            gpu_done.0,
            gpu_done.1,
        );
    }
}
