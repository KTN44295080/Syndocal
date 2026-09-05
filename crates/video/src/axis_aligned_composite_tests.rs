use super::*;

fn frame(width: u32, height: u32, opaque: bool) -> VideoFrame {
    let data = (0..width as usize * height as usize * 4)
        .map(|i| {
            if opaque && i % 4 == 3 {
                255
            } else {
                (i * 73 + i / 4 * 17) as u8
            }
        })
        .collect();
    VideoFrame {
        layer_id: 1,
        width,
        height,
        pts_ms: 1,
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data,
    }
}

fn run(
    destination: &mut [u8],
    source: &VideoFrame,
    width: u32,
    height: u32,
    transform: &Transform2D,
    generic: bool,
) {
    let params = BlendTransformedRgba8Params {
        destination,
        frame: source,
        width,
        height,
        opacity: 1.0,
        blend_mode: &VideoBlendMode::Normal,
        transform,
        color: &VideoColorAdjust::default(),
        fx: &VideoFxAdjust::default(),
    };
    if generic {
        blend_transformed_rgba8_generic(params);
    } else {
        blend_transformed_rgba8(params);
    }
}

#[test]
fn axis_aligned_composite_matches_generic_sampling_and_alpha() {
    for (source_width, source_height, width, height) in [
        (7, 3, 13, 9),
        (16, 8, 32, 18),
        (19, 11, 8, 6),
        (256, 1, 256, 1),
    ] {
        for opaque in [false, true] {
            let source = frame(source_width, source_height, opaque);
            for transform in [
                Transform2D::default(),
                Transform2D {
                    scale_y: 32.0 / 45.0,
                    ..Default::default()
                },
                Transform2D {
                    x: 0.13,
                    y: -0.17,
                    scale_x: 1.3,
                    scale_y: 0.71,
                    ..Default::default()
                },
                Transform2D {
                    crop_left: 0.2,
                    crop_right: 0.1,
                    crop_top: 0.1,
                    crop_bottom: 0.3,
                    ..Default::default()
                },
                Transform2D {
                    scale_x: -1.0,
                    scale_y: 0.0,
                    ..Default::default()
                },
                Transform2D {
                    rotation_deg: 23.0,
                    ..Default::default()
                },
            ] {
                for filled in [false, true] {
                    let mut expected = if filled {
                        frame(width, height, false).data
                    } else {
                        vec![0; width as usize * height as usize * 4]
                    };
                    let mut actual = expected.clone();
                    run(&mut expected, &source, width, height, &transform, true);
                    run(&mut actual, &source, width, height, &transform, false);
                    assert_eq!(actual, expected, "{source_width}x{source_height} -> {width}x{height}, {transform:?}, opaque={opaque}, filled={filled}");
                }
            }
        }
    }
}

#[test]
fn axis_aligned_composite_default_color_is_byte_identity() {
    for value in 0..=255 {
        let pixel = [value, value, value, value];
        assert_eq!(
            adjust_rgba_pixel(&pixel, &VideoColorAdjust::default()),
            pixel
        );
    }
}

#[test]
#[ignore = "release-only synthetic renderer cost measurement; no devices or output"]
fn axis_aligned_composite_release_cost() {
    for (width, height) in [(3840, 2160), (1920, 1080)] {
        let source = frame(width, width * 2 / 5, true);
        let transform = Transform2D {
            scale_y: 32.0 / 45.0,
            ..Default::default()
        };
        let mut destination = vec![0; width as usize * height as usize * 4];
        for generic in [true, false] {
            let mut times = Vec::new();
            for _ in 0..8 {
                let started = std::time::Instant::now();
                run(
                    &mut destination,
                    &source,
                    width,
                    height,
                    &transform,
                    generic,
                );
                std::hint::black_box(&destination);
                times.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            eprintln!("axis_aligned_composite {width}x{height} generic={generic} median_ms={:.3} max_ms={:.3}", times[4], times[7]);
        }
    }
}

#[test]
fn axis_aligned_composite_ineligible_adjustments_retain_generic() {
    let source = frame(17, 9, false);
    for mode in [
        VideoBlendMode::Normal,
        VideoBlendMode::Add,
        VideoBlendMode::Multiply,
        VideoBlendMode::Screen,
    ] {
        for opacity in [0.0, 0.37, 1.0] {
            for (color, fx, transform) in [
                (
                    VideoColorAdjust::default(),
                    VideoFxAdjust::default(),
                    Transform2D::default(),
                ),
                (
                    VideoColorAdjust {
                        gamma: 1.3,
                        ..Default::default()
                    },
                    VideoFxAdjust::default(),
                    Transform2D::default(),
                ),
                (
                    VideoColorAdjust::default(),
                    VideoFxAdjust {
                        pixelate: 2.0,
                        ..Default::default()
                    },
                    Transform2D::default(),
                ),
                (
                    VideoColorAdjust::default(),
                    VideoFxAdjust::default(),
                    Transform2D {
                        x: f32::NAN,
                        ..Default::default()
                    },
                ),
                (
                    VideoColorAdjust::default(),
                    VideoFxAdjust::default(),
                    Transform2D {
                        scale_y: f32::INFINITY,
                        ..Default::default()
                    },
                ),
                (
                    VideoColorAdjust::default(),
                    VideoFxAdjust::default(),
                    Transform2D {
                        crop_top: f32::NAN,
                        ..Default::default()
                    },
                ),
            ] {
                let mut expected = frame(23, 13, false).data;
                let mut actual = expected.clone();
                for (destination, generic) in [(&mut expected, true), (&mut actual, false)] {
                    let params = BlendTransformedRgba8Params {
                        destination,
                        frame: &source,
                        width: 23,
                        height: 13,
                        opacity,
                        blend_mode: &mode,
                        transform: &transform,
                        color: &color,
                        fx: &fx,
                    };
                    if generic {
                        blend_transformed_rgba8_generic(params);
                    } else {
                        blend_transformed_rgba8(params);
                    }
                }
                assert_eq!(actual, expected);
            }
        }
    }
}
