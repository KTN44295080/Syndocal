from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


harness = Path("app/scripts/check-blackout-release-runtime.mjs")

# The shortcut-suppression guard is landed directly in source before the
# generated R4 migration. Preserve that already-present keyboardSource slot
# while adding the generated-source audit inputs.
replace_exact(
    harness,
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource, keyboardSource] = await Promise.all([\n''',
    '''const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource, keyboardSource, shortcutActionsSource, workspaceSource, backendSource, runtimeSource, invokeManifestSource] = await Promise.all([\n''',
    "harness workspace/backend/runtime/manifest source slots after shortcut guard",
)
replace_exact(
    harness,
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8"),\n]);\n''',
    '''  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8"),\n  readFile(new URL("../src/appShortcutActions.ts", import.meta.url), "utf8"),\n  readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8"),\n  readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8"),\n  readFile(new URL("../src-tauri/src/control_plane_runtime.rs", import.meta.url), "utf8"),\n  readFile(new URL("../src/tauri-invoke-manifest.json", import.meta.url), "utf8"),\n]);\n''',
    "harness reads keyboard action workspace backend runtime and frontend authority after shortcut guard",
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
    '''assert.doesNotMatch(appSource, /invoke\\("set_blackout", \\{ enabled: false \\}\\)/);\nassert.doesNotMatch(appSource, /invoke\\("set_all_blackout"/);\nassert.match(appSource, /await safetyBlackoutRuntime\\.engage\\(\\);[\\s\\S]*await setVideoBlackout\\(true\\);/);\nassert.match(appSource, /blackoutReleaseDisabled=\\{operatorLockMode\\(\\) === "Full"\\}/);\nassert.doesNotMatch(panelSource, />All Clear</);\nassert.match(controlSource, /disabled=\\{busy\\(\\) \\|\\| props\\.disabled\\}/);\nassert.doesNotMatch(controlSource, /Date\\.now\\(\\) >= consent\\.challenge\\.expires_at_unix_ms/);\nassert.match(controllerSource, /"receipt_expired"/);\nassert.doesNotMatch(workspaceSource, /onSetBlackout\\(!props\\.blackout\\)/);\nassert.doesNotMatch(workspaceSource, /onSetAllBlackout\\(!\\(props\\.blackout && props\\.videoBlackout\\)\\)/);\nassert.match(workspaceSource, /onSetBlackout\\(true\\)/);\nassert.match(workspaceSource, /onSetAllBlackout\\(true\\)/);\nassert.doesNotMatch(shortcutActionsSource, /KeyB[^\\n]*enabled: !context\\.blackout/);\nassert.match(\n  shortcutActionsSource,\n  /if \\(event\\.code === "KeyB"\\) return \\{ kind: "toggleBlackout", enabled: true \\};/,\n);\n\n// The UI is not the trust boundary. Legacy target-valued blackout setters are\n// unavailable in both directions until those adapters are migrated to canonical\n// principal/capability/request identity. Otherwise `true` can create the ordinary\n// blackout bit that R4 Release cannot clear, while `false` bypasses consent.\nassert.match(backendSource, /Legacy DMX blackout engage is disabled/);\nassert.match(backendSource, /Legacy DMX blackout release is disabled/);\nassert.match(backendSource, /Legacy All Blackout is disabled/);\nassert.match(backendSource, /Legacy All Blackout clear is disabled/);\nassert.match(backendSource, /Ignoring legacy MIDI blackout control/);\nassert.match(backendSource, /Ignoring legacy \\{source\\} blackout control/);\nassert.match(backendSource, /Ignoring legacy remote blackout control/);\nassert.match(backendSource, /if lock_mode == OperatorLockMode::Full \\{[\\s\\S]*control_plane_security\\.retire_all\\(\\)/);\nassert.doesNotMatch(backendSource, /MidiControlEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /MidiControlEvent::AllBlackout\\(enabled\\) => EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /OscInputEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /OscInputEvent::AllBlackout\\(enabled\\) => EngineCommand::SetAllBlackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /RemoteInputEvent::Blackout\\(enabled\\) => EngineCommand::Blackout\\(enabled\\)/);\nassert.doesNotMatch(backendSource, /EngineCommand::SetAllBlackout\\(enabled\\)/);\n\n// Full Lock must be enforced again in the backend at both human-presence\n// preparation and execution. The explicit lock transition also retires any\n// previously prepared token above, but these gates are still required for\n// policy/session races and callers which bypass the renderer UI.\nconst prepareStart = runtimeSource.indexOf("pub(crate) fn prepare_output_consent(");\nconst prepareEnd = runtimeSource.indexOf("pub(crate) fn output_consent_status(", prepareStart);\nassert.ok(prepareStart >= 0 && prepareEnd > prepareStart);\nconst prepareRuntimeSource = runtimeSource.slice(prepareStart, prepareEnd);\nassert.match(prepareRuntimeSource, /ensure_project_operator_video_clip_slot_runtime_allowed\\(/);\n\nconst executeStart = runtimeSource.indexOf("pub(crate) fn execute_output_control(");\nconst executeEnd = runtimeSource.indexOf("fn output_control_rejection(", executeStart);\nassert.ok(executeStart >= 0 && executeEnd > executeStart);\nconst executeRuntimeSource = runtimeSource.slice(executeStart, executeEnd);\nassert.match(executeRuntimeSource, /ensure_project_operator_video_clip_slot_runtime_allowed\\(/);\nassert.match(executeRuntimeSource, /consume_consent\\(/);\n\nconst invokeManifest = JSON.parse(invokeManifestSource);\nassert.equal(Array.isArray(invokeManifest), true);\nassert.equal(invokeManifest.includes("set_blackout"), false);\nassert.equal(invokeManifest.includes("set_all_blackout"), false);\nassert.equal(invokeManifest.includes("execute_blackout_release_v1"), true);\nassert.match(appSource, /onBlackoutReleased=\\{refreshSnapshot\\}/);\n''',
    "harness locks canonical blackout source, engage-only keyboard, Full Lock backend and renderer-authority invariants",
)
