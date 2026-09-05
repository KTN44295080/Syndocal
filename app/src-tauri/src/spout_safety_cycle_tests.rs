// Real engine authority with injected output definitions; no SDK/device calls.
#[test]
fn spout_safety_cycle_final_sdk_check_rejects_enqueue_before_token_publication() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..Default::default()
    });
    let id = 98_313;
    install_injected_spout_output(&engine, id);
    let authority = injected_spout_authority(&engine, id);
    let mut queued = authority.blackout_authority();
    queued.engaged = true;
    queued.generation += 1;
    let mut sends = 0;
    let result = send_frame_if_authorized_typed("Spout", &engine, &authority, None, || {
        // Injection point is after outer revalidation and transport validation.
        // The safety latch changed, while the snapshot/token is still old.
        validate_final_sdk_sample(&authority, authority.presentation_config_token(), queued)?;
        sends += 1;
        Ok(())
    });
    assert!(matches!(result, Err(PhysicalOutputSendError::Revoked(_))));
    assert_eq!(sends, 0);
}

#[test]
fn spout_safety_cycle_materializes_black_before_snapshot_catches_up() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..Default::default()
    });
    let id = 98_310;
    install_injected_spout_output_extent(&engine, id, 4, 2);
    let sample = engine.video_presentation_sample();
    assert!(!sample.snapshot.blackout);
    let mut safety = engine.safety_blackout_authority();
    safety.engaged = true;
    safety.generation += 1;
    let authority = capture_output_presentation_authority(
        "Spout",
        protocol::VideoOutputKind::SpoutSender,
        &sample,
        engine.output_ownership_status(),
        safety,
        id,
        &injected_spout_endpoint(id),
    )
    .unwrap();
    assert!(authority.project_blackout());
    let frame = materialize_current_hard_blackout(&authority).unwrap();
    assert_eq!((frame.width, frame.height), (4, 2));
    assert!(frame.data.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));

    let mut lingering = sample.clone();
    lingering.snapshot.blackout = true;
    safety.engaged = false;
    let authority = capture_output_presentation_authority(
        "Spout",
        protocol::VideoOutputKind::SpoutSender,
        &lingering,
        engine.output_ownership_status(),
        safety,
        id,
        &injected_spout_endpoint(id),
    )
    .unwrap();
    assert!(
        !authority.project_blackout(),
        "DMX-only snapshot blackout must not blank video after S0 release"
    );
}

#[test]
fn spout_safety_cycle_discards_old_frame_and_preserves_sdk_failure() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..Default::default()
    });
    let id = 98_311;
    install_injected_spout_output(&engine, id);
    let old = injected_spout_authority(&engine, id);
    for _ in 0..2 {
        engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(5))
            .unwrap();
        let current = injected_spout_authority(&engine, id);
        assert!(current.project_blackout());
        assert!(matches!(
            revalidate_output_presentation_authority_classified(&engine, "Spout", &old),
            Err(OutputPresentationRevalidationError::SafetyChanged(_))
        ));
        let mut sends = 0;
        assert!(matches!(
            send_frame_if_authorized("Spout", &engine, &old, None, || {
                sends += 1;
                Ok(())
            }),
            Err(PhysicalOutputSendError::Revoked(_))
        ));
        assert_eq!(sends, 0);
        assert!(matches!(
            classify_spout_keepalive_send_error(
                &engine,
                &old,
                PhysicalOutputSendError::Revoked("changed".into())
            ),
            OutputPresentationRevalidationError::SafetyChanged(_)
        ));
        assert!(matches!(classify_spout_keepalive_send_error(&engine, &old,
            PhysicalOutputSendError::Sdk("injected SDK failure".into())),
            OutputPresentationRevalidationError::Other(reason) if reason == "injected SDK failure"));
        let safety = engine.safety_blackout_authority();
        engine
            .safety_blackout_release_published(
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
        let fresh = injected_spout_authority(&engine, id);
        send_frame_if_authorized("Spout", &engine, &fresh, None, || {
            sends += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(sends, 1);
    }
    assert_eq!(
        engine.output_ownership_status().state,
        OutputOwnershipState::Ready
    );
}

#[test]
fn spout_safety_cycle_does_not_hide_removed_output_or_ownership_loss() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..Default::default()
    });
    let id = 98_312;
    install_injected_spout_output(&engine, id);
    let old = injected_spout_authority(&engine, id);
    engine
        .safety_blackout_engage_published(Instant::now() + Duration::from_secs(5))
        .unwrap();
    engine.remove_video_output_published(id).unwrap();
    assert!(
        matches!(revalidate_output_presentation_authority_classified(&engine, "Spout", &old),
        Err(OutputPresentationRevalidationError::Other(reason)) if reason.contains("removed"))
    );
    let _fence = engine.begin_output_ownership_failure_fence("injected ownership loss");
    assert!(
        matches!(revalidate_output_presentation_authority_classified(&engine, "Spout", &old),
        Err(OutputPresentationRevalidationError::Other(reason)) if reason.contains("authority changed"))
    );
}
