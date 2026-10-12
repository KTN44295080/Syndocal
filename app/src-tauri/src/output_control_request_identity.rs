//! Renderer request identities share the native, restart-seeded output sequence.
//! Allocation grants no output authority and never resets durable replay guards.
use super::{AppState, Ordering};
use protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;

pub(crate) fn allocate(state: &AppState, window_label: &str) -> Result<u64, String> {
    state.ensure_window_authority_not_blocked(window_label)?;
    let durable_next = state
        .output_lease_durable_receipts
        .lock()
        .map_err(|_| "Output request identity journal is unavailable")?
        .next_request_id()?;
    let registry_next = state
        .output_lease_registry
        .lock()
        .map_err(|_| "Output request identity registry is unavailable")?
        .next_request_id()?;
    let admitted_next =
        super::control_plane_runtime::output_control_request_id_floor(state, window_label)?;
    let floor = durable_next.max(registry_next).max(admitted_next);
    state
        .next_output_lease_request_id
        .fetch_max(floor, Ordering::AcqRel);
    let mut current = state.next_output_lease_request_id.load(Ordering::Acquire);
    loop {
        if current == 0 || current > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err("OutputControl request identity is exhausted".to_string());
        }
        match state.next_output_lease_request_id.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return Ok(current),
            Err(actual) => current = actual,
        }
    }
}

#[cfg(test)]
#[path = "output_control_request_identity_tests.rs"]
mod tests;
