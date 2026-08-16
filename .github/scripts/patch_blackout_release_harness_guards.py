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
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource, workspaceSource, backendSource, invokeManifestSource] = await Promise.all([\n''',
    "harness workspace/backend/manifest source slots",
)
replace_exact(
    harness,
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),\n]);\n''',
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8"),\n  readFile(new URL("../src/tauri-invoke-manifest.json", import.meta.url), "utf8"),\n]);\n''',
    "harness reads workspace backend and frontend authority",
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
    '''assert.doesNotMatch(appSource, /invoke\\("set_blackout", \\{ enabled: false \\}\\)/);\nassert.doesNotMatch(appSource, /invoke\\("set_all_blackout"/);\nassert.match(appSource, /await safetyBlackoutRuntime\\.engage\\(\\);[\\s\\S]*await setVideoBlackout\\(true\\);/);\nassert.match(appSource, /blackoutReleaseDisabled=\\{operatorLockMode\\(\\) === "Full"\\}/);\nassert.doesNotMatch(panelSource, />All Clear</);\nassert.match(controlSource, /disabled=\\{busy\\(\\) \\|\\| props\\.disabled\\}/);\nassert.doesNotMatch(controlSource, /Date\\.now\\(\\) >= consent\\.challenge\\.expires_at_unix_ms/);\nassert.match(controllerSource, /"receipt_expired"/);\nassert.doesNotMatch(workspaceSource, /onSetBlackout\\(!props\\.blackout\\)/);\nassert.doesNotMatch(workspaceSource, /onSetAllBlackout\\(!\\(props\\.blackout && props\\.videoBlackout\\)\\)/);\nassert.match(workspaceSource, /onSetBlackout\\(true\\)/);\nassert.match(workspaceSource, /onSetAllBlackout\\(true\\)/);\n\n// The UI is not the trust boundary. Legacy target-valued blackout setters are\n// unavailable in both directions until those adapters are migrated to canonical\n// principal/capability/request identity. Otherwise `true` can create the ordinary\n// blackout bit that R4 Release cannot clear, while `false` bypasses consent.\nassert.match(backendSource, /Legacy DMX blackout engage is disabled/);\nassert.match(backendSource, /Legacy DMX blackout release is disabled/);\nassert.match(backendSource, /Legacy All Blackout is disabled/);\nassert.match(backendSource, /Legacy All Blackout clear is disabled/);\nassert.match(backendSource, /Ignoring legacy MIDI blackout control/);\nassert.match(backendSource, /Ignoring legacy \\{source\\} blackout control/);\nassert.match(backendSource, /Ignoring legacy remote blackout control/);\nassert.doesNotMatch(backendSource, /MidiControlEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /MidiControlEvent::AllBlackout\\(enabled\\) => EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /OscInputEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /OscInputEvent::AllBlackout\\(enabled\\) => EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /RemoteInputEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /EngineCommand::SetAllBlackout\\(enabled\\)/);\n\nconst invokeManifest = JSON.parse(invokeManifestSource);\nassert.equal(Array.isArray(invokeManifest), true);\nassert.equal(invokeManifest.includes("set_blackout"), false);\nassert.equal(invokeManifest.includes("set_all_blackout"), false);\nassert.equal(invokeManifest.includes("execute_blackout_release_v1"), true);\nassert.match(appSource, /onBlackoutReleased=\\{refreshSnapshot\\}/);\n''',
    "harness locks canonical blackout source and renderer-authority invariants",
)
