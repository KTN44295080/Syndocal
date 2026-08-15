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
    // Any operator output-authority transition, including entering Standby,
    // invalidates a previously prepared Release token before the transition
    // can alter the fence it was bound to.
    state.control_plane_security.retire_all();
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
''',
    "output ownership transition invalidates prepared Release consent",
)
