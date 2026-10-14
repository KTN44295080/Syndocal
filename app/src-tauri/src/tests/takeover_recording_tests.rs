use super::*;

fn lease(session_id: &str, output_id: u64) -> RecordingLeaseRecord {
    RecordingLeaseRecord {
        version: recording_lease::RECORDING_LEASE_VERSION,
        session_id: session_id.to_string(),
        take_id: "1".to_string(),
        sequence: 4,
        state: RecordingLeaseState::Recording,
        output_id,
        frame_rate: 30,
        audio_requested: true,
    }
}

fn plan() -> TakeoverPlan {
    let predecessors = vec![lease("100-1", 2)];
    TakeoverPlan {
        directory: PathBuf::from("unused"),
        takeover: recording_lease::takeover_record(
            "200-2".to_string(),
            vec!["100-1".to_string()],
            predecessors.clone(),
            false,
        ),
        watched_sessions: vec!["100-1".to_string()],
        predecessors,
        project_epoch: 9,
    }
}

fn observation<'a>(
    folder: Option<&'a str>,
    admission: Result<(), String>,
    predecessor: PredecessorState,
    receipt_error: Option<&'a str>,
) -> TakeoverObservation<'a> {
    TakeoverObservation {
        folder,
        project_epoch: 9,
        admission,
        predecessor,
        receipt_error,
    }
}

#[test]
fn the_takeover_take_starts_only_with_a_folder_arm_and_a_proven_end() {
    let plan = plan();
    let folder = Some("D:/takeover");
    let armed = || Ok(());
    // Nothing configured: blocked, never silent.
    assert!(matches!(
        decide(
            &plan,
            observation(None, armed(), PredecessorState::Ended, None)
        ),
        TakeoverStep::Wait(TakeoverRecordingState::Blocked, _)
    ));
    // Not armed yet: pending with the admission reason.
    assert_eq!(
        decide(
            &plan,
            observation(
                folder,
                Err("arm first".to_string()),
                PredecessorState::Ended,
                None
            )
        ),
        TakeoverStep::Wait(TakeoverRecordingState::Pending, "arm first".to_string())
    );
    // The old Primary still renews its lease, or it cannot be read: pending.
    assert!(matches!(
        decide(&plan, observation(folder, armed(), PredecessorState::Recording, None)),
        TakeoverStep::Wait(TakeoverRecordingState::Pending, reason) if reason.contains("previous Primary")
    ));
    assert!(matches!(
        decide(
            &plan,
            observation(folder, armed(), PredecessorState::Unknown("denied".to_string()), None)
        ),
        TakeoverStep::Wait(TakeoverRecordingState::Pending, reason) if reason.contains("denied")
    ));
    // An explicit end starts it even when the receipt is not written.
    assert_eq!(
        decide(
            &plan,
            observation(folder, armed(), PredecessorState::Ended, Some("io"))
        ),
        TakeoverStep::Start
    );
    // A quiet lease starts it only once the old Primary was told to stop.
    assert_eq!(
        decide(
            &plan,
            observation(folder, armed(), PredecessorState::Quiet, None)
        ),
        TakeoverStep::Start
    );
    assert!(matches!(
        decide(&plan, observation(folder, armed(), PredecessorState::Quiet, Some("io"))),
        TakeoverStep::Wait(TakeoverRecordingState::Blocked, reason) if reason.contains("not told to stop")
    ));
    // Another project cancels it whatever else holds.
    let mut other_project = observation(folder, armed(), PredecessorState::Ended, None);
    other_project.project_epoch = 10;
    assert!(matches!(
        decide(&plan, other_project),
        TakeoverStep::Cancel(_)
    ));
}

#[test]
fn every_recording_predecessor_is_waited_for_and_unreadable_ones_are_reported() {
    let directory = std::env::temp_dir().join(format!(
        "syndocal-takeover-predecessors-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let write = |record: &RecordingLeaseRecord| {
        std::fs::write(
            recording_lease::lease_path(&directory, &record.session_id),
            serde_json::to_vec(record).unwrap(),
        )
        .unwrap();
    };
    let mut ended = lease("100-3", 7);
    ended.state = RecordingLeaseState::Ended;
    write(&lease("100-1", 2));
    write(&lease("100-2", 5));
    write(&ended);
    std::fs::write(recording_lease::lease_path(&directory, "100-4"), b"{").unwrap();
    let sessions = ["100-1", "100-2", "100-3", "100-4", "100-5"].map(String::from);
    let (recording, unreadable) = recording_predecessors(&directory, &sessions);
    assert_eq!(
        recording
            .iter()
            .map(|lease| lease.session_id.as_str())
            .collect::<Vec<_>>(),
        ["100-1", "100-2"]
    );
    assert_eq!(
        unreadable
            .iter()
            .map(|(session, _)| session.as_str())
            .collect::<Vec<_>>(),
        ["100-4"]
    );

    // The probe reports Recording while any predecessor still renews.
    let plan = TakeoverPlan {
        directory: directory.clone(),
        watched_sessions: vec!["100-1".to_string(), "100-2".to_string()],
        predecessors: recording,
        takeover: plan().takeover,
        project_epoch: 9,
    };
    let mut probe = PredecessorProbe::default();
    let now = Instant::now();
    assert_eq!(probe.observe(&plan, now), PredecessorState::Recording);
    let mut first_ended = lease("100-1", 2);
    first_ended.state = RecordingLeaseState::Ended;
    write(&first_ended);
    let later = now + recording_lease::RECORDING_LEASE_CLEAR_AFTER;
    // 100-2 went quiet; 100-1 ended: quiet overall.
    assert_eq!(probe.observe(&plan, later), PredecessorState::Quiet);
    // An unreadable session is waited for too, and reported.
    let watched_unreadable = TakeoverPlan {
        watched_sessions: vec!["100-1".to_string(), "100-4".to_string()],
        ..plan.clone()
    };
    assert!(matches!(
        PredecessorProbe::default().observe(&watched_unreadable, later),
        PredecessorState::Unknown(_)
    ));
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn takeover_settings_default_to_no_folder_and_reject_unknown_versions() {
    let directory =
        std::env::temp_dir().join(format!("syndocal-takeover-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = settings_path(&directory);
    assert_eq!(load_settings(&path).unwrap().folder, None);
    std::fs::write(&path, br#"{"version":1,"folder":"D:/takeover"}"#).unwrap();
    assert_eq!(
        load_settings(&path).unwrap().folder.as_deref(),
        Some("D:/takeover")
    );
    std::fs::write(&path, br#"{"version":2,"folder":null}"#).unwrap();
    assert!(load_settings(&path).is_err());
    std::fs::write(&path, br#"{"version":1,"folder":null,"extra":true}"#).unwrap();
    assert!(load_settings(&path).is_err());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn a_superseded_or_cancelled_takeover_never_records_its_outcome() {
    let mut recorder = TakeoverRecorder::default();
    let pending = TakeoverRecordingStatus {
        state: TakeoverRecordingState::Pending,
        ..TakeoverRecordingStatus::default()
    };
    let first = recorder.supersede(pending.clone());
    let second = recorder.supersede(pending);
    // The first thread finishing late changes nothing.
    recorder.finish(first, TakeoverRecordingState::Started, None);
    assert_eq!(recorder.status.state, TakeoverRecordingState::Pending);
    recorder.cancel_pending();
    assert_eq!(recorder.status.state, TakeoverRecordingState::Cancelled);
    // The cancelled generation can no longer start.
    recorder.finish(second, TakeoverRecordingState::Started, None);
    assert_eq!(recorder.status.state, TakeoverRecordingState::Cancelled);
}

#[test]
fn takeover_settings_round_trip_atomically() {
    let directory =
        std::env::temp_dir().join(format!("syndocal-takeover-persist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let path = settings_path(&directory.join("nested"));
    persist_settings(&path, Some("D:/takeover".to_string())).unwrap();
    assert_eq!(
        load_settings(&path).unwrap().folder.as_deref(),
        Some("D:/takeover")
    );
    persist_settings(&path, None).unwrap();
    assert_eq!(load_settings(&path).unwrap().folder, None);
    let _ = std::fs::remove_dir_all(&directory);
}
