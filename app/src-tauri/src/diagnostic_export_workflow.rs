//! Validate and publish the same immutable sanitized capture to a chosen destination.
use std::path::{Path, PathBuf};

pub(super) fn export_prepared_diagnostic_package(
    bytes: Vec<u8>,
    choose_destination: impl FnOnce() -> Option<PathBuf>,
    publish: impl FnOnce(&Path, &[u8]) -> Result<(), String>,
) -> Result<Option<PathBuf>, String> {
    super::diagnostic_package::diagnostic_package_preview(&bytes)
        .map_err(|error| error.to_string())?;
    let Some(path) = choose_destination() else {
        return Ok(None);
    };
    publish(&path, &bytes)?;
    Ok(Some(path))
}

#[cfg(test)]
#[path = "diagnostic_export_workflow_tests.rs"]
mod tests;
