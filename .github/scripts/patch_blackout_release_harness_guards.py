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
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource] = await Promise.all([\n''',
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource, workspaceSource, backendSource] = await Promise.all([\n''',
    "harness workspace/backend source slots",
)
replace_exact(
    harness,
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),\n]);\n''',
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8"),\n]);\n''',
    "harness reads workspace and backend release adapters",
)
replace_exact(
    harness,
    '''assert.match(panelSource, /<BlackoutReleaseControl\\s+onReleased=\\{props\\.onBlackoutReleased\\}\\s*\\/>/);\n''',
    '''assert.match(panelSource, /<BlackoutReleaseControl[\\s\\S]*disabled=\\{props\\.blackoutReleaseDisabled\\}[\\s\\S]*onReleased=\\{props\\.onBlackoutReleased\\}/);\n''',
    "harness expects Full Lock Release binding",
)
replace_exact(
    harness,
    '''assert.doesNotMatch(appSource, /invoke\\("set_blackout", \\{ enabled: false \\}\\)/);\nassert.match(appSource, /onBlackoutReleased=\\{refreshSnapshot\\}/);\n''',
    '''assert.doesNotMatch(appSource, /invoke\\("set_blackout", \\{ enabled: false \\}\\)/);\nassert.doesNotMatch(appSource, /invoke\\("set_all_blackout"/);\nassert.match(appSource, /await safetyBlackoutRuntime\\.engage\\(\\);[\\s\\S]*await setVideoBlackout\\(true\\);/);\nassert.match(appSource, /blackoutReleaseDisabled=\\{operatorLockMode\\(\\) === "Full"\\}/);\nassert.doesNotMatch(panelSource, />All Clear</);\nassert.match(controlSource, /disabled=\\{busy\\(\\) \\|\\| props\\.disabled\\}/);\nassert.doesNotMatch(controlSource, /Date\\.now\\(\\) >= consent\\.challenge\\.expires_at_unix_ms/);\nassert.match(controllerSource, /"receipt_expired"/);\nassert.doesNotMatch(workspaceSource, /onSetBlackout\\(!props\\.blackout\\)/);\nassert.doesNotMatch(workspaceSource, /onSetAllBlackout\\(!\\(props\\.blackout && props\\.videoBlackout\\)\\)/);\nassert.match(workspaceSource, /onSetBlackout\\(true\\)/);\nassert.match(workspaceSource, /onSetAllBlackout\\(true\\)/);\n\n// The UI is not the trust boundary. Legacy local/external adapters must also\n// reject the energizing release direction rather than bypassing R4 by sending\n// EngineCommand::Blackout(false) / SetAllBlackout(false) directly.\nassert.match(backendSource, /Legacy DMX blackout release is disabled/);\nassert.match(backendSource, /Legacy All Blackout clear is disabled/);\nassert.match(backendSource, /Ignoring MIDI blackout release/);\nassert.match(backendSource, /Ignoring \\{source\\} blackout release/);\nassert.match(backendSource, /Ignoring remote blackout release/);\nassert.doesNotMatch(backendSource, /MidiControlEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /MidiControlEvent::AllBlackout\\(enabled\\) => EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /OscInputEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /OscInputEvent::AllBlackout\\(enabled\\) => EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /RemoteInputEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.match(appSource, /onBlackoutReleased=\\{refreshSnapshot\\}/);\n''',
    "harness locks local and legacy-adapter release invariants",
)
