//! Decoder hand-back must reuse storage without retaining or mutating readers.
use super::*;
use protocol::VideoSourceSummary;

fn request() -> VideoFrameRequest {
    VideoFrameRequest {
        layer_id: 1,
        label: "RGBA hand-back".into(),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("reuse-fixture.mp4".into()),
            name: None,
            codec: Some("h264".into()),
            metadata: None,
        },
        position_ms: 0,
        width: 64,
        height: 36,
    }
}

fn pixels(request: &VideoFrameRequest, token: u8) -> Arc<VideoFrame> {
    Arc::new(rgba_video_frame(
        request,
        request.position_ms,
        vec![token; rgba_frame_len(request).unwrap()],
    ))
}

fn control() -> Arc<std::sync::Mutex<LibavRunControl>> {
    Arc::new(std::sync::Mutex::new(LibavRunControl {
        until_ms: 100,
        limit: 4,
        outstanding: 2,
        finished: false,
        clip_end: false,
        wrap_until_ms: 100,
        wrap_cover_ms: 25,
        rgba_spare: None,
        rgba_spare_in_decode: false,
    }))
}

fn pending(control: Arc<std::sync::Mutex<LibavRunControl>>) -> LibavLookahead {
    let (_, done) = std::sync::mpsc::sync_channel(1);
    LibavLookahead {
        key: protocol::VideoRenderInputKey::LEGACY,
        layer_id: 1,
        served_pts_ms: 0,
        last_request_ms: Some(0),
        hardware: false,
        ready: VecDeque::new(),
        frames: None,
        control: Some(control),
        done,
    }
}

#[test]
fn exclusive_rgba_hand_back_reuses_the_exact_allocation() {
    let request = request();
    let frame = pixels(&request, 19);
    let pointer = frame.data.as_ptr();
    let extent = (request.width, request.height);
    let mut spare = ReturnedRgba::reclaim(frame, extent);
    let data = rgba_reuse::destination(&mut spare, extent, rgba_frame_len(&request).unwrap());
    assert_eq!(data.as_ptr(), pointer);
    assert!(
        spare.is_none(),
        "the returned allocation is consumed exactly once"
    );
    assert!(data.iter().all(|byte| *byte == 19));
}

#[test]
fn shared_wrong_extent_and_excess_capacity_are_never_reclaimed() {
    let request = request();
    let extent = (request.width, request.height);
    let held = pixels(&request, 37);
    let pointer = held.data.as_ptr();
    assert!(ReturnedRgba::reclaim(Arc::clone(&held), extent).is_none());
    assert_eq!(held.data.as_ptr(), pointer);
    assert!(held.data.iter().all(|byte| *byte == 37));
    assert!(ReturnedRgba::reclaim(pixels(&request, 0), (36, 64)).is_none());
    let len = rgba_frame_len(&request).unwrap();
    let mut oversized = Vec::with_capacity(len + 1);
    oversized.resize(len, 0);
    assert!(
        ReturnedRgba::reclaim(Arc::new(rgba_video_frame(&request, 0, oversized)), extent,)
            .is_none()
    );
}

#[test]
fn projection_reuses_packed_and_row_copy_buffers_with_exact_pixel_parity() {
    use ffmpeg::{format::Pixel, util::frame::video::Video};
    let mut source = Video::new(Pixel::RGBA, 32, 18);
    for pixel in source.data_mut(0).chunks_exact_mut(4) {
        pixel.copy_from_slice(&[19, 47, 83, 255]);
    }
    for extent in [(64, 36), (66, 38)] {
        let mut request = request();
        (request.width, request.height) = extent;
        let mut hardware_frames = 0;
        let baseline = project_decoded_frame(
            &request,
            &source,
            33,
            false,
            &mut hardware_frames,
            &StopState::new(None),
            &mut None,
        )
        .unwrap();
        let old = pixels(&request, 211);
        let pointer = old.data.as_ptr();
        let mut spare = ReturnedRgba::reclaim(old, extent);
        let reused = project_decoded_frame(
            &request,
            &source,
            33,
            false,
            &mut hardware_frames,
            &StopState::new(None),
            &mut spare,
        )
        .unwrap();
        assert_eq!(
            reused, baseline,
            "{extent:?}: every packed byte is overwritten"
        );
        assert_eq!(
            reused.data.as_ptr(),
            pointer,
            "{extent:?}: exact storage reuse"
        );
        assert!(spare.is_none());
    }
}

#[test]
fn pending_ready_replacement_returns_one_spare_without_retaining_old_frames() {
    let mut request = request();
    let extent = (request.width, request.height);
    let control = control();
    let mut pending = pending(Arc::clone(&control));
    let mut retired = rgba_reuse::RetiredRgba::default();
    let mut cached = pixels(&request, 11);
    let old_pointer = cached.data.as_ptr();
    request.position_ms = 33;
    pending.ready.push_back(pixels(&request, 22));
    let selected = pending
        .take_ready(17, &mut cached, extent, &mut retired)
        .unwrap();
    assert!(Arc::ptr_eq(&cached, &selected));
    drop(selected);
    let mut guard = lock_run_control(&control);
    assert_eq!(guard.outstanding, 1);
    let first = guard.rgba_spare.take().unwrap();
    let data = first
        .into_buffer(extent, rgba_frame_len(&request).unwrap())
        .unwrap();
    assert_eq!(
        data.as_ptr(),
        old_pointer,
        "the direct resident replacement returns its old allocation"
    );
    guard.rgba_spare = ReturnedRgba::reclaim(Arc::new(rgba_video_frame(&request, 0, data)), extent);
    drop(guard);

    request.position_ms = 67;
    pending.ready.push_back(pixels(&request, 33));
    pending
        .take_ready(50, &mut cached, extent, &mut retired)
        .unwrap();
    let mut guard = lock_run_control(&control);
    let kept = guard.rgba_spare.take().unwrap();
    let kept = kept
        .into_buffer(extent, rgba_frame_len(&request).unwrap())
        .unwrap();
    assert_eq!(
        kept.as_ptr(),
        old_pointer,
        "a full spare slot is never replaced or extended"
    );
    guard.rgba_spare_in_decode = true;
    let mut rejected = ReturnedRgba::reclaim(pixels(&request, 44), extent);
    guard.accept_returned_rgba(&mut rejected);
    assert!(
        guard.rgba_spare.is_none(),
        "a worker-owned spare reserves the same slot"
    );
    assert!(
        rejected.is_some(),
        "the caller owns rejected storage until after the guard"
    );
    drop(guard);
    drop(rejected);
}

fn media(stem: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "syndocal-rgba-reuse-{stem}-{}.mkv",
        std::process::id()
    ));
    let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let generated = std::process::Command::new(ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=32x18:rate=30:duration=2",
            "-c:v",
            "ffv1",
            "-g",
            "10",
            "-y",
        ])
        .arg(&path)
        .output()
        .expect("FFmpeg reuse fixture generator must be available");
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    path
}

fn media_input(path: &std::path::Path) -> VideoRenderInput {
    let mut request = request();
    request.source.path = Some(path.to_string_lossy().into_owned());
    request.source.codec = Some("ffv1".into());
    VideoRenderInput::legacy(request)
}

#[test]
fn real_worker_takes_control_spare_and_returns_exact_oracle_pixels() {
    let path = media("worker");
    let input = media_input(&path);
    let extent = (input.request.width, input.request.height);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let served = decoder
        .decode_tick(std::slice::from_ref(&input), false)
        .pop()
        .unwrap()
        .unwrap()
        .unwrap();
    let mut session = decoder.sessions.pop().unwrap();
    let control = control();
    let returned = pixels(&input.request, 211);
    let pointer = returned.data.as_ptr() as usize;
    {
        let mut control = lock_run_control(&control);
        control.outstanding = 0;
        control.rgba_spare = ReturnedRgba::reclaim(returned, extent);
    }
    let worker_control = Arc::clone(&control);
    let (frames_sender, frames) = std::sync::mpsc::channel();
    let (done_sender, done) = std::sync::mpsc::sync_channel(1);
    let request = input.request.clone();
    libav_decode_pool().unwrap().spawn(move || {
        let (result, hardware_frames) = run_libav_lookahead(
            request,
            &mut session,
            served,
            &worker_control,
            frames_sender,
        );
        let _ = done_sender.send(LibavLookaheadDone {
            session,
            result: Some(result),
            hardware_frames,
        });
    });
    decoder.lookaheads.push(LibavLookahead {
        key: input.key,
        layer_id: input.layer_id(),
        served_pts_ms: 0,
        last_request_ms: Some(0),
        hardware: false,
        ready: VecDeque::new(),
        frames: Some(frames),
        control: Some(Arc::clone(&control)),
        done,
    });
    decoder.join_lookaheads();
    assert_eq!(
        decoder.sessions.len(),
        1,
        "join recovers the real worker's session"
    );
    let reused = decoder.sessions[0]
        .next_frames
        .iter()
        .find(|frame| frame.data.as_ptr() as usize == pointer)
        .expect("a worker projection must consume the exact control-owned allocation");
    assert!(
        reused.pts_ms > 33,
        "the first decode hands the spare to the next projection"
    );
    let mut oracle_input = input.clone();
    oracle_input.request.position_ms = reused.pts_ms;
    let mut oracle = LibavFrameDecoder::software_for_tests();
    let expected = oracle.decode_input_frame(&oracle_input).unwrap().unwrap();
    assert_eq!(
        reused.as_ref(),
        &expected,
        "reuse must overwrite every poisoned byte"
    );
    assert!(decoder.sessions[0].rgba_spare.is_none());
    let control = lock_run_control(&control);
    assert!(control.rgba_spare.is_none());
    assert!(
        !control.rgba_spare_in_decode,
        "the worker releases the spare reservation"
    );
    drop(control);
    decoder.retain_inputs(&[]);
    assert!(decoder.sessions.is_empty() && decoder.lookaheads.is_empty());
    drop(oracle);
    drop(decoder);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn decoder_drop_cancels_and_reaps_real_worker_with_retained_spare() {
    use std::sync::atomic::Ordering;
    let path = media("cancel");
    let input = media_input(&path);
    let external = Arc::new(AtomicBool::new(false));
    let mut decoder =
        LibavFrameDecoder::software_for_tests().with_stop_signal(Arc::clone(&external));
    let served = decoder
        .decode_tick(std::slice::from_ref(&input), false)
        .pop()
        .unwrap()
        .unwrap()
        .unwrap();
    let mut session = decoder.sessions.pop().unwrap();
    session.rgba_spare = ReturnedRgba::reclaim(
        pixels(&input.request, 211),
        (input.request.width, input.request.height),
    );
    assert!(session.rgba_spare.is_some());
    let stop = session.stop.clone();
    let (done_sender, done) = std::sync::mpsc::sync_channel(1);
    let (completed_sender, completed) = std::sync::mpsc::sync_channel(1);
    let release = Arc::new(AtomicBool::new(false));
    let worker_release = Arc::clone(&release);
    let request = input.request.clone();
    libav_decode_pool().unwrap().spawn(move || {
        // Deterministically retain the spare until its owner signals stop.
        // The release flag ensures a broken Drop cannot leave a detached task.
        while !stop.is_cancelled() && !worker_release.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        let result =
            decode_libav_session_frame(&request, &mut session, false, Some(&served), &mut 0);
        let cancelled_with_spare =
            result == Err(VideoDecodeError::Cancelled) && session.rgba_spare.is_some();
        let _ = done_sender.send(LibavLookaheadDone {
            session,
            result: Some(result),
            hardware_frames: 0,
        });
        let _ = completed_sender.send(cancelled_with_spare);
    });
    decoder.lookaheads.push(LibavLookahead {
        key: input.key,
        layer_id: input.layer_id(),
        served_pts_ms: 0,
        last_request_ms: Some(0),
        hardware: false,
        ready: VecDeque::new(),
        frames: None,
        control: None,
        done,
    });
    let (dropped_sender, dropped) = std::sync::mpsc::sync_channel(1);
    let dropping = std::thread::spawn(move || {
        drop(decoder);
        let _ = dropped_sender.send(());
    });
    let result = dropped.recv_timeout(std::time::Duration::from_secs(5));
    release.store(true, Ordering::Release);
    let cancelled_with_spare = completed
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    dropping.join().unwrap();
    assert_eq!(
        result,
        Ok(()),
        "Drop acknowledges the worker before returning"
    );
    assert!(
        cancelled_with_spare,
        "cancelled projection preserves the retained spare until reap"
    );
    assert!(
        !external.load(Ordering::Acquire),
        "decoder retirement preserves the owner's external signal"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn eight_generation_history_releases_an_exact_allocation_each_displacement() {
    let request = request();
    let extent = (request.width, request.height);
    let mut retired = rgba_reuse::RetiredRgba::default();
    let mut history = VecDeque::new();
    let mut pointers = Vec::new();
    for token in 0..40_u8 {
        let frame = pixels(&request, token);
        pointers.push(frame.data.as_ptr());
        history.push_back(Arc::clone(&frame));
        if history.len() > 8 {
            history.pop_front();
        }
        let returned = retired.retire(frame, extent);
        if token < 8 {
            assert!(
                returned.is_none(),
                "every candidate still has an immutable history reader"
            );
        } else {
            let data = returned
                .expect("a full history horizon must recycle on every displacement")
                .into_buffer(extent, rgba_frame_len(&request).unwrap())
                .unwrap();
            assert_eq!(data.as_ptr(), pointers[token as usize - 8]);
            assert!(data.iter().all(|byte| *byte == token - 8));
        }
        for (index, reader) in history.iter().enumerate() {
            let expected = token.saturating_sub(7) + index as u8;
            assert!(reader.data.iter().all(|byte| *byte == expected));
        }
    }
}

#[test]
fn retirement_bound_deduplicates_and_releases_overflow_and_retired_owners() {
    let request = request();
    let extent = (request.width, request.height);
    let mut retired = rgba_reuse::RetiredRgba::default();
    let held: Vec<_> = (0..10).map(|token| pixels(&request, token)).collect();
    for frame in &held[..8] {
        assert!(retired.retire(Arc::clone(frame), extent).is_none());
    }
    for _ in 0..12 {
        assert!(retired.retire(Arc::clone(&held[0]), extent).is_none());
    }
    assert_eq!(
        Arc::strong_count(&held[0]),
        2,
        "duplicate candidates do not consume slots"
    );
    assert!(retired.retire(Arc::clone(&held[8]), extent).is_none());
    assert_eq!(
        Arc::strong_count(&held[8]),
        1,
        "overflow is released without waiting"
    );
    drop(retired);
    assert!(held.iter().all(|frame| Arc::strong_count(frame) == 1));
    // A cache-entry owner releases its deferred candidates on input retirement.
    let path = media("retire-history");
    let input = media_input(&path);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let frame = decoder
        .decode_tick(std::slice::from_ref(&input), false)
        .pop()
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(decoder.entries[0]
        .rgba_returns
        .retire(Arc::clone(&held[0]), extent)
        .is_none());
    assert_eq!(Arc::strong_count(&held[0]), 2);
    decoder.retain_inputs(&[]);
    assert_eq!(Arc::strong_count(&held[0]), 1);
    drop(frame);
    drop(decoder);
    std::fs::remove_file(path).unwrap();
}
