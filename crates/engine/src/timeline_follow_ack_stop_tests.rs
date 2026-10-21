use super::*;
use crate::timeline_follow_ack::{wait_for_publication, wait_for_publication_until};
use std::cell::Cell;

fn terminal_results() -> [TimelineFollowSettlementAckResult; 3] {
    [
        TimelineFollowSettlementAckResult::Applied,
        TimelineFollowSettlementAckResult::NotApplicable,
        TimelineFollowSettlementAckResult::Fault {
            fault: "physical output failed before Stop".to_string(),
        },
    ]
}

fn audio_ack(
    runtime: &EngineRuntime,
    result: TimelineFollowSettlementAckResult,
) -> TimelineFollowSettlementAck {
    TimelineFollowSettlementAck {
        epoch: runtime.output_ownership_gate.status().epoch,
        generation: runtime.timeline_follow_runtime.generation,
        domain: TimelineFollowSettlementDomain::Audio,
        consumer_id: TimelineFollowSettlementConsumerId::Audio,
        result,
    }
}

fn take_queued_command(engine: &EngineHandle) -> EngineCommand {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(queued) = engine.queue.pop() {
            return queued.command;
        }
        assert!(
            Instant::now() < deadline,
            "the handle did not enqueue its Follow acknowledgement"
        );
        thread::yield_now();
    }
}

#[test]
fn follow_ack_stopped_handle_does_not_enqueue_any_terminal_result() {
    let runtime = timeline_follow_settling_runtime(
        protocol::TimelineFollowFaultPolicy::Hold,
        Instant::now(),
    );
    let engine = allocator_test_handle(Arc::new(RwLock::new(runtime.build_snapshot(0))));
    let stop = Arc::new(AtomicBool::new(true));
    for result in terminal_results() {
        let error = engine
            .acknowledge_timeline_follow_settlement_published_with_stop(
                audio_ack(&runtime, result),
                Instant::now() + Duration::from_secs(1),
                Arc::clone(&stop),
            )
            .unwrap_err();
        assert!(error.contains("stopped"));
        assert!(engine.queue.is_empty());
    }
}

#[test]
fn follow_ack_queued_stop_rejects_each_result_before_runtime_mutation() {
    for result in terminal_results() {
        let mut runtime = timeline_follow_settling_runtime(
            protocol::TimelineFollowFaultPolicy::Hold,
            Instant::now(),
        );
        let before = runtime.timeline_follow_runtime.clone();
        let before_revision = runtime.timeline_audio_transport_revision;
        let published = Arc::new(RwLock::new(runtime.build_snapshot(0)));
        // This handle has no worker. The test explicitly controls when its
        // actual queued command reaches the runtime.
        let engine = allocator_test_handle(Arc::clone(&published));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_engine = engine.clone();
        let acknowledgement = audio_ack(&runtime, result);
        let waiter = thread::spawn(move || {
            worker_engine.acknowledge_timeline_follow_settlement_published_with_stop(
                acknowledgement,
                Instant::now() + Duration::from_secs(2),
                worker_stop,
            )
        });
        let command = take_queued_command(&engine);
        match &command {
            EngineCommand::AcknowledgeTimelineFollowSettlement {
                stop: Some(queued_stop),
                ..
            } => assert!(Arc::ptr_eq(queued_stop, &stop)),
            _ => panic!("the queued acknowledgement lost its exact worker Stop token"),
        }
        stop.store(true, Ordering::Release);
        runtime.apply_command(command);
        runtime.publish_pending_command_acks(0, published.as_ref());
        assert!(waiter.join().unwrap().unwrap_err().contains("stopped"));
        assert_eq!(runtime.timeline_follow_runtime, before);
        assert_eq!(runtime.timeline_audio_transport_revision, before_revision);
        assert!(runtime.timeline_follow_terminal_settlement_receipt.is_none());
        assert!(runtime
            .timeline_follow_transition
            .as_ref()
            .unwrap()
            .accepted_settlement_acknowledgements
            .is_empty());
    }
}

#[test]
fn follow_ack_stop_during_allocator_contention_does_not_wait_for_gate() {
    let runtime = timeline_follow_settling_runtime(
        protocol::TimelineFollowFaultPolicy::Hold,
        Instant::now(),
    );
    let engine = allocator_test_handle(Arc::new(RwLock::new(runtime.build_snapshot(0))));
    let allocator_guard = engine.allocator_gate.lock().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_engine = engine.clone();
    let acknowledgement = audio_ack(&runtime, TimelineFollowSettlementAckResult::Applied);
    let (returned, result) = mpsc::sync_channel(1);
    let waiter = thread::spawn(move || {
        let outcome = worker_engine.acknowledge_timeline_follow_settlement_published_with_stop(
            acknowledgement,
            Instant::now() + Duration::from_secs(2),
            worker_stop,
        );
        let _ = returned.send(outcome);
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    let command = loop {
        if let Some(queued) = engine.queue.pop() {
            break Some(queued.command);
        }
        if Instant::now() >= deadline {
            break None;
        }
        thread::yield_now();
    };
    stop.store(true, Ordering::Release);
    let stopped_while_gate_held = result.recv_timeout(Duration::from_secs(1));
    // Always release and join before assertions, including a regression that
    // blocks in the ordinary allocator path. No test worker is abandoned.
    drop(allocator_guard);
    waiter.join().unwrap();
    assert!(stopped_while_gate_held.unwrap().unwrap_err().contains("stopped"));
    match command.expect("runtime-only ACK must enqueue while the allocator gate is held") {
        EngineCommand::AcknowledgeTimelineFollowSettlement {
            stop: Some(queued_stop),
            ..
        } => assert!(Arc::ptr_eq(&queued_stop, &stop)),
        _ => panic!("the stop-aware ACK lost its exact worker Stop token"),
    }
}

#[test]
fn follow_ack_stop_during_wait_returns_at_the_next_short_slice() {
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let now = Cell::new(start);
    let mut waits = Vec::new();
    let result = wait_for_publication_until(
        &stop,
        start + Duration::from_secs(3),
        || Err(mpsc::TryRecvError::Empty),
        |timeout| {
            waits.push(timeout);
            now.set(now.get() + timeout);
            stop.store(true, Ordering::Release);
            Err(mpsc::RecvTimeoutError::Timeout)
        },
        || now.get(),
    );
    assert!(result.unwrap_err().contains("stopped"));
    assert_eq!(waits, vec![Duration::from_millis(10)]);
}

#[test]
fn follow_ack_wait_keeps_one_deadline_and_clips_the_final_slice() {
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let now = Cell::new(start);
    let mut waits = Vec::new();
    let result = wait_for_publication_until(
        &stop,
        start + Duration::from_millis(25),
        || Err(mpsc::TryRecvError::Empty),
        |timeout| {
            waits.push(timeout);
            now.set(now.get() + timeout);
            Err(mpsc::RecvTimeoutError::Timeout)
        },
        || now.get(),
    );
    assert_eq!(
        result.unwrap_err(),
        format!(
            "Timeline Follow settlement acknowledgement failed: {}",
            mpsc::RecvTimeoutError::Timeout
        )
    );
    assert_eq!(
        waits,
        vec![
            Duration::from_millis(10),
            Duration::from_millis(10),
            Duration::from_millis(5),
        ]
    );
}

#[test]
fn follow_ack_ready_definitive_reply_wins_over_stop() {
    for expected in [Ok(()), Err("earlier publication failure".to_string())] {
        let (ack, receiver) = mpsc::sync_channel(1);
        ack.send(expected.clone()).unwrap();
        let stop = AtomicBool::new(true);
        assert_eq!(wait_for_publication(&receiver, Some(&stop)), expected);
    }
}

#[test]
fn follow_ack_reply_arriving_during_stopped_slice_remains_definitive() {
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let result = wait_for_publication_until(
        &stop,
        start + Duration::from_secs(3),
        || Err(mpsc::TryRecvError::Empty),
        |_| {
            stop.store(true, Ordering::Release);
            Ok(Ok(()))
        },
        || start,
    );
    assert_eq!(result, Ok(()));
    assert!(stop.load(Ordering::Acquire));
}

#[test]
fn follow_ack_disconnected_reply_preserves_the_error_contract() {
    for stop in [None, Some(AtomicBool::new(false))] {
        let (ack, receiver) = mpsc::sync_channel::<Result<(), String>>(1);
        drop(ack);
        assert_eq!(
            wait_for_publication(&receiver, stop.as_ref()).unwrap_err(),
            format!(
                "Timeline Follow settlement acknowledgement failed: {}",
                mpsc::RecvTimeoutError::Disconnected
            )
        );
    }
}

#[test]
fn follow_ack_applied_receipt_survives_stopped_waiter_and_publication_failure() {
    let mut runtime = timeline_follow_settling_runtime(
        protocol::TimelineFollowFaultPolicy::Hold,
        Instant::now(),
    );
    let published = RwLock::new(runtime.build_snapshot(0));
    let acknowledgement = audio_ack(&runtime, TimelineFollowSettlementAckResult::Applied);
    let stop = Arc::new(AtomicBool::new(false));
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::AcknowledgeTimelineFollowSettlement {
        settlement_ack: acknowledgement.clone(),
        expires_at: Instant::now() + Duration::from_secs(1),
        stop: Some(Arc::clone(&stop)),
        ack,
    });
    assert!(runtime.pending_command_acks.last().unwrap().result.is_ok());
    assert_eq!(runtime.timeline_id, TimelineId(8_102));
    assert!(runtime.timeline_follow_transition.is_none());
    let completed = runtime.timeline_follow_runtime.clone();
    let installed_revision = runtime.timeline_audio_transport_revision;
    assert!(runtime
        .timeline_follow_terminal_settlement_receipt
        .as_ref()
        .unwrap()
        .acknowledgements
        .contains(&acknowledgement));

    stop.store(true, Ordering::Release);
    assert!(wait_for_publication(&receiver, Some(stop.as_ref()))
        .unwrap_err()
        .contains("stopped"));
    drop(receiver);
    runtime.fail_next_pending_publication = true;
    runtime.publish_pending_command_acks(0, &published);
    assert_eq!(runtime.timeline_follow_runtime, completed);
    assert_eq!(runtime.timeline_audio_transport_revision, installed_revision);

    // An ordinary retry still recovers the accepted receipt after both Stop
    // and publication loss; it cannot install the destination a second time.
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::AcknowledgeTimelineFollowSettlement {
        settlement_ack: acknowledgement.clone(),
        expires_at: Instant::now() + Duration::from_secs(1),
        stop: None,
        ack,
    });
    runtime.publish_pending_command_acks(0, &published);
    assert_eq!(receiver.try_recv().unwrap(), Ok(()));
    assert_eq!(runtime.timeline_follow_runtime, completed);
    assert_eq!(runtime.timeline_audio_transport_revision, installed_revision);
    assert_eq!(published.read().unwrap().timeline.id, TimelineId(8_102));
    assert!(runtime
        .timeline_follow_terminal_settlement_receipt
        .as_ref()
        .unwrap()
        .acknowledgements
        .contains(&acknowledgement));
}

#[test]
fn follow_ack_expired_command_still_rejects_without_mutation() {
    let mut runtime = timeline_follow_settling_runtime(
        protocol::TimelineFollowFaultPolicy::Hold,
        Instant::now(),
    );
    let before = runtime.timeline_follow_runtime.clone();
    let published = RwLock::new(runtime.build_snapshot(0));
    let acknowledgement = audio_ack(&runtime, TimelineFollowSettlementAckResult::Applied);
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::AcknowledgeTimelineFollowSettlement {
        settlement_ack: acknowledgement,
        expires_at: Instant::now() - Duration::from_millis(1),
        stop: Some(Arc::new(AtomicBool::new(false))),
        ack,
    });
    runtime.publish_pending_command_acks(0, &published);
    assert!(receiver.try_recv().unwrap().unwrap_err().contains("expired"));
    assert_eq!(runtime.timeline_follow_runtime, before);
    assert!(runtime.timeline_follow_terminal_settlement_receipt.is_none());
}

#[test]
fn follow_ack_legacy_handle_still_publishes_without_a_stop_token() {
    let mut runtime = timeline_follow_settling_runtime(
        protocol::TimelineFollowFaultPolicy::Hold,
        Instant::now(),
    );
    let published = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let engine = allocator_test_handle(Arc::clone(&published));
    let worker_engine = engine.clone();
    let acknowledgement = audio_ack(&runtime, TimelineFollowSettlementAckResult::Applied);
    let waiter = thread::spawn(move || {
        worker_engine.acknowledge_timeline_follow_settlement_published(
            acknowledgement,
            Instant::now() + Duration::from_secs(2),
        )
    });
    let command = take_queued_command(&engine);
    assert!(matches!(
        &command,
        EngineCommand::AcknowledgeTimelineFollowSettlement { stop: None, .. }
    ));
    runtime.apply_command(command);
    runtime.publish_pending_command_acks(0, published.as_ref());
    assert_eq!(waiter.join().unwrap(), Ok(()));
    assert_eq!(published.read().unwrap().timeline.id, TimelineId(8_102));
}
