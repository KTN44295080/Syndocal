use super::*;

fn input(generator: VideoGenerator, position_ms: u64, width: u32, height: u32) -> VideoRenderInput {
    VideoRenderInput::legacy(VideoFrameRequest {
        layer_id: 7,
        label: "Generator".into(),
        source: VideoSourceSummary::from_generator(generator).unwrap(),
        position_ms,
        width,
        height,
    })
}

fn noise_generator(speed: u16) -> VideoGenerator {
    VideoGenerator::Noise {
        seed: 12345,
        scale: 5,
        speed,
        color_a: [0, 30, 80, 20],
        color_b: [255, 200, 0, 255],
    }
}

#[test]
fn solid_and_gradient_have_fixed_rgba_bytes_and_angles() {
    let mut cache = GeneratorFrameCache::default();
    let solid = VideoGenerator::Solid {
        color: [23, 42, 199, 75],
    };
    let frame = cache.frame(&input(solid, 500, 3, 2), solid, None).unwrap();
    assert_eq!(frame.data, [23, 42, 199, 75].repeat(6));
    for (angle, expected) in [
        (0, vec![0, 128, 255, 0, 128, 255]),
        (90, vec![0, 0, 0, 255, 255, 255]),
        (180, vec![255, 128, 0, 255, 128, 0]),
        (270, vec![255, 255, 255, 0, 0, 0]),
    ] {
        let generator = VideoGenerator::LinearGradient {
            color_a: [0, 0, 0, 0],
            color_b: [255, 255, 255, 255],
            angle_degrees: angle,
        };
        let frame = cache
            .frame(&input(generator, 0, 3, 2), generator, None)
            .unwrap();
        assert_eq!(
            frame.data,
            expected
                .into_iter()
                .flat_map(|v| [v; 4])
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn noise_is_reproducible_and_quantized_by_layer_time_not_call_order() {
    let generator = noise_generator(100);
    let mut first = GeneratorFrameCache::default();
    let mut second = GeneratorFrameCache::default();
    let a = first
        .frame(&input(generator, 1000, 13, 7), generator, None)
        .unwrap();
    let same_bucket = first
        .frame(&input(generator, 1033, 13, 7), generator, None)
        .unwrap();
    assert!(Arc::ptr_eq(&a, &same_bucket));
    let b = second
        .frame(&input(generator, 1000, 13, 7), generator, None)
        .unwrap();
    assert_eq!(a.data, b.data);
    assert_eq!((a.pts_ms, a.duration_ms), (1000, 34));
    let next = first
        .frame(&input(generator, 1034, 13, 7), generator, None)
        .unwrap();
    assert_ne!(a.data, next.data);
    let rewind = first
        .frame(&input(generator, 1000, 13, 7), generator, None)
        .unwrap();
    assert_eq!(a.data, rewind.data);
    let static_generator = noise_generator(0);
    let a = first
        .frame(&input(static_generator, 0, 13, 7), static_generator, None)
        .unwrap();
    let b = first
        .frame(
            &input(static_generator, u64::MAX, 13, 7),
            static_generator,
            None,
        )
        .unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn noise_hash_and_smoothstep_are_explicit_integer_golden_values() {
    assert_eq!(smooth(0), 0);
    assert_eq!(smooth(32768), 32768);
    assert_eq!(smooth(65535), 65535);
    assert_eq!(hash(0, 0, 0, 0), 0);
    // Includes all eight spatial/temporal corners, plus RGBA interpolation.
    let bytes = render(noise_generator(100), 3, 2, 15, None).unwrap();
    assert_eq!(bytes.len(), 24);
    // A stable FNV1a checksum makes byte output independent of a second renderer.
    let checksum = bytes.iter().fold(2166136261u32, |hash, byte| {
        (hash ^ *byte as u32).wrapping_mul(16777619)
    });
    assert_eq!(checksum, 1397938806);
}

#[test]
fn static_cache_reuses_storage_and_changes_parameters_size_and_ownership() {
    let generator = VideoGenerator::Solid {
        color: [10, 20, 30, 255],
    };
    let mut cache = GeneratorFrameCache::default();
    let a = cache
        .frame(&input(generator, 0, 16, 9), generator, None)
        .unwrap();
    let b = cache
        .frame(&input(generator, 9999, 16, 9), generator, None)
        .unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    let resized = cache
        .frame(&input(generator, 0, 32, 18), generator, None)
        .unwrap();
    assert_eq!(resized.data.len(), 32 * 18 * 4);
    assert!(!Arc::ptr_eq(&a, &resized));
    let changed = VideoGenerator::Solid {
        color: [11, 20, 30, 255],
    };
    let changed_frame = cache
        .frame(&input(changed, 0, 16, 9), changed, None)
        .unwrap();
    assert_ne!(a.data, changed_frame.data);
    let gradient = VideoGenerator::LinearGradient {
        color_a: [0; 4],
        color_b: [255; 4],
        angle_degrees: 37,
    };
    let a = cache
        .frame(&input(gradient, 0, 16, 9), gradient, None)
        .unwrap();
    let b = cache
        .frame(&input(gradient, u64::MAX, 16, 9), gradient, None)
        .unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    for id in 1..=40 {
        let mut request = input(generator, 0, 2, 2);
        request.request.layer_id = id;
        cache.frame(&request, generator, None).unwrap();
    }
    assert_eq!(cache.entries.len(), CACHE_ENTRIES);
    cache.retain_layers(&[40]);
    assert_eq!(cache.entries.len(), 1);
    cache.retain_inputs(&[]);
    assert!(cache.entries.is_empty());
}

#[test]
fn invalid_generators_fail_closed_instead_of_decoding_or_showing_placeholder() {
    let mut provider = DecoderBackedFrameProvider::new(NullVideoDecoder);
    let generator = VideoGenerator::Solid {
        color: [0, 0, 0, 255],
    };
    let mut request = input(generator, 0, 0, 18);
    assert!(matches!(
        provider.frame_for_input(&request),
        Err(VideoFrameProviderError::Decode { .. })
    ));
    request.request.width = 32;
    request.request.source.path = Some("should-not-decode.mp4".into());
    assert!(matches!(
        provider.frame_for_input(&request),
        Err(VideoFrameProviderError::Decode { .. })
    ));
    request.request.source.path = None;
    request.request.source.kind = VideoSourceKind::Generator(VideoGenerator::Noise {
        seed: 1,
        scale: 0,
        speed: 100,
        color_a: [0; 4],
        color_b: [255; 4],
    });
    assert!(matches!(
        provider.frame_for_input(&request),
        Err(VideoFrameProviderError::Decode { .. })
    ));
}

#[test]
fn both_providers_use_identical_full_size_rgba_and_bypass_decoder_prefetch() {
    let mut preview = PreviewFrameProvider::new();
    let mut decoded = DecoderBackedFrameProvider::new(NullVideoDecoder)
        .with_placeholder_when_missing(false)
        .with_prefetch(3, 33);
    for generator in [
        VideoGenerator::Solid {
            color: [10, 20, 30, 255],
        },
        VideoGenerator::LinearGradient {
            color_a: [0; 4],
            color_b: [255; 4],
            angle_degrees: 37,
        },
        noise_generator(100),
    ] {
        let request = input(generator, 1379, 71, 39);
        decoded.decode_inputs_ahead(&[request.clone(), input(generator, 3000, 71, 39)]);
        assert!(decoded.decoded_ahead.is_empty());
        let a = preview.frame_for_input(&request).unwrap();
        let b = decoded.frame_for_input(&request).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.format, VideoPixelFormat::Rgba8);
        assert_eq!(a.data.len(), 71 * 39 * 4);
    }
}

#[test]
fn shared_live_provider_path_keeps_cached_frame_storage_and_honors_cancellation() {
    let generator = VideoGenerator::Solid {
        color: [10, 20, 30, 255],
    };
    let layer = VideoLayerSummary {
        id: 7,
        label: "Generated".into(),
        source: VideoSourceSummary::from_generator(generator).unwrap(),
        media_asset_id: None,
        blend_mode: VideoBlendMode::Normal,
        state: VideoLayerState::default(),
        isf_effect: None,
        clip_slots: Vec::new(),
        default_clip_slot_id: None,
    };
    let mut provider = DecoderBackedFrameProvider::new(NullVideoDecoder).with_prefetch(4, 33);
    let a = provider.frames_for_layer_shared(&layer, 32, 18).unwrap();
    let b = provider.frames_for_layer_shared(&layer, 32, 18).unwrap();
    assert_eq!(a.len(), 1);
    assert!(Arc::ptr_eq(&a[0], &b[0]));
    let running = || false;
    let c = provider
        .frames_for_layer_shared_with_cancellation(&layer, 32, 18, &running)
        .unwrap();
    assert!(Arc::ptr_eq(&a[0], &c[0]));
    let mut preview = PreviewFrameProvider::new();
    let a = preview.frames_for_layer_shared(&layer, 32, 18).unwrap();
    let b = preview.frames_for_layer_shared(&layer, 32, 18).unwrap();
    assert!(Arc::ptr_eq(&a[0], &b[0]));
    let c = preview
        .frames_for_layer_shared_with_cancellation(&layer, 32, 18, &running)
        .unwrap();
    assert!(Arc::ptr_eq(&a[0], &c[0]));
    let cancelled = || true;
    assert_eq!(
        preview.frames_for_layer_shared_with_cancellation(&layer, 32, 18, &cancelled),
        Err(VideoFrameProviderError::Cancelled)
    );
    assert_eq!(
        provider.frames_for_layer_shared_with_cancellation(&layer, 32, 18, &cancelled),
        Err(VideoFrameProviderError::Cancelled)
    );
    assert_eq!(
        provider.frame_for_input_with_cancellation(&input(generator, 0, 32, 18), &cancelled),
        Err(VideoFrameProviderError::Cancelled)
    );
    let checks = std::cell::Cell::new(0);
    let cancel_mid_frame = || {
        checks.set(checks.get() + 1);
        checks.get() > 4
    };
    let mut cache = GeneratorFrameCache::default();
    assert_eq!(
        cache.frame(
            &input(noise_generator(100), 0, 1920, 1080),
            noise_generator(100),
            Some(&cancel_mid_frame)
        ),
        Err(VideoFrameProviderError::Cancelled)
    );
    assert!(cache.entries.is_empty());
}

#[test]
fn noise_handles_extreme_timestamps_and_thin_outputs_with_identical_bytes() {
    let generator = VideoGenerator::Noise {
        seed: u32::MAX,
        scale: 256,
        speed: 1000,
        color_a: [0, 255, 0, 0],
        color_b: [255, 0, 255, 255],
    };
    for (width, height) in [(1, 1), (1, 1024), (1024, 1)] {
        let request = input(generator, u64::MAX, width, height);
        let a = GeneratorFrameCache::default()
            .frame(&request, generator, None)
            .unwrap();
        let b = GeneratorFrameCache::default()
            .frame(&request, generator, None)
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.data.len(), width as usize * height as usize * 4);
        assert!(a.pts_ms <= request.request.position_ms);
        assert!((request.request.position_ms as u128) < a.pts_ms as u128 + a.duration_ms as u128);
    }
}

#[test]
fn generator_frames_pass_the_regular_transition_operators() {
    let mut provider = PreviewFrameProvider::new();
    let outgoing = provider
        .frame_for_input(&input(
            VideoGenerator::Solid {
                color: [255, 0, 0, 255],
            },
            300,
            16,
            9,
        ))
        .unwrap();
    let incoming = provider
        .frame_for_input(&input(
            VideoGenerator::Solid {
                color: [0, 0, 255, 255],
            },
            0,
            16,
            9,
        ))
        .unwrap();
    for kind in [
        protocol::VideoClipTakeKind::Crossfade,
        protocol::VideoClipTakeKind::Dip,
        protocol::VideoClipTakeKind::Wipe,
        protocol::VideoClipTakeKind::Blur,
    ] {
        let frame = transition_video_frames_rgba8(&outgoing, &incoming, kind, 500).unwrap();
        assert_eq!((frame.width, frame.height), (16, 9));
        assert_eq!(frame.format, VideoPixelFormat::Rgba8);
        assert_ne!(frame.data, outgoing.data);
        assert_ne!(frame.data, incoming.data);
    }
}

#[test]
fn keyed_input_generator_edits_retire_older_animated_queue_frames() {
    let key = protocol::VideoRenderInputKey {
        project_render_epoch: 1,
        input_id: protocol::VideoRenderInputId(5),
    };
    let keyed = |generator| VideoRenderInput::new(key, input(generator, 1234, 8, 4).request);
    let mix = CompositionInputMix {
        inputs: vec![CompositionRenderInput {
            key,
            layer: CompositionLayerPlan {
                layer_id: 7,
                label: "Generator".into(),
                source: VideoSourceSummary::from_generator(noise_generator(100)).unwrap(),
                blend_mode: VideoBlendMode::Normal,
                opacity: 1.0,
                position_ms: 1234,
                transform: Transform2D::default(),
                color: VideoColorAdjust::default(),
                fx: VideoFxAdjust::default(),
            },
        }],
    };
    let snapshot = VideoSnapshot::default();
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        audio_frame: None,
        clip_runtime: &clip_runtime,
        project_render_epoch: 1,
    };
    let solid = VideoGenerator::Solid {
        color: [200, 10, 40, 255],
    };
    for with_effects in [false, true] {
        let render = |renderer: &mut VideoPreviewRenderer, generator| {
            let inputs = [keyed(generator)];
            if with_effects {
                renderer.render_input_mix_with_effects(&snapshot, context, &inputs, &mix, 8, 4)
            } else {
                renderer.render_input_mix(&inputs, &mix, 8, 4)
            }
            .unwrap()
        };
        let mut reused = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
        let animated = render(&mut reused, noise_generator(100));
        let replaced = render(&mut reused, solid);
        let fresh = render(
            &mut VideoPreviewRenderer::new(VideoRuntimeConfig::default()),
            solid,
        );
        assert_ne!(animated.data, fresh.data);
        assert_eq!(replaced.data, fresh.data, "with_effects={with_effects}");
    }
}
#[test]
#[ignore = "opt-in release performance measurement"]
fn generator_noise_cost_measurement() {
    let generator = noise_generator(100);
    for (width, height) in [(1920, 1080), (3840, 2160)] {
        let mut cache = GeneratorFrameCache::default();
        let mut milliseconds = Vec::new();
        for frame in 0..34 {
            let request = input(generator, frame * 34, width, height);
            let start = std::time::Instant::now();
            let output = cache.frame(&request, generator, None).unwrap();
            std::hint::black_box(output);
            if frame >= 4 {
                milliseconds.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        milliseconds.sort_by(f64::total_cmp);
        eprintln!("generator noise {width}x{height}: 30 frames mean {:.3}ms p50 {:.3}ms p95 {:.3}ms max {:.3}ms",
            milliseconds.iter().sum::<f64>() / milliseconds.len() as f64,
            milliseconds[15], milliseconds[28], milliseconds[29]);
        for generator in [
            VideoGenerator::Solid {
                color: [1, 2, 3, 255],
            },
            VideoGenerator::LinearGradient {
                color_a: [0; 4],
                color_b: [255; 4],
                angle_degrees: 37,
            },
        ] {
            let request = input(generator, 0, width, height);
            let start = std::time::Instant::now();
            let original = cache.frame(&request, generator, None).unwrap();
            let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                let frame = cache.frame(&request, generator, None).unwrap();
                assert!(Arc::ptr_eq(&original, &frame));
                std::hint::black_box(frame);
            }
            eprintln!("generator {generator:?} {width}x{height}: cold {cold_ms:.3}ms, cached mean {:.3}us (1000 hits, identical Arc)",
                start.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
