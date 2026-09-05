use crate::*;

fn fixture(foreground: bool) -> (VideoOutputSummary, CompositionPlan) {
    let output = VideoOutputSummary {
        id: 1, label: "Show".into(), kind: VideoOutputKind::SpoutSender, enabled: true,
        composition_id: 1, fullscreen: false, monitor_id: None, monitor_identity: None,
        width: if foreground { 3840 } else { 1920 }, height: if foreground { 2160 } else { 1080 },
        endpoint_name: Some(if foreground { "Syndocal Foreground" } else { "Syndocal Background" }.into()),
        opacity: 1.0, blackout: false, mapping: VideoOutputMapping::default(),
    };
    let composition = CompositionPlan {
        composition_id: 1, label: "Show".into(), output_ids: vec![1], master_opacity: 1.0, blackout: false,
        layers: vec![CompositionLayerPlan {
            layer_id: 1, label: "5:2".into(),
            source: VideoSourceSummary { kind: VideoSourceKind::File, path: Some("wide.mp4".into()), name: None, codec: None,
                metadata: Some(VideoMediaMetadata { width: Some(2500), height: Some(1000), duration_ms: None, frame_rate: None, has_audio: false }) },
            blend_mode: VideoBlendMode::Normal, opacity: 1.0, position_ms: 0,
            transform: Transform2D::default(), color: VideoColorAdjust::default(), fx: VideoFxAdjust::default(),
        }],
    };
    (output, composition)
}

#[test]
fn show_spout_aspect_fit_is_role_scoped_and_preserves_authored_transform() {
    for foreground in [false, true] {
        let (output, mut composition) = fixture(foreground);
        composition.layers[0].transform = Transform2D { x: 0.13, y: -0.2, scale_x: 0.7, scale_y: 0.8,
            rotation_deg: 37.0, crop_left: 0.1, crop_top: 0.2, crop_right: 0.15, crop_bottom: 0.1 };
        let original = composition.clone();
        let plan = video_output_render_plan(output.clone(), composition).unwrap();
        let mut expected = original.layers[0].transform;
        expected.scale_y *= 32.0 / 45.0;
        assert_eq!(plan.composition.layers[0].transform, expected);
        assert_eq!(plan.mapping, output.mapping);
        for mutation in 0..3 {
            let mut excluded = output.clone();
            match mutation { 0 => excluded.kind = VideoOutputKind::Display,
                1 => excluded.endpoint_name = Some("Syndocal Foreground_1".into()), _ => excluded.width += 1 }
            assert_eq!(video_output_render_plan(excluded, original.clone()).unwrap().composition, original);
        }
    }
}

#[test]
fn show_spout_aspect_fit_rejects_unknown_dimensions_except_nonvisible_sources() {
    for zero in [false, true] {
        let (output, mut composition) = fixture(true);
        if zero { composition.layers[0].source.metadata.as_mut().unwrap().height = Some(0); }
        else { composition.layers[0].source.metadata = None; }
        assert_eq!(video_output_render_plan(output.clone(), composition.clone()).unwrap_err(),
            VideoOutputRenderError::ShowSpoutSourceDimensionsUnavailable { output_id: 1, layer_id: 1 });
        let mut blackout = output.clone(); blackout.blackout = true;
        assert!(video_output_render_plan(blackout, composition.clone()).unwrap().composition.layers.is_empty());
        composition.layers[0].opacity = 0.0;
        assert!(video_output_render_plan(output, composition).unwrap().composition.layers.is_empty());
    }
}

fn patterned_frame() -> VideoFrame {
    // Exact 5:2 image with individually identifiable source quadrants.
    let mut data = Vec::new();
    for y in 0..40 { for x in 0..100 {
        data.extend_from_slice(&[if x < 50 { 240 } else { 20 }, if y < 20 { 220 } else { 30 }, 90, 255]);
    }}
    VideoFrame { layer_id: 1, width: 100, height: 40, pts_ms: 123, duration_ms: 33, format: VideoPixelFormat::Rgba8, data }
}

fn assert_fit_pixels(frame: &VideoFrame) {
    let pixel = |x: usize, y: usize| &frame.data[(y * 160 + x) * 4..(y * 160 + x + 1) * 4];
    for y in [0, 12, 77, 89] { for x in [0, 40, 159] { assert_eq!(&pixel(x, y)[..3], &[0, 0, 0]); }}
    // The central 160x64 (5:2) receiver crop retains every source quadrant.
    assert_eq!(pixel(0, 13), &[240, 220, 90, 255]);
    assert_eq!(pixel(159, 13), &[20, 220, 90, 255]);
    assert_eq!(pixel(0, 76), &[240, 30, 90, 255]);
    assert_eq!(pixel(159, 76), &[20, 30, 90, 255]);
}

#[test]
fn show_spout_aspect_fit_cpu_preserves_black_bars_and_receiver_center_crop() {
    for foreground in [false, true] {
        let (output, composition) = fixture(foreground);
        let plan = video_output_render_plan(output, composition).unwrap();
        let rendered = composite_rgba8(&plan.composition, &[patterned_frame()], 160, 90).unwrap();
        assert_fit_pixels(&rendered);
    }
}

#[test]
#[ignore = "requires a real GPU adapter; run explicitly in the native gate"]
fn show_spout_aspect_fit_gpu_matches_cpu_pattern_and_transformed_source() {
    let gpu = GpuCompositor::new().expect("GPU adapter required");
    for foreground in [false, true] { for transformed in [false, true] {
        let (output, mut composition) = fixture(foreground);
        if transformed {
            composition.layers[0].transform = Transform2D { x: 0.05, y: -0.1, scale_x: 0.7, scale_y: 0.8,
                rotation_deg: 37.0, crop_left: 0.1, crop_top: 0.2, crop_right: 0.15, crop_bottom: 0.1 };
        }
        let plan = video_output_render_plan(output, composition).unwrap();
        let frames = [patterned_frame()];
        let cpu = composite_rgba8(&plan.composition, &frames, 160, 90).unwrap();
        let actual = gpu.composite_rgba8(&plan.composition, &frames, 160, 90).unwrap();
        for image in [&cpu, &actual] {
            assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] != [0, 0, 0]),
                "parity must include rendered source pixels");
            assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] == [0, 0, 0]),
                "fitted/transformed image must retain uncovered canvas pixels");
        }
        assert_eq!(actual.data, cpu.data);
        if !transformed { assert_fit_pixels(&actual); }
    }}
}
