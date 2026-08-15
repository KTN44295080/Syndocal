from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


harness = Path("app/scripts/check-blackout-release-runtime.mjs")

replace_exact(
    harness,
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource] = await Promise.all([
''',
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource, workspaceSource] = await Promise.all([
''',
    "harness WorkspaceChrome source slot",
)
replace_exact(
    harness,
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),
]);
''',
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8"),
]);
''',
    "harness reads WorkspaceChrome",
)
replace_exact(
    harness,
    '''assert.match(panelSource, /<BlackoutReleaseControl\\s+onReleased=\\{props\\.onBlackoutReleased\\}\\s*\\/>/);
''',
    '''assert.match(panelSource, /<BlackoutReleaseControl[\\s\\S]*disabled=\\{props\\.blackoutReleaseDisabled\\}[\\s\\S]*onReleased=\\{props\\.onBlackoutReleased\\}/);
''',
    "harness expects Full Lock Release binding",
)
replace_exact(
    harness,
    '''assert.doesNotMatch(appSource, /invoke\\("set_blackout", \\{ enabled: false \\}\\)/);
assert.match(appSource, /onBlackoutReleased=\\{refreshSnapshot\\}/);
''',
    '''assert.doesNotMatch(appSource, /invoke\\("set_blackout", \\{ enabled: false \\}\\)/);
assert.doesNotMatch(appSource, /invoke\\("set_all_blackout"/);
assert.match(appSource, /await safetyBlackoutRuntime\\.engage\\(\\);[\\s\\S]*await setVideoBlackout\\(true\\);/);
assert.match(appSource, /blackoutReleaseDisabled=\\{operatorLockMode\\(\\) === "Full"\\}/);
assert.doesNotMatch(panelSource, />All Clear</);
assert.match(controlSource, /disabled=\\{busy\\(\\) \\|\\| props\\.disabled\\}/);
assert.doesNotMatch(controlSource, /Date\\.now\\(\\) >= consent\\.challenge\\.expires_at_unix_ms/);
assert.doesNotMatch(workspaceSource, /onSetBlackout\\(!props\\.blackout\\)/);
assert.doesNotMatch(workspaceSource, /onSetAllBlackout\\(!\\(props\\.blackout && props\\.videoBlackout\\)\\)/);
assert.match(workspaceSource, /onSetBlackout\\(true\\)/);
assert.match(workspaceSource, /onSetAllBlackout\\(true\\)/);
assert.match(appSource, /onBlackoutReleased=\\{refreshSnapshot\\}/);
''',
    "harness locks legacy-release and Full Lock UI invariants",
)
