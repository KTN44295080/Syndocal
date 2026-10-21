//! Collect only owned output workers before atomically fencing and stopping
//! them through Engine::reserve_output_ownership_retirement. Collection never
//! releases owners; existing join + native retirement ACK paths still do that.
use super::*;

/// Reap completed transport owners before a retry reserves another epoch. Stop
/// signal collection remains separate and never consumes a worker or lease.
pub(super) fn harvest_finished_outputs(_state: &AppState) -> Result<(), String> {
    #[cfg(feature = "ndi")]
    _state.ndi_transport.lock()
        .map_err(|_| "NDI output registry was poisoned before retirement harvest".to_string())?
        .harvest_failed_workers(&_state.engine)
        .map_err(|error| error.message)?;
    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    {
        harvest_show_spout_output_failures(_state.show_spout_transport.as_ref(), &_state.engine)?;
        harvest_spout_output_failures(_state.spout_transport.as_ref(), &_state.engine)?;
    }
    #[cfg(all(feature = "syphon", target_os = "macos"))]
    harvest_syphon_output_failures(_state)?;
    Ok(())
}

pub(super) fn stop_signals(state: &AppState) -> Result<Vec<Arc<AtomicBool>>, String> {
    let mut signals: Vec<_> = state.native_video_output_workers.lock()
        .map_err(|_| "Native video output registry was poisoned before retirement".to_string())?
        .values().map(|worker| Arc::clone(&worker.stop)).collect();
    #[cfg(feature = "ndi")]
    signals.extend(state.ndi_transport.lock()
        .map_err(|_| "NDI output registry was poisoned before retirement".to_string())?
        .output_stop_signals());
    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    {
        signals.extend(state.spout_transport.lock()
            .map_err(|_| "Spout output registry was poisoned before retirement".to_string())?
            .output_stop_signals());
        signals.extend(state.show_spout_transport.lock()
            .map_err(|_| "Show Spout output registry was poisoned before retirement".to_string())?
            .authority_change_stop_signals());
    }
    #[cfg(all(feature = "syphon", target_os = "macos"))]
    signals.extend(state.syphon_transport.lock()
        .map_err(|_| "Syphon output registry was poisoned before retirement".to_string())?
        .output_stop_signals());
    Ok(signals)
}
