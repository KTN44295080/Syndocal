use super::*;

fn fixture_request(path: String) -> VideoFrameRequest {
    VideoFrameRequest {
        layer_id: 9001,
        label: "Seek projection fixture".into(),
        source: protocol::VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some(path),
            name: None,
            codec: Some("h264".into()),
            metadata: None,
        },
        position_ms: 0,
        width: 320,
        height: 128,
    }
}

#[test]
fn seek_projection_retains_latest_raw_candidate_and_terminal_fallback() {
    let mut request = fixture_request("unused.mp4".into());
    request.width = 2;
    request.height = 1;
    let mut raw = ffmpeg::util::frame::video::Video::new(ffmpeg::format::Pixel::RGBA, 2, 1);
    raw.data_mut(0)[..8].copy_from_slice(&[12, 34, 56, 255, 78, 90, 123, 255]);
    let mut transfers = 0;
    let actual = finish_decoded_candidate(
        &request,
        Some(DecodedCandidate {
            frame: raw,
            pts_ms: 234,
        }),
        None,
        false,
        &mut transfers,
    )
    .unwrap();
    assert_eq!(actual.pts_ms, 234);
    assert_eq!(actual.data, [12, 34, 56, 255, 78, 90, 123, 255]);
    assert_eq!(transfers, 0);
    assert_eq!(
        finish_decoded_candidate(&request, None, Some(&actual), false, &mut transfers).unwrap(),
        actual
    );
    assert!(finish_decoded_candidate(&request, None, None, false, &mut transfers).is_err());
}

#[test]
#[ignore = "requires SYNDOCAL_GPU_TEST_VIDEO (>3 seconds); real GPU on Windows"]
fn seek_projection_real_video_matches_eager_oracle_with_fewer_transfers() {
    use std::time::{Duration, Instant};
    let path = std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set original video path");
    ffmpeg::init().unwrap();
    let duration_ms = ffmpeg::format::input(&path).unwrap().duration() / 1000;
    assert!(duration_ms > 3000, "fixture must exceed three seconds");
    let positions = [0, 123, 875, 1500, 2200, duration_ms as u64 + 100];
    for round in 0..3 {
        let mut results = Vec::new();
        for eager in if round % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        } {
            let mut frames = Vec::new();
            let mut transfers = 0;
            let mut elapsed = Duration::ZERO;
            for position in std::hint::black_box(positions) {
                let mut request = fixture_request(path.clone());
                request.position_ms = position;
                let signature = StillImageSignature::from_path(std::path::Path::new(&path));
                let mut session = open_libav_session(
                    protocol::VideoRenderInputKey::LEGACY,
                    &request,
                    path.clone().into(),
                    signature,
                    true,
                )
                .unwrap();
                let past_eof = position > duration_ms as u64;
                // Exercise the real EOF drain from the final GOP without decoding
                // an entire long show file merely to reach its terminal frame.
                if past_eof {
                    session.input.seek((duration_ms - 1000) * 1000, ..).unwrap();
                    session.decoder.flush();
                }
                let started = Instant::now();
                let frame = if eager {
                    eager_decode(
                        std::hint::black_box(&request),
                        &mut session,
                        !past_eof,
                        None,
                        &mut transfers,
                    )
                } else {
                    decode_libav_session_frame(
                        std::hint::black_box(&request),
                        &mut session,
                        !past_eof,
                        None,
                        &mut transfers,
                    )
                }
                .unwrap();
                std::hint::black_box(&frame);
                elapsed += started.elapsed();
                if past_eof {
                    assert!(session.decoder_drained);
                }
                frames.push(frame);
            }
            results.push((eager, frames, transfers, elapsed));
        }
        let old = results.iter().find(|result| result.0).unwrap();
        let new = results.iter().find(|result| !result.0).unwrap();
        assert_eq!(
            old.1, new.1,
            "PTS, dimensions, duration and RGBA bytes must match"
        );
        #[cfg(target_os = "windows")]
        {
            assert_eq!(new.2, positions.len() as u64);
            assert!(
                old.2 > new.2,
                "fixture must exercise skipped intermediate frames"
            );
        }
        eprintln!("seek projection round={round} requests={} old_transfers={} new_transfers={} old_decode={:?} new_decode={:?}; session creation excluded", positions.len(), old.2, new.2, old.3, new.3);
    }
}

// Frozen eager conversion oracle from the pre-optimization receive path.
fn eager_decode(
    request: &VideoFrameRequest,
    session: &mut LibavDecodeSession,
    seek_before_decode: bool,
    terminal_frame: Option<&VideoFrame>,
    hardware_frames: &mut u64,
) -> Result<VideoFrame, VideoDecodeError> {
    let target_offset_us = request
        .position_ms
        .saturating_mul(1_000)
        .min(i64::MAX as u64) as i64;
    let target_us = target_offset_us;
    if seek_before_decode && request.position_ms > 0 && session.input.seek(target_us, ..).is_ok() {
        session.decoder.flush();
    }
    let mut candidate = None;

    let received = eager_receive(
        request,
        &mut session.decoder,
        session.hardware,
        hardware_frames,
        session.time_base,
        session.timestamp_origin,
        &mut candidate,
    )?;
    session.decoder_drained = received.decoder_drained;
    if let Some(frame) = received.frame {
        session.last_request_ms = Some(request.position_ms);
        return Ok(frame);
    }
    if session.decoder_drained {
        let frame = candidate
            .take()
            .or_else(|| terminal_frame.cloned())
            .ok_or_else(|| decode_message(request, "decoder produced no video frame"))?;
        session.last_request_ms = Some(request.position_ms);
        return Ok(frame);
    }

    for (packet_stream, packet) in session.input.packets() {
        if packet_stream.index() != session.stream_index {
            continue;
        }
        session
            .decoder
            .send_packet(&packet)
            .map_err(|error| decode_error(request, error))?;
        let received = eager_receive(
            request,
            &mut session.decoder,
            session.hardware,
            hardware_frames,
            session.time_base,
            session.timestamp_origin,
            &mut candidate,
        )?;
        session.decoder_drained = received.decoder_drained;
        if let Some(frame) = received.frame {
            session.last_request_ms = Some(request.position_ms);
            return Ok(frame);
        }
    }
    if !session.eof_sent {
        session
            .decoder
            .send_eof()
            .map_err(|error| decode_error(request, error))?;
        session.eof_sent = true;
    }
    let received = eager_receive(
        request,
        &mut session.decoder,
        session.hardware,
        hardware_frames,
        session.time_base,
        session.timestamp_origin,
        &mut candidate,
    )?;
    session.decoder_drained = received.decoder_drained;
    if let Some(frame) = received.frame {
        session.last_request_ms = Some(request.position_ms);
        return Ok(frame);
    }
    let frame = candidate
        .or_else(|| terminal_frame.cloned())
        .ok_or_else(|| decode_message(request, "decoder produced no video frame"))?;
    session.last_request_ms = Some(request.position_ms);
    Ok(frame)
}

#[cfg(feature = "libav")]
fn eager_receive(
    request: &VideoFrameRequest,
    decoder: &mut ffmpeg_next::decoder::Video,
    hardware: bool,
    hardware_frames: &mut u64,
    time_base: ffmpeg_next::Rational,
    timestamp_origin: i64,
    candidate: &mut Option<VideoFrame>,
) -> Result<LibavReceiveResult, VideoDecodeError> {
    use ffmpeg_next::error::EAGAIN;
    use ffmpeg_next::util::frame::video::Video;

    let mut decoded = Video::empty();
    loop {
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {
                let pts_ms = match decoded.timestamp().and_then(|timestamp| {
                    normalized_timestamp_ms(timestamp, timestamp_origin, time_base)
                }) {
                    Some(pts_ms) if pts_ms >= 0 => pts_ms as u64,
                    Some(_) => continue,
                    None => request.position_ms,
                };
                let transferred = hardware::transfer(request, hardware, &decoded)?;
                if hardware {
                    *hardware_frames = hardware_frames.saturating_add(1);
                }
                let source = transferred.as_ref().unwrap_or(&decoded);
                let mut scaler = acquire_cached_libav_scaler(
                    request,
                    source.format(),
                    source.width(),
                    source.height(),
                )?;
                let mut rgba = Video::empty();
                scaler
                    .run(source, &mut rgba)
                    .map_err(|error| decode_error(request, error))?;
                let frame = copy_rgba_frame(request, &rgba, pts_ms)?;
                if frame.pts_ms >= request.position_ms {
                    return Ok(LibavReceiveResult {
                        frame: Some(frame),
                        decoder_drained: false,
                    });
                }
                *candidate = Some(frame);
            }
            Err(ffmpeg_next::Error::Eof) => {
                return Ok(LibavReceiveResult {
                    frame: None,
                    decoder_drained: true,
                });
            }
            Err(ffmpeg_next::Error::Other { errno }) if errno == EAGAIN => {
                return Ok(LibavReceiveResult {
                    frame: None,
                    decoder_drained: false,
                });
            }
            Err(error) => return Err(decode_error(request, error)),
        }
    }
}
