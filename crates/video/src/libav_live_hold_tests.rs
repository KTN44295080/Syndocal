//! The live resident Display acquisition may answer a tick with the layer's
//! last served frame while its streamed look-ahead is behind. These tests pin
//! the boundaries of that hold: it never serves a frame ahead of the request,
//! never goes backwards, never survives a seek, and never changes what the
//! exact (preview/recording/export) acquisition returns afterwards.
use super::*;
use protocol::VideoSourceSummary;

fn fixture(name: &str, size: &str, seconds: u32) -> PathBuf {
    let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let path = std::env::temp_dir().join(format!(
        "syndocal-libav-live-hold-{name}-{}.mkv",
        std::process::id()
    ));
    let generated = std::process::Command::new(&ffmpeg)
        .args(["-v", "error", "-f", "lavfi", "-i"])
        .arg(format!("testsrc2=size={size}:rate=30:duration={seconds}"))
        .args(["-c:v", "ffv1", "-g", "10", "-y"])
        .arg(&path)
        .output()
        .expect("FFmpeg fixture generator must be available");
    assert!(
        generated.status.success(),
        "fixture generation failed: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    path
}

fn input(path: &std::path::Path, width: u32, height: u32, position_ms: u64) -> VideoRenderInput {
    VideoRenderInput::legacy(VideoFrameRequest {
        layer_id: 7301,
        label: "Live hold fixture".into(),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some(path.to_string_lossy().into_owned()),
            name: None,
            codec: Some("ffv1".into()),
            metadata: None,
        },
        position_ms,
        width,
        height,
    })
}

fn exact(path: &std::path::Path, width: u32, height: u32, position_ms: u64) -> Arc<VideoFrame> {
    LibavFrameDecoder::software_for_tests()
        .decode_input_frame_shared(&input(path, width, height, position_ms))
        .unwrap()
        .unwrap()
}

#[test]
fn live_frames_never_lead_the_request_or_go_backwards_and_count_holds() {
    // A larger fixture keeps the look-ahead busy so back-to-back live ticks can
    // find its stream empty; holds are allowed, not required, on a fast host.
    let (width, height) = (640, 360);
    let path = fixture("burst", "640x360", 3);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let mut last_pts = 0;
    for step in 0..60u64 {
        let position = step * 33;
        let frame = decoder
            .decode_input_frame_shared_live(&input(&path, width, height, position))
            .unwrap()
            .unwrap();
        assert!(
            frame.pts_ms <= position,
            "tick {step}: pts {} leads request {position}",
            frame.pts_ms
        );
        assert!(
            frame.pts_ms >= last_pts,
            "tick {step}: pts {} went back from {last_pts}",
            frame.pts_ms
        );
        last_pts = frame.pts_ms;
    }
    let diagnostics = decoder.session_diagnostics();
    assert!(diagnostics.live_held_frames <= 60);
    // Exact acquisition after a live burst is unaffected by any hold.
    let after = decoder
        .decode_input_frame_shared(&input(&path, width, height, 60 * 33))
        .unwrap()
        .unwrap();
    let reference = exact(&path, width, height, 60 * 33);
    assert_eq!(after.pts_ms, reference.pts_ms);
    assert_eq!(
        after.data, reference.data,
        "exact acquisition after live ticks"
    );
}

#[test]
fn a_backward_seek_is_never_answered_by_a_held_frame() {
    let (width, height) = (640, 360);
    let path = fixture("seek", "640x360", 3);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    for step in 0..30u64 {
        decoder
            .decode_input_frame_shared_live(&input(&path, width, height, 1_000 + step * 33))
            .unwrap();
    }
    for position in [100, 0, 1_500] {
        let frame = decoder
            .decode_input_frame_shared_live(&input(&path, width, height, position))
            .unwrap()
            .unwrap();
        let reference = exact(&path, width, height, position);
        assert_eq!(frame.pts_ms, reference.pts_ms, "seek to {position}");
        assert_eq!(frame.data, reference.data, "seek to {position}: pixels");
    }
}

#[test]
fn a_live_run_with_a_ready_stream_matches_exact_acquisition() {
    let (width, height) = (48, 27);
    let path = fixture("ready", "48x27", 2);
    let mut live = LibavFrameDecoder::software_for_tests();
    let mut reference = LibavFrameDecoder::software_for_tests();
    for step in 0..40u64 {
        let position = step * 33;
        let request = input(&path, width, height, position);
        let frame = live
            .decode_input_frame_shared_live(&request)
            .unwrap()
            .unwrap();
        let expected = reference
            .decode_input_frame_shared(&request)
            .unwrap()
            .unwrap();
        if live.session_diagnostics().live_held_frames == 0 {
            assert_eq!(frame.pts_ms, expected.pts_ms, "tick {step}");
            assert_eq!(frame.data, expected.data, "tick {step}: pixels");
        }
        // Give the tiny fixture's look-ahead time to stay ahead.
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// A streamed look-ahead whose worker is gated: its stream is open but empty
/// and it has not sent `done`, exactly the "decoder behind" state.
struct StalledLookahead {
    stream: Option<std::sync::mpsc::Sender<Arc<VideoFrame>>>,
    release: Option<std::sync::mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl StalledLookahead {
    fn release(&mut self) {
        self.stream.take();
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

impl Drop for StalledLookahead {
    fn drop(&mut self) {
        self.release();
    }
}

fn stall_streamed_lookahead(
    decoder: &mut LibavFrameDecoder,
    input: &VideoRenderInput,
    served: &Arc<VideoFrame>,
) -> StalledLookahead {
    assert!(decoder.lookaheads.is_empty());
    let index = decoder.session_index(input.key, input.layer_id()).unwrap();
    let session = decoder.sessions.swap_remove(index);
    let last_request_ms = session.last_request_ms;
    let (stream, frames) = std::sync::mpsc::channel();
    let (release, gate) = std::sync::mpsc::channel::<()>();
    let (sender, done) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = gate.recv();
        let _ = sender.send(LibavLookaheadDone {
            session,
            result: None,
            hardware_frames: 0,
        });
    });
    decoder.lookaheads.push(LibavLookahead {
        key: input.key,
        layer_id: input.layer_id(),
        served_pts_ms: served.pts_ms,
        last_request_ms,
        hardware: false,
        ready: std::collections::VecDeque::new(),
        frames: Some(frames),
        control: None,
        done,
    });
    StalledLookahead {
        stream: Some(stream),
        release: Some(release),
        worker: Some(worker),
    }
}

#[test]
fn a_stalled_streamed_lookahead_is_held_by_live_acquisition_without_waiting() {
    let (width, height) = (48, 27);
    let path = fixture("stalled", "48x27", 2);
    let first = input(&path, width, height, 16);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    decoder.decode_input_frame(&first).unwrap().unwrap();
    let served = Arc::clone(&decoder.entries[0].frame);
    let mut stall = stall_streamed_lookahead(&mut decoder, &first, &served);
    let next = input(&path, width, height, served.pts_ms + 33);
    let (finished, ready) = std::sync::mpsc::sync_channel(1);
    let checking = std::thread::spawn(move || {
        let frame = decoder
            .decode_input_frame_shared_live(&next)
            .unwrap()
            .unwrap();
        let held = decoder.session_diagnostics().live_held_frames;
        let pending = decoder.lookaheads.len();
        finished.send((frame, held, pending)).unwrap();
        decoder
    });
    // The gate, not elapsed speed, proves the hold: the worker can never
    // stream or finish before release, so only a hold can answer.
    let (frame, held, pending) = ready
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("live acquisition must not wait for a stalled look-ahead");
    assert!(
        Arc::ptr_eq(&frame, &served),
        "the live tick holds the served frame"
    );
    assert_eq!(held, 1);
    assert_eq!(
        pending, 1,
        "a held tick keeps the pending run instead of joining it"
    );
    let mut decoder = checking.join().unwrap();
    stall.release();
    // After the run ends, exact acquisition returns the exact frame.
    let position = served.pts_ms + 33;
    let after = decoder
        .decode_input_frame_shared(&input(&path, width, height, position))
        .unwrap()
        .unwrap();
    let reference = exact(&path, width, height, position);
    assert_eq!(after.pts_ms, reference.pts_ms);
    assert_eq!(after.data, reference.data);
}
