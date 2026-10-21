use super::*;
use std::cell::Cell;

fn engine_and_key() -> (EngineHandle, TimelineFollowOutputFrameKey) {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    let key = TimelineFollowOutputFrameKey {
        epoch: engine.output_ownership_status().epoch,
        generation: 1,
        output_id: 96_450,
        width: 4,
        height: 2,
    };
    (engine, key)
}

fn results() -> [TimelineFollowSettlementAckResult; 3] {
    [
        TimelineFollowSettlementAckResult::Applied,
        TimelineFollowSettlementAckResult::NotApplicable,
        TimelineFollowSettlementAckResult::Fault {
            fault: "earlier physical failure".to_string(),
        },
    ]
}

#[test]
fn follow_stop_before_ack_never_queues_or_publishes_any_result() {
    let (engine, key) = engine_and_key();
    for result in results() {
        let mut state = TimelineFollowOutputState::default();
        publish_timeline_follow_output_result_until_resolved_with(
            &engine,
            &mut state,
            key,
            result.clone(),
            || true,
            |_| panic!("a stopped worker must never issue a Follow ACK"),
        );
        assert!(state.pending_ack.is_none());
        assert!(!state.published_for(key));

        assert!(!publish_timeline_follow_output_result(
            &engine,
            &mut state,
            key,
            result,
            &Arc::new(AtomicBool::new(true)),
        )
        .unwrap());
        assert!(state.pending_ack.is_none());
        assert!(!state.published_for(key));
    }
}

#[test]
fn follow_stop_at_publish_boundary_retains_unpublished_ack() {
    let (engine, key) = engine_and_key();
    for result in results() {
        let mut state = TimelineFollowOutputState::default();
        let checks = Cell::new(0);
        publish_timeline_follow_output_result_until_resolved_with(
            &engine,
            &mut state,
            key,
            result.clone(),
            || {
                let previous = checks.get();
                checks.set(previous + 1);
                previous != 0
            },
            |_| panic!("stop after queueing must prevent actual publication"),
        );
        assert!(!state.published_for(key));
        assert_eq!(
            state.pending_ack.as_ref().unwrap().ack,
            key.settlement_ack(result),
        );
    }
}

#[test]
fn follow_stop_before_retry_never_reissues_pending_ack() {
    let (engine, key) = engine_and_key();
    for result in results() {
        let mut state = TimelineFollowOutputState::default();
        let attempts = Cell::new(0);
        let checked_after_publish = Cell::new(false);
        publish_timeline_follow_output_result_until_resolved_with(
            &engine,
            &mut state,
            key,
            result.clone(),
            || {
                // The first post-publication observation allows a retry wait.
                // Stop is visible on the next observation, before that retry.
                attempts.get() != 0 && checked_after_publish.replace(true)
            },
            |ack| {
                assert_eq!(*ack, key.settlement_ack(result.clone()));
                attempts.set(attempts.get() + 1);
                Err("injected lost ACK reply".to_string())
            },
        );
        assert_eq!(attempts.get(), 1, "stop must prevent the next ACK attempt");
        assert!(!state.published_for(key));
        assert_eq!(
            state.pending_ack.as_ref().unwrap().ack,
            key.settlement_ack(result),
        );
    }
}

#[test]
fn follow_stop_at_single_shot_boundary_does_not_report_publication() {
    let (_engine, key) = engine_and_key();
    for result in results() {
        let mut state = TimelineFollowOutputState::default();
        let checks = Cell::new(0);
        assert!(!publish_timeline_follow_output_result_with(
            &mut state,
            key,
            result.clone(),
            || {
                let previous = checks.get();
                checks.set(previous + 1);
                previous != 0
            },
            |_| panic!("stop after single-shot queueing must prevent publication"),
        )
        .unwrap());
        assert!(!state.published_for(key));
        assert_eq!(
            state.pending_ack.as_ref().unwrap().ack,
            key.settlement_ack(result),
        );
    }
}
