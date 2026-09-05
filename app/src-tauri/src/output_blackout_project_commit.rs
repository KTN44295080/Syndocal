//! Preflight and verify the persisted target-blackout image before publishing its fence.
use crate::{
    apply_internal_media_asset_transaction_after_preflight, current_unix_ms,
    prepare_internal_media_asset_commit, project_checkpoint_hash, project_file_for_save_from_parts,
    PreparedInternalMediaAssetCommit, ProjectCheckpoint, ProjectCoordinator,
};
use engine::EnginePersistenceMutationSubmission;
use protocol::control_plane_command::OutputControlTargetRoleV1;

pub(super) struct BlackoutProjectCommit {
    plan: PreparedInternalMediaAssetCommit,
    expected_hash: String,
    changed: bool,
}

pub(super) fn prepare(
    submission: &EnginePersistenceMutationSubmission<'_>,
    coordinator: &ProjectCoordinator,
    target: OutputControlTargetRoleV1,
    enabled: bool,
) -> Result<BlackoutProjectCommit, String> {
    let snapshot = submission.persistence_snapshot()?;
    let before = ProjectCheckpoint {
        project: project_file_for_save_from_parts(snapshot.clone(), &coordinator.ancillary),
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: coordinator.checkpoint_hash.clone(),
    };
    let mut candidate = snapshot;
    // Project persistence selects authored_video over the rendered video image.
    // Move that authoritative image into place before editing its blackout bit.
    crate::use_authored_video_snapshot(&mut candidate);
    if matches!(
        target,
        OutputControlTargetRoleV1::Lighting | OutputControlTargetRoleV1::Both
    ) {
        candidate.blackout = enabled;
        candidate.authored_blackout = enabled;
    }
    if matches!(
        target,
        OutputControlTargetRoleV1::Video | OutputControlTargetRoleV1::Both
    ) {
        candidate.video.blackout = enabled;
    }
    let after = ProjectCheckpoint {
        project: project_file_for_save_from_parts(candidate, &coordinator.ancillary),
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: String::new(),
    };
    let expected_hash = project_checkpoint_hash(&after.project, &after.mappings)?;
    let changed = expected_hash != before.hash;
    let plan = prepare_internal_media_asset_commit(
        coordinator,
        "Set Output Blackout",
        "",
        before,
        after,
        current_unix_ms().min(u64::MAX as u128) as u64,
    )?;
    Ok(BlackoutProjectCommit {
        plan,
        expected_hash,
        changed,
    })
}

impl BlackoutProjectCommit {
    pub(super) fn changed(&self) -> bool {
        self.changed
    }

    pub(super) fn verify(
        &self,
        submission: &EnginePersistenceMutationSubmission<'_>,
        coordinator: &ProjectCoordinator,
        applied: bool,
    ) -> Result<(), String> {
        let actual = submission.persistence_snapshot().and_then(|snapshot| {
            let project = project_file_for_save_from_parts(snapshot, &coordinator.ancillary);
            project_checkpoint_hash(&project, &coordinator.mappings)
        })?;
        if !applied || actual != self.expected_hash {
            return Err("Target blackout published an unexpected project image; project mutations remain fenced until restart".to_string());
        }
        Ok(())
    }

    pub(super) fn commit(self, coordinator: &mut ProjectCoordinator) {
        apply_internal_media_asset_transaction_after_preflight(coordinator, self.plan);
    }
}
