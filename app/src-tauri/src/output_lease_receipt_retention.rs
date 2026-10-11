//! Bound the canonical/public receipt pair together. Physical pending barriers
//! and the fixed one-shot probe receipts cannot be evicted by ordinary traffic.
use super::*;

pub(super) fn trim(state: &mut PersistedOutputLeaseReceiptState) -> Result<(), output_lease::OutputLeaseError> {
    while state.receipts.len() > MAX_DURABLE_OUTPUT_LEASE_RECEIPTS {
        // The newest receipt is the operation currently committing. Never
        // report success after evicting the very receipt being published.
        let index = state.receipts.iter().take(state.receipts.len() - 1).position(|receipt| {
            receipt.key.domain != OUTPUT_LEASE_DSF2026_PROBE_DOMAIN
                && receipt.key.domain != OUTPUT_LEASE_DSF2026_PROBE_RECONCILE_DOMAIN
                && !state.managed_exact_both_output_control_pending_terminals.iter().any(|pending| {
                    pending.private_receipt.key == receipt.key || (pending.principal == receipt.key.principal
                        && pending.request_id == receipt.key.request_id
                        && agent_bridge_output::receipt_domain_matches_operation(&receipt.key.domain, &pending.operation_id))
                })
        }).ok_or(output_lease::OutputLeaseError::RequestCapacity)?;
        let evicted = state.receipts.remove(index);
        let before = state.managed_exact_both_output_control_terminals.len();
        state.managed_exact_both_output_control_terminals.retain(|terminal| {
            !matches!(&terminal.response, OutputControlResponseV2::Receipt(public)
                if terminal.principal == evicted.key.principal && public.request_id == evicted.key.request_id
                    && agent_bridge_output::receipt_domain_matches_operation(&evicted.key.domain, &public.operation_id))
        });
        if before != state.managed_exact_both_output_control_terminals.len() {
            // A retired physical terminal must never become a fresh physical
            // request after its last ordinary receipt leaves this origin.
            if let Some(origin) = state.origins.iter_mut().find(|origin| {
                origin.principal == evicted.key.principal && origin.domain == evicted.key.domain
            }) { origin.retired_physical_replay_guard = true; }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PersistedOutputLeaseReceiptState {
        let mut state = serde_json::from_str(include_str!("../tests/fixtures/output-receipt-retention.json")).unwrap();
        assert!(migrate_output_lease_receipt_state_to_current(&mut state).unwrap());
        state
    }
    #[test]
    fn full_journal_advances_public_private_receipts_atomically_and_keeps_replay_guard_after_restart() {
        let mut state = fixture();
        validate_output_lease_receipt_state(&state).unwrap();
        let original = state.receipts[0].clone();
        for id in 1..=MAX_DURABLE_OUTPUT_LEASE_RECEIPTS + 2 {
            let mut receipt = original.clone();
            receipt.key.request_id = original.key.request_id + id as u64;
            receipt.audit_sequence += id as u64;
            OutputLeaseDurableReceiptJournal::record_receipt_in_candidate(&mut state, &receipt).unwrap();
            validate_output_lease_receipt_state(&state).unwrap();
        }
        assert_eq!(state.receipts.len(), MAX_DURABLE_OUTPUT_LEASE_RECEIPTS);
        assert!(state.managed_exact_both_output_control_terminals.is_empty());
        for id in 1..=MAX_DURABLE_OUTPUT_LEASE_RECEIPTS {
            let mut receipt = original.clone();
            receipt.key.domain = "retention-other-domain".to_string();
            receipt.key.request_id += id as u64;
            receipt.audit_sequence += id as u64;
            OutputLeaseDurableReceiptJournal::record_receipt_in_candidate(&mut state, &receipt).unwrap();
        }
        let reloaded = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        validate_output_lease_receipt_state(&reloaded).unwrap();
        let mut journal = OutputLeaseDurableReceiptJournal::in_memory(); journal.state = reloaded;
        let origin = journal.state.origins.iter().find(|origin| origin.principal == original.key.principal && origin.domain == original.key.domain).unwrap();
        assert!(origin.retired_physical_replay_guard);
        assert!(journal.origin_is_retained(origin));
        assert!(origin.high_water_request_id > original.key.request_id);
        let retired = OutputLeaseRequest::from_action(original.key.principal.clone(), original.key.domain.clone(),
            original.key.request_id, OutputLeaseRequestAction::ObserveExpiry { lease_id: original.lease_id.unwrap() }).unwrap();
        assert_eq!(journal.lookup(&retired), Err(output_lease::OutputLeaseError::ReceiptNotRetained));
    }
    #[test]
    fn legacy_two_migrates_once_and_future_or_mislabeled_guards_are_rejected() {
        let state = fixture(); assert_eq!(state.version, 3);
        let mut current = state.clone(); assert!(!migrate_output_lease_receipt_state_to_current(&mut current).unwrap());
        let mut future = state.clone(); future.version = 4; assert!(migrate_output_lease_receipt_state_to_current(&mut future).is_err());
        let mut invalid = state; invalid.version = 2; invalid.origins[0].retired_physical_replay_guard = true;
        assert!(migrate_output_lease_receipt_state_to_current(&mut invalid).is_err());
    }
    #[test]
    fn one_shot_probe_receipts_survive_ordinary_receipt_pressure() {
        let mut state = fixture();
        state.managed_exact_both_output_control_terminals.clear();
        state.receipts[0].key.domain = OUTPUT_LEASE_DSF2026_PROBE_DOMAIN.to_string();
        let pinned = state.receipts[0].clone();
        for id in 1..=MAX_DURABLE_OUTPUT_LEASE_RECEIPTS + 2 {
            let mut receipt = pinned.clone(); receipt.key.domain = OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN.to_string();
            receipt.key.request_id += id as u64; state.receipts.push(receipt); trim(&mut state).unwrap();
        }
        assert!(state.receipts.iter().any(|receipt| receipt.key == pinned.key));
        assert_eq!(state.receipts.len(), MAX_DURABLE_OUTPUT_LEASE_RECEIPTS);
    }
}
