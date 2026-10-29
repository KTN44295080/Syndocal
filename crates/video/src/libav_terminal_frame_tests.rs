//! EOF lookahead must retain an immutable terminal owner, not a pixel copy.
use super::*;
use protocol::VideoSourceSummary;

fn request(path: Option<&std::path::Path>) -> VideoFrameRequest {
    VideoFrameRequest {
        layer_id: 1,
        label: "EOF ownership".into(),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: path.map(|path| path.to_string_lossy().into_owned()),
            name: None,
            codec: Some("ffv1".into()),
            metadata: None,
        },
        position_ms: 1_000,
        width: 48,
        height: 32,
    }
}

#[test]
fn eof_reuses_terminal_pixels_while_the_previous_owner_remains_live() {
    let request = request(None);
    let terminal = Arc::new(VideoFrame {
        layer_id: request.layer_id,
        width: request.width,
        height: request.height,
        pts_ms: 967,
        duration_ms: 33,
        format: crate::VideoPixelFormat::Rgba8,
        data: (0..48 * 32 * 4).map(|i| (i % 251) as u8).collect(),
    });
    let mut hardware_frames = 0;
    let frame = finish_decoded_candidate_with_stop(
        &request,
        None,
        Some(&terminal),
        false,
        &mut hardware_frames,
        &StopState::new(None),
    )
    .unwrap();
    assert_eq!(frame.data, terminal.data);
    assert_eq!(frame.pts_ms, terminal.pts_ms);
    assert_eq!(
        frame.data.as_ptr(),
        terminal.data.as_ptr(),
        "EOF must not allocate another full-size pixel buffer"
    );
    assert_eq!(hardware_frames, 0);
}

#[test]
fn eof_sharing_still_rejects_cancelled_or_missing_terminal_frames() {
    let request = request(None);
    let stop = StopState::new(None);
    let mut hardware_frames = 0;
    assert!(finish_decoded_candidate_with_stop(
        &request,
        None,
        None,
        false,
        &mut hardware_frames,
        &stop
    )
    .is_err());
    let terminal = Arc::new(rgba_video_frame(&request, 967, vec![0; 48 * 32 * 4]));
    stop.cancel();
    assert_eq!(
        finish_decoded_candidate_with_stop(
            &request,
            None,
            Some(&terminal),
            false,
            &mut hardware_frames,
            &stop
        ),
        Err(VideoDecodeError::Cancelled)
    );
}

#[test]
fn eof_run_keeps_shared_terminal_pixels_during_full_wrap_predecode() {
    let path = std::env::temp_dir().join(format!("syndocal-eof-owner-{}.mkv", std::process::id()));
    let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let output = std::process::Command::new(ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=48x32:rate=30:duration=1",
            "-c:v",
            "ffv1",
            "-g",
            "10",
            "-y",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let request = request(Some(&path));
    let input = VideoRenderInput::legacy(request.clone());
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let terminal = decoder
        .decode_tick(std::slice::from_ref(&input), false)
        .pop()
        .unwrap()
        .unwrap()
        .unwrap();
    let mut session = decoder.sessions.pop().unwrap();
    assert!(session.decoder_drained);
    let control = std::sync::Mutex::new(LibavRunControl {
        until_ms: 1_100,
        limit: 4,
        outstanding: 0,
        finished: false,
        clip_end: false,
        wrap_until_ms: 91,
    });
    let (tx, rx) = std::sync::mpsc::channel();
    let (result, _) =
        run_libav_lookahead(request, &mut session, Arc::clone(&terminal), &control, tx);
    let result = result.unwrap();
    assert_eq!(result.data, terminal.data);
    assert_eq!(
        result.data.as_ptr(),
        terminal.data.as_ptr(),
        "the terminal result stays live while the next pass is decoded"
    );
    assert!(rx.try_iter().next().is_none());
    let wrap = session
        .wrap
        .as_ref()
        .expect("same full wrap predecode remains available");
    assert_eq!(wrap.frames.len(), 4);
    assert!(wrap.frames.back().unwrap().0.pts_ms >= 91);
    assert!(session.decoder_drained);
    drop(session);
    drop(decoder);
    std::fs::remove_file(path).unwrap();
}
