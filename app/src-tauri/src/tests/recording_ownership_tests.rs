use super::*;

#[test]
fn recording_admission_requires_ready_video_ownership_and_no_standby_or_split_brain() {
    let clear = RecordingSyncContext::default();
    for role in [MachineOutputRole::Video, MachineOutputRole::Both] {
        let admission =
            recording_admission_from(&OutputOwnershipStatus::ready(role, Some(role), 3, 7), clear)
                .expect("a Ready video owner records");
        assert_eq!(admission.owner_epoch, 7);
    }
    for role in [MachineOutputRole::Lighting, MachineOutputRole::Standby] {
        let error =
            recording_admission_from(&OutputOwnershipStatus::ready(role, Some(role), 3, 7), clear)
                .unwrap_err();
        assert!(error.contains("own video output"), "{role:?}: {error}");
    }
    // Startup all-deny, an in-flight transition and a failure fence refuse.
    for status in [
        OutputOwnershipStatus::default(),
        OutputOwnershipStatus::transitioning(MachineOutputRole::Both, None, 3, 8),
        OutputOwnershipStatus::failed(
            MachineOutputRole::Both,
            None,
            3,
            9,
            protocol::OutputOwnershipReason::TransitionFailed,
            "fence".to_string(),
        ),
    ] {
        assert!(
            recording_admission_from(&status, clear).is_err(),
            "{status:?}"
        );
    }
    let both = OutputOwnershipStatus::ready(MachineOutputRole::Both, None, 3, 7);
    let standby = RecordingSyncContext {
        running_as_standby: true,
        split_brain: false,
    };
    assert!(recording_admission_from(&both, standby)
        .unwrap_err()
        .contains("Standby"));
    let split = RecordingSyncContext {
        running_as_standby: false,
        split_brain: true,
    };
    assert!(recording_admission_from(&both, split)
        .unwrap_err()
        .contains("Two Primary"));
}
