// These state-ownership regressions use already-completed workers. The
// separate project-retirement worker regressions run the real SDK-fake loop.
fn completed_pair_for_harvest_test(worker_error: bool) -> ShowSpoutTransportState {
    let control = Arc::new(control_for_tests(|| Ok(())));
    // Represent the completed exact engine-retirement acknowledgement; this
    // test concerns which caller owns the consuming join/finish receipt.
    control.teardown.lock().unwrap().automatic_engine_retirement = Some(Ok(()));
    let expected = control.expected.clone();
    let mut state = ShowSpoutTransportState::default();
    state.active = Some(ShowSpoutOutputPair {
        expected,
        control,
        background: SpoutRouteWorker::completed_worker_for_retirement_tests(None),
        foreground: SpoutRouteWorker::completed_worker_for_retirement_tests(
            worker_error.then(|| "injected prior SDK worker failure".to_string()),
        ),
    });
    state
}

#[test]
fn project_retirement_spout_planned_pair_is_not_stolen_by_failure_harvest() {
    for worker_error in [false, true] {
        // Also observe between the gate's first and second atomic stop store.
        for stop_count in [1, 2] {
            let mut state = completed_pair_for_harvest_test(worker_error);
            assert!(state.active.as_ref().unwrap().has_failed_worker());
            for signal in state.authority_change_stop_signals().iter().take(stop_count) {
                signal.store(true, Ordering::Release);
            }
            assert!(state.take_failed_active_pair().unwrap().is_none());
            assert!(state.active.is_some());
            assert!(state.reaping.is_none());
            let owned = state.take_active_for_authority_change().unwrap().unwrap();
            assert!(state.active.is_none());
            let retired = owned.retire();
            assert_eq!(retired.cleanup.is_err(), worker_error);
            let finished = state.finish_active_for_authority_change(retired);
            if worker_error {
                assert!(finished.unwrap_err().message.contains("injected prior SDK worker failure"));
            } else {
                finished.unwrap();
            }
            assert!(state.active.is_none());
            assert!(state.reaping.is_none());
        }
    }
}

#[test]
fn project_retirement_spout_unplanned_worker_failure_is_still_harvested() {
    let mut state = completed_pair_for_harvest_test(true);
    let failed = state.take_failed_active_pair().unwrap().unwrap();
    let result = state.finish_failed_active_pair(failed.retire());
    assert!(result.message.contains("injected prior SDK worker failure"));
    assert!(state.active.is_none());
    assert!(state.reaping.is_none());
}
