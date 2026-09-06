//! Windows publication uses locked handles and a recoverable two-rename protocol.
//! It is deliberately not described as atomic replacement: the target can be
//! absent between renames. A synced intent precedes that gap and owns recovery.
use super::{identity, state_from_file, RecordingArtifact, TargetState};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[path = "recording_publication_windows.rs"]
mod platform;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    version: u32,
    target: PathBuf,
    staging: PathBuf,
    backup: PathBuf,
    previous: TargetState,
    replacement: TargetState,
}

fn intent_path(target: &Path) -> PathBuf {
    let key = target.to_string_lossy().to_lowercase();
    let digest = Sha256::digest(key.as_bytes());
    target.with_file_name(format!(".syndocal-recording-{digest:x}.recovery.json"))
}

fn paths(intent: &Intent) -> String {
    format!(
        "target {}; recording {}; previous {}; recovery {}",
        intent.target.display(),
        intent.staging.display(),
        intent.backup.display(),
        intent_path(&intent.target).display()
    )
}

fn validate(intent: &Intent, target: &Path) -> Result<(), String> {
    let valid_staging = intent
        .staging
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|name| {
            name.starts_with(".syndocal-recording-") && name.ends_with(".partial.mp4")
        });
    if intent.version != 1
        || intent.target != target
        || !valid_staging
        || intent.staging.parent() != target.parent()
        || intent.backup != intent.staging.with_extension("previous.mp4")
        || intent.staging == intent.target
        || intent.backup == intent.target
        || !matches!(intent.previous, TargetState::Present { .. })
        || !matches!(intent.replacement, TargetState::Present { .. })
        || intent.previous == intent.replacement
    {
        return Err(format!(
            "Invalid recording recovery intent at {}; nothing was changed",
            intent_path(target).display()
        ));
    }
    Ok(())
}

fn lock_optional(path: &Path) -> Result<Option<(File, TargetState)>, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Cannot inspect recording recovery path {}: {error}",
                path.display()
            ))
        }
        Ok(_) => {}
    }
    let file = platform::open_locked(path)?;
    let state = state_from_file(&file)?;
    Ok(Some((file, state)))
}

/// Only a completed intent may be retired. Ambiguous identities remain untouched.
pub(super) fn recover(target: &Path) -> Result<(), String> {
    let path = intent_path(target);
    let Some((mut journal, _)) = lock_optional(&path)? else {
        return Ok(());
    };
    if journal.metadata().map_err(|e| e.to_string())?.len() > 65_536 {
        return Err(format!(
            "Recording recovery intent is oversized: {}; nothing was changed",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    journal.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let intent: Intent = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "Invalid recording recovery intent {}: {e}; nothing was changed",
            path.display()
        )
    })?;
    validate(&intent, target)?;
    let current = lock_optional(target)?;
    let backup = lock_optional(&intent.backup)?;
    let completed = current
        .as_ref()
        .is_some_and(|(_, s)| s == &intent.replacement);
    if completed {
        if let Some((file, state)) = backup {
            if state != intent.previous {
                return Err(format!(
                    "Recording backup changed; retained {}",
                    paths(&intent)
                ));
            }
            platform::delete_owned(&file)?;
        }
        platform::delete_owned(&journal)?;
        return Ok(());
    }
    let staging = lock_optional(&intent.staging)?;
    if !staging
        .as_ref()
        .is_some_and(|(_, s)| s == &intent.replacement)
    {
        return Err(format!(
            "Recording recovery cannot verify the replacement; retained {}",
            paths(&intent)
        ));
    }
    match (current, backup) {
        (Some((_, state)), None) if state == intent.previous => {}
        (None, Some((file, state))) if state == intent.previous => {
            platform::rename_no_replace(&file, target)
                .map_err(|e| format!("{e}; recording recovery retained {}", paths(&intent)))?;
        }
        _ => {
            return Err(format!(
                "Recording recovery is ambiguous; nothing was overwritten; retained {}",
                paths(&intent)
            ))
        }
    }
    platform::delete_owned(&journal)?;
    Err(format!("Previous recording restored or preserved; completed partial retained at {}. Retry recording or inspect that partial first", intent.staging.display()))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PublicationPoint {
    Prepared,
    BeforeInstall,
    Installed,
    BackupRemoved,
}

pub(super) fn publish(
    artifact: &mut RecordingArtifact,
    mut checkpoint: impl FnMut(PublicationPoint),
) -> Result<(), String> {
    let expected = state_from_file(
        artifact
            .file
            .as_ref()
            .ok_or("Recording staging handle is unavailable")?,
    )?;
    artifact.file.take();
    let staging = platform::open_locked(&artifact.staging)?;
    if identity(&staging)? != artifact.identity || state_from_file(&staging)? != expected {
        return Err(format!(
            "Recording partial ownership or content changed: {}",
            artifact.staging.display()
        ));
    }
    staging
        .sync_all()
        .map_err(|e| format!("Cannot sync recording: {e}"))?;
    if artifact.target_state == TargetState::Absent {
        checkpoint(PublicationPoint::BeforeInstall);
        platform::rename_no_replace(&staging, &artifact.target)?;
        artifact.owned = false;
        return Ok(());
    }
    let previous = platform::open_locked(&artifact.target)?;
    if state_from_file(&previous)? != artifact.target_state {
        return Err(format!(
            "Recording target changed; refusing to overwrite {}",
            artifact.target.display()
        ));
    }
    let intent = Intent {
        version: 1,
        target: artifact.target.clone(),
        staging: artifact.staging.clone(),
        backup: artifact.staging.with_extension("previous.mp4"),
        previous: state_from_file(&previous)?,
        replacement: expected,
    };
    validate(&intent, &artifact.target)?;
    let bytes = serde_json::to_vec(&intent).map_err(|e| e.to_string())?;
    if bytes.len() > 65_536 {
        return Err(
            "Recording recovery intent exceeds 64 KiB; previous output was preserved".into(),
        );
    }
    let path = intent_path(&artifact.target);
    let mut journal = platform::create_locked(&path)?;
    // From this point, even unwinding must not let ordinary artifact Drop erase
    // a file named by recovery. Filesystem identity, not an in-memory phase,
    // determines which rename completed after an interrupted process.
    artifact.owned = false;
    journal
        .write_all(&bytes)
        .and_then(|_| journal.sync_all())
        .map_err(|e| {
            format!(
                "Cannot persist recording recovery intent: {e}; retained {}",
                paths(&intent)
            )
        })?;
    checkpoint(PublicationPoint::Prepared);
    platform::rename_no_replace(&previous, &intent.backup)
        .map_err(|e| format!("{e}; retained {}", paths(&intent)))?;
    checkpoint(PublicationPoint::BeforeInstall);
    if let Err(error) = platform::rename_no_replace(&staging, &artifact.target) {
        let restore = platform::rename_no_replace(&previous, &artifact.target);
        if restore.is_ok() {
            platform::delete_owned(&journal)?;
        }
        return Err(format!(
            "{error}; previous restore: {restore:?}; retained {}",
            paths(&intent)
        ));
    }
    checkpoint(PublicationPoint::Installed);
    platform::delete_owned(&previous).map_err(|e| {
        format!(
            "Recording published, but previous-file cleanup failed: {e}; retained {}",
            paths(&intent)
        )
    })?;
    drop(previous);
    checkpoint(PublicationPoint::BackupRemoved);
    platform::delete_owned(&journal).map_err(|e| {
        format!(
            "Recording published, but recovery cleanup failed: {e}; retained {}",
            paths(&intent)
        )
    })?;
    Ok(())
}

#[cfg(test)]
#[path = "recording_publication_tests.rs"]
mod tests;
