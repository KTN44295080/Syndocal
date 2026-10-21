// Included inside show_spout_transport::tests. No SDK or physical device is opened.
mod retirement_stop_tests {
    use super::*;
    use std::sync::mpsc;

    type LeaseSlot = Arc<Mutex<Option<engine::OutputOwnershipTeardownLease>>>;

    struct BlockedShowPair {
        state: Arc<Mutex<ShowSpoutTransportState>>,
        control: Arc<ShowSpoutWorkerControl>,
        activation: OutputOwnershipActivation,
        stops: [Arc<AtomicBool>; 2],
        leases: [LeaseSlot; 2],
        sdk_drops: Arc<AtomicUsize>,
        foreground_dropped: mpsc::Receiver<()>,
        release_drop: Option<mpsc::Sender<()>>,
        release_ack: Option<mpsc::Sender<()>>,
    }

    impl Drop for BlockedShowPair {
        fn drop(&mut self) {
            // Release both deterministic barriers before state-owned worker
            // destructors run, including when an assertion unwinds this test.
            if let Some(release) = self.release_drop.take() {
                let _ = release.send(());
            }
            if let Some(release) = self.release_ack.take() {
                let _ = release.send(());
            }
        }
    }

    impl BlockedShowPair {
        fn new(prior_failure: Option<&str>) -> Self {
            let control = Arc::new(control_for_tests(|| Ok(())));
            let engine = &control.engine;
            seed_show_spout_v2_compositions(engine);
            let safety = engine.safety_blackout_authority();
            engine.enable_show_spout_outputs_published(
                control.expected.background.clone(),
                control.expected.foreground.clone(),
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1),
            ).unwrap();
            control.arm_automatic_engine_retirement().unwrap();

            let activation = engine.admit_output_activation(protocol::MachineOutputRole::Both)
                .unwrap();
            // Exercise both worker-owned teardown slots and undelivered
            // creation leases. request_retirement must retain the latter when
            // its startup receiver is absent; consuming stop releases them.
            let background_creation = engine.admit_output_resource_creation(&activation).unwrap();
            let foreground_creation = engine.admit_output_resource_creation(&activation).unwrap();
            let leases = [
                Arc::new(Mutex::new(Some(engine.begin_video_output_resource_teardown().unwrap()))),
                Arc::new(Mutex::new(Some(engine.begin_video_output_resource_teardown().unwrap()))),
            ];
            let stops = [
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicBool::new(false)),
            ];
            let sdk_drops = Arc::new(AtomicUsize::new(0));
            let background_control = Arc::clone(&control);
            let background_drops = Arc::clone(&sdk_drops);
            let prior_failure = prior_failure.map(str::to_string);
            let thread_failure = prior_failure.clone();
            let background = std::thread::spawn(move || {
                background_drops.fetch_add(1, Ordering::AcqRel);
                background_control.note_physical_sender_drop();
                thread_failure.map_or(Ok(()), |message| {
                    Err(crate::spout_transport::SpoutOutputWorkerStopError { message })
                })
            });
            assert!(wait_for_output_worker(Duration::from_secs(1), || background.is_finished()));

            let (release_drop, await_drop) = mpsc::channel();
            let (release_ack, await_ack) = mpsc::channel();
            let (drop_observed, foreground_dropped) = mpsc::channel();
            let foreground_control = Arc::clone(&control);
            let foreground_drops = Arc::clone(&sdk_drops);
            let foreground_stops = [Arc::clone(&stops[0]), Arc::clone(&stops[1])];
            let foreground = std::thread::spawn(move || {
                // The first barrier models SDK Drop; the second models a
                // blocked post-drop engine acknowledgement. The actual exact
                // engine-retirement callback is run only after its release.
                await_drop.recv_timeout(Duration::from_secs(10)).unwrap();
                assert!(foreground_stops.iter().all(|stop| stop.load(Ordering::Acquire)));
                foreground_drops.fetch_add(1, Ordering::AcqRel);
                drop_observed.send(()).unwrap();
                await_ack.recv_timeout(Duration::from_secs(10)).unwrap();
                foreground_control.note_physical_sender_drop();
                Ok(())
            });

            let pair = ShowSpoutOutputPair {
                expected: control.expected.clone(),
                control: Arc::clone(&control),
                background: SpoutRouteWorker::retained_worker_for_retirement_tests(
                    control.expected.background.endpoint_name.clone().unwrap(),
                    Arc::clone(&stops[0]),
                    background,
                    prior_failure,
                    Arc::clone(&leases[0]),
                    Some(background_creation),
                ),
                foreground: SpoutRouteWorker::retained_worker_for_retirement_tests(
                    control.expected.foreground.endpoint_name.clone().unwrap(),
                    Arc::clone(&stops[1]),
                    foreground,
                    None,
                    Arc::clone(&leases[1]),
                    Some(foreground_creation),
                ),
            };
            let state = Arc::new(Mutex::new(ShowSpoutTransportState {
                active: Some(pair),
                ..ShowSpoutTransportState::default()
            }));
            Self {
                state, control, activation, stops, leases, sdk_drops,
                foreground_dropped,
                release_drop: Some(release_drop),
                release_ack: Some(release_ack),
            }
        }

        fn assert_pending(&self, kind: ShowSpoutReapingKind) {
            let mut state = self.state.try_lock()
                .expect("retirement must return without retaining the show-state mutex");
            assert!(state.active.is_none());
            assert_eq!(state.pending_retirements.len(), 1);
            let pending = &state.pending_retirements[0];
            assert_eq!(pending.reaping.kind, kind);
            assert_eq!(pending.reaping.expected, self.control.expected);
            assert!(Arc::ptr_eq(&pending.control, &self.control));
            assert!(pending.retirement_lease.is_some());
            let pair = pending.pair.as_ref().unwrap();
            assert!(pair.background.has_finished());
            assert!(!pair.foreground.has_finished());
            assert!(Arc::ptr_eq(&pair.background.stop_signal(), &self.stops[0]));
            assert!(Arc::ptr_eq(&pair.foreground.stop_signal(), &self.stops[1]));
            let signals = state.authority_change_stop_signals();
            assert_eq!(signals.len(), 2);
            for (actual, expected) in signals.iter().zip(&self.stops) {
                assert!(Arc::ptr_eq(actual, expected));
                assert!(actual.load(Ordering::Acquire));
            }
            assert!(self.leases.iter().all(|lease| lease.lock().unwrap().is_some()),
                "even the already-finished worker must keep its lease until both may be joined");
            assert!(state.blocked_engine_retirement.is_none(),
                "a running post-drop callback is not a failed or missing engine ACK");

            let mut generic = transport_for_blocked_retry();
            for expected in [
                self.control.expected.clone(),
                crate::show_spout_outputs::build_show_spout_outputs(101, 102, 3, 2).unwrap(),
            ] {
                let error = match state.prepare_show_spout_outputs(
                    &mut generic,
                    &self.control.engine,
                    expected,
                    self.activation.clone(),
                    || Ok(()),
                ) {
                    Err(error) => error,
                    Ok(_) => panic!("neither old nor freshly allocated IDs may reuse fixed names"),
                };
                assert!(error.contains("reaping"), "{error}");
                assert_eq!(generic.strict_show_startup_resource_count(), 0);
            }
            assert!(state.sync_content_state(true).unwrap_err().contains("reaping"));
            let started = Instant::now();
            assert!(state.take_failed_active_pair().is_err());
            assert!(state.take_active_for_authority_change().is_err());
            assert!(started.elapsed() < Duration::from_millis(500),
                "pending harvest must poll rather than wait with the state mutex held");
            assert_eq!(state.pending_retirements.len(), 1);
            assert!(self.control.engine.acquire_video_output().is_err());
        }

        fn release_sdk_drop(&mut self) {
            self.release_drop.take().unwrap().send(()).unwrap();
            self.foreground_dropped.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!(self.sdk_drops.load(Ordering::Acquire), 2);
            assert!(self.control.automatic_engine_retirement().is_none());
            assert_eq!(self.control.engine.video_outputs_snapshot().len(), 2);
        }

        fn release_engine_ack(&mut self) {
            self.release_ack.take().unwrap().send(()).unwrap();
            assert!(wait_for_output_worker(Duration::from_secs(3), || {
                self.state.lock().unwrap().pending_retirements[0].has_finished()
            }));
        }

        fn assert_reaped(&self) {
            let state = self.state.lock().unwrap();
            assert!(state.active.is_none());
            assert!(state.reaping.is_none());
            assert!(state.pending_retirements.is_empty());
            assert!(state.blocked_engine_retirement.is_none());
            assert!(self.leases.iter().all(|lease| lease.lock().unwrap().is_none()));
            assert_eq!(self.sdk_drops.load(Ordering::Acquire), 2);
            assert_eq!(self.control.teardown.lock().unwrap().physically_dropped_senders, 2);
            assert!(matches!(self.control.automatic_engine_retirement(), Some(Ok(()))));
            assert!(self.control.engine.video_outputs_snapshot().is_empty());
            state.ensure_preparation_admitted().unwrap();
            drop(state);
            // This also proves that undelivered creation and timeout failure
            // leases were retained and then released without leaking an owner.
            self.control.engine
                .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
                .unwrap().complete().unwrap();
            assert!(self.control.engine.acquire_video_output().is_ok());
        }
    }

    #[test]
    fn show_spout_retirement_active_deadline_retains_owners_through_sdk_drop_and_engine_ack() {
        let mut fixture = BlockedShowPair::new(None);
        let reservation = fixture.control.engine.reserve_output_ownership_retirement(
            protocol::MachineOutputRole::Standby,
            &fixture.state.lock().unwrap().authority_change_stop_signals(),
        ).unwrap();
        let retiring = fixture.state.lock().unwrap()
            .take_active_for_authority_change().unwrap().unwrap();
        let detached_signals = fixture.state.lock().unwrap().authority_change_stop_signals();
        assert_eq!(detached_signals.len(), 2);
        for (actual, expected) in detached_signals.iter().zip(&fixture.stops) {
            assert!(Arc::ptr_eq(actual, expected));
        }
        let started = Instant::now();
        let attempt = retiring.retire();
        assert!(started.elapsed() < Duration::from_secs(2),
            "the shared production one-second deadline must bound a blocked worker");
        assert!(attempt.pending.is_some());
        let failure = fixture.state.lock().unwrap().finish_retiring_pair(attempt).unwrap_err();
        assert!(failure.message.contains("stop timed out"));
        assert!(failure.automatic_engine_retirement.is_none());
        reservation.fail(failure.message);
        fixture.assert_pending(ShowSpoutReapingKind::AuthorityChange);

        // A stale completion must not clear the exact state-owned interlock
        // while the retained worker is still alive, even with matching IDs.
        {
            let mut state = fixture.state.lock().unwrap();
            let reaping = state.reaping.as_ref().unwrap().clone();
            let mut substituted = reaping.clone();
            substituted.stop_signals[0] = Arc::new(AtomicBool::new(true));
            let failure = state.finish_retiring_pair(ShowSpoutRetirementAttempt {
                expected: substituted.expected.clone(),
                reaping: substituted,
                worker_failure: None,
                cleanup: Ok(()),
                automatic_engine_retirement: Some(Ok(())),
                pending: None,
            }).unwrap_err();
            assert!(failure.message.contains("different state-owned reaping receipt"));
            let failure = state.finish_retiring_pair(ShowSpoutRetirementAttempt {
                expected: reaping.expected.clone(),
                reaping,
                worker_failure: None,
                cleanup: Ok(()),
                automatic_engine_retirement: Some(Ok(())),
                pending: None,
            }).unwrap_err();
            assert!(failure.message.contains("retirement remains pending"));
            assert_eq!(state.pending_retirements.len(), 1);
            assert!(state.reaping.is_some());
        }

        fixture.release_sdk_drop();
        fixture.assert_pending(ShowSpoutReapingKind::AuthorityChange);
        fixture.release_engine_ack();
        // A normal failure/status harvest must complete this original planned
        // authority retirement without inventing a new worker failure.
        let retiring = fixture.state.lock().unwrap().take_failed_active_pair().unwrap().unwrap();
        let attempt = retiring.retire_with_timeout(Duration::ZERO);
        assert!(attempt.pending.is_none());
        fixture.state.lock().unwrap().finish_retiring_pair(attempt).unwrap();
        fixture.assert_reaped();
        assert!(fixture.state.lock().unwrap().take_failed_active_pair().unwrap().is_none());
    }

    #[test]
    fn show_spout_retirement_failed_pair_preserves_prior_failure_and_original_reaping_identity() {
        let mut fixture = BlockedShowPair::new(Some("injected prior SDK send failure"));
        let retiring = fixture.state.lock().unwrap().take_failed_active_pair().unwrap().unwrap();
        let started = Instant::now();
        let attempt = retiring.retire_with_timeout(Duration::from_millis(15));
        assert!(started.elapsed() < Duration::from_secs(1));
        let failure = fixture.state.lock().unwrap().finish_retiring_pair(attempt).unwrap_err();
        assert!(failure.message.contains("injected prior SDK send failure"));
        assert!(failure.automatic_engine_retirement.is_none());
        fixture.assert_pending(ShowSpoutReapingKind::FailedWorker);
        fixture.release_sdk_drop();
        fixture.assert_pending(ShowSpoutReapingKind::FailedWorker);
        fixture.release_engine_ack();
        // Conversely, an authority-change caller must not erase the earlier
        // real SDK failure when it performs the eventual completed-pair join.
        let retiring = fixture.state.lock().unwrap()
            .take_active_for_authority_change().unwrap().unwrap();
        let attempt = retiring.retire_with_timeout(Duration::ZERO);
        assert!(attempt.pending.is_none());
        let failure = fixture.state.lock().unwrap().finish_retiring_pair(attempt).unwrap_err();
        assert!(failure.message.contains("injected prior SDK send failure"));
        assert!(matches!(failure.automatic_engine_retirement, Some(Ok(()))));
        fixture.assert_reaped();
        assert!(fixture.state.lock().unwrap().take_active_for_authority_change().unwrap().is_none());
    }
}
