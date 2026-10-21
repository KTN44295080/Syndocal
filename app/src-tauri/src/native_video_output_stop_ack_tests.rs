use super::*;

#[test]
fn native_video_output_stop_during_fault_ack_preserves_original_failure_and_lease() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    let stop = Arc::new(AtomicBool::new(false));
    let teardown = NativeVideoOutputWorker::new_teardown_lease_slot();
    let permit = engine.acquire_video_output().unwrap();
    let original_epoch = engine.output_ownership_status().epoch;
    let original_error = "injected physical present failure".to_string();
    let error = native_video_output_worker_result(
        &stop,
        engine.output_ownership_status().state,
        Err(original_error.clone()),
    )
    .unwrap_err();
    let acknowledgement = TimelineFollowSettlementAck {
        epoch: original_epoch,
        generation: 73,
        domain: TimelineFollowSettlementDomain::Video,
        consumer_id: TimelineFollowSettlementConsumerId::VideoOutput { output_id: 96_480 },
        result: TimelineFollowSettlementAckResult::Fault {
            fault: original_error.clone(),
        },
    };
    let failure = retain_native_video_output_failure_before_ack(
        &engine,
        &teardown,
        "video-output-96480",
        error,
        Some(acknowledgement.clone()),
        |ack| {
            assert_eq!(engine.output_ownership_status().state, protocol::OutputOwnershipState::Failed);
            assert!(teardown.lock().unwrap().is_some());
            assert!(engine.acquire_video_output().is_err());
            assert_ne!(ack.epoch, original_epoch);
            assert_eq!(ack.epoch, engine.output_ownership_status().epoch);
            assert_eq!(ack.generation, acknowledgement.generation);
            assert_eq!(ack.consumer_id, acknowledgement.consumer_id);
            assert_eq!(ack.result, acknowledgement.result);
            // Model a stop arriving while the ACK waiter is still active.
            stop.store(true, Ordering::Release);
        },
    );
    let result = native_video_output_result_after_latched_failure(
        &stop,
        engine.output_ownership_status().state,
        Err(original_error.clone()),
        Some(failure),
    );
    assert_eq!(result.as_ref().unwrap_err(), &original_error);
    drop(permit);

    let join = std::thread::spawn(move || result);
    let mut worker = NativeVideoOutputWorker::with_teardown_lease(
        Arc::clone(&stop),
        join,
        Arc::clone(&teardown),
    );
    let error = worker.join_with_deadline("video-output-96480", Duration::from_secs(1)).unwrap_err();
    assert!(error.contains(&original_error));
    assert!(teardown.lock().unwrap().is_some(), "join alone is not native retirement ACK");
    assert!(engine.begin_output_ownership_transition(MachineOutputRole::Both).is_err());

    worker.release_teardown_lease_after_retirement_ack();
    assert!(teardown.lock().unwrap().is_none());
    engine.begin_output_ownership_transition(MachineOutputRole::Both).unwrap().complete().unwrap();
}
