use super::*;
use std::io::Write;

fn bytes() -> Vec<u8> {
    crate::diagnostic_package::build_diagnostic_package(&[
        (
            "manifest.json",
            serde_json::to_vec(&serde_json::json!({
                "app": "Syndocal", "version": 2, "app_version": env!("CARGO_PKG_VERSION"),
                "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            }))
            .unwrap(),
        ),
        ("project-summary.json", br#"{"fixtures":3}"#.to_vec()),
        ("engine-telemetry.json", br#"{"version":1}"#.to_vec()),
        ("video-runtime.json", br#"{"backends":[]}"#.to_vec()),
        ("audit-history.json",serde_json::to_vec(&crate::diagnostic_audit::fixture_history()).unwrap()),
    ])
    .unwrap()
}
fn target() -> PathBuf {
    std::env::temp_dir().join("diagnostic-session-test.zip")
}

#[test]
fn diagnostic_session_rejects_untrusted_window_invalid_archive_and_target() {
    let state = DiagnosticExports::default();
    assert!(state.prepare("output", bytes(), target()).is_err());
    assert!(state.prepare("main", vec![], target()).is_err());
    assert!(state
        .prepare("main", bytes(), "relative.zip".into())
        .is_err());
    assert!(state
        .prepare("main", bytes(), target().with_extension("sdc"))
        .is_err());
    let mut nonce = [0u8; 16];
    getrandom::getrandom(&mut nonce).unwrap();
    let suffix: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    let occupied = std::env::temp_dir().join(format!("diagnostic-session-occupied-{suffix}.zip"));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&occupied)
        .unwrap()
        .write_all(b"existing data")
        .unwrap();
    assert!(state.prepare("main", bytes(), occupied.clone()).is_err());
    assert_eq!(std::fs::read(&occupied).unwrap(), b"existing data");
    std::fs::remove_file(&occupied).unwrap();
    assert!(state.0.lock().unwrap().is_empty());
}

#[test]
fn diagnostic_session_binds_exact_capture_and_consumes_before_publication() {
    let state = DiagnosticExports::default();
    let original = bytes();
    let preview = state.prepare("main", original.clone(), target()).unwrap();
    assert!(preview.summary.contains("6 entries"));
    assert_eq!(preview.sha256, format!("{:x}", Sha256::digest(&original)));
    assert!(state
        .finish_with(
            "output",
            &preview.capture_id,
            &preview.sha256,
            true,
            Instant::now(),
            |_, _| panic!()
        )
        .is_err());
    assert!(state
        .finish_with(
            "main",
            &preview.capture_id,
            &"0".repeat(64),
            true,
            Instant::now(),
            |_, _| panic!()
        )
        .is_err());
    let result = state
        .finish_with(
            "main",
            &preview.capture_id,
            &preview.sha256,
            true,
            Instant::now(),
            |path, captured| {
                assert_eq!(path, target());
                assert_eq!(captured, original);
                assert!(!state.0.lock().unwrap().contains_key(&preview.capture_id));
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(result.as_deref(), target().to_str());
    assert!(state
        .finish_with(
            "main",
            &preview.capture_id,
            &preview.sha256,
            true,
            Instant::now(),
            |_, _| panic!()
        )
        .is_err());
}

#[test]
fn diagnostic_session_cancel_and_ambiguous_failure_never_replay() {
    let state = DiagnosticExports::default();
    for approved in [false, true] {
        let preview = state.prepare("main", bytes(), target()).unwrap();
        let result = state.finish_with(
            "main",
            &preview.capture_id,
            &preview.sha256,
            approved,
            Instant::now(),
            |_, _| {
                assert!(approved, "cancel must not publish");
                Err("ambiguous publication".into())
            },
        );
        if approved {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), None);
        }
        assert!(state
            .finish_with(
                "main",
                &preview.capture_id,
                &preview.sha256,
                true,
                Instant::now(),
                |_, _| panic!()
            )
            .is_err());
    }
}

#[test]
fn diagnostic_session_capacity_expiry_and_restart_are_fail_closed() {
    let state = DiagnosticExports::default();
    let now = Instant::now();
    let mut previews = Vec::new();
    for _ in 0..CAPACITY {
        previews.push(state.prepare_at("main", bytes(), target(), now).unwrap());
    }
    assert!(state.prepare_at("main", bytes(), target(), now).is_err());
    let preview = &previews[0];
    assert!(state
        .finish_with(
            "main",
            &preview.capture_id,
            &preview.sha256,
            true,
            now + TTL,
            |_, _| panic!()
        )
        .is_err());
    assert!(state
        .prepare_at("main", bytes(), target(), now + TTL)
        .is_ok());
    assert!(DiagnosticExports::default()
        .finish_with(
            "main",
            &preview.capture_id,
            &preview.sha256,
            true,
            now,
            |_, _| panic!()
        )
        .is_err());
}
