//! The takeover take (DEC-RECORD-OWN-001, phase 3; owner answer 2
//! 2026-10-14): after a Standby PC takes over and arms video output, it
//! continues the predecessor's recording automatically into a configured
//! local folder, without a dialog. It starts only once the predecessor's
//! take has provably ended (see `recording_lease`); until then it is
//! Pending, and without a folder it is Blocked, never silent.

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use serde::{Deserialize, Serialize};

use super::recording_lease::{
    self, PredecessorState, PredecessorWatch, RecordingLeaseRecord, RecordingLeaseState,
    RecordingTakeoverRecord,
};

pub(crate) const TAKEOVER_RECORDING_SETTINGS_FILE: &str = "takeover-recording-v1.json";
const TAKEOVER_RECORDING_SETTINGS_VERSION: u32 = 1;
const MAX_SETTINGS_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TakeoverRecordingSettings {
    pub(crate) version: u32,
    pub(crate) folder: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TakeoverRecordingState {
    /// No takeover is waiting to record.
    Idle,
    /// Waiting for Arm or for the predecessor's take to end.
    Pending,
    /// Cannot start until the operator acts (no folder, receipt failure).
    Blocked,
    Started,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct TakeoverRecordingStatus {
    pub(crate) state: TakeoverRecordingState,
    pub(crate) folder: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) predecessor_take_id: Option<String>,
}

impl Default for TakeoverRecordingStatus {
    fn default() -> Self {
        Self {
            state: TakeoverRecordingState::Idle,
            folder: None,
            reason: None,
            predecessor_take_id: None,
        }
    }
}

/// What the operator can do with the takeover take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TakeoverRecordingAction {
    ChooseFolder,
    ClearFolder,
    CancelPending,
}

/// The takeover take on this PC. A new generation supersedes the recorder
/// thread of the previous one, which then exits without acting.
#[derive(Debug, Default)]
pub(crate) struct TakeoverRecorder {
    pub(crate) status: TakeoverRecordingStatus,
    pub(crate) generation: u64,
}

impl TakeoverRecorder {
    /// Replaces any waiting takeover; returns the new generation.
    pub(crate) fn supersede(&mut self, status: TakeoverRecordingStatus) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.status = status;
        self.generation
    }

    /// Records the outcome of a generation, unless it was superseded.
    pub(crate) fn finish(
        &mut self,
        generation: u64,
        state: TakeoverRecordingState,
        reason: Option<String>,
    ) {
        if self.generation == generation {
            self.generation = self.generation.wrapping_add(1);
            self.status.state = state;
            self.status.reason = reason;
        }
    }

    pub(crate) fn cancel_pending(&mut self) {
        if matches!(
            self.status.state,
            TakeoverRecordingState::Pending | TakeoverRecordingState::Blocked
        ) {
            let generation = self.generation;
            self.finish(
                generation,
                TakeoverRecordingState::Cancelled,
                Some("Cancelled by the operator".to_string()),
            );
        }
    }
}

pub(crate) fn settings_path(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join(TAKEOVER_RECORDING_SETTINGS_FILE)
}

pub(crate) fn load_settings(path: &Path) -> Result<TakeoverRecordingSettings, String> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TakeoverRecordingSettings {
                version: TAKEOVER_RECORDING_SETTINGS_VERSION,
                folder: None,
            })
        }
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take(MAX_SETTINGS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_SETTINGS_BYTES {
        return Err(format!("{} is too large", path.display()));
    }
    let settings = serde_json::from_slice::<TakeoverRecordingSettings>(&bytes)
        .map_err(|error| format!("{} is invalid: {error}", path.display()))?;
    if settings.version != TAKEOVER_RECORDING_SETTINGS_VERSION {
        return Err(format!("{} has an unsupported version", path.display()));
    }
    Ok(settings)
}

/// Saves the folder atomically; a missing app data folder is created.
pub(crate) fn persist_settings(path: &Path, folder: Option<String>) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent folder", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create {}: {error}", parent.display()))?;
    let bytes = serde_json::to_vec_pretty(&TakeoverRecordingSettings {
        version: TAKEOVER_RECORDING_SETTINGS_VERSION,
        folder,
    })
    .map_err(|error| format!("Cannot encode takeover recording settings: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)
            .map_err(|error| format!("Cannot write {}: {error}", temporary.display()))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("Cannot write {}: {error}", temporary.display()))?;
    }
    super::replace_file_atomically(&temporary, path)
}

/// What the takeover recorder continues: the predecessors' takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TakeoverPlan {
    pub(crate) directory: PathBuf,
    /// Every predecessor lease still recording at Take Over, the followed
    /// Primary first. The takeover take continues the first one's output,
    /// frame rate and audio choice.
    pub(crate) predecessors: Vec<RecordingLeaseRecord>,
    /// The sessions waited for: every recording predecessor and every
    /// session whose lease could not be read at Take Over.
    pub(crate) watched_sessions: Vec<String>,
    /// The receipt telling the predecessors to stop; retried until written.
    pub(crate) takeover: RecordingTakeoverRecord,
    /// The project the takeover loaded; another project cancels the plan.
    pub(crate) project_epoch: u64,
}

/// The predecessor leases still recording, and the sessions whose lease
/// could not be read (with the error).
pub(crate) fn recording_predecessors(
    directory: &Path,
    sessions: &[String],
) -> (Vec<RecordingLeaseRecord>, Vec<(String, String)>) {
    let mut recording = Vec::new();
    let mut unreadable = Vec::new();
    for session in sessions {
        match recording_lease::read_lease(directory, session) {
            Ok(Some(lease)) if lease.state == RecordingLeaseState::Recording => {
                recording.push(lease)
            }
            Ok(_) => {}
            Err(error) => unreadable.push((session.clone(), error)),
        }
    }
    (recording, unreadable)
}

/// One decision of the takeover recorder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TakeoverStep {
    Wait(TakeoverRecordingState, String),
    Start,
    Cancel(String),
}

/// What the recorder observed on one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TakeoverObservation<'a> {
    pub(crate) folder: Option<&'a str>,
    pub(crate) project_epoch: u64,
    pub(crate) admission: Result<(), String>,
    pub(crate) predecessor: PredecessorState,
    /// None once the takeover receipt is written; otherwise why it is not.
    pub(crate) receipt_error: Option<&'a str>,
}

/// The pure decision, kept free of app state for tests.
pub(crate) fn decide(plan: &TakeoverPlan, observation: TakeoverObservation<'_>) -> TakeoverStep {
    if observation.project_epoch != plan.project_epoch {
        return TakeoverStep::Cancel(
            "Another project was opened after Take Over; the takeover take was cancelled"
                .to_string(),
        );
    }
    if observation.folder.is_none() {
        return TakeoverStep::Wait(
            TakeoverRecordingState::Blocked,
            "Choose a takeover recording folder to continue the previous Primary's recording"
                .to_string(),
        );
    }
    if let Err(reason) = observation.admission {
        return TakeoverStep::Wait(TakeoverRecordingState::Pending, reason);
    }
    match observation.predecessor {
        PredecessorState::Ended => TakeoverStep::Start,
        PredecessorState::Quiet => match observation.receipt_error {
            None => TakeoverStep::Start,
            // A quiet lease proves nothing while the old Primary was never
            // told to stop: it may only be paused.
            Some(error) => TakeoverStep::Wait(
                TakeoverRecordingState::Blocked,
                format!(
                    "The previous Primary was not told to stop (the takeover receipt could not be written: {error}); its take may still be running"
                ),
            ),
        },
        PredecessorState::Recording => TakeoverStep::Wait(
            TakeoverRecordingState::Pending,
            "Waiting for the previous Primary's take to end".to_string(),
        ),
        PredecessorState::Unknown(error) => TakeoverStep::Wait(
            TakeoverRecordingState::Pending,
            format!("The previous Primary's recording lease cannot be read: {error}"),
        ),
    }
}

/// Watches every predecessor lease between decisions.
#[derive(Debug, Default)]
pub(crate) struct PredecessorProbe {
    watches: Vec<PredecessorWatch>,
}

impl PredecessorProbe {
    /// The least settled state over all predecessors.
    pub(crate) fn observe(&mut self, plan: &TakeoverPlan, now: Instant) -> PredecessorState {
        self.watches
            .resize_with(plan.watched_sessions.len(), PredecessorWatch::default);
        let mut combined = PredecessorState::Ended;
        for (watch, session) in self.watches.iter_mut().zip(&plan.watched_sessions) {
            let lease = recording_lease::read_lease(&plan.directory, session);
            let state = watch.observe(
                lease.as_ref().map(Option::as_ref).map_err(Clone::clone),
                now,
            );
            combined = match (combined, state) {
                (PredecessorState::Recording, _) | (_, PredecessorState::Recording) => {
                    PredecessorState::Recording
                }
                (PredecessorState::Unknown(error), _) | (_, PredecessorState::Unknown(error)) => {
                    PredecessorState::Unknown(error)
                }
                (PredecessorState::Quiet, _) | (_, PredecessorState::Quiet) => {
                    PredecessorState::Quiet
                }
                (PredecessorState::Ended, PredecessorState::Ended) => PredecessorState::Ended,
            };
        }
        combined
    }
}

#[cfg(test)]
#[path = "tests/takeover_recording_tests.rs"]
mod tests;
