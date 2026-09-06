//! Persistence preflight for the fixed show-Spout activation.
//!
//! Enabling an authored disabled pair changes the project image (`enabled`),
//! so the activation cannot return the old project fence.  This module keeps
//! the candidate hash and all checked coordinator counters ready before the
//! engine/transport commit; the final coordinator update is assignment-only.

use crate::{
    apply_internal_media_asset_transaction_after_preflight, current_unix_ms,
    prepare_internal_media_asset_commit, project_checkpoint_hash, project_file_for_save_from_parts,
    synchronize_derived_video_compositions, use_authored_video_snapshot, AppState,
    PreparedInternalMediaAssetCommit, ProjectCheckpoint, ProjectCoordinator,
};
use protocol::{EngineSnapshot, VideoOutputKind};

use crate::show_spout_outputs::ShowSpoutOutputs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShowSpoutProjectChange {
    None,
    Add,
    EnableExisting,
}

#[derive(Debug, Clone)]
pub(crate) struct ShowSpoutProjectCommitPlan {
    plan: PreparedInternalMediaAssetCommit,
    rollback_hash: String,
    expected_hash: String,
}

pub(crate) fn prepare(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    expected: &ShowSpoutOutputs,
    prior_disabled: Option<&ShowSpoutOutputs>,
    change: ShowSpoutProjectChange,
) -> Result<ShowSpoutProjectCommitPlan, String> {
    if !coordinator.history.pending.is_empty() {
        return Err(
            "Show Spout project activation cannot begin while a project edit is pending"
                .to_string(),
        );
    }

    let snapshot = state.engine.persistence_snapshot()?;
    let before_project = project_file_for_save_from_parts(snapshot.clone(), &coordinator.ancillary);
    let before_hash = project_checkpoint_hash(&before_project, &coordinator.mappings)?;
    if before_hash != coordinator.checkpoint_hash {
        return Err(
            "Show Spout project image changed before activation persistence preflight".to_string(),
        );
    }

    let mut candidate = snapshot;
    // The authored image is the persistence source.  The rendered copy may
    // contain blackout/runtime state and must never hide the authored pair.
    use_authored_video_snapshot(&mut candidate);
    match change {
        ShowSpoutProjectChange::None => {}
        ShowSpoutProjectChange::Add => {
            if candidate
                .video
                .outputs
                .iter()
                .any(|output| output.kind == VideoOutputKind::SpoutSender)
            {
                return Err(
                    "Show Spout project image gained a sender before activation preflight"
                        .to_string(),
                );
            }
            candidate
                .video
                .outputs
                .extend([expected.background.clone(), expected.foreground.clone()]);
        }
        ShowSpoutProjectChange::EnableExisting => {
            let prior_disabled = prior_disabled.ok_or_else(|| {
                "Show Spout disabled-pair activation is missing its staged pair".to_string()
            })?;
            replace_existing_disabled_pair(&mut candidate, prior_disabled, expected)?;
        }
    }
    if !matches!(change, ShowSpoutProjectChange::None) {
        synchronize_derived_video_compositions(&mut candidate.video);
    }

    let after_project = project_file_for_save_from_parts(candidate, &coordinator.ancillary);
    let expected_hash = project_checkpoint_hash(&after_project, &coordinator.mappings)?;
    let changed = expected_hash != before_hash;
    if changed != !matches!(change, ShowSpoutProjectChange::None) {
        return Err(
            "Show Spout activation project preflight produced an unexpected persistence image"
                .to_string(),
        );
    }
    let before = ProjectCheckpoint {
        project: before_project,
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: before_hash,
    };
    let after = ProjectCheckpoint {
        project: after_project,
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: expected_hash.clone(),
    };
    let rollback_hash = before.hash.clone();
    let plan = prepare_internal_media_asset_commit(
        coordinator,
        "Enable Show Spout outputs",
        "",
        before,
        after,
        current_unix_ms().min(u64::MAX as u128) as u64,
    )?;
    Ok(ShowSpoutProjectCommitPlan {
        plan,
        rollback_hash,
        expected_hash,
    })
}

fn replace_existing_disabled_pair(
    snapshot: &mut EngineSnapshot,
    prior_disabled: &ShowSpoutOutputs,
    expected: &ShowSpoutOutputs,
) -> Result<(), String> {
    let mut found_background = false;
    let mut found_foreground = false;
    for output in &mut snapshot.video.outputs {
        if output.id == prior_disabled.background.id {
            if *output != prior_disabled.background {
                return Err(
                    "Show Spout staged Background changed before activation preflight".to_string(),
                );
            }
            *output = expected.background.clone();
            found_background = true;
        } else if output.id == prior_disabled.foreground.id {
            if *output != prior_disabled.foreground {
                return Err(
                    "Show Spout staged Foreground changed before activation preflight".to_string(),
                );
            }
            *output = expected.foreground.clone();
            found_foreground = true;
        }
    }
    if !found_background || !found_foreground {
        return Err(
            "Show Spout staged disabled pair was replaced before activation preflight".to_string(),
        );
    }
    Ok(())
}

impl ShowSpoutProjectCommitPlan {
    pub(crate) fn verify(
        &self,
        state: &AppState,
        coordinator: &ProjectCoordinator,
    ) -> Result<(), String> {
        let snapshot = state.engine.persistence_snapshot()?;
        let project = project_file_for_save_from_parts(snapshot, &coordinator.ancillary);
        let actual_hash = project_checkpoint_hash(&project, &coordinator.mappings)?;
        if actual_hash != self.expected_hash {
            return Err(
                "Show Spout engine publication produced an unexpected project image".to_string(),
            );
        }
        Ok(())
    }

    pub(crate) fn verify_rollback(
        &self,
        state: &AppState,
        coordinator: &ProjectCoordinator,
    ) -> Result<(), String> {
        let snapshot = state.engine.persistence_snapshot()?;
        let project = project_file_for_save_from_parts(snapshot, &coordinator.ancillary);
        let actual_hash = project_checkpoint_hash(&project, &coordinator.mappings)?;
        if actual_hash != self.rollback_hash {
            return Err(
                "Show Spout candidate rollback produced an unexpected project image".to_string(),
            );
        }
        Ok(())
    }

    /// Commit only values checked in `prepare`; no fallible operation is
    /// permitted after the engine and native pair have been published. The
    /// prepared history entry retains ordinary Undo semantics for the authored
    /// output change.
    pub(crate) fn commit(self, coordinator: &mut ProjectCoordinator) {
        apply_internal_media_asset_transaction_after_preflight(coordinator, self.plan);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_pair_replacement_preserves_ids_and_only_enables_the_pair() {
        let fixture = crate::show_spout_outputs::build_show_spout_outputs(7, 8, 3, 2)
            .expect("show Spout project fixture");
        let mut disabled_background = fixture.background;
        disabled_background.enabled = false;
        let mut disabled_foreground = fixture.foreground;
        disabled_foreground.enabled = false;
        let prior = ShowSpoutOutputs {
            background: disabled_background.clone(),
            foreground: disabled_foreground.clone(),
        };
        let mut expected_background = disabled_background.clone();
        expected_background.enabled = true;
        let mut expected_foreground = disabled_foreground.clone();
        expected_foreground.enabled = true;
        let expected = ShowSpoutOutputs {
            background: expected_background.clone(),
            foreground: expected_foreground.clone(),
        };
        let mut snapshot = EngineSnapshot::default();
        snapshot.video.outputs = vec![disabled_background, disabled_foreground];
        replace_existing_disabled_pair(&mut snapshot, &prior, &expected).unwrap();
        assert_eq!(
            snapshot.video.outputs,
            vec![expected_background, expected_foreground]
        );
    }
}
