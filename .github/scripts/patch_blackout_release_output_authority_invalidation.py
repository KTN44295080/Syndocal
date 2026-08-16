from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


main = Path("app/src-tauri/src/main.rs")
replace_exact(
    main,
    '''fn apply_output_ownership_role(
    app: &tauri::AppHandle,
    state: &AppState,
    role: MachineOutputRole,
) -> Result<OutputOwnershipStatus, String> {
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
''',
    '''fn apply_output_ownership_role(
    app: &tauri::AppHandle,
    state: &AppState,
    role: MachineOutputRole,
) -> Result<OutputOwnershipStatus, String> {
    // Any explicit operator ownership-role transition invalidates a previously
    // prepared Release token before the transition can alter its output fence.
    state.control_plane_security.retire_all();
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
''',
    "operator output ownership transition invalidates prepared Release consent",
)

replace_exact(
    main,
    '''    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    harvest_spout_output_failures(state.spout_transport.as_ref(), &state.engine)
        .map_err(|error| (Some(path.clone()), error))?;
    state
        .engine
        .begin_output_ownership_transition(MachineOutputRole::Standby)
''',
    '''    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    harvest_spout_output_failures(state.spout_transport.as_ref(), &state.engine)
        .map_err(|error| (Some(path.clone()), error))?;
    // Standby is an explicit roadmap invalidation boundary for outstanding
    // human-present consent. This helper can be reached independently of the
    // ordinary role setter, so retire here as well rather than relying only on
    // a later stale-fence rejection.
    state.control_plane_security.retire_all();
    state
        .engine
        .begin_output_ownership_transition(MachineOutputRole::Standby)
''',
    "direct Standby transition invalidates prepared Release consent",
)
