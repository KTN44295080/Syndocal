use super::*;
use std::fs;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let mut nonce = [0_u8; 16];
        getrandom::getrandom(&mut nonce).unwrap();
        let key = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let root = std::env::temp_dir().join(format!("syndocal-diagnostic-publication-{key}"));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

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
fn diagnostic_package_publication_replaces_only_after_package_validation() {
    let scratch = Scratch::new();
    let target = scratch.path("diagnostics.zip");
    let previous = b"previous selected diagnostic export";
    fs::write(&target, previous).unwrap();
    assert!(publish_diagnostic_package(&target, b"not a diagnostic archive").is_err());
    assert_eq!(fs::read(&target).unwrap(), previous);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);

    let bytes = package();
    publish_diagnostic_package(&target, &bytes).unwrap();
    assert_eq!(fs::read(&target).unwrap(), bytes);
    crate::diagnostic_package::validate_diagnostic_package(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[test]
fn diagnostic_package_publication_failure_preserves_previous_and_staging() {
    let scratch = Scratch::new();
    let target = scratch.path("diagnostics.zip");
    let temporary = scratch.path(".owned.partial.zip");
    let previous = b"previous selected diagnostic export";
    fs::write(&target, previous).unwrap();
    let bytes = package();
    let error = publish_with(&target, &temporary, &bytes, |source, destination| {
        assert_eq!(fs::read(source).unwrap(), bytes);
        assert_eq!(fs::read(destination).unwrap(), previous);
        Err("injected pre-replacement failure".into())
    })
    .unwrap_err();
    assert!(error.contains("Inspect the destination and staging candidate"));
    assert_eq!(fs::read(&target).unwrap(), previous);
    assert_eq!(fs::read(&temporary).unwrap(), bytes);
}

#[test]
fn diagnostic_package_publication_staging_collision_never_deletes_other_file() {
    let scratch = Scratch::new();
    let target = scratch.path("diagnostics.zip");
    let temporary = scratch.path(".other-owner.partial.zip");
    fs::write(&target, b"previous").unwrap();
    fs::write(&temporary, b"another owner").unwrap();
    let error = publish_with(&target, &temporary, &package(), |_, _| {
        panic!("replace must not run after staging collision")
    })
    .unwrap_err();
    assert!(error.contains("destination unchanged"));
    assert_eq!(fs::read(&target).unwrap(), b"previous");
    assert_eq!(fs::read(&temporary).unwrap(), b"another owner");
}

#[test]
fn diagnostic_package_publication_rejects_alias_and_cross_directory_staging() {
    let scratch = Scratch::new();
    let target = scratch.path("diagnostics.zip");
    fs::write(&target, b"previous").unwrap();
    for temporary in [target.clone(), scratch.path("uncreated-child/staging.zip")] {
        assert!(publish_with(&target, &temporary, &package(), |_, _| {
            panic!("replace must not run for invalid staging")
        })
        .unwrap_err()
        .contains("distinct file"));
        assert_eq!(fs::read(&target).unwrap(), b"previous");
    }
}

#[test]
fn diagnostic_package_publication_rejects_ambiguous_target_paths() {
    assert!(validate_target(Path::new("relative.zip")).is_err());
    let scratch = Scratch::new();
    assert!(validate_target(&scratch.path("invalid\0suffix.zip")).is_err());
    assert!(validate_target(&scratch.path("project.sdc")).is_err());
    assert!(validate_target(&scratch.path("diagnostics.ZIP")).is_ok());
    let first = staging_path(&scratch.path("diagnostics.zip")).unwrap();
    let second = staging_path(&scratch.path("diagnostics.zip")).unwrap();
    assert_ne!(first, second);
    assert_eq!(first.parent(), Some(scratch.0.as_path()));
}
