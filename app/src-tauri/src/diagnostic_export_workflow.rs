//! Bind confirmation and publication to the same immutable sanitized capture.
use std::path::{Path, PathBuf};

pub(super) fn export_prepared_diagnostic_package(
    bytes: Vec<u8>,
    confirm: impl FnOnce(&str) -> bool,
    choose_destination: impl FnOnce() -> Option<PathBuf>,
    publish: impl FnOnce(&Path, &[u8]) -> Result<(), String>,
) -> Result<Option<PathBuf>, String> {
    let preview = super::diagnostic_package::diagnostic_package_preview(&bytes)
        .map_err(|error| error.to_string())?;
    if !confirm(&preview) {
        return Ok(None);
    }
    let Some(path) = choose_destination() else {
        return Ok(None);
    };
    publish(&path, &bytes)?;
    Ok(Some(path))
}

#[cfg(test)]
#[path = "diagnostic_export_workflow_tests.rs"]
mod tests;
