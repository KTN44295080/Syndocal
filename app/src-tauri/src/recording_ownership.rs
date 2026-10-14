//! Recording follows video output ownership (DEC-RECORD-OWN-001; owner
//! answer 2026-10-14: on every setup, including a single PC).
//!
//! Only a machine whose ownership gate is Ready with video allowed may
//! start a take, and never while it runs Standby sync or a split-brain
//! (two Primary sessions) is detected. The frame loop re-proves the admitted
//! ownership epoch for every frame, so a role change, project open, swap or
//! Undo/Redo (each retires output), Standby or a failure fence ends the take;
//! the frames already captured are finalized and published, never abandoned.
//!
//! A split-brain or another PC's Take Over that begins during a take does
//! not end it yet: that needs the two-PC recording lease (plan phase 3).

use super::*;

/// The ownership a take was admitted under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RecordingAdmission {
    pub owner_epoch: u64,
}

/// Standby sync as seen by recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct RecordingSyncContext {
    pub running_as_standby: bool,
    pub split_brain: bool,
}

/// The admission rule itself, kept free of app state for tests.
pub(super) fn recording_admission_from(
    ownership: &OutputOwnershipStatus,
    sync: RecordingSyncContext,
) -> Result<RecordingAdmission, String> {
    if ownership.state != protocol::OutputOwnershipState::Ready || !ownership.video_allowed {
        return Err(
            "Recording needs this PC to own video output: arm output as Video or Both first."
                .to_string(),
        );
    }
    if sync.running_as_standby {
        return Err("This PC runs as Standby; only the Primary records.".to_string());
    }
    if sync.split_brain {
        return Err(
            "Two Primary sessions were detected; recording stays off until only one remains."
                .to_string(),
        );
    }
    Ok(RecordingAdmission {
        owner_epoch: ownership.epoch,
    })
}

fn recording_sync_context(state: &AppState) -> Result<RecordingSyncContext, String> {
    // Project replacement, Stop and Take Over hold this lock while they join
    // the sync worker. Admission never waits for that: it is refused until
    // the change settles.
    let runtime = match state.standby_sync.try_lock() {
        Ok(runtime) => runtime,
        Err(std::sync::TryLockError::WouldBlock) => {
            return Err("Standby sync is changing; recording waits until it settles.".to_string())
        }
        Err(std::sync::TryLockError::Poisoned(_)) => {
            return Err("Standby sync lock was poisoned; recording stays off".to_string())
        }
    };
    let status = runtime
        .status
        .lock()
        .map_err(|_| "Standby sync status lock was poisoned; recording stays off".to_string())?;
    Ok(RecordingSyncContext {
        running_as_standby: status.running && status.role == Some(StandbySyncRole::Standby),
        split_brain: status.split_brain,
    })
}

/// Whether this machine may start a take now.
pub(super) fn admit_recording(state: &AppState) -> Result<RecordingAdmission, String> {
    let sync = recording_sync_context(state)?;
    recording_admission_from(&state.engine.output_ownership_status(), sync)
}

/// `status` with the admission as of now, not as of its take's start.
pub(super) fn with_current_admission(
    state: &AppState,
    mut status: VideoRecordingStatus,
) -> VideoRecordingStatus {
    let admission = admit_recording(state);
    status.admission_allowed = admission.is_ok();
    status.admission_reason = admission.err();
    status
}

#[cfg(test)]
#[path = "tests/recording_ownership_tests.rs"]
mod tests;
