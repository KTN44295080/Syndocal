use super::*;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

enum AsioSelectionRestoreTestRead {
    Bytes(Vec<u8>),
    Error(std::io::ErrorKind),
}

struct AsioSelectionRestoreTestReader {
    declared_len: Result<u64, std::io::ErrorKind>,
    read: AsioSelectionRestoreTestRead,
}

fn asio_selection_restore_test_directory(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!("syndocal-{label}-{}-{nonce}", std::process::id()))
}

impl AsioInputSelectionStorageReader for AsioSelectionRestoreTestReader {
    fn declared_len(&self) -> std::io::Result<u64> {
        match &self.declared_len {
            Ok(declared_len) => Ok(*declared_len),
            Err(error_kind) => Err(std::io::Error::from(*error_kind)),
        }
    }

    fn read_bounded_to_end(
        &mut self,
        maximum_bytes: u64,
        bytes: &mut Vec<u8>,
    ) -> std::io::Result<usize> {
        match &self.read {
            AsioSelectionRestoreTestRead::Bytes(source) => {
                let limit = usize::try_from(maximum_bytes).unwrap_or(usize::MAX);
                let copied = source.len().min(limit);
                bytes.extend_from_slice(&source[..copied]);
                Ok(copied)
            }
            AsioSelectionRestoreTestRead::Error(error_kind) => {
                Err(std::io::Error::from(*error_kind))
            }
        }
    }
}

fn assert_asio_selection_restore_is_invalid_and_preserves_bytes(
    path: &Path,
    expected_bytes: &[u8],
    outcome: AsioSelectionRestoreOutcome,
    expected_reason_fragment: Option<&str>,
) {
    let (reason, message) = match outcome {
        AsioSelectionRestoreOutcome::Invalid { reason, message } => (reason, message),
        AsioSelectionRestoreOutcome::Absent => panic!("storage fault must not become absent"),
        AsioSelectionRestoreOutcome::Restored(_) => {
            panic!("storage fault must not restore a selection")
        }
    };
    assert!(!reason.trim().is_empty());
    if let Some(expected_reason_fragment) = expected_reason_fragment {
        assert!(
            reason.contains(expected_reason_fragment),
            "expected {reason:?} to contain {expected_reason_fragment:?}"
        );
    }
    assert!(message.contains("stays locked"));
    assert_eq!(fs::read(path).unwrap(), expected_bytes);

    let mut selection_state = AsioInputSelectionState {
        selection: Some(asio_contract_selection()),
        ..AsioInputSelectionState::default()
    };
    install_asio_input_selection_restore(
        &mut selection_state,
        path.to_path_buf(),
        AsioSelectionRestoreOutcome::Invalid {
            reason: reason.clone(),
            message: message.clone(),
        },
    );
    assert_eq!(selection_state.path.as_deref(), Some(path));
    assert!(selection_state.selection.is_none());
    let status = selection_state.status_snapshot().unwrap();
    assert_eq!(status.state, "invalid");
    assert_eq!(status.reason.as_deref(), Some(reason.as_str()));
    assert_eq!(status.message.as_deref(), Some(message.as_str()));
    assert_eq!(fs::read(path).unwrap(), expected_bytes);
}

#[test]
fn asio_selection_restore_missing_file_is_absent_and_installs_no_status() {
    let path = asio_selection_restore_test_directory("asio-selection-restore-missing")
        .join(ASIO_INPUT_SELECTION_FILE);
    assert!(!path.exists());

    let outcome = load_asio_input_selection_from_path(&path);

    assert!(matches!(outcome, AsioSelectionRestoreOutcome::Absent));
    let mut selection_state = AsioInputSelectionState::default();
    install_asio_input_selection_restore(&mut selection_state, path.clone(), outcome);
    assert_eq!(selection_state.path.as_deref(), Some(path.as_path()));
    assert!(selection_state.selection.is_none());
    assert!(selection_state.status_snapshot().is_none());
    assert!(!path.exists());
}

#[test]
fn asio_selection_restore_storage_faults_become_invalid_without_mutating_payload() {
    let directory = asio_selection_restore_test_directory("asio-selection-restore-faults");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(ASIO_INPUT_SELECTION_FILE);
    let preserved = b"operator-owned persisted selection payload";
    fs::write(&path, preserved).unwrap();

    let open_failure: std::io::Result<AsioSelectionRestoreTestReader> =
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &path,
        preserved,
        load_asio_input_selection_from_reader(&path, open_failure),
        Some("open failed"),
    );

    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &path,
        preserved,
        load_asio_input_selection_from_reader(
            &path,
            Ok(AsioSelectionRestoreTestReader {
                declared_len: Err(std::io::ErrorKind::PermissionDenied),
                read: AsioSelectionRestoreTestRead::Bytes(Vec::new()),
            }),
        ),
        Some("metadata failed"),
    );

    // Only the initial File::open NotFound is Absent. A file that was
    // opened and then disappears before metadata is an invalid/stale
    // payload, and the existing bytes must remain untouched.
    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &path,
        preserved,
        load_asio_input_selection_from_reader(
            &path,
            Ok(AsioSelectionRestoreTestReader {
                declared_len: Err(std::io::ErrorKind::NotFound),
                read: AsioSelectionRestoreTestRead::Bytes(Vec::new()),
            }),
        ),
        Some("metadata failed"),
    );

    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &path,
        preserved,
        load_asio_input_selection_from_reader(
            &path,
            Ok(AsioSelectionRestoreTestReader {
                declared_len: Ok(0),
                read: AsioSelectionRestoreTestRead::Error(std::io::ErrorKind::UnexpectedEof),
            }),
        ),
        Some("read failed"),
    );

    // Likewise, post-open read NotFound is never promoted to Absent.
    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &path,
        preserved,
        load_asio_input_selection_from_reader(
            &path,
            Ok(AsioSelectionRestoreTestReader {
                declared_len: Ok(0),
                read: AsioSelectionRestoreTestRead::Error(std::io::ErrorKind::NotFound),
            }),
        ),
        Some("read failed"),
    );

    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &path,
        preserved,
        load_asio_input_selection_from_reader(
            &path,
            Ok(AsioSelectionRestoreTestReader {
                declared_len: Ok(MAX_ASIO_INPUT_SELECTION_BYTES),
                read: AsioSelectionRestoreTestRead::Bytes(vec![
                    b'x';
                    (MAX_ASIO_INPUT_SELECTION_BYTES + 1)
                        as usize
                ]),
            }),
        ),
        Some("payload too large"),
    );

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn asio_selection_restore_invalid_file_bytes_stay_unchanged_and_install_a_visible_lock() {
    let directory = asio_selection_restore_test_directory("asio-selection-restore-invalid-bytes");
    fs::create_dir_all(&directory).unwrap();
    for (label, bytes, expected_reason_fragment) in [
        ("utf8", vec![0xff, 0xfe], Some("UTF-8 decode failed")),
        ("parse", br#"{}"#.to_vec(), None),
        (
            "bounded-read",
            vec![b'x'; (MAX_ASIO_INPUT_SELECTION_BYTES + 1) as usize],
            Some("payload too large"),
        ),
    ] {
        let path = directory.join(format!("{label}-{ASIO_INPUT_SELECTION_FILE}"));
        fs::write(&path, &bytes).unwrap();
        let outcome = load_asio_input_selection_from_path(&path);
        assert_asio_selection_restore_is_invalid_and_preserves_bytes(
            &path,
            &bytes,
            outcome,
            expected_reason_fragment,
        );
    }
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn asio_selection_restore_exact_byte_limit_restores_and_larger_payload_stays_invalid() {
    let directory = asio_selection_restore_test_directory("asio-selection-restore-byte-limit");
    fs::create_dir_all(&directory).unwrap();
    let exact_path = directory.join(ASIO_INPUT_SELECTION_FILE);
    let mut exact_bytes = asio_contract_selection()
        .storage_text()
        .unwrap()
        .into_bytes();
    assert!(exact_bytes.len() < MAX_ASIO_INPUT_SELECTION_BYTES as usize);
    exact_bytes.resize(MAX_ASIO_INPUT_SELECTION_BYTES as usize, b' ');
    assert_eq!(
        exact_bytes.len(),
        MAX_ASIO_INPUT_SELECTION_BYTES as usize,
        "the exact byte limit is permitted when the payload remains canonical JSON plus whitespace"
    );
    fs::write(&exact_path, &exact_bytes).unwrap();

    let outcome = load_asio_input_selection_from_path(&exact_path);
    match &outcome {
        AsioSelectionRestoreOutcome::Restored(selection) => {
            assert_eq!(selection.driver_id(), ASIO_CONTRACT_DRIVER);
        }
        AsioSelectionRestoreOutcome::Absent => {
            panic!("an exactly bounded valid payload must not be absent")
        }
        AsioSelectionRestoreOutcome::Invalid { reason, message } => {
            panic!(
                "an exactly bounded valid payload must restore, got invalid {reason:?}: {message:?}"
            )
        }
    }
    let mut selection_state = AsioInputSelectionState::default();
    install_asio_input_selection_restore(&mut selection_state, exact_path.clone(), outcome);

    assert_eq!(fs::read(&exact_path).unwrap(), exact_bytes);
    assert_eq!(selection_state.path.as_deref(), Some(exact_path.as_path()));
    assert_eq!(
        selection_state.selection.as_ref().unwrap().driver_id(),
        ASIO_CONTRACT_DRIVER
    );
    let status = selection_state.status_snapshot().unwrap();
    assert_eq!(status.state, "restored");
    assert!(status.message.as_deref().unwrap().contains("stays locked"));

    let above_path = directory.join(format!("above-{ASIO_INPUT_SELECTION_FILE}"));
    let mut above_bytes = exact_bytes.clone();
    above_bytes.push(b' ');
    assert_eq!(
        above_bytes.len(),
        (MAX_ASIO_INPUT_SELECTION_BYTES + 1) as usize
    );
    fs::write(&above_path, &above_bytes).unwrap();
    assert_asio_selection_restore_is_invalid_and_preserves_bytes(
        &above_path,
        &above_bytes,
        load_asio_input_selection_from_path(&above_path),
        Some("payload too large"),
    );
    let _ = fs::remove_dir_all(directory);
}
