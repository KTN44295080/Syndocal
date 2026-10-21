//! A previous-project lease is retired and a fresh identity is acquired in
//! one canonical receipt. Other operations retain their one-change contract.
use super::*;

impl OutputControlLeaseResultV2 {
    pub(super) fn validate_video_authoring_replacement(&self) -> Result<(), OutputControlValidationErrorV1> {
        let invalid = OutputControlValidationErrorV1::InvalidReceiptOutcome;
        self.authority.validate()?;
        if self.changes.len() != 2 || self.audit_sequence == 0
            || self.audit_sequence > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.outcome != OutputLeaseReceiptOutcomeV2::Acquired
            || self.phase != OutputLeaseReceiptPhaseV2::HeldActive
            || !(self.resources == [OutputControlTargetRoleV1::Video]
                || self.resources == [OutputControlTargetRoleV1::Lighting, OutputControlTargetRoleV1::Video]) {
            return Err(invalid);
        }
        let new = self.changes.iter().find(|change| change.lease_id == self.authority.lease_id).ok_or(invalid)?;
        let old = self.changes.iter().find(|change| change.lease_id != self.authority.lease_id).ok_or(invalid)?;
        let before = old.before_generation.ok_or(invalid)?;
        OutputLeaseAuthorityV1 { lease_id: old.lease_id.clone(), generation: before }.validate()?;
        if new.before_generation.is_some() || new.before_phase.is_some() || !new.before_resources.is_empty()
            || new.after_generation != Some(self.authority.generation)
            || new.after_phase != Some(OutputLeaseReceiptPhaseV2::HeldActive)
            || new.after_resources != self.resources
            || !matches!(old.before_phase, Some(OutputLeaseReceiptPhaseV2::HeldActive | OutputLeaseReceiptPhaseV2::HeldOrphaned))
            || old.before_resources != self.resources
            || old.after_generation != before.checked_add(1)
            || old.after_generation.is_none_or(|value| value > MAX_SAFE_JAVASCRIPT_INTEGER)
            || old.after_phase != Some(OutputLeaseReceiptPhaseV2::Unclaimed)
            || !old.after_resources.is_empty() {
            return Err(invalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn replacement() -> OutputControlLeaseResultV2 {
        let resources = vec![OutputControlTargetRoleV1::Video];
        OutputControlLeaseResultV2 {
            authority: OutputLeaseAuthorityV1 { lease_id: "lease-0000000000000002".into(), generation: 1 },
            resources: resources.clone(), phase: OutputLeaseReceiptPhaseV2::HeldActive,
            outcome: OutputLeaseReceiptOutcomeV2::Acquired, audit_sequence: 1,
            changes: vec![
                OutputLeaseReceiptChangeV2 { lease_id: "lease-0000000000000002".into(), before_generation: None,
                    after_generation: Some(1), before_resources: vec![], after_resources: resources.clone(),
                    before_phase: None, after_phase: Some(OutputLeaseReceiptPhaseV2::HeldActive) },
                OutputLeaseReceiptChangeV2 { lease_id: "lease-0000000000000001".into(), before_generation: Some(7),
                    after_generation: Some(8), before_resources: resources, after_resources: vec![],
                    before_phase: Some(OutputLeaseReceiptPhaseV2::HeldOrphaned), after_phase: Some(OutputLeaseReceiptPhaseV2::Unclaimed) },
            ],
        }
    }
    #[test]
    fn video_authoring_replacement_receipt_requires_exact_retire_and_fresh_acquire() {
        let receipt = replacement();
        receipt.validate_for_operation(OUTPUT_VIDEO_AUTHORING_LEASE_REPLACE_OPERATION_ID).unwrap();
        assert!(receipt.validate_for_operation(OUTPUT_LEASE_RECOVER_OPERATION_ID).is_err());
        assert!(receipt.validate_for_operation(OUTPUT_LEASE_ACQUIRE_OPERATION_ID).is_err());
        let mutations: &[fn(&mut OutputControlLeaseResultV2)] = &[
            |r| r.changes[1].lease_id = r.authority.lease_id.clone(),
            |r| r.changes[1].after_generation = Some(9),
            |r| r.changes[1].after_phase = Some(OutputLeaseReceiptPhaseV2::HeldActive),
            |r| r.changes[1].before_resources = vec![OutputControlTargetRoleV1::Lighting, OutputControlTargetRoleV1::Video],
            |r| r.changes[0].before_generation = Some(1),
            |r| r.changes.pop().map(|_| ()).unwrap(),
        ];
        for mutate in mutations {
            let mut corrupt = receipt.clone(); mutate(&mut corrupt);
            assert!(corrupt.validate_for_operation(OUTPUT_VIDEO_AUTHORING_LEASE_REPLACE_OPERATION_ID).is_err());
        }
    }
    #[test]
    fn video_authoring_replacement_action_has_distinct_canonical_identity() {
        let action = OutputControlActionV2::ReplaceVideoAuthoringLease {
            lease: OutputLeaseAuthorityV1 { lease_id: "lease-0000000000000001".into(), generation: 7 },
        };
        assert_eq!(action.operation_id(), OUTPUT_VIDEO_AUTHORING_LEASE_REPLACE_OPERATION_ID);
        let wire = serde_json::to_value(&action).unwrap();
        assert_eq!(serde_json::from_value::<OutputControlActionV2>(wire.clone()).unwrap(), action);
        let mut invalid = wire; invalid["project_identity"] = "renderer-controlled".into();
        assert!(serde_json::from_value::<OutputControlActionV2>(invalid).is_err());
    }
}
