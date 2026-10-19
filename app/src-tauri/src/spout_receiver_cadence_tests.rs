//! Opt-in, receive-only measurement of the explicitly named Task 17 QA sender.
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires a running QA Task17 Spout sender named by SYNDOCAL_TASK17_SPOUT_SENDER"]
fn task17_qa_spout_receiver_cadence() {
    let name =
        std::env::var("SYNDOCAL_TASK17_SPOUT_SENDER").expect("set the exact QA Task17 sender name");
    assert!(name.starts_with("QA Task17"), "refusing a non-QA sender");
    let mut receiver = spout2::dx::Receiver::new(Some(&name)).unwrap();
    assert!(
        receiver.sender_list().iter().any(|sender| sender == &name),
        "exact QA sender is unavailable"
    );
    let info = receiver
        .sender_info(&name)
        .expect("QA sender information is unavailable");
    let (mut width, mut height) = (info.width, info.height);
    assert!(width > 0 && height > 0 && width <= 16_384 && height <= 16_384);
    let mut pixels = vec![0_u8; width as usize * height as usize * 4];
    let connected_deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let connected = receiver
            .receive_image(&mut pixels, width, height, false, false)
            .unwrap();
        if receiver.is_updated() {
            (width, height) = receiver.sender_size();
            assert!(width > 0 && height > 0 && width <= 16_384 && height <= 16_384);
            pixels.resize(width as usize * height as usize * 4, 0);
        } else if connected && receiver.is_frame_new() {
            assert_eq!(
                receiver.sender_name(),
                name,
                "SDK attached to a different sender"
            );
            break;
        }
        assert!(
            Instant::now() < connected_deadline,
            "QA receiver did not connect within 3 seconds"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let duration = Duration::from_secs(25);
    let started = Instant::now();
    let mut previous = None;
    let mut intervals = Vec::with_capacity(3_000);
    let mut received = 0_u64;
    let mut first_sdk = None;
    let mut last_sdk = None;
    let mut sdk_skips = 0_u64;
    let mut sdk_valid = true;
    let mut sdk_frames = 0_u64;
    let mut content_previous = None;
    let mut content_last = None;
    let mut content_changes = 0_u64;
    let mut content_intervals = Vec::with_capacity(900);
    while started.elapsed() < duration {
        let connected = receiver
            .receive_image(&mut pixels, width, height, false, false)
            .unwrap();
        assert!(
            !receiver.is_updated(),
            "sender dimensions changed during cadence measurement"
        );
        if connected && receiver.is_frame_new() {
            assert_eq!(
                receiver.sender_name(),
                name,
                "SDK attached to a different sender"
            );
            let now = Instant::now();
            received += 1;
            // Spout's frame counter can be disabled in the machine's SDK
            // configuration, in which case is_frame_new() remains true even
            // for the same texture. Never report polling speed as delivery.
            let fingerprint =
                pixels
                    .chunks_exact(4)
                    .step_by(997)
                    .fold(0xcbf29ce484222325_u64, |hash, pixel| {
                        (hash ^ u32::from_le_bytes(pixel.try_into().unwrap()) as u64)
                            .wrapping_mul(0x100000001b3)
                    });
            if content_previous.replace(fingerprint) != Some(fingerprint) {
                if let Some(last) = content_last.replace(now) {
                    content_intervals.push(now.duration_since(last));
                }
                content_changes += 1;
            }
            let sdk = receiver.sender_frame();
            if sdk > 0 {
                first_sdk.get_or_insert(sdk);
                let old_sdk = last_sdk.replace(sdk);
                if let Some(previous) = old_sdk {
                    assert!(
                        sdk >= previous,
                        "sender SDK frame counter reset; measurement invalid"
                    );
                    sdk_skips += sdk.saturating_sub(previous).saturating_sub(1) as u64;
                }
                if old_sdk != Some(sdk) {
                    sdk_frames += 1;
                    if let Some(previous) = previous.replace(now) {
                        intervals.push(now.duration_since(previous));
                    }
                }
            } else {
                sdk_valid = false;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(received >= 2, "no receiver cadence could be measured");
    intervals.sort_unstable();
    content_intervals.sort_unstable();
    let cadence_available = sdk_valid && sdk_frames >= 2;
    let percentile_ms = |percentile: usize| {
        intervals[(intervals.len() * percentile).div_ceil(100) - 1].as_secs_f64() * 1_000.0
    };
    eprintln!(
        "TASK17_SPOUT_RECEIVER {}",
        serde_json::json!({
            "sender": name, "width": width, "height": height,
            "seconds": started.elapsed().as_secs_f64(), "receivedFrames": received,
            "deliveredFps": cadence_available.then(|| sdk_frames as f64 / started.elapsed().as_secs_f64()),
            "frameTimeP50Ms": cadence_available.then(|| percentile_ms(50)),
            "frameTimeP95Ms": cadence_available.then(|| percentile_ms(95)),
            "frameTimeP99Ms": cadence_available.then(|| percentile_ms(99)),
            "senderCadenceAvailable": cadence_available,
            "sdkFramesObserved": sdk_frames,
            "contentChangesObserved": content_changes,
            "contentChangeFps": content_changes as f64 / started.elapsed().as_secs_f64(),
            "contentChangeP95Ms": (!content_intervals.is_empty()).then(|| content_intervals[(content_intervals.len() * 95).div_ceil(100) - 1].as_secs_f64() * 1_000.0),
            "sdkFrameDelta": cadence_available.then(|| last_sdk.unwrap()-first_sdk.unwrap()),
            "sdkSkippedFrames": cadence_available.then_some(sdk_skips),
            "boundary": "independent_spout_receiver", "pollSleepMs": 1,
            "limitation": "receiver intervals include polling/readback cost; disabled SDK frame count makes sender cadence unavailable; sampled content changes are source motion, not duplicate sender submissions"
        })
    );
}
