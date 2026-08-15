from pathlib import Path

path = Path("app/src-tauri/src/main.rs")
source = path.read_text(encoding="utf-8")

guard = '    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]\n'
block = '''    spout
        .harvest_failed_workers(engine)
        .map_err(|error| error.message)?;
'''
guarded = guard + block
guarded_count = source.count(guarded)
if guarded_count == 1:
    pass
elif guarded_count == 0 and source.count(block) == 1:
    source = source.replace(block, guarded, 1)
else:
    raise RuntimeError(
        f"unexpected Spout harvest shape: guarded={guarded_count}, block={source.count(block)}"
    )

helper = '''fn replace_prepared_project_snapshot(
    state: &State<'_, AppState>,
'''
repaired = '''fn replace_prepared_project_snapshot(
    state: &AppState,
'''
helper_count = source.count(helper)
repaired_count = source.count(repaired)
if repaired_count == 1:
    pass
elif repaired_count == 0 and helper_count == 1:
    source = source.replace(helper, repaired, 1)
else:
    raise RuntimeError(
        f"unexpected prepared-project helper shape: repaired={repaired_count}, original={helper_count}"
    )

standby_helper = '''fn replace_prepared_project_snapshot_after_standby_stop(
    state: &State<'_, AppState>,
'''
standby_repaired = '''fn replace_prepared_project_snapshot_after_standby_stop(
    state: &AppState,
'''
standby_helper_count = source.count(standby_helper)
standby_repaired_count = source.count(standby_repaired)
if standby_repaired_count == 1:
    pass
elif standby_repaired_count == 0 and standby_helper_count == 1:
    source = source.replace(standby_helper, standby_repaired, 1)
else:
    raise RuntimeError(
        "unexpected standby-stop helper shape: "
        f"repaired={standby_repaired_count}, original={standby_helper_count}"
    )

path.write_text(source, encoding="utf-8")
Path(".github/workflows/diagnose-ai3-main.yml").unlink(missing_ok=True)
