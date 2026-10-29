use std::time::Instant;

use ffmpeg::software::scaling::context::Context;

use super::*;

/// The pre-C4R12 conversion: `sws_getContext` + `sws_scale` into an
/// FFmpeg-allocated frame, then a packed row copy.
fn reference_rgba(source: &Video, target_width: u32, target_height: u32) -> Vec<u8> {
    let mut context = Context::get(
        source.format(),
        source.width(),
        source.height(),
        Pixel::RGBA,
        target_width,
        target_height,
        Flags::BILINEAR,
    )
    .unwrap();
    let mut rgba = Video::empty();
    context.run(source, &mut rgba).unwrap();
    packed(&rgba, target_width, target_height)
}

fn packed(frame: &Video, width: u32, height: u32) -> Vec<u8> {
    let row = width as usize * 4;
    let stride = frame.stride(0);
    let mut data = vec![0; row * height as usize];
    for (y, out) in data.chunks_exact_mut(row).enumerate() {
        out.copy_from_slice(&frame.data(0)[y * stride..y * stride + row]);
    }
    data
}

/// The decoder's conversion: in place when the destination allows it,
/// otherwise through an FFmpeg frame and a row copy.
fn converted(scaler: &mut RgbaScaler, source: &Video) -> (Vec<u8>, bool) {
    let mut data = vec![0; scaler.target_width as usize * 4 * scaler.target_height as usize];
    if scaler.writes_in_place(&data) {
        scaler.run_into(source, &mut data).unwrap();
        (data, true)
    } else {
        let mut rgba = Video::empty();
        scaler.run(source, &mut rgba).unwrap();
        (
            packed(&rgba, scaler.target_width, scaler.target_height),
            false,
        )
    }
}

fn noise_frame(format: Pixel, width: u32, height: u32, seed: u64) -> Video {
    let mut frame = Video::new(format, width, height);
    let mut state = seed | 1;
    for plane in 0..frame.planes() {
        for byte in frame.data_mut(plane).iter_mut() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = (state >> 24) as u8;
        }
    }
    frame
}

#[test]
fn in_place_slice_threaded_conversion_is_bit_exact_with_the_frame_copy_path() {
    // Output-sized scalers get slice threads only on a decode-pool worker.
    assert_eq!(
        RgbaScaler::new(Pixel::NV12, 1920, 1080, 2560, 1600)
            .unwrap()
            .threads(),
        1,
        "a scaler created outside the decode pool must own no threads"
    );
    super::super::libav_decode_pool()
        .expect("decode pool")
        .install(bit_exact_cases_on_the_decode_pool);
}

fn bit_exact_cases_on_the_decode_pool() {
    // (source, target, in place, slice threads)
    let cases = [
        ((1920, 1080), (2560, 1600), true, SLICE_THREADS as i64),
        ((1920, 1080), (1280, 720), true, 1),
        ((1280, 720), (1920, 1080), true, SLICE_THREADS as i64),
        // Widths whose packed row is not a multiple of 64 bytes take the
        // FFmpeg-frame fallback: unaligned (1366, 66), and 4 mod 8 pixels,
        // where a SIMD row loop of 8 pixels would reach into the next row
        // (1700 is a threaded downscale with a multi-tap vertical filter).
        ((1920, 1080), (1366, 768), false, SLICE_THREADS as i64),
        ((1920, 1080), (1700, 1000), false, SLICE_THREADS as i64),
        ((1920, 1080), (1928, 1200), false, SLICE_THREADS as i64),
        ((32, 18), (64, 36), true, 1),
        ((32, 18), (68, 36), false, 1),
        ((33, 19), (66, 38), false, 1),
    ];
    for format in [Pixel::NV12, Pixel::YUV420P] {
        for (index, ((sw, sh), (tw, th), in_place, threads)) in cases.into_iter().enumerate() {
            let mut scaler = RgbaScaler::new(format, sw, sh, tw, th).unwrap();
            assert_eq!(scaler.threads(), threads, "{format:?} {sw}x{sh}->{tw}x{th}");
            for seed in 0..3_u64 {
                let source = noise_frame(format, sw, sh, 0x9E37_79B9 + seed * 7 + index as u64);
                let (actual, wrote_in_place) = converted(&mut scaler, &source);
                assert_eq!(wrote_in_place, in_place, "{format:?} {tw}x{th} path");
                let expected = reference_rgba(&source, tw, th);
                assert!(
                    actual == expected,
                    "{format:?} {sw}x{sh}->{tw}x{th} seed {seed}: {} of {} bytes differ",
                    actual.iter().zip(&expected).filter(|(a, b)| a != b).count(),
                    expected.len()
                );
            }
        }
    }
}

#[test]
fn conversion_rejects_a_changed_source_or_destination_without_writing() {
    let mut scaler = RgbaScaler::new(Pixel::NV12, 32, 18, 64, 36).unwrap();
    let mut destination = vec![7_u8; 64 * 36 * 4];
    let changed = noise_frame(Pixel::YUV420P, 32, 18, 3);
    assert_eq!(
        scaler.run_into(&changed, &mut destination),
        Err(ffmpeg::Error::InputChanged)
    );
    let source = noise_frame(Pixel::NV12, 32, 18, 3);
    let mut short = vec![7_u8; 64 * 36 * 4 - 4];
    assert_eq!(
        scaler.run_into(&source, &mut short),
        Err(ffmpeg::Error::InvalidData)
    );
    assert!(destination.iter().chain(&short).all(|&byte| byte == 7));
    // The context stays usable after rejected calls.
    scaler.run_into(&source, &mut destination).unwrap();
    assert_eq!(destination, reference_rgba(&source, 64, 36));
}

/// C4R12 headless conversion benchmark on the C4 output geometry
/// (1920x1080 NV12, the D3D11VA transfer format, to 2560x1600 RGBA).
/// Each iteration allocates its output frame, as the decoder does.
/// With `SYNDOCAL_QA_C4_MEDIA`, the fixture's first decoded frames are also
/// compared byte for byte between the two paths, in its own format and as NV12.
#[test]
#[ignore = "C4R12 benchmark; run explicitly with --ignored --nocapture"]
fn c4_geometry_conversion_bench() {
    const ITERATIONS: usize = 80;
    let (sw, sh, tw, th) = (1920, 1080, 2560, 1600);
    let len = tw as usize * 4 * th as usize;
    let sources: Vec<Video> = (0..4)
        .map(|seed| noise_frame(Pixel::NV12, sw, sh, 11 + seed))
        .collect();
    let mut reference =
        Context::get(Pixel::NV12, sw, sh, Pixel::RGBA, tw, th, Flags::BILINEAR).unwrap();
    let mut threaded: Vec<(i32, RgbaScaler)> = [1, 2, 4, 6, 8]
        .into_iter()
        .map(|threads| {
            (
                threads,
                RgbaScaler::with_threads(Pixel::NV12, sw, sh, tw, th, threads).unwrap(),
            )
        })
        .collect();
    let mut samples: Vec<(String, Vec<f64>)> = vec![("frame+copy (before)".into(), Vec::new())];
    samples.extend(
        threaded
            .iter()
            .map(|(threads, _)| (format!("in place, {threads} threads"), Vec::new())),
    );
    for iteration in 0..ITERATIONS + 8 {
        let source = &sources[iteration % sources.len()];
        let started = Instant::now();
        let mut rgba = Video::empty();
        reference.run(source, &mut rgba).unwrap();
        let copied = packed(&rgba, tw, th);
        drop(rgba);
        let before = started.elapsed().as_secs_f64() * 1e3;
        std::hint::black_box(&copied);
        drop(copied);
        let mut after = Vec::new();
        for (_, scaler) in &mut threaded {
            let started = Instant::now();
            let mut data = vec![0; len];
            scaler.run_into(source, &mut data).unwrap();
            after.push(started.elapsed().as_secs_f64() * 1e3);
            std::hint::black_box(&data);
        }
        if iteration >= 8 {
            samples[0].1.push(before);
            for (slot, value) in samples[1..].iter_mut().zip(after) {
                slot.1.push(value);
            }
        }
    }
    let mut report = Vec::new();
    for (name, mut values) in samples {
        values.sort_by(f64::total_cmp);
        let quantile = |f: f64| values[((f * values.len() as f64).ceil() as usize).max(1) - 1];
        let line = format!(
            "{name}: n={} mean={:.3} p50={:.3} p95={:.3} p99={:.3} ms",
            values.len(),
            values.iter().sum::<f64>() / values.len() as f64,
            quantile(0.5),
            quantile(0.95),
            quantile(0.99)
        );
        println!("C4R12 conversion {line}");
        report.push(line);
    }
    if let Some(path) = std::env::var_os("SYNDOCAL_QA_C4_MEDIA") {
        let compared = compare_fixture_frames(std::path::Path::new(&path), tw, th);
        println!("C4R12 fixture parity: {compared} frames x 2 formats bit-exact");
        report.push(format!(
            "fixture parity: {compared} frames x 2 formats bit-exact"
        ));
    }
    if let Some(out) = std::env::var_os("SYNDOCAL_QA_BENCH_OUT") {
        std::fs::write(
            std::path::Path::new(&out).join("c4r12-conversion-bench.txt"),
            report.join("\n") + "\n",
        )
        .unwrap();
    }
}

fn compare_fixture_frames(path: &std::path::Path, tw: u32, th: u32) -> usize {
    ffmpeg::init().unwrap();
    let mut input = ffmpeg::format::input(path).unwrap();
    let stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .expect("fixture video stream");
    let index = stream.index();
    let mut decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .unwrap()
        .decoder()
        .video()
        .unwrap();
    let mut compared = 0;
    let mut frame = Video::empty();
    for (stream, packet) in input.packets() {
        if stream.index() != index {
            continue;
        }
        decoder.send_packet(&packet).unwrap();
        while decoder.receive_frame(&mut frame).is_ok() {
            let mut nv12 = Video::empty();
            Context::get(
                frame.format(),
                frame.width(),
                frame.height(),
                Pixel::NV12,
                frame.width(),
                frame.height(),
                Flags::BILINEAR,
            )
            .unwrap()
            .run(&frame, &mut nv12)
            .unwrap();
            for source in [&frame, &nv12] {
                let mut scaler =
                    RgbaScaler::new(source.format(), source.width(), source.height(), tw, th)
                        .unwrap();
                let (actual, in_place) = converted(&mut scaler, source);
                assert!(in_place);
                assert!(
                    actual == reference_rgba(source, tw, th),
                    "fixture frame {compared} {:?} differs",
                    source.format()
                );
            }
            compared += 1;
            if compared == 60 {
                return compared;
            }
        }
    }
    compared
}
