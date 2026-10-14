//! Local recording receipt journal (DEC-RECORD-OWN-001, phase 2).
//!
//! Every take writes an immutable `TakeStarted` receipt before it can record
//! and exactly one `TakeEnded` receipt when it ends, including when its
//! worker panics. A take whose start receipt cannot be written never
//! records. At startup, every orphaned `TakeStarted` (a crash or power loss)
//! receives one `TakeEnded { interrupted_unknown }`; its partial file is left
//! where it is for inspection.
//!
//! Receipts are created with `create_new`, so an existing receipt is never
//! overwritten. A receipt whose write fails is removed again. A damaged
//! receipt still counts by its file name: a damaged `TakeStarted` without an
//! end is recovered as interrupted, and a damaged `TakeEnded` still ends its
//! take.

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub(crate) const RECORDING_RECEIPT_VERSION: u32 = 1;
pub(crate) const RECORDING_RECEIPT_DIRECTORY: &str = "recording-receipts";
const STARTED_SUFFIX: &str = ".started.json";
const ENDED_SUFFIX: &str = ".ended.json";
const TAKE_PREFIX: &str = "take-";
const MAX_TAKE_ID_BYTES: usize = 64;
const MAX_ERROR_BYTES: usize = 1_024;

/// Bounds of the startup scan and pruning.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JournalLimits {
    /// Receipts held in memory while scanning; other entries are skipped.
    pub(crate) max_receipts: usize,
    /// Completed takes kept; older completed pairs are removed at startup.
    pub(crate) retained_completed_takes: usize,
}

pub(crate) const JOURNAL_LIMITS: JournalLimits = JournalLimits {
    // Pruning keeps a healthy journal at about 2,000 receipts, so this is
    // reached only by about 24,000 takes in a single session.
    max_receipts: 50_000,
    retained_completed_takes: 1_000,
};

static NEXT_TAKE_SERIAL: AtomicU64 = AtomicU64::new(0);
static JOURNAL: OnceLock<Result<PathBuf, String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TakeEndReason {
    OperatorStop,
    OwnershipRevoked,
    Fault,
    /// The process ended before the take's outcome was recorded.
    InterruptedUnknown,
}

impl From<super::VideoRecordingEndReason> for TakeEndReason {
    fn from(reason: super::VideoRecordingEndReason) -> Self {
        match reason {
            super::VideoRecordingEndReason::OperatorStop => Self::OperatorStop,
            super::VideoRecordingEndReason::OwnershipRevoked => Self::OwnershipRevoked,
            super::VideoRecordingEndReason::Fault => Self::Fault,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TakeStartedReceipt {
    pub(crate) version: u32,
    pub(crate) take_id: String,
    pub(crate) owner_epoch: u64,
    pub(crate) output_id: u64,
    pub(crate) target_path: String,
    pub(crate) staging_path: String,
    pub(crate) started_unix_ms: u64,
    pub(crate) process_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TakeEndedReceipt {
    pub(crate) version: u32,
    pub(crate) take_id: String,
    pub(crate) end_reason: TakeEndReason,
    /// None when unknown: the process or worker ended before the outcome
    /// was recorded, so the target may or may not hold this take.
    pub(crate) published: Option<bool>,
    /// None when unknown, as above.
    pub(crate) frames_written: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) ended_unix_ms: u64,
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

/// A new take id. Every part is zero-padded, so ids sort in start order.
pub(crate) fn new_take_id(started_unix_ms: u64) -> String {
    let serial = NEXT_TAKE_SERIAL.fetch_add(1, Ordering::Relaxed);
    format!(
        "{started_unix_ms:020}-{:010}-{serial:010}",
        std::process::id()
    )
}

/// The take's start time, or now when the status could not be read.
pub(crate) fn take_start_unix_ms(status_started_unix_ms: Option<u64>) -> u64 {
    status_started_unix_ms.unwrap_or_else(now_unix_ms)
}

fn valid_take_id(take_id: &str) -> bool {
    !take_id.is_empty()
        && take_id.len() <= MAX_TAKE_ID_BYTES
        && take_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'-')
}

fn started_name(take_id: &str) -> String {
    format!("{TAKE_PREFIX}{take_id}{STARTED_SUFFIX}")
}

fn ended_name(take_id: &str) -> String {
    format!("{TAKE_PREFIX}{take_id}{ENDED_SUFFIX}")
}

fn bounded_error(error: String) -> String {
    if error.len() <= MAX_ERROR_BYTES {
        return error;
    }
    let mut end = MAX_ERROR_BYTES;
    while !error.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &error[..end])
}

/// Write `bytes` as a new file; never replaces an existing receipt. A
/// failed write removes the file again, so no torn receipt is left.
fn write_new_receipt(directory: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let path = directory.join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            format!(
                "Cannot create recording receipt {}: {error}",
                path.display()
            )
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(format!(
            "Cannot write recording receipt {}: {error}",
            path.display()
        ));
    }
    sync_directory(directory);
    Ok(())
}

/// Makes a new receipt's directory entry durable where the platform allows.
fn sync_directory(directory: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = fs::File::open(directory) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = directory;
}

fn ended_receipt_bytes(receipt: &TakeEndedReceipt) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("Cannot encode recording receipt: {error}"))
}

/// A take with a written `TakeStarted` receipt. `end` writes its one
/// `TakeEnded` receipt; dropping it unended (a worker panic) writes a
/// `fault` end whose outcome is unknown.
#[derive(Debug)]
pub(crate) struct TakeJournal {
    directory: PathBuf,
    take_id: String,
    ended: bool,
}

impl TakeJournal {
    pub(crate) fn begin(directory: &Path, receipt: &TakeStartedReceipt) -> Result<Self, String> {
        if receipt.version != RECORDING_RECEIPT_VERSION || !valid_take_id(&receipt.take_id) {
            return Err("Recording take receipt is invalid".to_string());
        }
        let bytes = serde_json::to_vec_pretty(receipt)
            .map_err(|error| format!("Cannot encode recording receipt: {error}"))?;
        write_new_receipt(directory, &started_name(&receipt.take_id), &bytes)?;
        Ok(Self {
            directory: directory.to_path_buf(),
            take_id: receipt.take_id.clone(),
            ended: false,
        })
    }

    pub(crate) fn end(
        mut self,
        end_reason: TakeEndReason,
        published: bool,
        frames_written: u64,
        error: Option<String>,
    ) -> Result<(), String> {
        self.ended = true;
        self.write_end(TakeEndedReceipt {
            version: RECORDING_RECEIPT_VERSION,
            take_id: self.take_id.clone(),
            end_reason,
            published: Some(published),
            frames_written: Some(frames_written),
            error: error.map(bounded_error),
            ended_unix_ms: now_unix_ms(),
        })
    }

    fn write_end(&self, receipt: TakeEndedReceipt) -> Result<(), String> {
        let bytes = ended_receipt_bytes(&receipt)?;
        write_new_receipt(&self.directory, &ended_name(&self.take_id), &bytes)
    }
}

impl Drop for TakeJournal {
    fn drop(&mut self) {
        if self.ended {
            return;
        }
        let receipt = TakeEndedReceipt {
            version: RECORDING_RECEIPT_VERSION,
            take_id: self.take_id.clone(),
            end_reason: TakeEndReason::Fault,
            published: None,
            frames_written: None,
            error: Some("The recording worker stopped unexpectedly".to_string()),
            ended_unix_ms: now_unix_ms(),
        };
        if let Err(error) = self.write_end(receipt) {
            eprintln!("Recording end receipt failed: {error}");
        }
    }
}

/// Take ids found in the journal, by receipt kind.
#[derive(Debug, Default)]
struct JournalListing {
    started: BTreeSet<String>,
    ended: BTreeSet<String>,
}

fn list_journal(directory: &Path, limits: JournalLimits) -> Result<JournalListing, String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "Cannot read recording receipts {}: {error}",
            directory.display()
        )
    })?;
    let mut listing = JournalListing::default();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Cannot read recording receipts {}: {error}",
                directory.display()
            )
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix(TAKE_PREFIX) else {
            continue;
        };
        let (set, take_id) = if let Some(take_id) = rest.strip_suffix(STARTED_SUFFIX) {
            (&mut listing.started, take_id)
        } else if let Some(take_id) = rest.strip_suffix(ENDED_SUFFIX) {
            (&mut listing.ended, take_id)
        } else {
            continue;
        };
        if !valid_take_id(take_id) {
            continue;
        }
        set.insert(take_id.to_string());
        if listing.started.len() + listing.ended.len() > limits.max_receipts {
            return Err(format!(
                "Recording receipts {} hold more than {} receipts; move old receipts away and restart Syndocal to record again",
                directory.display(),
                limits.max_receipts
            ));
        }
    }
    Ok(listing)
}

/// Startup recovery: ends every orphaned take as `interrupted_unknown`, then
/// prunes the oldest completed takes and lone end receipts. Returns the
/// recovered take ids. Pruning is best effort and never fails recovery.
pub(crate) fn recover_interrupted_takes(
    directory: &Path,
    limits: JournalLimits,
) -> Result<Vec<String>, String> {
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Cannot create recording receipts {}: {error}",
            directory.display()
        )
    })?;
    let listing = list_journal(directory, limits)?;
    let orphans = listing
        .started
        .difference(&listing.ended)
        .cloned()
        .collect::<Vec<_>>();
    for take_id in &orphans {
        let receipt = TakeEndedReceipt {
            version: RECORDING_RECEIPT_VERSION,
            take_id: take_id.clone(),
            end_reason: TakeEndReason::InterruptedUnknown,
            published: None,
            frames_written: None,
            error: Some(
                "Syndocal ended before this take's outcome was recorded; check its target and partial file"
                    .to_string(),
            ),
            ended_unix_ms: now_unix_ms(),
        };
        let bytes = ended_receipt_bytes(&receipt)?;
        match write_new_receipt(directory, &ended_name(take_id), &bytes) {
            Ok(()) => {}
            // Another recovery already ended it.
            Err(_) if directory.join(ended_name(take_id)).exists() => {}
            Err(error) => return Err(error),
        }
    }
    if let Err(error) = prune(directory, &listing, &orphans, limits) {
        eprintln!("Recording receipt pruning stopped: {error}");
    }
    Ok(orphans)
}

fn prune(
    directory: &Path,
    listing: &JournalListing,
    recovered: &[String],
    limits: JournalLimits,
) -> Result<(), String> {
    // A lone end receipt is left by a crash between the two deletes below.
    for take_id in listing.ended.difference(&listing.started) {
        remove_receipt(directory, &ended_name(take_id))?;
    }
    // Takes recovered just now stay visible for at least one session.
    let completed = listing
        .started
        .intersection(&listing.ended)
        .filter(|take_id| !recovered.contains(take_id))
        .collect::<Vec<_>>();
    let excess = completed
        .len()
        .saturating_sub(limits.retained_completed_takes);
    for take_id in &completed[..excess] {
        // Start first: a lone end receipt is harmless, a lone start would be
        // recovered again as interrupted.
        remove_receipt(directory, &started_name(take_id))?;
        remove_receipt(directory, &ended_name(take_id))?;
    }
    Ok(())
}

fn remove_receipt(directory: &Path, name: &str) -> Result<(), String> {
    match fs::remove_file(directory.join(name)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Cannot prune recording receipt {name}: {error}")),
    }
}

/// Runs startup recovery once and remembers the journal for later takes. A
/// failure is remembered too: recording stays refused with that reason.
pub(crate) fn initialize(directory: PathBuf) -> Result<Vec<String>, String> {
    let recovered = recover_interrupted_takes(&directory, JOURNAL_LIMITS);
    let journal = recovered.as_ref().map(|_| directory).map_err(Clone::clone);
    let _ = JOURNAL.set(journal);
    recovered
}

/// Remembers why there is no journal at all.
pub(crate) fn initialize_unavailable(reason: String) {
    let _ = JOURNAL.set(Err(reason));
}

/// The journal directory, or why recording is unavailable.
pub(crate) fn journal_directory() -> Result<PathBuf, String> {
    match JOURNAL.get() {
        Some(Ok(directory)) => Ok(directory.clone()),
        Some(Err(error)) => Err(format!("Recording receipts are unavailable: {error}")),
        None => Err("Recording receipts are not initialized yet".to_string()),
    }
}

#[cfg(test)]
#[path = "tests/recording_receipts_tests.rs"]
mod tests;
