#[cfg(test)]
mod live_audio_input_tests {
    use super::*;
    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    use crate::asio_bridge_v2::StreamCallbackHooks;
    use std::cell::Cell;
    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    use std::{collections::VecDeque, sync::atomic::AtomicUsize};

    fn wasapi_live_audio_request() -> LiveAudioInputStartRequest {
        LiveAudioInputStartRequest {
            backend: LiveAudioInputBackend::WasapiShared,
            device_id: None,
            sample_rate: None,
            stream_channels: None,
            sample_format: None,
            buffer_frames: None,
            channel_mix: LiveAudioChannelMix::AverageAll,
        }
    }

    fn feature_frame(
        bands: [f32; audio::live_features::MAX_LIVE_AUDIO_BANDS],
        band_count: usize,
    ) -> audio::live_features::LiveAudioFeatureFrame {
        audio::live_features::LiveAudioFeatureFrame {
            sequence: 7,
            end_sample: 96_000,
            sample_rate: 48_000,
            rms: 0.4,
            peak: 0.8,
            bands,
            band_count,
            spectral_flux: 0.3,
            spectral_centroid: 0.55,
            spectral_density_fast: 0.6,
            spectral_density_slow: 0.45,
            kick_strength: 0.9,
            snare_strength: 0.2,
            kick_pulse: true,
            snare_pulse: false,
            onset: true,
            onset_strength: 0.6,
            bpm: Some(120.0),
            bpm_confidence: 0.75,
        }
    }

    fn assert_reactive_levels_zero(levels: &LiveAudioInputLevels) {
        assert_eq!((levels.bass, levels.mid, levels.high), (0.0, 0.0, 0.0));
        assert!(levels.bands.iter().all(|level| *level == 0.0));
        assert_eq!(levels.band_count, 0);
        assert_eq!(
            (levels.rms, levels.peak, levels.spectral_flux),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(
            (
                levels.spectral_centroid,
                levels.spectral_density_fast,
                levels.spectral_density_slow,
                levels.kick_strength,
                levels.snare_strength,
            ),
            (0.0, 0.0, 0.0, 0.0, 0.0)
        );
        assert!(!levels.kick_event);
        assert!(!levels.snare_event);
        assert!(!levels.onset);
        assert_eq!(levels.onset_strength, 0.0);
        assert_eq!(levels.bpm, None);
        assert_eq!((levels.bpm_confidence, levels.beat_phase), (0.0, 0.0));
        assert_eq!(levels.feature_sequence, 0);
    }

    #[test]
    fn live_audio_levels_are_clamped_and_zeroed_when_unsafe() {
        let mut bands = [0.0; audio::live_features::MAX_LIVE_AUDIO_BANDS];
        bands[0] = f32::NAN;
        bands[1] = -0.25;
        bands[2] = 0.5;
        bands[3] = f32::INFINITY;
        bands[4] = 1.25;
        let live = LiveAudioInputLevels::from_status(&LiveAudioInputStatus {
            running: true,
            bass: -0.25,
            mid: 0.5,
            high: 1.25,
            bands,
            band_count: usize::MAX,
            rms: f32::NAN,
            peak: 1.5,
            spectral_flux: -0.5,
            spectral_centroid: 0.75,
            spectral_density_fast: 1.5,
            spectral_density_slow: f32::NAN,
            kick_strength: 0.9,
            snare_strength: -0.5,
            kick_event: true,
            snare_event: true,
            onset: true,
            onset_strength: 0.6,
            bpm: Some(120.0),
            bpm_confidence: 1.5,
            beat_phase: 1.25,
            feature_sequence: 42,
            ..LiveAudioInputStatus::default()
        });
        assert_eq!((live.bass, live.mid, live.high), (0.0, 0.5, 1.0));
        assert_eq!(&live.bands[..5], &[0.0, 0.0, 0.5, 0.0, 1.0]);
        assert_eq!(live.band_count, audio::live_features::MAX_LIVE_AUDIO_BANDS);
        assert_eq!((live.rms, live.peak, live.spectral_flux), (0.0, 1.0, 0.0));
        assert_eq!(
            (
                live.spectral_centroid,
                live.spectral_density_fast,
                live.spectral_density_slow,
                live.kick_strength,
                live.snare_strength,
            ),
            (0.75, 1.0, 0.0, 0.9, 0.0)
        );
        assert!(live.kick_event);
        assert!(live.snare_event);
        assert!(live.onset);
        assert_eq!(live.onset_strength, 0.6);
        assert_eq!(live.bpm, Some(120.0));
        assert_eq!((live.bpm_confidence, live.beat_phase), (1.0, 0.25));
        assert_eq!(live.feature_sequence, 42);

        let invalid_bpm = LiveAudioInputLevels::from_status(&LiveAudioInputStatus {
            running: true,
            bpm: Some(-120.0),
            ..LiveAudioInputStatus::default()
        });
        assert_eq!(invalid_bpm.bpm, None);

        for status in [
            LiveAudioInputStatus {
                running: true,
                stale: true,
                bass: 0.25,
                mid: 0.5,
                high: 0.75,
                bands: [1.0; audio::live_features::MAX_LIVE_AUDIO_BANDS],
                band_count: audio::live_features::MAX_LIVE_AUDIO_BANDS,
                rms: 1.0,
                peak: 1.0,
                spectral_flux: 1.0,
                spectral_centroid: 1.0,
                spectral_density_fast: 1.0,
                spectral_density_slow: 1.0,
                kick_strength: 1.0,
                snare_strength: 1.0,
                kick_event: true,
                snare_event: true,
                onset: true,
                onset_strength: 1.0,
                bpm: Some(120.0),
                bpm_confidence: 1.0,
                beat_phase: 0.5,
                feature_sequence: 42,
                ..LiveAudioInputStatus::default()
            },
            LiveAudioInputStatus {
                running: true,
                safety_clear_pending: true,
                bass: 0.25,
                mid: 0.5,
                high: 0.75,
                bands: [1.0; audio::live_features::MAX_LIVE_AUDIO_BANDS],
                band_count: audio::live_features::MAX_LIVE_AUDIO_BANDS,
                rms: 1.0,
                peak: 1.0,
                spectral_flux: 1.0,
                spectral_centroid: 1.0,
                spectral_density_fast: 1.0,
                spectral_density_slow: 1.0,
                kick_strength: 1.0,
                snare_strength: 1.0,
                kick_event: true,
                snare_event: true,
                onset: true,
                onset_strength: 1.0,
                bpm: Some(120.0),
                bpm_confidence: 1.0,
                beat_phase: 0.5,
                feature_sequence: 42,
                ..LiveAudioInputStatus::default()
            },
            LiveAudioInputStatus {
                running: false,
                bass: 0.25,
                mid: 0.5,
                high: 0.75,
                bands: [1.0; audio::live_features::MAX_LIVE_AUDIO_BANDS],
                band_count: audio::live_features::MAX_LIVE_AUDIO_BANDS,
                rms: 1.0,
                peak: 1.0,
                spectral_flux: 1.0,
                spectral_centroid: 1.0,
                spectral_density_fast: 1.0,
                spectral_density_slow: 1.0,
                kick_strength: 1.0,
                snare_strength: 1.0,
                kick_event: true,
                snare_event: true,
                onset: true,
                onset_strength: 1.0,
                bpm: Some(120.0),
                bpm_confidence: 1.0,
                beat_phase: 0.5,
                feature_sequence: 42,
                ..LiveAudioInputStatus::default()
            },
        ] {
            let levels = LiveAudioInputLevels::from_status(&status);
            assert_reactive_levels_zero(&levels);
        }
    }

    #[test]
    fn live_audio_feature_spectrum_groups_all_sixteen_bands_and_honors_band_count() {
        let mut bands = [0.0; audio::live_features::MAX_LIVE_AUDIO_BANDS];
        bands[0] = 0.1;
        bands[4] = 0.5;
        bands[5] = 0.2;
        bands[9] = 0.7;
        bands[10] = 0.3;
        bands[14] = f32::NAN;
        bands[15] = 1.25;

        let spectrum = live_audio_feature_spectrum(&feature_frame(bands, 16));
        assert_eq!(spectrum.time_ms, 2_000);
        assert_eq!(
            (spectrum.bass, spectrum.mid, spectrum.high),
            (0.5, 0.7, 1.0)
        );

        let partial = live_audio_feature_spectrum(&feature_frame(bands, 8));
        assert_eq!((partial.bass, partial.mid, partial.high), (0.5, 0.2, 0.0));
    }

    #[test]
    fn live_audio_reactive_transport_carries_and_sanitizes_every_descriptor() {
        let mut bands = std::array::from_fn(|index| index as f32 / 15.0);
        bands[0] = f32::NAN;
        let mut frame = feature_frame(bands, usize::MAX);
        frame.rms = -1.0;
        frame.peak = 2.0;
        frame.spectral_flux = f32::INFINITY;
        frame.spectral_centroid = 0.55;
        frame.spectral_density_fast = 0.65;
        frame.spectral_density_slow = 0.45;
        frame.kick_strength = 0.9;
        frame.snare_strength = 0.25;
        frame.kick_pulse = true;
        frame.snare_pulse = false;
        frame.onset = true;
        frame.onset_strength = 0.7;
        frame.bpm = Some(128.0);
        frame.bpm_confidence = 0.85;

        let features = live_audio_reactive_features(&LiveAudioFeaturePresentation {
            frame,
            beat_phase: 1.25,
        });

        assert_eq!(
            usize::from(features.band_count),
            protocol::LIVE_AUDIO_FEATURE_BAND_CAPACITY
        );
        assert_eq!(features.bands[0], 0.0);
        assert_eq!(features.bands[15], 1.0);
        assert_eq!(
            (features.rms, features.peak, features.spectral_flux),
            (0.0, 1.0, 0.0)
        );
        assert_eq!(features.spectral_centroid, 0.55);
        assert_eq!(features.spectral_density_fast, 0.65);
        assert_eq!(features.spectral_density_slow, 0.45);
        assert_eq!(
            (features.kick_strength, features.snare_strength),
            (0.9, 0.25)
        );
        assert!(features.kick_event);
        assert!(!features.snare_event);
        assert!(features.onset);
        assert_eq!(features.onset_strength, 0.7);
        assert_eq!(features.bpm, Some(128.0));
        assert_eq!(features.bpm_confidence, 0.85);
        assert_eq!(features.beat_phase, 0.25);
    }

    #[test]
    fn coalesced_drum_pulses_are_not_lost_before_the_next_publish() {
        let mut previous = feature_frame([0.0; audio::live_features::MAX_LIVE_AUDIO_BANDS], 16);
        previous.kick_pulse = true;
        previous.snare_pulse = false;
        let mut current = feature_frame([0.0; audio::live_features::MAX_LIVE_AUDIO_BANDS], 16);
        current.kick_pulse = false;
        current.snare_pulse = true;

        coalesce_live_audio_feature_pulses(&mut current, Some(&previous));

        assert!(current.kick_pulse);
        assert!(current.snare_pulse);
    }

    #[test]
    fn live_audio_feature_beat_phase_is_relative_to_the_last_real_onset() {
        let frame = feature_frame([0.0; audio::live_features::MAX_LIVE_AUDIO_BANDS], 16);
        assert_eq!(live_audio_feature_beat_phase(&frame, Some(90_000)), 0.25);
        assert_eq!(
            live_audio_feature_beat_phase(&frame, Some(frame.end_sample)),
            0.0
        );
        assert_eq!(live_audio_feature_beat_phase(&frame, None), 0.0);

        for bpm in [None, Some(f32::NAN), Some(0.0), Some(-120.0)] {
            let invalid = audio::live_features::LiveAudioFeatureFrame { bpm, ..frame };
            assert_eq!(live_audio_feature_beat_phase(&invalid, Some(90_000)), 0.0);
        }
    }

    #[test]
    fn live_audio_feature_publish_sanitizes_the_full_status_contract() {
        let mut bands = [0.0; audio::live_features::MAX_LIVE_AUDIO_BANDS];
        bands[0] = f32::NAN;
        bands[1] = -1.0;
        bands[2] = 0.5;
        bands[3] = f32::INFINITY;
        bands[4] = 2.0;
        bands[5] = 0.75;
        let mut frame = feature_frame(bands, 6);
        frame.rms = f32::NAN;
        frame.peak = f32::INFINITY;
        frame.spectral_flux = -1.0;
        frame.spectral_centroid = 1.5;
        frame.spectral_density_fast = f32::NAN;
        frame.spectral_density_slow = 0.4;
        frame.kick_strength = -0.25;
        frame.snare_strength = 0.65;
        frame.kick_pulse = true;
        frame.snare_pulse = true;
        frame.onset_strength = 2.0;
        frame.bpm = Some(-120.0);
        frame.bpm_confidence = f32::NAN;
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();

        assert!(publish_live_audio_analysis_with(
            &status,
            &safety,
            &protocol::AudioSpectrumPoint {
                time_ms: 0,
                bass: f32::NAN,
                mid: -1.0,
                high: f32::INFINITY,
            },
            Some(&LiveAudioFeaturePresentation {
                frame,
                beat_phase: f32::NAN,
            }),
            || Ok(()),
            || Ok(()),
        ));

        let current = status.lock().unwrap().clone();
        assert_eq!((current.bass, current.mid, current.high), (0.0, 0.0, 0.0));
        assert_eq!(&current.bands[..6], &[0.0, 0.0, 0.5, 0.0, 1.0, 0.75]);
        assert!(current.bands[6..].iter().all(|level| *level == 0.0));
        assert_eq!(current.band_count, 6);
        assert_eq!(
            (current.rms, current.peak, current.spectral_flux),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(
            (
                current.spectral_centroid,
                current.spectral_density_fast,
                current.spectral_density_slow,
                current.kick_strength,
                current.snare_strength,
            ),
            (1.0, 0.0, 0.4, 0.0, 0.65)
        );
        assert!(current.kick_event);
        assert!(current.snare_event);
        assert!(current.onset);
        assert_eq!(current.onset_strength, 1.0);
        assert_eq!(current.bpm, None);
        assert_eq!((current.bpm_confidence, current.beat_phase), (0.0, 0.0));
        assert_eq!(current.feature_sequence, 7);
        assert!(serde_json::to_value(current).is_ok());
    }

    #[test]
    fn zero_live_audio_status_clears_every_reactive_feature() {
        let mut status = LiveAudioInputStatus {
            running: true,
            bass: 0.2,
            mid: 0.4,
            high: 0.6,
            bands: [0.8; audio::live_features::MAX_LIVE_AUDIO_BANDS],
            band_count: audio::live_features::MAX_LIVE_AUDIO_BANDS,
            rms: 0.3,
            peak: 0.9,
            spectral_flux: 0.5,
            spectral_centroid: 0.7,
            spectral_density_fast: 0.8,
            spectral_density_slow: 0.6,
            kick_strength: 0.9,
            snare_strength: 0.4,
            kick_event: true,
            snare_event: true,
            onset: true,
            onset_strength: 0.7,
            bpm: Some(128.0),
            bpm_confidence: 0.85,
            beat_phase: 0.25,
            feature_sequence: 99,
            ..LiveAudioInputStatus::default()
        };

        zero_live_audio_input_status(&mut status, "stale".to_string());

        let levels = LiveAudioInputLevels::from_status(&status);
        assert!(status.stale);
        assert_eq!(status.last_error.as_deref(), Some("stale"));
        assert_reactive_levels_zero(&levels);
        assert!(status.bands.iter().all(|level| *level == 0.0));
        assert_eq!(status.band_count, 0);
        assert_eq!(status.bpm, None);
        assert_eq!(status.feature_sequence, 0);
    }

    #[test]
    fn pending_live_audio_onsets_publish_as_one_atomic_retryable_payload() {
        let mut pending = PendingLiveAudioOnsets::default();
        for sequence in 10..10 + LIVE_AUDIO_CAPTURE_SLOT_COUNT as u64 {
            pending.push(sequence).unwrap();
        }
        assert_eq!(pending.push(99), Err(LIVE_AUDIO_ONSET_QUEUE_OVERFLOW_ERROR));

        assert_eq!(
            pending.publish_frame_with(|sequences, count| {
                assert_eq!(usize::from(count), LIVE_AUDIO_CAPTURE_SLOT_COUNT);
                assert_eq!(&sequences[..usize::from(count)], &[10, 11, 12, 13]);
                Err("frame queue full".to_string())
            }),
            Err("frame queue full".to_string())
        );
        assert_eq!(pending.len, LIVE_AUDIO_CAPTURE_SLOT_COUNT);
        assert_eq!(&pending.sequences[..pending.len], &[10, 11, 12, 13]);

        let mut delivered = [0_u64; LIVE_AUDIO_CAPTURE_SLOT_COUNT];
        let delivered_len = Cell::new(0_usize);
        pending
            .publish_frame_with(|sequences, count| {
                let len = usize::from(count);
                delivered[..len].copy_from_slice(&sequences[..len]);
                delivered_len.set(len);
                Ok(())
            })
            .unwrap();
        assert_eq!(&delivered[..delivered_len.get()], &[10, 11, 12, 13]);
        assert_eq!(pending.len, 0);
    }

    #[test]
    fn recoverable_stale_rotates_the_generation_shared_with_stop_and_fault_paths() {
        let initial = allocate_live_audio_generation().unwrap();
        let active = Arc::new(AtomicU64::new(initial));
        let stop_owner = Arc::clone(&active);

        let recovered = rotate_live_audio_generation(&active).unwrap();

        assert!(recovered > initial);
        assert_eq!(stop_owner.load(Ordering::Acquire), recovered);
    }

    #[test]
    fn live_audio_generation_allocation_is_checked_and_never_wraps() {
        let cell = AtomicU64::new(0);
        assert_eq!(allocate_live_audio_generation_from(&cell), Ok(1));
        assert_eq!(allocate_live_audio_generation_from(&cell), Ok(2));
        assert_eq!(cell.load(Ordering::Acquire), 2);

        let saturated = AtomicU64::new(u64::MAX);
        assert_eq!(
            allocate_live_audio_generation_from(&saturated),
            Err("Live audio input generation counter was exhausted".to_string()),
            "overflow must reject instead of wrapping onto identity 0"
        );
        assert_eq!(
            saturated.load(Ordering::Acquire),
            u64::MAX,
            "a rejected allocation must not consume identity 0"
        );
        assert_eq!(saturated.load(Ordering::Acquire), u64::MAX);
    }

    #[test]
    fn device_catalog_generation_bump_is_checked_and_never_wraps() {
        assert_eq!(checked_next_device_catalog_generation(0), Ok(1));
        assert_eq!(checked_next_device_catalog_generation(41), Ok(42));
        assert_eq!(
            checked_next_device_catalog_generation(u64::MAX),
            Err("Live audio input device catalog generation was exhausted; restart Syndocal before refreshing devices".to_string()),
            "device ids embed the generation, so wrap-around would resurrect stale identities"
        );
    }

    fn callback_info(device_delay: Duration) -> rodio::cpal::InputCallbackInfo {
        let capture = rodio::cpal::StreamInstant::new(1, 0);
        let callback = capture.add(device_delay).unwrap();
        rodio::cpal::InputCallbackInfo::new(rodio::cpal::InputStreamTimestamp { callback, capture })
    }

    fn capture_slot_pool(
        slots: usize,
    ) -> (
        crossbeam_queue::ArrayQueue<LiveAudioSampleChunk>,
        crossbeam_queue::ArrayQueue<LiveAudioSampleChunk>,
    ) {
        let free = crossbeam_queue::ArrayQueue::new(slots);
        let ready = crossbeam_queue::ArrayQueue::new(slots);
        for _ in 0..slots {
            free.push(LiveAudioSampleChunk::new()).unwrap();
        }
        (free, ready)
    }

    fn supported_config(
        channels: u16,
        sample_rate: u32,
        sample_format: rodio::cpal::SampleFormat,
        buffer_size: rodio::cpal::SupportedBufferSize,
    ) -> rodio::cpal::SupportedStreamConfig {
        rodio::cpal::SupportedStreamConfig::new(
            channels,
            rodio::cpal::SampleRate(sample_rate),
            buffer_size,
            sample_format,
        )
    }

    fn supported_range(
        channels: u16,
        min_sample_rate: u32,
        max_sample_rate: u32,
        sample_format: rodio::cpal::SampleFormat,
        buffer_size: rodio::cpal::SupportedBufferSize,
    ) -> rodio::cpal::SupportedStreamConfigRange {
        rodio::cpal::SupportedStreamConfigRange::new(
            channels,
            rodio::cpal::SampleRate(min_sample_rate),
            rodio::cpal::SampleRate(max_sample_rate),
            buffer_size,
            sample_format,
        )
    }

    #[test]
    fn live_audio_device_labels_and_opaque_ids_remain_unambiguous() {
        let labels = disambiguate_live_audio_input_labels(&[
            "Microphone".to_string(),
            "Microphone".to_string(),
            "Microphone (2)".to_string(),
        ]);
        assert_eq!(labels[0], "Microphone");
        assert_eq!(labels[1], "Microphone (2)");
        assert_eq!(labels[2], "Microphone (2) (2)");
        assert_eq!(labels.iter().collect::<HashSet<_>>().len(), labels.len());

        let first = live_audio_input_device_id("WASAPI", 7, 0);
        assert_ne!(first, live_audio_input_device_id("WASAPI", 7, 1));
        assert_ne!(first, live_audio_input_device_id("WASAPI", 8, 0));
        assert_eq!(first, "wasapi:7:0");
    }

    #[test]
    fn live_audio_runtime_storage_channel_mix_stays_snake_case() {
        assert_eq!(
            serde_json::to_value(LiveAudioChannelMix::StereoPair {
                left_channel_index: 0,
                right_channel_index: 1,
            })
            .unwrap(),
            json!({
                "mode": "stereo_pair",
                "left_channel_index": 0,
                "right_channel_index": 1
            })
        );
        assert_eq!(
            serde_json::to_value(LiveAudioChannelMix::Single { channel_index: 3 }).unwrap(),
            json!({ "mode": "single", "channel_index": 3 })
        );
        assert_eq!(
            serde_json::to_value(LiveAudioBufferCapability::Range {
                min_frames: 64,
                max_frames: 512,
            })
            .unwrap(),
            json!({ "kind": "range", "min_frames": 64, "max_frames": 512 })
        );
    }

    #[test]
    fn live_audio_channel_mix_runtime_validation_stays_internal_and_fail_closed() {
        assert_eq!(
            LiveAudioChannelMix::StereoPair {
                left_channel_index: 0,
                right_channel_index: 1,
            }
            .validate(2),
            Ok(())
        );
        assert!(LiveAudioChannelMix::StereoPair {
            left_channel_index: 0,
            right_channel_index: 0,
        }
        .validate(2)
        .is_err());
    }

    #[test]
    fn live_audio_backend_contract_keeps_asio_explicit_and_separate() {
        let backends = live_audio_input_backends();
        assert_eq!(backends.len(), 2);
        assert_eq!(backends[0].id, "wasapi_shared");
        assert!(!backends[0].requires_explicit_device);
        assert_eq!(backends[0].distribution, "default");
        assert_eq!(
            backends[0].availability,
            if cfg!(target_os = "windows") {
                "ready"
            } else {
                "unsupported"
            }
        );
        if backends[0].availability == "ready" {
            assert!(backends[0].availability_detail.is_none());
        } else {
            assert!(backends[0]
                .availability_detail
                .as_deref()
                .is_some_and(|detail| !detail.trim().is_empty()));
        }
        assert_eq!(backends[1].id, "asio");
        assert!(backends[1].requires_explicit_device);
        assert_eq!(backends[1].distribution, "separate_artifact");
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        let expected_asio_built = true;
        #[cfg(not(all(target_os = "windows", target_arch = "x86_64", feature = "asio")))]
        let expected_asio_built = false;
        assert_eq!(backends[1].built, expected_asio_built);
        assert!(matches!(
            backends[1].availability.as_str(),
            "ready" | "not_packaged" | "fault" | "unsupported"
        ));
        if backends[1].availability == "ready" {
            assert!(backends[1].availability_detail.is_none());
        } else {
            assert!(backends[1]
                .availability_detail
                .as_deref()
                .is_some_and(|detail| !detail.trim().is_empty()));
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        match asio_bridge_v2::probe_canonical_bridge() {
            asio_bridge_v2::AsioBridgeAvailability::Ready(_) => {
                assert_eq!(backends[1].availability, "ready")
            }
            asio_bridge_v2::AsioBridgeAvailability::NotPackaged { .. } => {
                assert_eq!(backends[1].availability, "not_packaged")
            }
            asio_bridge_v2::AsioBridgeAvailability::Fault(_) => {
                assert_eq!(backends[1].availability, "fault")
            }
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86_64", feature = "asio")))]
        assert_eq!(backends[1].availability, "unsupported");
    }

    #[test]
    fn live_audio_backend_availability_wire_contract_covers_every_probe_class() {
        let cases = [
            (LiveAudioInputAvailability::Ready, None, true, "ready"),
            (
                LiveAudioInputAvailability::NotPackaged,
                Some("canonical bridge is not packaged"),
                true,
                "not_packaged",
            ),
            (
                LiveAudioInputAvailability::Fault,
                Some("canonical bridge failed to load"),
                true,
                "fault",
            ),
            (
                LiveAudioInputAvailability::Unsupported,
                Some("backend is unsupported on this target"),
                false,
                "unsupported",
            ),
        ];
        for (availability, detail, built, expected_wire) in cases {
            let probe = LiveAudioInputBackendProbe {
                availability,
                detail: detail.map(str::to_owned),
                built,
            };
            assert_eq!(probe.availability.as_str(), expected_wire);
            assert_eq!(probe.built, built);
            if availability == LiveAudioInputAvailability::Ready {
                assert!(probe.detail.is_none());
            } else {
                assert!(probe
                    .detail
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()));
            }
        }
    }

    #[cfg(all(test, target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    const ASIO_CONTRACT_DRIVER: &str = "asio:TOPPING Pro USB Audio Device";

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    fn asio_contract_selection() -> asio_bridge_v2::PersistedAsioSelection {
        asio_bridge_v2::PersistedAsioSelection::new(
            ASIO_CONTRACT_DRIVER.to_owned(),
            "TOPPING Pro USB Audio Device".to_owned(),
            48_000,
            2,
            "i32".to_owned(),
            128,
            vec![
                asio_bridge_v2::ChannelMixGainJson {
                    channel_index: 0,
                    gain: 0.5,
                },
                asio_bridge_v2::ChannelMixGainJson {
                    channel_index: 1,
                    gain: 0.5,
                },
            ],
        )
        .unwrap()
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[path = "../../live_audio_input_tests/asio_selection_restore_tests.rs"]
    mod asio_selection_restore_tests;

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    fn asio_contract_catalog_json(driver_id: &str) -> String {
        serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [{
                "id": driver_id,
                "name": "TOPPING Pro USB Audio Device",
                "inputChannels": 2,
                "sampleFormats": ["f32", "i24", "i32"],
                "sampleRatesHz": [44100, 48000, 96000],
                "bufferFrames": {"min": 8, "max": 2048}
            }]
        }))
        .unwrap()
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    fn asio_contract_capabilities_json(rate: u32) -> String {
        serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "capabilities",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "driver": {
                "id": ASIO_CONTRACT_DRIVER,
                "name": "TOPPING Pro USB Audio Device",
                "inputChannels": 2,
                "sampleFormats": ["f32", "i24", "i32"],
                "sampleRatesHz": [44100, 48000, 96000],
                "bufferFrames": {"min": 8, "max": 2048}
            },
            "inputConfigs": [{
                "channels": 2,
                "sampleFormat": "i32",
                "sampleRateHz": rate,
                "bufferFrames": {"min": 8, "max": 2048}
            }]
        }))
        .unwrap()
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    struct AsioContractHooks {
        events: Mutex<Vec<String>>,
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    impl AsioContractHooks {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
            })
        }

        fn recorded(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    impl asio_bridge_v2::StreamCallbackHooks for AsioContractHooks {
        fn on_samples(&self, mono_samples: &[f32], capture_delay_ns: u64) {
            self.events
                .lock()
                .unwrap()
                .push(format!("samples:{}:{capture_delay_ns}", mono_samples.len()));
        }

        fn on_terminal_latch(&self, kind: u32) {
            self.events.lock().unwrap().push(format!("terminal:{kind}"));
        }
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    struct AsioContractTransport {
        start_results:
            Mutex<VecDeque<Result<asio_bridge_v2::RawStart, asio_bridge_v2::BridgeCallFailure>>>,
        stop_results: Mutex<VecDeque<Result<String, asio_bridge_v2::BridgeCallFailure>>>,
        close_results: Mutex<VecDeque<Result<String, asio_bridge_v2::BridgeCallFailure>>>,
        stop_calls: AtomicU64,
        close_calls: AtomicU64,
        close_slots_seen: Mutex<Vec<usize>>,
        captured_sample_fn: std::sync::atomic::AtomicUsize,
        captured_event_fn: std::sync::atomic::AtomicUsize,
        captured_context: std::sync::atomic::AtomicUsize,
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    impl AsioContractTransport {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                start_results: Mutex::new(std::collections::VecDeque::new()),
                stop_results: Mutex::new(std::collections::VecDeque::new()),
                close_results: Mutex::new(std::collections::VecDeque::new()),
                stop_calls: AtomicU64::new(0),
                close_calls: AtomicU64::new(0),
                close_slots_seen: Mutex::new(Vec::new()),
                captured_sample_fn: AtomicUsize::new(0),
                captured_event_fn: AtomicUsize::new(0),
                captured_context: AtomicUsize::new(0),
            })
        }

        fn push_start_success(&self, handle_value: usize) {
            self.start_results
                .lock()
                .unwrap()
                .push_back(Ok(asio_bridge_v2::RawStart {
                    handle: asio_bridge_v2::RawStreamHandle(handle_value as *mut std::ffi::c_void),
                    result_json: serde_json::to_string(&serde_json::json!({
                        "schemaVersion": 2,
                        "kind": "start",
                        "actualBufferFrames": 128
                    }))
                    .unwrap(),
                }));
        }

        fn invoke_captured_sample(&self, samples: &[f32], capture_delay_ns: u64) {
            let context = self.captured_context.load(Ordering::Acquire) as *mut std::ffi::c_void;
            assert!(!context.is_null(), "start must capture a callback context");
            let sample_fn = unsafe {
                std::mem::transmute::<usize, asio_bridge_v2::SampleCallbackFn>(
                    self.captured_sample_fn.load(Ordering::Acquire),
                )
            };
            unsafe {
                sample_fn(
                    context,
                    samples.as_ptr(),
                    samples.len(),
                    capture_delay_ns,
                    samples.len() as u32,
                )
            };
        }

        fn invoke_captured_event(&self, severity: u32, kind: u32) {
            let context = self.captured_context.load(Ordering::Acquire) as *mut std::ffi::c_void;
            assert!(!context.is_null(), "start must capture a callback context");
            let event_fn = unsafe {
                std::mem::transmute::<usize, asio_bridge_v2::EventCallbackFn>(
                    self.captured_event_fn.load(Ordering::Acquire),
                )
            };
            unsafe { event_fn(context, severity, kind, std::ptr::null(), 0) };
        }
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    impl asio_bridge_v2::BridgeTransport for AsioContractTransport {
        fn start(
            &self,
            _request_json: &[u8],
            sample: asio_bridge_v2::SampleCallbackFn,
            event: asio_bridge_v2::EventCallbackFn,
            context: *mut std::ffi::c_void,
        ) -> Result<asio_bridge_v2::RawStart, asio_bridge_v2::BridgeCallFailure> {
            self.captured_sample_fn
                .store(sample as usize, Ordering::Release);
            self.captured_event_fn
                .store(event as usize, Ordering::Release);
            self.captured_context
                .store(context as usize, Ordering::Release);
            self.start_results
                .lock()
                .unwrap()
                .pop_front()
                .expect("contract transport was not scripted for start")
        }

        fn stop(
            &self,
            handle: asio_bridge_v2::RawStreamHandle,
        ) -> Result<String, asio_bridge_v2::BridgeCallFailure> {
            self.stop_calls.fetch_add(1, Ordering::Relaxed);
            assert!(!handle.is_null());
            self.stop_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Ok(serde_json::to_string(&serde_json::json!({
                        "schemaVersion": 2,
                        "kind": "stop",
                        "stopped": true,
                        "streamWasActive": true
                    }))
                    .unwrap())
                })
        }

        fn telemetry_json(
            &self,
            _handle: asio_bridge_v2::RawStreamHandle,
        ) -> Result<String, asio_bridge_v2::BridgeCallFailure> {
            Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "telemetry",
                "callbacks": 1,
                "xruns": 0,
                "callbackDurationNs": {"p50": 1, "p95": 1, "p99": 1, "max": 1},
                "captureDelayNs": {"p50": 1, "p95": 1, "p99": 1, "max": 1}
            }))
            .unwrap())
        }

        fn close(
            &self,
            slot: &mut asio_bridge_v2::RawCloseSlot,
        ) -> Result<String, asio_bridge_v2::BridgeCallFailure> {
            self.close_calls.fetch_add(1, Ordering::Relaxed);
            self.close_slots_seen.lock().unwrap().push(slot.0 as usize);
            slot.0 = std::ptr::null_mut();
            self.close_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Ok(serde_json::to_string(&serde_json::json!({
                        "schemaVersion": 2,
                        "kind": "close",
                        "handleReleased": true,
                        "streamWasActive": true
                    }))
                    .unwrap())
                })
        }
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    fn contract_session(
        transport: &Arc<AsioContractTransport>,
        live_generation: Arc<AtomicU64>,
        hooks: Option<Arc<dyn asio_bridge_v2::StreamCallbackHooks>>,
    ) -> asio_bridge_v2::AsioStreamSession<AsioContractTransport> {
        asio_bridge_v2::publish_generation_and_start(
            Arc::clone(transport),
            &asio_contract_selection().start_request().unwrap(),
            live_generation,
            hooks,
        )
        .unwrap()
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_probe_loads_only_the_canonical_dll_filename_at_the_probed_directory() {
        let directory = env::temp_dir().join(format!(
            "syndocal_asio_contract_probe_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        // Only the exact canonical filename is ever consulted.  The
        // `probe_canonical_bridge_at` seam shares the canonical probe's
        // resolution path without touching process environment state, so
        // alias independence is proven deterministically here and override
        // independence follows structurally: no code path reads any
        // environment variable, so there is nothing to race over.
        fs::write(directory.join("syndocal-asio-bridge.dll"), b"decoy").unwrap();
        let availability_with_alias = asio_bridge_v2::probe_canonical_bridge_at(&directory);
        assert_eq!(availability_with_alias.label(), "NOT_PACKAGED");

        fs::write(directory.join("syndocal_asio_bridge.dll"), b"not a dll").unwrap();
        let availability_with_canonical_garbage =
            asio_bridge_v2::probe_canonical_bridge_at(&directory);
        assert_eq!(availability_with_canonical_garbage.label(), "FAULT");
        let _ = fs::remove_dir_all(&directory);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_start_revalidation_fails_closed_on_stale_catalog_and_capabilities() {
        let selection = asio_contract_selection();

        let missing_driver = asio_contract_catalog_json("asio:HOTONE AUDIO USB Audio Device");
        match asio_bridge_v2::revalidate_selection_with_catalog(&selection, &missing_driver) {
            asio_bridge_v2::SelectionRevalidationOutcome::StillLocked { reason, message } => {
                assert_eq!(reason, asio_bridge_v2::StaleLockReason::DriverMissing);
                assert!(message.contains(ASIO_CONTRACT_DRIVER));
                assert!(message.contains("stays locked"));
            }
            other => panic!("expected StillLocked for a missing driver, got {other:?}"),
        }

        match asio_bridge_v2::revalidate_selection_with_capabilities(
            &selection,
            &asio_contract_capabilities_json(44_100),
        ) {
            asio_bridge_v2::SelectionRevalidationOutcome::StillLocked { reason, .. } => {
                assert_eq!(reason, asio_bridge_v2::StaleLockReason::ConfigurationDrift);
            }
            other => panic!("expected StillLocked for rate drift, got {other:?}"),
        }

        match asio_bridge_v2::revalidate_selection_with_catalog(
            &selection,
            &asio_contract_catalog_json(ASIO_CONTRACT_DRIVER),
        ) {
            asio_bridge_v2::SelectionRevalidationOutcome::Revalidated {
                current_driver_id, ..
            } => assert_eq!(current_driver_id, ASIO_CONTRACT_DRIVER),
            other => panic!("expected Revalidated, got {other:?}"),
        }

        let request = match asio_bridge_v2::revalidate_selection_with_capabilities(
            &selection,
            &asio_contract_capabilities_json(48_000),
        ) {
            asio_bridge_v2::SelectionRevalidationOutcome::Revalidated { start_request, .. } => {
                start_request
            }
            other => panic!("expected Revalidated capabilities, got {other:?}"),
        };
        assert_eq!(request.driver_id, ASIO_CONTRACT_DRIVER);
        assert_eq!(request.sample_rate_hz, 48_000);
        assert_eq!(request.input_channels, 2);
        assert_eq!(request.sample_format, "i32");
        assert_eq!(request.fixed_buffer_frames, 128);
        assert_eq!(request.channel_mix.len(), 2);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_close_owns_the_handle_exactly_once_and_reports_failures_visibly() {
        let transport = AsioContractTransport::new();
        transport.push_start_success(0xFEED);
        let session = contract_session(&transport, Arc::new(AtomicU64::new(0)), None);
        assert_eq!(
            teardown_asio_v2_stream(session),
            Ok(()),
            "an orderly teardown stops once and closes once"
        );
        assert_eq!(transport.stop_calls.load(Ordering::Acquire), 1);
        assert_eq!(transport.close_calls.load(Ordering::Acquire), 1);
        assert_eq!(
            transport.close_slots_seen.lock().unwrap().as_slice(),
            &[0xFEED]
        );

        let failing = AsioContractTransport::new();
        failing.push_start_success(0xBEEF);
        failing
            .stop_results
            .lock()
            .unwrap()
            .push_back(Err(asio_bridge_v2::BridgeCallFailure {
                status: asio_bridge_v2::BridgeStatusOrUnknown::Known(
                    asio_bridge_v2::BridgeStatus::BackendError,
                ),
                error_payload: None,
                detail: "stop exploded".to_owned(),
            }));
        failing
            .close_results
            .lock()
            .unwrap()
            .push_back(Err(asio_bridge_v2::BridgeCallFailure {
                status: asio_bridge_v2::BridgeStatusOrUnknown::Known(
                    asio_bridge_v2::BridgeStatus::Terminal,
                ),
                error_payload: None,
                detail: "close exploded".to_owned(),
            }));
        let session = contract_session(&failing, Arc::new(AtomicU64::new(0)), None);
        let teardown_error = teardown_asio_v2_stream(session).unwrap_err();
        assert!(teardown_error.contains("stop exploded"));
        assert!(teardown_error.contains("close exploded"));
        assert_eq!(failing.stop_calls.load(Ordering::Acquire), 1);
        assert_eq!(failing.close_calls.load(Ordering::Acquire), 1);
        assert_eq!(
            failing.close_slots_seen.lock().unwrap().as_slice(),
            &[0xBEEF],
            "close receives the raw handle even when the DLL reports failure"
        );

        let dropped = AsioContractTransport::new();
        dropped.push_start_success(0xD00D);
        let session = contract_session(&dropped, Arc::new(AtomicU64::new(0)), None);
        let context = Arc::clone(session.context());
        drop(session);
        assert_eq!(dropped.close_calls.load(Ordering::Acquire), 0);
        let snapshot = context.snapshot();
        assert!(snapshot.safety_zero_requested);
        assert_eq!(
            snapshot.latched_terminal_kind,
            Some(asio_bridge_v2::INTERNAL_FAULT_DROP_WITHOUT_CLOSE)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_callback_generation_fencing_gates_the_application_hooks() {
        let transport = AsioContractTransport::new();
        transport.push_start_success(0xFACE);
        let hooks = AsioContractHooks::new();
        let live_generation = Arc::new(AtomicU64::new(0));
        let session = contract_session(
            &transport,
            Arc::clone(&live_generation),
            Some(Arc::clone(&hooks) as Arc<dyn asio_bridge_v2::StreamCallbackHooks>),
        );
        assert_eq!(session.generation(), 1);

        let clean = [0.25_f32; 16];
        transport.invoke_captured_sample(&clean, 100);
        assert_eq!(hooks.recorded(), vec!["samples:16:100".to_owned()]);

        live_generation.store(99, Ordering::Release);
        transport.invoke_captured_sample(&clean, 200);
        transport.invoke_captured_event(2, 5);
        assert_eq!(
            hooks.recorded(),
            vec!["samples:16:100".to_owned()],
            "stale-generation callbacks must never reach the application hooks"
        );
        let snapshot = session.context().snapshot();
        assert_eq!(
            snapshot.stale_callbacks_total, 2,
            "the stale sample and stale event are independently counted while both stay fenced from hooks",
        );
        assert_eq!(snapshot.latched_terminal_kind, None);
        assert!(!snapshot.safety_zero_requested);
        drop(session);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_terminal_events_latch_once_and_freeze_the_application_hooks() {
        let transport = AsioContractTransport::new();
        transport.push_start_success(0xC0DE);
        let hooks = AsioContractHooks::new();
        let session = contract_session(
            &transport,
            Arc::new(AtomicU64::new(0)),
            Some(Arc::clone(&hooks) as Arc<dyn asio_bridge_v2::StreamCallbackHooks>),
        );

        let clean = [0.25_f32; 16];
        transport.invoke_captured_sample(&clean, 500);
        assert_eq!(hooks.recorded(), vec!["samples:16:500".to_owned()]);

        transport.invoke_captured_event(2, 5);
        let snapshot = session.context().snapshot();
        assert_eq!(snapshot.latched_terminal_kind, Some(5));
        assert!(snapshot.safety_zero_requested);

        let clean_after_fault = [0.5_f32; 16];
        transport.invoke_captured_sample(&clean_after_fault, 900);
        transport.invoke_captured_event(2, 6);
        assert_eq!(
            hooks.recorded(),
            vec!["samples:16:500".to_owned(), "terminal:5".to_owned()],
            "delivery freezes after the one-shot terminal latch and later faults stay silent"
        );
        drop(session);

        let message = deferred_live_audio_terminal_fault_message(DeferredLiveAudioTerminalFault {
            source: LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE,
            kind: 5,
        });
        assert!(message.contains("explicit ASIO input device was lost"));
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_internal_terminal_kinds_are_visible_and_latched_once() {
        for (kind, expected_fragment) in [
            (
                asio_bridge_v2::INTERNAL_FAULT_DROP_WITHOUT_CLOSE,
                "without an explicit Close",
            ),
            (
                asio_bridge_v2::INTERNAL_FAULT_MALFORMED_EVENT,
                "event payload was malformed",
            ),
            (
                asio_bridge_v2::INTERNAL_FAULT_FRAME_MISMATCH,
                "frame count changed after Start",
            ),
            (
                asio_bridge_v2::INTERNAL_FAULT_NONFINITE_SAMPLE,
                "non-finite sample reached the ASIO callback",
            ),
            (6, "callback gap exceeded 250 ms"),
            (1, "ASIO input xrun"),
        ] {
            let message =
                deferred_live_audio_terminal_fault_message(DeferredLiveAudioTerminalFault {
                    source: LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE,
                    kind,
                });
            assert!(
                message.contains(&format!("ASIO terminal fault kind")),
                "unexpected message shape: {message}"
            );
            assert!(
                message.contains(expected_fragment),
                "kind {kind} message {message:?} lacks {expected_fragment:?}"
            );
        }

        let latch = DeferredLiveAudioTerminalFaultLatch::default();
        assert!(latch.latch(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 2));
        assert!(!latch.latch(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 3));
        let claimed = latch.claim().unwrap();
        assert_eq!(claimed.source, LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE);
        assert_eq!(claimed.kind, 2);
        assert!(latch.claim().is_none());
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
    #[test]
    fn asio_stream_hooks_feed_the_bounded_capture_queues_and_stop_on_terminal() {
        let (free_slots, ready_chunks) = capture_slot_pool(4);
        let free_slots = Arc::new(free_slots);
        let ready_chunks = Arc::new(ready_chunks);
        let capture_telemetry = Arc::new(LiveAudioInputCaptureTelemetry::default());
        let safety = Arc::new(LiveAudioInputSafety::default());
        let deferred = Arc::new(DeferredLiveAudioTerminalFaultLatch::default());
        let hooks = AsioV2StreamHooks {
            sample_rate: 48_000,
            free_capture_slots: Arc::clone(&free_slots),
            ready_capture_chunks: Arc::clone(&ready_chunks),
            capture_telemetry,
            safety: Arc::clone(&safety),
            deferred_terminal_fault: Arc::clone(&deferred),
            worker_wake: std::thread::current(),
        };

        let samples = [0.5_f32; 128];
        hooks.on_samples(&samples, 1_000);
        assert_eq!(ready_chunks.len(), 1);
        let chunk = ready_chunks.pop().unwrap();
        assert_eq!(chunk.len, 128);
        assert!(chunk.samples[..128].iter().all(|value| *value == 0.5));

        hooks.on_terminal_latch(5);
        assert!(safety.terminal_faulted());
        let fault = deferred.claim().unwrap();
        assert_eq!(fault.source, LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE);
        assert_eq!(fault.kind, 5);

        hooks.on_samples(&samples, 2_000);
        assert_eq!(
            ready_chunks.len(),
            0,
            "the terminal fault must stop further queueing"
        );
        assert!(deferred.claim().is_none());
    }

    #[test]
    fn live_audio_config_selection_preserves_the_default_when_auto() {
        let default = supported_config(
            2,
            48_000,
            rodio::cpal::SampleFormat::F32,
            rodio::cpal::SupportedBufferSize::Range {
                min: 64,
                max: 2_048,
            },
        );
        let selected = resolve_live_audio_stream_config(
            &default,
            &[],
            &LiveAudioInputStartRequest {
                stream_channels: Some(2),
                sample_format: Some("f32".to_string()),
                ..wasapi_live_audio_request()
            },
        )
        .unwrap();
        assert_eq!(selected.stream_config.channels, 2);
        assert_eq!(selected.stream_config.sample_rate.0, 48_000);
        assert_eq!(
            selected.stream_config.buffer_size,
            rodio::cpal::BufferSize::Default
        );
        assert_eq!(selected.sample_format, rodio::cpal::SampleFormat::F32);
        assert_eq!(
            selected.buffer_capability,
            LiveAudioBufferCapability::Range {
                min_frames: 64,
                max_frames: 2_048,
            }
        );
        assert_eq!(selected.configured_buffer_frames, None);
        assert_eq!(selected.channel_mix, LiveAudioChannelMix::AverageAll);

        let explicit_default = resolve_live_audio_stream_config(
            &default,
            &[],
            &LiveAudioInputStartRequest {
                sample_rate: Some(48_000),
                stream_channels: Some(2),
                sample_format: Some("f32".to_string()),
                ..wasapi_live_audio_request()
            },
        )
        .unwrap();
        assert_eq!(explicit_default.sample_format, selected.sample_format);
        assert_eq!(explicit_default.stream_config, selected.stream_config);
    }

    #[test]
    fn live_audio_config_selection_is_exact_and_prefers_the_default_sample_format() {
        let default = supported_config(
            2,
            44_100,
            rodio::cpal::SampleFormat::I16,
            rodio::cpal::SupportedBufferSize::Range {
                min: 64,
                max: 1_024,
            },
        );
        let supported = vec![
            supported_range(
                2,
                48_000,
                48_000,
                rodio::cpal::SampleFormat::F32,
                rodio::cpal::SupportedBufferSize::Range {
                    min: 64,
                    max: 1_024,
                },
            ),
            supported_range(
                2,
                48_000,
                48_000,
                rodio::cpal::SampleFormat::I16,
                rodio::cpal::SupportedBufferSize::Range {
                    min: 64,
                    max: 1_024,
                },
            ),
        ];
        let request = LiveAudioInputStartRequest {
            sample_rate: Some(48_000),
            stream_channels: Some(2),
            sample_format: Some("i16".to_string()),
            buffer_frames: Some(128),
            channel_mix: LiveAudioChannelMix::Single { channel_index: 1 },
            ..wasapi_live_audio_request()
        };
        let selected = resolve_live_audio_stream_config(&default, &supported, &request).unwrap();
        assert_eq!(selected.sample_format, rodio::cpal::SampleFormat::I16);
        assert_eq!(selected.stream_config.sample_rate.0, 48_000);
        assert_eq!(selected.stream_config.channels, 2);
        assert_eq!(
            selected.stream_config.buffer_size,
            rodio::cpal::BufferSize::Fixed(128)
        );
        assert_eq!(selected.configured_buffer_frames, Some(128));

        let preview_request = LiveAudioInputStartRequest {
            buffer_frames: None,
            sample_format: None,
            ..request.clone()
        };
        let preview =
            resolve_live_audio_stream_config(&default, &supported, &preview_request).unwrap();
        assert_eq!(preview.sample_format, selected.sample_format);
        assert_eq!(
            preview.stream_config.channels,
            selected.stream_config.channels
        );

        let known_buffer_supported = vec![
            supported_range(
                2,
                48_000,
                48_000,
                rodio::cpal::SampleFormat::I16,
                rodio::cpal::SupportedBufferSize::Unknown,
            ),
            supported_range(
                2,
                48_000,
                48_000,
                rodio::cpal::SampleFormat::F32,
                rodio::cpal::SupportedBufferSize::Range {
                    min: 64,
                    max: 1_024,
                },
            ),
        ];
        assert_eq!(
            resolve_live_audio_stream_config(
                &default,
                &known_buffer_supported,
                &LiveAudioInputStartRequest {
                    sample_format: None,
                    ..request.clone()
                },
            )
            .unwrap()
            .sample_format,
            rodio::cpal::SampleFormat::F32,
            "a verified fixed-buffer range must outrank an unknown default-format range"
        );

        let unsupported = LiveAudioInputStartRequest {
            sample_rate: Some(96_000),
            ..wasapi_live_audio_request()
        };
        assert!(resolve_live_audio_stream_config(&default, &supported, &unsupported).is_err());
    }

    #[test]
    fn live_audio_config_selection_rejects_unsafe_or_out_of_range_buffers() {
        let default = supported_config(
            1,
            8_000,
            rodio::cpal::SampleFormat::F32,
            rodio::cpal::SupportedBufferSize::Range {
                min: 64,
                max: 10_000,
            },
        );
        for frames in [0, 1_601, 8_193] {
            let request = LiveAudioInputStartRequest {
                buffer_frames: Some(frames),
                ..wasapi_live_audio_request()
            };
            assert!(
                resolve_live_audio_stream_config(&default, &[], &request).is_err(),
                "buffer {frames} should be rejected"
            );
        }

        let narrow = supported_config(
            2,
            48_000,
            rodio::cpal::SampleFormat::F32,
            rodio::cpal::SupportedBufferSize::Range { min: 64, max: 512 },
        );
        let out_of_range = LiveAudioInputStartRequest {
            buffer_frames: Some(32),
            ..wasapi_live_audio_request()
        };
        assert!(resolve_live_audio_stream_config(&narrow, &[], &out_of_range).is_err());

        let unknown = supported_config(
            2,
            48_000,
            rodio::cpal::SampleFormat::F32,
            rodio::cpal::SupportedBufferSize::Unknown,
        );
        let exact_attempt = LiveAudioInputStartRequest {
            buffer_frames: Some(256),
            ..wasapi_live_audio_request()
        };
        assert_eq!(
            resolve_live_audio_stream_config(&unknown, &[], &exact_attempt)
                .unwrap()
                .stream_config
                .buffer_size,
            rodio::cpal::BufferSize::Fixed(256)
        );

        let explicit_rate_default = supported_config(
            2,
            44_100,
            rodio::cpal::SampleFormat::I16,
            rodio::cpal::SupportedBufferSize::Range { min: 64, max: 512 },
        );
        let heterogeneous = vec![
            supported_range(
                2,
                48_000,
                48_000,
                rodio::cpal::SampleFormat::I16,
                rodio::cpal::SupportedBufferSize::Range { min: 64, max: 64 },
            ),
            supported_range(
                2,
                48_000,
                48_000,
                rodio::cpal::SampleFormat::F32,
                rodio::cpal::SupportedBufferSize::Range {
                    min: 64,
                    max: 1_024,
                },
            ),
        ];
        let preview = resolve_live_audio_stream_config(
            &explicit_rate_default,
            &heterogeneous,
            &LiveAudioInputStartRequest {
                sample_rate: Some(48_000),
                ..wasapi_live_audio_request()
            },
        )
        .unwrap();
        assert_eq!(preview.sample_format, rodio::cpal::SampleFormat::I16);
        let start = LiveAudioInputStartRequest {
            sample_rate: Some(48_000),
            stream_channels: Some(preview.stream_config.channels),
            sample_format: Some(preview.sample_format.to_string()),
            buffer_frames: Some(128),
            ..wasapi_live_audio_request()
        };
        assert!(
            resolve_live_audio_stream_config(&explicit_rate_default, &heterogeneous, &start)
                .is_err(),
            "a fixed buffer must be rejected instead of silently switching to another format"
        );
    }

    #[test]
    fn live_audio_channel_mix_validates_and_selects_channels_without_allocation() {
        assert_eq!(
            mix_live_audio_frame(
                &[1.0_f32, -1.0, 0.5],
                LiveAudioChannelMix::AverageAll,
                &|value| value,
            ),
            1.0 / 6.0
        );
        assert_eq!(
            mix_live_audio_frame(
                &[1.0_f32, -1.0, 0.5],
                LiveAudioChannelMix::Single { channel_index: 1 },
                &|value| value,
            ),
            -1.0
        );
        assert_eq!(
            mix_live_audio_frame(
                &[1.0_f32, -1.0, 0.5],
                LiveAudioChannelMix::StereoPair {
                    left_channel_index: 0,
                    right_channel_index: 2,
                },
                &|value| value,
            ),
            0.75
        );
        assert!(LiveAudioChannelMix::Single { channel_index: 3 }
            .validate(3)
            .is_err());
        assert!(LiveAudioChannelMix::StereoPair {
            left_channel_index: 1,
            right_channel_index: 1,
        }
        .validate(2)
        .is_err());
    }

    #[test]
    fn every_cpal_input_sample_format_normalizes_to_f32() {
        fn normalized<T>(value: T) -> f32
        where
            T: rodio::cpal::Sample,
            f32: rodio::cpal::FromSample<T>,
        {
            <f32 as rodio::cpal::Sample>::from_sample(value)
        }
        fn assert_signed<T>(min: T, equilibrium: T, max: T)
        where
            T: rodio::cpal::Sample + std::fmt::Debug,
            f32: rodio::cpal::FromSample<T>,
        {
            assert!(normalized(min) <= -0.99);
            assert!(normalized(equilibrium).abs() <= f32::EPSILON);
            assert!(normalized(max) >= 0.98);
        }
        fn assert_unsigned<T>(min: T, equilibrium: T, max: T)
        where
            T: rodio::cpal::Sample + std::fmt::Debug,
            f32: rodio::cpal::FromSample<T>,
        {
            assert!(normalized(min) <= -0.99);
            assert!(normalized(equilibrium).abs() <= f32::EPSILON);
            assert!(normalized(max) >= 0.98);
        }

        assert_signed(i8::MIN, 0_i8, i8::MAX);
        assert_signed(i16::MIN, 0_i16, i16::MAX);
        assert_signed(
            rodio::cpal::I24::new(-(1 << 23)).unwrap(),
            rodio::cpal::I24::new(0).unwrap(),
            rodio::cpal::I24::new((1 << 23) - 1).unwrap(),
        );
        assert_signed(i32::MIN, 0_i32, i32::MAX);
        assert_signed(i64::MIN, 0_i64, i64::MAX);
        assert_unsigned(u8::MIN, <u8 as rodio::cpal::Sample>::EQUILIBRIUM, u8::MAX);
        assert_unsigned(
            u16::MIN,
            <u16 as rodio::cpal::Sample>::EQUILIBRIUM,
            u16::MAX,
        );
        assert_unsigned(
            u32::MIN,
            <u32 as rodio::cpal::Sample>::EQUILIBRIUM,
            u32::MAX,
        );
        assert_unsigned(
            u64::MIN,
            <u64 as rodio::cpal::Sample>::EQUILIBRIUM,
            u64::MAX,
        );
        assert_eq!(normalized(-1.0_f32), -1.0);
        assert_eq!(normalized(0.0_f32), 0.0);
        assert_eq!(normalized(1.0_f32), 1.0);
        assert_eq!(normalized(-1.0_f64), -1.0);
        assert_eq!(normalized(0.0_f64), 0.0);
        assert_eq!(normalized(1.0_f64), 1.0);
    }

    #[test]
    fn live_audio_callback_downmixes_interleaved_channels_without_blocking() {
        let (free_capture_slots, ready_capture_chunks) = capture_slot_pool(1);
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        let safety = LiveAudioInputSafety::default();
        let worker_wake = std::thread::current();
        queue_live_audio_samples(
            &[1.0_f32, -1.0, 0.5, 0.25],
            2,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::from_millis(3)),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &worker_wake,
            ),
            |value| value,
        );
        let mut first = ready_capture_chunks.pop().unwrap();
        assert_eq!(&first.samples[..first.len], &[0.0, 0.375]);
        assert_eq!(first.device_delay, Duration::from_millis(3));
        first.len = 0;
        free_capture_slots.push(first).unwrap();

        queue_live_audio_samples(
            &[1.0_f32, 1.0],
            2,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::ZERO),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &worker_wake,
            ),
            |value| value,
        );
        let second = ready_capture_chunks.pop().unwrap();
        assert_eq!(&second.samples[..second.len], &[1.0]);
        assert_eq!(capture_telemetry.dropped_chunks.load(Ordering::Relaxed), 0);
        assert_eq!(capture_telemetry.dropped_frames.load(Ordering::Relaxed), 0);
        assert_eq!(capture_telemetry.callback_count.load(Ordering::Relaxed), 2);
        assert_eq!(
            capture_telemetry
                .last_callback_frames
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            capture_telemetry
                .min_callback_frames
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            capture_telemetry
                .max_callback_frames
                .load(Ordering::Relaxed),
            2
        );
        assert_eq!(
            capture_telemetry
                .queue_depth_high_water
                .load(Ordering::Relaxed),
            1
        );
        let status = Mutex::new(LiveAudioInputStatus::default());
        sync_live_audio_capture_telemetry(&status, &ready_capture_chunks, &capture_telemetry);
        let status = status.lock().unwrap();
        assert_eq!(status.callback_count, 2);
        assert_eq!(status.last_callback_frames, 1);
        assert_eq!(status.min_callback_frames, 1);
        assert_eq!(status.max_callback_frames, 2);
        assert_eq!(status.queue_depth_high_water, 1);
    }

    #[test]
    fn live_audio_callback_replaces_the_oldest_slot_when_the_pool_is_exhausted() {
        let (free_capture_slots, ready_capture_chunks) = capture_slot_pool(1);
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        let safety = LiveAudioInputSafety::default();
        let worker_wake = std::thread::current();

        queue_live_audio_samples(
            &[0.0_f32, 0.5],
            1,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::ZERO),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &worker_wake,
            ),
            |value| value,
        );
        queue_live_audio_samples(
            &[1.0_f32],
            1,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::ZERO),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &worker_wake,
            ),
            |value| value,
        );

        let retained = ready_capture_chunks.pop().unwrap();
        assert_eq!(&retained.samples[..retained.len], &[1.0]);
        assert_eq!(capture_telemetry.dropped_chunks.load(Ordering::Relaxed), 1);
        assert_eq!(capture_telemetry.dropped_frames.load(Ordering::Relaxed), 2);
        assert_eq!(capture_telemetry.callback_count.load(Ordering::Relaxed), 2);
        assert_eq!(
            capture_telemetry
                .max_callback_frames
                .load(Ordering::Relaxed),
            2
        );
    }

    #[test]
    fn live_audio_callback_splits_large_buffers_without_losing_order() {
        let (free_capture_slots, ready_capture_chunks) = capture_slot_pool(2);
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        let safety = LiveAudioInputSafety::default();
        let input = (0..LIVE_AUDIO_CAPTURE_SLOT_FRAMES + 2)
            .map(|index| index as f32)
            .collect::<Vec<_>>();

        queue_live_audio_samples(
            &input,
            1,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::from_millis(100)),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &std::thread::current(),
            ),
            |value| value,
        );

        let first = ready_capture_chunks.pop().unwrap();
        let second = ready_capture_chunks.pop().unwrap();
        assert_eq!(first.len, LIVE_AUDIO_CAPTURE_SLOT_FRAMES);
        assert_eq!(first.samples[0], 0.0);
        assert_eq!(first.samples[first.len - 1], (first.len - 1) as f32);
        assert_eq!(second.samples[0], LIVE_AUDIO_CAPTURE_SLOT_FRAMES as f32);
        assert_eq!(
            second.samples[1],
            (LIVE_AUDIO_CAPTURE_SLOT_FRAMES + 1) as f32
        );
        assert_eq!(first.device_delay, Duration::from_millis(100));
        assert_eq!(
            second.device_delay,
            Duration::from_millis(100).saturating_sub(live_audio_frame_duration(
                LIVE_AUDIO_CAPTURE_SLOT_FRAMES,
                48_000
            ))
        );
        assert_eq!(capture_telemetry.dropped_chunks.load(Ordering::Relaxed), 0);
        assert!(free_capture_slots.is_empty());
        assert!(ready_capture_chunks.is_empty());
    }

    #[test]
    fn live_audio_callback_bounds_conversion_to_the_newest_pool_capacity() {
        let (free_capture_slots, ready_capture_chunks) =
            capture_slot_pool(LIVE_AUDIO_CAPTURE_SLOT_COUNT);
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        let safety = LiveAudioInputSafety::default();
        let capture_capacity = LIVE_AUDIO_CAPTURE_SLOT_COUNT * LIVE_AUDIO_CAPTURE_SLOT_FRAMES;
        let skipped_frames = 100;
        let input = (0..capture_capacity + skipped_frames)
            .map(|index| index as f32)
            .collect::<Vec<_>>();
        let converted = Cell::new(0_usize);

        queue_live_audio_samples(
            &input,
            1,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::from_millis(200)),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &std::thread::current(),
            ),
            |value| {
                converted.set(converted.get() + 1);
                value
            },
        );

        assert_eq!(converted.get(), capture_capacity);
        assert_eq!(ready_capture_chunks.len(), LIVE_AUDIO_CAPTURE_SLOT_COUNT);
        let first = ready_capture_chunks.pop().unwrap();
        assert_eq!(first.samples[0], skipped_frames as f32);
        assert_eq!(capture_telemetry.dropped_chunks.load(Ordering::Relaxed), 1);
        assert_eq!(
            capture_telemetry.dropped_frames.load(Ordering::Relaxed),
            skipped_frames as u64
        );
    }

    #[test]
    fn live_audio_callback_sanitizes_non_finite_pcm_and_recovers() {
        let (free_capture_slots, ready_capture_chunks) = capture_slot_pool(1);
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        let safety = LiveAudioInputSafety::default();

        queue_live_audio_samples(
            &[f32::NAN, 1.0, f32::INFINITY, f32::NEG_INFINITY],
            2,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::ZERO),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &std::thread::current(),
            ),
            |value| value,
        );
        let mut invalid = ready_capture_chunks.pop().unwrap();
        assert_eq!(&invalid.samples[..invalid.len], &[0.5, 0.0]);
        invalid.len = 0;
        free_capture_slots.push(invalid).unwrap();

        queue_live_audio_samples(
            &[0.25_f32, 0.75],
            2,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::ZERO),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &std::thread::current(),
            ),
            |value| value,
        );
        let recovered = ready_capture_chunks.pop().unwrap();
        assert_eq!(&recovered.samples[..recovered.len], &[0.5]);
    }

    #[test]
    fn live_audio_watchdog_stales_at_the_deadline_once_and_rearms() {
        let started = Instant::now();
        let mut watchdog = LiveAudioInputWatchdog::new(started);

        assert_eq!(
            watchdog.receive_wait(started + Duration::from_millis(249)),
            Duration::from_millis(1)
        );
        assert!(!watchdog.mark_stale_if_due(started + Duration::from_millis(249)));
        assert!(watchdog.mark_stale_if_due(started + Duration::from_millis(250)));
        assert!(!watchdog.mark_stale_if_due(started + Duration::from_secs(1)));

        assert!(watchdog.record_chunk(started + Duration::from_millis(300)));
        assert!(!watchdog.mark_stale_if_due(started + Duration::from_millis(549)));
        assert!(watchdog.mark_stale_if_due(started + Duration::from_millis(550)));

        let mut starved_worker = LiveAudioInputWatchdog::new(started);
        assert!(starved_worker.record_chunk(started + LIVE_AUDIO_STALE_AFTER));
        assert!(!starved_worker
            .record_chunk(started + LIVE_AUDIO_STALE_AFTER + Duration::from_millis(1)));
    }

    #[test]
    fn live_audio_worker_heartbeat_stales_at_the_same_safety_boundary() {
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        capture_telemetry
            .worker_heartbeat_us
            .store(1, Ordering::Release);
        let stale_after_us = LIVE_AUDIO_STALE_AFTER.as_micros() as u64;

        assert!(!capture_telemetry.worker_heartbeat_is_stale_at(stale_after_us));
        assert!(capture_telemetry.worker_heartbeat_is_stale_at(stale_after_us + 1));
    }

    #[test]
    fn heartbeat_stale_recheck_does_not_clear_a_generation_that_just_recovered() {
        let initial = allocate_live_audio_generation().unwrap();
        let recovered = allocate_live_audio_generation().unwrap();
        let generation = AtomicU64::new(initial);
        let generation_gate = Mutex::new(());
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        let heartbeat_stale = Cell::new(true);
        let heartbeat_checks = Cell::new(0_u8);
        let cleared_generation = Cell::new(None);

        let failed_closed = fail_closed_live_audio_if_worker_heartbeat_stale_with(
            &generation,
            &generation_gate,
            &status,
            &safety,
            || {
                let check = heartbeat_checks.get();
                heartbeat_checks.set(check + 1);
                if check == 0 {
                    let observed_stale = heartbeat_stale.get();
                    // Model the worker winning the generation gate after the status call
                    // sampled a stale heartbeat but before it can act on that sample.
                    let _worker_generation_gate = generation_gate.lock().unwrap();
                    generation.store(recovered, Ordering::Release);
                    heartbeat_stale.set(false);
                    observed_stale
                } else {
                    heartbeat_stale.get()
                }
            },
            |generation| {
                cleared_generation.set(Some(generation));
                Ok(())
            },
        )
        .unwrap();

        assert!(!failed_closed);
        assert_eq!(heartbeat_checks.get(), 2);
        assert_eq!(generation.load(Ordering::Acquire), recovered);
        assert_eq!(cleared_generation.get(), None);
        let current = status.lock().unwrap();
        assert!(!current.stale);
        assert!(current.last_error.is_none());
        assert!(!current.safety_clear_pending);
        assert!(!safety.clear_pending());
    }

    #[test]
    fn live_audio_stream_fault_zeroes_meters_and_retries_a_full_clear_queue() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            bass: 0.25,
            mid: 0.5,
            high: 0.75,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        let clear_attempts = Cell::new(0_u32);

        fail_closed_live_audio_with(&status, &safety, true, "stream failed".to_string(), || {
            clear_attempts.set(clear_attempts.get() + 1);
            Err("queue full".to_string())
        });

        let current = status.lock().unwrap().clone();
        assert!(current.running);
        assert!(current.stale);
        assert!(current.safety_clear_pending);
        assert_eq!((current.bass, current.mid, current.high), (0.0, 0.0, 0.0));
        assert_eq!(current.last_error.as_deref(), Some("stream failed"));
        assert!(safety.terminal_faulted());
        assert!(safety.clear_pending());

        assert!(retry_live_audio_clear_with(&status, &safety, || {
            clear_attempts.set(clear_attempts.get() + 1);
            Ok(())
        }));
        assert_eq!(clear_attempts.get(), 2);
        assert!(!safety.clear_pending());
        assert!(!status.lock().unwrap().safety_clear_pending);
    }

    #[test]
    fn deferred_live_audio_terminal_before_capture_start_is_finalized_once() {
        let deferred = Arc::new(DeferredLiveAudioTerminalFaultLatch::default());
        let safety = Arc::new(LiveAudioInputSafety::default());
        let errors = LiveAudioStreamErrorContext {
            deferred_terminal_fault: Arc::clone(&deferred),
            safety: Arc::clone(&safety),
            worker_wake: std::thread::current(),
        };
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            bass: 0.5,
            ..LiveAudioInputStatus::default()
        });
        let generation_gate = Mutex::new(());
        let clear_attempts = Cell::new(0_u32);

        assert!(errors.defer_terminal_fault(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 5));
        assert!(safety.terminal_faulted());
        assert!(finalize_deferred_live_audio_terminal_fault_with(
            &deferred,
            &generation_gate,
            &status,
            &safety,
            || {
                clear_attempts.set(clear_attempts.get() + 1);
                Ok(())
            },
        ));
        assert!(!finalize_deferred_live_audio_terminal_fault_with(
            &deferred,
            &generation_gate,
            &status,
            &safety,
            || panic!("a handled deferred terminal fault must not clear twice"),
        ));

        assert_eq!(clear_attempts.get(), 1);
        let current = status.lock().unwrap();
        assert!(current.running);
        assert!(current.stale);
        assert_eq!(current.bass, 0.0);
        assert_eq!(
            current.last_error.as_deref(),
            Some("ASIO terminal fault kind 5: explicit ASIO input device was lost")
        );
        assert!(!current.safety_clear_pending);
    }

    #[test]
    fn deferred_live_audio_terminal_callback_stays_nonblocking_and_clear_is_exactly_once() {
        let deferred = Arc::new(DeferredLiveAudioTerminalFaultLatch::default());
        let safety = Arc::new(LiveAudioInputSafety::default());
        let errors = LiveAudioStreamErrorContext {
            deferred_terminal_fault: Arc::clone(&deferred),
            safety: Arc::clone(&safety),
            worker_wake: std::thread::current(),
        };
        let status = Arc::new(Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        }));
        let generation_gate = Arc::new(Mutex::new(()));
        let clear_attempts = Arc::new(AtomicU64::new(0));
        let (clear_entered_tx, clear_entered_rx) = std::sync::mpsc::channel();
        let (release_clear_tx, release_clear_rx) = std::sync::mpsc::channel();

        assert!(errors.defer_terminal_fault(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 6));
        let worker_deferred = Arc::clone(&deferred);
        let worker_status = Arc::clone(&status);
        let worker_safety = Arc::clone(&safety);
        let worker_generation_gate = Arc::clone(&generation_gate);
        let worker_clear_attempts = Arc::clone(&clear_attempts);
        let worker = std::thread::spawn(move || {
            finalize_deferred_live_audio_terminal_fault_with(
                &worker_deferred,
                &worker_generation_gate,
                &worker_status,
                &worker_safety,
                || {
                    worker_clear_attempts.fetch_add(1, Ordering::AcqRel);
                    clear_entered_tx.send(()).unwrap();
                    release_clear_rx.recv().unwrap();
                    Ok(())
                },
            )
        });
        clear_entered_rx.recv().unwrap();

        let duplicate_errors = errors.clone();
        let (callback_returned_tx, callback_returned_rx) = std::sync::mpsc::channel();
        let callback = std::thread::spawn(move || {
            let latched =
                duplicate_errors.defer_terminal_fault(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 1);
            callback_returned_tx.send(latched).unwrap();
        });
        assert!(!callback_returned_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("callback-side atomic defer must not wait for the blocked engine clear"));
        release_clear_tx.send(()).unwrap();
        callback.join().unwrap();
        assert!(worker.join().unwrap());

        assert_eq!(clear_attempts.load(Ordering::Acquire), 1);
        assert!(!finalize_deferred_live_audio_terminal_fault_with(
            &deferred,
            &generation_gate,
            &status,
            &safety,
            || panic!("duplicate terminal callback must not create another clear owner"),
        ));
    }

    #[test]
    fn deferred_live_audio_terminal_races_stop_without_a_second_fail_closed_owner() {
        let deferred = Arc::new(DeferredLiveAudioTerminalFaultLatch::default());
        let safety = Arc::new(LiveAudioInputSafety::default());
        let errors = LiveAudioStreamErrorContext {
            deferred_terminal_fault: Arc::clone(&deferred),
            safety: Arc::clone(&safety),
            worker_wake: std::thread::current(),
        };
        let status = Arc::new(Mutex::new(LiveAudioInputStatus {
            running: true,
            bass: 0.5,
            ..LiveAudioInputStatus::default()
        }));
        let generation_gate = Arc::new(Mutex::new(()));
        let clear_attempts = Arc::new(AtomicU64::new(0));
        let start = Arc::new(std::sync::Barrier::new(3));

        assert!(errors.defer_terminal_fault(
            LIVE_AUDIO_TERMINAL_SOURCE_WASAPI_STREAM,
            LIVE_AUDIO_TERMINAL_KIND_DEVICE_NOT_AVAILABLE,
        ));

        let worker_deferred = Arc::clone(&deferred);
        let worker_status = Arc::clone(&status);
        let worker_safety = Arc::clone(&safety);
        let worker_generation_gate = Arc::clone(&generation_gate);
        let worker_clear_attempts = Arc::clone(&clear_attempts);
        let worker_start = Arc::clone(&start);
        let worker = std::thread::spawn(move || {
            worker_start.wait();
            finalize_deferred_live_audio_terminal_fault_with(
                &worker_deferred,
                &worker_generation_gate,
                &worker_status,
                &worker_safety,
                || {
                    worker_clear_attempts.fetch_add(1, Ordering::AcqRel);
                    Ok(())
                },
            )
        });

        let stop_status = Arc::clone(&status);
        let stop_safety = Arc::clone(&safety);
        let stop_generation_gate = Arc::clone(&generation_gate);
        let stop_clear_attempts = Arc::clone(&clear_attempts);
        let stop_start = Arc::clone(&start);
        let stop = std::thread::spawn(move || {
            stop_start.wait();
            let _generation_gate = stop_generation_gate.lock().unwrap();
            request_live_audio_shutdown_with(&stop_status, &stop_safety, || {
                stop_clear_attempts.fetch_add(1, Ordering::AcqRel);
                Ok(())
            });
        });

        start.wait();
        worker.join().unwrap();
        stop.join().unwrap();

        assert_eq!(clear_attempts.load(Ordering::Acquire), 1);
        assert!(safety.shutdown_requested());
        assert!(safety.terminal_faulted());
        assert!(status.lock().unwrap().stale);
        assert!(deferred.claim().is_none());
    }

    #[test]
    fn deferred_live_audio_terminal_does_not_reopen_after_final_drop_clear() {
        let deferred = DeferredLiveAudioTerminalFaultLatch::default();
        let safety = LiveAudioInputSafety::default();
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let generation_gate = Mutex::new(());
        let event_clears = Cell::new(0_u32);
        let drop_clears = Cell::new(0_u32);
        let generation = AtomicU64::new(41);

        safety.mark_terminal_fault();
        assert!(deferred.latch(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 2));
        assert!(finalize_deferred_live_audio_terminal_fault_with(
            &deferred,
            &generation_gate,
            &status,
            &safety,
            || {
                event_clears.set(event_clears.get() + 1);
                Ok(())
            },
        ));
        assert!(send_current_live_audio_clear_bounded_with(
            &generation,
            Duration::ZERO,
            Duration::ZERO,
            |observed_generation| {
                assert_eq!(observed_generation, 41);
                drop_clears.set(drop_clears.get() + 1);
                Ok(())
            },
        ));

        assert!(!deferred.latch(LIVE_AUDIO_TERMINAL_SOURCE_ASIO_BRIDGE, 3));
        assert!(!finalize_deferred_live_audio_terminal_fault_with(
            &deferred,
            &generation_gate,
            &status,
            &safety,
            || panic!("a late callback after final drop must stay inert"),
        ));
        assert_eq!(event_clears.get(), 1);
        assert_eq!(drop_clears.get(), 1);
    }

    #[test]
    fn live_audio_stop_keeps_the_retry_owner_inert_until_clear_is_accepted() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            bass: 0.25,
            mid: 0.5,
            high: 0.75,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        request_live_audio_shutdown_with(&status, &safety, || Err("queue full".to_string()));

        let pending = status.lock().unwrap().clone();
        assert!(pending.running);
        assert!(pending.stale);
        assert!(pending.safety_clear_pending);
        assert!(safety.shutdown_requested());
        assert!(safety.terminal_faulted());
        assert!(safety.clear_pending());

        let (free_capture_slots, ready_capture_chunks) = capture_slot_pool(1);
        let capture_telemetry = LiveAudioInputCaptureTelemetry::default();
        queue_live_audio_samples(
            &[1.0_f32, 1.0],
            2,
            48_000,
            LiveAudioChannelMix::AverageAll,
            &callback_info(Duration::ZERO),
            (
                &free_capture_slots,
                &ready_capture_chunks,
                &capture_telemetry,
                &safety,
                &std::thread::current(),
            ),
            |value| value,
        );
        assert!(ready_capture_chunks.is_empty());
        assert_eq!(free_capture_slots.len(), 1);
        assert_eq!(capture_telemetry.callback_count.load(Ordering::Relaxed), 0);

        assert!(retry_live_audio_clear_with(&status, &safety, || Ok(())));
        assert!(!safety.clear_pending());
        let accepted = status.lock().unwrap();
        assert!(accepted.running);
        assert!(!accepted.safety_clear_pending);
    }

    #[test]
    fn late_stream_fault_cannot_replace_an_accepted_stop_clear() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        request_live_audio_shutdown_with(&status, &safety, || Ok(()));
        let late_clear_attempts = Cell::new(0_u32);

        fail_closed_live_audio_with(
            &status,
            &safety,
            false,
            LIVE_AUDIO_STALE_ERROR.to_string(),
            || {
                late_clear_attempts.set(late_clear_attempts.get() + 1);
                Err("queue full".to_string())
            },
        );

        latch_terminal_live_audio_fault_with(
            &status,
            &safety,
            "late stream error".to_string(),
            || {
                late_clear_attempts.set(late_clear_attempts.get() + 1);
                Err("queue full".to_string())
            },
        );

        assert_eq!(late_clear_attempts.get(), 0);
        assert!(!safety.clear_pending());
        let current = status.lock().unwrap();
        assert!(!current.safety_clear_pending);
        assert_eq!(
            current.last_error.as_deref(),
            Some("Live audio input Stop requested; waiting for the engine safety clear")
        );
    }

    #[test]
    fn live_audio_drop_clear_retry_is_bounded_and_can_recover_queue_pressure() {
        let attempts = Cell::new(0_u32);
        assert!(send_live_audio_clear_bounded_with(
            Duration::from_millis(10),
            Duration::ZERO,
            || {
                attempts.set(attempts.get() + 1);
                (attempts.get() >= 3)
                    .then_some(())
                    .ok_or_else(|| "queue full".to_string())
            },
        ));
        assert_eq!(attempts.get(), 3);

        let attempts = Cell::new(0_u32);
        assert!(!send_live_audio_clear_bounded_with(
            Duration::ZERO,
            Duration::ZERO,
            || {
                attempts.set(attempts.get() + 1);
                Err("queue full".to_string())
            },
        ));
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn final_drop_clear_publishes_the_generation_recovered_after_stop_observation() {
        let engine = EngineHandle::start_for_tests(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let initial = allocate_live_audio_generation().unwrap();
        let generation = AtomicU64::new(initial);
        let stop_observed_generation = generation.load(Ordering::Acquire);
        engine
            .clear_live_audio_input(stop_observed_generation)
            .unwrap();

        let recovered = rotate_live_audio_generation(&generation).unwrap();
        engine
            .send(EngineCommand::PublishLiveAudioFrame {
                frame: protocol::LiveAudioFrame {
                    generation: recovered,
                    feature_sequence: 1,
                    spectrum: protocol::AudioSpectrumPoint {
                        time_ms: 0,
                        bass: 0.25,
                        mid: 0.5,
                        high: 0.75,
                    },
                    features: protocol::LiveAudioReactiveFeatures::default(),
                    onset_feature_sequences: [0; protocol::MAX_LIVE_AUDIO_FRAME_ONSETS],
                    onset_count: 0,
                },
                captured_at: Instant::now(),
            })
            .unwrap();

        let published_clear_generation = Cell::new(None);
        assert!(send_current_live_audio_clear_bounded_with(
            &generation,
            LIVE_AUDIO_STOP_CLEAR_RETRY,
            LIVE_AUDIO_STOP_CLEAR_RETRY_INTERVAL,
            |generation| {
                published_clear_generation.set(Some(generation));
                engine.clear_live_audio_input(generation)
            },
        ));

        assert_eq!(published_clear_generation.get(), Some(recovered));
        assert_eq!(
            engine.snapshot().video.auto_vj.status.live_audio_generation,
            Some(recovered)
        );
    }

    #[test]
    fn live_audio_publish_failure_fails_closed_and_latches_the_fault() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        let clear_attempts = Cell::new(0_u32);
        let spectrum = protocol::AudioSpectrumPoint {
            time_ms: 0,
            bass: 0.2,
            mid: 0.4,
            high: 0.6,
        };

        assert!(!publish_live_audio_spectrum_with(
            &status,
            &safety,
            &spectrum,
            || Err("queue full".to_string()),
            || {
                clear_attempts.set(clear_attempts.get() + 1);
                Err("queue full".to_string())
            },
        ));

        let current = status.lock().unwrap().clone();
        assert!(current.running);
        assert!(current.stale);
        assert!(current.safety_clear_pending);
        assert_eq!((current.bass, current.mid, current.high), (0.0, 0.0, 0.0));
        assert_eq!(
            current.last_error.as_deref(),
            Some("Live audio spectrum publish failed: queue full")
        );
        assert!(safety.terminal_faulted());
        assert!(safety.clear_pending());
        assert_eq!(clear_attempts.get(), 1);
    }

    #[test]
    fn live_audio_poisoned_status_claims_terminal_clear_once() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _status = status.lock().unwrap();
            panic!("poison the live audio status lock");
        }));
        let safety = LiveAudioInputSafety::default();
        let spectrum_sends = Cell::new(0_u32);
        let clear_sends = Cell::new(0_u32);
        let spectrum = protocol::AudioSpectrumPoint {
            time_ms: 0,
            bass: 0.2,
            mid: 0.4,
            high: 0.6,
        };

        for _ in 0..2 {
            assert!(!publish_live_audio_spectrum_with(
                &status,
                &safety,
                &spectrum,
                || {
                    spectrum_sends.set(spectrum_sends.get() + 1);
                    Ok(())
                },
                || {
                    clear_sends.set(clear_sends.get() + 1);
                    Ok(())
                },
            ));
        }

        assert_eq!(spectrum_sends.get(), 0);
        assert_eq!(clear_sends.get(), 1);
        assert!(safety.terminal_faulted());
        assert!(!safety.clear_pending());
    }

    #[test]
    fn live_audio_terminal_fault_prevents_a_late_spectrum_publish() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        fail_closed_live_audio_with(&status, &safety, true, "stream failed".to_string(), || {
            Ok(())
        });
        let spectrum_sends = Cell::new(0_u32);
        let clear_sends = Cell::new(0_u32);

        assert!(!publish_live_audio_spectrum_with(
            &status,
            &safety,
            &protocol::AudioSpectrumPoint {
                time_ms: 0,
                bass: 1.0,
                mid: 1.0,
                high: 1.0,
            },
            || {
                spectrum_sends.set(spectrum_sends.get() + 1);
                Ok(())
            },
            || {
                clear_sends.set(clear_sends.get() + 1);
                Ok(())
            },
        ));
        assert_eq!(spectrum_sends.get(), 0);
        assert_eq!(clear_sends.get(), 0);
        let current = status.lock().unwrap();
        assert!(!current.safety_clear_pending);
        assert_eq!(current.analyzed_windows, 0);
    }

    #[test]
    fn live_audio_watchdog_stale_recovers_only_from_a_fresh_chunk() {
        let status = Mutex::new(LiveAudioInputStatus {
            running: true,
            bass: 0.2,
            mid: 0.4,
            high: 0.6,
            ..LiveAudioInputStatus::default()
        });
        let safety = LiveAudioInputSafety::default();
        fail_closed_live_audio_with(
            &status,
            &safety,
            false,
            LIVE_AUDIO_STALE_ERROR.to_string(),
            || Err("queue full".to_string()),
        );
        let current = status.lock().unwrap().clone();
        assert!(current.stale);
        assert!(current.safety_clear_pending);
        assert!(safety.clear_pending());

        assert!(!recover_live_audio_input_status(&status, &safety));
        assert!(retry_live_audio_clear_with(&status, &safety, || Ok(())));
        assert!(!status.lock().unwrap().safety_clear_pending);
        assert!(recover_live_audio_input_status(&status, &safety));
        let current = status.lock().unwrap().clone();
        assert!(current.running);
        assert!(!current.stale);
        assert!(current.last_error.is_none());
        assert!(!safety.terminal_faulted());
        assert!(!safety.clear_pending());

        fail_closed_live_audio_with(
            &status,
            &safety,
            false,
            LIVE_AUDIO_WORKER_STALE_ERROR.to_string(),
            || Ok(()),
        );
        assert!(recover_live_audio_input_status(&status, &safety));
        assert!(status.lock().unwrap().last_error.is_none());
    }

    #[cfg(test)]
    use tests::{
        assert_media_asset_a6_same_terminal, sample_operator_policy, MediaAssetA6CommandHarness,
        MediaAssetA6CommandMutationBaseline, MEDIA_ASSET_A6_OWNER,
    };

    /// Seed an engine-ready B1/T2 layer through the same single EngineHandle
    /// media transaction that production uses. B3 command tests deliberately
    /// avoid fabricated snapshots so their persistence/hash/history assertions
    /// cover the actual backend authority bridge.
    fn seed_video_clip_slot_layer(
        harness: &MediaAssetA6CommandHarness,
    ) -> (VideoLayerId, MediaAssetId, MediaAssetId, VideoClipSlotId) {
        let path = harness.local_media_path("b3-seed", VideoSourceKind::File);
        let cancel = AtomicBool::new(false);
        let (prepared, _) =
            prepare_local_media_asset_batch(VideoSourceKind::File, vec![path], &cancel)
                .expect("prepare B3 seed media");
        let prepared = prepared.into_iter().next().expect("B3 seed media exists");
        let layer_id = harness.state.engine.allocate_video_layer_id();
        let asset_id = harness.state.engine.allocate_media_asset_id();
        let alternate_asset_id = harness.state.engine.allocate_media_asset_id();
        let slot_id = harness.state.engine.allocate_video_clip_slot_id();
        let source = prepared.source.clone();
        harness
            .state
            .engine
            .media_asset_transaction_published(MediaAssetTransaction::Import(
                MediaAssetImportCandidate {
                    assets: vec![
                        MediaAssetSummary {
                            id: asset_id,
                            label: "B3 seed media".to_string(),
                            source: source.clone(),
                            content_hash: Some(prepared.content_hash.clone()),
                            byte_size: Some(prepared.byte_size),
                        },
                        MediaAssetSummary {
                            id: alternate_asset_id,
                            label: "B3 alternate media".to_string(),
                            source: source.clone(),
                            content_hash: Some(prepared.content_hash),
                            byte_size: Some(prepared.byte_size),
                        },
                    ],
                    layers: vec![protocol::VideoLayerSummary {
                        id: layer_id,
                        label: "B3 seed layer".to_string(),
                        source,
                        media_asset_id: Some(asset_id),
                        blend_mode: VideoBlendMode::Normal,
                        state: VideoLayerState::default(),
                        isf_effect: None,
                        clip_slots: vec![VideoClipSlotSummary {
                            id: slot_id,
                            media_asset_id: asset_id,
                            in_point_ms: 0,
                            out_point_ms: None,
                            loop_mode: Default::default(),
                            // A stopped active source gives the direct-import
                            // regression a deterministic playhead: the test
                            // can prove import-and-assign preserves runtime
                            // truth without racing the normal frame clock.
                            speed: 0.0,
                            cue_points: Vec::new(),
                            launch_quantization: Default::default(),
                            effect_overrides: Vec::new(),
                        }],
                        default_clip_slot_id: Some(slot_id),
                    }],
                },
            ))
            .expect("seed B3 engine-ready layer");
        *harness
            .state
            .project_coordinator
            .lock()
            .expect("reset B3 coordinator after seed") =
            project_coordinator_for_initial_snapshot(harness.state.engine.snapshot());
        (layer_id, asset_id, alternate_asset_id, slot_id)
    }

    /// Seed one real local media asset with both streams and the three explicit
    /// unified-Timeline lane kinds used by InsertMedia tests.  The source file
    /// is still prepared through the existing EngineHandle fixture; only its
    /// probe metadata is amended so the command path exercises the linked AV
    /// branch deterministically without depending on a machine codec.
    fn seed_timeline_insert_media_fixture(
        harness: &MediaAssetA6CommandHarness,
    ) -> (MediaAssetId, u32, u32, u32) {
        let (_video_layer_id, media_asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(harness);
        let snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read InsertMedia media seed");
        let mut asset = snapshot
            .video
            .media_assets
            .iter()
            .find(|asset| asset.id == media_asset_id)
            .cloned()
            .expect("InsertMedia media asset exists");
        asset.source.metadata = Some(protocol::VideoMediaMetadata {
            duration_ms: Some(1_000),
            width: Some(16),
            height: Some(16),
            frame_rate: Some(30.0),
            has_audio: true,
        });
        harness
            .state
            .engine
            .media_asset_transaction_published(MediaAssetTransaction::Update(asset))
            .expect("publish deterministic linked AV metadata");

        let video_lane_id = harness.state.engine.allocate_timeline_layer_id();
        let audio_lane_id = harness.state.engine.allocate_timeline_layer_id();
        let lighting_lane_id = harness.state.engine.allocate_timeline_layer_id();
        for (id, label, order, kind) in [
            (video_lane_id, "Insert Video", 1, TimelineLayerKind::Video),
            (audio_lane_id, "Insert Audio", 2, TimelineLayerKind::Audio),
            (
                lighting_lane_id,
                "Insert Lighting",
                3,
                TimelineLayerKind::Lighting,
            ),
        ] {
            harness
                .state
                .engine
                .add_timeline_layer(protocol::TimelineLayerSummary {
                    id,
                    label: label.to_string(),
                    order,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind,
                })
                .expect("seed InsertMedia Timeline lane");
        }
        c1_stabilize_fixture_authority(harness);
        (
            media_asset_id,
            video_lane_id,
            audio_lane_id,
            lighting_lane_id,
        )
    }

    fn seed_timeline_insert_media_stream_fixture(
        harness: &MediaAssetA6CommandHarness,
        has_video: bool,
        has_audio: bool,
    ) -> (MediaAssetId, u32, u32, u32) {
        let ids = seed_timeline_insert_media_fixture(harness);
        let media_asset_id = ids.0;
        let snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read stream-specific InsertMedia seed");
        let mut asset = snapshot
            .video
            .media_assets
            .iter()
            .find(|asset| asset.id == media_asset_id)
            .cloned()
            .expect("stream-specific InsertMedia media asset exists");
        asset.source.metadata = Some(protocol::VideoMediaMetadata {
            duration_ms: Some(1_000),
            width: has_video.then_some(16),
            height: has_video.then_some(16),
            frame_rate: has_video.then_some(30.0),
            has_audio,
        });
        harness
            .state
            .engine
            .media_asset_transaction_published(MediaAssetTransaction::Update(asset))
            .expect("publish stream-specific InsertMedia metadata");
        c1_stabilize_fixture_authority(harness);
        ids
    }

    fn b3_authority_arguments(harness: &MediaAssetA6CommandHarness) -> (u64, u64, String) {
        let authority = harness.authority();
        (
            authority.epoch,
            authority.revision,
            authority.checkpoint_hash,
        )
    }

    fn b3_assert_authored_mutation_count(
        harness: &MediaAssetA6CommandHarness,
        baseline: MediaAssetA6CommandMutationBaseline,
        expected: u64,
    ) {
        assert_eq!(
            harness
                .state
                .video_clip_slot_authoritative_publish_attempts
                .load(Ordering::Acquire),
            expected,
            "only first delivery of each authored request publishes"
        );
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("B3 coordinator after authored commands");
        assert_eq!(coordinator.revision, baseline.revision + expected);
        assert_eq!(
            coordinator.history_generation,
            baseline.history_generation + expected
        );
        assert_eq!(
            coordinator.history.undo.len(),
            baseline.undo_len + expected as usize
        );
        assert_eq!(
            coordinator.publication_generation,
            baseline.publication_generation + expected
        );
        assert!(coordinator.history.pending.is_empty());
    }

    fn b3_persistence_hash(harness: &MediaAssetA6CommandHarness) -> String {
        let snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("B3 persistence snapshot");
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("B3 persistence coordinator");
        project_checkpoint_hash(
            &project_file_for_save_from_parts(snapshot, &coordinator.ancillary),
            &coordinator.mappings,
        )
        .expect("hash B3 persisted project")
    }

    fn b3_assert_authority_matches_persistence(harness: &MediaAssetA6CommandHarness) {
        let persistence_hash = b3_persistence_hash(harness);
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("B3 authoritative coordinator");
        assert_eq!(
            coordinator.checkpoint_hash, persistence_hash,
            "the backend candidate B hash must exactly equal engine persistence before the next command"
        );
    }

    /// Small, real ISF source with an Event input. C1 must accept its pulse at
    /// the IPC boundary but persist the catalog and its legacy projection with
    /// the value reset to zero.
    const C1_TEST_ISF_EVENT_SOURCE: &str = r#"/*{
      "DESCRIPTION": "C1 event seam",
      "INPUTS": [
        {"NAME":"inputImage","TYPE":"image"},
        {"NAME":"pulse","TYPE":"event"}
      ]
    }*/
    void main() {
      gl_FragColor = IMG_THIS_PIXEL(inputImage);
    }"#;

    fn c1_test_event_effect(value: f32) -> VideoIsfEffectSummary {
        let prepared = video::prepare_isf_shader(C1_TEST_ISF_EVENT_SOURCE)
            .expect("prepare C1 Event ISF effect");
        let mut effect = video::isf_effect_from_prepared(
            "C1 event seam".to_string(),
            C1_TEST_ISF_EVENT_SOURCE.to_string(),
            Some("filters/c1-event-seam.fs".to_string()),
            &prepared,
        );
        let pulse = effect
            .controls
            .iter_mut()
            .find(|control| control.kind == VideoIsfControlKind::Event)
            .expect("prepared C1 ISF has Event control");
        pulse.value = [value, 0.0, 0.0, 0.0];
        effect
    }

    fn c1_catalog_chain(
        _harness: &MediaAssetA6CommandHarness,
        scope: VideoEffectScope,
        label: &str,
        effect: VideoIsfEffectSummary,
    ) -> VideoEffectChainSummary {
        // C1 creation uses the wire-level zero sentinel. The terminal lane is
        // responsible for replacing it with EngineHandle allocator values and
        // retaining the allocated result for an exact retry.
        VideoEffectChainSummary {
            id: protocol::VideoEffectChainId(0),
            scope,
            bypassed: false,
            stages: vec![protocol::VideoEffectStageSummary {
                id: protocol::VideoEffectStageId(0),
                enabled: true,
                label: label.to_string(),
                effect: protocol::VideoEffectSummary {
                    id: protocol::VideoEffectId(0),
                    kind: VideoEffectKind::Isf { effect },
                },
            }],
        }
    }

    fn c1_layer_catalog_request(
        harness: &MediaAssetA6CommandHarness,
        layer_id: VideoLayerId,
        event_value: f32,
    ) -> VideoEffectCatalogApplyRequest {
        VideoEffectCatalogApplyRequest {
            effect_chains: vec![c1_catalog_chain(
                harness,
                VideoEffectScope::Layer { layer_id },
                "C1 layer stage",
                c1_test_event_effect(event_value),
            )],
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
        }
    }

    /// The shared B3 fixture seeds through an intentionally direct Engine
    /// transaction, then resets its coordinator from the render snapshot.
    /// C1 tests start from the persistence authority that a real caller would
    /// capture after that setup has settled, rather than treating the fixture
    /// setup itself as an untracked catalog mutation.
    fn c1_stabilize_fixture_authority(harness: &MediaAssetA6CommandHarness) {
        let mut coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("stabilize C1 fixture coordinator");
        reconcile_project_checkpoint_for_coordinator(&harness.state, &mut coordinator)
            .expect("reconcile C1 fixture persistence authority");
    }

    fn c1_assert_one_authored_mutation(
        harness: &MediaAssetA6CommandHarness,
        baseline: MediaAssetA6CommandMutationBaseline,
    ) {
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            1,
            "only the original C1 request reaches EngineHandle::apply_video_effect_catalog"
        );
        assert_one_authoritative_history_mutation(harness, baseline);
    }

    fn assert_one_authoritative_history_mutation(
        harness: &MediaAssetA6CommandHarness,
        baseline: MediaAssetA6CommandMutationBaseline,
    ) {
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("C1 coordinator after catalog commit");
        assert_eq!(
            coordinator.revision,
            baseline.revision + 1,
            "C1 revision mismatch: history_generation={}, undo={}, publication_generation={}, next_transaction_id={}",
            coordinator.history_generation,
            coordinator.history.undo.len(),
            coordinator.publication_generation,
            coordinator.next_transaction_id,
        );
        assert_eq!(
            coordinator.history_generation,
            baseline.history_generation + 1
        );
        assert_eq!(coordinator.history.undo.len(), baseline.undo_len + 1);
        assert_eq!(
            coordinator.publication_generation,
            baseline.publication_generation + 1
        );
        assert!(coordinator.history.pending.is_empty());
    }

    #[test]
    fn timeline_follow_runtime_abort_terminal_is_exact_and_history_free() {
        let harness = MediaAssetA6CommandHarness::new();
        let mut full_policy = sample_operator_policy();
        full_policy.lock_mode = OperatorLockMode::Full;
        harness
            .state
            .project_coordinator
            .lock()
            .expect("install Full Lock policy for Follow abort")
            .ancillary
            .operator_policy = Some(full_policy);
        c1_stabilize_fixture_authority(&harness);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let observed = get_timeline_follow_runtime_impl(
            &harness.state,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("read Follow runtime through the project E/R/H fence");
        let request = TimelineFollowAbortRequest {
            expected_follow_epoch: observed.runtime.epoch,
            expected_generation: observed.runtime.generation,
        };
        let baseline = harness.mutation_baseline();
        let applied = abort_timeline_follow_authoritative_command_impl(
            &harness.state,
            request.clone(),
            96_401,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("Full Lock must still permit runtime Follow abort safety");
        let retried = abort_timeline_follow_authoritative_command_impl(
            &harness.state,
            request.clone(),
            96_401,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss retry returns the exact terminal Follow abort receipt");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "exact retry must not enqueue a second Follow abort"
        );
        let after = harness.mutation_baseline();
        assert_eq!(
            (
                after.revision,
                after.history_generation,
                after.undo_len,
                after.next_transaction_id,
                after.publication_generation,
            ),
            (
                baseline.revision,
                baseline.history_generation,
                baseline.undo_len,
                baseline.next_transaction_id,
                baseline.publication_generation,
            ),
            "Follow abort must never change authored revision/history/dirty state"
        );
        let receipt = get_timeline_follow_operation_terminal_result_impl(
            &harness.state,
            request.clone(),
            96_401,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("query Follow terminal receipt")
        .expect("completed Follow terminal receipt exists");
        assert_eq!(
            serde_json::to_value(&receipt.terminal).unwrap(),
            serde_json::to_value(&applied).unwrap(),
            "terminal query returns the definitive runtime outcome"
        );
        let shape_conflict = TimelineFollowAbortRequest {
            expected_follow_epoch: request.expected_follow_epoch,
            expected_generation: request.expected_generation.saturating_add(1),
        };
        assert!(abort_timeline_follow_authoritative_command_impl(
            &harness.state,
            shape_conflict,
            96_401,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("same request ID with another Follow ABA token must reject")
        .contains("already completed with shape"));

        // Re-registering the same owner string under a new backend-issued
        // WebView incarnation must not let a terminal receipt cross the ABA
        // boundary. This mirrors the real rotation path's C1 purge.
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("retire Follow owner")
            .remove("media-asset-a6");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("retire Follow owner incarnation")
            .remove("media-asset-a6");
        harness
            .state
            .media_asset_operations
            .purge_video_effect_catalog_authoritative_for_owner(MEDIA_ASSET_A6_OWNER);
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("re-register Follow owner")
            .insert(
                "media-asset-a6".to_string(),
                MEDIA_ASSET_A6_OWNER.to_string(),
            );
        let replacement_incarnation =
            allocate_project_transaction_owner_incarnation(&harness.state)
                .expect("allocate Follow replacement incarnation");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("register Follow replacement incarnation")
            .insert("media-asset-a6".to_string(), replacement_incarnation);
        assert!(get_timeline_follow_operation_terminal_result_impl(
            &harness.state,
            request,
            96_401,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("new Follow owner incarnation can query its own namespace")
        .is_none());
    }

    #[test]
    fn canonical_timeline_follow_abort_full_lock_retry_is_one_publish_and_history_free() {
        let harness = MediaAssetA6CommandHarness::new();
        harness
            .state
            .engine
            .set_output_ownership_role(MachineOutputRole::Standby)
            .expect("establish a non-zero output ownership epoch");
        let audio_layer = protocol::TimelineLayerSummary {
            id: 96_512,
            label: "Canonical Follow audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: false,
            kind: TimelineLayerKind::Audio,
        };
        let mut source = TimelineSnapshot {
            id: TimelineId(96_510),
            label: "Canonical Follow abort source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![TimelineAudioClipSummary {
                id: 96_513,
                layer_id: audio_layer.id,
                media_asset_id: None,
                path: "memory://canonical-follow-abort-source.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            }],
            duration_ms: 100,
            ..TimelineSnapshot::default()
        };
        source.follow = Some(TimelineFollowSummary {
            enabled: true,
            next_timeline_id: TimelineId(96_511),
            duration: VideoClipTakeDuration::milliseconds(60_000),
            curve: VideoLayerTransitionCurve::Linear,
            video_kind: VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = TimelineSnapshot {
            id: TimelineId(96_511),
            label: "Canonical Follow abort target".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![TimelineAudioClipSummary {
                id: 96_514,
                layer_id: audio_layer.id,
                media_asset_id: None,
                path: "memory://canonical-follow-abort-target.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            }],
            duration_ms: 100,
            ..TimelineSnapshot::default()
        };
        harness
            .state
            .engine
            .apply_timeline_bank_published(vec![source, target], TimelineId(96_510), false)
            .expect("install a real Follow bank");
        harness
            .state
            .engine
            .send(EngineCommand::SeekTimeline(90))
            .expect("seek source near its Follow boundary");
        harness
            .state
            .engine
            .send(EngineCommand::SetTimelinePlaying(true))
            .expect("start the source Timeline");
        let output_ownership_epoch = harness.state.engine.output_ownership_status().epoch;
        let follow_deadline = Instant::now() + Duration::from_secs(2);
        let before_runtime = loop {
            let runtime = harness
                .state
                .engine
                .timeline_follow_runtime_status(output_ownership_epoch);
            if matches!(
                runtime.status,
                protocol::TimelineFollowRuntimeStatus::Transitioning
            ) {
                break runtime;
            }
            assert!(
                Instant::now() < follow_deadline,
                "canonical fixture never entered a real Follow transition: {runtime:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        let mut full_policy = sample_operator_policy();
        full_policy.lock_mode = OperatorLockMode::Full;
        harness
            .state
            .project_coordinator
            .lock()
            .expect("install Full Lock policy for canonical Follow abort")
            .ancillary
            .operator_policy = Some(full_policy);
        c1_stabilize_fixture_authority(&harness);

        let project = {
            let coordinator = harness
                .state
                .project_coordinator
                .lock()
                .expect("capture canonical Follow project fence");
            ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 1,
                project_epoch: coordinator.epoch,
                project_revision: coordinator.revision,
                project_checkpoint_hash: coordinator.checkpoint_hash.clone(),
                project_publication_generation: coordinator.publication_generation,
            }
        };
        let fence = protocol::control_plane_command::TimelineFollowAbortRuntimeFenceV1 {
            project,
            domain: protocol::control_plane_command::TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1
                .to_string(),
            output_ownership_epoch,
            follow_generation: before_runtime.generation,
        };
        let authority_id = "AAAAAAAAAAAAAAAAAAAAAA".to_string();
        control_plane_runtime::issue_timeline_follow_abort_authority_for_test_window(
            &harness.state,
            "media-asset-a6",
            fence.clone(),
            authority_id.clone(),
            Instant::now(),
        )
        .expect("issue owner-bound canonical Follow abort authority");
        let request = TimelineFollowAbortRuntimeRequestV1 {
            operation_id: protocol::control_plane_command::TIMELINE_FOLLOW_ABORT_OPERATION_ID
                .to_string(),
            authority_id,
            request_id: 96_501,
            expected_fence: fence,
        };
        let baseline = harness.mutation_baseline();
        let applied = control_plane_runtime::abort_timeline_follow_runtime_for_test_window(
            &harness.state,
            "media-asset-a6",
            request.clone(),
            Instant::now(),
        );
        let retried = control_plane_runtime::abort_timeline_follow_runtime_for_test_window(
            &harness.state,
            "media-asset-a6",
            request,
            Instant::now(),
        );
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "reply-loss retry returns the exact retained canonical receipt"
        );
        let TimelineFollowAbortRuntimeResponseV1::Receipt(receipt) = applied else {
            panic!("canonical Follow abort must return an applied receipt");
        };
        assert_eq!(
            receipt.outcome,
            protocol::control_plane_command::RuntimeCommandReceiptOutcomeV1::Applied
        );
        assert_eq!(
            receipt.follow_generation_after,
            receipt.fence_before.follow_generation + 1,
            "the real EngineHandle mutation advances the Follow ABA fence exactly once"
        );
        let after = harness.mutation_baseline();
        assert_eq!(
            (
                after.revision,
                after.history_generation,
                after.undo_len,
                after.next_transaction_id,
                after.publication_generation,
            ),
            (
                baseline.revision,
                baseline.history_generation,
                baseline.undo_len,
                baseline.next_transaction_id,
                baseline.publication_generation,
            ),
            "canonical Follow abort stays runtime-only under Full Lock"
        );
    }

    #[test]
    fn canonical_safety_blackout_full_lock_retry_is_latched_and_history_free() {
        let harness = MediaAssetA6CommandHarness::new();
        let mut full_policy = sample_operator_policy();
        full_policy.lock_mode = OperatorLockMode::Full;
        harness
            .state
            .project_coordinator
            .lock()
            .expect("install Full Lock for S0 blackout")
            .ancillary
            .operator_policy = Some(full_policy);

        let baseline = harness.mutation_baseline();
        let persistence_a = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("capture pre-S0 persistence A");
        let request = SafetyBlackoutEngageRequestV1 {
            operation_id: protocol::control_plane_command::SAFETY_BLACKOUT_ENGAGE_OPERATION_ID
                .to_string(),
            request_id: 96_601,
        };
        let applied = control_plane_runtime::engage_safety_blackout_for_test_window(
            &harness.state,
            "media-asset-a6",
            request.clone(),
            Instant::now(),
        );
        let retried = control_plane_runtime::engage_safety_blackout_for_test_window(
            &harness.state,
            "media-asset-a6",
            request,
            Instant::now(),
        );
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "lost IPC reply recovers the exact retained S0 receipt"
        );
        let SafetyBlackoutEngageResponseV1::Receipt(first_receipt) = applied else {
            panic!("first S0 request must return a receipt")
        };
        assert_eq!(
            first_receipt.outcome,
            protocol::control_plane_command::SafetyBlackoutEngageOutcomeV1::Applied
        );
        assert!(harness.state.engine.snapshot().blackout);
        assert_eq!(
            project_snapshot_for_save(harness.state.engine.persistence_snapshot().unwrap()),
            project_snapshot_for_save(persistence_a.clone()),
            "the runtime safety latch never enters authored persistence"
        );
        let after_s0 = harness.mutation_baseline();
        assert_eq!(
            (
                after_s0.revision,
                after_s0.history_generation,
                after_s0.undo_len,
                after_s0.next_transaction_id,
                after_s0.publication_generation,
            ),
            (
                baseline.revision,
                baseline.history_generation,
                baseline.undo_len,
                baseline.next_transaction_id,
                baseline.publication_generation,
            ),
            "S0 engage stays revision/history/publication neutral"
        );

        harness
            .state
            .engine
            .send(EngineCommand::Blackout(true))
            .expect("set the legacy authored blackout true");
        let wait_deadline = Instant::now() + Duration::from_secs(2);
        while !harness
            .state
            .engine
            .persistence_snapshot()
            .expect("observe authored blackout true")
            .blackout
        {
            assert!(
                Instant::now() < wait_deadline,
                "legacy blackout true did not apply"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        harness
            .state
            .engine
            .send(EngineCommand::Blackout(false))
            .expect("attempt a legacy blackout release");
        let wait_deadline = Instant::now() + Duration::from_secs(2);
        while harness
            .state
            .engine
            .persistence_snapshot()
            .expect("observe authored blackout false")
            .blackout
        {
            assert!(
                Instant::now() < wait_deadline,
                "legacy blackout false did not apply"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            harness.state.engine.snapshot().blackout,
            "ordinary authored release cannot clear the S0 runtime latch"
        );
        harness
            .state
            .engine
            .load_project_snapshot_and_wait(persistence_a)
            .expect("replace the authored project beneath the S0 latch");
        assert!(
            harness.state.engine.snapshot().blackout,
            "project replacement cannot clear the S0 runtime latch"
        );

        let second = control_plane_runtime::engage_safety_blackout_for_test_window(
            &harness.state,
            "media-asset-a6",
            SafetyBlackoutEngageRequestV1 {
                operation_id: protocol::control_plane_command::SAFETY_BLACKOUT_ENGAGE_OPERATION_ID
                    .to_string(),
                request_id: 96_602,
            },
            Instant::now(),
        );
        let SafetyBlackoutEngageResponseV1::Receipt(second_receipt) = second else {
            panic!("second S0 request must return a receipt")
        };
        assert_eq!(
            second_receipt.outcome,
            protocol::control_plane_command::SafetyBlackoutEngageOutcomeV1::NoOp
        );
        assert_eq!(
            second_receipt.audit_sequence,
            first_receipt.audit_sequence + 1
        );
    }

    #[test]
    fn video_effect_catalog_authoritative_success_reply_loss_and_event_persistence_are_atomic() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        let request = c1_layer_catalog_request(&harness, layer_id, 1.0);
        let request_shape = video_effect_catalog_authoritative_shape(&request)
            .expect("encode original C1 request shape");
        let before_snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("capture C1 candidate baseline");
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture C1 caller incarnation");
        let applied = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request.clone(),
            84_001,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
        )
        .expect("apply C1 catalog through authoritative core");
        let retried = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request,
            84_001,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
        )
        .expect("reply-loss retry returns the recorded terminal result");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "a retry must recover exactly the first terminal DTO"
        );
        assert!(
            applied.catalog.effect_chains.iter().all(|chain| {
                chain.id.0 != 0
                    && chain
                        .stages
                        .iter()
                        .all(|stage| stage.id.0 != 0 && stage.effect.id.0 != 0)
            }),
            "the receipt, rather than the caller, owns every allocated C1 ID"
        );
        let expected_candidate = video_effect_catalog_candidate_snapshot(
            before_snapshot,
            &VideoEffectCatalogApplyRequest {
                effect_chains: applied.catalog.effect_chains.clone(),
                effect_presets: applied.catalog.effect_presets.clone(),
                layer_groups: applied.catalog.layer_groups.clone(),
                transition_buses: applied.catalog.transition_buses.clone(),
            },
        )
        .expect("rebuild the normalized C1 candidate B");
        c1_assert_one_authored_mutation(&harness, baseline);
        let actual_persistence = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("capture engine C1 persistence B");
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("C1 coordinator for candidate hash");
        let candidate_project =
            project_file_for_save_from_parts(expected_candidate.clone(), &coordinator.ancillary);
        let actual_project =
            project_file_for_save_from_parts(actual_persistence.clone(), &coordinator.ancillary);
        assert_eq!(
            candidate_project, actual_project,
            "candidate B and engine persistence must have the same durable project image"
        );
        let actual_persistence_hash =
            project_checkpoint_hash(&actual_project, &coordinator.mappings)
                .expect("hash engine C1 persistence B");
        assert_eq!(
            applied.mutation.authority.checkpoint_hash, actual_persistence_hash,
            "C1 committed authority must name the catalog candidate B"
        );
        drop(coordinator);

        let persisted = actual_persistence
            .authored_video
            .expect("C1 authored video snapshot");
        assert_eq!(
            expected_candidate
                .authored_video
                .expect("C1 candidate retains the canonical authored video"),
            persisted,
            "engine persistence must equal the fully projected candidate video B"
        );
        assert_eq!(persisted.effect_chains, applied.catalog.effect_chains);
        assert_eq!(persisted.effect_presets, applied.catalog.effect_presets);
        assert_eq!(persisted.layer_groups, applied.catalog.layer_groups);
        let event_value = match &persisted.effect_chains[0].stages[0].effect.kind {
            VideoEffectKind::Isf { effect } => {
                effect
                    .controls
                    .iter()
                    .find(|control| control.kind == VideoIsfControlKind::Event)
                    .expect("persisted C1 Event control")
                    .value[0]
            }
        };
        assert_eq!(event_value, 0.0, "Event state is never persisted");
        let legacy_event_value = persisted
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .expect("persisted C1 layer")
            .isf_effect
            .as_ref()
            .expect("canonical Layer chain reprojects legacy ISF")
            .controls
            .iter()
            .find(|control| control.kind == VideoIsfControlKind::Event)
            .expect("legacy projection C1 Event control")
            .value[0];
        assert_eq!(legacy_event_value, 0.0);
        let recovered = get_video_effect_catalog_operation_terminal_result_impl(
            &harness.state,
            84_001,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding),
        )
        .expect("read C1 terminal receipt")
        .expect("C1 terminal receipt exists");
        assert_eq!(recovered.shape_fingerprint, request_shape.fingerprint);
    }

    #[test]
    fn timeline_delete_items_expands_group_across_every_authored_domain_and_is_fail_closed() {
        let mut timeline = TimelineSnapshot {
            events: vec![
                protocol::TimelineCueEventSummary {
                    id: 20,
                    cue_id: 7,
                    time_ms: 0,
                    time_beats: None,
                    track: TimelineTrackKind::Lighting,
                    layer_id: None,
                    duration_ms: 1_000,
                    duration_beats: None,
                    conform_to_tempo: false,
                    loop_fill: false,
                    source_offset_ms: 0,
                    fade_in_ms: 0,
                    fade_out_ms: 0,
                    rate: None,
                    loop_count: 1,
                    jump_to_event_id: None,
                },
                protocol::TimelineCueEventSummary {
                    id: 21,
                    cue_id: 7,
                    time_ms: 1_000,
                    time_beats: None,
                    track: TimelineTrackKind::Lighting,
                    layer_id: None,
                    duration_ms: 1_000,
                    duration_beats: None,
                    conform_to_tempo: false,
                    loop_fill: false,
                    source_offset_ms: 0,
                    fade_in_ms: 0,
                    fade_out_ms: 0,
                    rate: None,
                    loop_count: 1,
                    jump_to_event_id: Some(20),
                },
            ],
            ..TimelineSnapshot::default()
        };
        timeline.audio_clips.push(TimelineAudioClipSummary {
            id: 31,
            layer_id: 3,
            media_asset_id: None,
            path: "linked.wav".to_string(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        });
        timeline.video_clips.push(TimelineVideoClipSummary {
            id: protocol::TimelineVideoClipId(32),
            layer_id: 2,
            media_asset_id: 9,
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 1_000,
            fade_in_ms: 0,
            fade_out_ms: 0,
        });
        timeline
            .automations
            .push(protocol::TimelineAutomationSummary {
                id: 33,
                fixture_id: 1,
                attribute: "Dimmer".to_string(),
                track: TimelineTrackKind::Lighting,
                timeline_layer_id: None,
                keyframes: vec![AutomationKeyframeSummary {
                    time_ms: 0,
                    value: 32_768,
                    interpolation: protocol::AutomationInterpolation::Linear,
                }],
                enabled: true,
            });
        timeline
            .video_automations
            .push(protocol::TimelineVideoAutomationSummary {
                id: 34,
                layer_id: 2,
                param: protocol::VideoParam::Opacity,
                track: TimelineTrackKind::Video,
                timeline_layer_id: None,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 0.5,
                    interpolation: protocol::AutomationInterpolation::Linear,
                }],
                enabled: true,
            });
        let removed_event_id = timeline.events[0].id;
        timeline.item_groups.push(TimelineItemGroupSummary {
            id: TimelineItemGroupId(35),
            members: vec![
                TimelineItemRef::LightingEvent {
                    event_id: removed_event_id,
                },
                TimelineItemRef::VideoClip {
                    clip_id: protocol::TimelineVideoClipId(32),
                },
                TimelineItemRef::AudioClip { clip_id: 31 },
                TimelineItemRef::LightingAutomation { automation_id: 33 },
                TimelineItemRef::VideoAutomation { automation_id: 34 },
            ],
        });

        let mut duplicated = timeline.clone();
        duplicated.events[0].jump_to_event_id = Some(removed_event_id);
        duplicated.events[0].time_beats = Some(0.0);
        duplicated.events[0].conform_to_tempo = true;
        let duplicate_harness = MediaAssetA6CommandHarness::new();
        let duplicate_refs = duplicate_timeline_items(
            &duplicate_harness.state,
            &mut duplicated,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            250,
            120.0,
        )
        .expect("duplicate one member as the complete cross-domain group");
        assert_eq!(duplicate_refs.len(), 5);
        assert_eq!(duplicated.events.len(), 3);
        assert_eq!(duplicated.video_clips.len(), 2);
        assert_eq!(duplicated.audio_clips.len(), 2);
        assert_eq!(duplicated.automations.len(), 2);
        assert_eq!(duplicated.video_automations.len(), 2);
        assert_eq!(duplicated.item_groups.len(), 2);
        let duplicate_event_id = duplicate_refs
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::LightingEvent { event_id } => Some(*event_id),
                _ => None,
            })
            .expect("duplicated Lighting event identity");
        let duplicate_event = duplicated
            .events
            .iter()
            .find(|event| event.id == duplicate_event_id)
            .expect("duplicated Lighting event");
        assert_eq!(duplicate_event.time_ms, 250);
        assert_eq!(duplicate_event.time_beats, Some(0.5));
        assert_eq!(
            duplicate_event.jump_to_event_id,
            Some(duplicate_event_id),
            "internal event jump references must follow fresh duplicate IDs"
        );
        let duplicate_group = duplicated
            .item_groups
            .iter()
            .find(|group| group.id != TimelineItemGroupId(35))
            .expect("duplicated group with a fresh stable ID");
        assert_eq!(
            duplicate_group
                .members
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            duplicate_refs.iter().copied().collect::<BTreeSet<_>>()
        );
        let mut overflow = timeline.clone();
        overflow.audio_clips[0].start_ms = u64::MAX;
        let overflow_before = overflow.clone();
        assert!(duplicate_timeline_items(
            &duplicate_harness.state,
            &mut overflow,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            1,
            120.0,
        )
        .unwrap_err()
        .contains("exceeds"));
        assert_eq!(
            overflow, overflow_before,
            "duplicate overflow must reject before mutating A"
        );
        let mut nudged = timeline.clone();
        nudged.events[0].time_beats = Some(0.0);
        nudged.events[0].conform_to_tempo = true;
        let before_early_nudge = nudged.clone();
        assert!(nudge_timeline_items(
            &mut nudged,
            &[TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(32),
            }],
            -1,
            120.0,
        )
        .unwrap_err()
        .contains("before time zero"));
        assert_eq!(nudged, before_early_nudge);
        let nudged_refs = nudge_timeline_items(
            &mut nudged,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            500,
            120.0,
        )
        .expect("nudge one member as the complete cross-domain group");
        assert_eq!(nudged_refs.len(), 5);
        assert_eq!(nudged.events[0].time_ms, 500);
        assert_eq!(nudged.events[0].time_beats, Some(1.0));
        assert_eq!(nudged.video_clips[0].start_ms, 500);
        assert_eq!(nudged.audio_clips[0].start_ms, 500);
        assert_eq!(nudged.automations[0].keyframes[0].time_ms, 500);
        assert_eq!(nudged.video_automations[0].keyframes[0].time_ms, 500);
        nudge_timeline_items(
            &mut nudged,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            -500,
            120.0,
        )
        .expect("nudge the complete group back without relative drift");
        assert_eq!(nudged.events[0].time_ms, 0);
        assert_eq!(nudged.events[0].time_beats, Some(0.0));
        assert_eq!(nudged.video_clips[0].start_ms, 0);
        assert_eq!(nudged.audio_clips[0].start_ms, 0);
        assert_eq!(nudged.automations[0].keyframes[0].time_ms, 0);
        assert_eq!(nudged.video_automations[0].keyframes[0].time_ms, 0);

        let mut rippled = timeline.clone();
        rippled.events[0].time_beats = Some(0.0);
        rippled.events[0].conform_to_tempo = true;
        let ripple_refs = ripple_timeline_items(
            &mut rippled,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            500,
            120.0,
        )
        .expect("ripple one linked member and every following Timeline item");
        assert_eq!(ripple_refs.len(), 5);
        assert_eq!(rippled.events[0].time_ms, 500);
        assert_eq!(rippled.events[0].time_beats, Some(1.0));
        assert_eq!(rippled.events[1].time_ms, 1_500);
        assert_eq!(rippled.video_clips[0].start_ms, 500);
        assert_eq!(rippled.audio_clips[0].start_ms, 500);
        assert_eq!(rippled.automations[0].keyframes[0].time_ms, 500);
        assert_eq!(rippled.video_automations[0].keyframes[0].time_ms, 500);
        let before_invalid_ripple = timeline.clone();
        let mut invalid_ripple = before_invalid_ripple.clone();
        assert!(ripple_timeline_items(
            &mut invalid_ripple,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            -1,
            120.0,
        )
        .unwrap_err()
        .contains("before time zero"));
        assert_eq!(invalid_ripple, before_invalid_ripple);

        let mut quantized = timeline.clone();
        quantized.events[0].time_ms = 750;
        quantized.events[0].time_beats = Some(1.5);
        quantized.events[0].conform_to_tempo = true;
        quantized.video_clips[0].start_ms = 1_000;
        quantized.audio_clips[0].start_ms = 1_500;
        quantized.automations[0].keyframes[0].time_ms = 2_000;
        quantized.video_automations[0].keyframes[0].time_ms = 2_500;
        let quantized_refs = quantize_timeline_items(
            &mut quantized,
            &[TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(32),
            }],
            1_000,
            120.0,
        )
        .expect("quantize one member as the complete cross-domain group");
        assert_eq!(quantized_refs.len(), 5);
        assert_eq!(quantized.events[0].time_ms, 1_000);
        assert_eq!(quantized.events[0].time_beats, Some(2.0));
        assert_eq!(quantized.video_clips[0].start_ms, 1_250);
        assert_eq!(quantized.audio_clips[0].start_ms, 1_750);
        assert_eq!(quantized.automations[0].keyframes[0].time_ms, 2_250);
        assert_eq!(quantized.video_automations[0].keyframes[0].time_ms, 2_750);
        let before_invalid_quantize = quantized.clone();
        assert!(quantize_timeline_items(
            &mut quantized,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            0,
            120.0,
        )
        .unwrap_err()
        .contains("greater than zero"));
        assert_eq!(quantized, before_invalid_quantize);

        let mut pasted_source = timeline.clone();
        pasted_source.events[0].time_ms = 1_000;
        pasted_source.events[0].time_beats = Some(2.0);
        pasted_source.events[0].conform_to_tempo = true;
        pasted_source.events[0].jump_to_event_id = Some(removed_event_id);
        pasted_source.video_clips[0].start_ms = 1_250;
        pasted_source.audio_clips[0].start_ms = 1_500;
        pasted_source.automations[0].keyframes[0].time_ms = 1_750;
        pasted_source.video_automations[0].keyframes[0].time_ms = 2_000;
        let paste_refs = paste_timeline_items(
            &duplicate_harness.state,
            &mut pasted_source,
            &[TimelineItemRef::AudioClip { clip_id: 31 }],
            250,
            120.0,
        )
        .expect("paste one member as a fresh complete group at an earlier target");
        assert_eq!(paste_refs.len(), 5);
        let pasted_event_id = paste_refs
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::LightingEvent { event_id } => Some(*event_id),
                _ => None,
            })
            .expect("pasted Lighting event identity");
        let pasted_event = pasted_source
            .events
            .iter()
            .find(|event| event.id == pasted_event_id)
            .expect("pasted Lighting event");
        assert_eq!(pasted_event.time_ms, 250);
        assert_eq!(pasted_event.time_beats, Some(0.5));
        assert_eq!(pasted_event.jump_to_event_id, Some(pasted_event_id));
        let pasted_video_id = paste_refs
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::VideoClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .unwrap();
        let pasted_audio_id = paste_refs
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::AudioClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            pasted_source
                .video_clips
                .iter()
                .find(|clip| clip.id == pasted_video_id)
                .unwrap()
                .start_ms,
            500
        );
        assert_eq!(
            pasted_source
                .audio_clips
                .iter()
                .find(|clip| clip.id == pasted_audio_id)
                .unwrap()
                .start_ms,
            750
        );
        assert_eq!(pasted_source.item_groups.len(), 2);
        assert_eq!(
            pasted_source.events[0].time_ms, 1_000,
            "copy source remains unchanged"
        );

        delete_timeline_items(&mut timeline, &[TimelineItemRef::AudioClip { clip_id: 31 }])
            .expect("deleting one member must close over the complete group");
        assert!(timeline
            .events
            .iter()
            .all(|event| event.id != removed_event_id));
        assert!(timeline.video_clips.is_empty());
        assert!(timeline.audio_clips.is_empty());
        assert!(timeline.automations.is_empty());
        assert!(timeline.video_automations.is_empty());
        assert!(timeline.item_groups.is_empty());
        assert_eq!(timeline.events[0].jump_to_event_id, None);

        let before_stale = timeline.clone();
        assert!(delete_timeline_items(
            &mut timeline,
            &[TimelineItemRef::AudioClip { clip_id: 9_999 }],
        )
        .unwrap_err()
        .contains("no longer exists"));
        assert_eq!(
            timeline, before_stale,
            "stale deletion must preserve A exactly"
        );
    }

    #[test]
    fn timeline_trim_items_is_atomic_across_every_authored_domain_and_supports_isolate() {
        let mut timeline = TimelineSnapshot {
            id: TimelineId(1),
            label: "Trim".to_string(),
            layers: vec![
                protocol::TimelineLayerSummary {
                    id: 1,
                    label: "Lighting".to_string(),
                    order: 0,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: TimelineLayerKind::Lighting,
                },
                protocol::TimelineLayerSummary {
                    id: 2,
                    label: "Video".to_string(),
                    order: 1,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: TimelineLayerKind::Video,
                },
                protocol::TimelineLayerSummary {
                    id: 3,
                    label: "Audio".to_string(),
                    order: 2,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: TimelineLayerKind::Audio,
                },
            ],
            ..TimelineSnapshot::default()
        };
        timeline.events.push(protocol::TimelineCueEventSummary {
            id: 20,
            cue_id: 7,
            time_ms: 1_000,
            time_beats: Some(2.0),
            track: TimelineTrackKind::Lighting,
            layer_id: Some(1),
            duration_ms: 1_000,
            duration_beats: Some(2.0),
            conform_to_tempo: true,
            loop_fill: false,
            source_offset_ms: 100,
            rate: Some(2.0),
            fade_in_ms: 400,
            fade_out_ms: 400,
            loop_count: 1,
            jump_to_event_id: Some(20),
        });
        timeline.video_clips.push(TimelineVideoClipSummary {
            id: protocol::TimelineVideoClipId(21),
            layer_id: 2,
            media_asset_id: 9,
            start_ms: 1_100,
            offset_ms: 500,
            duration_ms: 1_000,
            fade_in_ms: 400,
            fade_out_ms: 400,
        });
        timeline.audio_clips.push(TimelineAudioClipSummary {
            id: 22,
            layer_id: 3,
            media_asset_id: None,
            path: "trim.wav".to_string(),
            start_ms: 1_200,
            offset_ms: 500,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 400,
            fade_out_ms: 400,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        });
        timeline
            .automations
            .push(protocol::TimelineAutomationSummary {
                id: 23,
                fixture_id: 1,
                attribute: "Dimmer".to_string(),
                track: TimelineTrackKind::Lighting,
                timeline_layer_id: None,
                keyframes: vec![
                    AutomationKeyframeSummary {
                        time_ms: 1_300,
                        value: 10_000,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    AutomationKeyframeSummary {
                        time_ms: 1_800,
                        value: 20_000,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    AutomationKeyframeSummary {
                        time_ms: 2_300,
                        value: 30_000,
                        interpolation: protocol::AutomationInterpolation::Step,
                    },
                ],
                enabled: true,
            });
        timeline
            .video_automations
            .push(protocol::TimelineVideoAutomationSummary {
                id: 24,
                layer_id: 99,
                param: protocol::VideoParam::Opacity,
                track: TimelineTrackKind::Video,
                timeline_layer_id: None,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 1_400,
                        value: 0.25,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 1_900,
                        value: 0.75,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 2_400,
                        value: 1.0,
                        interpolation: protocol::AutomationInterpolation::Step,
                    },
                ],
                enabled: true,
            });
        let members = vec![
            TimelineItemRef::LightingEvent { event_id: 20 },
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(21),
            },
            TimelineItemRef::AudioClip { clip_id: 22 },
            TimelineItemRef::LightingAutomation { automation_id: 23 },
            TimelineItemRef::VideoAutomation { automation_id: 24 },
        ];
        timeline.item_groups.push(TimelineItemGroupSummary {
            id: TimelineItemGroupId(25),
            members: members.clone(),
        });

        let untrimmed = timeline.clone();
        let selected = trim_timeline_items(
            &mut timeline,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 22 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            TimelineTrimEdge::Start,
            1_250,
            (false, 120.0),
        )
        .expect("trim one member as the complete mixed group");
        assert_eq!(
            selected.into_iter().collect::<BTreeSet<_>>(),
            members.iter().copied().collect::<BTreeSet<_>>()
        );
        let event = &timeline.events[0];
        assert_eq!((event.time_ms, event.duration_ms), (1_250, 750));
        assert_eq!(
            (event.time_beats, event.duration_beats),
            (Some(2.5), Some(1.5))
        );
        assert_eq!(
            event.source_offset_ms, 600,
            "2x source clock advances exactly"
        );
        assert_eq!((event.fade_in_ms, event.fade_out_ms), (350, 400));
        assert_eq!(event.jump_to_event_id, Some(20));
        assert_eq!(
            (
                timeline.video_clips[0].start_ms,
                timeline.video_clips[0].offset_ms,
                timeline.video_clips[0].duration_ms,
                timeline.video_clips[0].fade_in_ms,
                timeline.video_clips[0].fade_out_ms,
            ),
            (1_350, 750, 750, 350, 400)
        );
        assert_eq!(
            (
                timeline.audio_clips[0].start_ms,
                timeline.audio_clips[0].offset_ms,
                timeline.audio_clips[0].duration_ms,
            ),
            (1_450, 750, 750)
        );
        assert_eq!(timeline.automations[0].keyframes[0].time_ms, 1_550);
        assert_eq!(timeline.automations[0].keyframes[0].value, 15_000);
        assert_eq!(timeline.video_automations[0].keyframes[0].time_ms, 1_650);
        assert!((timeline.video_automations[0].keyframes[0].value - 0.5).abs() < 1.0e-6);

        let mut isolated = untrimmed.clone();
        let isolated_selection = trim_timeline_items(
            &mut isolated,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 22 }],
            TimelineItemRef::AudioClip { clip_id: 22 },
            TimelineTrimEdge::Start,
            1_450,
            (true, 120.0),
        )
        .expect("Alt isolates exactly one trim without dissolving its group");
        assert_eq!(isolated_selection.len(), 5);
        assert_eq!(isolated.audio_clips[0].start_ms, 1_450);
        assert_eq!(isolated.events, untrimmed.events);
        assert_eq!(isolated.video_clips, untrimmed.video_clips);
        assert_eq!(isolated.item_groups, untrimmed.item_groups);

        let mut invalid = untrimmed.clone();
        invalid.audio_clips[0].duration_ms = 100;
        let invalid_before = invalid.clone();
        assert!(trim_timeline_items(
            &mut invalid,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 22 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            TimelineTrimEdge::Start,
            1_250,
            (false, 120.0),
        )
        .unwrap_err()
        .contains("complete span"));
        assert_eq!(
            invalid, invalid_before,
            "one invalid member preserves all of A"
        );

        let mut bezier = untrimmed.clone();
        bezier.automations[0].keyframes[0].interpolation =
            protocol::AutomationInterpolation::Bezier;
        let bezier_before = bezier.clone();
        assert!(trim_timeline_items(
            &mut bezier,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 23 }],
            TimelineItemRef::LightingAutomation { automation_id: 23 },
            TimelineTrimEdge::Start,
            1_550,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("existing keyframe"));
        assert_eq!(bezier, bezier_before);

        let mut looped = untrimmed;
        looped.events[0].conform_to_tempo = false;
        looped.events[0].duration_ms = 500;
        looped.events[0].loop_count = 2;
        let looped_before = looped.clone();
        assert!(trim_timeline_items(
            &mut looped,
            &[],
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            TimelineTrimEdge::End,
            2_251,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("whole-loop"));
        assert_eq!(looped, looped_before);

        let mut loop_fill = timeline.clone();
        loop_fill.events[0].loop_fill = true;
        loop_fill.events[0].duration_beats = Some(99.0);
        trim_timeline_items(
            &mut loop_fill,
            &[],
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            TimelineTrimEdge::End,
            1_750,
            (true, 120.0),
        )
        .expect("Loop-fill trim keeps millisecond and beat-domain spans coherent");
        assert_eq!(loop_fill.events[0].duration_ms, 500);
        assert_eq!(loop_fill.events[0].duration_beats, Some(1.0));
    }

    fn timeline_split_test_snapshot() -> TimelineSnapshot {
        let mut timeline = TimelineSnapshot {
            id: TimelineId(1),
            label: "Split".to_string(),
            layers: vec![
                protocol::TimelineLayerSummary {
                    id: 1,
                    label: "Lighting".to_string(),
                    order: 0,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: TimelineLayerKind::Lighting,
                },
                protocol::TimelineLayerSummary {
                    id: 2,
                    label: "Video".to_string(),
                    order: 1,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: TimelineLayerKind::Video,
                },
                protocol::TimelineLayerSummary {
                    id: 3,
                    label: "Audio".to_string(),
                    order: 2,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: TimelineLayerKind::Audio,
                },
            ],
            ..TimelineSnapshot::default()
        };
        timeline.events = vec![
            protocol::TimelineCueEventSummary {
                id: 20,
                cue_id: 7,
                time_ms: 1_000,
                time_beats: Some(2.0),
                track: TimelineTrackKind::Lighting,
                layer_id: Some(1),
                duration_ms: 1_000,
                duration_beats: Some(2.0),
                conform_to_tempo: true,
                loop_fill: false,
                source_offset_ms: 100,
                rate: Some(1.5),
                fade_in_ms: 120,
                fade_out_ms: 180,
                loop_count: 1,
                jump_to_event_id: Some(21),
            },
            protocol::TimelineCueEventSummary {
                id: 21,
                cue_id: 7,
                time_ms: 1_050,
                time_beats: Some(2.1),
                track: TimelineTrackKind::Lighting,
                layer_id: Some(1),
                duration_ms: 1_000,
                duration_beats: Some(2.0),
                conform_to_tempo: true,
                loop_fill: false,
                source_offset_ms: 200,
                rate: Some(1.5),
                fade_in_ms: 100,
                fade_out_ms: 100,
                loop_count: 1,
                jump_to_event_id: None,
            },
        ];
        timeline.video_clips.push(TimelineVideoClipSummary {
            id: protocol::TimelineVideoClipId(22),
            layer_id: 2,
            media_asset_id: 9,
            start_ms: 1_100,
            offset_ms: 200,
            duration_ms: 1_100,
            fade_in_ms: 120,
            fade_out_ms: 180,
        });
        timeline.audio_clips.push(TimelineAudioClipSummary {
            id: 23,
            layer_id: 3,
            media_asset_id: None,
            path: "split.wav".to_string(),
            start_ms: 1_200,
            offset_ms: 300,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 120,
            fade_out_ms: 180,
            output_bus: protocol::TimelineAudioOutputBus::Cue,
        });
        timeline
            .automations
            .push(protocol::TimelineAutomationSummary {
                id: 24,
                fixture_id: 1,
                attribute: "Dimmer".to_string(),
                track: TimelineTrackKind::Lighting,
                timeline_layer_id: None,
                keyframes: vec![
                    AutomationKeyframeSummary {
                        time_ms: 1_200,
                        value: 10_000,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    AutomationKeyframeSummary {
                        time_ms: 1_700,
                        value: 20_000,
                        interpolation: protocol::AutomationInterpolation::Step,
                    },
                    AutomationKeyframeSummary {
                        time_ms: 2_200,
                        value: 30_000,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                ],
                enabled: true,
            });
        timeline
            .video_automations
            .push(protocol::TimelineVideoAutomationSummary {
                id: 25,
                layer_id: 99,
                param: protocol::VideoParam::Opacity,
                track: TimelineTrackKind::Video,
                timeline_layer_id: None,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 1_300,
                        value: 0.2,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 1_800,
                        value: 0.8,
                        interpolation: protocol::AutomationInterpolation::Step,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 2_300,
                        value: 1.0,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                ],
                enabled: true,
            });
        timeline.item_groups.push(TimelineItemGroupSummary {
            id: TimelineItemGroupId(26),
            members: vec![
                TimelineItemRef::LightingEvent { event_id: 20 },
                TimelineItemRef::LightingEvent { event_id: 21 },
                TimelineItemRef::VideoClip {
                    clip_id: protocol::TimelineVideoClipId(22),
                },
                TimelineItemRef::AudioClip { clip_id: 23 },
                TimelineItemRef::LightingAutomation { automation_id: 24 },
                TimelineItemRef::VideoAutomation { automation_id: 25 },
            ],
        });
        timeline
    }

    #[test]
    fn timeline_split_items_is_atomic_across_all_domains_groups_and_isolate() {
        let harness = MediaAssetA6CommandHarness::new();
        let original = timeline_split_test_snapshot();
        let mut timeline = original.clone();
        let selected = split_timeline_items(
            &harness.state,
            &mut timeline,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            1_500,
            (false, 120.0),
        )
        .expect("split one grouped member as the complete mixed group");
        assert_eq!(selected.len(), 6);
        assert_eq!(timeline.events.len(), 4);
        assert_eq!(timeline.video_clips.len(), 2);
        assert_eq!(timeline.audio_clips.len(), 2);
        assert_eq!(timeline.automations.len(), 2);
        assert_eq!(timeline.video_automations.len(), 2);
        assert_eq!(timeline.item_groups.len(), 2);
        assert_eq!(
            timeline.item_groups[0], original.item_groups[0],
            "the original group remains the left-side group"
        );
        let right_group = timeline
            .item_groups
            .iter()
            .find(|group| group.id != TimelineItemGroupId(26))
            .expect("full-group split creates a fresh right-side group");
        assert_eq!(
            right_group.members.iter().copied().collect::<BTreeSet<_>>(),
            selected.iter().copied().collect::<BTreeSet<_>>()
        );

        let right_event_id = selected
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::LightingEvent { event_id } => Some(*event_id),
                _ => None,
            })
            .expect("right Scene Block selection");
        let right_events = selected
            .iter()
            .filter_map(|item| match item {
                TimelineItemRef::LightingEvent { event_id } => Some(*event_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let right_event = timeline
            .events
            .iter()
            .find(|event| event.id == right_event_id)
            .expect("right Scene Block exists");
        assert_eq!((right_event.time_ms, right_event.duration_ms), (1_400, 600));
        assert_eq!(right_event.source_offset_ms, 700);
        assert_eq!(
            (
                timeline.events[0].fade_in_ms,
                timeline.events[0].fade_out_ms,
                right_event.fade_in_ms,
                right_event.fade_out_ms,
            ),
            (120, 0, 0, 180),
            "a Scene Block split preserves the envelope without manufacturing a cut fade"
        );
        assert!(
            right_events.contains(&right_event.jump_to_event_id.expect("rewired right jump")),
            "internal Scene Block jumps follow the right-side stable IDs"
        );
        assert_eq!(
            (
                timeline.events[0].duration_ms,
                timeline.video_clips[0].duration_ms
            ),
            (400, 400)
        );
        let right_video_id = selected
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::VideoClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("right Video clip selection");
        let right_audio_id = selected
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::AudioClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("right Audio clip selection");
        let right_video = timeline
            .video_clips
            .iter()
            .find(|clip| clip.id == right_video_id)
            .unwrap();
        let right_audio = timeline
            .audio_clips
            .iter()
            .find(|clip| clip.id == right_audio_id)
            .unwrap();
        assert_eq!(
            (
                right_video.start_ms,
                right_video.offset_ms,
                right_video.duration_ms
            ),
            (1_500, 600, 700)
        );
        assert_eq!(
            (
                right_audio.start_ms,
                right_audio.offset_ms,
                right_audio.duration_ms
            ),
            (1_600, 700, 600)
        );
        assert_eq!(
            (
                timeline.video_clips[0].fade_in_ms,
                timeline.video_clips[0].fade_out_ms,
                right_video.fade_in_ms,
                right_video.fade_out_ms,
            ),
            (120, 0, 0, 180),
            "a split preserves the Video envelope without manufacturing a cut fade"
        );
        assert_eq!(
            (
                timeline.audio_clips[0].fade_in_ms,
                timeline.audio_clips[0].fade_out_ms,
                right_audio.fade_in_ms,
                right_audio.fade_out_ms,
            ),
            (120, 0, 0, 180),
            "a split preserves the Audio envelope without manufacturing a cut fade"
        );
        assert_eq!(
            (timeline.audio_clips[0].output_bus, right_audio.output_bus),
            (
                protocol::TimelineAudioOutputBus::Cue,
                protocol::TimelineAudioOutputBus::Cue
            ),
            "split preserves CUE on both exact source segments"
        );
        assert_eq!(
            right_audio.start_ms - right_video.start_ms,
            original.audio_clips[0].start_ms - original.video_clips[0].start_ms,
            "the primary-relative offset is applied to every group member"
        );
        let left_lighting_keys = &timeline.automations[0].keyframes;
        let right_lighting = timeline
            .automations
            .iter()
            .find(|automation| automation.id != 24)
            .unwrap();
        assert_eq!(left_lighting_keys.last().unwrap().time_ms, 1_600);
        assert_eq!(right_lighting.keyframes[0].time_ms, 1_600);
        assert_eq!(left_lighting_keys.last().unwrap().value, 18_000);
        let left_video_keys = &timeline.video_automations[0].keyframes;
        let right_video_automation = timeline
            .video_automations
            .iter()
            .find(|automation| automation.id != 25)
            .unwrap();
        assert_eq!(left_video_keys.last().unwrap().time_ms, 1_700);
        assert_eq!(right_video_automation.keyframes[0].time_ms, 1_700);
        assert!((left_video_keys.last().unwrap().value - 0.68).abs() < 1.0e-6);

        let mut isolated = original.clone();
        let isolated_selected = split_timeline_items(
            &harness.state,
            &mut isolated,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::AudioClip { clip_id: 23 },
            1_600,
            (true, 120.0),
        )
        .expect("isolate splits only the primary without dissolving its group");
        assert_eq!(isolated_selected.len(), 1);
        assert_eq!(isolated.item_groups, original.item_groups);
        assert_eq!(isolated.audio_clips.len(), 2);
        assert_eq!(isolated.events, original.events);
        assert_eq!(isolated.video_clips, original.video_clips);
        assert!(
            isolated
                .item_groups
                .iter()
                .all(|group| !group.members.contains(&isolated_selected[0])),
            "the fresh isolate right half intentionally remains ungrouped"
        );

        let mut locked = original.clone();
        locked.layers[2].locked = true;
        let locked_before = locked.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut locked,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            1_500,
            (false, 120.0),
        )
        .unwrap_err()
        .contains("locked lane"));
        assert_eq!(locked, locked_before);

        let mut bezier = original.clone();
        bezier.automations[0].keyframes[0].interpolation =
            protocol::AutomationInterpolation::Bezier;
        let bezier_before = bezier.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut bezier,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 24 }],
            TimelineItemRef::LightingAutomation { automation_id: 24 },
            1_600,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("existing keyframe"));
        assert_eq!(bezier, bezier_before);

        let mut invalid_boundary = original.clone();
        let invalid_boundary_before = invalid_boundary.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut invalid_boundary,
            &[],
            &[TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            1_100,
            (false, 120.0),
        )
        .unwrap_err()
        .contains("strictly inside"));
        assert_eq!(invalid_boundary, invalid_boundary_before);

        let mut video_fade_ramp = original.clone();
        let video_fade_ramp_before = video_fade_ramp.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut video_fade_ramp,
            &[],
            &[TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            1_150,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("fade envelope"));
        assert_eq!(video_fade_ramp, video_fade_ramp_before);

        let mut audio_fade_ramp = original.clone();
        let audio_fade_ramp_before = audio_fade_ramp.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut audio_fade_ramp,
            &[],
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::AudioClip { clip_id: 23 },
            2_100,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("fade envelope"));
        assert_eq!(audio_fade_ramp, audio_fade_ramp_before);

        let mut scene_fade_ramp = original.clone();
        let scene_fade_ramp_before = scene_fade_ramp.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut scene_fade_ramp,
            &[],
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            1_050,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("fade envelope"));
        assert_eq!(scene_fade_ramp, scene_fade_ramp_before);

        let mut looped = original.clone();
        looped.events[0].conform_to_tempo = false;
        looped.events[0].duration_ms = 500;
        looped.events[0].loop_count = 2;
        let looped_before = looped.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut looped,
            &[],
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            1_400,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("loop playback is ambiguous"));
        assert_eq!(looped, looped_before);
    }

    #[test]
    fn timeline_automation_lock_preflight_uses_explicit_timeline_lane_ids() {
        let harness = MediaAssetA6CommandHarness::new();
        let mut base = timeline_split_test_snapshot();
        base.layers.extend([
            protocol::TimelineLayerSummary {
                id: 4,
                label: "Lighting automation lane".to_string(),
                order: 3,
                muted: false,
                locked: false,
                solo: false,
                expanded: false,
                kind: TimelineLayerKind::Lighting,
            },
            protocol::TimelineLayerSummary {
                id: 5,
                label: "Video automation lane".to_string(),
                order: 4,
                muted: false,
                locked: false,
                solo: false,
                expanded: false,
                kind: TimelineLayerKind::Video,
            },
        ]);
        base.layers[0].locked = true;
        base.layers[1].locked = true;
        base.automations[0].timeline_layer_id = Some(4);
        base.video_automations[0].timeline_layer_id = Some(5);

        // The first compatible lane of each kind is locked, but the explicit
        // lane is not.  Trim/Split must use the automation's authored lane,
        // not the historical first-lane fallback.
        assert!(!timeline_item_is_on_locked_layer(
            &base,
            TimelineItemRef::LightingAutomation { automation_id: 24 },
        )
        .expect("explicit Lighting lane resolves"));
        let mut lighting_explicit_unlocked = base.clone();
        trim_timeline_items(
            &mut lighting_explicit_unlocked,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 24 }],
            TimelineItemRef::LightingAutomation { automation_id: 24 },
            TimelineTrimEdge::Start,
            1_600,
            (true, 120.0),
        )
        .expect("an explicit unlocked Lighting automation lane may be trimmed");

        assert!(!timeline_item_is_on_locked_layer(
            &base,
            TimelineItemRef::VideoAutomation { automation_id: 25 },
        )
        .expect("explicit Video lane resolves"));
        let mut video_explicit_unlocked = base.clone();
        split_timeline_items(
            &harness.state,
            &mut video_explicit_unlocked,
            &[],
            &[TimelineItemRef::VideoAutomation { automation_id: 25 }],
            TimelineItemRef::VideoAutomation { automation_id: 25 },
            1_700,
            (true, 120.0),
        )
        .expect("an explicit unlocked Video automation lane may be split");

        // Invert the locks: an unlocked first lane must not bypass an
        // explicit locked automation lane.  Each operation retains A.
        let mut lighting_explicit_locked = base.clone();
        lighting_explicit_locked.layers[0].locked = false;
        lighting_explicit_locked
            .layers
            .iter_mut()
            .find(|layer| layer.id == 4)
            .unwrap()
            .locked = true;
        let lighting_locked_before = lighting_explicit_locked.clone();
        assert!(timeline_item_is_on_locked_layer(
            &lighting_explicit_locked,
            TimelineItemRef::LightingAutomation { automation_id: 24 },
        )
        .expect("explicit locked Lighting lane resolves"));
        assert!(trim_timeline_items(
            &mut lighting_explicit_locked,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 24 }],
            TimelineItemRef::LightingAutomation { automation_id: 24 },
            TimelineTrimEdge::Start,
            1_600,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("locked lane"));
        assert_eq!(lighting_explicit_locked, lighting_locked_before);

        let mut video_explicit_locked = base.clone();
        video_explicit_locked.layers[1].locked = false;
        video_explicit_locked
            .layers
            .iter_mut()
            .find(|layer| layer.id == 5)
            .unwrap()
            .locked = true;
        let video_locked_before = video_explicit_locked.clone();
        assert!(timeline_item_is_on_locked_layer(
            &video_explicit_locked,
            TimelineItemRef::VideoAutomation { automation_id: 25 },
        )
        .expect("explicit locked Video lane resolves"));
        assert!(split_timeline_items(
            &harness.state,
            &mut video_explicit_locked,
            &[],
            &[TimelineItemRef::VideoAutomation { automation_id: 25 }],
            TimelineItemRef::VideoAutomation { automation_id: 25 },
            1_700,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("locked lane"));
        assert_eq!(video_explicit_locked, video_locked_before);

        // A stale explicit lane must be rejected by the lock preflight rather
        // than treated as an unlocked lane and deferred to a later validator.
        let mut missing_lighting_lane = base.clone();
        missing_lighting_lane.automations[0].timeline_layer_id = Some(999);
        let missing_lighting_before = missing_lighting_lane.clone();
        assert!(trim_timeline_items(
            &mut missing_lighting_lane,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 24 }],
            TimelineItemRef::LightingAutomation { automation_id: 24 },
            TimelineTrimEdge::Start,
            1_600,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("missing Timeline lane"));
        assert_eq!(missing_lighting_lane, missing_lighting_before);

        let mut missing_video_lane = base;
        missing_video_lane.video_automations[0].timeline_layer_id = Some(999);
        let missing_video_before = missing_video_lane.clone();
        assert!(split_timeline_items(
            &harness.state,
            &mut missing_video_lane,
            &[],
            &[TimelineItemRef::VideoAutomation { automation_id: 25 }],
            TimelineItemRef::VideoAutomation { automation_id: 25 },
            1_700,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("missing Timeline lane"));
        assert_eq!(missing_video_lane, missing_video_before);
    }

    #[test]
    fn timeline_automation_lock_preflight_preserves_legacy_implicit_lanes() {
        let harness = MediaAssetA6CommandHarness::new();
        let mut legacy = timeline_split_test_snapshot();
        legacy.layers.clear();
        legacy.automations[0].timeline_layer_id = None;
        legacy.video_automations[0].timeline_layer_id = None;
        let legacy_a = legacy.clone();

        // Historical persistence has no authored lanes.  `None` therefore
        // means implicit unlocked Lighting=0 / Video=1 during preflight, not
        // a missing lane and not an A mutation.
        assert!(!timeline_item_is_on_locked_layer(
            &legacy,
            TimelineItemRef::LightingAutomation { automation_id: 24 },
        )
        .expect("legacy Lighting automation resolves its implicit lane"));
        assert!(!timeline_item_is_on_locked_layer(
            &legacy,
            TimelineItemRef::VideoAutomation { automation_id: 25 },
        )
        .expect("legacy Video automation resolves its implicit lane"));

        trim_timeline_items(
            &mut legacy,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 24 }],
            TimelineItemRef::LightingAutomation { automation_id: 24 },
            TimelineTrimEdge::Start,
            1_600,
            (true, 120.0),
        )
        .expect("legacy implicit Lighting automation may be trimmed");
        split_timeline_items(
            &harness.state,
            &mut legacy,
            &[],
            &[TimelineItemRef::VideoAutomation { automation_id: 25 }],
            TimelineItemRef::VideoAutomation { automation_id: 25 },
            1_700,
            (true, 120.0),
        )
        .expect("legacy implicit Video automation may be split");

        // B contains only the intended automation edits; its persisted
        // legacy lane representation remains empty/None just as A did.
        assert!(legacy.layers.is_empty());
        assert!(legacy_a.layers.is_empty());
        assert_eq!(legacy.automations[0].timeline_layer_id, None);
        assert_eq!(legacy.video_automations[0].timeline_layer_id, None);
        assert!(legacy
            .video_automations
            .iter()
            .all(|automation| automation.timeline_layer_id.is_none()));
        assert_eq!(legacy.automations[0].keyframes[0].time_ms, 1_600);
        assert_eq!(legacy.video_automations.len(), 2);

        // An explicit lane never adopts implicit semantics: it is stale in
        // this legacy A and must still fail closed before mutation.
        let mut stale_explicit = legacy_a;
        stale_explicit.automations[0].timeline_layer_id = Some(0);
        let stale_explicit_a = stale_explicit.clone();
        assert!(trim_timeline_items(
            &mut stale_explicit,
            &[],
            &[TimelineItemRef::LightingAutomation { automation_id: 24 }],
            TimelineItemRef::LightingAutomation { automation_id: 24 },
            TimelineTrimEdge::Start,
            1_600,
            (true, 120.0),
        )
        .unwrap_err()
        .contains("missing Timeline lane"));
        assert_eq!(stale_explicit, stale_explicit_a);
    }

    #[test]
    fn timeline_move_items_to_lanes_is_atomic_across_all_domains_and_isolate() {
        let mut original = timeline_split_test_snapshot();
        let target_layer = |id: u32, label: &str, order: u32, kind: TimelineLayerKind| {
            protocol::TimelineLayerSummary {
                id,
                label: label.to_string(),
                order,
                muted: false,
                locked: false,
                solo: false,
                expanded: false,
                kind,
            }
        };
        original.layers.extend([
            target_layer(4, "Lighting target", 3, TimelineLayerKind::Lighting),
            target_layer(5, "Video target", 4, TimelineLayerKind::Video),
            target_layer(6, "Audio target", 5, TimelineLayerKind::Audio),
        ]);
        let group_before = original.item_groups.clone();
        let full_targets = vec![
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 20 },
                target_layer_id: 4,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 21 },
                target_layer_id: 4,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::VideoClip {
                    clip_id: protocol::TimelineVideoClipId(22),
                },
                target_layer_id: 5,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::AudioClip { clip_id: 23 },
                target_layer_id: 6,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingAutomation { automation_id: 24 },
                target_layer_id: 4,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::VideoAutomation { automation_id: 25 },
                target_layer_id: 5,
            },
        ];

        let mut moved = original.clone();
        let selected = move_timeline_items_to_lanes(
            &mut moved,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &full_targets,
            120,
            false,
            120.0,
        )
        .expect("move the complete heterogeneous group with its exact lane targets");
        assert_eq!(selected.len(), 6);
        assert_eq!(moved.item_groups, group_before);
        assert_eq!(moved.events[0].layer_id, Some(4));
        assert_eq!(moved.events[0].track, original.events[0].track);
        assert_eq!(moved.events[0].time_ms, original.events[0].time_ms + 120);
        assert!(
            (moved.events[0].time_beats.unwrap() - original.events[0].time_beats.unwrap() - 0.24)
                .abs()
                < 1.0e-9
        );
        assert_eq!(moved.video_clips[0].layer_id, 5);
        assert_eq!(
            moved.video_clips[0].start_ms,
            original.video_clips[0].start_ms + 120
        );
        assert_eq!(moved.audio_clips[0].layer_id, 6);
        assert_eq!(
            moved.audio_clips[0].output_bus,
            protocol::TimelineAudioOutputBus::Cue,
            "lane movement preserves the Audio logical bus"
        );
        assert_eq!(
            moved.audio_clips[0].start_ms,
            original.audio_clips[0].start_ms + 120
        );
        assert_eq!(moved.automations[0].timeline_layer_id, Some(4));
        assert_eq!(
            moved.automations[0].keyframes[0].time_ms,
            original.automations[0].keyframes[0].time_ms + 120
        );
        assert_eq!(moved.video_automations[0].timeline_layer_id, Some(5));
        assert_eq!(
            moved.video_automations[0].layer_id, original.video_automations[0].layer_id,
            "a Timeline lane move must not retarget the automated Video layer"
        );
        assert_eq!(
            moved.video_automations[0].keyframes[0].time_ms,
            original.video_automations[0].keyframes[0].time_ms + 120
        );

        let mut isolated = original.clone();
        let isolated_selected = move_timeline_items_to_lanes(
            &mut isolated,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::AudioClip { clip_id: 23 },
            &[TimelineItemLaneTarget {
                item: TimelineItemRef::AudioClip { clip_id: 23 },
                target_layer_id: 6,
            }],
            -100,
            true,
            120.0,
        )
        .expect("Alt isolates one member for a diagonal lane move");
        assert_eq!(
            isolated_selected,
            vec![TimelineItemRef::AudioClip { clip_id: 23 }]
        );
        assert_eq!(isolated.item_groups, group_before);
        assert_eq!(isolated.audio_clips[0].layer_id, 6);
        assert_eq!(
            isolated.audio_clips[0].start_ms,
            original.audio_clips[0].start_ms - 100
        );
        assert_eq!(isolated.events, original.events);
        assert_eq!(isolated.video_clips, original.video_clips);
        assert_eq!(isolated.automations, original.automations);
        assert_eq!(isolated.video_automations, original.video_automations);

        let mut horizontal_alt = original.clone();
        move_timeline_items_to_lanes(
            &mut horizontal_alt,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::AudioClip { clip_id: 23 },
            &[TimelineItemLaneTarget {
                item: TimelineItemRef::AudioClip { clip_id: 23 },
                target_layer_id: 3,
            }],
            100,
            true,
            120.0,
        )
        .expect("Alt horizontal motion on the current lane is not a no-op");
        assert_eq!(horizontal_alt.audio_clips[0].layer_id, 3);
        assert_eq!(
            horizontal_alt.audio_clips[0].start_ms,
            original.audio_clips[0].start_ms + 100
        );

        let mut video_scene = original.clone();
        video_scene.events[0].track = TimelineTrackKind::Video;
        move_timeline_items_to_lanes(
            &mut video_scene,
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            &[TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 20 },
                target_layer_id: 2,
            }],
            0,
            true,
            120.0,
        )
        .expect("a Video-track Scene may move to a Video Timeline lane");
        assert_eq!(video_scene.events[0].layer_id, Some(2));
        assert_eq!(video_scene.events[0].track, TimelineTrackKind::Video);
        assert_eq!(
            video_scene.events[1], original.events[1],
            "Alt Scene move keeps the rest of the authored group unchanged"
        );

        let mut audio_scene = original.clone();
        let audio_scene_before = audio_scene.clone();
        assert!(move_timeline_items_to_lanes(
            &mut audio_scene,
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            &[TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 20 },
                target_layer_id: 3,
            }],
            0,
            true,
            120.0,
        )
        .unwrap_err()
        .contains("incompatible"));
        assert_eq!(audio_scene, audio_scene_before);

        let mut incompatible = original.clone();
        let incompatible_before = incompatible.clone();
        let mut incompatible_targets = full_targets.clone();
        incompatible_targets[3].target_layer_id = 4;
        assert!(move_timeline_items_to_lanes(
            &mut incompatible,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &incompatible_targets,
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("incompatible"));
        assert_eq!(incompatible, incompatible_before);

        let mut locked_target = original.clone();
        locked_target
            .layers
            .iter_mut()
            .find(|layer| layer.id == 4)
            .unwrap()
            .locked = true;
        let locked_target_before = locked_target.clone();
        assert!(move_timeline_items_to_lanes(
            &mut locked_target,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &full_targets,
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("locked lane"));
        assert_eq!(locked_target, locked_target_before);

        let mut locked_source = original.clone();
        locked_source
            .layers
            .iter_mut()
            .find(|layer| layer.id == 2)
            .unwrap()
            .locked = true;
        let locked_source_before = locked_source.clone();
        assert!(move_timeline_items_to_lanes(
            &mut locked_source,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &full_targets,
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("locked lane"));
        assert_eq!(locked_source, locked_source_before);

        let no_op_targets = vec![
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 20 },
                target_layer_id: 1,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 21 },
                target_layer_id: 1,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::VideoClip {
                    clip_id: protocol::TimelineVideoClipId(22),
                },
                target_layer_id: 2,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::AudioClip { clip_id: 23 },
                target_layer_id: 3,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingAutomation { automation_id: 24 },
                target_layer_id: 1,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::VideoAutomation { automation_id: 25 },
                target_layer_id: 2,
            },
        ];
        let mut no_op = original.clone();
        let no_op_before = no_op.clone();
        assert!(move_timeline_items_to_lanes(
            &mut no_op,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &no_op_targets,
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("already selected"));
        assert_eq!(no_op, no_op_before);

        let mut duplicate_target = original.clone();
        let duplicate_target_before = duplicate_target.clone();
        let mut duplicate_targets = full_targets.clone();
        duplicate_targets.push(full_targets[0].clone());
        assert!(move_timeline_items_to_lanes(
            &mut duplicate_target,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &duplicate_targets,
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("duplicate"));
        assert_eq!(duplicate_target, duplicate_target_before);

        let mut missing_target = original.clone();
        let missing_target_before = missing_target.clone();
        assert!(move_timeline_items_to_lanes(
            &mut missing_target,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &full_targets[..full_targets.len() - 1],
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("exactly once"));
        assert_eq!(missing_target, missing_target_before);

        let mut extra_target = original.clone();
        let extra_target_before = extra_target.clone();
        let mut extra_targets = full_targets.clone();
        extra_targets.push(TimelineItemLaneTarget {
            item: TimelineItemRef::LightingEvent { event_id: 9_999 },
            target_layer_id: 4,
        });
        assert!(move_timeline_items_to_lanes(
            &mut extra_target,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &extra_targets,
            0,
            false,
            120.0,
        )
        .unwrap_err()
        .contains("unselected"));
        assert_eq!(extra_target, extra_target_before);

        let mut underflow = original.clone();
        let underflow_before = underflow.clone();
        assert!(move_timeline_items_to_lanes(
            &mut underflow,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &full_targets,
            -2_000,
            false,
            120.0,
        )
        .is_err());
        assert_eq!(underflow, underflow_before);

        let mut overflow = original.clone();
        overflow.events[0].time_ms = u64::MAX - 10;
        let overflow_before = overflow.clone();
        assert!(move_timeline_items_to_lanes(
            &mut overflow,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::VideoClip {
                clip_id: protocol::TimelineVideoClipId(22),
            },
            &full_targets,
            120,
            false,
            120.0,
        )
        .is_err());
        assert_eq!(overflow, overflow_before);
    }

    #[test]
    fn timeline_move_items_to_lanes_materializes_legacy_effective_lanes_atomically() {
        let mut legacy = timeline_split_test_snapshot();
        legacy.layers.clear();
        for event in &mut legacy.events {
            event.layer_id = None;
        }
        legacy.events[1].track = TimelineTrackKind::Video;
        legacy.video_clips[0].layer_id = 1;
        legacy.audio_clips[0].layer_id = 2;
        legacy.automations[0].timeline_layer_id = None;
        legacy.video_automations[0].timeline_layer_id = None;
        let targets = vec![
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 20 },
                target_layer_id: 0,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingEvent { event_id: 21 },
                target_layer_id: 1,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::VideoClip {
                    clip_id: protocol::TimelineVideoClipId(22),
                },
                target_layer_id: 1,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::AudioClip { clip_id: 23 },
                target_layer_id: 2,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::LightingAutomation { automation_id: 24 },
                target_layer_id: 0,
            },
            TimelineItemLaneTarget {
                item: TimelineItemRef::VideoAutomation { automation_id: 25 },
                target_layer_id: 1,
            },
        ];
        move_timeline_items_to_lanes(
            &mut legacy,
            &[TimelineItemRef::LightingEvent { event_id: 20 }],
            TimelineItemRef::LightingEvent { event_id: 20 },
            &targets,
            100,
            false,
            120.0,
        )
        .expect("legacy implicit lanes accept the UI Lighting/Video/Audio targets");
        assert_eq!(
            legacy
                .layers
                .iter()
                .map(|layer| (layer.id, layer.kind, layer.order))
                .collect::<Vec<_>>(),
            vec![
                (2, TimelineLayerKind::Audio, 0),
                (0, TimelineLayerKind::Lighting, 1),
                (1, TimelineLayerKind::Video, 2),
            ],
            "materialization preserves the engine's canonical Audio/Lighting/Video order"
        );
        assert_eq!(legacy.events[0].layer_id, Some(0));
        assert_eq!(legacy.events[0].track, TimelineTrackKind::Lighting);
        assert_eq!(legacy.events[1].layer_id, Some(1));
        assert_eq!(legacy.events[1].track, TimelineTrackKind::Video);
        assert_eq!(legacy.automations[0].timeline_layer_id, Some(0));
        assert_eq!(legacy.video_automations[0].timeline_layer_id, Some(1));
        assert_eq!(legacy.audio_clips[0].layer_id, 2);

        let mut colliding_audio = timeline_split_test_snapshot();
        colliding_audio.layers.clear();
        colliding_audio.audio_clips[0].layer_id = 0;
        let colliding_before = colliding_audio.clone();
        assert!(move_timeline_items_to_lanes(
            &mut colliding_audio,
            &[TimelineItemRef::AudioClip { clip_id: 23 }],
            TimelineItemRef::AudioClip { clip_id: 23 },
            &[TimelineItemLaneTarget {
                item: TimelineItemRef::AudioClip { clip_id: 23 },
                target_layer_id: 2,
            }],
            100,
            true,
            120.0,
        )
        .unwrap_err()
        .contains("collides"));
        assert_eq!(
            colliding_audio, colliding_before,
            "a failed legacy materialization must leave the complete implicit A image untouched"
        );
    }

    fn assert_timeline_insert_media_rejected_without_delta(
        harness: &MediaAssetA6CommandHarness,
        request: TimelineAdvancedMutationRequest,
        request_id: u64,
        expected_error: &str,
    ) {
        let before_snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read InsertMedia rejection baseline");
        let before_project_hash = b3_persistence_hash(harness);
        let baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(harness);
        let error = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            request_id,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("invalid explicit InsertMedia lane must fail closed");
        assert!(
            error.contains(expected_error),
            "unexpected InsertMedia rejection: {error}"
        );
        let after_snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read InsertMedia rejection result");
        // Engine runtime telemetry/clock counters continue to tick while a
        // command is being rejected.  Compare the authored persistence image
        // (the mutation boundary) rather than those live counters.
        assert_eq!(
            after_snapshot.timeline, before_snapshot.timeline,
            "rejected InsertMedia must not publish a Timeline snapshot delta"
        );
        assert_eq!(
            after_snapshot.timeline_bank, before_snapshot.timeline_bank,
            "rejected InsertMedia must not publish a Timeline bank delta"
        );
        assert_eq!(
            b3_persistence_hash(harness),
            before_project_hash,
            "rejected InsertMedia must not change the authored project checkpoint"
        );
        let after = harness.mutation_baseline();
        assert_eq!(
            (
                after.revision,
                after.history_generation,
                after.undo_len,
                after.next_transaction_id,
                after.publication_generation,
            ),
            (
                baseline.revision,
                baseline.history_generation,
                baseline.undo_len,
                baseline.next_transaction_id,
                baseline.publication_generation,
            ),
            "rejected InsertMedia must not advance history, revision, transaction, or publication"
        );
    }

    fn apply_root_scene_block_timing_authoritative<F>(
        harness: &MediaAssetA6CommandHarness,
        request_id: u64,
        mutate: F,
    ) -> TimelineAdvancedAuthoritativeResult
    where
        F: FnOnce(&mut protocol::TimelineEventPlacementUpdate),
    {
        let snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read root Scene Block timing baseline");
        let event = snapshot
            .timeline
            .events
            .first()
            .cloned()
            .expect("root Scene Block timing fixture has an event");
        let mut update = protocol::TimelineEventPlacementUpdate {
            event_id: event.id,
            cue_id: event.cue_id,
            time_ms: event.time_ms,
            time_beats: event.time_beats,
            track: event.track.clone(),
            layer_id: event.layer_id,
            duration_ms: event.duration_ms,
            duration_beats: event.duration_beats,
            conform_to_tempo: event.conform_to_tempo,
            loop_fill: event.loop_fill,
            fade_in_ms: event.fade_in_ms,
            fade_out_ms: event.fade_out_ms,
            loop_count: event.loop_count,
            jump_to_event_id: event.jump_to_event_id,
        };
        mutate(&mut update);
        let mut authoring = timeline_advanced_authoring_from_snapshot(&snapshot.timeline);
        authoring.snap_request = Some(TimelineSnapRequest {
            event_placements: vec![update],
            lighting_automations: Vec::new(),
            video_automations: Vec::new(),
        });
        let (epoch, revision, hash) = b3_authority_arguments(harness);
        let request = TimelineAdvancedMutationRequest::Apply {
            authoring: Box::new(authoring),
        };
        // Capturing the production E/R/H fence may reconcile a just-navigated
        // history image. Measure the one Apply mutation only after that fence
        // is stable, exactly as the UI captures its commit baseline.
        let baseline = harness.mutation_baseline();
        let applied = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request.clone(),
            request_id,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("root Scene Block timing Apply succeeds");
        let retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            request_id,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss retry returns the original root Scene Block receipt");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "exact timing retry must not record a second authoritative history entry"
        );
        assert!(
            applied.authoring.snap_request.is_none(),
            "the canonical public result must not retain the publication-only snap request"
        );
        assert_one_authoritative_history_mutation(harness, baseline);
        b3_assert_authority_matches_persistence(harness);
        applied
    }

    struct RootSceneBlockHistoryPlatform;

    impl ProjectReplacementPlatform for RootSceneBlockHistoryPlatform {
        fn advance_recovery_authority(
            &self,
            _state: &AppState,
            coordinator: &mut ProjectCoordinator,
            transition: ProjectRecoveryAuthorityTransition,
        ) -> Result<u64, String> {
            advance_project_recovery_authority_serial_with_persist(
                coordinator,
                transition,
                |_, _| Ok(()),
            )
        }

        fn fence_and_retire_outputs(&self, _state: &AppState) -> Result<(), String> {
            Ok(())
        }

        fn emit_authority_event(
            &self,
            _coordinator_effect: ProjectReplacementCoordinatorEffect,
            _result: &ProjectLoadResult,
        ) {
        }
    }

    fn root_scene_block_timing_harness() -> MediaAssetA6CommandHarness {
        let harness = MediaAssetA6CommandHarness::new();
        {
            let state = &harness.state;
            let cue_id = state.engine.allocate_cue_id();
            state
                .engine
                .create_cue_published(
                    cue_id,
                    protocol::DEFAULT_CUE_LIST_ID,
                    "Root Scene Block timing".to_string(),
                    None,
                    RecallMode::Coexist,
                    0,
                    Some(4.0),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
                .expect("seed Cue for root Scene Block timing");
            let mut seeded = state
                .engine
                .persistence_snapshot()
                .expect("read root Scene Block timing seed");
            let event_id = state.engine.allocate_timeline_event_id();
            let mut timeline = timeline_split_test_snapshot();
            timeline.id = seeded.timeline.id;
            timeline.label = seeded.timeline.label.clone();
            timeline.events.truncate(1);
            timeline.events[0].id = event_id;
            timeline.events[0].cue_id = cue_id;
            timeline.events[0].jump_to_event_id = None;
            let mut companion = timeline.events[0].clone();
            companion.id = state.engine.allocate_timeline_event_id();
            companion.time_ms = timeline.events[0].time_ms.saturating_add(10_000);
            companion.time_beats = timeline.events[0].time_beats.map(|beats| beats + 20.0);
            companion.jump_to_event_id = None;
            timeline.events.push(companion);
            timeline.item_groups.clear();
            timeline.video_clips.clear();
            timeline.audio_clips.clear();
            timeline.automations.clear();
            timeline.video_automations.clear();
            seeded.timeline = timeline.clone();
            seeded.timeline_bank = vec![timeline];
            state
                .engine
                .load_project_snapshot_and_wait(seeded)
                .expect("load root Scene Block timing seed through EngineHandle");
        }
        c1_stabilize_fixture_authority(&harness);
        harness
    }

    fn root_scene_block_timeline(harness: &MediaAssetA6CommandHarness) -> TimelineSnapshot {
        harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read root Scene Block Timeline")
            .timeline
    }

    fn root_scene_block_history_status(
        harness: &MediaAssetA6CommandHarness,
    ) -> ProjectHistoryStatus {
        let coordinator = lock_project_coordinator(&harness.state)
            .expect("lock root Scene Block history coordinator");
        project_history_status_for_coordinator(&coordinator)
    }

    fn assert_root_scene_block_roundtrip(
        harness: &MediaAssetA6CommandHarness,
        before: &TimelineSnapshot,
        after: &TimelineSnapshot,
        committed: &ProjectHistoryStatus,
        label: &str,
    ) {
        let state = &harness.state;
        let ready_to_undo = root_scene_block_history_status(harness);
        assert_eq!(
            ready_to_undo.undo_entry_id, committed.undo_entry_id,
            "{label}: the current Undo head remains the committed entry"
        );
        {
            let _external = lock_project_external_command_admission(state)
                .expect("lock root Scene Block external admission for Undo");
            let mut coordinator =
                lock_project_coordinator(state).expect("lock root Scene Block coordinator for Undo");
            navigate_project_history_with_coordinator(
                state,
                &mut coordinator,
                true,
                Some(ready_to_undo.project_epoch),
                ready_to_undo.undo_entry_id,
                Some(ready_to_undo.checkpoint_hash.clone()),
                |state, prepared, coordinator| {
                    replace_prepared_project_snapshot_with_coordinator_and_platform(
                        state,
                        prepared,
                        coordinator,
                        project_replacement_plan!(
                            ProjectReplacementCoordinatorEffect::RevisionMutation,
                            false,
                            no_project_replacement_validation,
                            no_project_replacement_capture,
                            None
                        ),
                        &RootSceneBlockHistoryPlatform,
                    )
                    .map(|(result, (), _)| result)
                },
            )
            .unwrap_or_else(|error| panic!("{label} Undo navigation failed: {error}"));
        }
        assert_eq!(
            root_scene_block_timeline(harness),
            *before,
            "{label}: Undo restores the exact prior Timeline image"
        );
        b3_assert_authority_matches_persistence(harness);

        let ready_to_redo = root_scene_block_history_status(harness);
        assert_eq!(
            ready_to_redo.redo_entry_id, committed.undo_entry_id,
            "{label}: Redo targets the same terminal committed entry"
        );
        let redone = {
            let _external = lock_project_external_command_admission(state)
                .expect("lock root Scene Block external admission for Redo");
            let mut coordinator =
                lock_project_coordinator(state).expect("lock root Scene Block coordinator for Redo");
            navigate_project_history_with_coordinator(
                state,
                &mut coordinator,
                false,
                Some(ready_to_redo.project_epoch),
                ready_to_redo.redo_entry_id,
                ready_to_redo.redo_checkpoint_hash.clone(),
                |state, prepared, coordinator| {
                    replace_prepared_project_snapshot_with_coordinator_and_platform(
                        state,
                        prepared,
                        coordinator,
                        project_replacement_plan!(
                            ProjectReplacementCoordinatorEffect::RevisionMutation,
                            false,
                            no_project_replacement_validation,
                            no_project_replacement_capture,
                            None
                        ),
                        &RootSceneBlockHistoryPlatform,
                    )
                    .map(|(result, (), _)| result)
                },
            )
            .unwrap_or_else(|error| panic!("{label} Redo navigation failed: {error}"))
        };
        assert_eq!(
            root_scene_block_timeline(harness),
            *after,
            "{label}: Redo restores the exact committed Timeline image"
        );
        assert_eq!(
            redone.history_status,
            root_scene_block_history_status(harness),
            "{label}: history status stays authoritative"
        );
        b3_assert_authority_matches_persistence(harness);
    }

    fn prove_root_scene_block_timing_apply<F, V>(
        request_id: u64,
        label: &str,
        mutate: F,
        verify: V,
    ) where
        F: FnOnce(&mut protocol::TimelineEventPlacementUpdate),
        V: FnOnce(&TimelineSnapshot, &TimelineSnapshot),
    {
        let harness = root_scene_block_timing_harness();
        let before = root_scene_block_timeline(&harness);
        let applied =
            apply_root_scene_block_timing_authoritative(&harness, request_id, mutate);
        let after = root_scene_block_timeline(&harness);
        verify(&before, &after);
        let committed = applied.mutation.history_status;
        assert_eq!(committed, root_scene_block_history_status(&harness));
        assert_eq!(committed.undo_depth, 1, "{label}: one gesture adds one Undo entry");
        assert_root_scene_block_roundtrip(&harness, &before, &after, &committed, label);
    }

    fn assert_root_scene_block_identity_rejection(
        request_id: u64,
        mutate: fn(&mut protocol::TimelineEventPlacementUpdate),
        expected_error: &str,
    ) {
        let harness = root_scene_block_timing_harness();
        let state = &harness.state;
        let before = root_scene_block_timeline(&harness);
        let event = before.events[0].clone();
        let mut update = protocol::TimelineEventPlacementUpdate {
            event_id: event.id,
            cue_id: event.cue_id,
            time_ms: event.time_ms,
            time_beats: event.time_beats,
            track: event.track.clone(),
            layer_id: event.layer_id,
            duration_ms: event.duration_ms,
            duration_beats: event.duration_beats,
            conform_to_tempo: event.conform_to_tempo,
            loop_fill: event.loop_fill,
            fade_in_ms: event.fade_in_ms,
            fade_out_ms: event.fade_out_ms,
            loop_count: event.loop_count,
            jump_to_event_id: event.jump_to_event_id,
        };
        mutate(&mut update);
        let mut authoring = timeline_advanced_authoring_from_snapshot(&before);
        authoring.snap_request = Some(TimelineSnapRequest {
            event_placements: vec![update],
            lighting_automations: Vec::new(),
            video_automations: Vec::new(),
        });
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let before_hash = b3_persistence_hash(&harness);
        let baseline = harness.mutation_baseline();
        let error = apply_timeline_advanced_authoritative_command_impl(
            state,
            TimelineAdvancedMutationRequest::Apply {
                authoring: Box::new(authoring),
            },
            request_id,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("identity-changing Scene Block timing Apply must fail closed");
        assert!(
            error.contains(expected_error),
            "unexpected Scene Block identity rejection: {error}"
        );
        assert_eq!(
            root_scene_block_timeline(&harness),
            before,
            "identity rejection must leave the live Timeline unchanged"
        );
        assert_eq!(
            b3_persistence_hash(&harness),
            before_hash,
            "identity rejection must leave the persisted project checkpoint unchanged"
        );
        let after = harness.mutation_baseline();
        assert_eq!(
            (
                after.revision,
                after.history_generation,
                after.undo_len,
                after.next_transaction_id,
                after.publication_generation,
            ),
            (
                baseline.revision,
                baseline.history_generation,
                baseline.undo_len,
                baseline.next_transaction_id,
                baseline.publication_generation,
            ),
            "identity rejection must not change history, revision, transaction, or publication"
        );
    }

    fn prove_root_scene_block_move_group() {
        let harness = root_scene_block_timing_harness();
        let state = &harness.state;
        let event_ids = root_scene_block_timeline(&harness)
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>();
        assert_eq!(event_ids.len(), 2, "MoveGroup fixture has two Scene Blocks");
        let (group_epoch, group_revision, group_hash) = b3_authority_arguments(&harness);
        let grouped = apply_timeline_advanced_authoritative_command_impl(
            state,
            TimelineAdvancedMutationRequest::Group {
                members: event_ids
                    .iter()
                    .map(|event_id| TimelineItemRef::LightingEvent {
                        event_id: *event_id,
                    })
                    .collect(),
            },
            86_210,
            group_epoch,
            group_revision,
            group_hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("seed a two-event group for the MoveGroup publication proof");
        let group_id = grouped
            .authoring
            .item_groups
            .first()
            .expect("MoveGroup fixture has exactly one item group")
            .id;
        c1_stabilize_fixture_authority(&harness);
        let before = root_scene_block_timeline(&harness);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let request = TimelineAdvancedMutationRequest::MoveGroup {
            group_id,
            delta_ms: 250,
        };
        let baseline = harness.mutation_baseline();
        let moved = apply_timeline_advanced_authoritative_command_impl(
            state,
            request.clone(),
            86_211,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("MoveGroup publishes its Scene Block timing delta");
        let retried = apply_timeline_advanced_authoritative_command_impl(
            state,
            request,
            86_211,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("MoveGroup reply-loss retry returns the original receipt");
        assert_eq!(
            serde_json::to_value(&moved).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "MoveGroup retry must not create a second history entry or publish twice"
        );
        assert!(
            moved.authoring.snap_request.is_none(),
            "MoveGroup public authoring must not retain the publication-only snap request"
        );
        assert_one_authoritative_history_mutation(&harness, baseline);
        let after = root_scene_block_timeline(&harness);
        assert_eq!(
            after.events[0].time_ms,
            before.events[0].time_ms + 250,
            "MoveGroup publication updates the live Scene Block timing"
        );
        assert_eq!(
            after.events[1].time_ms,
            before.events[1].time_ms + 250,
            "MoveGroup publication updates every grouped Scene Block"
        );
        assert_root_scene_block_roundtrip(
            &harness,
            &before,
            &after,
            &moved.mutation.history_status,
            "MoveGroup Scene Block timing",
        );
    }

    fn prove_root_scene_block_unknown_event_rejection() {
        let harness = root_scene_block_timing_harness();
        let state = &harness.state;
        let before = root_scene_block_timeline(&harness);
        let event = before.events[0].clone();
        let baseline = harness.mutation_baseline();
        let mut authoring = timeline_advanced_authoring_from_snapshot(&before);
        authoring.snap_request = Some(TimelineSnapRequest {
            event_placements: vec![protocol::TimelineEventPlacementUpdate {
                event_id: event.id + 99_999,
                cue_id: event.cue_id,
                time_ms: 2_000,
                time_beats: None,
                track: event.track.clone(),
                layer_id: event.layer_id,
                duration_ms: 1_000,
                duration_beats: None,
                conform_to_tempo: false,
                loop_fill: false,
                fade_in_ms: 0,
                fade_out_ms: 0,
                loop_count: 1,
                jump_to_event_id: None,
            }],
            lighting_automations: Vec::new(),
            video_automations: Vec::new(),
        });
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let error = apply_timeline_advanced_authoritative_command_impl(
            state,
            TimelineAdvancedMutationRequest::Apply {
                authoring: Box::new(authoring),
            },
            86_209,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("unknown Scene Block snap update must reject before publication");
        assert!(error.contains("was not found"), "unexpected timing rejection: {error}");
        assert_eq!(
            root_scene_block_timeline(&harness),
            before,
            "rejected timing Apply leaves the Timeline unchanged"
        );
        let after = harness.mutation_baseline();
        assert_eq!(
            (
                after.revision,
                after.history_generation,
                after.undo_len,
                after.next_transaction_id,
                after.publication_generation,
            ),
            (
                baseline.revision,
                baseline.history_generation,
                baseline.undo_len,
                baseline.next_transaction_id,
                baseline.publication_generation,
            ),
            "rejected timing Apply must not create history or publish"
        );
    }

    #[test]
    fn root_scene_block_timing_apply_is_receipted_undoable_and_fail_closed() {
        prove_root_scene_block_timing_apply(
            86_200,
            "move root Scene Block",
            |update| {
                update.time_ms = 1_500;
                update.time_beats = Some(3.0);
            },
            |before, after| {
                let root = &before.events[0];
                let moved = &after.events[0];
                assert_eq!(moved.time_ms, 1_500);
                assert_eq!(
                    (
                        moved.cue_id,
                        moved.source_offset_ms,
                        moved.track.clone(),
                        moved.layer_id,
                    ),
                    (
                        root.cue_id,
                        root.source_offset_ms,
                        root.track.clone(),
                        root.layer_id,
                    ),
                    "the timing Apply path preserves Cue/source/lane identity"
                );
            },
        );
        prove_root_scene_block_timing_apply(
            86_201,
            "RATE resize",
            |update| {
                update.duration_ms = 2_000;
                update.duration_beats = Some(4.0);
                update.conform_to_tempo = true;
                update.loop_fill = false;
            },
            |_, after| assert_eq!(after.events[0].duration_ms, 2_000),
        );
        prove_root_scene_block_timing_apply(
            86_202,
            "WINDOW resize",
            |update| {
                update.duration_ms = 3_000;
                update.duration_beats = Some(6.0);
                update.conform_to_tempo = true;
                update.loop_fill = true;
            },
            |_, after| {
                assert!(after.events[0].loop_fill);
                assert_eq!(
                    after.events[0].rate,
                    Some(1.0),
                    "WINDOW without an Effect free-run candidate uses the Engine's canonical 1x rate"
                );
            },
        );
        prove_root_scene_block_timing_apply(
            86_203,
            "Fade In",
            |update| update.fade_in_ms = 400,
            |_, after| assert_eq!(after.events[0].fade_in_ms, 400),
        );
        prove_root_scene_block_timing_apply(
            86_204,
            "Fade Out",
            |update| update.fade_out_ms = 800,
            |_, after| assert_eq!(after.events[0].fade_out_ms, 800),
        );
        prove_root_scene_block_timing_apply(
            86_205,
            "Snap Items timing",
            |update| {
                update.time_ms = 1_750;
                update.time_beats = Some(3.5);
            },
            |_, after| assert_eq!(after.events[0].time_ms, 1_750),
        );
        prove_root_scene_block_move_group();
        assert_root_scene_block_identity_rejection(
            86_206,
            |update| update.cue_id = update.cue_id.saturating_add(1),
            "Cue identity",
        );
        assert_root_scene_block_identity_rejection(
            86_207,
            |update| update.track = TimelineTrackKind::Video,
            "track identity",
        );
        assert_root_scene_block_identity_rejection(
            86_208,
            |update| update.layer_id = None,
            "layer identity",
        );
        prove_root_scene_block_unknown_event_rejection();
    }

    #[test]
    fn root_scene_block_timing_remains_editable_after_undo_redo() {
        let harness = root_scene_block_timing_harness();
        let move_before = root_scene_block_timeline(&harness);
        let moved = apply_root_scene_block_timing_authoritative(&harness, 86_220, |update| {
            update.time_ms = 1_500;
            update.time_beats = Some(3.0);
        });
        let move_after = root_scene_block_timeline(&harness);
        assert_root_scene_block_roundtrip(
            &harness,
            &move_before,
            &move_after,
            &moved.mutation.history_status,
            "move before chained RATE edit",
        );

        // No coordinator reset or fixture stabilization is allowed here. This
        // is the product regression: the next authoritative edit must commit
        // against the exact post-Redo recovery/history authority.
        let rate_before = root_scene_block_timeline(&harness);
        let rate = apply_root_scene_block_timing_authoritative(&harness, 86_221, |update| {
            update.time_ms = 1_750;
            update.time_beats = Some(3.5);
            update.duration_ms = 1_000;
            update.duration_beats = Some(2.0);
            update.conform_to_tempo = true;
            update.loop_fill = false;
        });
        let rate_after = root_scene_block_timeline(&harness);
        assert_eq!(rate_after.events[0].duration_ms, 1_000);
        assert_eq!(rate_after.events[0].rate, Some(2.0));
        assert_root_scene_block_roundtrip(
            &harness,
            &rate_before,
            &rate_after,
            &rate.mutation.history_status,
            "RATE edit after Undo/Redo",
        );

        let window_before = root_scene_block_timeline(&harness);
        let window = apply_root_scene_block_timing_authoritative(&harness, 86_222, |update| {
            update.duration_ms = 3_000;
            update.duration_beats = None;
            update.conform_to_tempo = true;
            update.loop_fill = true;
        });
        let window_after = root_scene_block_timeline(&harness);
        assert_eq!(window_after.events[0].duration_ms, 3_000);
        assert_eq!(window_after.events[0].loop_count, 2);
        assert_eq!(
            window_after.events[0].rate,
            Some(1.0),
            "no-Effect WINDOW must replace the prior RATE=2 authority"
        );
        assert_root_scene_block_roundtrip(
            &harness,
            &window_before,
            &window_after,
            &window.mutation.history_status,
            "WINDOW edit after RATE Undo/Redo",
        );

        // No stabilization is allowed after the RATE -> WINDOW hash boundary.
        // A third ordinary Apply proves that history was not silently cleared
        // by reconciliation after the canonical WINDOW rate changed.
        let final_move =
            apply_root_scene_block_timing_authoritative(&harness, 86_223, |update| {
                update.time_ms = 2_000;
                update.time_beats = Some(4.0);
            });
        let final_timeline = root_scene_block_timeline(&harness);
        assert_eq!(final_timeline.events[0].time_ms, 2_000);
        assert_eq!(final_timeline.events[0].rate, Some(1.0));
        assert_eq!(
            final_move.mutation.history_status,
            root_scene_block_history_status(&harness),
            "the Apply after RATE -> WINDOW keeps authoritative history live"
        );
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn timeline_insert_media_authoritative_uses_exact_lanes_and_groups_av_once() {
        let harness = MediaAssetA6CommandHarness::new();
        let (media_asset_id, video_lane_id, audio_lane_id, _lighting_lane_id) =
            seed_timeline_insert_media_fixture(&harness);
        let baseline = harness.mutation_baseline();
        let request = TimelineAdvancedMutationRequest::InsertMedia {
            media_asset_id,
            start_ms: 1_250,
            video_layer_id: Some(video_lane_id),
            audio_layer_id: Some(audio_lane_id),
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let applied = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request.clone(),
            86_100,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("explicit unlocked Video and Audio lanes accept linked AV placement");
        assert_eq!(applied.authoring.video_clips.len(), 1);
        assert_eq!(applied.authoring.audio_clips.len(), 1);
        let video = &applied.authoring.video_clips[0];
        let audio = &applied.authoring.audio_clips[0];
        assert_eq!(
            (video.layer_id, video.media_asset_id, video.start_ms),
            (video_lane_id, media_asset_id, 1_250)
        );
        assert_eq!(
            (audio.layer_id, audio.media_asset_id, audio.start_ms),
            (audio_lane_id, Some(media_asset_id), 1_250)
        );
        assert_eq!(applied.authoring.item_groups.len(), 1);
        assert_eq!(
            applied.authoring.item_groups[0].members,
            vec![
                TimelineItemRef::VideoClip { clip_id: video.id },
                TimelineItemRef::AudioClip { clip_id: audio.id },
            ],
            "one linked AV insertion creates one exact Video+Audio group"
        );
        assert_one_authoritative_history_mutation(&harness, baseline);
        let retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            86_100,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss retry returns the exact AV insertion");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "same InsertMedia request must not allocate or publish a second group"
        );
        let conflict_baseline = harness.mutation_baseline();
        let conflict_snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read AV insertion before shape-conflict retry");
        let conflict_hash = b3_persistence_hash(&harness);
        let shape_conflict = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::InsertMedia {
                media_asset_id,
                start_ms: 1_250,
                video_layer_id: Some(_lighting_lane_id),
                audio_layer_id: Some(audio_lane_id),
            },
            86_100,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("same request ID with a changed target lane must be a shape conflict");
        assert!(
            shape_conflict.contains("already completed with shape"),
            "unexpected InsertMedia shape-conflict error: {shape_conflict}"
        );
        let after_conflict = harness.mutation_baseline();
        assert_eq!(
            (
                after_conflict.revision,
                after_conflict.history_generation,
                after_conflict.undo_len,
                after_conflict.next_transaction_id,
                after_conflict.publication_generation,
            ),
            (
                conflict_baseline.revision,
                conflict_baseline.history_generation,
                conflict_baseline.undo_len,
                conflict_baseline.next_transaction_id,
                conflict_baseline.publication_generation,
            ),
            "shape-conflict retry must not create history or publication"
        );
        let after_conflict_snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read AV insertion after shape-conflict retry");
        assert_eq!(
            after_conflict_snapshot.timeline, conflict_snapshot.timeline,
            "shape-conflict retry must not publish a Timeline snapshot"
        );
        assert_eq!(
            b3_persistence_hash(&harness),
            conflict_hash,
            "shape-conflict retry must not change the project checkpoint"
        );
        let canonical = get_video_effect_catalog_operation_terminal_result_impl(
            &harness.state,
            86_100,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("query canonical InsertMedia receipt after shape conflict")
        .expect("canonical InsertMedia receipt remains available");
        match canonical.terminal {
            VideoEffectCatalogAuthoritativeTerminalResult::Timeline(canonical_result) => {
                assert_eq!(
                    serde_json::to_value(&canonical_result).unwrap(),
                    serde_json::to_value(&applied).unwrap(),
                    "shape conflict cannot replace the canonical Timeline receipt"
                );
            }
            other => panic!("unexpected canonical InsertMedia terminal: {other:?}"),
        }
        let first_video_id = applied.authoring.video_clips[0].id;
        let first_audio_id = applied.authoring.audio_clips[0].id;
        let first_group_id = applied.authoring.item_groups[0].id;
        let next_request = TimelineAdvancedMutationRequest::InsertMedia {
            media_asset_id,
            start_ms: 2_500,
            video_layer_id: Some(video_lane_id),
            audio_layer_id: Some(audio_lane_id),
        };
        let (next_epoch, next_revision, next_hash) = b3_authority_arguments(&harness);
        let next = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            next_request,
            86_120,
            next_epoch,
            next_revision,
            next_hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("a fresh request remains publishable after shape conflict");
        // Video, Audio, and the linking group intentionally share one
        // monotonic Timeline allocator.  A fresh AV insertion therefore
        // advances each domain by three; any shape-conflict allocation would
        // leave an extra gap here.
        assert_eq!(next.authoring.video_clips[1].id.0, first_video_id.0 + 3);
        assert_eq!(next.authoring.audio_clips[1].id, first_audio_id + 3);
        assert_eq!(next.authoring.item_groups[1].id.0, first_group_id.0 + 3);
        let after_next = harness.mutation_baseline();
        assert_eq!(after_next.revision, baseline.revision + 2);
        assert_eq!(
            after_next.history_generation,
            baseline.history_generation + 2
        );
        assert_eq!(after_next.undo_len, baseline.undo_len + 2);
        assert_eq!(
            after_next.next_transaction_id,
            baseline.next_transaction_id + 2,
            "the shape-conflict path did not consume a transaction ID"
        );
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read linked AV insertion persistence");
        assert_eq!(
            timeline_advanced_authoring_from_snapshot(&persisted.timeline),
            next.authoring,
            "engine persistence must equal the exact post-retry AV insertion candidate"
        );
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn timeline_insert_media_explicit_lane_errors_are_atomic_and_none_falls_back() {
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, _video, audio, lighting) = seed_timeline_insert_media_fixture(&harness);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 0,
                    video_layer_id: Some(lighting),
                    audio_layer_id: Some(audio),
                },
                86_101,
                "expected Video",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, _video, audio, _lighting) = seed_timeline_insert_media_fixture(&harness);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 0,
                    video_layer_id: Some(99_999_991),
                    audio_layer_id: Some(audio),
                },
                86_102,
                "was not found",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, audio, _lighting) = seed_timeline_insert_media_fixture(&harness);
            let mut locked_audio = harness
                .state
                .engine
                .persistence_snapshot()
                .expect("read lane before locking");
            locked_audio
                .timeline
                .layers
                .iter_mut()
                .find(|layer| layer.id == audio)
                .expect("Audio lane to lock")
                .locked = true;
            let locked_audio_layer = locked_audio
                .timeline
                .layers
                .into_iter()
                .find(|layer| layer.id == audio)
                .expect("locked Audio lane remains available");
            harness
                .state
                .engine
                .update_timeline_layer(locked_audio_layer)
                .expect("publish locked Audio lane for rejection test");
            c1_stabilize_fixture_authority(&harness);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 0,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(audio),
                },
                86_103,
                "is locked",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, _video, _audio, _lighting) = seed_timeline_insert_media_fixture(&harness);
            let fallback_lanes = harness
                .state
                .engine
                .persistence_snapshot()
                .expect("read fallback lanes")
                .timeline
                .layers;
            let expected_video = fallback_lanes
                .iter()
                .find(|layer| layer.kind == TimelineLayerKind::Video && !layer.locked)
                .expect("fallback Video lane")
                .id;
            let expected_audio = fallback_lanes
                .iter()
                .find(|layer| layer.kind == TimelineLayerKind::Audio && !layer.locked)
                .expect("fallback Audio lane")
                .id;
            let baseline = harness.mutation_baseline();
            let (epoch, revision, hash) = b3_authority_arguments(&harness);
            let applied = apply_timeline_advanced_authoritative_command_impl(
                &harness.state,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 400,
                    video_layer_id: None,
                    audio_layer_id: None,
                },
                86_104,
                epoch,
                revision,
                hash,
                MEDIA_ASSET_A6_OWNER.to_string(),
                None,
            )
            .expect("None lane IDs retain accessibility fallback compatibility");
            assert_eq!(applied.authoring.video_clips[0].layer_id, expected_video);
            assert_eq!(applied.authoring.audio_clips[0].layer_id, expected_audio);
            assert_one_authoritative_history_mutation(&harness, baseline);
        }
    }

    #[test]
    fn timeline_insert_media_stream_presence_requires_exact_requested_lanes() {
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, _video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, false, true);
            let (epoch, revision, hash) = b3_authority_arguments(&harness);
            let applied = apply_timeline_advanced_authoritative_command_impl(
                &harness.state,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 100,
                    video_layer_id: None,
                    audio_layer_id: Some(audio),
                },
                86_110,
                epoch,
                revision,
                hash,
                MEDIA_ASSET_A6_OWNER.to_string(),
                None,
            )
            .expect("audio-only media accepts its exact Audio lane");
            assert!(applied.authoring.video_clips.is_empty());
            assert_eq!(applied.authoring.audio_clips.len(), 1);
            assert_eq!(applied.authoring.audio_clips[0].layer_id, audio);
            assert!(applied.authoring.item_groups.is_empty());
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, false, true);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 100,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(audio),
                },
                86_111,
                "has no video stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, _video, audio, lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, false, true);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 100,
                    video_layer_id: Some(lighting),
                    audio_layer_id: Some(audio),
                },
                86_112,
                "has no video stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, _video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, false, true);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 100,
                    video_layer_id: Some(99_999_991),
                    audio_layer_id: Some(audio),
                },
                86_113,
                "has no video stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, false, true);
            let mut snapshot = harness
                .state
                .engine
                .persistence_snapshot()
                .expect("read audio-only lane before locking opposite");
            snapshot
                .timeline
                .layers
                .iter_mut()
                .find(|layer| layer.id == video)
                .expect("opposite Video lane to lock")
                .locked = true;
            let locked_video = snapshot
                .timeline
                .layers
                .into_iter()
                .find(|layer| layer.id == video)
                .expect("locked opposite Video lane remains available");
            harness
                .state
                .engine
                .update_timeline_layer(locked_video)
                .expect("publish locked opposite Video lane");
            c1_stabilize_fixture_authority(&harness);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 100,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(audio),
                },
                86_114,
                "has no video stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, true, false);
            let (epoch, revision, hash) = b3_authority_arguments(&harness);
            let applied = apply_timeline_advanced_authoritative_command_impl(
                &harness.state,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 200,
                    video_layer_id: Some(video),
                    audio_layer_id: None,
                },
                86_115,
                epoch,
                revision,
                hash,
                MEDIA_ASSET_A6_OWNER.to_string(),
                None,
            )
            .expect("video-only media accepts its exact Video lane");
            assert_eq!(applied.authoring.video_clips.len(), 1);
            assert_eq!(applied.authoring.video_clips[0].layer_id, video);
            assert!(applied.authoring.audio_clips.is_empty());
            assert!(applied.authoring.item_groups.is_empty());
            let _ = audio;
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, true, false);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 200,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(audio),
                },
                86_116,
                "has no audio stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, _audio, lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, true, false);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 200,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(lighting),
                },
                86_117,
                "has no audio stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, _audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, true, false);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 200,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(99_999_992),
                },
                86_118,
                "has no audio stream",
            );
        }
        {
            let harness = MediaAssetA6CommandHarness::new();
            let (asset, video, audio, _lighting) =
                seed_timeline_insert_media_stream_fixture(&harness, true, false);
            let mut snapshot = harness
                .state
                .engine
                .persistence_snapshot()
                .expect("read video-only lane before locking opposite");
            snapshot
                .timeline
                .layers
                .iter_mut()
                .find(|layer| layer.id == audio)
                .expect("opposite Audio lane to lock")
                .locked = true;
            let locked_audio = snapshot
                .timeline
                .layers
                .into_iter()
                .find(|layer| layer.id == audio)
                .expect("locked opposite Audio lane remains available");
            harness
                .state
                .engine
                .update_timeline_layer(locked_audio)
                .expect("publish locked opposite Audio lane");
            c1_stabilize_fixture_authority(&harness);
            assert_timeline_insert_media_rejected_without_delta(
                &harness,
                TimelineAdvancedMutationRequest::InsertMedia {
                    media_asset_id: asset,
                    start_ms: 200,
                    video_layer_id: Some(video),
                    audio_layer_id: Some(audio),
                },
                86_119,
                "has no audio stream",
            );
        }
    }

    #[test]
    fn timeline_move_items_to_lanes_authoritative_is_receipted_persistent_and_atomic() {
        let harness = MediaAssetA6CommandHarness::new();
        let (_video_layer_id, media_asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let video_source_lane_id = harness.state.engine.allocate_timeline_layer_id();
        let audio_source_lane_id = harness.state.engine.allocate_timeline_layer_id();
        let video_target_lane_id = harness.state.engine.allocate_timeline_layer_id();
        let audio_target_lane_id = harness.state.engine.allocate_timeline_layer_id();
        for (id, label, order, kind) in [
            (
                video_source_lane_id,
                "Move source video",
                1,
                TimelineLayerKind::Video,
            ),
            (
                audio_source_lane_id,
                "Move source audio",
                2,
                TimelineLayerKind::Audio,
            ),
            (
                video_target_lane_id,
                "Move target video",
                3,
                TimelineLayerKind::Video,
            ),
            (
                audio_target_lane_id,
                "Move target audio",
                4,
                TimelineLayerKind::Audio,
            ),
        ] {
            harness
                .state
                .engine
                .add_timeline_layer(protocol::TimelineLayerSummary {
                    id,
                    label: label.to_string(),
                    order,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind,
                })
                .expect("seed lane-move Timeline lane");
        }
        c1_stabilize_fixture_authority(&harness);

        let snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read lane-move authoritative seed");
        let mut authoring = timeline_advanced_authoring_from_snapshot(&snapshot.timeline);
        let video_clip_id = harness.state.engine.allocate_timeline_video_clip_id();
        let audio_clip_id = harness.state.engine.allocate_timeline_audio_clip_id();
        authoring.video_clips.push(TimelineVideoClipSummary {
            id: video_clip_id,
            layer_id: video_source_lane_id,
            media_asset_id,
            start_ms: 2_000,
            offset_ms: 0,
            duration_ms: 1_000,
            fade_in_ms: 0,
            fade_out_ms: 0,
        });
        authoring.audio_clips.push(TimelineAudioClipSummary {
            id: audio_clip_id,
            layer_id: audio_source_lane_id,
            media_asset_id: None,
            path: "authoritative-lane-move.wav".to_string(),
            start_ms: 2_100,
            offset_ms: 0,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Cue,
        });
        authoring.item_groups.push(TimelineItemGroupSummary {
            id: harness.state.engine.allocate_timeline_item_group_id(),
            members: vec![
                TimelineItemRef::VideoClip {
                    clip_id: video_clip_id,
                },
                TimelineItemRef::AudioClip {
                    clip_id: audio_clip_id,
                },
            ],
        });
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::Apply {
                authoring: Box::new(authoring),
            },
            85_100,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("seed lane-move items through the authoritative lane");
        c1_stabilize_fixture_authority(&harness);

        let baseline = harness.mutation_baseline();
        let request = TimelineAdvancedMutationRequest::MoveItemsToLanes {
            items: vec![TimelineItemRef::AudioClip {
                clip_id: audio_clip_id,
            }],
            primary: TimelineItemRef::VideoClip {
                clip_id: video_clip_id,
            },
            lane_targets: vec![
                TimelineItemLaneTarget {
                    item: TimelineItemRef::VideoClip {
                        clip_id: video_clip_id,
                    },
                    target_layer_id: video_target_lane_id,
                },
                TimelineItemLaneTarget {
                    item: TimelineItemRef::AudioClip {
                        clip_id: audio_clip_id,
                    },
                    target_layer_id: audio_target_lane_id,
                },
            ],
            delta_ms: 250,
            isolate: false,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let applied = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request.clone(),
            85_101,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("move linked clips through the terminal authoritative lane");
        let retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            85_101,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss retry reuses the lane-move terminal receipt");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "exact retry must not allocate or record a second lane move"
        );
        assert_eq!(
            applied.selected_items,
            vec![
                TimelineItemRef::VideoClip {
                    clip_id: video_clip_id,
                },
                TimelineItemRef::AudioClip {
                    clip_id: audio_clip_id,
                },
            ],
            "the terminal result selects the full moved group closure"
        );
        let video_clip = applied
            .authoring
            .video_clips
            .iter()
            .find(|clip| clip.id == video_clip_id)
            .expect("moved Video clip remains authored");
        assert_eq!(
            (video_clip.layer_id, video_clip.start_ms),
            (video_target_lane_id, 2_250)
        );
        let audio_clip = applied
            .authoring
            .audio_clips
            .iter()
            .find(|clip| clip.id == audio_clip_id)
            .expect("moved Audio clip remains authored");
        assert_eq!(
            (audio_clip.layer_id, audio_clip.start_ms),
            (audio_target_lane_id, 2_350)
        );
        assert_one_authoritative_history_mutation(&harness, baseline);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read lane-move persistence");
        assert_eq!(
            timeline_advanced_authoring_from_snapshot(&persisted.timeline),
            applied.authoring,
            "engine persistence must equal the terminal lane-move candidate B"
        );
        b3_assert_authority_matches_persistence(&harness);

        // A stale/cross-Timeline reference must fail before publication and
        // leave both the persisted A image and history untouched.
        let before_rejection = persisted;
        let rejected_baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::CreateTimeline {
                label: "Other Timeline".to_string(),
            },
            85_102,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("create another active Timeline for cross-Timeline rejection");
        let before_cross_timeline = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("capture A before cross-Timeline lane move");
        let cross_baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let error = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::MoveItemsToLanes {
                items: vec![TimelineItemRef::AudioClip {
                    clip_id: audio_clip_id,
                }],
                primary: TimelineItemRef::VideoClip {
                    clip_id: video_clip_id,
                },
                lane_targets: vec![
                    TimelineItemLaneTarget {
                        item: TimelineItemRef::VideoClip {
                            clip_id: video_clip_id,
                        },
                        target_layer_id: video_target_lane_id,
                    },
                    TimelineItemLaneTarget {
                        item: TimelineItemRef::AudioClip {
                            clip_id: audio_clip_id,
                        },
                        target_layer_id: audio_target_lane_id,
                    },
                ],
                delta_ms: 1,
                isolate: false,
            },
            85_103,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("a reference from a non-active Timeline must be rejected");
        assert!(
            error.contains("no longer exists"),
            "unexpected cross-Timeline error: {error}"
        );
        let after_cross_rejection_snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read rejected cross-Timeline persistence");
        assert_eq!(
            after_cross_rejection_snapshot.timeline, before_cross_timeline.timeline,
            "rejected cross-Timeline mutation must retain the active authored Timeline A"
        );
        assert_eq!(
            after_cross_rejection_snapshot.timeline_bank, before_cross_timeline.timeline_bank,
            "rejected cross-Timeline mutation must retain the complete authored Timeline bank A"
        );
        let after_cross_rejection = harness.mutation_baseline();
        assert_eq!(after_cross_rejection.revision, cross_baseline.revision);
        assert_eq!(
            after_cross_rejection.history_generation,
            cross_baseline.history_generation
        );
        assert_eq!(after_cross_rejection.undo_len, cross_baseline.undo_len);
        assert_eq!(
            after_cross_rejection.publication_generation, cross_baseline.publication_generation,
            "rejected cross-Timeline mutation must not create history or publish"
        );
        assert_ne!(
            before_rejection.timeline.id, before_cross_timeline.timeline.id,
            "the successful create establishes a distinct active Timeline before rejection"
        );
        assert_eq!(rejected_baseline.revision + 1, cross_baseline.revision);
    }

    #[test]
    fn timeline_move_items_to_lanes_authoritative_materializes_legacy_persistence_once() {
        let harness = MediaAssetA6CommandHarness::new();
        let (video_layer_id, _media_asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let cue_id = harness.state.engine.allocate_cue_id();
        harness
            .state
            .engine
            .create_cue_published(
                cue_id,
                protocol::DEFAULT_CUE_LIST_ID,
                "Legacy lane move cue".to_string(),
                None,
                RecallMode::Coexist,
                0,
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .expect("seed cue for legacy Timeline Scene");
        let mut loaded = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read legacy Timeline load seed");
        let event_id = harness.state.engine.allocate_timeline_event_id();
        let video_automation_id = harness.state.engine.allocate_automation_id();
        let group_id = harness.state.engine.allocate_timeline_item_group_id();
        let mut legacy = timeline_split_test_snapshot();
        legacy.id = loaded.timeline.id;
        legacy.label = loaded.timeline.label.clone();
        legacy.layers.clear();
        legacy.events.truncate(1);
        legacy.events[0].id = event_id;
        legacy.events[0].cue_id = cue_id;
        legacy.events[0].layer_id = None;
        legacy.events[0].track = TimelineTrackKind::Lighting;
        legacy.events[0].jump_to_event_id = None;
        legacy.events[0].conform_to_tempo = false;
        legacy.events[0].time_beats = None;
        legacy.events[0].duration_beats = None;
        legacy.video_clips.clear();
        legacy.audio_clips.clear();
        legacy.automations.clear();
        legacy.video_automations[0].id = video_automation_id;
        legacy.video_automations[0].layer_id = video_layer_id;
        legacy.video_automations[0].timeline_layer_id = None;
        legacy.item_groups = vec![TimelineItemGroupSummary {
            id: group_id,
            members: vec![
                TimelineItemRef::LightingEvent { event_id },
                TimelineItemRef::VideoAutomation {
                    automation_id: video_automation_id,
                },
            ],
        }];
        loaded.timeline = legacy.clone();
        loaded.timeline_bank = vec![legacy];
        harness
            .state
            .engine
            .load_project_snapshot_and_wait(loaded)
            .expect("load the historical empty-layer Timeline through EngineHandle");
        c1_stabilize_fixture_authority(&harness);
        let legacy_persistence = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read historical empty-layer persistence");
        assert!(
            legacy_persistence.timeline.layers.is_empty(),
            "the test must begin at the authored legacy empty-layer A image"
        );
        assert!(legacy_persistence.timeline.events[0].layer_id.is_none());
        assert!(legacy_persistence.timeline.video_automations[0]
            .timeline_layer_id
            .is_none());

        let baseline = harness.mutation_baseline();
        let request = TimelineAdvancedMutationRequest::MoveItemsToLanes {
            items: vec![TimelineItemRef::LightingEvent { event_id }],
            primary: TimelineItemRef::LightingEvent { event_id },
            lane_targets: vec![
                TimelineItemLaneTarget {
                    item: TimelineItemRef::LightingEvent { event_id },
                    target_layer_id: 0,
                },
                TimelineItemLaneTarget {
                    item: TimelineItemRef::VideoAutomation {
                        automation_id: video_automation_id,
                    },
                    target_layer_id: 1,
                },
            ],
            delta_ms: 100,
            isolate: false,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let applied = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request.clone(),
            85_104,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("move a historical empty-layer Timeline through the terminal lane");
        let retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            85_104,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss retry reuses the materialized legacy terminal receipt");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, baseline);
        let active = applied
            .timeline_bank
            .iter()
            .find(|timeline| timeline.id == applied.active_timeline_id)
            .expect("terminal result contains the active materialized Timeline");
        assert_eq!(
            active
                .layers
                .iter()
                .map(|layer| (layer.id, layer.kind, layer.order))
                .collect::<Vec<_>>(),
            vec![
                (0, TimelineLayerKind::Lighting, 0),
                (1, TimelineLayerKind::Video, 1),
            ]
        );
        assert_eq!(active.events[0].layer_id, Some(0));
        assert_eq!(active.video_automations[0].timeline_layer_id, Some(1));
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read materialized legacy terminal persistence");
        assert_eq!(persisted.timeline.layers, active.layers);
        assert_eq!(persisted.timeline.events[0].layer_id, Some(0));
        assert_eq!(
            persisted.timeline.video_automations[0].timeline_layer_id,
            Some(1)
        );
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn timeline_split_items_authoritative_is_persistent_receipted_and_does_not_reallocate() {
        let harness = MediaAssetA6CommandHarness::new();
        let (_video_layer_id, media_asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let video_lane_id = harness.state.engine.allocate_timeline_layer_id();
        let audio_lane_id = harness.state.engine.allocate_timeline_layer_id();
        for (id, label, order, kind) in [
            (video_lane_id, "Video", 1, TimelineLayerKind::Video),
            (audio_lane_id, "Audio", 2, TimelineLayerKind::Audio),
        ] {
            harness
                .state
                .engine
                .add_timeline_layer(protocol::TimelineLayerSummary {
                    id,
                    label: label.to_string(),
                    order,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind,
                })
                .expect("seed Split Timeline lane");
        }
        c1_stabilize_fixture_authority(&harness);
        let snapshot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read Split authoritative seed");
        let mut authoring = timeline_advanced_authoring_from_snapshot(&snapshot.timeline);
        let video_clip_id = harness.state.engine.allocate_timeline_video_clip_id();
        let audio_clip_id = harness.state.engine.allocate_timeline_audio_clip_id();
        authoring.video_clips.push(TimelineVideoClipSummary {
            id: video_clip_id,
            layer_id: video_lane_id,
            media_asset_id,
            start_ms: 2_000,
            offset_ms: 0,
            duration_ms: 1_000,
            fade_in_ms: 100,
            fade_out_ms: 100,
        });
        authoring.audio_clips.push(TimelineAudioClipSummary {
            id: audio_clip_id,
            layer_id: audio_lane_id,
            media_asset_id: None,
            path: "authoritative-split.wav".to_string(),
            start_ms: 2_100,
            offset_ms: 200,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 100,
            fade_out_ms: 100,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        });
        authoring.item_groups.push(TimelineItemGroupSummary {
            id: harness.state.engine.allocate_timeline_item_group_id(),
            members: vec![
                TimelineItemRef::VideoClip {
                    clip_id: video_clip_id,
                },
                TimelineItemRef::AudioClip {
                    clip_id: audio_clip_id,
                },
            ],
        });
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::Apply {
                authoring: Box::new(authoring),
            },
            85_001,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("seed authored Split items through the authoritative lane");
        c1_stabilize_fixture_authority(&harness);

        let baseline = harness.mutation_baseline();
        let request = TimelineAdvancedMutationRequest::SplitItems {
            items: vec![TimelineItemRef::AudioClip {
                clip_id: audio_clip_id,
            }],
            primary: TimelineItemRef::VideoClip {
                clip_id: video_clip_id,
            },
            boundary_ms: 2_400,
            isolate: false,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let applied = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request.clone(),
            85_002,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("split linked clips through the terminal authoritative lane");
        let retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            85_002,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss retry recovers the original Split terminal result");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "exact retry must reuse the recorded right-side IDs"
        );
        assert_eq!(applied.selected_items.len(), 2);
        assert_eq!(applied.authoring.item_groups.len(), 2);
        let right_video_id = applied
            .selected_items
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::VideoClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("right Video clip selection");
        let right_audio_id = applied
            .selected_items
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::AudioClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("right Audio clip selection");
        assert_ne!(right_video_id, video_clip_id);
        assert_ne!(right_audio_id, audio_clip_id);
        let right_video = applied
            .authoring
            .video_clips
            .iter()
            .find(|clip| clip.id == right_video_id)
            .unwrap();
        let right_audio = applied
            .authoring
            .audio_clips
            .iter()
            .find(|clip| clip.id == right_audio_id)
            .unwrap();
        assert_eq!(
            (
                right_video.start_ms,
                right_video.offset_ms,
                right_video.duration_ms
            ),
            (2_400, 400, 600)
        );
        assert_eq!(
            (
                right_audio.start_ms,
                right_audio.offset_ms,
                right_audio.duration_ms
            ),
            (2_500, 600, 600),
            "the non-primary clip uses the same primary-relative 400ms split offset"
        );
        assert_one_authoritative_history_mutation(&harness, baseline);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read Split persistence");
        assert_eq!(
            timeline_advanced_authoring_from_snapshot(&persisted.timeline),
            applied.authoring,
            "engine persistence must equal the exact terminal candidate B"
        );
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn timeline_copy_paste_duplicate_nudge_quantize_ripple_and_delete_are_authoritative() {
        let harness = MediaAssetA6CommandHarness::new();
        let (_video_layer_id, media_asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let video_lane_id = harness.state.engine.allocate_timeline_layer_id();
        harness
            .state
            .engine
            .add_timeline_layer(protocol::TimelineLayerSummary {
                id: video_lane_id,
                label: "Video".to_string(),
                order: 1,
                muted: false,
                locked: false,
                solo: false,
                expanded: false,
                kind: TimelineLayerKind::Video,
            })
            .expect("seed a Video Timeline lane");
        let audio_lane_id = harness.state.engine.allocate_timeline_layer_id();
        harness
            .state
            .engine
            .add_timeline_layer(protocol::TimelineLayerSummary {
                id: audio_lane_id,
                label: "Audio".to_string(),
                order: 2,
                muted: false,
                locked: false,
                solo: false,
                expanded: false,
                kind: TimelineLayerKind::Audio,
            })
            .expect("seed an Audio Timeline lane");
        c1_stabilize_fixture_authority(&harness);

        let project = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read Timeline delete seed project");
        let timeline = &project.timeline;
        let video_clip_id = protocol::TimelineVideoClipId(9_101);
        let audio_clip_id = 9_102;
        let mut authoring = timeline_advanced_authoring_from_snapshot(timeline);
        authoring.video_clips.push(TimelineVideoClipSummary {
            id: video_clip_id,
            layer_id: video_lane_id,
            media_asset_id,
            start_ms: 2_000,
            offset_ms: 0,
            duration_ms: 1_000,
            fade_in_ms: 0,
            fade_out_ms: 0,
        });
        authoring.audio_clips.push(TimelineAudioClipSummary {
            id: audio_clip_id,
            layer_id: audio_lane_id,
            media_asset_id: None,
            path: "linked.wav".to_string(),
            start_ms: 2_000,
            offset_ms: 0,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Cue,
        });
        authoring.item_groups.push(TimelineItemGroupSummary {
            id: TimelineItemGroupId(9_103),
            members: vec![
                TimelineItemRef::VideoClip {
                    clip_id: video_clip_id,
                },
                TimelineItemRef::AudioClip {
                    clip_id: audio_clip_id,
                },
            ],
        });
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::Apply {
                authoring: Box::new(authoring),
            },
            84_680,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("seed linked A/V items through the authoritative lane");
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let duplicate_baseline = harness.mutation_baseline();
        let duplicate_request = TimelineAdvancedMutationRequest::DuplicateItems {
            items: vec![TimelineItemRef::AudioClip {
                clip_id: audio_clip_id,
            }],
            offset_ms: 500,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let duplicated = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            duplicate_request.clone(),
            84_681,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("duplicate one grouped member through the authoritative lane");
        assert_eq!(duplicated.authoring.video_clips.len(), 2);
        assert_eq!(duplicated.authoring.audio_clips.len(), 2);
        assert_eq!(duplicated.authoring.item_groups.len(), 2);
        assert_eq!(duplicated.selected_items.len(), 2);
        let duplicate_audio_clip_id = duplicated
            .selected_items
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::AudioClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("duplicate result selects the fresh Audio peer");
        assert_ne!(duplicate_audio_clip_id, audio_clip_id);
        assert_eq!(
            duplicated
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == duplicate_audio_clip_id)
                .unwrap()
                .start_ms,
            2_500
        );
        assert_eq!(
            duplicated
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == duplicate_audio_clip_id)
                .unwrap()
                .output_bus,
            protocol::TimelineAudioOutputBus::Cue,
            "duplicate preserves the exact logical bus"
        );

        let duplicate_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            duplicate_request,
            84_681,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked duplicate terminal receipt");
        assert_eq!(
            serde_json::to_value(&duplicated).unwrap(),
            serde_json::to_value(&duplicate_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, duplicate_baseline);
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let nudge_baseline = harness.mutation_baseline();
        let nudge_request = TimelineAdvancedMutationRequest::NudgeItems {
            items: vec![TimelineItemRef::AudioClip {
                clip_id: duplicate_audio_clip_id,
            }],
            delta_ms: -250,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let nudged = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            nudge_request.clone(),
            84_682,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("nudge one duplicated member through the authoritative lane");
        assert_eq!(nudged.selected_items, duplicated.selected_items);
        let duplicate_video_clip_id = duplicated
            .selected_items
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::VideoClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("duplicate result selects the fresh Video peer");
        assert_eq!(
            nudged
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == duplicate_audio_clip_id)
                .unwrap()
                .start_ms,
            2_250
        );
        assert_eq!(
            nudged
                .authoring
                .video_clips
                .iter()
                .find(|clip| clip.id == duplicate_video_clip_id)
                .unwrap()
                .start_ms,
            2_250
        );
        let nudge_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            nudge_request,
            84_682,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked nudge terminal receipt");
        assert_eq!(
            serde_json::to_value(&nudged).unwrap(),
            serde_json::to_value(&nudge_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, nudge_baseline);
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let quantize_baseline = harness.mutation_baseline();
        let quantize_request = TimelineAdvancedMutationRequest::QuantizeItems {
            items: vec![TimelineItemRef::AudioClip {
                clip_id: duplicate_audio_clip_id,
            }],
            grid_ms: 1_000,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let quantized = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            quantize_request.clone(),
            84_683,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("quantize one duplicated member through the authoritative lane");
        assert_eq!(quantized.selected_items, duplicated.selected_items);
        assert_eq!(
            quantized
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == duplicate_audio_clip_id)
                .unwrap()
                .start_ms,
            2_000
        );
        assert_eq!(
            quantized
                .authoring
                .video_clips
                .iter()
                .find(|clip| clip.id == duplicate_video_clip_id)
                .unwrap()
                .start_ms,
            2_000
        );
        let quantize_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            quantize_request,
            84_683,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked quantize terminal receipt");
        assert_eq!(
            serde_json::to_value(&quantized).unwrap(),
            serde_json::to_value(&quantize_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, quantize_baseline);
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let paste_baseline = harness.mutation_baseline();
        let paste_request = TimelineAdvancedMutationRequest::PasteItems {
            items: duplicated.selected_items.clone(),
            target_ms: 500,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let pasted = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            paste_request.clone(),
            84_684,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("paste the copied linked group at the playhead through the authoritative lane");
        assert_eq!(pasted.authoring.video_clips.len(), 3);
        assert_eq!(pasted.authoring.audio_clips.len(), 3);
        assert_eq!(pasted.authoring.item_groups.len(), 3);
        assert_eq!(pasted.selected_items.len(), 2);
        let pasted_audio_clip_id = pasted
            .selected_items
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::AudioClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("paste result selects the fresh Audio peer");
        let pasted_video_clip_id = pasted
            .selected_items
            .iter()
            .find_map(|item| match item {
                TimelineItemRef::VideoClip { clip_id } => Some(*clip_id),
                _ => None,
            })
            .expect("paste result selects the fresh Video peer");
        assert!(!duplicated
            .selected_items
            .contains(&TimelineItemRef::AudioClip {
                clip_id: pasted_audio_clip_id,
            }));
        assert_eq!(
            pasted
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == pasted_audio_clip_id)
                .unwrap()
                .start_ms,
            500
        );
        assert_eq!(
            pasted
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == pasted_audio_clip_id)
                .unwrap()
                .output_bus,
            protocol::TimelineAudioOutputBus::Cue,
            "paste preserves the exact logical bus"
        );
        assert_eq!(
            pasted
                .authoring
                .video_clips
                .iter()
                .find(|clip| clip.id == pasted_video_clip_id)
                .unwrap()
                .start_ms,
            500
        );
        let paste_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            paste_request,
            84_684,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked paste terminal receipt");
        assert_eq!(
            serde_json::to_value(&pasted).unwrap(),
            serde_json::to_value(&paste_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, paste_baseline);
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let ripple_baseline = harness.mutation_baseline();
        let ripple_request = TimelineAdvancedMutationRequest::RippleItems {
            items: vec![TimelineItemRef::AudioClip {
                clip_id: pasted_audio_clip_id,
            }],
            delta_ms: 250,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let rippled = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            ripple_request.clone(),
            84_685,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("ripple the pasted group and every following authored item");
        assert_eq!(rippled.selected_items, pasted.selected_items);
        assert_eq!(
            rippled
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == pasted_audio_clip_id)
                .unwrap()
                .start_ms,
            750
        );
        assert!(rippled
            .authoring
            .audio_clips
            .iter()
            .filter(|clip| clip.id != pasted_audio_clip_id)
            .all(|clip| clip.start_ms == 2_250));
        let ripple_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            ripple_request,
            84_685,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked ripple terminal receipt");
        assert_eq!(
            serde_json::to_value(&rippled).unwrap(),
            serde_json::to_value(&ripple_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, ripple_baseline);
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let trim_baseline = harness.mutation_baseline();
        let trim_request = TimelineAdvancedMutationRequest::TrimItems {
            items: vec![
                TimelineItemRef::AudioClip {
                    clip_id: pasted_audio_clip_id,
                },
                TimelineItemRef::VideoClip {
                    clip_id: pasted_video_clip_id,
                },
            ],
            primary: TimelineItemRef::VideoClip {
                clip_id: pasted_video_clip_id,
            },
            edge: TimelineTrimEdge::Start,
            boundary_ms: 1_000,
            isolate: false,
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let trimmed = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            trim_request.clone(),
            84_686,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("trim from a non-first primary as the complete linked group");
        assert_eq!(trimmed.selected_items, pasted.selected_items);
        {
            let clip_id = pasted_audio_clip_id;
            let clip = trimmed
                .authoring
                .audio_clips
                .iter()
                .find(|clip| clip.id == clip_id)
                .unwrap();
            assert_eq!(
                (clip.start_ms, clip.offset_ms, clip.duration_ms),
                (1_000, 250, 750)
            );
            assert_eq!(clip.output_bus, protocol::TimelineAudioOutputBus::Cue);
        }
        let clip = trimmed
            .authoring
            .video_clips
            .iter()
            .find(|clip| clip.id == pasted_video_clip_id)
            .unwrap();
        assert_eq!(
            (clip.start_ms, clip.offset_ms, clip.duration_ms),
            (1_000, 250, 750)
        );
        let trim_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            trim_request,
            84_686,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked trim terminal receipt");
        assert_eq!(
            serde_json::to_value(&trimmed).unwrap(),
            serde_json::to_value(&trim_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, trim_baseline);
        b3_assert_authority_matches_persistence(&harness);
        c1_stabilize_fixture_authority(&harness);

        let delete_baseline = harness.mutation_baseline();
        let delete_request = TimelineAdvancedMutationRequest::DeleteItems {
            items: vec![
                TimelineItemRef::AudioClip {
                    clip_id: duplicate_audio_clip_id,
                },
                TimelineItemRef::AudioClip {
                    clip_id: pasted_audio_clip_id,
                },
            ],
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let deleted = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            delete_request.clone(),
            84_687,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("delete the duplicated group by one member");
        assert_eq!(deleted.authoring.video_clips.len(), 1);
        assert_eq!(deleted.authoring.audio_clips.len(), 1);
        assert_eq!(deleted.authoring.item_groups.len(), 1);
        assert_eq!(deleted.authoring.video_clips[0].id, video_clip_id);
        assert_eq!(deleted.authoring.audio_clips[0].id, audio_clip_id);
        let delete_retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            delete_request,
            84_687,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact linked delete terminal receipt");
        assert_eq!(
            serde_json::to_value(&deleted).unwrap(),
            serde_json::to_value(&delete_retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, delete_baseline);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read linked delete persistence");
        assert_eq!(persisted.timeline.video_clips.len(), 1);
        assert_eq!(persisted.timeline.audio_clips.len(), 1);
        assert_eq!(persisted.timeline.item_groups.len(), 1);
        assert_eq!(
            persisted.timeline.audio_clips[0].output_bus,
            protocol::TimelineAudioOutputBus::Cue,
            "the project save image retains CUE after duplicate/paste/trim"
        );
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn timeline_advanced_authoritative_bank_mutations_are_ordered_recoverable_and_history_safe() {
        let harness = MediaAssetA6CommandHarness::new();
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let request = TimelineAdvancedMutationRequest::CreateTimeline {
            label: "Verse".to_string(),
        };
        let created = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request.clone(),
            84_701,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("create a second Timeline through the authoritative bank lane");
        assert_eq!(created.timeline_bank.len(), 2);
        assert_eq!(created.active_timeline_id, created.timeline_bank[1].id);
        assert_eq!(created.timeline_bank[1].label, "Verse");
        let retried = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            request,
            84_701,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("recover the exact Timeline create receipt without allocating again");
        assert_eq!(
            serde_json::to_value(&created).unwrap(),
            serde_json::to_value(&retried).unwrap()
        );
        assert_one_authoritative_history_mutation(&harness, baseline);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let duplicated = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::DuplicateTimeline {
                timeline_id: created.active_timeline_id,
            },
            84_702,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("duplicate an authored Timeline with a fresh stable ID");
        assert_eq!(duplicated.timeline_bank.len(), 3);
        assert_ne!(duplicated.active_timeline_id, created.active_timeline_id);

        let reversed_ids = duplicated
            .timeline_bank
            .iter()
            .rev()
            .map(|timeline| timeline.id)
            .collect::<Vec<_>>();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let reordered = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::ReorderTimelines {
                timeline_ids: reversed_ids.clone(),
            },
            84_703,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reorder the complete bank atomically");
        assert_eq!(
            reordered
                .timeline_bank
                .iter()
                .map(|timeline| timeline.id)
                .collect::<Vec<_>>(),
            reversed_ids
        );

        let remove_id = reordered.timeline_bank[1].id;
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let removed = apply_timeline_advanced_authoritative_command_impl(
            &harness.state,
            TimelineAdvancedMutationRequest::RemoveTimeline {
                timeline_id: remove_id,
            },
            84_704,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("remove one non-final Timeline atomically");
        assert_eq!(removed.timeline_bank.len(), 2);
        assert!(removed
            .timeline_bank
            .iter()
            .all(|timeline| timeline.id != remove_id));
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read final Timeline bank persistence");
        assert_eq!(persisted.timeline_bank, removed.timeline_bank);
        protocol::validate_timeline_bank(&persisted, &persisted.video.media_assets).unwrap();
    }

    #[test]
    fn video_effect_catalog_authoritative_rejects_stale_window_owner_and_invalid_candidate_without_mutation(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let request = c1_layer_catalog_request(&harness, layer_id, 0.0);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let baseline = harness.mutation_baseline();
        let wrong_window = VideoClipSlotCallerBinding {
            window_label: "other-c1-window".to_string(),
            owner_id: MEDIA_ASSET_A6_OWNER.to_string(),
            incarnation: 1,
        };
        let error = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request.clone(),
            84_011,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(wrong_window),
        )
        .expect_err("unregistered WebView must not borrow the owner binding");
        assert!(error.contains("renderer incarnation changed"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            0
        );
        assert_eq!(harness.mutation_baseline().revision, baseline.revision);

        let invalid_request = c1_layer_catalog_request(&harness, u64::MAX, 0.0);
        let error = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            invalid_request,
            84_012,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("invalid catalog must fail before one Published ACK");
        assert!(error.contains("unknown") || error.contains("missing"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            0
        );
        let after = harness.mutation_baseline();
        assert_eq!(after.revision, baseline.revision);
        assert_eq!(after.history_generation, baseline.history_generation);
        assert_eq!(after.undo_len, baseline.undo_len);
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn video_effect_catalog_authoritative_accepts_five_scopes_preset_and_group_in_one_history_entry(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let authored = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("C1 five-scope persistence")
            .authored_video
            .expect("C1 five-scope authored video");
        let composition_id = authored
            .compositions
            .iter()
            .find(|composition| composition.layer_ids.contains(&layer_id))
            .expect("seed layer belongs to a composition")
            .id;
        let group_id = protocol::VideoLayerGroupId(0);
        let request = VideoEffectCatalogApplyRequest {
            effect_chains: vec![
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Layer { layer_id },
                    "Layer",
                    c1_test_event_effect(0.0),
                ),
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Clip { layer_id, slot_id },
                    "Clip",
                    c1_test_event_effect(0.0),
                ),
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Transition {
                        owner: protocol::VideoTransitionEffectOwner::ClipTake { layer_id },
                    },
                    "Clip take",
                    c1_test_event_effect(0.0),
                ),
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Composition { composition_id },
                    "Composition",
                    c1_test_event_effect(0.0),
                ),
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Group { group_id },
                    "Group",
                    c1_test_event_effect(0.0),
                ),
            ],
            effect_presets: vec![protocol::VideoEffectPresetSummary {
                id: protocol::VideoEffectPresetId(0),
                label: "C1 immutable preset".to_string(),
                payload: protocol::VideoEffectPresetPayload {
                    bypassed: false,
                    stages: vec![protocol::VideoEffectPresetStagePayload {
                        enabled: true,
                        label: "Preset stage".to_string(),
                        effect: VideoEffectKind::Isf {
                            effect: c1_test_event_effect(0.0),
                        },
                    }],
                },
            }],
            layer_groups: vec![VideoLayerGroupSummary {
                id: group_id,
                label: "C1 group".to_string(),
                composition_id,
                layer_ids: vec![layer_id],
            }],
            transition_buses: Vec::new(),
        };
        let baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let result = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request,
            84_021,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("all C1 catalog scopes, preset, and group commit atomically");
        c1_assert_one_authored_mutation(&harness, baseline);
        assert_eq!(result.catalog.effect_chains.len(), 5);
        assert_eq!(result.catalog.effect_presets.len(), 1);
        assert_eq!(result.catalog.layer_groups.len(), 1);
        let allocated_group_id = result.catalog.layer_groups[0].id;
        assert_ne!(allocated_group_id.0, 0);
        assert_ne!(result.catalog.effect_presets[0].id.0, 0);
        assert!(result.catalog.effect_chains.iter().all(|chain| {
            chain.id.0 != 0
                && chain
                    .stages
                    .iter()
                    .all(|stage| stage.id.0 != 0 && stage.effect.id.0 != 0)
        }));
        assert!(result.catalog.effect_chains.iter().any(|chain| {
            chain.scope
                == VideoEffectScope::Group {
                    group_id: allocated_group_id,
                }
        }));
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("persisted five-scope catalog")
            .authored_video
            .expect("persisted five-scope authored video");
        assert_eq!(persisted.effect_chains, result.catalog.effect_chains);
        assert_eq!(persisted.effect_presets, result.catalog.effect_presets);
        assert_eq!(persisted.layer_groups, result.catalog.layer_groups);
    }

    #[test]
    fn video_effect_catalog_authoritative_receipts_are_bound_to_the_original_window_incarnation() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let request = c1_layer_catalog_request(&harness, layer_id, 0.0);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let old_binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture original C1 owner incarnation");
        apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request,
            84_031,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(old_binding.clone()),
        )
        .expect("create C1 receipt owned by the original window");
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("retire C1 owner")
            .remove("media-asset-a6");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("retire C1 owner incarnation")
            .remove("media-asset-a6");
        harness
            .state
            .media_asset_operations
            .purge_video_effect_catalog_authoritative_for_owner(MEDIA_ASSET_A6_OWNER);
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("re-register C1 owner string")
            .insert(
                "media-asset-a6".to_string(),
                MEDIA_ASSET_A6_OWNER.to_string(),
            );
        let new_incarnation = allocate_project_transaction_owner_incarnation(&harness.state)
            .expect("allocate C1 replacement incarnation");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("register C1 replacement incarnation")
            .insert("media-asset-a6".to_string(), new_incarnation);
        let error = get_video_effect_catalog_operation_terminal_result_impl(
            &harness.state,
            84_031,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(old_binding),
        )
        .expect_err("ABA owner reuse cannot recover the original C1 receipt");
        assert!(error.contains("renderer incarnation changed"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            1,
            "an old terminal receipt cannot republish under a replacement window"
        );
    }

    #[test]
    fn video_effect_catalog_authoritative_operator_locks_block_new_catalog_mutations() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let request = c1_layer_catalog_request(&harness, layer_id, 0.0);
        let mut partial_policy = sample_operator_policy();
        partial_policy.lock_mode = OperatorLockMode::Partial;
        {
            let mut coordinator = harness
                .state
                .project_coordinator
                .lock()
                .expect("install C1 Partial lock policy");
            coordinator.ancillary.operator_policy = Some(partial_policy.clone());
            harness
                .state
                .project_operator_sessions
                .lock()
                .expect("install locked C1 Partial session")
                .insert(
                    MEDIA_ASSET_A6_OWNER.to_string(),
                    ProjectOperatorSession {
                        project_epoch: coordinator.epoch,
                        policy: partial_policy,
                        unlocked: false,
                    },
                );
        }
        c1_stabilize_fixture_authority(&harness);
        let partial_baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let error = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request.clone(),
            84_041,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("Partial operator lock blocks C1 authoring");
        assert!(error.contains("Partial Lock"));
        let after_partial = harness.mutation_baseline();
        assert_eq!(after_partial.revision, partial_baseline.revision);
        assert_eq!(
            after_partial.history_generation,
            partial_baseline.history_generation
        );
        assert_eq!(after_partial.undo_len, partial_baseline.undo_len);
        let mut full_policy = sample_operator_policy();
        full_policy.lock_mode = OperatorLockMode::Full;
        {
            let mut coordinator = harness
                .state
                .project_coordinator
                .lock()
                .expect("replace C1 coordinator policy with Full");
            coordinator.ancillary.operator_policy = Some(full_policy.clone());
            harness
                .state
                .project_operator_sessions
                .lock()
                .expect("replace C1 lock session with Full")
                .get_mut(MEDIA_ASSET_A6_OWNER)
                .expect("C1 partial session exists")
                .policy = full_policy;
        }
        c1_stabilize_fixture_authority(&harness);
        let full_baseline = harness.mutation_baseline();
        let (full_epoch, full_revision, full_hash) = b3_authority_arguments(&harness);
        let error = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request,
            84_042,
            full_epoch,
            full_revision,
            full_hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("Full operator lock blocks C1 authoring");
        assert!(error.contains("Full Lock"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            0
        );
        let after = harness.mutation_baseline();
        assert_eq!(after.revision, full_baseline.revision);
        assert_eq!(after.history_generation, full_baseline.history_generation);
        assert_eq!(after.undo_len, full_baseline.undo_len);
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_isf_adapter_projects_canonical_history_once() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(1.0)),
                )
            },
            || {},
        )
        .expect("legacy adapter captures A before constructing its C1 candidate");
        c1_assert_one_authored_mutation(&harness, baseline);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("legacy adapter persistence")
            .authored_video
            .expect("legacy adapter authored video");
        let chain = persisted
            .effect_chains
            .iter()
            .find(|chain| chain.scope == VideoEffectScope::Layer { layer_id })
            .expect("legacy adapter persists canonical Layer chain");
        assert_eq!(chain.stages.len(), 1);
        let VideoEffectKind::Isf { effect } = &chain.stages[0].effect.kind;
        assert_eq!(
            effect
                .controls
                .iter()
                .find(|control| control.kind == VideoIsfControlKind::Event)
                .expect("canonical Event control")
                .value[0],
            0.0,
            "legacy ingress may never persist Event=1"
        );
        let legacy = persisted
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .expect("legacy adapter layer")
            .isf_effect
            .as_ref()
            .expect("legacy projection remains available");
        assert_eq!(legacy.label, effect.label);
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_adapter_same_layer_race_rejects_without_overwrite()
    {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        let binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture legacy adapter caller binding");
        let concurrent_state = Arc::clone(&harness.state);
        let concurrent_request = c1_layer_catalog_request(&harness, layer_id, 0.0);

        let error = mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding),
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(1.0)),
                )
            },
            move || {
                // Pause after A, publish another Layer chain, then resume the
                // stale adapter candidate. It must fail instead of restoring
                // its old whole-catalog image over this newer chain.
                let authority = {
                    let coordinator = concurrent_state
                        .project_coordinator
                        .lock()
                        .expect("read concurrent C1 authority");
                    MediaAssetPrepareAuthority {
                        epoch: coordinator.epoch,
                        revision: coordinator.revision,
                        checkpoint_hash: coordinator.checkpoint_hash.clone(),
                    }
                };
                apply_video_effect_catalog_authoritative_command_impl(
                    &concurrent_state,
                    concurrent_request,
                    84_061,
                    authority.epoch,
                    authority.revision,
                    authority.checkpoint_hash,
                    MEDIA_ASSET_A6_OWNER.to_string(),
                    None,
                )
                .expect("concurrent same-layer C1 catalog commits while adapter is paused");
            },
        )
        .expect_err("stale legacy catalog must not overwrite same-layer C1 chain");
        assert!(
            error.contains("Project changed")
                || error.contains("revision")
                || error.contains("checkpoint")
                || error.contains("epoch"),
            "stale A must fail as an authority conflict: {error}"
        );
        c1_assert_one_authored_mutation(&harness, baseline);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("same-layer race persistence")
            .authored_video
            .expect("same-layer race authored video");
        let chain = persisted
            .effect_chains
            .iter()
            .find(|chain| chain.scope == VideoEffectScope::Layer { layer_id })
            .expect("concurrent same-layer chain survives stale adapter");
        assert_eq!(chain.stages[0].label, "C1 layer stage");

        mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(1.0)),
                )
            },
            || {},
        )
        .expect("fresh legacy retry replaces the current same-layer chain once");
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            2,
            "the failed stale adapter reaches no second Published command"
        );
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("same-layer retry persistence")
            .authored_video
            .expect("same-layer retry authored video");
        let chain = persisted
            .effect_chains
            .iter()
            .find(|chain| chain.scope == VideoEffectScope::Layer { layer_id })
            .expect("fresh legacy retry replaces same-layer chain");
        assert_eq!(chain.stages[0].label, "C1 event seam");
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_adapter_different_layer_race_preserves_both_on_retry(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let (legacy_layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let (concurrent_layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        let concurrent_state = Arc::clone(&harness.state);
        let concurrent_request = c1_layer_catalog_request(&harness, concurrent_layer_id, 0.0);

        let error = mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    legacy_layer_id,
                    Some(c1_test_event_effect(1.0)),
                )
            },
            move || {
                let authority = {
                    let coordinator = concurrent_state
                        .project_coordinator
                        .lock()
                        .expect("read different-layer C1 authority");
                    MediaAssetPrepareAuthority {
                        epoch: coordinator.epoch,
                        revision: coordinator.revision,
                        checkpoint_hash: coordinator.checkpoint_hash.clone(),
                    }
                };
                apply_video_effect_catalog_authoritative_command_impl(
                    &concurrent_state,
                    concurrent_request,
                    84_062,
                    authority.epoch,
                    authority.revision,
                    authority.checkpoint_hash,
                    MEDIA_ASSET_A6_OWNER.to_string(),
                    None,
                )
                .expect("concurrent different-layer C1 catalog commits while adapter is paused");
            },
        )
        .expect_err("stale legacy catalog must not erase a different-layer chain");
        assert!(
            error.contains("Project changed")
                || error.contains("revision")
                || error.contains("checkpoint")
                || error.contains("epoch")
        );
        c1_assert_one_authored_mutation(&harness, baseline);

        mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    legacy_layer_id,
                    Some(c1_test_event_effect(1.0)),
                )
            },
            || {},
        )
        .expect("fresh legacy retry keeps the concurrent different-layer chain");
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("different-layer retry persistence")
            .authored_video
            .expect("different-layer retry authored video");
        assert!(persisted.effect_chains.iter().any(|chain| chain.scope
            == VideoEffectScope::Layer {
                layer_id: legacy_layer_id
            }));
        assert!(persisted.effect_chains.iter().any(|chain| {
            chain.scope
                == VideoEffectScope::Layer {
                    layer_id: concurrent_layer_id,
                }
        }));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            2,
            "only the concurrent catalog and the fresh retry publish"
        );
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_adapter_rejects_a_rotated_window_after_a_capture()
    {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        let binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture original legacy adapter incarnation");
        let rotated_state = Arc::clone(&harness.state);

        let error = mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding),
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(1.0)),
                )
            },
            move || {
                let _rotation = rotated_state
                    .project_transaction_owner_rotation
                    .lock()
                    .expect("lock legacy owner rotation");
                let incarnation = allocate_project_transaction_owner_incarnation(&rotated_state)
                    .expect("allocate replacement legacy renderer incarnation");
                rotated_state
                    .project_transaction_owner_incarnations
                    .lock()
                    .expect("rotate legacy renderer incarnation")
                    .insert("media-asset-a6".to_string(), incarnation);
            },
        )
        .expect_err("legacy adapter must reject its A image after WebView rotation");
        assert!(error.contains("renderer incarnation changed"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            0,
            "a rotated caller cannot enqueue an authoritative catalog publish"
        );
        let after = harness.mutation_baseline();
        assert_eq!(after.revision, baseline.revision);
        assert_eq!(after.history_generation, baseline.history_generation);
        assert_eq!(after.undo_len, baseline.undo_len);
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn video_effect_catalog_authoritative_zero_ids_allocate_distinct_domains_and_reject_new_nonzero(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let (first_layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let (second_layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let baseline = harness.mutation_baseline();
        let mut forbidden = c1_layer_catalog_request(&harness, first_layer_id, 0.0);
        forbidden.effect_chains[0].id = protocol::VideoEffectChainId(9_999_991);
        let error = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            forbidden,
            84_071,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("external callers cannot mint arbitrary nonzero C1 identities");
        assert!(error.contains("use ID 0 for a new entity"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            0
        );
        assert_eq!(harness.mutation_baseline().revision, baseline.revision);

        let request = VideoEffectCatalogApplyRequest {
            effect_chains: vec![
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Layer {
                        layer_id: first_layer_id,
                    },
                    "first zero chain",
                    c1_test_event_effect(0.0),
                ),
                c1_catalog_chain(
                    &harness,
                    VideoEffectScope::Layer {
                        layer_id: second_layer_id,
                    },
                    "second zero chain",
                    c1_test_event_effect(0.0),
                ),
            ],
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
        };
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let applied = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request,
            84_072,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("two zero chain/stage/effect triples allocate independently");
        assert_ne!(
            applied.catalog.effect_chains[0].id,
            applied.catalog.effect_chains[1].id
        );
        assert_ne!(
            applied.catalog.effect_chains[0].stages[0].id,
            applied.catalog.effect_chains[1].stages[0].id
        );
        assert_ne!(
            applied.catalog.effect_chains[0].stages[0].effect.id,
            applied.catalog.effect_chains[1].stages[0].effect.id
        );
        c1_assert_one_authored_mutation(&harness, baseline);
    }

    #[test]
    fn video_effect_catalog_authoritative_zero_allocator_fails_partial_request_without_mutating_input(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        let authored_video = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("capture zero allocator A")
            .authored_video
            .expect("fixture has authored video");
        let request = c1_layer_catalog_request(&harness, layer_id, 0.0);
        let allocated_chain = std::cell::Cell::new(false);
        let error = normalize_video_effect_catalog_ids_with_allocator(
            &authored_video,
            &request,
            (
                || {
                    allocated_chain.set(true);
                    Ok(protocol::VideoEffectChainId(700_001))
                },
                || Err("synthetic Video effect stage allocator exhausted".to_string()),
                || Ok(protocol::VideoEffectId(700_001)),
                || Ok(protocol::VideoEffectPresetId(700_001)),
                || Ok(protocol::VideoLayerGroupId(700_001)),
                || Ok(protocol::VideoTransitionBusId(700_001)),
            ),
        )
        .expect_err("allocation overflow after a prior domain allocation fails closed");
        assert!(allocated_chain.get());
        assert!(error.contains("allocator exhausted"));
        assert_eq!(request.effect_chains[0].id.0, 0);
        assert_eq!(request.effect_chains[0].stages[0].id.0, 0);
        assert_eq!(request.effect_chains[0].stages[0].effect.id.0, 0);
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            0,
            "a partial backend allocation reaches neither Published nor history"
        );
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_readd_after_removal_never_reuses_retired_ids() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let initial = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            c1_layer_catalog_request(&harness, layer_id, 0.0),
            84_081,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("allocate initial C1 chain through the real EngineHandle");
        let retired_chain_id = initial.catalog.effect_chains[0].id;
        let retired_stage_id = initial.catalog.effect_chains[0].stages[0].id;
        let retired_effect_id = initial.catalog.effect_chains[0].stages[0].effect.id;

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            VideoEffectCatalogApplyRequest {
                effect_chains: Vec::new(),
                effect_presets: Vec::new(),
                layer_groups: Vec::new(),
                transition_buses: Vec::new(),
            },
            84_082,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("remove the complete canonical C1 catalog");
        c1_stabilize_fixture_authority(&harness);

        mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(0.0)),
                )
            },
            || {},
        )
        .expect("legacy re-add uses the same real EngineHandle allocator lane");
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read legacy re-add persistence")
            .authored_video
            .expect("read legacy re-add authored video");
        let readded = persisted
            .effect_chains
            .iter()
            .find(|chain| chain.scope == VideoEffectScope::Layer { layer_id })
            .expect("legacy re-add persists one Layer chain");
        assert!(readded.id.0 > retired_chain_id.0);
        assert!(readded.stages[0].id.0 > retired_stage_id.0);
        assert!(readded.stages[0].effect.id.0 > retired_effect_id.0);
        b3_assert_authority_matches_persistence(&harness);
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_response_keeps_effect_shape_and_nested_mutation() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let (effect, mutation) = mutate_legacy_video_effect_catalog_authoritatively(
            &harness.state,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(0.0)),
                )?;
                Ok(c1_test_event_effect(0.0))
            },
            || {},
        )
        .expect("legacy mutation produces the authoritative history result");
        let value =
            serde_json::to_value(LegacyVideoIsfEffectAuthoritativeResult { effect, mutation })
                .expect("serialize legacy effect response");
        assert!(
            value.get("label").is_some(),
            "old effect fields stay top-level"
        );
        assert!(
            value
                .get("mutation")
                .and_then(|mutation| mutation.get("authority"))
                .is_some(),
            "server-authoritative frontend routing can advance E/R/H from every effect response"
        );
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_terminal_retry_recovers_one_ack_and_rejects_aba() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture legacy terminal caller binding");
        let effect = c1_test_event_effect(0.0);
        let shape = legacy_video_effect_catalog_authoritative_shape(
            "set_video_layer_isf_effect",
            json!({ "layer_id": layer_id, "effect": effect }),
        )
        .expect("encode legacy terminal intent shape");
        let applied = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_091,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
            shape.clone(),
            move |video, request| {
                legacy_replace_layer_effect_chain(
                    video,
                    request,
                    layer_id,
                    Some(c1_test_event_effect(0.0)),
                )
            },
        )
        .expect("the legacy terminal lane publishes exactly one canonical catalog");
        let retried = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_091,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
            shape.clone(),
            |_video, _request| -> Result<(), String> {
                panic!("exact legacy retry must return its receipt before rebuilding A")
            },
        )
        .expect("reply-loss retry recovers the original legacy terminal result");
        assert_eq!(
            serde_json::to_value(&applied).unwrap(),
            serde_json::to_value(&retried).unwrap(),
            "normal and recovered legacy catalog DTOs are byte-identical"
        );
        c1_assert_one_authored_mutation(&harness, baseline);
        let recovered = get_video_effect_catalog_operation_terminal_result_impl(
            &harness.state,
            84_091,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
        )
        .expect("legacy terminal query succeeds")
        .expect("legacy terminal receipt exists");
        let recovered_json = serde_json::to_value(&recovered).expect("serialize legacy receipt");
        assert!(
            recovered_json
                .pointer("/terminal/result/mutation/authority")
                .is_some(),
            "recovered catalog receipt exposes the same authoritative E/R/H mutation"
        );

        let mismatched_shape = legacy_video_effect_catalog_authoritative_shape(
            "set_video_layer_isf_effect",
            json!({ "layer_id": layer_id, "effect": c1_test_event_effect(1.0) }),
        )
        .expect("encode mismatched legacy intent shape");
        let error = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_091,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
            mismatched_shape,
            |_video, _request| Ok(()),
        )
        .expect_err("same legacy terminal identity with different intent must not republish");
        assert!(error.contains("already completed with shape"));

        let replacement_incarnation =
            allocate_project_transaction_owner_incarnation(&harness.state)
                .expect("allocate replacement legacy terminal incarnation");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("rotate legacy terminal incarnation")
            .insert("media-asset-a6".to_string(), replacement_incarnation);
        let error = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_091,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding),
            shape,
            |_video, _request| Ok(()),
        )
        .expect_err("a retired legacy window cannot recover its receipt after ABA rotation");
        assert!(error.contains("renderer incarnation changed"));
        assert_eq!(
            harness
                .state
                .video_effect_catalog_authoritative_publish_attempts
                .load(Ordering::Acquire),
            1,
            "legacy reply recovery never creates a second Published ACK"
        );
    }

    #[test]
    fn video_effect_catalog_authoritative_legacy_append_intents_are_cross_kind_distinct_and_keep_added_response(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture append caller binding");
        let builtin = video::builtin_isf_effect("invert")
            .expect("resolve invert builtin")
            .expect("invert builtin exists");
        let direct_shape = legacy_video_effect_catalog_append_shape(
            "add_video_layer_isf_effect",
            layer_id,
            &builtin,
            None,
        )
        .expect("encode direct add shape");
        let builtin_shape = legacy_video_effect_catalog_append_shape(
            "add_builtin_video_isf_effect",
            layer_id,
            &builtin,
            Some("invert"),
        )
        .expect("encode builtin add shape");
        assert_ne!(
            direct_shape.fingerprint, builtin_shape.fingerprint,
            "a resolved builtin effect may equal direct input but never shares its terminal intent"
        );
        let baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let direct_effect = builtin.clone();
        mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_101,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
            direct_shape,
            move |video, request| {
                let index = legacy_materialize_layer_effect_chain(video, request, layer_id)?;
                request.effect_chains[index]
                    .stages
                    .push(legacy_video_effect_stage_summary(
                        video_isf_stage_from_root(direct_effect),
                        protocol::VideoEffectStageId(0),
                        protocol::VideoEffectId(0),
                    ));
                Ok(())
            },
        )
        .expect("direct add publishes one canonical catalog");
        let error = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_101,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding.clone()),
            builtin_shape,
            |_video, _request| -> Result<(), String> {
                panic!("cross-kind receipt collision must reject before rebuilding A")
            },
        )
        .expect_err("direct add receipt cannot be replayed as add_builtin");
        assert!(error.contains("already completed with shape"));
        c1_assert_one_authored_mutation(&harness, baseline);

        let second_effect = c1_test_event_effect(0.0);
        let second_shape = legacy_video_effect_catalog_append_shape(
            "add_video_layer_isf_effect",
            layer_id,
            &second_effect,
            None,
        )
        .expect("encode later direct add shape");
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let second_for_mutation = second_effect.clone();
        let second_result = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            84_102,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(binding),
            second_shape,
            move |video, request| {
                let index = legacy_materialize_layer_effect_chain(video, request, layer_id)?;
                request.effect_chains[index]
                    .stages
                    .push(legacy_video_effect_stage_summary(
                        video_isf_stage_from_root(second_for_mutation),
                        protocol::VideoEffectStageId(0),
                        protocol::VideoEffectId(0),
                    ));
                Ok(())
            },
        )
        .expect("second add publishes a later canonical stage");
        let normal_response =
            legacy_video_effect_catalog_append_response(second_effect.clone(), second_result);
        assert_eq!(normal_response.effect.label, second_effect.label);
        assert_eq!(normal_response.effect.source, second_effect.source);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("read second add persistence")
            .authored_video
            .expect("read second add authored video");
        let chain = persisted
            .effect_chains
            .iter()
            .find(|chain| chain.scope == VideoEffectScope::Layer { layer_id })
            .expect("second add Layer chain");
        let VideoEffectKind::Isf { effect: root } = &chain.stages[0].effect.kind;
        assert_ne!(
            normal_response.effect.label, root.label,
            "second-stage normal response is the newly-added effect, not the legacy root projection"
        );

        // Exercise the same response seam for a second-stage add_builtin.
        // The resolved source is intentionally different from the root so a
        // catalog re-projection would be observable here.
        let builtin_harness = MediaAssetA6CommandHarness::new();
        let (builtin_layer_id, _asset_id, _alternate_asset_id, _slot_id) =
            seed_video_clip_slot_layer(&builtin_harness);
        c1_stabilize_fixture_authority(&builtin_harness);
        let builtin_binding = capture_video_clip_slot_caller_binding_for_window_label(
            &builtin_harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture builtin append caller binding");
        let first_builtin = video::builtin_isf_effect("invert")
            .expect("resolve initial builtin")
            .expect("initial builtin exists");
        let first_shape = legacy_video_effect_catalog_append_shape(
            "add_builtin_video_isf_effect",
            builtin_layer_id,
            &first_builtin,
            Some("invert"),
        )
        .expect("encode initial builtin shape");
        let (epoch, revision, hash) = b3_authority_arguments(&builtin_harness);
        let first_for_mutation = first_builtin.clone();
        mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &builtin_harness.state,
            84_103,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(builtin_binding.clone()),
            first_shape,
            move |video, request| {
                let index =
                    legacy_materialize_layer_effect_chain(video, request, builtin_layer_id)?;
                request.effect_chains[index]
                    .stages
                    .push(legacy_video_effect_stage_summary(
                        video_isf_stage_from_root(first_for_mutation),
                        protocol::VideoEffectStageId(0),
                        protocol::VideoEffectId(0),
                    ));
                Ok(())
            },
        )
        .expect("initial builtin add publishes one root stage");
        let second_builtin = video::builtin_isf_effect("monochrome")
            .expect("resolve second builtin")
            .expect("second builtin exists");
        let second_builtin_shape = legacy_video_effect_catalog_append_shape(
            "add_builtin_video_isf_effect",
            builtin_layer_id,
            &second_builtin,
            Some("monochrome"),
        )
        .expect("encode second builtin shape");
        let (epoch, revision, hash) = b3_authority_arguments(&builtin_harness);
        let second_builtin_for_mutation = second_builtin.clone();
        let second_builtin_result = mutate_legacy_video_effect_catalog_authoritative_command_impl(
            &builtin_harness.state,
            84_104,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(builtin_binding),
            second_builtin_shape,
            move |video, request| {
                let index =
                    legacy_materialize_layer_effect_chain(video, request, builtin_layer_id)?;
                request.effect_chains[index]
                    .stages
                    .push(legacy_video_effect_stage_summary(
                        video_isf_stage_from_root(second_builtin_for_mutation),
                        protocol::VideoEffectStageId(0),
                        protocol::VideoEffectId(0),
                    ));
                Ok(())
            },
        )
        .expect("second builtin add publishes a later stage");
        let builtin_response = legacy_video_effect_catalog_append_response(
            second_builtin.clone(),
            second_builtin_result,
        );
        assert_eq!(builtin_response.effect.label, second_builtin.label);
        assert_eq!(builtin_response.effect.source, second_builtin.source);
        assert_ne!(builtin_response.effect.label, first_builtin.label);
    }

    #[test]
    fn video_transition_bus_c3_catalog_runtime_retry_release_and_read_are_authoritative() {
        let harness = MediaAssetA6CommandHarness::new();
        let (first_layer_id, _, _, _) = seed_video_clip_slot_layer(&harness);
        let (second_layer_id, _, _, _) = seed_video_clip_slot_layer(&harness);
        c1_stabilize_fixture_authority(&harness);
        let authored = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("capture C3 authored A")
            .authored_video
            .expect("C3 fixture has authored video");
        let composition_id = authored
            .compositions
            .iter()
            .find(|composition| {
                composition.layer_ids.contains(&first_layer_id)
                    && composition.layer_ids.contains(&second_layer_id)
            })
            .expect("both C3 layers share Main")
            .id;
        let first = VideoLayerTransitionTarget::Layer {
            layer_id: first_layer_id,
        };
        let second = VideoLayerTransitionTarget::Layer {
            layer_id: second_layer_id,
        };
        let request = VideoEffectCatalogApplyRequest {
            effect_chains: vec![c1_catalog_chain(
                &harness,
                VideoEffectScope::Transition {
                    owner: protocol::VideoTransitionEffectOwner::LayerBus {
                        bus_id: VideoTransitionBusId(0),
                    },
                },
                "C3 Bus FX",
                c1_test_event_effect(0.0),
            )],
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: vec![VideoLayerTransitionBusSummary {
                id: VideoTransitionBusId(0),
                label: "C3 A/B".to_string(),
                composition_id,
                enabled: true,
                members: vec![first.clone(), second.clone()],
                default_from: first.clone(),
                default_to: second.clone(),
                default_kind: VideoClipTakeKind::Crossfade,
                default_duration: VideoClipTakeDuration {
                    unit: protocol::VideoClipTakeDurationUnit::Beats,
                    value_milliunits: 1_000,
                },
                default_curve: VideoLayerTransitionCurve::EaseInOut,
                matte_source: None,
            }],
        };
        let baseline = harness.mutation_baseline();
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let applied = apply_video_effect_catalog_authoritative_command_impl(
            &harness.state,
            request,
            84_201,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("C3 bus and LayerBus FX publish in one authored transaction");
        let bus = applied.catalog.transition_buses[0].clone();
        assert_ne!(bus.id.0, 0);
        assert_eq!(
            applied.catalog.effect_chains[0].scope,
            VideoEffectScope::Transition {
                owner: protocol::VideoTransitionEffectOwner::LayerBus { bus_id: bus.id },
            }
        );
        c1_assert_one_authored_mutation(&harness, baseline);
        b3_assert_authority_matches_persistence(&harness);

        let runtime_baseline = harness.mutation_baseline();
        let authority = harness.authority();
        let launch_request = VideoLayerTransitionLaunchRequest {
            bus_id: bus.id,
            from: first,
            to: second,
            kind: bus.default_kind,
            duration: bus.default_duration,
            curve: bus.default_curve,
        };
        let launched = launch_video_layer_transition_bus_authoritative_command_impl(
            &harness.state,
            launch_request.clone(),
            84_202,
            authority.epoch,
            authority.revision,
            authority.checkpoint_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("launch C3 bus through history-free authority lane");
        assert_eq!(
            launched.command_kind,
            VideoLayerTransitionAuthoritativeRuntimeCommandKind::Launch
        );
        assert_eq!(launched.runtime.buses.len(), 1);
        assert_eq!(launched.runtime.buses[0].duration_ms, 500);
        let retried = launch_video_layer_transition_bus_authoritative_command_impl(
            &harness.state,
            launch_request,
            84_202,
            authority.epoch,
            authority.revision,
            authority.checkpoint_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("lost C3 launch reply recovers fresh runtime without republish");
        assert!(retried.runtime_generation >= launched.runtime_generation);
        assert_eq!(
            harness.mutation_baseline().revision,
            runtime_baseline.revision
        );
        assert_eq!(
            harness.mutation_baseline().undo_len,
            runtime_baseline.undo_len
        );

        release_video_layer_transition_bus_authoritative_command_impl(
            &harness.state,
            VideoLayerTransitionReleaseRequest { bus_id: bus.id },
            84_203,
            authority.epoch,
            authority.revision,
            authority.checkpoint_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("release C3 bus at any progress");
        let terminal = get_video_effect_catalog_operation_terminal_result_impl(
            &harness.state,
            84_202,
            authority.epoch,
            authority.revision,
            authority.checkpoint_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("query C3 launch receipt")
        .expect("C3 launch receipt exists");
        assert!(matches!(
            terminal.terminal,
            VideoEffectCatalogAuthoritativeTerminalResult::Runtime(_)
        ));
        assert!(terminal
            .runtime
            .expect("fresh C3 runtime attached")
            .runtime
            .buses
            .is_empty());
        let read = get_video_layer_transition_runtime_impl(
            &harness.state,
            authority.epoch,
            authority.revision,
            authority.checkpoint_hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("Full-readable C3 runtime report");
        assert!(read.runtime.buses.is_empty());
        assert_eq!(
            harness.mutation_baseline().revision,
            runtime_baseline.revision
        );
    }

    #[test]
    fn video_clip_slot_b3_authored_commands_use_one_ack_history_and_exact_retry() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, asset_id, alternate_asset_id, original_slot_id) =
            seed_video_clip_slot_layer(&harness);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let baseline = harness.mutation_baseline();
        let caller_binding = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture stable authored B3 incarnation");
        let create = VideoClipSlotCreateRequest {
            layer_id,
            media_asset_id: asset_id,
            in_point_ms: 12,
            out_point_ms: Some(120),
            loop_mode: Default::default(),
            speed: 1.0,
            cue_points: Vec::new(),
            launch_quantization: Default::default(),
            effect_overrides: Vec::new(),
            before_slot_id: Some(original_slot_id),
            make_default: false,
        };
        let created = create_video_clip_slot_authoritative_command_impl(
            &harness.state,
            create.clone(),
            83_101,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(caller_binding.clone()),
        )
        .expect("create clip slot through B3 command core");
        let created_slot_id = created.created_slot_ids[0];
        let retried = create_video_clip_slot_authoritative_command_impl(
            &harness.state,
            create,
            83_101,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(caller_binding),
        )
        .expect("lost-reply create retry returns canonical result");
        assert_media_asset_a6_same_terminal(&created, &retried);
        assert_eq!(
            harness
                .state
                .video_clip_slot_authoritative_publish_attempts
                .load(Ordering::Acquire),
            1,
            "exact create retry must return its receipt before allocation or publication"
        );
        b3_assert_authority_matches_persistence(&harness);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        assign_video_clip_slot_asset_authoritative_command_impl(
            &harness.state,
            VideoClipSlotAssignRequest {
                layer_id,
                slot_id: created_slot_id,
                media_asset_id: alternate_asset_id,
            },
            83_102,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("assign clip slot through B3 command core");
        b3_assert_authority_matches_persistence(&harness);

        let mut updated_slot = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("B3 persistence after create")
            .authored_video
            .expect("authored B3 video")
            .layers
            .into_iter()
            .find(|layer| layer.id == layer_id)
            .expect("B3 layer")
            .clip_slots
            .into_iter()
            .find(|slot| slot.id == created_slot_id)
            .expect("created B3 slot");
        updated_slot.speed = 1.25;
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        update_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotUpdateRequest {
                layer_id,
                slot: updated_slot,
            },
            83_103,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("update clip slot through B3 command core");
        b3_assert_authority_matches_persistence(&harness);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let duplicated = duplicate_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotDuplicateRequest {
                layer_id,
                source_slot_id: original_slot_id,
                before_slot_id: Some(created_slot_id),
            },
            83_104,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("duplicate clip slot through B3 command core");
        let duplicate_slot_id = duplicated.created_slot_ids[0];
        b3_assert_authority_matches_persistence(&harness);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        reorder_video_clip_slots_authoritative_command_impl(
            &harness.state,
            VideoClipSlotReorderRequest {
                layer_id,
                slot_ids: vec![created_slot_id, duplicate_slot_id, original_slot_id],
            },
            83_105,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reorder clip slots through B3 command core");
        b3_assert_authority_matches_persistence(&harness);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        set_default_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotSetDefaultRequest {
                layer_id,
                slot_id: created_slot_id,
            },
            83_106,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("set default clip slot through B3 command core");
        b3_assert_authority_matches_persistence(&harness);

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        remove_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotRemoveRequest {
                layer_id,
                slot_id: duplicate_slot_id,
            },
            83_107,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("remove clip slot through B3 command core");
        b3_assert_authority_matches_persistence(&harness);

        b3_assert_authored_mutation_count(&harness, baseline, 7);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("B3 final persistence");
        let layer = persisted
            .authored_video
            .expect("B3 final authored video")
            .layers
            .into_iter()
            .find(|layer| layer.id == layer_id)
            .expect("B3 final layer");
        assert_eq!(layer.default_clip_slot_id, Some(created_slot_id));
        assert_eq!(
            layer
                .clip_slots
                .iter()
                .map(|slot| slot.id)
                .collect::<Vec<_>>(),
            vec![created_slot_id, original_slot_id],
            "duplicate was inserted before its explicit anchor then removed"
        );
    }

    #[test]
    fn video_clip_slot_b3_runtime_commands_are_history_free_and_partial_only() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, asset_id, _alternate_asset_id, slot_id) =
            seed_video_clip_slot_layer(&harness);
        let mut full_policy = sample_operator_policy();
        full_policy.lock_mode = OperatorLockMode::Full;
        let full_epoch = {
            let mut coordinator = harness
                .state
                .project_coordinator
                .lock()
                .expect("configure initially-unlocked Full policy");
            coordinator.ancillary.operator_policy = Some(full_policy.clone());
            coordinator.epoch
        };
        harness
            .state
            .project_operator_sessions
            .lock()
            .expect("install unlocked B3 operator session")
            .insert(
                MEDIA_ASSET_A6_OWNER.to_string(),
                ProjectOperatorSession {
                    project_epoch: full_epoch,
                    policy: full_policy,
                    unlocked: true,
                },
            );
        let (queue_epoch, queue_revision, queue_hash) = b3_authority_arguments(&harness);
        let persistence_before = b3_persistence_hash(&harness);
        let baseline = harness.mutation_baseline();
        let queued = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_201,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("queue clip slot through B3 runtime command core");
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let cancelled = cancel_queued_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotCancelQueueRequest { layer_id },
            83_202,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("cancel queued clip slot through B3 runtime command core");
        let retried_queue = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_201,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("reply-loss queue retry attaches current cancelled runtime");
        assert_ne!(queued.runtime, cancelled.runtime);
        assert_eq!(retried_queue.runtime, cancelled.runtime);
        assert_eq!(cancelled.runtime_generation, queued.runtime_generation + 1);
        assert_eq!(
            retried_queue.runtime_generation,
            cancelled.runtime_generation
        );
        let queued_terminal = get_video_clip_slot_operation_terminal_result_impl(
            &harness.state,
            0,
            83_201,
            0,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("read queue terminal receipt")
        .expect("queue terminal receipt");
        assert_eq!(
            queued_terminal.command_kind,
            VideoClipSlotAuthoritativeCommitKind::Queue
        );
        assert_eq!(
            queued_terminal
                .runtime
                .as_ref()
                .expect("fresh runtime query")
                .runtime,
            cancelled.runtime
        );
        assert_eq!(
            queued_terminal
                .runtime
                .as_ref()
                .expect("fresh runtime query")
                .runtime_generation,
            cancelled.runtime_generation
        );

        // A terminal receipt is read-only recovery. Lock policy and unrelated
        // editor activity may block a *new* runtime publication, but they must
        // not make an already acknowledged result disappear after reply loss.
        let recovery_baseline = harness.mutation_baseline();
        harness
            .state
            .project_operator_sessions
            .lock()
            .expect("engage Full lock after queue ACK")
            .get_mut(MEDIA_ASSET_A6_OWNER)
            .expect("installed B3 operator session")
            .unlocked = false;
        let recovered_under_full = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_201,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("Full lock cannot block exact runtime receipt recovery");
        assert_eq!(recovered_under_full.runtime, cancelled.runtime);
        let full_new_error = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_210,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("Full lock still blocks a new runtime publication");
        assert!(full_new_error.contains("Full Lock"), "{full_new_error}");
        let queried_under_full = get_video_clip_slot_operation_terminal_result_impl(
            &harness.state,
            0,
            83_201,
            0,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("Full lock permits terminal query")
        .expect("queue receipt under Full lock");
        assert_eq!(
            queried_under_full
                .runtime
                .expect("fresh Full-lock runtime")
                .runtime,
            cancelled.runtime
        );
        harness
            .state
            .project_operator_sessions
            .lock()
            .expect("unlock Full policy after recovery")
            .get_mut(MEDIA_ASSET_A6_OWNER)
            .expect("installed B3 operator session")
            .unlocked = true;

        let pending_before = {
            let coordinator = harness
                .state
                .project_coordinator
                .lock()
                .expect("capture pending receipt-recovery checkpoint");
            project_checkpoint_for_coordinator(&harness.state, &coordinator)
                .expect("capture coherent pending checkpoint")
        };
        harness
            .state
            .project_coordinator
            .lock()
            .expect("install unrelated pending transaction")
            .history
            .pending
            .insert(
                83_211,
                PendingProjectTransaction {
                    transaction_id: 83_211,
                    client_operation_id: "test-unrelated".to_string(),
                    shape_fingerprint: "test-unrelated-shape".to_string(),
                    command_name: "test_command".to_string(),
                    schema_version: PROJECT_TRANSACTION_SCHEMA_VERSION,
                    owner_id: "renderer:unrelated".to_string(),
                    window_label: "main".to_string(),
                    owner_incarnation: 1,
                    label: "Unrelated edit".to_string(),
                    coalesce_key: String::new(),
                    epoch: pending_before.epoch,
                    before: pending_before,
                    command_result: None,
                    command_indeterminate_error: None,
                    closing: false,
                },
            );
        harness
            .state
            .project_transaction_active
            .store(true, Ordering::Release);
        get_video_clip_slot_operation_terminal_result_impl(
            &harness.state,
            0,
            83_201,
            0,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("pending/active cannot block terminal query")
        .expect("queue receipt while pending/active");
        let pending_new_error = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_212,
            queue_epoch,
            queue_revision,
            queue_hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("pending transaction still blocks a new runtime publication");
        assert!(
            pending_new_error.to_ascii_lowercase().contains("pending")
                || pending_new_error
                    .to_ascii_lowercase()
                    .contains("still being committed")
                // The external-admission gate must precede the coordinator,
                // so it cannot inspect the pending map while the shared
                // active flag is armed. A truthful generic active rejection
                // is therefore also the expected fail-closed outcome.
                || pending_new_error.to_ascii_lowercase().contains("active"),
            "{pending_new_error}"
        );
        harness
            .state
            .project_coordinator
            .lock()
            .expect("clear unrelated pending transaction")
            .history
            .pending
            .clear();
        let active_new_error = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_213,
            queue_epoch,
            queue_revision,
            queue_hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("active transaction still blocks a new runtime publication");
        assert!(
            active_new_error.to_ascii_lowercase().contains("active"),
            "{active_new_error}"
        );
        harness
            .state
            .project_transaction_active
            .store(false, Ordering::Release);
        let recovery_after = harness.mutation_baseline();
        assert_eq!(recovery_after.revision, recovery_baseline.revision);
        assert_eq!(
            recovery_after.history_generation,
            recovery_baseline.history_generation
        );
        assert_eq!(recovery_after.undo_len, recovery_baseline.undo_len);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        launch_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotLaunchRequest {
                layer_id,
                slot_id: Some(slot_id),
                transition_kind: VideoClipTakeKind::Cut,
                transition_duration_ms: 0,
                transition_duration: None,
            },
            83_203,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("launch clip slot through B3 runtime command core");
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        seek_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotSeekRequest {
                layer_id,
                position_ms: 33,
            },
            83_204,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("seek clip slot through B3 runtime command core");
        assert_eq!(
            b3_persistence_hash(&harness),
            persistence_before,
            "runtime slot transport cannot change the persisted project hash"
        );
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("B3 runtime coordinator");
        assert_eq!(coordinator.revision, baseline.revision);
        assert_eq!(coordinator.history_generation, baseline.history_generation);
        assert_eq!(coordinator.history.undo.len(), baseline.undo_len);
        drop(coordinator);

        let generation_before_overflow = harness
            .state
            .video_clip_slot_runtime_generation
            .load(Ordering::Acquire);
        harness
            .state
            .video_clip_slot_runtime_generation
            .store(VIDEO_CLIP_RUNTIME_GENERATION_MAX - 1, Ordering::Release);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let boundary = queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_208,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("last JavaScript-exact runtime generation publishes");
        assert_eq!(
            boundary.runtime_generation,
            VIDEO_CLIP_RUNTIME_GENERATION_MAX
        );
        let runtime_before_overflow = harness.state.engine.snapshot().video_clip_runtime;
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        assert!(cancel_queued_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotCancelQueueRequest { layer_id },
            83_209,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("unsafe JSON runtime generation fails before engine publication")
        .contains("generation is exhausted"));
        assert_eq!(
            harness.state.engine.snapshot().video_clip_runtime,
            runtime_before_overflow
        );
        assert_eq!(
            harness
                .state
                .video_clip_slot_runtime_generation
                .load(Ordering::Acquire),
            VIDEO_CLIP_RUNTIME_GENERATION_MAX
        );
        harness
            .state
            .video_clip_slot_runtime_generation
            .store(generation_before_overflow, Ordering::Release);

        harness
            .state
            .project_coordinator
            .lock()
            .expect("install Partial lock")
            .ancillary
            .operator_policy = Some(sample_operator_policy());
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_205,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("Partial lock permits runtime transport");
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        assert!(create_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotCreateRequest {
                layer_id,
                media_asset_id: asset_id,
                in_point_ms: 0,
                out_point_ms: None,
                loop_mode: Default::default(),
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: Default::default(),
                effect_overrides: Vec::new(),
                before_slot_id: None,
                make_default: false,
            },
            83_206,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("Partial lock rejects authored B3 commands")
        .contains("Partial Lock"));
        harness
            .state
            .project_coordinator
            .lock()
            .expect("upgrade Full lock")
            .ancillary
            .operator_policy
            .as_mut()
            .expect("installed policy")
            .lock_mode = OperatorLockMode::Full;
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        assert!(queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_207,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("Full lock rejects runtime B3 commands")
        .contains("Full Lock"));
    }

    #[test]
    fn video_clip_slot_b3_window_binding_rotation_and_runtime_read_fail_closed() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, asset_id, _alternate_asset_id, slot_id) =
            seed_video_clip_slot_layer(&harness);
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("register second B3 pane")
            .insert("b3-pane-b".to_string(), "renderer:b3-pane-b".to_string());

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let baseline = harness.mutation_baseline();
        let old_incarnation = capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "media-asset-a6",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect("capture old B3 wrapper incarnation before barrier");
        create_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotCreateRequest {
                layer_id,
                media_asset_id: asset_id,
                in_point_ms: 0,
                out_point_ms: None,
                loop_mode: Default::default(),
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: Default::default(),
                effect_overrides: Vec::new(),
                before_slot_id: Some(slot_id),
                make_default: false,
            },
            83_251,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(old_incarnation.clone()),
        )
        .expect("create A-owned B3 receipt");
        b3_assert_authored_mutation_count(&harness, baseline, 1);

        let mut full_policy = sample_operator_policy();
        full_policy.lock_mode = OperatorLockMode::Full;
        harness
            .state
            .project_coordinator
            .lock()
            .expect("install B3 Full lock")
            .ancillary
            .operator_policy = Some(full_policy);
        let publish_before = harness
            .state
            .video_clip_slot_authoritative_publish_attempts
            .load(Ordering::Acquire);
        assert!(capture_video_clip_slot_caller_binding_for_window_label(
            &harness.state,
            "b3-pane-b",
            MEDIA_ASSET_A6_OWNER,
        )
        .expect_err("pane B cannot pass pane A's owner to any B3 public wrapper")
        .contains("does not match"));
        assert_eq!(
            harness
                .state
                .video_clip_slot_authoritative_publish_attempts
                .load(Ordering::Acquire),
            publish_before,
            "window mismatch is rejected before receipt lookup or engine publication"
        );

        let runtime_before = harness.state.engine.snapshot().video_clip_runtime;
        let read = get_video_clip_slot_runtime_impl(
            &harness.state,
            epoch,
            revision.wrapping_add(1),
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect_err("read rejects stale E/R/H without reconciling");
        assert!(read.contains("changed"));
        let current = {
            let coordinator = harness
                .state
                .project_coordinator
                .lock()
                .expect("capture read-only B3 authority");
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.checkpoint_hash.clone(),
            )
        };
        let read_baseline = harness.mutation_baseline();
        let read = get_video_clip_slot_runtime_impl(
            &harness.state,
            current.0,
            current.1,
            current.2,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("Full lock permits truthful B3 runtime read");
        assert_eq!(read.runtime, runtime_before);
        let after_read = harness.mutation_baseline();
        assert_eq!(after_read.revision, read_baseline.revision);
        assert_eq!(
            after_read.history_generation,
            read_baseline.history_generation
        );

        // Model same-string owner re-registration after the old renderer is
        // retired. Production retirement purges after it removes the window,
        // then a new window incarnation can register the same visible string.
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("retire old B3 owner")
            .remove("media-asset-a6");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("retire old B3 incarnation")
            .remove("media-asset-a6");
        harness
            .state
            .media_asset_operations
            .purge_video_clip_slot_authoritative_for_owner(MEDIA_ASSET_A6_OWNER);
        harness
            .state
            .project_transaction_owners
            .lock()
            .expect("re-register same B3 owner string")
            .insert(
                "media-asset-a6".to_string(),
                MEDIA_ASSET_A6_OWNER.to_string(),
            );
        let new_incarnation = allocate_project_transaction_owner_incarnation(&harness.state)
            .expect("allocate re-registered B3 incarnation");
        harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .expect("register new B3 incarnation")
            .insert("media-asset-a6".to_string(), new_incarnation);
        assert!(get_video_clip_slot_operation_terminal_result_impl(
            &harness.state,
            0,
            83_251,
            0,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(old_incarnation.clone()),
        )
        .expect_err("retired same-string owner cannot query old B3 receipt")
        .contains("renderer incarnation changed"));
        assert!(create_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotCreateRequest {
                layer_id,
                media_asset_id: asset_id,
                in_point_ms: 0,
                out_point_ms: None,
                loop_mode: Default::default(),
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: Default::default(),
                effect_overrides: Vec::new(),
                before_slot_id: Some(slot_id),
                make_default: false,
            },
            83_251,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(old_incarnation.clone()),
        )
        .expect_err("retired same-string owner cannot retry old B3 mutation")
        .contains("renderer incarnation changed"));
        let current = b3_authority_arguments(&harness);
        assert!(queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest { layer_id, slot_id },
            83_252,
            current.0,
            current.1,
            current.2,
            MEDIA_ASSET_A6_OWNER.to_string(),
            Some(old_incarnation),
        )
        .expect_err("unknown old-incarnation key is rejected after wrapper validation barrier")
        .contains("renderer incarnation changed"));
        assert_eq!(
            harness
                .state
                .video_clip_slot_authoritative_publish_attempts
                .load(Ordering::Acquire),
            publish_before,
            "retired receipt retry cannot republish"
        );
    }

    #[test]
    fn video_clip_slot_b3_public_ipc_surface_binds_every_command_to_its_window() {
        const B3_PUBLIC_COMMANDS: &[&str] = &[
            "create_video_clip_slot_authoritative",
            "assign_video_clip_slot_asset_authoritative",
            "update_video_clip_slot_authoritative",
            "remove_video_clip_slot_authoritative",
            "reorder_video_clip_slots_authoritative",
            "duplicate_video_clip_slot_authoritative",
            "set_default_video_clip_slot_authoritative",
            "queue_video_clip_slot_authoritative",
            "cancel_queued_video_clip_slot_authoritative",
            "launch_video_clip_slot_authoritative",
            "seek_video_clip_slot_authoritative",
            "import_and_assign_video_clip_slots_authoritative",
            "get_video_clip_slot_operation_terminal_result",
            "get_video_clip_slot_runtime",
        ];
        // This test module is included by main.rs but physically lives in
        // src/tests after the mechanical split, so inspect the authoritative
        // parent source explicitly rather than resolving against this folder.
        let source = include_str!("../main.rs");
        for command in B3_PUBLIC_COMMANDS {
            let needle = format!("fn {command}(");
            let start = source
                .find(&needle)
                .unwrap_or_else(|| panic!("missing registered B3 command {command}"));
            let body = &source[start..];
            let end = body
                .find("\n}\n")
                .unwrap_or_else(|| panic!("unterminated B3 command {command}"));
            let body = &body[..end];
            assert!(
                body.contains("window: WebviewWindow"),
                "{command} must receive the concrete invoking WebView"
            );
            assert!(
                body.contains("capture_video_clip_slot_caller_binding_for_window_label"),
                "{command} must capture its exact backend window incarnation before receipt/mutation/query"
            );
        }
    }

    #[test]
    fn video_clip_slot_c2_crossfade_polling_advances_generation_and_retry_returns_current_truth() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, alternate_asset_id, outgoing_slot_id) =
            seed_video_clip_slot_layer(&harness);
        let incoming_slot_id = harness.state.engine.allocate_video_clip_slot_id();
        harness
            .state
            .engine
            .create_video_clip_slot_published(
                layer_id,
                VideoClipSlotSummary {
                    id: incoming_slot_id,
                    media_asset_id: alternate_asset_id,
                    in_point_ms: 0,
                    out_point_ms: None,
                    loop_mode: Default::default(),
                    speed: 0.0,
                    cue_points: Vec::new(),
                    launch_quantization: Default::default(),
                    effect_overrides: Vec::new(),
                },
                None,
                false,
            )
            .expect("add C2 incoming slot");
        *harness
            .state
            .project_coordinator
            .lock()
            .expect("reset C2 coordinator after seed") =
            project_coordinator_for_initial_snapshot(harness.state.engine.snapshot());

        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        let initial = get_video_clip_slot_runtime_impl(
            &harness.state,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("observe initial C2 runtime");
        assert_eq!(
            initial.runtime.layers[0].active_slot_id,
            Some(outgoing_slot_id)
        );
        queue_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotQueueRequest {
                layer_id,
                slot_id: incoming_slot_id,
            },
            83_250,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("queue C2 incoming slot");
        let request = VideoClipSlotLaunchRequest {
            layer_id,
            slot_id: None,
            transition_kind: VideoClipTakeKind::Wipe,
            transition_duration_ms: 0,
            transition_duration: Some(VideoClipTakeDuration {
                unit: protocol::VideoClipTakeDurationUnit::Beats,
                value_milliunits: 500,
            }),
        };
        let launched = launch_video_clip_slot_authoritative_command_impl(
            &harness.state,
            request.clone(),
            83_251,
            epoch,
            revision,
            hash.clone(),
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("launch C2 crossfade");
        let transition = launched.runtime.layers[0]
            .transition
            .as_ref()
            .expect("C2 launch publishes dual-source runtime truth");
        assert_eq!(transition.outgoing_slot_id, outgoing_slot_id);
        assert_eq!(transition.incoming_slot_id, incoming_slot_id);
        assert_eq!(transition.duration_ms, 250);
        assert_eq!(transition.kind, VideoClipTakeKind::Wipe);
        assert_eq!(transition.duration, request.effective_transition_duration());

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let completed = loop {
            let report = get_video_clip_slot_runtime_impl(
                &harness.state,
                epoch,
                revision,
                hash.clone(),
                MEDIA_ASSET_A6_OWNER.to_string(),
                None,
            )
            .expect("poll current C2 runtime");
            if report.runtime.layers[0].transition.is_none()
                && report.runtime.layers[0].active_slot_id == Some(incoming_slot_id)
            {
                break report;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "C2 transition timed out"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(completed.runtime_generation > launched.runtime_generation);
        let retried = launch_video_clip_slot_authoritative_command_impl(
            &harness.state,
            request,
            83_251,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("exact C2 retry reads current runtime truth");
        assert_eq!(retried.runtime_generation, completed.runtime_generation);
        assert_eq!(retried.runtime, completed.runtime);
    }

    #[test]
    fn video_clip_slot_b3_direct_import_assign_is_ordered_atomic_and_receipted() {
        let harness = MediaAssetA6CommandHarness::new();
        let (layer_id, _asset_id, _alternate_asset_id, anchor_slot_id) =
            seed_video_clip_slot_layer(&harness);
        let (epoch, revision, hash) = b3_authority_arguments(&harness);
        launch_video_clip_slot_authoritative_command_impl(
            &harness.state,
            VideoClipSlotLaunchRequest {
                layer_id,
                slot_id: Some(anchor_slot_id),
                transition_kind: VideoClipTakeKind::Cut,
                transition_duration_ms: 0,
                transition_duration: None,
            },
            83_301,
            epoch,
            revision,
            hash,
            MEDIA_ASSET_A6_OWNER.to_string(),
            None,
        )
        .expect("launch seed slot before direct import");
        let runtime_before = harness.state.engine.snapshot().video_clip_runtime;
        let rendered_source_before = harness
            .state
            .engine
            .snapshot()
            .video
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .expect("rendered B3 seed layer")
            .source
            .clone();
        let baseline = harness.mutation_baseline();
        let identity = harness.stage_import(
            83_302,
            VideoSourceKind::File,
            &["b3-direct-a", "b3-direct-b"],
        );
        let request = VideoClipSlotImportAndAssignRequest {
            target_layer_id: layer_id,
            before_slot_id: Some(anchor_slot_id),
            make_default: false,
        };
        let (token, request_id, generation, epoch, revision, hash, owner_id) = identity.arguments();
        let imported = import_and_assign_video_clip_slots_authoritative_command_impl(
            &harness.state,
            request.clone(),
            token,
            request_id,
            generation,
            epoch,
            revision,
            hash.clone(),
            owner_id.clone(),
            None,
        )
        .expect("two-file B3 direct import and assignment");
        let retried = import_and_assign_video_clip_slots_authoritative_command_impl(
            &harness.state,
            request,
            token,
            request_id,
            generation,
            epoch,
            revision,
            hash.clone(),
            owner_id.clone(),
            None,
        )
        .expect("lost reply direct import returns its terminal receipt");
        assert_media_asset_a6_same_terminal(&imported, &retried);
        assert_eq!(imported.insertion_before_slot_id, Some(anchor_slot_id));
        assert_eq!(imported.created_slot_ids.len(), 2);
        assert_eq!(imported.imported_asset_ids.len(), 2);
        let persisted = harness
            .state
            .engine
            .persistence_snapshot()
            .expect("B3 direct import persistence");
        let layer = persisted
            .authored_video
            .expect("B3 direct authored video")
            .layers
            .into_iter()
            .find(|layer| layer.id == layer_id)
            .expect("B3 direct layer");
        assert_eq!(
            layer
                .clip_slots
                .iter()
                .map(|slot| slot.id)
                .collect::<Vec<_>>(),
            vec![
                imported.created_slot_ids[0],
                imported.created_slot_ids[1],
                anchor_slot_id,
            ],
            "the direct drop inserts the prepared Vec in order before its explicit anchor"
        );
        assert_eq!(layer.default_clip_slot_id, Some(anchor_slot_id));
        assert_eq!(
            harness.state.engine.snapshot().video_clip_runtime,
            runtime_before
        );
        assert_eq!(
            harness
                .state
                .engine
                .snapshot()
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)
                .expect("rendered B3 direct layer")
                .source,
            rendered_source_before,
            "direct import must not replace the active rendered source"
        );
        b3_assert_authored_mutation_count(&harness, baseline, 1);
        let receipt = get_video_clip_slot_operation_terminal_result_impl(
            &harness.state,
            token,
            request_id,
            generation,
            epoch,
            revision,
            hash,
            owner_id,
            None,
        )
        .expect("query B3 direct terminal receipt")
        .expect("B3 direct terminal receipt exists");
        assert_eq!(
            serde_json::to_value(receipt.terminal).expect("serialize receipt terminal"),
            serde_json::to_value(VideoClipSlotAuthoritativeTerminalResult::Authored(
                Box::new(imported)
            ))
            .expect("serialize direct terminal"),
            "terminal query recovers the allocated ordered IDs and result"
        );

        // An invalid direct insertion anchor must fail during candidate
        // preflight. It may allocate only ephemeral candidate IDs; neither
        // catalog/bank/default/rendered/runtime nor B authority/history may
        // change before the engine's sole publication boundary.
        let failure_before_hash = b3_persistence_hash(&harness);
        let failure_before_runtime = harness.state.engine.snapshot().video_clip_runtime;
        let failure_before = harness.mutation_baseline();
        let failed_identity =
            harness.stage_import(83_303, VideoSourceKind::File, &["b3-direct-invalid-anchor"]);
        let (token, request_id, generation, epoch, revision, hash, owner_id) =
            failed_identity.arguments();
        assert!(
            import_and_assign_video_clip_slots_authoritative_command_impl(
                &harness.state,
                VideoClipSlotImportAndAssignRequest {
                    target_layer_id: layer_id,
                    before_slot_id: Some(VideoClipSlotId(999_999)),
                    make_default: true,
                },
                token,
                request_id,
                generation,
                epoch,
                revision,
                hash,
                owner_id,
                None,
            )
            .expect_err("invalid anchor rejects direct import before engine publication")
            .contains("was not found")
        );
        assert_eq!(b3_persistence_hash(&harness), failure_before_hash);
        assert_eq!(
            harness.state.engine.snapshot().video_clip_runtime,
            failure_before_runtime
        );
        let coordinator = harness
            .state
            .project_coordinator
            .lock()
            .expect("B3 direct anchor failure coordinator");
        assert_eq!(coordinator.revision, failure_before.revision);
        assert_eq!(
            coordinator.history_generation,
            failure_before.history_generation
        );
        assert_eq!(coordinator.history.undo.len(), failure_before.undo_len);
        assert_eq!(
            coordinator.publication_generation,
            failure_before.publication_generation
        );
        drop(coordinator);
        assert_eq!(
            harness
                .state
                .video_clip_slot_authoritative_publish_attempts
                .load(Ordering::Acquire),
            1,
            "invalid anchor cannot produce a second direct publication"
        );
    }
}
