//! Process-local, bounded captures for explicit preview/acknowledge export.
//! Only the trusted main window may call this service. External MCP must enter
//! through its separately authenticated, consent-bound adapter, never by treating
//! a capture id as a grant. Publication consumes a capture before any filesystem
//! operation; an ambiguous failure cannot be retried with the same capture.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

const CAPACITY: usize = 8;
const TTL: Duration = Duration::from_secs(120);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Preview {
    pub capture_id: String,
    pub sha256: String,
    pub destination: String,
    pub summary: String,
    pub valid_for_ms: u64,
}

struct Capture {
    bytes: Vec<u8>,
    destination: PathBuf,
    sha256: String,
    expires: Instant,
}

#[derive(Default)]
pub(crate) struct DiagnosticExports(Mutex<BTreeMap<String, Capture>>);

impl DiagnosticExports {
    pub(crate) fn prepare(
        &self,
        window: &str,
        bytes: Vec<u8>,
        destination: PathBuf,
    ) -> Result<Preview, String> {
        self.prepare_at(window, bytes, destination, Instant::now())
    }

    fn prepare_at(
        &self,
        window: &str,
        bytes: Vec<u8>,
        destination: PathBuf,
        now: Instant,
    ) -> Result<Preview, String> {
        main_only(window)?;
        super::diagnostic_package_publication::require_new_target(&destination)?;
        let mut summary = super::diagnostic_package::diagnostic_package_preview(&bytes)
            .map_err(|error| error.to_string())?;
        summary.push_str("\nDestination must be new; existing files are never replaced.");
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        let destination_text = destination
            .to_str()
            .ok_or("Diagnostic destination must be Unicode")?
            .to_owned();
        let mut captures = self
            .0
            .lock()
            .map_err(|_| "Diagnostic capture state unavailable")?;
        captures.retain(|_, capture| capture.expires > now);
        if captures.len() >= CAPACITY {
            return Err("Diagnostic capture capacity reached; cancel or wait for expiry".into());
        }
        let mut nonce = [0u8; 32];
        getrandom::getrandom(&mut nonce).map_err(|_| "Diagnostic capture identity unavailable")?;
        let capture_id: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
        if captures.contains_key(&capture_id) {
            return Err("Diagnostic capture identity conflict".into());
        }
        let result = Preview {
            capture_id: capture_id.clone(),
            sha256: sha256.clone(),
            destination: destination_text,
            summary,
            valid_for_ms: TTL.as_millis() as u64,
        };
        captures.insert(
            capture_id,
            Capture {
                bytes,
                destination,
                sha256,
                expires: now + TTL,
            },
        );
        Ok(result)
    }

    pub(crate) fn finish(
        &self,
        window: &str,
        capture_id: &str,
        sha256: &str,
        approved: bool,
    ) -> Result<Option<String>, String> {
        self.finish_with(
            window,
            capture_id,
            sha256,
            approved,
            Instant::now(),
            super::diagnostic_package_publication::publish_new_diagnostic_package,
        )
    }

    fn finish_with(
        &self,
        window: &str,
        capture_id: &str,
        sha256: &str,
        approved: bool,
        now: Instant,
        publish: impl FnOnce(&std::path::Path, &[u8]) -> Result<(), String>,
    ) -> Result<Option<String>, String> {
        main_only(window)?;
        let capture = {
            let mut captures = self
                .0
                .lock()
                .map_err(|_| "Diagnostic capture state unavailable")?;
            captures.retain(|_, capture| capture.expires > now);
            let capture = captures
                .get(capture_id)
                .ok_or("Diagnostic capture unknown, consumed or expired; prepare again")?;
            if capture.sha256 != sha256 {
                return Err("Diagnostic preview digest does not match; no file changed".into());
            }
            captures
                .remove(capture_id)
                .expect("capture checked under lock")
        };
        if !approved {
            return Ok(None);
        }
        publish(&capture.destination, &capture.bytes)?;
        Ok(Some(capture.destination.to_string_lossy().into_owned()))
    }
}

fn main_only(window: &str) -> Result<(), String> {
    if window != "main" {
        return Err("Diagnostic export requires the trusted main window".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "diagnostic_export_session_tests.rs"]
mod tests;
