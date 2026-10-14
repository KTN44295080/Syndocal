use super::*;

static NEXT_FOLDER: AtomicU64 = AtomicU64::new(0);

struct SyncFolder(PathBuf);

impl SyncFolder {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "syndocal-recording-lease-{label}-{}-{}",
            std::process::id(),
            NEXT_FOLDER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for SyncFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn take(take_id: &str) -> LeaseTake {
    LeaseTake {
        take_id: take_id.to_string(),
        output_id: 2,
        frame_rate: 30,
        audio_requested: true,
    }
}

fn primary(folder: &Path, session_id: &str) -> LeaseObservation {
    LeaseObservation {
        primary: Some((folder.to_path_buf(), session_id.to_string())),
        split_brain: Some(false),
    }
}

const SOLO: LeaseObservation = LeaseObservation {
    primary: None,
    split_brain: Some(false),
};

fn takeover(takeover_id: &str, sessions: &[&str]) -> RecordingTakeoverRecord {
    takeover_record(
        takeover_id.to_string(),
        sessions.iter().map(|session| session.to_string()).collect(),
        Vec::new(),
        true,
    )
}

#[test]
fn a_take_without_primary_sync_needs_no_lease() {
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    fence.renew(&SOLO, &take("1"), start);
    assert!(fence.check(start + Duration::from_secs(60)).is_ok());
}

#[test]
fn a_primary_take_renews_its_lease_and_self_fences_after_the_ttl() {
    let folder = SyncFolder::new("renew");
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    fence.renew(&primary(&folder.0, "100-1"), &take("1"), start);
    let lease = read_lease(&folder.0, "100-1").unwrap().unwrap();
    assert_eq!(
        (lease.sequence, lease.state),
        (1, RecordingLeaseState::Recording)
    );
    assert!(fence.check(start + RECORDING_LEASE_TTL).is_ok());
    // Losing the folder leaves the lease unrenewed; the take fences itself
    // and says why.
    fs::remove_dir_all(&folder.0).unwrap();
    fence.renew(
        &primary(&folder.0, "100-1"),
        &take("1"),
        start + Duration::from_secs(1),
    );
    let error = fence
        .check(start + RECORDING_LEASE_TTL + Duration::from_millis(1))
        .unwrap_err();
    assert!(
        error.contains("could not be renewed: Cannot read sync folder"),
        "{error}"
    );
}

#[test]
fn binding_mid_take_counts_as_a_renewal_until_the_ttl() {
    let folder = SyncFolder::new("mid-take");
    let missing = folder.0.join("not-yet-created");
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    fence.renew(&SOLO, &take("1"), start);
    // Sync starts mid-take in a folder whose first lease write fails: the
    // frame loop keeps recording until the TTL, then fences.
    fence.renew(&primary(&missing, "100-1"), &take("1"), start);
    assert!(fence.check(start).is_ok());
    assert!(fence.check(start + RECORDING_LEASE_TTL).is_ok());
    assert!(fence
        .check(start + RECORDING_LEASE_TTL + Duration::from_millis(1))
        .is_err());
}

#[test]
fn the_bound_folder_survives_stopping_sync() {
    let folder = SyncFolder::new("bound");
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    fence.renew(&primary(&folder.0, "100-1"), &take("1"), start);
    // Sync stopped: the take keeps renewing the session it bound.
    fence.renew(&SOLO, &take("1"), start + Duration::from_secs(1));
    assert_eq!(read_lease(&folder.0, "100-1").unwrap().unwrap().sequence, 2);
    assert!(fence.check(start + Duration::from_secs(3)).is_ok());
}

#[test]
fn a_restarted_sync_session_rebinds_and_a_takeover_of_either_session_ends_the_take() {
    let folder = SyncFolder::new("rebind");
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    fence.renew(&primary(&folder.0, "100-1"), &take("1"), start);
    fence.renew(
        &primary(&folder.0, "100-2"),
        &take("1"),
        start + Duration::from_secs(1),
    );
    // The old session's lease is left to go quiet (the take still runs);
    // the new one records.
    let old = read_lease(&folder.0, "100-1").unwrap().unwrap();
    assert_eq!(
        (old.sequence, old.state),
        (1, RecordingLeaseState::Recording)
    );
    assert_eq!(
        read_lease(&folder.0, "100-2").unwrap().unwrap().state,
        RecordingLeaseState::Recording
    );
    // A Standby that still names only the first session ends the take.
    write_takeover(&folder.0, &takeover("200-2", &["100-1"])).unwrap();
    fence.renew(
        &primary(&folder.0, "100-2"),
        &take("1"),
        start + Duration::from_secs(2),
    );
    assert!(fence.check(start).unwrap_err().contains("took over"));
    // When the take really ends, every lease it renewed says so.
    fence.end(&take("1"));
    for session in ["100-1", "100-2"] {
        assert_eq!(
            read_lease(&folder.0, session).unwrap().unwrap().state,
            RecordingLeaseState::Ended
        );
    }
}

#[test]
fn a_takeover_receipt_in_an_earlier_sync_folder_still_ends_the_take() {
    let first = SyncFolder::new("first-folder");
    let second = SyncFolder::new("second-folder");
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    fence.renew(&primary(&first.0, "100-1"), &take("1"), start);
    fence.renew(
        &primary(&second.0, "100-2"),
        &take("1"),
        start + Duration::from_secs(1),
    );
    write_takeover(&first.0, &takeover("200-2", &["100-1"])).unwrap();
    fence.renew(
        &primary(&second.0, "100-2"),
        &take("1"),
        start + Duration::from_secs(2),
    );
    assert!(fence.check(start).unwrap_err().contains("took over"));
}

#[test]
fn only_a_persistent_split_brain_ends_the_take() {
    let folder = SyncFolder::new("split");
    let fence = RecordingLeaseFence::default();
    let start = Instant::now();
    let split = LeaseObservation {
        primary: Some((folder.0.clone(), "100-1".to_string())),
        split_brain: Some(true),
    };
    let unknown = LeaseObservation {
        primary: None,
        split_brain: None,
    };
    // A starting Primary briefly reports old sessions as active.
    fence.renew(&split, &take("1"), start);
    fence.renew(&split, &take("1"), start + Duration::from_secs(6));
    fence.renew(
        &primary(&folder.0, "100-1"),
        &take("1"),
        start + Duration::from_secs(7),
    );
    assert!(fence.check(start + Duration::from_secs(7)).is_ok());
    // A split-brain that persists ends it; a busy sync lock in between
    // (unknown) does not restart the wait.
    let later = start + Duration::from_secs(8);
    fence.renew(&split, &take("1"), later);
    fence.renew(&unknown, &take("1"), later + Duration::from_secs(5));
    fence.renew(&split, &take("1"), later + SPLIT_BRAIN_FENCE_AFTER);
    assert!(fence
        .check(later + SPLIT_BRAIN_FENCE_AFTER)
        .unwrap_err()
        .contains("Two Primary"));
}

#[test]
fn takeover_receipts_are_never_replaced_and_old_ones_are_pruned() {
    let folder = SyncFolder::new("immutable");
    let record = takeover("200-2", &["100-1"]);
    write_takeover(&folder.0, &record).unwrap();
    assert!(write_takeover(&folder.0, &record).is_err());
    let mut invalid = record.clone();
    invalid.takeover_id = "../x".to_string();
    assert!(write_takeover(&folder.0, &invalid).is_err());
    // A receipt older than the retention goes when a new one is written.
    let mut old = takeover("150-1", &["90-1"]);
    old.written_at_unix_ms = 1;
    write_takeover(&folder.0, &old).unwrap();
    write_takeover(&folder.0, &takeover("300-3", &["200-2"])).unwrap();
    assert!(takeover_superseding(&folder.0, &["90-1".to_string()])
        .unwrap()
        .is_none());
    assert!(takeover_superseding(&folder.0, &["100-1".to_string()])
        .unwrap()
        .is_some());
}

#[test]
fn the_new_owner_tells_ended_from_quiet_recording_and_unreadable() {
    let start = Instant::now();
    // An absent lease is quiet only after the same wait.
    let mut absent = PredecessorWatch::default();
    assert_eq!(absent.observe(Ok(None), start), PredecessorState::Recording);
    assert_eq!(
        absent.observe(Ok(None), start + RECORDING_LEASE_CLEAR_AFTER),
        PredecessorState::Quiet
    );
    let mut watch = PredecessorWatch::default();
    assert!(matches!(
        watch.observe(Err("denied".to_string()), start),
        PredecessorState::Unknown(_)
    ));
    let mut lease = RecordingLeaseRecord {
        version: RECORDING_LEASE_VERSION,
        session_id: "100-1".to_string(),
        take_id: "1".to_string(),
        sequence: 1,
        state: RecordingLeaseState::Recording,
        output_id: 2,
        frame_rate: 30,
        audio_requested: false,
    };
    assert_eq!(
        watch.observe(Ok(Some(&lease)), start),
        PredecessorState::Recording
    );
    // Still renewing: never quiet, however long it runs.
    for second in 1..=10 {
        lease.sequence += 1;
        let now = start + Duration::from_secs(second);
        assert_eq!(
            watch.observe(Ok(Some(&lease)), now),
            PredecessorState::Recording
        );
    }
    let quiet = start + Duration::from_secs(10);
    assert_eq!(
        watch.observe(
            Ok(Some(&lease)),
            quiet + RECORDING_LEASE_CLEAR_AFTER - Duration::from_millis(1)
        ),
        PredecessorState::Recording
    );
    assert_eq!(
        watch.observe(Ok(Some(&lease)), quiet + RECORDING_LEASE_CLEAR_AFTER),
        PredecessorState::Quiet
    );
    // Only an explicit end proves the take ended.
    lease.state = RecordingLeaseState::Ended;
    assert_eq!(
        PredecessorWatch::default().observe(Ok(Some(&lease)), start),
        PredecessorState::Ended
    );
    assert!(RECORDING_LEASE_CLEAR_AFTER > RECORDING_LEASE_TTL);
}

#[test]
fn ending_a_take_marks_its_lease_ended_but_never_a_newer_take() {
    let folder = SyncFolder::new("end");
    let observation = primary(&folder.0, "100-1");
    let first = RecordingLeaseKeeper::start(take("1"), {
        let observation = observation.clone();
        Box::new(move || observation.clone())
    })
    .unwrap();
    drop(first);
    let lease = read_lease(&folder.0, "100-1").unwrap().unwrap();
    assert_eq!(
        (lease.take_id.as_str(), lease.state),
        ("1", RecordingLeaseState::Ended)
    );

    let second = RecordingLeaseFence::default();
    second.renew(&observation, &take("2"), Instant::now());
    // A late end from the first take leaves the second take's lease alone.
    let stale = RecordingLeaseFence::default();
    stale.renew(&observation, &take("1"), Instant::now());
    second.renew(&observation, &take("2"), Instant::now());
    stale.end(&take("1"));
    let lease = read_lease(&folder.0, "100-1").unwrap().unwrap();
    assert_eq!(
        (lease.take_id.as_str(), lease.state),
        ("2", RecordingLeaseState::Recording)
    );
}

#[test]
fn a_take_already_superseded_or_unwritable_never_starts() {
    let folder = SyncFolder::new("start");
    write_takeover(&folder.0, &takeover("200-2", &["100-1"])).unwrap();
    let superseded = RecordingLeaseKeeper::start(take("1"), {
        let observation = primary(&folder.0, "100-1");
        Box::new(move || observation.clone())
    });
    assert!(superseded.err().unwrap().contains("took over"));
    let missing = folder.0.join("missing");
    let unwritable = RecordingLeaseKeeper::start(take("2"), {
        let observation = primary(&missing, "100-3");
        Box::new(move || observation.clone())
    });
    assert!(unwritable.err().unwrap().contains("could not be written"));
}
