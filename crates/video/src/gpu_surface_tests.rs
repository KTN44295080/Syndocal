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
        output_mapping_identity: crate::VideoOutputMappingIdentity::from_mapping(&output_mapping),
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
        resolve_output_artistic_presentation(&substituted_mapping, &presentation_contract(2, 1),),
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
