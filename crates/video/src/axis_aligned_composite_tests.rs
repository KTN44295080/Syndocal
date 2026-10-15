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

const LEGACY_BLEND_MODES: [VideoBlendMode; 4] = [
    VideoBlendMode::Normal,
    VideoBlendMode::Add,
    VideoBlendMode::Multiply,
    VideoBlendMode::Screen,
];

const BLEND_MODES: [VideoBlendMode; 10] = [
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

fn run(
    destination: &mut [u8],
    source: &VideoFrame,
    width: u32,
    height: u32,
    transform: &Transform2D,
    opacity: f32,
    blend_mode: &VideoBlendMode,
    generic: bool,
) {
    let params = BlendTransformedRgba8Params {
        destination,
        frame: source,
        width,
        height,
        opacity,
        blend_mode,
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
                    for blend_mode in &BLEND_MODES {
                        for opacity in [1.0, 0.5, 0.003, 0.0, f32::NAN] {
                            let mut expected = if filled {
                                frame(width, height, false).data
                            } else {
                                vec![0; width as usize * height as usize * 4]
                            };
                            let mut actual = expected.clone();
                            run(
                                &mut expected,
                                &source,
                                width,
                                height,
                                &transform,
                                opacity,
                                blend_mode,
                                true,
                            );
                            run(
                                &mut actual,
                                &source,
                                width,
                                height,
                                &transform,
                                opacity,
                                blend_mode,
                                false,
                            );
                            assert_eq!(actual, expected, "{source_width}x{source_height} -> {width}x{height}, {transform:?}, opaque={opaque}, filled={filled}, {blend_mode:?} at {opacity}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn banded_composite_matches_one_pass_for_every_path() {
    let source = frame(29, 17, false);
    let transforms = [
        Transform2D::default(),
        Transform2D {
            x: 0.13,
            y: -0.17,
            scale_x: 1.3,
            scale_y: 0.71,
            ..Default::default()
        },
        Transform2D {
            rotation_deg: 23.0,
            ..Default::default()
        },
    ];
    let adjustments = [
        (VideoColorAdjust::default(), VideoFxAdjust::default()),
        (
            VideoColorAdjust {
                gamma: 1.3,
                ..Default::default()
            },
            VideoFxAdjust {
                blur: 2.0,
                glow: 1.0,
                ..Default::default()
            },
        ),
    ];
    for (width, height) in [(31, 19), (8, 3), (5, 1)] {
        for transform in &transforms {
            for (color, fx) in &adjustments {
                for blend_mode in &BLEND_MODES {
                    let mut expected = frame(width, height, false).data;
                    blend_transformed_rgba8_in_bands(
                        BlendTransformedRgba8Params {
                            destination: &mut expected,
                            frame: &source,
                            width,
                            height,
                            opacity: 0.6,
                            blend_mode,
                            transform,
                            color,
                            fx,
                        },
                        Some(1),
                    );
                    for bands in [2, 3, 5, 8, 64] {
                        let mut actual = frame(width, height, false).data;
                        blend_transformed_rgba8_in_bands(
                            BlendTransformedRgba8Params {
                                destination: &mut actual,
                                frame: &source,
                                width,
                                height,
                                opacity: 0.6,
                                blend_mode,
                                transform,
                                color,
                                fx,
                            },
                            Some(bands),
                        );
                        assert_eq!(
                            actual, expected,
                            "{width}x{height}, {bands} bands, {transform:?}, {blend_mode:?}"
                        );
                    }
                }
            }
        }
    }
}

fn assert_round_clamp_matches(value: f32) {
    let expected = value.round().clamp(0.0, 255.0) as u8;
    assert_eq!(
        round_clamp_u8(value),
        expected,
        "{value:?} ({:#010x})",
        value.to_bits()
    );
}

#[test]
fn round_clamp_matches_round_then_clamp_near_every_half() {
    for special in [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
    ] {
        assert_round_clamp_matches(special);
    }
    // Every multiple of 1/1024 from -4 to 260, and the 16 floats on each
    // side of it, so every .5 boundary is crossed at full precision.
    for step in -4 * 1024..=260 * 1024 {
        let value = step as f32 / 1024.0;
        for offset in 1..=16_u32 {
            assert_round_clamp_matches(f32::from_bits(value.to_bits().wrapping_add(offset)));
            assert_round_clamp_matches(f32::from_bits(value.to_bits().wrapping_sub(offset)));
        }
        assert_round_clamp_matches(value);
    }
}

#[test]
#[ignore = "exhaustive over all 2^32 bit patterns; run in release"]
fn round_clamp_matches_round_then_clamp_for_every_f32() {
    for bits in 0..=u32::MAX {
        assert_round_clamp_matches(f32::from_bits(bits));
    }
}

#[test]
fn opaque_source_blend_table_matches_blend_pixel() {
    for blend_mode in &BLEND_MODES {
        for opacity in [1.0, 0.999, 0.5, 0.37, 0.003, 0.0, -1.0, f32::NAN] {
            let table = OpaqueSourceBlendTable::new(opacity, blend_mode);
            for destination in 0..=255_u8 {
                for source in 0..=255_u8 {
                    let alpha = destination.wrapping_mul(37);
                    let mut expected = [destination, source, destination ^ source, alpha];
                    let mut actual = expected;
                    let source_pixel = [source, destination, source.wrapping_add(destination), 255];
                    blend_pixel(&mut expected, &source_pixel, opacity, blend_mode);
                    if let Some(table) = &table {
                        table.apply(&mut actual, &source_pixel);
                    }
                    assert_eq!(
                        actual, expected,
                        "{blend_mode:?} at {opacity}, {destination} <- {source}"
                    );
                }
            }
        }
    }
}

#[test]
fn large_axis_aligned_layers_use_the_table_and_bands_and_match_generic() {
    // 480x330 with full rows covered is above BLEND_TABLE_MIN_PIXELS; the
    // scaled and off-screen layers cover less (or nothing).
    let (width, height) = (480, 330);
    let transforms = [
        Transform2D {
            x: 0.07,
            scale_x: 1.1,
            ..Default::default()
        },
        Transform2D {
            x: -0.2,
            y: 0.1,
            scale_x: 0.5,
            scale_y: 0.4,
            ..Default::default()
        },
        Transform2D {
            x: 2.0,
            ..Default::default()
        },
        Transform2D {
            y: -1.5,
            ..Default::default()
        },
    ];
    assert!(width as usize * height as usize >= BLEND_TABLE_MIN_PIXELS);
    for opaque in [true, false] {
        let source = frame(97, 61, opaque);
        for transform in &transforms {
            for blend_mode in &BLEND_MODES {
                for opacity in [1.0, 0.5] {
                    let mut expected = frame(width, height, false).data;
                    run(
                        &mut expected,
                        &source,
                        width,
                        height,
                        transform,
                        opacity,
                        blend_mode,
                        true,
                    );
                    for bands in [None, Some(3)] {
                        let mut actual = frame(width, height, false).data;
                        blend_transformed_rgba8_in_bands(
                            BlendTransformedRgba8Params {
                                destination: &mut actual,
                                frame: &source,
                                width,
                                height,
                                opacity,
                                blend_mode,
                                transform,
                                color: &VideoColorAdjust::default(),
                                fx: &VideoFxAdjust::default(),
                            },
                            bands,
                        );
                        assert!(
                            actual == expected,
                            "{transform:?}, {blend_mode:?} at {opacity}, opaque={opaque}, bands={bands:?}"
                        );
                    }
                }
            }
        }
    }
}

/// `blend_pixel` exactly as it was before `round_clamp_u8`, the lookup
/// table and the channel/alpha split.
fn previous_blend_pixel(dst: &mut [u8], src: &[u8], opacity: f32, blend_mode: &VideoBlendMode) {
    let src_alpha = (src[3] as f32 / 255.0) * opacity;
    if src_alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let dst_value = dst[channel] as f32;
        let src_value = src[channel] as f32;
        let blended = match blend_mode {
            VideoBlendMode::Normal => src_value,
            VideoBlendMode::Add => (dst_value + src_value).min(255.0),
            VideoBlendMode::Multiply => dst_value * src_value / 255.0,
            VideoBlendMode::Screen => 255.0 - ((255.0 - dst_value) * (255.0 - src_value) / 255.0),
            _ => panic!("the historical reference covers only legacy modes"),
        };
        dst[channel] = (dst_value + (blended - dst_value) * src_alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    let dst_alpha = dst[3] as f32 / 255.0;
    dst[3] = ((src_alpha + dst_alpha * (1.0 - src_alpha)) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
}

#[test]
fn blend_pixel_matches_the_previous_implementation() {
    for blend_mode in &LEGACY_BLEND_MODES {
        for opacity in [1.0, 0.5, 0.37, 0.0, f32::NAN] {
            for source_alpha in [0_u8, 1, 127, 128, 254, 255] {
                for destination in 0..=255_u8 {
                    for source in 0..=255_u8 {
                        let pixel = [
                            destination,
                            source,
                            destination ^ source,
                            destination.wrapping_mul(29),
                        ];
                        let source_pixel =
                            [source, destination, source.wrapping_add(7), source_alpha];
                        let mut expected = pixel;
                        let mut actual = pixel;
                        previous_blend_pixel(&mut expected, &source_pixel, opacity, blend_mode);
                        blend_pixel(&mut actual, &source_pixel, opacity, blend_mode);
                        assert_eq!(
                            actual, expected,
                            "{blend_mode:?} at {opacity}, alpha {source_alpha}, {destination} <- {source}"
                        );
                    }
                }
            }
        }
    }
}

/// Independent normalized, double-precision reference: this deliberately
/// leaves the byte-domain implementation and its optimized lookup table out.
fn artistic_reference(dst: u8, src: u8, mode: &VideoBlendMode) -> f64 {
    let d = f64::from(dst) / 255.0;
    let s = f64::from(src) / 255.0;
    let result = match mode {
        VideoBlendMode::Overlay if d <= 0.5 => 2.0 * d * s,
        VideoBlendMode::Overlay => 1.0 - 2.0 * (1.0 - d) * (1.0 - s),
        VideoBlendMode::Subtract => (d - s).max(0.0),
        VideoBlendMode::Difference => (d - s).abs(),
        VideoBlendMode::Lighten => d.max(s),
        VideoBlendMode::Darken => d.min(s),
        VideoBlendMode::SoftLight if s <= 0.5 => d - (1.0 - 2.0 * s) * d * (1.0 - d),
        VideoBlendMode::SoftLight => {
            let curve = if d <= 0.25 {
                16.0 * d.powi(3) - 12.0 * d.powi(2) + 4.0 * d
            } else {
                d.sqrt()
            };
            d + (2.0 * s - 1.0) * (curve - d)
        }
        _ => panic!("the artistic reference covers only new modes"),
    };
    result * 255.0
}

#[test]
fn new_blend_channels_match_normalized_reference_for_every_byte_pair() {
    for mode in &BLEND_MODES[4..] {
        for dst in 0..=255_u8 {
            for src in 0..=255_u8 {
                let expected = artistic_reference(dst, src, mode).round() as u8;
                let actual = blend_channel(dst, src, 1.0, mode);
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "{mode:?}, {dst} <- {src}: {actual} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn new_blend_pixels_keep_source_alpha_and_opacity_policy() {
    for mode in &BLEND_MODES[4..] {
        // Both sides of the Overlay/SoftLight branches, endpoints and equal
        // channels; alpha/opacity affect coverage after the blend function.
        for dst in [0_u8, 1, 63, 64, 65, 127, 128, 129, 254, 255] {
            for src in [0_u8, 1, 63, 64, 65, 127, 128, 129, 254, 255] {
                for source_alpha in [0_u8, 1, 127, 128, 254, 255] {
                    for opacity in [0.0_f32, 0.003, 0.37, 0.5, 1.0] {
                        let mut actual = [dst, dst, dst, 93];
                        blend_pixel(&mut actual, &[src, src, src, source_alpha], opacity, mode);
                        let coverage = f64::from(source_alpha) / 255.0 * f64::from(opacity);
                        let expected = (f64::from(dst)
                            + (artistic_reference(dst, src, mode) - f64::from(dst)) * coverage)
                            .round() as u8;
                        assert!(actual[..3]
                            .iter()
                            .all(|value| value.abs_diff(expected) <= 1));
                        let expected_alpha =
                            ((coverage + 93.0 / 255.0 * (1.0 - coverage)) * 255.0).round() as u8;
                        assert_eq!(actual[3], expected_alpha);
                        if source_alpha == 0 || opacity == 0.0 {
                            assert_eq!(actual, [dst, dst, dst, 93]);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn small_layers_blend_on_the_calling_thread() {
    assert_eq!(blend_band_count(511 * 511, 511), 1);
    assert_eq!(blend_band_count(0, 0), 1);
    assert!(blend_band_count(1920 * 1080, 1080) >= 1);
    assert!(blend_band_count(1920 * 1080, 1080) <= MAX_BLEND_BANDS);
    assert_eq!(
        blend_band_count(1920 * 1080, 2).min(2),
        blend_band_count(1920 * 1080, 2)
    );
    assert_eq!(band_rows(0, 4, 3, 0), 0..0);
    assert_eq!(band_rows(4 * 4 * 2, 4, 3, 2), 2..3);
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
        for (opacity, blend_mode) in [(1.0, VideoBlendMode::Normal), (0.5, VideoBlendMode::Add)] {
            for (path, generic, bands) in [
                ("generic", true, 1),
                (
                    "generic",
                    true,
                    blend_band_count(width as usize * height as usize, height),
                ),
                ("axis-aligned", false, 1),
                (
                    "axis-aligned",
                    false,
                    blend_band_count(width as usize * height as usize, height),
                ),
            ] {
                let mut times = Vec::new();
                for _ in 0..8 {
                    let started = std::time::Instant::now();
                    let params = BlendTransformedRgba8Params {
                        destination: &mut destination,
                        frame: &source,
                        width,
                        height,
                        opacity,
                        blend_mode: &blend_mode,
                        transform: &transform,
                        color: &VideoColorAdjust::default(),
                        fx: &VideoFxAdjust::default(),
                    };
                    if generic {
                        blend_transformed_rgba8_generic_in_bands(params, Some(bands));
                    } else {
                        blend_transformed_rgba8_in_bands(params, Some(bands));
                    }
                    std::hint::black_box(&destination);
                    times.push(started.elapsed().as_secs_f64() * 1000.0);
                }
                times.sort_by(f64::total_cmp);
                eprintln!("axis_aligned_composite {width}x{height} {blend_mode:?}@{opacity} {path} bands={bands} median_ms={:.3} max_ms={:.3}", times[4], times[7]);
            }
        }
    }
}

#[test]
fn axis_aligned_composite_ineligible_adjustments_retain_generic() {
    let source = frame(17, 9, false);
    for mode in BLEND_MODES {
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
