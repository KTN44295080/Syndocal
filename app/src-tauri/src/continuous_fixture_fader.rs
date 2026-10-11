//! A bounded renderer gesture owns one existing history ticket. Each value
//! update borrows A, validates the owner and increasing sequence, then uses
//! the existing atomic engine attribute batch. Commit/Cancel still owns B/A.
use super::*;

#[derive(Debug, Default)]
pub(crate) struct Progress {
    sequence: u64,
    started: Option<Instant>,
    validated: HashSet<(FixtureId, String)>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Request {
    pub targets: Vec<TouchFeaturePresetTarget>,
    pub value: u16,
    pub sequence: u64,
    pub project_transaction_id: u64,
    pub expected_epoch: u64,
    pub owner_id: String,
}

impl Progress {
    fn prepare(
        &mut self,
        sequence: u64,
        now: Instant,
        targets: &[TouchFeaturePresetTarget],
        validate: impl Fn(FixtureId, &str) -> bool,
    ) -> Result<(), String> {
        if sequence != self.sequence + 1 || sequence > 512 {
            return Err("Fader stream sequence is stale, duplicated or exhausted".into());
        }
        if self
            .started
            .is_some_and(|start| now.saturating_duration_since(start) > Duration::from_secs(5))
        {
            return Err("Fader gesture expired; begin a new history transaction".into());
        }
        if targets.is_empty() || targets.len() > 512 {
            return Err("Fader update requires from 1 to 512 targets".into());
        }
        let mut seen = HashSet::with_capacity(targets.len());
        for target in targets {
            let key = (target.fixture_id, target.attribute.clone());
            if !seen.insert(key.clone())
                || (!self.validated.contains(&key) && !validate(key.0, &key.1))
            {
                return Err(
                    "Fader target is duplicated, missing or has no matching attribute".into(),
                );
            }
        }
        if self.validated.len() + seen.difference(&self.validated).count() > 1024 {
            return Err("Fader gesture target capacity exceeded".into());
        }
        self.validated.extend(seen);
        Ok(())
    }
}

pub(crate) fn write(state: &AppState, window: &str, mut request: Request) -> Result<(), String> {
    // This route owns its admission. Borrow the pending checkpoint: cloning
    // the entire project at every pointer sample would defeat the hot path.
    let _external = lock_project_transaction_owner_lifecycle_admission(state)?;
    let coordinator = lock_project_coordinator(state)?;
    let pending = coordinator
        .history
        .pending
        .get(&request.project_transaction_id)
        .ok_or("Fader history transaction is no longer active")?;
    if !state.project_transaction_active.load(Ordering::Acquire)
        || pending.epoch != request.expected_epoch
        || coordinator.epoch != request.expected_epoch
        || pending.owner_id != request.owner_id
        || pending.command_name != "stream_fixture_attribute_v1"
        || pending.closing
    {
        return Err("Fader transaction identity changed; nothing was applied".into());
    }
    ensure_project_transaction_pending_binding(state, pending, window, &request.owner_id)?;
    ensure_project_operator_authoritative_mutation_allowed(state, &coordinator, &request.owner_id)?;
    let lane = project_transaction_lane_for_operation(state, &pending.client_operation_id)?;
    let _admission = lane.admit()?;
    let mut progress = lane
        .fixture_fader_progress
        .lock()
        .map_err(|_| "Fader stream lock was poisoned")?;
    for target in &mut request.targets {
        target.attribute = normalize_attribute_name(target.attribute.clone())?;
    }
    let now = Instant::now();
    progress.prepare(request.sequence, now, &request.targets, |id, attribute| {
        pending
            .before
            .project
            .snapshot
            .fixtures
            .iter()
            .any(|fixture| {
                fixture.id == id
                    && fixture
                        .controls
                        .iter()
                        .any(|control| control.attribute == attribute)
            })
    })?;
    state
        .engine
        .send(EngineCommand::SetFixtureAttributeBatch {
            targets: request.targets,
            value: request.value,
        })
        .map_err(|error| error.to_string())?;
    progress.sequence = request.sequence;
    progress.started.get_or_insert(now);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(id: u64) -> TouchFeaturePresetTarget {
        TouchFeaturePresetTarget {
            fixture_id: id,
            attribute: "Dimmer".into(),
        }
    }
    #[test]
    fn increasing_values_reuse_validation_but_replay_and_invalid_batches_fail() {
        let mut progress = Progress::default();
        let now = Instant::now();
        progress
            .prepare(1, now, &[target(7)], |id, _| id == 7)
            .unwrap();
        progress.sequence = 1;
        progress.started = Some(now);
        assert!(progress.prepare(1, now, &[target(7)], |_, _| true).is_err());
        assert!(progress.prepare(3, now, &[target(7)], |_, _| true).is_err());
        assert!(progress
            .prepare(2, now, &[target(7), target(8)], |_, _| false)
            .is_err());
        assert!(!progress.validated.contains(&(8, "Dimmer".into())));
        assert!(progress
            .prepare(2, now, &[target(7), target(7)], |_, _| true)
            .is_err());
        progress
            .prepare(2, now, &[target(7)], |_, _| {
                panic!("cached target must not rescan A")
            })
            .unwrap();
    }
    #[test]
    fn gesture_lifetime_and_target_limits_are_bounded() {
        let mut progress = Progress::default();
        let now = Instant::now();
        assert!(progress.prepare(1, now, &[], |_, _| true).is_err());
        assert!(progress
            .prepare(1, now, &(1..514).map(target).collect::<Vec<_>>(), |_, _| {
                true
            })
            .is_err());
        progress
            .prepare(1, now, &(1..513).map(target).collect::<Vec<_>>(), |_, _| true)
            .unwrap();
        progress.started = Some(now);
        assert!(progress
            .prepare(1, now + Duration::from_secs(6), &[target(7)], |_, _| true)
            .is_err());
    }
}
