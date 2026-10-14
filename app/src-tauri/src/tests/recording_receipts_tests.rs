use super::*;

struct TempJournal(PathBuf);

impl TempJournal {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "syndocal-recording-receipts-{label}-{}-{}",
            std::process::id(),
            NEXT_TAKE_SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempJournal {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SMALL: JournalLimits = JournalLimits {
    max_receipts: 64,
    retained_completed_takes: 2,
};

fn started(take_id: &str) -> TakeStartedReceipt {
    TakeStartedReceipt {
        version: RECORDING_RECEIPT_VERSION,
        take_id: take_id.to_string(),
        owner_epoch: 7,
        output_id: 2,
        target_path: "C:/recordings/take.mp4".to_string(),
        staging_path: "C:/recordings/.syndocal-recording-1.partial.mp4".to_string(),
        started_unix_ms: 1_000,
        process_id: 42,
    }
}

fn completed_take(journal: &TempJournal, started_unix_ms: u64) -> String {
    let take_id = new_take_id(started_unix_ms);
    TakeJournal::begin(&journal.0, &started(&take_id))
        .unwrap()
        .end(TakeEndReason::OperatorStop, true, 1, None)
        .unwrap();
    take_id
}

fn orphan_take(journal: &TempJournal, started_unix_ms: u64) -> String {
    let take_id = new_take_id(started_unix_ms);
    let take = TakeJournal::begin(&journal.0, &started(&take_id)).unwrap();
    // A crash or power loss: neither end nor the drop guard runs.
    std::mem::forget(take);
    take_id
}

fn read_ended(directory: &Path, take_id: &str) -> TakeEndedReceipt {
    serde_json::from_slice(&fs::read(directory.join(ended_name(take_id))).unwrap()).unwrap()
}

#[test]
fn take_ids_sort_in_start_order() {
    let early = new_take_id(9);
    let late = new_take_id(10);
    assert!(valid_take_id(&early) && valid_take_id(&late));
    assert!(early < late, "{early} {late}");
    assert_eq!(early.len(), late.len());
    assert_eq!(take_start_unix_ms(Some(5)), 5);
    assert!(take_start_unix_ms(None) > 0);
}

#[test]
fn take_receipts_are_immutable_and_end_exactly_once() {
    let journal = TempJournal::new("immutable");
    let take_id = new_take_id(1_000);
    let take = TakeJournal::begin(&journal.0, &started(&take_id)).unwrap();
    assert_eq!(take.take_id, take_id);
    // A second start with the same id never overwrites the first receipt.
    assert!(TakeJournal::begin(&journal.0, &started(&take_id)).is_err());
    take.end(TakeEndReason::OwnershipRevoked, true, 40, None)
        .unwrap();
    let ended = read_ended(&journal.0, &take_id);
    assert_eq!(ended.end_reason, TakeEndReason::OwnershipRevoked);
    assert_eq!(ended.published, Some(true));
    assert_eq!(ended.frames_written, Some(40));
    // Recovery leaves a completed take alone.
    assert!(recover_interrupted_takes(&journal.0, SMALL)
        .unwrap()
        .is_empty());
    assert_eq!(read_ended(&journal.0, &take_id), ended);
}

#[test]
fn a_take_whose_start_receipt_cannot_be_written_is_refused() {
    let journal = TempJournal::new("refused");
    let missing = journal.0.join("missing-directory");
    assert!(TakeJournal::begin(&missing, &started(&new_take_id(1_000))).is_err());
    let mut invalid = started("../escape");
    assert!(TakeJournal::begin(&journal.0, &invalid).is_err());
    invalid.take_id = new_take_id(1_000);
    invalid.version = RECORDING_RECEIPT_VERSION + 1;
    assert!(TakeJournal::begin(&journal.0, &invalid).is_err());
    assert_eq!(fs::read_dir(&journal.0).unwrap().count(), 0);
}

#[test]
fn a_worker_that_unwinds_still_ends_its_take_as_a_fault_of_unknown_outcome() {
    let journal = TempJournal::new("panic");
    let take_id = new_take_id(1_000);
    let worker_journal = journal.0.clone();
    let worker_take_id = take_id.clone();
    let result = std::thread::spawn(move || {
        let _take = TakeJournal::begin(&worker_journal, &started(&worker_take_id)).unwrap();
        panic!("injected recording worker panic");
    })
    .join();
    assert!(result.is_err());
    let ended = read_ended(&journal.0, &take_id);
    assert_eq!(ended.end_reason, TakeEndReason::Fault);
    assert_eq!((ended.published, ended.frames_written), (None, None));
    assert!(recover_interrupted_takes(&journal.0, SMALL)
        .unwrap()
        .is_empty());
}

#[test]
fn startup_recovery_ends_each_orphan_once_as_interrupted_unknown() {
    let journal = TempJournal::new("recovery");
    let orphan = orphan_take(&journal, 1_000);
    let damaged = new_take_id(1_001);
    let completed = completed_take(&journal, 1_002);
    // A start receipt torn by power loss still counts by its name.
    fs::write(journal.0.join(started_name(&damaged)), b"{\"version\":1,").unwrap();
    fs::write(journal.0.join("notes.txt"), b"ignored").unwrap();

    let recovered = recover_interrupted_takes(&journal.0, SMALL).unwrap();
    assert_eq!(recovered, vec![orphan.clone(), damaged.clone()]);
    for take_id in [&orphan, &damaged] {
        let ended = read_ended(&journal.0, take_id);
        assert_eq!(ended.end_reason, TakeEndReason::InterruptedUnknown);
        // Whether it was published is unknown, never claimed.
        assert_eq!((ended.published, ended.frames_written), (None, None));
    }
    assert_eq!(read_ended(&journal.0, &completed).published, Some(true));
    // A second startup recovers nothing more.
    assert!(recover_interrupted_takes(&journal.0, SMALL)
        .unwrap()
        .is_empty());
}

#[test]
fn startup_recovery_prunes_old_completed_takes_but_keeps_just_recovered_ones() {
    let journal = TempJournal::new("prune");
    // The oldest take is an orphan: recovered now, kept for this session.
    let orphan = orphan_take(&journal, 1);
    let completed = (0..4)
        .map(|index| completed_take(&journal, 100 + index))
        .collect::<Vec<_>>();
    // A lone end receipt left by a crash during an earlier pruning.
    let lone = new_take_id(50);
    fs::write(journal.0.join(ended_name(&lone)), b"{}").unwrap();

    assert_eq!(
        recover_interrupted_takes(&journal.0, SMALL).unwrap(),
        vec![orphan.clone()]
    );
    let listing = list_journal(&journal.0, SMALL).unwrap();
    assert_eq!(listing.started, listing.ended);
    let kept = [&orphan, &completed[2], &completed[3]]
        .into_iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(listing.started, kept);
    // Next startup the recovered take is an ordinary completed one.
    recover_interrupted_takes(&journal.0, SMALL).unwrap();
    let listing = list_journal(&journal.0, SMALL).unwrap();
    assert_eq!(
        listing.started,
        [&completed[2], &completed[3]]
            .into_iter()
            .cloned()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn an_overfull_journal_refuses_with_an_actionable_reason() {
    let journal = TempJournal::new("cap");
    for index in 0..3 {
        completed_take(&journal, 10 + index);
    }
    let limits = JournalLimits {
        max_receipts: 5,
        retained_completed_takes: 100,
    };
    let error = recover_interrupted_takes(&journal.0, limits).unwrap_err();
    assert!(error.contains("restart Syndocal"), "{error}");
    // Unrelated files never count toward the cap.
    let roomy = JournalLimits {
        max_receipts: 6,
        retained_completed_takes: 100,
    };
    for index in 0..20 {
        fs::write(journal.0.join(format!("other-{index}.txt")), b"x").unwrap();
    }
    assert!(recover_interrupted_takes(&journal.0, roomy).is_ok());
}

#[test]
fn end_receipt_errors_are_bounded() {
    let journal = TempJournal::new("bounded");
    let take_id = new_take_id(1_000);
    TakeJournal::begin(&journal.0, &started(&take_id))
        .unwrap()
        .end(TakeEndReason::Fault, false, 0, Some("é".repeat(4_000)))
        .unwrap();
    let error = read_ended(&journal.0, &take_id).error.unwrap();
    assert!(
        error.len() <= MAX_ERROR_BYTES + '…'.len_utf8(),
        "{}",
        error.len()
    );
}
