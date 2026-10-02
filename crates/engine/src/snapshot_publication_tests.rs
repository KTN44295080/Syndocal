use super::*;

fn runtime() -> EngineRuntime {
    EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    })
}

#[test]
fn tap_snapshot_publication_ack_waits_for_shared_clock_and_stops_command_drain() {
    let mut runtime = runtime();
    runtime.clock.tap(Instant::now() - Duration::from_secs(1));
    let published = RwLock::new(runtime.build_snapshot(0));
    let queue = ArrayQueue::new(3);
    let (ack, receiver) = mpsc::sync_channel(1);
    for command in [
        EngineCommand::TapBpm,
        EngineCommand::RequestSnapshotPublication {
            expires_at: Instant::now() + Duration::from_secs(1),
            ack,
        },
        EngineCommand::SetBpm(150.0),
    ] {
        assert!(queue
            .push(QueuedEngineCommand {
                command,
                queued_at: Instant::now()
            })
            .is_ok());
    }
    runtime.consume_commands(&queue);
    assert_eq!(
        queue.len(),
        1,
        "later mutations cannot pass the publication barrier"
    );
    assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
    assert_eq!(published.read().unwrap().clock.bpm, 120.0);
    assert!((59.0..61.0).contains(&runtime.clock.bpm));
    runtime.publish_pending_command_acks(queue.len(), &published);
    assert_eq!(receiver.recv().unwrap(), Ok(()));
    assert_eq!(published.read().unwrap().clock.bpm, runtime.clock.bpm);
    assert_eq!(published.read().unwrap().clock.tap_count, 2);
}

#[test]
fn tap_snapshot_publication_expiry_does_not_change_clock_or_authority() {
    let mut runtime = runtime();
    let before = runtime.build_persistence_snapshot();
    let published = RwLock::new(runtime.build_snapshot(0));
    let (ack, receiver) = mpsc::sync_channel(1);
    let command = EngineCommand::RequestSnapshotPublication {
        expires_at: Instant::now() - Duration::from_secs(1),
        ack,
    };
    assert!(!command.mutates_persistence_snapshot());
    assert!(!command.mutates_video_presentation_configuration());
    runtime.apply_command(command);
    runtime.publish_pending_command_acks(0, &published);
    assert!(receiver.recv().unwrap().unwrap_err().contains("expired"));
    let after = runtime.build_persistence_snapshot();
    assert_eq!(before.clock.bpm, after.clock.bpm);
    assert_eq!(before.clock.tap_count, after.clock.tap_count);
    assert_eq!(before.clock.source, after.clock.source);
    assert_eq!(
        before.timeline.transport_generation,
        after.timeline.transport_generation
    );
}

#[test]
fn tap_snapshot_publication_failure_never_acknowledges_or_replays_the_tap() {
    let mut runtime = runtime();
    runtime.clock.tap(Instant::now() - Duration::from_secs(1));
    let published = RwLock::new(runtime.build_snapshot(0));
    runtime.apply_command(EngineCommand::TapBpm);
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::RequestSnapshotPublication {
        expires_at: Instant::now() + Duration::from_secs(1),
        ack,
    });
    runtime.fail_next_pending_publication = true;
    runtime.publish_pending_command_acks(0, &published);
    assert!(receiver.recv().unwrap().is_err());
    assert_eq!(
        runtime.clock.tap_count, 2,
        "failure must not replay the tap"
    );
    assert_eq!(published.read().unwrap().clock.bpm, 120.0);
}
