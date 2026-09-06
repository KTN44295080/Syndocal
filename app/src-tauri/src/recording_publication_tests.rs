use super::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::atomic::{AtomicU64, Ordering},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    directory: PathBuf,
    artifact: RecordingArtifact,
}
impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "syndocal-publication-{}-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let target = directory.join("前景 movie.mp4");
        fs::write(&target, b"previous movie").unwrap();
        let artifact = RecordingArtifact::reserve(&target).unwrap();
        fs::write(artifact.path(), b"new complete movie").unwrap();
        Self {
            directory,
            artifact,
        }
    }
    fn interrupt(&mut self, point: PublicationPoint) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            publish(&mut self.artifact, |current| {
                if current == point {
                    panic!("injected publication interruption");
                }
            })
            .unwrap();
        }));
        assert!(result.is_err());
        assert!(
            !self.artifact.owned,
            "recovery must own partials before the first rename"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.artifact.file.take();
        self.artifact.owned = false;
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn recording_publication_preserves_late_creator_and_both_recovery_files() {
    let mut fixture = Fixture::new();
    let target = fixture.artifact.target.clone();
    let backup = fixture.artifact.staging.with_extension("previous.mp4");
    let staging = fixture.artifact.staging.clone();
    let result = publish(&mut fixture.artifact, |point| {
        if point == PublicationPoint::BeforeInstall {
            // The checked original object cannot be edited or renamed, even
            // after its pathname has moved out of the destination slot.
            assert!(fs::write(&backup, b"in-place intruder").is_err());
            assert!(fs::rename(&backup, backup.with_extension("stolen")).is_err());
            assert!(fs::write(&staging, b"in-place partial intruder").is_err());
            assert!(fs::rename(&staging, staging.with_extension("stolen")).is_err());
            fs::write(&target, b"late competing writer").unwrap();
        }
    });
    let error = result.unwrap_err();
    fixture.artifact.discard().unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"late competing writer");
    assert_eq!(fs::read(&backup).unwrap(), b"previous movie");
    assert_eq!(
        fs::read(fixture.artifact.path()).unwrap(),
        b"new complete movie"
    );
    assert!(error.contains(&backup.display().to_string()));
    assert!(intent_path(&target).exists());
    assert!(recover(&target).unwrap_err().contains("ambiguous"));
    assert_eq!(fs::read(&target).unwrap(), b"late competing writer");
}

#[test]
fn recording_publication_locked_final_check_rejects_change_after_early_comparison() {
    let mut fixture = Fixture::new();
    // Invoke the final publication boundary directly, after the earlier
    // orchestrator comparison could already have seen the reserved state.
    fs::write(&fixture.artifact.target, b"writer after early comparison").unwrap();
    let error = publish(&mut fixture.artifact, |_| {}).unwrap_err();
    assert!(error.contains("target changed"));
    assert_eq!(
        fs::read(&fixture.artifact.target).unwrap(),
        b"writer after early comparison"
    );
    assert!(!intent_path(&fixture.artifact.target).exists());
}

#[test]
fn recording_publication_modified_installed_target_does_not_discard_previous_recording() {
    let mut fixture = Fixture::new();
    fixture.interrupt(PublicationPoint::Installed);
    fs::write(&fixture.artifact.target, b"modified after interruption").unwrap();
    assert!(recover(&fixture.artifact.target).is_err());
    assert_eq!(
        fs::read(&fixture.artifact.target).unwrap(),
        b"modified after interruption"
    );
    assert_eq!(
        fs::read(fixture.artifact.staging.with_extension("previous.mp4")).unwrap(),
        b"previous movie"
    );
    assert!(intent_path(&fixture.artifact.target).exists());
}

#[test]
fn recording_publication_recovery_preserves_prepared_original_and_partial() {
    let mut fixture = Fixture::new();
    fixture.interrupt(PublicationPoint::Prepared);
    assert!(recover(&fixture.artifact.target)
        .unwrap_err()
        .contains("restored or preserved"));
    assert_eq!(
        fs::read(&fixture.artifact.target).unwrap(),
        b"previous movie"
    );
    assert_eq!(
        fs::read(fixture.artifact.path()).unwrap(),
        b"new complete movie"
    );
    assert!(!intent_path(&fixture.artifact.target).exists());
}

#[test]
fn recording_publication_next_reservation_restores_missing_target_without_deleting_partial() {
    let mut fixture = Fixture::new();
    fixture.interrupt(PublicationPoint::BeforeInstall);
    assert!(!fixture.artifact.target.exists());
    let error = RecordingArtifact::reserve(&fixture.artifact.target)
        .err()
        .unwrap();
    assert!(error.contains("restored or preserved"));
    assert_eq!(
        fs::read(&fixture.artifact.target).unwrap(),
        b"previous movie"
    );
    assert_eq!(
        fs::read(fixture.artifact.path()).unwrap(),
        b"new complete movie"
    );
    assert!(!intent_path(&fixture.artifact.target).exists());
    drop(RecordingArtifact::reserve(&fixture.artifact.target).unwrap());
}

#[test]
fn recording_publication_recovery_finishes_installed_recording_cleanup() {
    for point in [PublicationPoint::Installed, PublicationPoint::BackupRemoved] {
        let mut fixture = Fixture::new();
        fixture.interrupt(point);
        recover(&fixture.artifact.target).unwrap();
        assert_eq!(
            fs::read(&fixture.artifact.target).unwrap(),
            b"new complete movie"
        );
        assert!(!fixture
            .artifact
            .staging
            .with_extension("previous.mp4")
            .exists());
        assert!(!intent_path(&fixture.artifact.target).exists());
        assert!(!fixture.artifact.staging.exists());
    }
}

#[test]
fn recording_publication_live_intent_cannot_be_recovered_by_another_reservation() {
    let mut fixture = Fixture::new();
    let target = fixture.artifact.target.clone();
    publish(&mut fixture.artifact, |point| {
        if point == PublicationPoint::Prepared {
            let error = RecordingArtifact::reserve(&target).err().unwrap();
            assert!(error.contains("locked recording path"), "{error}");
        }
    })
    .unwrap();
    assert_eq!(fs::read(target).unwrap(), b"new complete movie");
}

#[test]
fn recording_publication_changed_backup_is_never_removed_during_recovery() {
    let mut fixture = Fixture::new();
    fixture.interrupt(PublicationPoint::Installed);
    let backup = fixture.artifact.staging.with_extension("previous.mp4");
    fs::remove_file(&backup).unwrap();
    fs::write(&backup, b"different file").unwrap();
    assert!(recover(&fixture.artifact.target)
        .unwrap_err()
        .contains("backup changed"));
    assert_eq!(fs::read(backup).unwrap(), b"different file");
    assert!(intent_path(&fixture.artifact.target).exists());
}

#[test]
fn recording_publication_rejects_invalid_future_and_escaping_recovery_intents() {
    for mutation in [
        "future",
        "unknown",
        "nested_unknown",
        "escape",
        "oversize",
        "truncated",
    ] {
        let mut fixture = Fixture::new();
        fixture.interrupt(PublicationPoint::Prepared);
        let path = intent_path(&fixture.artifact.target);
        let original = fs::read(&path).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match mutation {
            "future" => value["version"] = 2.into(),
            "unknown" => value["unexpected"] = true.into(),
            "nested_unknown" => value["previous"]["Present"]["unexpected"] = true.into(),
            "escape" => {
                value["staging"] = fixture
                    .directory
                    .join("elsewhere")
                    .join(".syndocal-recording-escape.partial.mp4")
                    .to_string_lossy()
                    .to_string()
                    .into()
            }
            "oversize" => {}
            "truncated" => {}
            _ => unreachable!(),
        }
        let bytes = match mutation {
            "oversize" => vec![b' '; 65_537],
            "truncated" => b"{\"version\":".to_vec(),
            _ => serde_json::to_vec(&value).unwrap(),
        };
        fs::write(&path, &bytes).unwrap();
        assert!(recover(&fixture.artifact.target).is_err(), "{mutation}");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            fs::read(&fixture.artifact.target).unwrap(),
            b"previous movie"
        );
        assert_eq!(
            fs::read(fixture.artifact.path()).unwrap(),
            b"new complete movie"
        );
    }
}

#[test]
#[ignore = "subprocess helper; selected only by the publication interruption test"]
fn recording_publication_crash_child() {
    let target = PathBuf::from(std::env::var_os("SYNDOCAL_PUBLICATION_TEST_TARGET").unwrap());
    let point = std::env::var("SYNDOCAL_PUBLICATION_TEST_POINT").unwrap();
    let mut artifact = RecordingArtifact::reserve(&target).unwrap();
    fs::write(artifact.path(), b"new complete movie").unwrap();
    publish(&mut artifact, |current| {
        if format!("{current:?}") == point {
            // Exit without Rust unwinding: only the OS closes transaction handles.
            std::process::exit(17);
        }
    })
    .unwrap();
    panic!("requested interruption point was not reached");
}

#[test]
fn recording_publication_recovers_after_process_exit_at_each_boundary() {
    for point in [
        PublicationPoint::Prepared,
        PublicationPoint::BeforeInstall,
        PublicationPoint::Installed,
        PublicationPoint::BackupRemoved,
    ] {
        let fixture = Fixture::new();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "recording_artifact::publication::tests::recording_publication_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("SYNDOCAL_PUBLICATION_TEST_TARGET", &fixture.artifact.target)
            .env("SYNDOCAL_PUBLICATION_TEST_POINT", format!("{point:?}"))
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(17),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let journal = intent_path(&fixture.artifact.target);
        let intent: Intent = serde_json::from_slice(&fs::read(&journal).unwrap()).unwrap();
        let recovered = recover(&fixture.artifact.target);
        if matches!(
            point,
            PublicationPoint::Prepared | PublicationPoint::BeforeInstall
        ) {
            let error = recovered.unwrap_err();
            assert!(error.contains("restored or preserved"), "{point:?}: {error}");
            assert_eq!(
                fs::read(&fixture.artifact.target).unwrap(),
                b"previous movie"
            );
            assert_eq!(fs::read(&intent.staging).unwrap(), b"new complete movie");
        } else {
            recovered.unwrap();
            assert_eq!(
                fs::read(&fixture.artifact.target).unwrap(),
                b"new complete movie"
            );
        }
        assert!(!journal.exists());
        assert!(!intent.backup.exists());
    }
}
