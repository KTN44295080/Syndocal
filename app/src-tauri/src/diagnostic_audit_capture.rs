//! Native source acquisition only. Source locks are never held together and
//! are never used to expire leases, reconcile projects or evict receipts.
use super::diagnostic_audit::{self as policy, Before, History};
use tauri::Manager;

pub(crate) fn read_lock<'a, T>(
    mutex: &'a std::sync::Mutex<T>,
    source: &str,
) -> Result<std::sync::MutexGuard<'a, T>, String> {
    match mutex.try_lock() {
        Ok(guard)=>Ok(guard),
        Err(std::sync::TryLockError::WouldBlock)=>Err(format!("diagnostic_audit_{source}_busy; capture failed before publication; use a fresh observation")),
        Err(std::sync::TryLockError::Poisoned(_))=>Err(format!("diagnostic_audit_{source}_unavailable; capture failed before publication; restart Syndocal")),
    }
}

pub(crate) fn capture(
    app: &tauri::AppHandle,
    state: &super::AppState,
    before: &Before,
    expected: Option<u64>,
) -> Result<History, String> {
    let process = app
        .try_state::<super::ControlPlaneQueryState>()
        .ok_or("diagnostic_audit_query_not_ready; wait for native startup")?
        .process_incarnation();
    before.validate(expected, process)?;
    let bridge = app
        .try_state::<super::agent_bridge::AgentBridge>()
        .ok_or("diagnostic_audit_not_ready; wait for native agent service startup")?;
    let mut pages = Vec::with_capacity(policy::SOURCES.len());
    pages.push(
        bridge
            .authority("main")?
            .diagnostic_audit(before.agent_authority)?,
    );
    pages.push(
        read_lock(&state.output_lease_registry, "output_lease")?
            .diagnostic_audit(before.output_lease)?,
    );
    pages.push(
        app.try_state::<super::project_file_control_plane::ProjectFileControlPlaneState>()
            .ok_or("diagnostic_audit_file_not_ready; wait for native startup")?
            .diagnostic_audit(before.project_file)?,
    );
    pages.push(
        app.try_state::<super::project_replacement_control_plane::ProjectReplacementControlPlaneState>(
        )
        .ok_or("diagnostic_audit_replacement_not_ready; wait for native startup")?
        .diagnostic_audit(before.project_replacement)?,
    );
    pages.extend(
        state
            .runtime_control_plane
            .diagnostic_audit(before.safety, before.output_control)?,
    );
    Ok(policy::history(process, pages))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_audit_busy_and_poisoned_sources_fail_without_mutating_history() {
        let mutex = std::sync::Mutex::new(vec![1_u64, 2, 3]);
        let guard = mutex.lock().unwrap();
        assert!(read_lock(&mutex, "file").unwrap_err().contains("file_busy"));
        assert_eq!(*guard, vec![1, 2, 3]);
        drop(guard);
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("isolated poison fixture");
        });
        assert!(read_lock(&mutex, "file")
            .unwrap_err()
            .contains("file_unavailable"));
        assert_eq!(*mutex.lock().unwrap_err().into_inner(), vec![1, 2, 3]);
    }
}
