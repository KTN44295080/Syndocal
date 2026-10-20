//! An accepted registry call may still contain a rejected lease receipt.
//! Keep physical Display changes behind the inner authorization result.
use super::output_lease::OutputLeaseRequestReceipt;

pub(super) fn commit_if_authorized<T>(
    receipt: &OutputLeaseRequestReceipt,
    physical_delta: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    super::ensure_output_lease_receipt_succeeded(receipt, "Display window authorization")?;
    physical_delta()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output_lease::{
        OutputLeaseError, OutputLeaseOwner, OutputLeaseRegistry, OutputLeaseRequest,
        OutputLeaseRequestAction, OutputLeaseResource, OutputLeaseResources,
    };
    use std::cell::Cell;

    fn display_receipt(video_only: bool, stale: bool) -> OutputLeaseRequestReceipt {
        let mut registry = OutputLeaseRegistry::fresh_process(41).unwrap();
        let owner = OutputLeaseOwner::new("display-qa", "main", 41, 1).unwrap();
        let both =
            OutputLeaseResources::new(&[OutputLeaseResource::Lighting, OutputLeaseResource::Video])
                .unwrap();
        let resources = if video_only {
            OutputLeaseResources::new(&[OutputLeaseResource::Video]).unwrap()
        } else {
            both.clone()
        };
        let acquire = OutputLeaseRequest::from_action(
            "display-qa",
            "display-test",
            1,
            OutputLeaseRequestAction::Acquire {
                owner: owner.clone(),
                resources,
                project_identity: "project-41".to_string(),
                ttl_ms: 100,
            },
        )
        .unwrap();
        let acquired = registry.submit_request(&acquire, 0).unwrap();
        assert!(acquired.outcome.is_ok());
        let request = OutputLeaseRequest::from_action(
            "display-qa",
            "display-test",
            2,
            OutputLeaseRequestAction::AuthorizeOrRecoverDisplay {
                lease_id: acquired.lease_id.unwrap(),
                owner,
                expected_generation: acquired.generation_after.unwrap() + u64::from(stale),
                exact_resources: both,
                project_identity: "project-41".to_string(),
                ttl_ms: 100,
            },
        )
        .unwrap();
        // The outer call succeeds even when the authorization outcome rejects.
        registry.submit_request(&request, 1).unwrap()
    }

    #[test]
    fn rejected_display_lease_never_runs_a_physical_delta() {
        let denied = display_receipt(true, false);
        assert_eq!(denied.outcome, Err(OutputLeaseError::ResourceConflict));
        let calls = Cell::new(0);
        for receipt in [denied, display_receipt(false, true)] {
            assert!(receipt.outcome.is_err());
            let result = commit_if_authorized(&receipt, || {
                calls.set(calls.get() + 1);
                Ok(())
            });
            assert!(result.is_err());
            assert_eq!(
                calls.get(),
                0,
                "no shell, worker, GPU, or close after rejection"
            );
        }
    }

    #[test]
    fn exact_both_display_authorization_runs_one_delta_and_preserves_its_result() {
        let authorized = display_receipt(false, false);
        assert!(authorized.outcome.is_ok());
        let calls = Cell::new(0);
        assert_eq!(
            commit_if_authorized(&authorized, || {
                calls.set(calls.get() + 1);
                Ok(42)
            }),
            Ok(42)
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            commit_if_authorized::<()>(&authorized, || Err("physical failure".into())),
            Err("physical failure".into())
        );
    }
}
