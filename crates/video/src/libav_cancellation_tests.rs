use super::*;
use protocol::VideoSourceSummary;
use std::sync::atomic::Ordering;

thread_local! {
    static CANCEL_AFTER_PACKET_READ: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn cancel_after_packet_read(stop: &StopState) {
    if CANCEL_AFTER_PACKET_READ.with(|armed| armed.replace(false)) {
        stop.cancel();
    }
}

#[test]
fn libav_cancelled_demand_never_starts_the_preferred_cli_fallback() {
    let path = fixture("preferred-no-fallback");
    let mut decoder = crate::PreferredVideoFrameDecoder::with_software_libav_for_tests(
        crate::FfmpegCliFrameDecoder::new("must-not-start-after-cancellation.exe"),
    );
    decoder
        .decode_input_frame(&input(1, &path, 0))
        .unwrap()
        .unwrap();
    CANCEL_AFTER_PACKET_READ.with(|armed| armed.set(true));
    assert_eq!(
        decoder.decode_input_frame(&input(1, &path, 200)),
        Err(VideoDecodeError::Cancelled)
    );
    assert!(!CANCEL_AFTER_PACKET_READ.with(Cell::get));
    assert_eq!(decoder.diagnostics().cli_fallback_requests, 0);
    assert_eq!(decoder.diagnostics().decode_failures, 0);
    drop(decoder);
    let _ = std::fs::remove_file(path);
}

#[test]
fn libav_mid_decode_stop_returns_cancelled_without_reopen_or_cached_success() {
    let path = fixture("mid-decode");
    let mut decoder = LibavFrameDecoder::software_for_tests();
    decoder
        .decode_input_frame(&input(1, &path, 0))
        .unwrap()
        .unwrap();
    assert_eq!(decoder.session_diagnostics().opens, 1);
    // Stop at the real packet-read ownership boundary while catching up. The
    // fixture has later frames, so EOF fallback cannot explain this result.
    CANCEL_AFTER_PACKET_READ.with(|armed| armed.set(true));
    assert_eq!(
        decoder.decode_input_frame(&input(1, &path, 200)),
        Err(VideoDecodeError::Cancelled)
    );
    assert!(
        !CANCEL_AFTER_PACKET_READ.with(Cell::get),
        "the real packet read must reach the stop seam"
    );
    assert_eq!(
        decoder.session_diagnostics().opens,
        1,
        "cancellation cannot reopen for seek recovery"
    );
    assert_eq!(decoder.session_diagnostics().hardware_errors, 0);
    assert_eq!(
        decoder.cache_len(),
        0,
        "partial decode cannot leave a successful cached answer"
    );
    assert_eq!(
        decoder.decode_input_frame(&input(1, &path, 0)),
        Err(VideoDecodeError::Cancelled)
    );
    drop(decoder);
    let _ = std::fs::remove_file(path);
}

fn fixture(name: &str) -> PathBuf {
    let base =
        std::env::temp_dir().join(format!("syndocal-libav-stop-{name}-{}", std::process::id()));
    let raw = base.with_extension("rgba");
    let path = base.with_extension("mkv");
    let pixels = [11, 77, 203, 255].repeat(16 * 16 * 12);
    std::fs::write(&raw, pixels).unwrap();
    let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let result = std::process::Command::new(ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgba",
            "-video_size",
            "16x16",
            "-framerate",
            "30",
            "-i",
        ])
        .arg(&raw)
        .args(["-c:v", "ffv1", "-pix_fmt", "bgra", "-y"])
        .arg(&path)
        .output()
        .expect("FFmpeg fixture encoder");
    let _ = std::fs::remove_file(raw);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    path
}

fn input(layer_id: u64, path: &std::path::Path, position_ms: u64) -> VideoRenderInput {
    VideoRenderInput::legacy(VideoFrameRequest {
        layer_id,
        label: format!("Stop fixture {layer_id}"),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some(path.to_string_lossy().into_owned()),
            name: None,
            codec: Some("ffv1".into()),
            metadata: None,
        },
        position_ms,
        width: 16,
        height: 16,
    })
}

#[test]
fn libav_cancelled_batch_does_not_open_decode_or_reclassify_as_hardware_failure() {
    let stop = Arc::new(AtomicBool::new(true));
    let mut decoder = LibavFrameDecoder::software_for_tests().with_stop_signal(stop);
    let inputs = [
        input(1, std::path::Path::new("must-not-open.mkv"), 0),
        input(2, std::path::Path::new("also-missing.mkv"), 0),
    ];
    assert_eq!(
        decoder.decode_input_frames_shared(&inputs),
        vec![Err(VideoDecodeError::Cancelled); 2]
    );
    assert_eq!(decoder.session_diagnostics().opens, 0);
    assert_eq!(decoder.session_diagnostics().errors, 0);
    assert_eq!(
        hardware::classify(&inputs[0].request, true, VideoDecodeError::Cancelled),
        VideoDecodeError::Cancelled
    );
}

#[test]
fn libav_stop_preserves_batch_and_shared_known_pixels_then_rejects_cached_frames() {
    let path = fixture("pixels");
    let stop = Arc::new(AtomicBool::new(false));
    let mut decoder = LibavFrameDecoder::software_for_tests().with_stop_signal(stop.clone());
    for position in [0, 33, 66, 100] {
        let inputs = [input(1, &path, position), input(2, &path, position)];
        let frames = decoder.decode_input_frames_shared(&inputs);
        for frame in frames {
            let frame = frame.unwrap().unwrap();
            assert_eq!((frame.width, frame.height), (16, 16));
            assert!(frame
                .data
                .chunks_exact(4)
                .all(|pixel| pixel == [11, 77, 203, 255]));
        }
    }
    let opens = decoder.session_diagnostics().opens;
    assert_eq!(opens, 2, "shared batch must retain its two sessions");
    stop.store(true, Ordering::Release);
    assert_eq!(
        decoder.decode_input_frames_shared(&[input(1, &path, 100), input(2, &path, 100)]),
        vec![Err(VideoDecodeError::Cancelled); 2]
    );
    assert_eq!(decoder.session_diagnostics().opens, opens);
    assert!(decoder.lookaheads.is_empty());
    drop(decoder);
    assert_eq!(
        Arc::strong_count(&stop),
        1,
        "interrupt userdata must be reclaimed after native input close"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn libav_interrupt_userdata_moves_with_session_and_is_freed_on_success_and_error() {
    let path = fixture("interrupt-owner");
    let external = Arc::new(AtomicBool::new(false));
    {
        let stop = StopState::new(Some(external.clone()));
        let native = InterruptInput::open(&path, &stop).unwrap();
        let moved = Box::new(native);
        let callback = unsafe { (*moved.as_ptr()).interrupt_callback };
        let invoke = callback.callback.expect("installed interrupt callback");
        assert_eq!(unsafe { invoke(callback.opaque) }, 0);
        external.store(true, Ordering::Release);
        assert_eq!(unsafe { invoke(callback.opaque) }, 1);
        drop(moved);
    }
    assert_eq!(Arc::strong_count(&external), 1);
    external.store(false, Ordering::Release);
    {
        let stop = StopState::new(Some(external.clone()));
        assert!(InterruptInput::open(&path.with_extension("missing"), &stop).is_err());
    }
    assert_eq!(
        Arc::strong_count(&external),
        1,
        "failed opens must free interrupt userdata"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn libav_decoder_drop_cancels_and_reaps_queued_session_without_stopping_worker_signal() {
    let path = fixture("drop-owner");
    let external = Arc::new(AtomicBool::new(false));
    let mut decoder = LibavFrameDecoder::software_for_tests().with_stop_signal(external.clone());
    let request = input(1, &path, 0);
    decoder.decode_input_frame(&request).unwrap().unwrap();
    let session = decoder.sessions.pop().unwrap();
    let stop = session.stop.clone();
    let (sender, done) = std::sync::mpsc::sync_channel(1);
    let completed = Arc::new(AtomicBool::new(false));
    let completed_worker = completed.clone();
    let release_on_regression = Arc::new(AtomicBool::new(false));
    let release_worker = release_on_regression.clone();
    let (task_finished, task_done) = std::sync::mpsc::sync_channel(1);
    libav_decode_pool().unwrap().spawn(move || {
        // Deliberately occupy the task until its owning decoder retires. No
        // timing race: completion can happen only after Drop signals stop.
        while !stop.is_cancelled() && !release_worker.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        completed_worker.store(true, Ordering::Release);
        let _ = sender.send(LibavLookaheadDone {
            session,
            result: Err(VideoDecodeError::Cancelled),
            hardware_frames: 0,
        });
        let _ = task_finished.send(());
    });
    decoder.lookaheads.push(LibavLookahead {
        key: request.key,
        layer_id: 1,
        served_pts_ms: 0,
        last_request_ms: Some(0),
        hardware: false,
        done,
    });
    let (drop_finished, drop_done) = std::sync::mpsc::sync_channel(1);
    let completed_at_drop = completed.clone();
    let dropping = std::thread::spawn(move || {
        drop(decoder);
        let _ = drop_finished.send(completed_at_drop.load(Ordering::Acquire));
    });
    let result = drop_done.recv_timeout(std::time::Duration::from_secs(1));
    // A broken implementation must fail without leaving the synthetic task
    // spinning or its native session detached from a test owner.
    release_on_regression.store(true, Ordering::Release);
    task_done
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    dropping.join().unwrap();
    assert!(
        result == Ok(true),
        "Drop must wait for queued owner completion"
    );
    assert!(
        !external.load(Ordering::Acquire),
        "replacing a decoder must not stop its whole worker"
    );
    let _ = std::fs::remove_file(path);
}
