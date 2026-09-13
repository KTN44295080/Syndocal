use super::*;
use std::cell::RefCell;

fn package() -> Vec<u8> {
    crate::diagnostic_package::build_diagnostic_package(&[
        (
            "manifest.json",
            serde_json::to_vec(&serde_json::json!({
                "app": "Syndocal", "version": 1,
                "app_version": env!("CARGO_PKG_VERSION"),
                "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            }))
            .unwrap(),
        ),
        ("project-summary.json", br#"{"fixtures":3}"#.to_vec()),
        ("engine-telemetry.json", br#"{"version":1}"#.to_vec()),
        ("video-runtime.json", br#"{"backends":[]}"#.to_vec()),
    ])
    .unwrap()
}

#[test]
fn diagnostic_export_invalid_capture_rejects_before_confirmation_or_destination() {
    assert!(export_prepared_diagnostic_package(
        b"invalid capture".to_vec(),
        |_| panic!("invalid bytes must not be offered for approval"),
        || panic!("invalid bytes must not prompt for a destination"),
        |_, _| panic!("invalid bytes must not be written"),
    )
    .is_err());
}

#[test]
fn diagnostic_export_rejected_preview_never_chooses_or_writes_destination() {
    let result = export_prepared_diagnostic_package(
        package(),
        |preview| {
            assert!(preview.contains("5 entries"));
            false
        },
        || panic!("rejected preview must not choose a destination"),
        |_, _| panic!("rejected preview must not write a destination"),
    )
    .unwrap();
    assert!(result.is_none());
}

#[test]
fn diagnostic_export_cancelled_file_dialog_never_writes_destination() {
    let events = RefCell::new(Vec::new());
    let result = export_prepared_diagnostic_package(
        package(),
        |_| {
            events.borrow_mut().push("preview");
            true
        },
        || {
            events.borrow_mut().push("destination");
            None
        },
        |_, _| panic!("cancelled file dialog must not write"),
    )
    .unwrap();
    assert!(result.is_none());
    assert_eq!(*events.borrow(), vec!["preview", "destination"]);
}

#[test]
fn diagnostic_export_publishes_exact_approved_capture_without_recapture() {
    let bytes = package();
    let expected = bytes.clone();
    let events = RefCell::new(Vec::new());
    let target = PathBuf::from("chosen-diagnostics.zip");
    let result = export_prepared_diagnostic_package(
        bytes,
        |preview| {
            assert!(preview.contains(&format!("{} bytes", expected.len())));
            events.borrow_mut().push("preview");
            true
        },
        || {
            events.borrow_mut().push("destination");
            Some(target.clone())
        },
        |path, contents| {
            events.borrow_mut().push("publish");
            assert_eq!(path, target);
            assert_eq!(contents, expected);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(result, Some(target));
    assert_eq!(*events.borrow(), vec!["preview", "destination", "publish"]);
}

#[test]
fn diagnostic_export_publication_error_is_not_reported_as_success() {
    let result = export_prepared_diagnostic_package(
        package(),
        |_| true,
        || Some(PathBuf::from("chosen.zip")),
        |_, _| Err("injected storage failure".into()),
    );
    assert_eq!(result, Err("injected storage failure".into()));
}
