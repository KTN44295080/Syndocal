use super::*;
use protocol::control_plane_command::OutputControlTargetRoleV1 as Target;

fn target_runtime() -> EngineRuntime {
    EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..Default::default()
    })
}

fn apply_target(
    runtime: &mut EngineRuntime,
    target: Target,
    enabled: bool,
    expires_at: Instant,
) -> mpsc::Receiver<Result<bool, String>> {
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::SetOutputBlackoutPublished {
        target,
        enabled,
        expires_at,
        admission: ProjectSnapshotLoadAdmission::new(),
        ack,
    });
    receiver
}

#[test]
fn output_blackout_target_ack_atomicity_noop_and_token_separation() {
    let mut runtime = target_runtime();
    let snapshot = RwLock::new(EngineSnapshot::default());
    let safety = runtime.shared_telemetry.safety_blackout_authority();
    for (target, enabled, expected, changed, changes_video) in [
        (Target::Lighting, true, (true, false), true, false),
        (Target::Lighting, true, (true, false), false, false),
        (Target::Video, true, (true, true), true, true),
        (Target::Both, false, (false, false), true, true),
        (Target::Both, false, (false, false), false, false),
        (Target::Both, true, (true, true), true, true),
        (Target::Lighting, false, (false, true), true, false),
    ] {
        let token = runtime.video_presentation_config_token;
        let receiver = apply_target(
            &mut runtime,
            target,
            enabled,
            Instant::now() + Duration::from_secs(2),
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        runtime.publish_pending_command_acks(0, &snapshot);
        assert_eq!(receiver.recv().unwrap(), Ok(changed));
        let published = snapshot.read().unwrap();
        assert_eq!((published.blackout, published.video.blackout), expected);
        assert_eq!(
            runtime.video_presentation_config_token != token,
            changes_video
        );
        assert_eq!(runtime.shared_telemetry.safety_blackout_authority(), safety);
    }
    assert!(!EngineCommand::Blackout(true).mutates_video_presentation_configuration());
}

#[test]
fn output_blackout_target_expiry_and_failed_publication_restore_both() {
    let mut runtime = target_runtime();
    let snapshot = RwLock::new(EngineSnapshot::default());
    let receiver = apply_target(
        &mut runtime,
        Target::Both,
        true,
        Instant::now() - Duration::from_secs(1),
    );
    runtime.publish_pending_command_acks(0, &snapshot);
    assert!(receiver.recv().unwrap().unwrap_err().contains("expired"));
    assert!(!runtime.blackout && !runtime.video_blackout);
    assert!(!runtime.video_presentation_config_dirty);
    let guard = snapshot.read().unwrap();
    let receiver = apply_target(
        &mut runtime,
        Target::Both,
        true,
        Instant::now() + Duration::from_secs(2),
    );
    runtime.publish_pending_command_acks(0, &snapshot);
    assert!(receiver.recv().unwrap().is_err());
    assert!(!runtime.blackout && !runtime.video_blackout);
    drop(guard);
}

#[test]
fn output_blackout_target_release_never_clears_emergency_safety() {
    let mut runtime = target_runtime();
    let snapshot = RwLock::new(EngineSnapshot::default());
    let (_, safety) = runtime.shared_telemetry.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    runtime.blackout = true;
    runtime.video_blackout = true;
    let receiver = apply_target(
        &mut runtime,
        Target::Both,
        false,
        Instant::now() + Duration::from_secs(2),
    );
    runtime.publish_pending_command_acks(0, &snapshot);
    assert_eq!(receiver.recv().unwrap(), Ok(true));
    assert!(!runtime.blackout && !runtime.video_blackout);
    assert_eq!(runtime.shared_telemetry.safety_blackout_authority(), safety);
    let published = snapshot.read().unwrap();
    assert!(published.blackout);
    assert!(!published.authored_blackout);
    assert!(published.safety_blackout_engaged);
}

#[test]
fn output_blackout_target_follow_outgoing_cannot_restore_lighting() {
    let now = Instant::now();
    let mut runtime =
        timeline_follow_transitioning_runtime(protocol::TimelineFollowFaultPolicy::Hold, now);
    assert!(!runtime.output.enabled, "fixture must not open a device");
    runtime
        .timeline_follow_transition
        .as_mut()
        .unwrap()
        .outgoing_dmx_frames
        .insert(runtime.output.universe, [200; 512]);
    let snapshot = RwLock::new(EngineSnapshot::default());
    runtime.tick(0, &snapshot);
    assert!(runtime.last_frame.iter().any(|value| *value > 0));
    runtime.blackout = true;
    runtime.tick(0, &snapshot);
    assert!(runtime.last_frame.iter().all(|value| *value == 0));
    assert!(runtime
        .last_frames_by_universe
        .values()
        .all(|frame| frame.iter().all(|value| *value == 0)));
}

#[test]
fn output_blackout_target_waiter_cancels_queued_but_waits_after_admission() {
    let mut runtime = target_runtime();
    let snapshot = RwLock::new(EngineSnapshot::default());
    let deadline = Instant::now() + Duration::from_millis(20);
    let admission = ProjectSnapshotLoadAdmission::new();
    let (ack, receiver) = mpsc::sync_channel(1);
    assert!(receive_admitted_command_ack_unbounded(
        receiver,
        &admission,
        deadline,
        Duration::from_millis(20),
        "output blackout test"
    )
    .is_err());
    runtime.apply_command(EngineCommand::SetOutputBlackoutPublished {
        target: Target::Both,
        enabled: true,
        expires_at: Instant::now() + Duration::from_secs(2),
        admission,
        ack,
    });
    runtime.publish_pending_command_acks(0, &snapshot);
    assert!(!runtime.blackout && !runtime.video_blackout);

    let deadline = Instant::now() + Duration::from_millis(20);
    let admission = ProjectSnapshotLoadAdmission::new();
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::SetOutputBlackoutPublished {
        target: Target::Both,
        enabled: true,
        expires_at: deadline,
        admission: admission.clone(),
        ack,
    });
    let (done, completed) = mpsc::sync_channel(1);
    let waiter = std::thread::spawn(move || {
        done.send(receive_admitted_command_ack_unbounded(
            receiver,
            &admission,
            deadline,
            Duration::from_millis(20),
            "output blackout test",
        ))
        .unwrap();
    });
    std::thread::sleep(Duration::from_millis(40));
    assert!(matches!(
        completed.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    runtime.publish_pending_command_acks(0, &snapshot);
    assert_eq!(
        completed.recv_timeout(Duration::from_secs(1)).unwrap(),
        Ok(true)
    );
    waiter.join().unwrap();
    assert!(snapshot.read().unwrap().authored_blackout);
    assert!(snapshot.read().unwrap().video.blackout);
}

#[test]
fn output_blackout_target_held_submission_publishes_without_reacquiring_gate() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..Default::default()
    });
    let submission = engine.begin_persistence_mutation_submission().unwrap();
    assert!(!submission.persistence_snapshot().unwrap().blackout);
    assert!(submission
        .set_output_blackout_published(Target::Both, true, Instant::now() + Duration::from_secs(2))
        .unwrap());
    let published = submission.persistence_snapshot().unwrap();
    assert!(published.blackout && published.video.blackout);
    assert!(!submission
        .set_output_blackout_published(Target::Both, true, Instant::now() + Duration::from_secs(2))
        .unwrap());
    drop(submission);
    assert!(engine
        .set_output_blackout_published(Target::Both, false, Instant::now() + Duration::from_secs(2))
        .unwrap());
}
