//! A fresh empty project has no outputs to energize. Do not carry the temporary
//! project-retirement blackout into its otherwise empty runtime.
use crate::{
    lock_project_coordinator, lock_project_external_command_admission, AppState, ProjectLoadResult,
};
use std::time::{Duration, Instant};

pub(crate) fn clear_empty_project_blackout(
    state: &AppState,
    loaded: &mut ProjectLoadResult,
) -> Result<(), String> {
    let _admission = lock_project_external_command_admission(state)?;
    let coordinator = lock_project_coordinator(state)?;
    if coordinator.epoch != loaded.project_epoch
        || coordinator.revision != loaded.project_revision
        || coordinator.checkpoint_hash != loaded.checkpoint_hash
    {
        return Ok(()); // A newer replacement owns its own output state.
    }
    let snapshot = state.engine.snapshot();
    if !snapshot.fixtures.is_empty()
        || snapshot.output.enabled
        || snapshot.dmx_outputs.iter().any(|output| output.enabled)
        || snapshot.video.outputs.iter().any(|output| output.enabled)
    {
        return Err(
            "New project output reset requires an empty project with all outputs disabled"
                .to_string(),
        );
    }
    let safety = state.engine.safety_blackout_authority();
    if safety.engaged {
        state
            .engine
            .safety_blackout_release_published(
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(2),
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(authority) = loaded.authority.as_mut() {
        authority.snapshot = state.engine.snapshot();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_empty_project_clears_retirement_blackout_without_enabling_outputs() {
        let harness = crate::tests::MediaAssetA6CommandHarness::new();
        let state = &harness.state;
        state
            .engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(2))
            .unwrap();
        let coordinator = state.project_coordinator.lock().unwrap();
        let mut loaded = crate::ProjectLoadResult {
            path: "New project".to_string(),
            current_project_path: None,
            profiles: Vec::new(),
            midi_mappings: Vec::new(),
            osc_mappings: Vec::new(),
            dmx_mappings: Vec::new(),
            dj_track_triggers: Vec::new(),
            warnings: Vec::new(),
            project_epoch: coordinator.epoch,
            project_revision: coordinator.revision,
            checkpoint_hash: coordinator.checkpoint_hash.clone(),
            history_generation: coordinator.history_generation,
            authority_disposition_generation: coordinator.authority_disposition_generation,
            authority_disposition: coordinator.authority_disposition,
            authority: None,
        };
        drop(coordinator);
        clear_empty_project_blackout(state, &mut loaded).unwrap();
        let snapshot = state.engine.snapshot();
        assert!(!snapshot.safety_blackout_engaged);
        assert!(!snapshot.output.enabled);
        assert!(snapshot.dmx_outputs.iter().all(|output| !output.enabled));
        assert!(snapshot.video.outputs.iter().all(|output| !output.enabled));
        // A stale earlier New reply must not clear a later project latch.
        state
            .engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(2))
            .unwrap();
        loaded.project_epoch += 1;
        clear_empty_project_blackout(state, &mut loaded).unwrap();
        assert!(state.engine.safety_blackout_authority().engaged);
    }
}
