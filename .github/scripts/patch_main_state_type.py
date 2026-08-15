from pathlib import Path

path = Path("app/src-tauri/src/main.rs")
data = path.read_bytes()

replacements = [
    (
        b"fn load_project_from_file_with_control_mappings_in_scope(\n    state: &State<'_, AppState>,\n",
        b"fn load_project_from_file_with_control_mappings_in_scope(\n    state: &AppState,\n",
        "scoped project-load core",
    ),
    (
        b"fn load_project_from_file_with_control_mappings_in_scope_and_disposition(\n    state: &State<'_, AppState>,\n",
        b"fn load_project_from_file_with_control_mappings_in_scope_and_disposition(\n    state: &AppState,\n",
        "scoped project-load disposition core",
    ),
]

for old, new, label in replacements:
    old_count = data.count(old)
    new_count = data.count(new)
    if old_count != 1 or new_count != 0:
        raise RuntimeError(
            f"{label}: expected old=1/new=0, found old={old_count}/new={new_count}"
        )
    data = data.replace(old, new, 1)

path.write_bytes(data)
