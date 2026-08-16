from pathlib import Path


main = Path("app/src-tauri/src/main.rs")
source = main.read_text(encoding="utf-8")
marker = "#[tauri::command]\nfn run_phase1_smoke_test"
if source.count(marker) != 1:
    raise RuntimeError(
        f"phase1 smoke Tauri source: expected exactly one match, found {source.count(marker)}"
    )
replacement = '''#[cfg(not(test))]
#[tauri::command]
fn run_phase1_smoke_test(_state: State<'_, AppState>) -> Result<Phase1SmokeReport, String> {
    Err(
        "Phase 1 smoke blackout verification is test-only; production output state must use canonical S0/R4 controls"
            .to_string(),
    )
}

#[cfg(test)]
#[tauri::command]
fn run_phase1_smoke_test'''
main.write_text(source.replace(marker, replacement, 1), encoding="utf-8")
