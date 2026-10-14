//! Two-PC recording lease and takeover receipts (DEC-RECORD-OWN-001,
//! phase 3).
//!
//! A take on a machine running Standby sync as Primary binds that sync
//! folder and session and renews `syndocal-recording-lease-<session>.json`
//! every second. When sync restarts with a new session the take re-binds
//! to it; when sync stops it keeps renewing the last one. The frame loop
//! self-fences when the last successful renewal is older than
//! `RECORDING_LEASE_TTL` on the recorder's own clock, when a takeover
//! receipt supersedes any session the take was bound to, or when a
//! split-brain persists past `SPLIT_BRAIN_FENCE_AFTER`. A freshly started
//! Primary briefly reports old sessions in a reused folder as active, so a
//! shorter split-brain must not end a take.
//!
//! The new owner never compares clocks across PCs. A predecessor's take has
//! provably ended when its lease says `ended`. A lease that stays unchanged
//! for `RECORDING_LEASE_CLEAR_AFTER` on the new owner's own clock is only
//! quiet: it counts as ended only once the takeover receipt that tells the
//! old Primary to stop has been written. That assumes the sync folder makes
//! a write visible within the margin, which the 5 s Standby heartbeat
//! already requires.

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub(crate) const RECORDING_LEASE_VERSION: u32 = 1;
pub(crate) const RECORDING_LEASE_RENEW_INTERVAL: Duration = Duration::from_secs(1);
/// Shorter than the 5 s Standby stale threshold.
pub(crate) const RECORDING_LEASE_TTL: Duration = Duration::from_secs(3);
pub(crate) const RECORDING_LEASE_CLEAR_AFTER: Duration = Duration::from_secs(5);
/// Longer than the about 5 s for which a starting Primary reports the old
/// sessions of a reused folder as active.
pub(crate) const SPLIT_BRAIN_FENCE_AFTER: Duration = Duration::from_secs(10);
const LEASE_PREFIX: &str = "syndocal-recording-lease-";
const TAKEOVER_PREFIX: &str = "syndocal-recording-takeover-";
const JSON_SUFFIX: &str = ".json";
const MAX_RECORD_BYTES: u64 = 16 * 1024;
const MAX_SESSION_ID_BYTES: usize = 64;
/// Takeover receipts examined per renewal.
const MAX_TAKEOVER_RECORDS: usize = 1_024;
/// Takeover receipts older than this are removed when a new one is written.
/// The age compares clocks of different PCs; only a skew of days matters.
const TAKEOVER_RECORD_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecordingLeaseState {
    Recording,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordingLeaseRecord {
    pub(crate) version: u32,
    pub(crate) session_id: String,
    pub(crate) take_id: String,
    pub(crate) sequence: u64,
    pub(crate) state: RecordingLeaseState,
    pub(crate) output_id: u64,
    pub(crate) frame_rate: u32,
    pub(crate) audio_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordingTakeoverRecord {
    pub(crate) version: u32,
    pub(crate) takeover_id: String,
    pub(crate) superseded_sessions: Vec<String>,
    /// The predecessor leases still recording at Take Over.
    pub(crate) predecessor_leases: Vec<RecordingLeaseRecord>,
    pub(crate) forced: bool,
    pub(crate) written_at_unix_ms: u64,
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

pub(crate) fn valid_session_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id.len() <= MAX_SESSION_ID_BYTES
        && session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub(crate) fn lease_path(directory: &Path, session_id: &str) -> PathBuf {
    directory.join(format!("{LEASE_PREFIX}{session_id}{JSON_SUFFIX}"))
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(format!(
            "{} is larger than {MAX_RECORD_BYTES} bytes",
            path.display()
        ));
    }
    Ok(Some(bytes))
}

/// A session's lease, `None` when absent.
pub(crate) fn read_lease(
    directory: &Path,
    session_id: &str,
) -> Result<Option<RecordingLeaseRecord>, String> {
    if !valid_session_id(session_id) {
        return Err("Recording lease session id is invalid".to_string());
    }
    let path = lease_path(directory, session_id);
    let Some(bytes) = read_bounded(&path)? else {
        return Ok(None);
    };
    let record = serde_json::from_slice::<RecordingLeaseRecord>(&bytes)
        .map_err(|error| format!("Recording lease {} is invalid: {error}", path.display()))?;
    if record.version != RECORDING_LEASE_VERSION || record.session_id != session_id {
        return Err(format!("Recording lease {} is invalid", path.display()));
    }
    Ok(Some(record))
}

fn write_lease(directory: &Path, record: &RecordingLeaseRecord) -> Result<(), String> {
    let target = lease_path(directory, &record.session_id);
    // Unique per write: a late worker of an earlier take never shares it.
    let temporary = directory.join(format!(
        ".{LEASE_PREFIX}{}-{}-{}.tmp",
        record.session_id,
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let bytes = serde_json::to_vec(record)
        .map_err(|error| format!("Cannot encode recording lease: {error}"))?;
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(&bytes)?;
            file.sync_all()
        });
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Cannot write recording lease: {error}"));
    }
    super::replace_file_atomically(&temporary, &target).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

/// The first takeover receipt in `directory` that supersedes any of
/// `sessions`.
pub(crate) fn takeover_superseding(
    directory: &Path,
    sessions: &[String],
) -> Result<Option<RecordingTakeoverRecord>, String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Cannot read sync folder {}: {error}", directory.display()))?;
    let mut examined = 0_usize;
    for entry in entries {
        let entry = entry
            .map_err(|error| format!("Cannot read sync folder {}: {error}", directory.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(TAKEOVER_PREFIX) || !name.ends_with(JSON_SUFFIX) {
            continue;
        }
        examined += 1;
        if examined > MAX_TAKEOVER_RECORDS {
            return Err(format!(
                "Sync folder {} holds more than {MAX_TAKEOVER_RECORDS} takeover receipts; remove old syndocal-recording-takeover files",
                directory.display()
            ));
        }
        // A receipt still being written or damaged is skipped; the renewal
        // after it is complete sees it.
        let Ok(Some(bytes)) = read_bounded(&entry.path()) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<RecordingTakeoverRecord>(&bytes) else {
            continue;
        };
        if record
            .superseded_sessions
            .iter()
            .any(|session| sessions.contains(session))
        {
            return Ok(Some(record));
        }
    }
    Ok(None)
}

/// Writes a new takeover receipt, never replacing one, and removes
/// receipts older than `TAKEOVER_RECORD_RETENTION` (best effort).
pub(crate) fn write_takeover(
    directory: &Path,
    record: &RecordingTakeoverRecord,
) -> Result<PathBuf, String> {
    if !valid_session_id(&record.takeover_id)
        || record
            .superseded_sessions
            .iter()
            .any(|session| !valid_session_id(session))
    {
        return Err("Recording takeover receipt is invalid".to_string());
    }
    let path = directory.join(format!(
        "{TAKEOVER_PREFIX}{}{JSON_SUFFIX}",
        record.takeover_id
    ));
    let bytes = serde_json::to_vec_pretty(record)
        .map_err(|error| format!("Cannot encode recording takeover receipt: {error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("Cannot create {}: {error}", path.display()))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(format!("Cannot write {}: {error}", path.display()));
    }
    prune_takeovers(directory, &path, record.written_at_unix_ms);
    Ok(path)
}

fn prune_takeovers(directory: &Path, keep: &Path, now_unix_ms: u64) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let retention_ms = TAKEOVER_RECORD_RETENTION.as_millis() as u64;
    for entry in entries.flatten().take(MAX_TAKEOVER_RECORDS * 4) {
        let path = entry.path();
        let is_takeover = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(TAKEOVER_PREFIX) && name.ends_with(JSON_SUFFIX));
        if !is_takeover || path == keep {
            continue;
        }
        let Ok(Some(bytes)) = read_bounded(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<RecordingTakeoverRecord>(&bytes) else {
            continue;
        };
        if now_unix_ms.saturating_sub(record.written_at_unix_ms) > retention_ms {
            let _ = fs::remove_file(&path);
        }
    }
}

/// Whether `takeover_id`'s receipt is in `directory` and reads back intact.
pub(crate) fn takeover_written(directory: &Path, takeover_id: &str) -> bool {
    let path = directory.join(format!("{TAKEOVER_PREFIX}{takeover_id}{JSON_SUFFIX}"));
    read_bounded(&path)
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<RecordingTakeoverRecord>(&bytes).ok())
        .is_some_and(|record| record.takeover_id == takeover_id)
}

/// A takeover receipt for the sessions this PC supersedes now.
pub(crate) fn takeover_record(
    takeover_id: String,
    superseded_sessions: Vec<String>,
    predecessor_leases: Vec<RecordingLeaseRecord>,
    forced: bool,
) -> RecordingTakeoverRecord {
    RecordingTakeoverRecord {
        version: RECORDING_LEASE_VERSION,
        takeover_id,
        superseded_sessions,
        predecessor_leases,
        forced,
        written_at_unix_ms: now_unix_ms(),
    }
}

/// What the recorder knows about Standby sync on each renewal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeaseObservation {
    /// The folder and session while Standby sync runs as Primary.
    pub(crate) primary: Option<(PathBuf, String)>,
    /// None when sync could not be read this time (its lock was busy).
    pub(crate) split_brain: Option<bool>,
}

/// Reads Standby sync for a take's lease, from the take's worker threads.
pub(crate) type LeaseObserver = Box<dyn Fn() -> LeaseObservation + Send>;

/// The take facts a lease publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeaseTake {
    pub(crate) take_id: String,
    pub(crate) output_id: u64,
    pub(crate) frame_rate: u32,
    pub(crate) audio_requested: bool,
}

#[derive(Debug, Default)]
struct FenceState {
    /// The folder and session the lease is renewed in.
    bound: Option<(PathBuf, String)>,
    /// Every folder and session this take was bound to. A takeover naming
    /// any of them ends the take; an earlier binding's lease is left to go
    /// quiet (never marked ended) until the take really ends.
    bindings: Vec<(PathBuf, String)>,
    last_renewed: Option<Instant>,
    last_error: Option<String>,
    split_brain_since: Option<Instant>,
    sequence: u64,
    revoked: Option<String>,
}

/// Shared between the renewal thread and the frame loop.
#[derive(Debug, Default)]
pub(crate) struct RecordingLeaseFence {
    state: Mutex<FenceState>,
}

impl RecordingLeaseFence {
    /// Whether the take may record another frame.
    pub(crate) fn check(&self, now: Instant) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Recording lease lock was poisoned".to_string())?;
        if let Some(reason) = &state.revoked {
            return Err(reason.clone());
        }
        if state.bound.is_some()
            && state
                .last_renewed
                .is_none_or(|renewed| now.saturating_duration_since(renewed) > RECORDING_LEASE_TTL)
        {
            return Err(match &state.last_error {
                Some(error) => {
                    format!("The recording lease in the sync folder could not be renewed: {error}")
                }
                None => "The recording lease in the sync folder could not be renewed".to_string(),
            });
        }
        Ok(())
    }

    fn revoke(&self, reason: String) {
        if let Ok(mut state) = self.state.lock() {
            state.revoked.get_or_insert(reason);
        }
    }

    fn fail(&self, error: String) {
        if let Ok(mut state) = self.state.lock() {
            state.last_error = Some(error);
        }
    }

    /// A takeover receipt superseding this take, in the current folder (an
    /// unreadable current folder is an error) or any earlier one (read best
    /// effort: it is no longer this take's sync folder).
    fn superseded(
        current: &Path,
        bindings: &[(PathBuf, String)],
    ) -> Result<Option<RecordingTakeoverRecord>, String> {
        let sessions = bindings
            .iter()
            .map(|(_, session)| session.clone())
            .collect::<Vec<_>>();
        if let Some(record) = takeover_superseding(current, &sessions)? {
            return Ok(Some(record));
        }
        let mut earlier = Vec::<&Path>::new();
        for (directory, _) in bindings {
            if directory.as_path() != current && !earlier.contains(&directory.as_path()) {
                earlier.push(directory);
            }
        }
        Ok(earlier
            .into_iter()
            .find_map(|directory| takeover_superseding(directory, &sessions).ok().flatten()))
    }

    /// One renewal: bind or re-bind, look for a takeover, write the lease.
    /// Errors only leave the lease unrenewed; `check` turns that into a
    /// fence after the TTL.
    pub(crate) fn renew(&self, observation: &LeaseObservation, take: &LeaseTake, now: Instant) {
        let (directory, bindings, session_id, sequence) = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.revoked.is_some() {
                return;
            }
            match observation.split_brain {
                Some(true) => {
                    let since = *state.split_brain_since.get_or_insert(now);
                    if now.saturating_duration_since(since) >= SPLIT_BRAIN_FENCE_AFTER {
                        state.revoked = Some(
                            "Two Primary sessions stayed active; this take ended so only one PC records"
                                .to_string(),
                        );
                        return;
                    }
                }
                Some(false) => state.split_brain_since = None,
                // Unknown this time: neither start nor reset the timer.
                None => {}
            }
            if let Some(primary) = &observation.primary {
                if state.bound.as_ref() != Some(primary) {
                    // Binding counts as a renewal, so the frame loop does
                    // not fence while the first lease write is in flight.
                    state.bound = Some(primary.clone());
                    if !state.bindings.contains(primary) {
                        state.bindings.push(primary.clone());
                    }
                    state.last_renewed = Some(now);
                }
            }
            let Some((directory, session_id)) = state.bound.clone() else {
                return;
            };
            state.sequence = state.sequence.saturating_add(1);
            (
                directory,
                state.bindings.clone(),
                session_id,
                state.sequence,
            )
        };
        match Self::superseded(&directory, &bindings) {
            Ok(Some(record)) => {
                self.revoke(format!(
                    "Another PC took over output ({}); this take ended",
                    record.takeover_id
                ));
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.fail(error);
                return;
            }
        }
        let record = RecordingLeaseRecord {
            version: RECORDING_LEASE_VERSION,
            session_id,
            take_id: take.take_id.clone(),
            sequence,
            state: RecordingLeaseState::Recording,
            output_id: take.output_id,
            frame_rate: take.frame_rate,
            audio_requested: take.audio_requested,
        };
        match write_lease(&directory, &record) {
            Ok(()) => {
                if let Ok(mut state) = self.state.lock() {
                    state.last_renewed = Some(now);
                    state.last_error = None;
                }
            }
            Err(error) => self.fail(error),
        }
    }

    /// Marks this take's lease in `session_id` ended. Best effort: a missed
    /// write expires instead. A later take that already owns the file is
    /// left alone.
    fn write_ended(directory: &Path, session_id: &str, take: &LeaseTake, sequence: u64) {
        match read_lease(directory, session_id) {
            Ok(Some(current)) if current.take_id != take.take_id => return,
            _ => {}
        }
        let _ = write_lease(
            directory,
            &RecordingLeaseRecord {
                version: RECORDING_LEASE_VERSION,
                session_id: session_id.to_string(),
                take_id: take.take_id.clone(),
                sequence,
                state: RecordingLeaseState::Ended,
                output_id: take.output_id,
                frame_rate: take.frame_rate,
                audio_requested: take.audio_requested,
            },
        );
    }

    /// The take really ended: every lease it ever renewed says so.
    fn end(&self, take: &LeaseTake) {
        let bindings = self.state.lock().ok().map(|mut state| {
            state.sequence = state.sequence.saturating_add(1);
            (state.bindings.clone(), state.sequence)
        });
        if let Some((bindings, sequence)) = bindings {
            for (directory, session_id) in bindings {
                Self::write_ended(&directory, &session_id, take, sequence);
            }
        }
    }
}

/// Renews the lease for one take until dropped.
pub(crate) struct RecordingLeaseKeeper {
    fence: Arc<RecordingLeaseFence>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

/// How long ending a take waits for the lease worker. A hung sync folder
/// must not hold the recording worker; the lease then expires instead.
const LEASE_WORKER_JOIN_WAIT: Duration = Duration::from_secs(2);

impl RecordingLeaseKeeper {
    /// Renews once synchronously: a take bound to a sync folder whose first
    /// lease cannot be written, or that was already superseded, never
    /// records.
    pub(crate) fn start(take: LeaseTake, observe: LeaseObserver) -> Result<Self, String> {
        let fence = Arc::new(RecordingLeaseFence::default());
        let now = Instant::now();
        fence.renew(&observe(), &take, now);
        let bound = fence
            .state
            .lock()
            .map(|state| state.bound.is_some())
            .unwrap_or(true);
        let first_write = fence
            .state
            .lock()
            .map(|state| state.last_error.clone())
            .unwrap_or_else(|_| Some("Recording lease lock was poisoned".to_string()));
        fence.check(now)?;
        if let (true, Some(error)) = (bound, first_write) {
            return Err(format!(
                "The recording lease in the sync folder could not be written: {error}"
            ));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let worker_fence = Arc::clone(&fence);
        let worker_stop = Arc::clone(&stop);
        let worker_take = take;
        let worker = std::thread::Builder::new()
            .name("syndocal-recording-lease".to_string())
            .spawn(move || {
                let mut next = Instant::now() + RECORDING_LEASE_RENEW_INTERVAL;
                while !worker_stop.load(Ordering::Acquire) {
                    let now = Instant::now();
                    if now < next {
                        std::thread::sleep((next - now).min(Duration::from_millis(100)));
                        continue;
                    }
                    worker_fence.renew(&observe(), &worker_take, now);
                    next = now + RECORDING_LEASE_RENEW_INTERVAL;
                }
                worker_fence.end(&worker_take);
            })
            .map_err(|error| format!("Cannot start the recording lease worker: {error}"))?;
        Ok(Self {
            fence,
            stop,
            worker: Some(worker),
        })
    }

    pub(crate) fn fence(&self) -> &RecordingLeaseFence {
        &self.fence
    }
}

impl Drop for RecordingLeaseKeeper {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let Some(worker) = self.worker.take() else {
            return;
        };
        let deadline = Instant::now() + LEASE_WORKER_JOIN_WAIT;
        while !worker.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if worker.is_finished() {
            let _ = worker.join();
        }
        // Otherwise the worker is detached; it holds only its own clones.
    }
}

/// What the new owner knows about one predecessor take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PredecessorState {
    /// Its lease says the take ended.
    Ended,
    /// Its lease stopped changing, or stayed absent, for long enough.
    /// Counts as ended only once the takeover receipt has been written.
    Quiet,
    Recording,
    /// Its lease cannot be read; nothing is proved.
    Unknown(String),
}

/// The new owner's view of one predecessor lease.
#[derive(Debug, Default)]
pub(crate) struct PredecessorWatch {
    /// The last lease seen; `Some(None)` when it was absent.
    last: Option<Option<(String, u64, RecordingLeaseState)>>,
    unchanged_since: Option<Instant>,
}

impl PredecessorWatch {
    pub(crate) fn observe(
        &mut self,
        lease: Result<Option<&RecordingLeaseRecord>, String>,
        now: Instant,
    ) -> PredecessorState {
        let lease = match lease {
            Ok(lease) => lease,
            Err(error) => return PredecessorState::Unknown(error),
        };
        if lease.is_some_and(|lease| lease.state == RecordingLeaseState::Ended) {
            return PredecessorState::Ended;
        }
        // An absent lease (a share blip, a removed file) proves nothing
        // either: it must stay that way as long as an unchanged one.
        let current = lease.map(|lease| (lease.take_id.clone(), lease.sequence, lease.state));
        if self.last.as_ref() != Some(&current) {
            self.last = Some(current);
            self.unchanged_since = Some(now);
        }
        match self.unchanged_since {
            Some(since) if now.saturating_duration_since(since) >= RECORDING_LEASE_CLEAR_AFTER => {
                PredecessorState::Quiet
            }
            _ => PredecessorState::Recording,
        }
    }
}

#[cfg(test)]
#[path = "tests/recording_lease_tests.rs"]
mod tests;
