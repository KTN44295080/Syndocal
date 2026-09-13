//! Publish already-sanitized diagnostics without truncating the destination.
//! Failure cleanup never deletes a path that could have been replaced by another
//! process. A replacement error requires inspecting both destination and staging;
//! no durability or post-failure disposition is inferred from an error alone.

use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

pub(super) fn publish_diagnostic_package(path: &Path, bytes: &[u8]) -> Result<(), String> {
    super::diagnostic_package::validate_diagnostic_package(bytes)
        .map_err(|error| error.to_string())?;
    let temporary = staging_path(path)?;
    publish_with(path, &temporary, bytes, super::replace_file_atomically)
}

fn staging_path(path: &Path) -> Result<PathBuf, String> {
    validate_target(path)?;
    let mut nonce = [0_u8; 16];
    getrandom::getrandom(&mut nonce)
        .map_err(|_| "Unable to allocate a diagnostic staging identity".to_string())?;
    let key = nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(path.parent().expect("validated parent").join(format!(
        ".syndocal-diagnostics-{}-{key}.partial.zip",
        std::process::id(),
    )))
}

fn validate_target(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path.file_name().is_none()
        || path.as_os_str().to_string_lossy().contains('\0')
    {
        return Err(
            "Diagnostic target must be an absolute file path without NUL characters".into(),
        );
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err(
            "Diagnostic destination must have the .zip extension; no file was changed".into(),
        );
    }
    Ok(())
}

fn publish_with(
    target: &Path,
    temporary: &Path,
    bytes: &[u8],
    replace: impl FnOnce(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    // Repeat validation at the publication boundary. Callers cannot turn an
    // arbitrary byte buffer into a trusted package merely by skipping preview.
    super::diagnostic_package::validate_diagnostic_package(bytes)
        .map_err(|error| error.to_string())?;
    validate_target(target)?;
    if temporary == target || temporary.parent() != target.parent() {
        return Err(
            "Diagnostic staging must be a distinct file in the destination directory".into(),
        );
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .map_err(|error| {
            format!("Cannot create diagnostic staging file; destination unchanged: {error}")
        })?;
    file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|error| {
        format!("Cannot write diagnostic staging file; destination unchanged. Partial retained at {}: {error}", temporary.display())
    })?;
    drop(file);
    replace(temporary, target).map_err(|error| {
        format!("Diagnostic publication returned an error. Inspect the destination and staging candidate at {} before retrying: {error}", temporary.display())
    })
}

#[cfg(test)]
#[path = "diagnostic_package_publication_tests.rs"]
mod tests;
