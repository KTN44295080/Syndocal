//! Candidate-only project preparation through the canonical Engine load path.
use super::*;

fn prepare_runtime(snapshot: EngineSnapshot) -> Result<EngineRuntime, String> {
    // This is an unstarted, private runtime. Standby denies both capabilities
    // for its entire lifetime, including enabled routes carried by the input.
    // Do not use EngineHandle::start: preparation owns no worker or device.
    let mut runtime = EngineRuntime::new_with_shared_telemetry_and_ownership(
        DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        },
        Arc::new(EngineSharedTelemetry::new()),
        OutputOwnershipGate::for_role(MachineOutputRole::Standby),
        Arc::new(RwLock::new(HashMap::new())),
        Arc::new(RwLock::new(TimelineAudioProjectionAuthority::default())),
        Arc::new(AtomicU64::new(1)),
        Arc::new(AtomicU64::new(1)),
        #[cfg(test)]
        (
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ),
    );
    // Decide legacy/current-schema from the original candidate exactly once,
    // before normalization. Invalid current references must still reject.
    runtime.load_project_snapshot_checked(snapshot)?;
    Ok(runtime)
}

/// Validate and prepare the Engine's persistent image before project effects.
/// Uses the real load/persistence primitives without a thread, tick, renderer,
/// media-file read, or output permit. Application-only persistence projection
/// (clock/preview/telemetry) remains the application's responsibility.
pub fn prepare_project_snapshot_persistence(
    snapshot: EngineSnapshot,
) -> Result<EngineSnapshot, String> {
    Ok(prepare_runtime(snapshot)?.build_persistence_snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_load_preparation_preserves_enabled_routes_without_opening_senders() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.output.enabled = true;
        snapshot.dmx_outputs = vec![
            snapshot.output.clone(),
            DmxOutputConfig {
                enabled: true,
                universe: 7,
                ..DmxOutputConfig::default()
            },
        ];
        let expected = snapshot.dmx_outputs.clone();
        let runtime = prepare_runtime(snapshot).unwrap();
        assert_eq!(
            runtime.output_ownership_gate.status().effective_role,
            MachineOutputRole::Standby
        );
        assert!(!runtime.output_ownership_gate.status().lighting_allowed);
        assert!(!runtime.output_ownership_gate.status().video_allowed);
        assert!(runtime.dmx_sender.is_none());
        assert!(runtime
            .additional_dmx_outputs
            .iter()
            .all(|route| route.sender.is_none()));
        assert_eq!(runtime.build_persistence_snapshot().dmx_outputs, expected);
    }

    #[test]
    fn project_load_preparation_rejects_current_schema_before_normalization() {
        let mut snapshot = prepare_project_snapshot_persistence(EngineSnapshot::default()).unwrap();
        snapshot.timeline_bank[0].label = "Conflicting authored bank".into();
        assert!(prepare_project_snapshot_persistence(snapshot)
            .unwrap_err()
            .contains("reference integrity"));
    }

    #[test]
    fn project_load_preparation_canonical_default_is_idempotent() {
        let first = prepare_project_snapshot_persistence(EngineSnapshot::default()).unwrap();
        let second = prepare_project_snapshot_persistence(first.clone()).unwrap();
        assert_eq!(first.timeline, second.timeline);
        assert_eq!(first.timeline_bank, second.timeline_bank);
        assert_eq!(first.authored_video, second.authored_video);
        assert_eq!(first.fixtures, second.fixtures);
        assert_eq!(first.clock.bpm, second.clock.bpm);
    }
}
