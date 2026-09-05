use super::*;
use protocol::control_plane_command::OutputControlTargetRoleV1 as Target;

#[test]
fn output_blackout_target_video_fence_ignores_lighting_and_preserves_s0() {
    for kind in [VideoOutputKind::SpoutSender, VideoOutputKind::NdiSender] {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..Default::default()
        });
        let mut snapshot = protocol::EngineSnapshot::default();
        snapshot.video.outputs.push(protocol::VideoOutputSummary {
            id: 1,
            label: "Target test".into(),
            kind: kind.clone(),
            enabled: true,
            composition_id: 1,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 4,
            height: 2,
            endpoint_name: Some("Target test".into()),
            opacity: 1.0,
            blackout: false,
            mapping: Default::default(),
        });
        engine.load_project_snapshot_and_wait(snapshot).unwrap();
        let capture = || {
            capture_output_presentation_authority(
                "Target test",
                kind.clone(),
                &engine.video_presentation_sample(),
                engine.output_ownership_status(),
                engine.safety_blackout_authority(),
                1,
                "Target test",
            )
            .unwrap()
        };
        let visible = capture();
        engine
            .set_output_blackout_published(
                Target::Lighting,
                true,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
        assert!(!capture().project_blackout());
        revalidate_output_presentation_authority(&engine, "Target test", &visible).unwrap();
        let mut sends = 0;
        send_frame_if_authorized("Target test", &engine, &visible, None, || {
            sends += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(sends, 1);
        engine
            .set_output_blackout_published(
                Target::Video,
                true,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
        assert!(capture().project_blackout());
        assert!(
            send_frame_if_authorized("Target test", &engine, &visible, None, || {
                sends += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(sends, 1);
        engine
            .set_output_blackout_published(
                Target::Video,
                false,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
        assert!(matches!(
            revalidate_output_presentation_authority_classified(&engine, "Target test", &visible),
            Err(OutputPresentationRevalidationError::PresentationChanged(_))
        ));
        assert!(
            send_frame_if_authorized("Target test", &engine, &visible, None, || {
                sends += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(
            sends, 1,
            "the ON/OFF round trip cannot revive a stale frame"
        );
        let fresh = capture();
        assert!(!fresh.project_blackout());
        revalidate_output_presentation_authority_classified(&engine, "Target test", &fresh)
            .unwrap();
        let mut wrong_route = fresh.clone();
        wrong_route.route_endpoint_name = "retired route".into();
        assert!(matches!(
            revalidate_output_presentation_authority_classified(
                &engine,
                "Target test",
                &wrong_route
            ),
            Err(OutputPresentationRevalidationError::Other(_))
        ));
        let mut wrong_owner = fresh.clone();
        wrong_owner.ownership.epoch = wrong_owner.ownership.epoch.saturating_add(1);
        assert!(matches!(
            revalidate_output_presentation_authority_classified(
                &engine,
                "Target test",
                &wrong_owner
            ),
            Err(OutputPresentationRevalidationError::Other(_))
        ));
        engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(2))
            .unwrap();
        engine
            .set_output_blackout_published(
                Target::Both,
                false,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
        assert!(capture().project_blackout());
        let frame = materialize_current_hard_blackout(&capture()).unwrap();
        assert!(frame
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 0, 0, 255]));
    }
}
